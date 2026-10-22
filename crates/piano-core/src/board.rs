//! The soundboard as one multi-input, multi-output modal bank.
//!
//! Inputs: the bridge force of each of the [`REGIONS`] bridge regions.
//! Outputs: the pressure at the two microphones, and the bridge velocity at
//! each region point (its time derivative drives the strings
//! sympathetically: a moving end forces string mode k with −(μL/kπ)·ü).
//! Each board mode n is a complex one-pole driven by Σ_r Φ_n(x_r)·T·F_r and
//! read through complex weights: radiation for the mics, Φ_n(x_p) for the
//! velocity at point p (exact at all frequencies, unlike iω_n·Φ_n for the
//! acceleration, which only holds near each mode's resonance). One bank
//! replaces one bank per region.

// Dense per-lane numeric code reads better with explicit indices.
#![allow(clippy::needless_range_loop)]

use crate::modal::{Isa, LANES};

pub const REGIONS: usize = 16;
pub const MICS: usize = 2;
/// Outputs: the mics, then the bridge velocity at each region point.
pub const OUTPUTS: usize = MICS + REGIONS;

/// One board mode, in physical units.
#[derive(Clone, Copy, Debug)]
pub struct BoardMode {
    pub freq_hz: f64,
    pub sigma: f64,
    /// Input gain from each region (Φ_n(x_r)·T).
    pub inputs: [f64; REGIONS],
    /// Complex output weights: mics, then region displacements.
    pub outputs: [(f64, f64); OUTPUTS],
}

#[derive(Clone, Copy)]
#[repr(C, align(64))]
struct Chunk {
    pr: [f32; LANES],
    pi: [f32; LANES],
    phi: [[f32; LANES]; REGIONS],
    wr: [[f32; LANES]; OUTPUTS],
    wi: [[f32; LANES]; OUTPUTS],
    zr: [f32; LANES],
    zi: [f32; LANES],
}

impl Default for Chunk {
    fn default() -> Self {
        Self {
            pr: [0.0; LANES],
            pi: [0.0; LANES],
            phi: [[0.0; LANES]; REGIONS],
            wr: [[0.0; LANES]; OUTPUTS],
            wi: [[0.0; LANES]; OUTPUTS],
            zr: [0.0; LANES],
            zi: [0.0; LANES],
        }
    }
}

pub struct BoardBank {
    chunks: Vec<Chunk>,
    n_modes: usize,
    /// Lane accumulators, index `output·max_block + t`.
    acc: Vec<[f32; LANES]>,
    /// State history of one chunk over the block (two-pass kernel).
    hist: Vec<([f32; LANES], [f32; LANES])>,
    max_block: usize,
    /// Inputs transposed per sample, only the active regions.
    xin: Vec<[f32; REGIONS]>,
    /// Direct feedthrough correcting impulse invariance (index output, region):
    /// a force impulse makes the velocity jump; the sampled response counts
    /// the jump fully at n = 0 instead of half.
    direct: [[f32; REGIONS]; OUTPUTS],
    /// Extra outputs computed on the first `extra_chunks` chunks only (the
    /// low modes): weights `[c·n_extra + e]` (re, im) and accumulators.
    /// They carry no direct term: meant for displacement-like outputs,
    /// which do not jump on an impulse.
    n_extra: usize,
    extra_chunks: usize,
    extra_w: Vec<[[f32; LANES]; 2]>,
    extra_acc: Vec<[f32; LANES]>,
    isa: Isa,
    /// fs², for [`Self::energy`].
    fs2: f32,
}

impl BoardBank {
    pub fn new(sample_rate: f64, modes: &[BoardMode], max_block: usize) -> Self {
        Self::with_extra(sample_rate, modes, &[], max_block)
    }

