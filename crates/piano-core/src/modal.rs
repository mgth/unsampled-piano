//! Bank of damped modes, the computational core of the engine.
//!
//! Each mode is a complex one-pole recursion `z[n] = p·z[n-1] + b·x[n]` with
//! `p = exp(-σT)·exp(iωT)`, and the mode's contribution is `Re(w·z)` with a
//! complex output weight w. This is the impulse-invariant discretisation of
//! a driven damped oscillator, as in Bank, Zambon & Fontana (IEEE TASLP
//! 2010). The complex ("rotation") form is used rather than a real biquad
//! because its frequency stays accurate in f32 down to A0 (see
//! `piano-bench precision`).
//!
//! Layout is AoSoA: modes are packed in chunks of [`LANES`] so the inner loop
//! vectorises across modes while the time recursion stays sequential. Several
//! chunks can be advanced in the same time loop (`group`) to hide the latency
//! of the recursion. Results are bit-identical whatever the ISA or group size:
//! no FMA contraction, and the summation order is fixed.

pub const LANES: usize = 16;

/// Parameters of one mode, in physical units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ModeSpec {
    pub freq_hz: f64,
    /// Amplitude decay rate σ in 1/s (T60 = ln(1000)/σ).
    pub sigma: f64,
    /// Complex input gain `b` (already scaled by the sample period).
    pub gain_re: f64,
    pub gain_im: f64,
    /// Complex output weight `w`: the mode contributes Re(w·z).
    pub out_re: f64,
    pub out_im: f64,
}

impl Default for ModeSpec {
    fn default() -> Self {
        Self { freq_hz: 0.0, sigma: 0.0, gain_re: 0.0, gain_im: 0.0, out_re: 1.0, out_im: 0.0 }
    }
}

