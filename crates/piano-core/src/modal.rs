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

use crate::simd::V16;

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
    /// Weight of the second output (see [`ModalBank::process2`]).
    pub out2_re: f64,
    pub out2_im: f64,
    /// Gain of the second input (see [`ModalBank::process_two_inputs`]).
    pub gain2_re: f64,
    pub gain2_im: f64,
    /// Extra decay rate per unit of [`ModalBank::set_damping`] (dampers).
    pub damp_w: f64,
}

impl Default for ModeSpec {
    fn default() -> Self {
        Self {
            freq_hz: 0.0,
            sigma: 0.0,
            gain_re: 0.0,
            gain_im: 0.0,
            out_re: 1.0,
            out_im: 0.0,
            out2_re: 0.0,
            out2_im: 0.0,
            gain2_re: 0.0,
            gain2_im: 0.0,
            damp_w: 0.0,
        }
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
    /// Best ISA of this CPU, capped by the `PIANO_ISA` environment variable
    /// (`generic`, `avx2`) to measure what smaller machines would see.
    pub fn detect() -> Self {
        let best = *Self::available().last().unwrap();
        let cap = match std::env::var("PIANO_ISA").as_deref() {
            Ok("generic") => Isa::Generic,
            #[cfg(target_arch = "x86_64")]
            Ok("avx2") => Isa::Avx2,
            _ => return best,
        };
        Self::available().into_iter().rfind(|&i| i as u8 <= cap as u8).unwrap_or(best)
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

    /// Chunks per time loop that maximise throughput of free-running banks
    /// (measured on Zen 5: AVX2 spills registers at 4, AVX-512 keeps 4
    /// groups in its 32 zmm). Driven banks are capped at 2.
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
    b2r: [f32; LANES],
    b2i: [f32; LANES],
    or: [f32; LANES],
    oi: [f32; LANES],
    o2r: [f32; LANES],
    o2i: [f32; LANES],
    zr: [f32; LANES],
    zi: [f32; LANES],
}

impl Chunk {
    /// Takes `o`'s coefficients, keeping this chunk's states.
    fn copy_coefs(&mut self, o: &Chunk) {
        let (zr, zi) = (self.zr, self.zi);
        *self = *o;
        self.zr = zr;
        self.zi = zi;
    }
}

/// Writes the coefficients of `modes` (at most `chunks.len()·LANES`) into
/// `chunks` and `dw`, padding lanes of the last chunk silent; returns how
/// many were written. States are left alone.
fn fill(chunks: &mut [Chunk], dw: &mut [f32], modes: impl IntoIterator<Item = ModeSpec>, sample_rate: f64) -> usize {
    let t = 1.0 / sample_rate;
    let mut n = 0;
    for m in modes.into_iter().take(chunks.len() * LANES) {
        let (c, l) = (&mut chunks[n / LANES], n % LANES);
        let r = (-m.sigma * t).exp();
        let (s, co) = (std::f64::consts::TAU * m.freq_hz * t).sin_cos();
        c.pr[l] = (r * co) as f32;
        c.pi[l] = (r * s) as f32;
        c.br[l] = m.gain_re as f32;
        c.bi[l] = m.gain_im as f32;
        c.b2r[l] = m.gain2_re as f32;
        c.b2i[l] = m.gain2_im as f32;
        c.or[l] = m.out_re as f32;
        c.oi[l] = m.out_im as f32;
        c.o2r[l] = m.out2_re as f32;
        c.o2i[l] = m.out2_im as f32;
        dw[n] = (m.damp_w * t) as f32;
        n += 1;
    }
    for i in n..n.next_multiple_of(LANES) {
        let (c, l) = (&mut chunks[i / LANES], i % LANES);
        for a in [&mut c.pr, &mut c.pi, &mut c.br, &mut c.bi, &mut c.b2r, &mut c.b2i, &mut c.or, &mut c.oi, &mut c.o2r, &mut c.o2i] {
            a[l] = 0.0;
        }
        dw[i] = 0.0;
    }
    n
}

/// A bank's coefficients computed ahead, off the audio thread (they take
/// an exp and a sin_cos per mode in f64): [`ModalBank::load`] only copies
/// them.
pub struct ModeTable {
    chunks: Vec<Chunk>,
    dw: Vec<f32>,
    n: usize,
}

impl ModeTable {
    pub fn new(sample_rate: f64, modes: &[ModeSpec]) -> Self {
        let n_chunks = modes.len().div_ceil(LANES);
        let mut chunks = vec![Chunk::default(); n_chunks];
        let mut dw = vec![0.0; n_chunks * LANES];
        let n = fill(&mut chunks, &mut dw, modes.iter().copied(), sample_rate);
        Self { chunks, dw, n }
    }

    pub fn len(&self) -> usize {
        self.n
    }

    pub fn is_empty(&self) -> bool {
        self.n == 0
    }
}

/// e^{−x} for x ≥ 0 in f32, within 2 ulp, written to vectorise:
/// e^{−x} = 2^k·e^r with k = round(−x/ln 2) and |r| ≤ ½·ln 2 (Cody–Waite:
/// ln 2 in two parts, so r is exact), e^r by its degree-7 series.
#[inline(always)]
fn exp_neg(x: f32) -> f32 {
    const LN2_HI: f32 = 0.693_145_75;
    const LN2_LO: f32 = 1.428_606_8e-6;
    // Round to nearest without a libm call (exact for |t| < 2²²).
    const MAGIC: f32 = 12_582_912.0;
    let x = x.min(87.0);
    let kk = -x * std::f32::consts::LOG2_E + MAGIC;
    let k = kk - MAGIC;
    // k as an integer, from the low mantissa bits of kk (no float→int
    // conversion: it would not vectorise on SSE2).
    let ki = kk.to_bits().wrapping_sub(MAGIC.to_bits());
    let r = (-x - k * LN2_HI) - k * LN2_LO;
    let p = 1.0
        + r * (1.0 + r * (0.5 + r * (1.0 / 6.0 + r * (1.0 / 24.0 + r * (1.0 / 120.0 + r * (1.0 / 720.0 + r * (1.0 / 5040.0)))))));
    p * f32::from_bits(ki.wrapping_add(127) << 23)
}

pub struct ModalBank {
    sample_rate: f64,
    chunks: Vec<Chunk>,
    n_modes: usize,
    /// Modes set by `set_modes` (≥ n_modes, see `set_active_len`).
    n_specs: usize,
    acc: Vec<[f32; LANES]>,
    acc2: Vec<[f32; LANES]>,
    isa: Isa,
    group: usize,
    /// Per mode: undamped pole and damper weight (for set_damping).
    pr0: Vec<f32>,
    pi0: Vec<f32>,
    dw: Vec<f32>,
    damping: f32,
}

impl ModalBank {
    /// Allocates everything up front; nothing allocates afterwards.
    pub fn new(sample_rate: f64, max_modes: usize, max_block: usize) -> Self {
        let n_chunks = max_modes.div_ceil(LANES);
        Self {
            sample_rate,
            chunks: vec![Chunk::default(); n_chunks],
            n_modes: 0,
            n_specs: 0,
            acc: vec![[0.0; LANES]; max_block],
            acc2: vec![[0.0; LANES]; max_block],
            isa: Isa::detect(),
            group: Isa::detect().best_group(),
            pr0: vec![0.0; n_chunks * LANES],
            pi0: vec![0.0; n_chunks * LANES],
            dw: vec![0.0; n_chunks * LANES],
            damping: 0.0,
        }
    }

    pub fn capacity(&self) -> usize {
        self.chunks.len() * LANES
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
    /// Modes beyond the capacity are dropped. Real-time safe, but computes
    /// an exp and a sin_cos per mode: see [`Self::load`].
    pub fn set_modes(&mut self, modes: impl IntoIterator<Item = ModeSpec>) {
        let n = fill(&mut self.chunks, &mut self.dw, modes, self.sample_rate);
        self.took(n);
    }

    /// Replaces the modes by a precomputed table (a copy), keeping the
    /// state of modes that keep their index. Real-time safe.
    pub fn load(&mut self, table: &ModeTable) {
        let nc = table.chunks.len().min(self.chunks.len());
        for (c, t) in self.chunks[..nc].iter_mut().zip(&table.chunks) {
            c.copy_coefs(t);
        }
        let n = table.n.min(nc * LANES);
        self.dw[..nc * LANES].copy_from_slice(&table.dw[..nc * LANES]);
        self.took(n);
    }

    /// After new coefficients for `n` modes: undamped poles, and silence
    /// in the padding lanes and the modes that disappeared.
    fn took(&mut self, n: usize) {
        for (i, c) in self.chunks[..n.div_ceil(LANES)].iter().enumerate() {
            self.pr0[i * LANES..(i + 1) * LANES].copy_from_slice(&c.pr);
            self.pi0[i * LANES..(i + 1) * LANES].copy_from_slice(&c.pi);
        }
        let old = self.n_modes;
        self.n_modes = n;
        self.n_specs = n;
        self.damping = 0.0;
        for i in n..old.max(n).next_multiple_of(LANES).min(self.capacity()) {
            let c = &mut self.chunks[i / LANES];
            c.zr[i % LANES] = 0.0;
            c.zi[i % LANES] = 0.0;
        }
    }

    /// Adds `scale·damp_w` to every mode's decay rate (dampers). Cheap: one
    /// vectorised exp per mode, no trigonometry. Real-time safe.
    pub fn set_damping(&mut self, scale: f32) {
        if scale == self.damping {
            return;
        }
        self.damping = scale;
        self.apply_damping(0, self.n_modes);
    }

    /// Damped poles of modes `from..to` at the current damper scale.
    fn apply_damping(&mut self, from: usize, to: usize) {
        let d = self.damping;
        let lane = |c: &mut Chunk, l: usize, i: usize, pr0: &[f32], pi0: &[f32], dw: &[f32]| {
            let r = if d == 0.0 { 1.0 } else { exp_neg(d * dw[i]) };
            c.pr[l] = r * pr0[i];
            c.pi[l] = r * pi0[i];
        };
        let (pr0, pi0, dw) = (&self.pr0[..], &self.pi0[..], &self.dw[..]);
        let (first, last) = (from.next_multiple_of(LANES).min(to), to - to % LANES);
        for i in from..first {
            lane(&mut self.chunks[i / LANES], i % LANES, i, pr0, pi0, dw);
        }
        // Whole chunks: vectorises.
        let (c0, c1) = (first / LANES, last.max(first) / LANES);
        let (chunks, p0, q0, w) = (&mut self.chunks[c0..c1], &pr0[c0 * LANES..c1 * LANES], &pi0[c0 * LANES..c1 * LANES], &dw[c0 * LANES..c1 * LANES]);
        match self.isa {
            Isa::Generic => damp_chunks(chunks, p0, q0, w, d),
            // SAFETY: only selectable when detected.
            #[cfg(target_arch = "x86_64")]
            Isa::Avx2 => unsafe { damp_chunks_avx2(chunks, p0, q0, w, d) },
            #[cfg(target_arch = "x86_64")]
            Isa::Avx512 => unsafe { damp_chunks_avx512(chunks, p0, q0, w, d) },
        }
        for i in last.max(first)..to {
            lane(&mut self.chunks[i / LANES], i % LANES, i, pr0, pi0, dw);
        }
    }

    pub fn damping(&self) -> f32 {
        self.damping
    }

    /// Number of modes run (a prefix of the modes set): sympathetic-only
    /// strings run their low partials only. Real-time safe.
    pub fn set_active_len(&mut self, n: usize) {
        let n = n.min(self.n_specs);
        let old = self.n_modes;
        self.n_modes = n;
        if n > old {
            // Newly enabled modes start at rest, damped like the others.
            self.apply_damping(old, n);
            for i in old..n {
                self.set_state(i, 0.0, 0.0);
            }
        }
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

    /// Stops running the last modes while their output-weighted energy,
    /// summed from the end, stays below `threshold` (whole chunks; their
    /// states are cleared). For a bank decaying freely, whose high modes die
    /// first. Returns the modes still run. Real-time safe.
    pub fn drop_tail(&mut self, threshold: f32) -> usize {
        let mut e = 0.0f32;
        let mut c = self.n_modes.div_ceil(LANES);
        while c > 0 {
            let ch = &self.chunks[c - 1];
            let mut ce = 0.0f32;
            for l in 0..LANES {
                ce += (ch.or[l] * ch.or[l] + ch.oi[l] * ch.oi[l]) * (ch.zr[l] * ch.zr[l] + ch.zi[l] * ch.zi[l]);
            }
            if e + ce >= threshold {
                break;
            }
            e += ce;
            c -= 1;
        }
        let n = (c * LANES).min(self.n_modes);
        if n < self.n_modes {
            for ch in &mut self.chunks[c..self.n_modes.div_ceil(LANES)] {
                ch.zr = [0.0; LANES];
                ch.zi = [0.0; LANES];
            }
            self.n_modes = n;
        }
        n
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
        self.process_impl::<false>(input, None, out, &mut []);
    }

    /// Like [`Self::process`] with a second output (weights `out2_*`) computed
    /// from the same states in the same pass (e.g. two microphones).
    pub fn process2(&mut self, input: Option<&[f32]>, out: &mut [f32], out2: &mut [f32]) {
        assert_eq!(out.len(), out2.len());
        self.process_impl::<true>(input, None, out, out2);
    }

    /// Two inputs (gains `gain_*` and `gain2_*`), one output.
    pub fn process_two_inputs(&mut self, x1: &[f32], x2: &[f32], out: &mut [f32]) {
        self.process_impl::<false>(Some(x1), Some(x2), out, &mut []);
    }

    fn process_impl<const O2: bool>(&mut self, x1: Option<&[f32]>, x2: Option<&[f32]>, out: &mut [f32], out2: &mut [f32]) {
        let len = out.len();
        assert!(len <= self.acc.len(), "block larger than max_block");
        for x in [x1, x2].into_iter().flatten() {
            assert_eq!(x.len(), len);
        }
        assert!(x1.is_some() || x2.is_none());
        let acc = &mut self.acc[..len];
        let acc2 = &mut self.acc2[..if O2 { len } else { 0 }];
        let chunks = &mut self.chunks[..self.n_modes.div_ceil(LANES)];
        // A driven bank holds 8–12 arrays per chunk: beyond 2 chunks per time
        // loop the kernel spills (≈ 20× slower, measured on all ISAs), and a
        // driven bank with a second input or output needs one chunk on AVX2.
        let heavy = x2.is_some() || O2;
        #[cfg_attr(target_arch = "aarch64", allow(unused))]
        let g = match (x1.is_some(), heavy, self.isa) {
            #[cfg(target_arch = "x86_64")]
            (true, true, Isa::Avx2) => 1,
            (true, _, _) => self.group.min(2),
            (false, true, _) => self.group.min(2),
            (false, false, _) => self.group,
        };
        let (a, b) = (x1.unwrap_or(&[]), x2.unwrap_or(&[]));
        acc.fill([0.0; LANES]);
        acc2.fill([0.0; LANES]);
        #[cfg_attr(target_arch = "aarch64", allow(unused_macros))]
        macro_rules! go {
            ($run:ident) => {
                match (x1.is_some(), x2.is_some()) {
                    (false, _) => $run::<false, false, O2>(chunks, a, b, acc, acc2, g),
                    (true, false) => $run::<true, false, O2>(chunks, a, b, acc, acc2, g),
                    (true, true) => $run::<true, true, O2>(chunks, a, b, acc, acc2, g),
                }
            };
        }
        match self.isa {
            // aarch64: explicit NEON, two chunks per time loop.
            #[cfg(target_arch = "aarch64")]
            Isa::Generic => match (x1.is_some(), x2.is_some()) {
                (false, _) => run_v::<crate::simd::arm::N, false, false, O2>(chunks, a, b, acc, acc2),
                (true, false) => run_v::<crate::simd::arm::N, true, false, O2>(chunks, a, b, acc, acc2),
                (true, true) => run_v::<crate::simd::arm::N, true, true, O2>(chunks, a, b, acc, acc2),
            },
            #[cfg(not(target_arch = "aarch64"))]
            Isa::Generic => go!(run),
            // SAFETY: these variants are only selectable when detected.
            #[cfg(target_arch = "x86_64")]
            Isa::Avx2 => unsafe { go!(run_avx2) },
            #[cfg(target_arch = "x86_64")]
            Isa::Avx512 => unsafe { go!(run_avx512) },
        }
        for (o, a) in out.iter_mut().zip(acc.iter()) {
            *o += a.iter().sum::<f32>();
        }
        if O2 {
            for (o, a) in out2.iter_mut().zip(acc2.iter()) {
                *o += a.iter().sum::<f32>();
            }
        }
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
fn damp_chunks_avx2(chunks: &mut [Chunk], pr0: &[f32], pi0: &[f32], dw: &[f32], d: f32) {
    damp_chunks(chunks, pr0, pi0, dw, d)
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx512f")]
fn damp_chunks_avx512(chunks: &mut [Chunk], pr0: &[f32], pi0: &[f32], dw: &[f32], d: f32) {
    damp_chunks(chunks, pr0, pi0, dw, d)
}

/// Damped poles of whole chunks: p = e^{−d·dw}·p0 (same result on every ISA).
#[inline(always)]
fn damp_chunks(chunks: &mut [Chunk], pr0: &[f32], pi0: &[f32], dw: &[f32], d: f32) {
    let (p0, q0, w) = (pr0.as_chunks::<LANES>().0, pi0.as_chunks::<LANES>().0, dw.as_chunks::<LANES>().0);
    for (((c, p0), q0), w) in chunks.iter_mut().zip(p0).zip(q0).zip(w) {
        if d == 0.0 {
            c.pr = *p0;
            c.pi = *q0;
            continue;
        }
        let mut r = [0.0f32; LANES];
        for l in 0..LANES {
            r[l] = exp_neg(d * w[l]);
        }
        for l in 0..LANES {
            c.pr[l] = r[l] * p0[l];
            c.pi[l] = r[l] * q0[l];
        }
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
fn run_avx2<const IN: bool, const IN2: bool, const O2: bool>(
    chunks: &mut [Chunk],
    x: &[f32],
    x2: &[f32],
    acc: &mut [[f32; LANES]],
    acc2: &mut [[f32; LANES]],
    g: usize,
) {
    // Driven banks: explicit 8-lane halves, two chunks per time loop (the
    // generic code spills there and runs one chunk, latency-bound).
    if IN {
        run_v::<crate::simd::x86::Y, IN, IN2, O2>(chunks, x, x2, acc, acc2)
    } else {
        run::<IN, IN2, O2>(chunks, x, x2, acc, acc2, g)
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx512f")]
fn run_avx512<const IN: bool, const IN2: bool, const O2: bool>(
    chunks: &mut [Chunk],
    x: &[f32],
    x2: &[f32],
    acc: &mut [[f32; LANES]],
    acc2: &mut [[f32; LANES]],
    g: usize,
) {
    run::<IN, IN2, O2>(chunks, x, x2, acc, acc2, g)
}

#[cfg_attr(target_arch = "aarch64", allow(dead_code))]
#[inline(always)]
fn run<const IN: bool, const IN2: bool, const O2: bool>(
    chunks: &mut [Chunk],
    x: &[f32],
    x2: &[f32],
    acc: &mut [[f32; LANES]],
    acc2: &mut [[f32; LANES]],
    g: usize,
) {
    let rest = match g {
        4 => {
            let (grps, rest) = chunks.as_chunks_mut::<4>();
            for grp in grps {
                group::<4, IN, IN2, O2>(grp, x, x2, acc, acc2);
            }
            rest
        }
        2 => {
            let (grps, rest) = chunks.as_chunks_mut::<2>();
            for grp in grps {
                group::<2, IN, IN2, O2>(grp, x, x2, acc, acc2);
            }
            rest
        }
        _ => chunks,
    };
    for grp in rest.as_chunks_mut::<1>().0 {
        group::<1, IN, IN2, O2>(grp, x, x2, acc, acc2);
    }
}

/// [`run`] with explicit 16-lane vectors, two chunks per time loop; the
/// same operations in the same order (bit-identical results).
#[inline(always)]
fn run_v<V: V16, const IN: bool, const IN2: bool, const O2: bool>(
    chunks: &mut [Chunk],
    x: &[f32],
    x2: &[f32],
    acc: &mut [[f32; LANES]],
    acc2: &mut [[f32; LANES]],
) {
    let (grps, rest) = chunks.as_chunks_mut::<2>();
    for grp in grps {
        group_v::<V, 2, IN, IN2, O2>(grp, x, x2, acc, acc2);
    }
    for grp in rest.as_chunks_mut::<1>().0 {
        group_v::<V, 1, IN, IN2, O2>(grp, x, x2, acc, acc2);
    }
}

/// [`group`] with the states in registers and the coefficients read from
/// the chunks at every sample (loads are cheap; registers are not).
#[inline(always)]
fn group_v<V: V16, const G: usize, const IN: bool, const IN2: bool, const O2: bool>(
    grp: &mut [Chunk; G],
    x: &[f32],
    x2: &[f32],
    acc: &mut [[f32; LANES]],
    acc2: &mut [[f32; LANES]],
) {
    let (mut zr, mut zi) = ([V::zero(); G], [V::zero(); G]);
    for g in 0..G {
        zr[g] = V::ld(&grp[g].zr);
        zi[g] = V::ld(&grp[g].zi);
    }
    for n in 0..acc.len() {
        let mut s = V::ld(&acc[n]);
        let mut s2 = if O2 { V::ld(&acc2[n]) } else { V::zero() };
        let xn = V::splat(if IN { x[n] } else { 0.0 });
        let xn2 = V::splat(if IN2 { x2[n] } else { 0.0 });
        for g in 0..G {
            let c = &grp[g];
            let (pr, pi) = (V::ld(&c.pr), V::ld(&c.pi));
            let mut r = pr.mul(zr[g]).sub(pi.mul(zi[g]));
            let mut i = pr.mul(zi[g]).add(pi.mul(zr[g]));
            if IN {
                r = r.add(V::ld(&c.br).mul(xn));
                i = i.add(V::ld(&c.bi).mul(xn));
            }
            if IN2 {
                r = r.add(V::ld(&c.b2r).mul(xn2));
                i = i.add(V::ld(&c.b2i).mul(xn2));
            }
            zr[g] = r;
            zi[g] = i;
            s = s.add(V::ld(&c.or).mul(r).sub(V::ld(&c.oi).mul(i)));
            if O2 {
                s2 = s2.add(V::ld(&c.o2r).mul(r).sub(V::ld(&c.o2i).mul(i)));
            }
        }
        s.st(&mut acc[n]);
        if O2 {
            s2.st(&mut acc2[n]);
        }
    }
    for g in 0..G {
        zr[g].st(&mut grp[g].zr);
        zi[g].st(&mut grp[g].zi);
    }
}

#[cfg_attr(target_arch = "aarch64", allow(dead_code))]
#[inline(always)]
fn group<const G: usize, const IN: bool, const IN2: bool, const O2: bool>(
    grp: &mut [Chunk; G],
    x: &[f32],
    x2: &[f32],
    acc: &mut [[f32; LANES]],
    acc2: &mut [[f32; LANES]],
) {
    let pr: [[f32; LANES]; G] = std::array::from_fn(|g| grp[g].pr);
    let pi: [[f32; LANES]; G] = std::array::from_fn(|g| grp[g].pi);
    let br: [[f32; LANES]; G] = std::array::from_fn(|g| grp[g].br);
    let bi: [[f32; LANES]; G] = std::array::from_fn(|g| grp[g].bi);
    let b2r: [[f32; LANES]; G] = std::array::from_fn(|g| grp[g].b2r);
    let b2i: [[f32; LANES]; G] = std::array::from_fn(|g| grp[g].b2i);
    let or: [[f32; LANES]; G] = std::array::from_fn(|g| grp[g].or);
    let oi: [[f32; LANES]; G] = std::array::from_fn(|g| grp[g].oi);
    let o2r: [[f32; LANES]; G] = std::array::from_fn(|g| grp[g].o2r);
    let o2i: [[f32; LANES]; G] = std::array::from_fn(|g| grp[g].o2i);
    let mut zr: [[f32; LANES]; G] = std::array::from_fn(|g| grp[g].zr);
    let mut zi: [[f32; LANES]; G] = std::array::from_fn(|g| grp[g].zi);

    let mut step = |a: &mut [f32; LANES], a2: &mut [f32; LANES], xn: f32, xn2: f32| {
        let mut s = *a;
        let mut s2 = *a2;
        for g in 0..G {
            for l in 0..LANES {
                let mut r = pr[g][l] * zr[g][l] - pi[g][l] * zi[g][l];
                let mut i = pr[g][l] * zi[g][l] + pi[g][l] * zr[g][l];
                if IN {
                    r += br[g][l] * xn;
                    i += bi[g][l] * xn;
                }
                if IN2 {
                    r += b2r[g][l] * xn2;
                    i += b2i[g][l] * xn2;
                }
                zr[g][l] = r;
                zi[g][l] = i;
                s[l] += or[g][l] * r - oi[g][l] * i;
                if O2 {
                    s2[l] += o2r[g][l] * r - o2i[g][l] * i;
                }
            }
        }
        *a = s;
        if O2 {
            *a2 = s2;
        }
    };
    let mut dummy = [0.0f32; LANES];
    for (n, a) in acc.iter_mut().enumerate() {
        let xn = if IN { x[n] } else { 0.0 };
        let xn2 = if IN2 { x2[n] } else { 0.0 };
        if O2 {
            step(a, &mut acc2[n], xn, xn2);
        } else {
            step(a, &mut dummy, xn, xn2);
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
        b.set_modes((1..=150).map(|k| ModeSpec { damp_w: 0.2 * k as f64, ..ModeSpec::driven(27.5 * k as f64, 0.5 + 0.01 * k as f64, 1e-3) }));
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
                for (i, blk) in out.chunks_mut(128).enumerate() {
                    b.set_damping(0.7 * i as f32);
                    if i % 2 == 0 {
                        b.process(Some(&x), blk);
                    } else {
                        b.process_two_inputs(&x, &x, blk);
                    }
                }
                match &reference {
                    None => reference = Some(out),
                    Some(r) => assert!(r.iter().zip(&out).all(|(a, b)| a.to_bits() == b.to_bits()), "{isa:?} g={g}"),
                }
            }
        }
    }

    #[test]
    fn second_output_matches_a_separate_bank() {
        let specs: Vec<ModeSpec> = (1..=70)
            .map(|k| ModeSpec {
                freq_hz: 31.0 * k as f64,
                sigma: 1.0 + 0.1 * k as f64,
                gain_re: 1e-3,
                gain_im: -2e-4,
                out_re: 0.7,
                out_im: -0.2,
                out2_re: -0.3,
                out2_im: 0.9,
                ..Default::default()
            })
            .collect();
        let alt: Vec<ModeSpec> = specs.iter().map(|s| ModeSpec { out_re: s.out2_re, out_im: s.out2_im, ..*s }).collect();
        let x: Vec<f32> = (0..128).map(|i| ((i * 37) % 11) as f32 - 5.0).collect();
        for isa in Isa::available() {
            let bank = |sp: &[ModeSpec]| {
                let mut b = ModalBank::new(48_000.0, 70, 128);
                b.set_isa(isa);
                b.set_modes(sp.iter().copied());
                b
            };
            let (mut o1, mut o2, mut r1, mut r2) = (vec![0.0f32; 128], vec![0.0f32; 128], vec![0.0f32; 128], vec![0.0f32; 128]);
            bank(&specs).process2(Some(&x), &mut o1, &mut o2);
            bank(&specs).process(Some(&x), &mut r1);
            bank(&alt).process(Some(&x), &mut r2);
            for (p, q) in o1.iter().zip(&r1).chain(o2.iter().zip(&r2)) {
                assert!((p - q).abs() <= 1e-6 * (1.0 + q.abs()), "{isa:?}: {p} vs {q}");
            }
        }
    }

    #[test]
    fn two_inputs_superpose_and_damping_is_exact() {
        let spec = |g2: f64| ModeSpec { freq_hz: 440.0, sigma: 2.0, gain_re: 0.3, gain_im: -0.1, gain2_re: g2, gain2_im: 0.2 * g2, damp_w: 5.0, ..Default::default() };
        let x1: Vec<f32> = (0..64).map(|i| (i as f32 * 0.37).sin()).collect();
        let x2: Vec<f32> = (0..64).map(|i| (i as f32 * 0.11).cos()).collect();
        for isa in Isa::available() {
            let mk = |g2| {
                let mut b = ModalBank::new(48_000.0, 1, 64);
                b.set_isa(isa);
                b.set_modes([spec(g2)]);
                b
            };
            // Bank with both inputs = bank with x1 + bank with x2 (gain2 used as gain).
            let mut both = vec![0.0f32; 64];
            mk(0.7).process_two_inputs(&x1, &x2, &mut both);
            let mut a = vec![0.0f32; 64];
            mk(0.0).process(Some(&x1), &mut a);
            let mut only2 = ModalBank::new(48_000.0, 1, 64);
            only2.set_isa(isa);
            only2.set_modes([ModeSpec { gain_re: 0.7, gain_im: 0.14, ..spec(0.0) }]);
            only2.process(Some(&x2), &mut a);
            for (p, q) in both.iter().zip(&a) {
                assert!((p - q).abs() < 1e-5, "{isa:?}: {p} vs {q}");
            }
        }
        // set_damping(s) ≡ sigma + s·damp_w.
        let mut d = ModalBank::new(48_000.0, 1, 64);
        d.set_modes([spec(0.0)]);
        d.set_damping(3.0);
        let mut e = ModalBank::new(48_000.0, 1, 64);
        e.set_modes([ModeSpec { sigma: 2.0 + 15.0, ..spec(0.0) }]);
        let (mut o1, mut o2) = (vec![0.0f32; 64], vec![0.0f32; 64]);
        d.process(Some(&x1), &mut o1);
        e.process(Some(&x1), &mut o2);
        for (p, q) in o1.iter().zip(&o2) {
            assert!((p - q).abs() < 1e-5);
        }
    }

    #[test]
    fn exp_neg_is_accurate() {
        let mut worst = 0.0f64;
        for i in 0..200_000 {
            let x = (i as f32 * 1e-4).powi(2);
            let want = (-(x as f64)).exp();
            if want > 1e-30 {
                worst = worst.max(((exp_neg(x) as f64) - want).abs() / want);
            }
        }
        assert!(worst < 2.5e-7, "relative error {worst:e}");
    }

    #[test]
    fn loading_a_table_equals_setting_the_modes() {
        let specs: Vec<ModeSpec> =
            (1..=150).map(|k| ModeSpec { damp_w: 0.3 * k as f64, ..ModeSpec::driven(27.5 * k as f64, 0.5, 1e-3) }).collect();
        let table = ModeTable::new(48_000.0, &specs);
        let mut x = vec![0.0f32; 128];
        x[0] = 1.0;
        let (mut a, mut b) = (bank(Isa::Generic, 1), bank(Isa::Generic, 1));
        a.set_modes(specs.iter().copied());
        b.load(&table);
        for bk in [&mut a, &mut b] {
            bk.set_active_len(40);
            bk.set_damping(2.0);
            bk.set_active_len(150);
        }
        let (mut oa, mut ob) = (vec![0.0f32; 128], vec![0.0f32; 128]);
        a.process(Some(&x), &mut oa);
        b.process(Some(&x), &mut ob);
        assert!(oa.iter().zip(&ob).all(|(p, q)| p.to_bits() == q.to_bits()));
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