    /// Bank with extra outputs: `extra[i]` holds the complex weights of mode
    /// `i` on each extra output (all the same length); modes past
    /// `extra.len()` do not contribute to them.
    pub fn with_extra(sample_rate: f64, modes: &[BoardMode], extra: &[Vec<(f64, f64)>], max_block: usize) -> Self {
        let t = 1.0 / sample_rate;
        let n_extra = extra.first().map_or(0, |e| e.len());
        let extra_chunks = if n_extra == 0 { 0 } else { extra.len().min(modes.len()).div_ceil(LANES) };
        let mut extra_w = vec![[[0.0f32; LANES]; 2]; extra_chunks * n_extra];
        for (i, ws) in extra.iter().take(modes.len()).enumerate() {
            for (e, w) in ws.iter().enumerate() {
                let cell = &mut extra_w[(i / LANES) * n_extra + e];
                cell[0][i % LANES] = w.0 as f32;
                cell[1][i % LANES] = w.1 as f32;
            }
        }
        let mut chunks = vec![Chunk::default(); modes.len().div_ceil(LANES)];
        let mut direct = [[0.0f64; REGIONS]; OUTPUTS];
        for m in modes {
            for (o, row) in direct.iter_mut().enumerate() {
                for (r, d) in row.iter_mut().enumerate() {
                    *d -= 0.5 * m.outputs[o].0 * m.inputs[r];
                }
            }
        }
        for (i, m) in modes.iter().enumerate() {
            let c = &mut chunks[i / LANES];
            let l = i % LANES;
            let r = (-m.sigma * t).exp();
            let (s, co) = (std::f64::consts::TAU * m.freq_hz * t).sin_cos();
            c.pr[l] = (r * co) as f32;
            c.pi[l] = (r * s) as f32;
            for (k, g) in m.inputs.iter().enumerate() {
                c.phi[k][l] = *g as f32;
            }
            for (o, w) in m.outputs.iter().enumerate() {
                c.wr[o][l] = w.0 as f32;
                c.wi[o][l] = w.1 as f32;
            }
        }
        Self {
            chunks,
            n_modes: modes.len(),
            acc: vec![[0.0; LANES]; OUTPUTS * max_block],
            hist: vec![([0.0; LANES], [0.0; LANES]); max_block],
            max_block,
            xin: vec![[0.0; REGIONS]; max_block],
            direct: direct.map(|row| row.map(|d| d as f32)),
            n_extra,
            extra_chunks,
            extra_w,
            extra_acc: vec![[0.0; LANES]; n_extra * max_block],
            // LLVM's AVX-512 code for this kernel is ≈ 10× slower than its
            // AVX2 code on Zen 5 (measured: 12.9 vs 1.3 ns/mode·sample).
            isa: match Isa::detect() {
                #[cfg(target_arch = "x86_64")]
                Isa::Avx512 => Isa::Avx2,
                isa => isa,
            },
            fs2: (sample_rate * sample_rate) as f32,
        }
    }

    pub fn len(&self) -> usize {
        self.n_modes
    }

    pub fn extra_outputs(&self) -> usize {
        self.n_extra
    }

    pub fn set_isa(&mut self, isa: Isa) {
        self.isa = isa;
    }

    pub fn is_empty(&self) -> bool {
        self.n_modes == 0
    }

    /// Output-weighted energy at the mics, to let the board sleep. The mic
    /// weights are acceleration weights ([`to_acceleration_weights`]): the
    /// differentiator's gain |1 − p|·fs at each mode restores the pressure.
    pub fn energy(&self) -> f32 {
        let mut e = 0.0;
        for c in &self.chunks {
            for l in 0..LANES {
                let w = c.wr[0][l] * c.wr[0][l] + c.wi[0][l] * c.wi[0][l];
                let d = (1.0 - c.pr[l]) * (1.0 - c.pr[l]) + c.pi[l] * c.pi[l];
                e += w * d * (c.zr[l] * c.zr[l] + c.zi[l] * c.zi[l]);
            }
        }
        e * self.fs2
    }