impl ModeSpec {
    /// Mode driven by a force: its impulse response is `amp·e^{-σt}·sin(ωt)`.
    pub fn driven(freq_hz: f64, sigma: f64, amp: f64) -> Self {
        Self { freq_hz, sigma, gain_im: -amp, ..Default::default() }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Isa {
    Generic,
    #[cfg(target_arch = "x86_64")]
    Avx2,
    #[cfg(target_arch = "x86_64")]
    Avx512,
}

impl Isa {
    pub fn detect() -> Self {
        *Self::available().last().unwrap()
    }

    pub fn available() -> Vec<Isa> {
        #[allow(unused_mut)]
        let mut v = vec![Isa::Generic];
        #[cfg(target_arch = "x86_64")]
        {
            if is_x86_feature_detected!("avx2") {
                v.push(Isa::Avx2);
            }
            if is_x86_feature_detected!("avx512f") {
                v.push(Isa::Avx512);
            }
        }
        v
    }

    /// Chunks per time loop that maximise throughput (measured on Zen 5:
    /// AVX2 spills registers at 4, AVX-512 keeps 4 groups in its 32 zmm).
    pub fn best_group(self) -> usize {
        match self {
            Isa::Generic => 1,
            #[cfg(target_arch = "x86_64")]
            Isa::Avx2 => 2,
            #[cfg(target_arch = "x86_64")]
            Isa::Avx512 => 4,
        }
    }
}

#[derive(Clone, Copy, Default)]
#[repr(C, align(64))]
struct Chunk {
    pr: [f32; LANES],
    pi: [f32; LANES],
    br: [f32; LANES],
    bi: [f32; LANES],
    or: [f32; LANES],
    oi: [f32; LANES],
    zr: [f32; LANES],
    zi: [f32; LANES],
}

pub struct ModalBank {
    sample_rate: f64,
    chunks: Vec<Chunk>,
    specs: Vec<ModeSpec>,
    n_modes: usize,
    acc: Vec<[f32; LANES]>,
    isa: Isa,
    group: usize,
}

impl ModalBank {
    /// Allocates everything up front; nothing allocates afterwards.
    pub fn new(sample_rate: f64, max_modes: usize, max_block: usize) -> Self {
        let n_chunks = max_modes.div_ceil(LANES);
        Self {
            sample_rate,
            chunks: vec![Chunk::default(); n_chunks],
            specs: vec![ModeSpec::default(); n_chunks * LANES],
            n_modes: 0,
            acc: vec![[0.0; LANES]; max_block],
            isa: Isa::detect(),
            group: Isa::detect().best_group(),
        }
    }

    pub fn capacity(&self) -> usize {
        self.specs.len()
    }

    pub fn len(&self) -> usize {
        self.n_modes
    }

    pub fn is_empty(&self) -> bool {
        self.n_modes == 0
    }

    pub fn max_block(&self) -> usize {
        self.acc.len()
    }

    /// Also resets the group size to the best one for this ISA.
    pub fn set_isa(&mut self, isa: Isa) {
        self.isa = isa;
        self.group = isa.best_group();
    }

    /// Number of chunks advanced together in the time loop (1, 2 or 4).
    pub fn set_group(&mut self, group: usize) {
        assert!(matches!(group, 1 | 2 | 4));
        self.group = group;
    }

    /// Replaces the modes, keeping the state of modes that keep their index.
    /// Modes beyond the capacity are dropped. Real-time safe.
    pub fn set_modes(&mut self, modes: impl IntoIterator<Item = ModeSpec>) {
        let mut n = 0;
        for m in modes.into_iter().take(self.specs.len()) {
            self.specs[n] = m;
            n += 1;
        }
        let old = self.n_modes;
        self.n_modes = n;
        self.update_poles(0.0);
        // Padding lanes and modes that disappeared must stay silent.
        for i in n..old.max(n).next_multiple_of(LANES).min(self.specs.len()) {
            let c = &mut self.chunks[i / LANES];
            c.zr[i % LANES] = 0.0;
            c.zi[i % LANES] = 0.0;
        }
    }

    /// Recomputes poles from the stored specs with an extra decay rate added
    /// to every mode (crude damper until the real one exists). Real-time safe.
    pub fn update_poles(&mut self, extra_sigma: f64) {
        let t = 1.0 / self.sample_rate;
        for i in 0..self.n_modes.next_multiple_of(LANES).min(self.specs.len()) {
            let c = &mut self.chunks[i / LANES];
            let l = i % LANES;
            if i < self.n_modes {
                let m = self.specs[i];
                let r = (-(m.sigma + extra_sigma) * t).exp();
                let (s, co) = (std::f64::consts::TAU * m.freq_hz * t).sin_cos();
                c.pr[l] = (r * co) as f32;
                c.pi[l] = (r * s) as f32;
                c.br[l] = m.gain_re as f32;
                c.bi[l] = m.gain_im as f32;
                c.or[l] = m.out_re as f32;
                c.oi[l] = m.out_im as f32;
            } else {
                c.pr[l] = 0.0;
                c.pi[l] = 0.0;
                c.br[l] = 0.0;
                c.bi[l] = 0.0;
                c.or[l] = 0.0;
                c.oi[l] = 0.0;
            }
        }
    }

    pub fn spec(&self, i: usize) -> ModeSpec {
        self.specs[i]
    }

    /// Complex state z of mode `i`.
    pub fn state(&self, i: usize) -> (f64, f64) {
        let c = &self.chunks[i / LANES];
        (c.zr[i % LANES] as f64, c.zi[i % LANES] as f64)
    }

    pub fn set_state(&mut self, i: usize, re: f64, im: f64) {
        let c = &mut self.chunks[i / LANES];
        c.zr[i % LANES] = re as f32;
        c.zi[i % LANES] = im as f32;
    }

    pub fn reset(&mut self) {
        for c in &mut self.chunks {
            c.zr = [0.0; LANES];
            c.zi = [0.0; LANES];
        }
    }

    /// Sum of |w·z|² over modes (output-weighted), a cheap loudness proxy
    /// for voice management.
    pub fn energy(&self) -> f32 {
        let mut e = [0.0f32; LANES];
        for c in &self.chunks[..self.n_modes.div_ceil(LANES)] {
            for (l, el) in e.iter_mut().enumerate() {
                let w2 = c.or[l] * c.or[l] + c.oi[l] * c.oi[l];
                *el += w2 * (c.zr[l] * c.zr[l] + c.zi[l] * c.zi[l]);
            }
        }
        e.iter().sum()
    }

    /// Advances the bank by `out.len()` samples and *adds* its output to `out`.
    /// `input` (the driving force) must have the same length when present.
    pub fn process(&mut self, input: Option<&[f32]>, out: &mut [f32]) {
        let len = out.len();
        assert!(len <= self.acc.len(), "block larger than max_block");
        if let Some(x) = input {
            assert_eq!(x.len(), len);
        }
        let acc = &mut self.acc[..len];
        let chunks = &mut self.chunks[..self.n_modes.div_ceil(LANES)];
        let g = self.group;
        match (self.isa, input) {
            (Isa::Generic, None) => run::<false>(chunks, &[], acc, g),
            (Isa::Generic, Some(x)) => run::<true>(chunks, x, acc, g),
            // SAFETY: these variants are only selectable when detected.
            #[cfg(target_arch = "x86_64")]
            (Isa::Avx2, None) => unsafe { run_avx2::<false>(chunks, &[], acc, g) },
            #[cfg(target_arch = "x86_64")]
            (Isa::Avx2, Some(x)) => unsafe { run_avx2::<true>(chunks, x, acc, g) },
            #[cfg(target_arch = "x86_64")]
            (Isa::Avx512, None) => unsafe { run_avx512::<false>(chunks, &[], acc, g) },
            #[cfg(target_arch = "x86_64")]
            (Isa::Avx512, Some(x)) => unsafe { run_avx512::<true>(chunks, x, acc, g) },
        }
        for (o, a) in out.iter_mut().zip(acc.iter()) {
            *o += a.iter().sum::<f32>();
        }
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
fn run_avx2<const IN: bool>(chunks: &mut [Chunk], x: &[f32], acc: &mut [[f32; LANES]], g: usize) {
    run::<IN>(chunks, x, acc, g)
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx512f")]
fn run_avx512<const IN: bool>(chunks: &mut [Chunk], x: &[f32], acc: &mut [[f32; LANES]], g: usize) {
    run::<IN>(chunks, x, acc, g)
}

#[inline(always)]
fn run<const IN: bool>(chunks: &mut [Chunk], x: &[f32], acc: &mut [[f32; LANES]], g: usize) {
    acc.fill([0.0; LANES]);
    let rest = match g {
        4 => {
            let (grps, rest) = chunks.as_chunks_mut::<4>();
            for grp in grps {
                group::<4, IN>(grp, x, acc);
            }
            rest
        }
        2 => {
            let (grps, rest) = chunks.as_chunks_mut::<2>();
            for grp in grps {
                group::<2, IN>(grp, x, acc);
            }
            rest
        }
        _ => chunks,
    };
    for grp in rest.as_chunks_mut::<1>().0 {
        group::<1, IN>(grp, x, acc);
    }
}

#[inline(always)]
fn group<const G: usize, const IN: bool>(grp: &mut [Chunk; G], x: &[f32], acc: &mut [[f32; LANES]]) {
    let pr: [[f32; LANES]; G] = std::array::from_fn(|g| grp[g].pr);
    let pi: [[f32; LANES]; G] = std::array::from_fn(|g| grp[g].pi);
    let br: [[f32; LANES]; G] = std::array::from_fn(|g| grp[g].br);
    let bi: [[f32; LANES]; G] = std::array::from_fn(|g| grp[g].bi);
    let or: [[f32; LANES]; G] = std::array::from_fn(|g| grp[g].or);
    let oi: [[f32; LANES]; G] = std::array::from_fn(|g| grp[g].oi);
    let mut zr: [[f32; LANES]; G] = std::array::from_fn(|g| grp[g].zr);
    let mut zi: [[f32; LANES]; G] = std::array::from_fn(|g| grp[g].zi);

    let mut step = |a: &mut [f32; LANES], xn: f32| {
        let mut s = *a;
        for g in 0..G {
            for l in 0..LANES {
                let mut r = pr[g][l] * zr[g][l] - pi[g][l] * zi[g][l];
                let mut i = pr[g][l] * zi[g][l] + pi[g][l] * zr[g][l];
                if IN {
                    r += br[g][l] * xn;
                    i += bi[g][l] * xn;
                }
                zr[g][l] = r;
                zi[g][l] = i;
                s[l] += or[g][l] * r - oi[g][l] * i;
            }
        }
        *a = s;
    };
    if IN {
        for (a, &xn) in acc.iter_mut().zip(x) {
            step(a, xn);
        }
    } else {
        for a in acc.iter_mut() {
            step(a, 0.0);
        }
    }

    for g in 0..G {
        grp[g].zr = zr[g];
        grp[g].zi = zi[g];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bank(isa: Isa, group: usize) -> ModalBank {
        let mut b = ModalBank::new(48_000.0, 200, 128);
        b.set_isa(isa);
        b.set_group(group);
        b.set_modes((1..=150).map(|k| ModeSpec::driven(27.5 * k as f64, 0.5 + 0.01 * k as f64, 1e-3)));
        b
    }

    #[test]
    fn impulse_response_matches_analytic() {
        let sr = 48_000.0;
        let mut b = ModalBank::new(sr, 1, 64);
        let (f, sigma, amp) = (440.0, 3.0, 0.5);
        b.set_modes([ModeSpec::driven(f, sigma, amp)]);
        let mut x = [0.0f32; 64];
        x[0] = 1.0;
        let mut y = [0.0f32; 64];
        b.process(Some(&x), &mut y);
        for (n, &v) in y.iter().enumerate() {
            let t = n as f64 / sr;
            let want = amp * (-sigma * t).exp() * (std::f64::consts::TAU * f * t).sin();
            assert!((v as f64 - want).abs() < 1e-6, "n={n} got {v} want {want}");
        }
    }

    #[test]
    fn all_isas_and_groups_are_bit_identical() {
        let mut x = vec![0.0f32; 128];
        x[3] = 1.0;
        x[40] = -0.3;
        let mut reference: Option<Vec<f32>> = None;
        for isa in Isa::available() {
            for g in [1, 2, 4] {
                let mut b = bank(isa, g);
                let mut out = vec![0.0f32; 128 * 4];
                for blk in out.chunks_mut(128) {
                    b.process(Some(&x), blk);
                }
                match &reference {
                    None => reference = Some(out),
                    Some(r) => assert!(r.iter().zip(&out).all(|(a, b)| a.to_bits() == b.to_bits()), "{isa:?} g={g}"),
                }
            }
        }
    }

    #[test]
    fn shrinking_mode_count_silences_dropped_modes() {
        let mut b = bank(Isa::Generic, 1);
        let mut x = vec![0.0f32; 128];
        x[0] = 1.0;
        let mut out = vec![0.0f32; 128];
        b.process(Some(&x), &mut out);
        b.set_modes([]);
        assert_eq!(b.energy(), 0.0);
    }
}