    /// Advances by `len` samples. `inputs[r][..len]` are the region forces;
    /// the outputs are *added* to `mics[o][..len]` and, when given,
    /// `disp[r][..len]` (bridge velocity at the region points) and
    /// `extra[e][..len]`.
    pub fn process(
        &mut self,
        inputs: &[Vec<f32>],
        len: usize,
        mics: [&mut [f32]; MICS],
        disp: Option<&mut [Vec<f32>]>,
        extra: Option<&mut [Vec<f32>]>,
    ) {
        assert!(len <= self.max_block && inputs.len() == REGIONS);
        // Only regions with input this block take part in the projection.
        let mut active = [0usize; REGIONS];
        let mut na = 0;
        for (r, x) in inputs.iter().enumerate() {
            if x[..len].iter().any(|&v| v != 0.0) {
                active[na] = r;
                na += 1;
            }
        }
        for (t, xi) in self.xin[..len].iter_mut().enumerate() {
            for (a, &r) in active[..na].iter().enumerate() {
                xi[a] = inputs[r][t];
            }
        }
        let (mb, acc, hist) = (self.max_block, &mut self.acc[..], &mut self.hist[..len]);
        let (chunks, xin) = (&mut self.chunks[..], &self.xin[..len]);
        let act = &active[..na];
        let n_ex = if extra.is_some() { self.n_extra } else { 0 };
        let (ex_w, ex_acc) = (&self.extra_w[..self.extra_chunks * n_ex], &mut self.extra_acc[..]);
        macro_rules! go {
            ($run:ident) => {
                if disp.is_some() {
                    $run::<OUTPUTS>(chunks, xin, act, acc, hist, mb, n_ex, ex_w, ex_acc)
                } else {
                    $run::<MICS>(chunks, xin, act, acc, hist, mb, n_ex, ex_w, ex_acc)
                }
            };
        }
        match self.isa {
            Isa::Generic => go!(run),
            // SAFETY: only selectable when detected.
            #[cfg(target_arch = "x86_64")]
            Isa::Avx2 => unsafe { go!(run_avx2) },
            #[cfg(target_arch = "x86_64")]
            Isa::Avx512 => unsafe { go!(run_avx512) },
        }
        let [m0, m1] = mics;
        let act = &active[..na];
        let sum = |o: usize, t: usize| {
            let feed: f32 = act.iter().enumerate().map(|(k, &r)| self.direct[o][r] * self.xin[t][k]).sum();
            hsum(&self.acc[o * mb + t]) + feed
        };
        for t in 0..len {
            m0[t] += sum(0, t);
            m1[t] += sum(1, t);
        }
        if let Some(disp) = disp {
            for (r, d) in disp.iter_mut().enumerate() {
                for (t, x) in d[..len].iter_mut().enumerate() {
                    *x += sum(MICS + r, t);
                }
            }
        }
        if let Some(extra) = extra {
            for (e, out) in extra.iter_mut().take(self.n_extra).enumerate() {
                for (x, a) in out[..len].iter_mut().zip(&self.extra_acc[e * mb..e * mb + len]) {
                    *x += hsum(a);
                }
            }
        }
    }
}

type Hist = ([f32; LANES], [f32; LANES]);

/// Extra outputs' weights of the low chunks, `[c·n_ex + e]`.
type ExtraW = [[f32; LANES]; 2];

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2,fma")]
#[allow(clippy::too_many_arguments)]
fn run_avx2<const NO: usize>(
    chunks: &mut [Chunk],
    xin: &[[f32; REGIONS]],
    act: &[usize],
    acc: &mut [[f32; LANES]],
    hist: &mut [Hist],
    mb: usize,
    n_ex: usize,
    ex_w: &[ExtraW],
    ex_acc: &mut [[f32; LANES]],
) {
    run::<NO>(chunks, xin, act, acc, hist, mb, n_ex, ex_w, ex_acc)
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx512f,avx512vl,avx512bw,avx512dq,fma")]
#[allow(clippy::too_many_arguments)]
fn run_avx512<const NO: usize>(
    chunks: &mut [Chunk],
    xin: &[[f32; REGIONS]],
    act: &[usize],
    acc: &mut [[f32; LANES]],
    hist: &mut [Hist],
    mb: usize,
    n_ex: usize,
    ex_w: &[ExtraW],
    ex_acc: &mut [[f32; LANES]],
) {
    run::<NO>(chunks, xin, act, acc, hist, mb, n_ex, ex_w, ex_acc)
}

/// Sum of the lanes as a tree (independent adds, not a serial chain).
#[inline(always)]
fn hsum(a: &[f32; LANES]) -> f32 {
    let mut v = *a;
    let mut w = LANES / 2;
    while w > 0 {
        for l in 0..w {
            v[l] += v[l + w];
        }
        w /= 2;
    }
    v[0]
}

#[inline(always)]
fn accumulate(acc: &mut [[f32; LANES]], hist: &[Hist], wr: &[f32; LANES], wi: &[f32; LANES]) {
    for (a, h) in acc.iter_mut().zip(hist) {
        for l in 0..LANES {
            a[l] += wr[l] * h.0[l] - wi[l] * h.1[l];
        }
    }
}

/// Two passes per chunk of 16 modes: the recursion over the block (states
/// kept in an L1-resident history), then each output streamed over time.
/// A single pass with 18 lane accumulators per sample spills (≈ 30× slower).
#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn run<const NO: usize>(
    chunks: &mut [Chunk],
    xin: &[[f32; REGIONS]],
    act: &[usize],
    acc: &mut [[f32; LANES]],
    hist: &mut [Hist],
    mb: usize,
    n_ex: usize,
    ex_w: &[ExtraW],
    ex_acc: &mut [[f32; LANES]],
) {
    let len = hist.len();
    for o in 0..NO {
        acc[o * mb..o * mb + len].fill([0.0; LANES]);
    }
    for e in 0..n_ex {
        ex_acc[e * mb..e * mb + len].fill([0.0; LANES]);
    }
    let ex_chunks = ex_w.len().checked_div(n_ex).unwrap_or(0);
    for (ci, c) in chunks.iter_mut().enumerate() {
        let (pr, pi) = (c.pr, c.pi);
        let (mut zr, mut zi) = (c.zr, c.zi);
        for (x, h) in xin.iter().zip(hist.iter_mut()) {
            let mut u = [0.0f32; LANES];
            for (k, &r) in act.iter().enumerate() {
                let xr = x[k];
                for l in 0..LANES {
                    u[l] += c.phi[r][l] * xr;
                }
            }
            for l in 0..LANES {
                let r = pr[l] * zr[l] - pi[l] * zi[l] + u[l];
                let i = pr[l] * zi[l] + pi[l] * zr[l];
                zr[l] = r;
                zi[l] = i;
            }
            *h = (zr, zi);
        }
        c.zr = zr;
        c.zi = zi;
        for o in 0..NO {
            accumulate(&mut acc[o * mb..o * mb + len], hist, &c.wr[o], &c.wi[o]);
        }
        if ci < ex_chunks {
            for (e, [wr, wi]) in ex_w[ci * n_ex..(ci + 1) * n_ex].iter().enumerate() {
                accumulate(&mut ex_acc[e * mb..e * mb + len], hist, wr, wi);
            }
        }
    }
}

/// Transfer at angular frequency ω (rad/s) from region input `r` to output
/// `o`, as the bank renders a real sinusoid: x(t) = cos ωt gives
/// y(t) = Re(H·e^{iωt}). Includes the counter-rotating part of each pole.
pub fn transfer(modes: &[BoardMode], r: usize, o: usize, omega: f64, sample_rate: f64) -> crate::complex::C64 {
    use crate::complex::C64;
    let t = 1.0 / sample_rate;
    let e_neg = C64::polar(1.0, -omega * t);
    let e_pos = C64::polar(1.0, omega * t);
    let mut h = C64::ZERO;
    for m in modes {
        let p = C64::polar((-m.sigma * t).exp(), std::f64::consts::TAU * m.freq_hz * t);
        let b = C64::new(m.inputs[r], 0.0);
        let w = C64::new(m.outputs[o].0, m.outputs[o].1);
        let a = b / (C64::ONE - p * e_neg).scale(2.0);
        let bb = b / (C64::ONE - p * e_pos).scale(2.0);
        h += w * a + (w * bb).conj();
        // Same direct-term correction as the bank.
        h += C64::new(-0.5 * m.outputs[o].0 * m.inputs[r], 0.0);
    }
    h
}

/// Response of the differentiator the engine applies to the summed mic
/// outputs, y[n] = (x[n] − x[n−1])·fs, at ω (rad/s).
pub fn mic_differentiator(omega: f64, sample_rate: f64) -> crate::complex::C64 {
    (crate::complex::C64::ONE - crate::complex::C64::polar(1.0, -omega / sample_rate)).scale(sample_rate)
}

/// The radiated pressure follows the surface *acceleration* (Rayleigh
/// integral, p = ρ/2π·∫ v̇(t − R/c)/R dS), while the mic weights are
/// evaluated once per mode at its own frequency. Rendering them on the
/// modal velocity made every mode radiate at its resonance's rate even when
/// driven far below it: under a bass fundamental, the quasi-static motion
/// of the modes above radiated ×ω_n/ω too much (+20–30 dB below 50 Hz,
/// docs/bass_radiation.md). The weights are divided by the engine's
/// differentiator at the mode frequency: unchanged at resonance, ∝ ω below.
pub fn to_acceleration_weights(modes: &mut [BoardMode], sample_rate: f64) {
    for m in modes {
        let d = mic_differentiator(std::f64::consts::TAU * m.freq_hz, sample_rate);
        for o in &mut m.outputs[..MICS] {
            let w = crate::complex::C64::new(o.0, o.1) / d;
            *o = (w.re, w.im);
        }
    }
}

/// Transfer to a mic output as rendered (acceleration weights followed by
/// the engine's differentiator), in pressure per unit force.
pub fn mic_transfer(modes: &[BoardMode], r: usize, mic: usize, omega: f64, sample_rate: f64) -> crate::complex::C64 {
    transfer(modes, r, mic, omega, sample_rate) * mic_differentiator(omega, sample_rate)
}

/// Board modes for a soundboard: inputs through the bridge at the region
/// keys, outputs at the two mics (Rayleigh) and bridge velocity there.
pub fn board_modes(sb: &crate::soundboard::Soundboard, sample_rate: f64) -> Vec<BoardMode> {
    let shapes: Vec<Vec<f64>> = (0..REGIONS).map(|r| sb.bridge_shapes(region_center_note(r))).collect();
    board_modes_with(sb, &shapes, sample_rate)
}

/// Same as [`board_modes`] with point inputs at `points` (m).
pub fn board_modes_at(sb: &crate::soundboard::Soundboard, points: &[(f64, f64)], sample_rate: f64) -> Vec<BoardMode> {
    let shapes: Vec<Vec<f64>> =
        points.iter().map(|&(x, y)| sb.modes.iter().map(|md| sb.shape(md, x, y)).collect()).collect();
    board_modes_with(sb, &shapes, sample_rate)
}

/// Board modes with per-region input shapes `shapes[r][mode]`.
fn board_modes_with(sb: &crate::soundboard::Soundboard, shapes: &[Vec<f64>], sample_rate: f64) -> Vec<BoardMode> {
    use crate::complex::C64;
    let rad = sb.radiation();
    let nyq = 0.45 * sample_rate;
    let t = 1.0 / sample_rate;
    sb.modes
        .iter()
        .zip(&rad)
        .enumerate()
        .filter(|(_, (md, _))| md.omega / std::f64::consts::TAU < nyq)
        .map(|(i, (md, w))| {
            let mut outputs = [(0.0, 0.0); OUTPUTS];
            outputs[0] = (w[0].re, w[0].im);
            outputs[1] = (w[1].re, w[1].im);
            for (r, sh) in shapes.iter().enumerate() {
                outputs[MICS + r] = (sh[i], 0.0);
            }
            // Exact velocity impulse response of a damped mode,
            // e^{−σt}(cos ω_d t − σ/ω_d·sin ω_d t) = Re((1 + iσ/ω_d)·e^{(−σ+iω_d)t}):
            // the factor matters for the strongly damped high modes.
            let sigma = sb.sigma(md);
            let shape = C64::new(1.0, sigma / md.omega);
            for o in outputs.iter_mut() {
                let w = C64::new(o.0, o.1) * shape;
                *o = (w.re, w.im);
            }
            BoardMode {
                freq_hz: md.omega / std::f64::consts::TAU,
                sigma,
                inputs: std::array::from_fn(|r| shapes[r][i] * t),
                outputs,
            }
        })
        .collect()
}

/// Bridge region of a key.
pub fn region_of(note: u8) -> usize {
    ((note.clamp(21, 108) - 21) as usize * REGIONS / 88).min(REGIONS - 1)
}

/// Key at the middle of a region (its bridge point stands for the region).
pub fn region_center_note(r: usize) -> u8 {
    21 + ((2 * r + 1) * 88 / (2 * REGIONS)) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::soundboard::{Soundboard, SoundboardParams};

    /// The rendered velocity transfer matches the plate's own mobility Y at
    /// the driving point, and a rendered sine matches `transfer`.
    #[test]
    fn transfer_matches_mobility_and_rendering() {
        let sr = 48_000.0;
        let sb = Soundboard::new(SoundboardParams::GRAND_2M);
        let modes = board_modes(&sb, sr);
        let r = 7;
        let shapes = sb.bridge_shapes(region_center_note(r));
        for f in [80.0, 440.0, 2500.0] {
            let w = std::f64::consts::TAU * f;
            let h = transfer(&modes, r, MICS + r, w, sr);
            let want = sb.mobility(&shapes, w);
            // Impulse invariance departs from the continuous mobility for the
            // strongly damped high modes (η up to ≈ 0.6): in the 80 Hz
            // conductance valley the small real part is off by ≈ 2.4×, i.e.
            // ≈ 27 % of |Y| (the reactive part matches to 0.3 %).
            let tol = if f < 100.0 { 0.3 } else { 0.2 };
            assert!((h - want).norm() < tol * want.norm(), "f {f}: {h:?} vs {want:?}");
        }
        // Render a sine through the bank and compare its steady state.
        let f = 440.0;
        let mut bank = BoardBank::new(sr, &modes, 256);
        let mut inputs = vec![vec![0.0f32; 256]; REGIONS];
        let mut disp = vec![vec![0.0f32; 256]; REGIONS];
        let (mut m0, mut m1) = (vec![0.0f32; 256], vec![0.0f32; 256]);
        let h = transfer(&modes, r, MICS + r, std::f64::consts::TAU * f, sr);
        let mut err = 0.0f64;
        let mut refp = 0.0f64;
        for blk in 0..400 {
            for (t, x) in inputs[r].iter_mut().enumerate() {
                *x = (std::f64::consts::TAU * f * (blk * 256 + t) as f64 / sr).cos() as f32;
            }
            for d in disp.iter_mut() {
                d.fill(0.0);
            }
            bank.process(&inputs, 256, [&mut m0, &mut m1], Some(&mut disp), None);
            if blk >= 380 {
                for t in 0..256 {
                    let ph = std::f64::consts::TAU * f * (blk * 256 + t) as f64 / sr;
                    let want = (h * crate::complex::C64::polar(1.0, ph)).re;
                    err += (disp[r][t] as f64 - want).powi(2);
                    refp += want * want;
                }
            }
        }
        assert!(err / refp < 1e-3, "relative error {}", err / refp);
    }
}
