//! High-resolution analysis of a partial: subband ESPRIT.
//!
//! The band around a partial is cut out of one long FFT and brought to
//! baseband (critically sampled, a few tens of Hz wide), then modelled as a
//! sum of damped complex exponentials with ESPRIT (Roy & Kailath 1989), the
//! approach used for piano tones by Ege, Boutillon & Rébillat (2013) and
//! Badeau's group. This separates the 2–3 strings of a unison (beating) and
//! damping-only differences (double decay), which the FFT cannot.

use nalgebra::{Complex, DMatrix, DVector};
use realfft::RealFftPlanner;
use rustfft::FftPlanner;

pub type C = Complex<f64>;

/// Maximum baseband samples given to ESPRIT.
pub const MAX_SAMPLES: usize = 360;

/// One damped component of a partial.
#[derive(Clone, Copy, Debug)]
pub struct Mode {
    pub freq: f64,
    /// Amplitude decay rate (1/s); T60 = ln(1000)/σ.
    pub sigma: f64,
    /// Real amplitude extrapolated to the segment start.
    pub amp: f64,
    pub phase: f64,
}

impl Mode {
    pub fn t60(&self) -> f64 {
        6.907_755_278_982_137 / self.sigma
    }
}

/// Complex baseband signal of one band.
pub struct Subband {
    pub y: Vec<C>,
    /// Sampling rate of `y`.
    pub fs: f64,
    /// Demodulation frequency.
    pub f_center: f64,
    /// Factor converting |y| into the real amplitude of a sinusoid.
    pub amp_scale: f64,
}

/// Spectrum of one long segment, from which subbands are cut.
pub struct SegmentSpectrum {
    bins: Vec<C>,
    n: usize,
    pub bin_hz: f64,
}

impl SegmentSpectrum {
    /// FFT of `x[start..start+len]`, with a short fade-out so the circular
    /// wrap of the band filters does not bring the tail back at t = 0.
    pub fn new(x: &[f32], sr: f64, start: usize, len: usize) -> Self {
        let seg = &x[start.min(x.len())..(start + len).min(x.len())];
        let n = seg.len();
        let fade = n / 20;
        let mut buf: Vec<f64> = seg.iter().map(|&v| v as f64).collect();
        for i in 0..fade {
            let w = 0.5 - 0.5 * (std::f64::consts::PI * i as f64 / fade as f64).cos();
            buf[n - 1 - i] *= w;
        }
        let fft = RealFftPlanner::<f64>::new().plan_fft_forward(n);
        let mut spec = fft.make_output_vec();
        fft.process(&mut buf, &mut spec).unwrap();
        Self { bins: spec, n, bin_hz: sr / n as f64 }
    }

    pub fn duration(&self) -> f64 {
        1.0 / self.bin_hz
    }

    /// Band [fc − half_bw, fc + half_bw] in baseband. The outer 20 % of each
    /// side is tapered to limit time-domain ringing.
    pub fn subband(&self, fc: f64, half_bw: f64) -> Subband {
        let c0 = (fc / self.bin_hz).round() as isize;
        let h = (half_bw / self.bin_hz).ceil().max(2.0) as isize;
        let nb = (2 * h + 1) as usize;
        let mut buf = vec![C::new(0.0, 0.0); nb];
        for j in -h..=h {
            let idx = c0 + j;
            if idx < 0 || idx as usize >= self.bins.len() {
                continue;
            }
            let u = j.unsigned_abs() as f64 / h as f64;
            let taper = if u <= 0.8 { 1.0 } else { 0.5 + 0.5 * (std::f64::consts::PI * (u - 0.8) / 0.2).cos() };
            buf[j.rem_euclid(nb as isize) as usize] = self.bins[idx as usize] * taper;
        }
        FftPlanner::new().plan_fft_inverse(nb).process(&mut buf);
        Subband { y: buf, fs: nb as f64 * self.bin_hz, f_center: c0 as f64 * self.bin_hz, amp_scale: 2.0 / self.n as f64 }
    }
}

/// ESPRIT: returns (pole z, complex amplitude a) with y[n] ≈ Σ a·zⁿ.
/// The order is the number of singular values above both `rel_db` below the
/// largest and 4× the noise level (median of the tail), at most `max_order`.
pub fn esprit(y: &[C], max_order: usize, rel_db: f64) -> Vec<(C, C)> {
    let n = y.len();
    let m = (n / 3).max(max_order + 2);
    if n < m + max_order + 2 {
        return Vec::new();
    }
    let l = n - m + 1;
    let h = DMatrix::from_fn(m, l, |i, j| y[i + j]);
    let svd = h.svd(true, false);
    let u = svd.u.expect("U requested");
    let mut order: Vec<usize> = (0..svd.singular_values.len()).collect();
    order.sort_by(|&a, &b| svd.singular_values[b].total_cmp(&svd.singular_values[a]));
    let s: Vec<f64> = order.iter().map(|&i| svd.singular_values[i]).collect();
    let tail = &s[(2 * max_order).min(s.len() - 1)..];
    let mut t = tail.to_vec();
    t.sort_by(f64::total_cmp);
    let noise = t[t.len() / 2];
    let thr = (s[0] * 10f64.powf(-rel_db / 20.0)).max(4.0 * noise);
    let r = s.iter().take_while(|&&v| v > thr).count().clamp(1, max_order);

    let us = DMatrix::from_fn(m, r, |i, k| u[(i, order[k])]);
    let u1 = us.rows(0, m - 1).into_owned();
    let u2 = us.rows(1, m - 1).into_owned();
    let Ok(pinv) = u1.pseudo_inverse(1e-12) else { return Vec::new() };
    let phi = pinv * u2;
    let Some(z) = phi.schur().eigenvalues() else { return Vec::new() };

    // Amplitudes by least squares on the Vandermonde system.
    let v = DMatrix::from_fn(n, r, |i, k| z[k].powu(i as u32));
    let yv = DVector::from_column_slice(y);
    let Ok(a) = v.svd(true, true).solve(&yv, 1e-12) else { return Vec::new() };
    z.iter().zip(a.iter()).map(|(&z, &a)| (z, a)).collect()
}

#[derive(Clone, Copy, Debug)]
pub struct Options {
    /// Half-width of the band: min(frac·spacing, max(min_hz, cents_frac·f)).
    pub band_frac_of_spacing: f64,
    pub band_min_hz: f64,
    pub band_rel: f64,
    pub max_order: usize,
    pub rel_db: f64,
    /// Components weaker than this (dB below the strongest) are dropped.
    pub keep_db: f64,
}

impl Default for Options {
    fn default() -> Self {
        Self { band_frac_of_spacing: 0.4, band_min_hz: 5.0, band_rel: 0.004, max_order: 6, rel_db: 45.0, keep_db: 40.0 }
    }
}

/// Components of the partial near `freq`, analysed from `skip` to
/// `skip + dur` seconds after the segment start.
pub fn analyse_partial(
    spec: &SegmentSpectrum,
    freq: f64,
    spacing: f64,
    skip: f64,
    dur: f64,
    opt: Options,
) -> Vec<Mode> {
    let hb = (opt.band_frac_of_spacing * spacing).min((opt.band_rel * freq).max(opt.band_min_hz));
    let sb = spec.subband(freq, hb);
    // The band filter smears the abrupt segment start over ~1/hb.
    let skip = skip.max(1.5 / hb);
    let i0 = (skip * sb.fs).ceil() as usize;
    // The Hankel SVD is O(N³): beyond a few hundred baseband samples the
    // resolution gain is nil and the cost explodes.
    let i1 = (((skip + dur) * sb.fs) as usize).min(i0 + MAX_SAMPLES);
    if i1 > sb.y.len() || i1 <= i0 {
        return Vec::new();
    }
    let t0 = i0 as f64 / sb.fs;
    let mut modes: Vec<Mode> = esprit(&sb.y[i0..i1], opt.max_order, opt.rel_db)
        .into_iter()
        .filter_map(|(z, a)| {
            let sigma = -z.norm().ln() * sb.fs;
            let freq = sb.f_center + z.arg() * sb.fs / std::f64::consts::TAU;
            // Growing or out-of-band poles are noise or filter artefacts.
            if sigma <= 0.0 || (freq - sb.f_center).abs() > 0.75 * hb {
                return None;
            }
            // Back to the segment start (t = 0), undoing the demodulation phase.
            let amp = a.norm() * sb.amp_scale * (sigma * t0).exp();
            let phase = a.arg() - std::f64::consts::TAU * (freq - sb.f_center) * t0;
            Some(Mode { freq, sigma, amp, phase })
        })
        .collect();
    modes.sort_by(|a, b| b.amp.total_cmp(&a.amp));
    if let Some(top) = modes.first().map(|m| m.amp) {
        let floor = top * 10f64.powf(-opt.keep_db / 20.0);
        modes.retain(|m| m.amp >= floor);
    }
    modes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synth::{render, Partial};

    fn p(freq: f64, amp: f64, sigma: f64, phase: f64) -> Partial {
        Partial { freq, amp, sigma, slow: None, phase }
    }

    fn check(found: &[Mode], want: &[(f64, f64, f64)]) {
        assert_eq!(found.len(), want.len(), "{found:#?}");
        for &(f, a, s) in want {
            let m = found
                .iter()
                .min_by(|x, y| {
                    let d = |m: &Mode| (m.freq - f).abs() + (m.sigma / s).ln().abs();
                    d(x).total_cmp(&d(y))
                })
                .unwrap();
            assert!((m.freq - f).abs() < 0.01, "freq {} vs {f}: {found:#?}", m.freq);
            assert!((m.sigma - s).abs() / s < 0.05, "sigma {} vs {s}: {found:#?}", m.sigma);
            assert!((20.0 * (m.amp / a).log10()).abs() < 0.5, "amp {} vs {a}: {found:#?}", m.amp);
        }
    }

    #[test]
    fn subband_esprit_single() {
        let sr = 48_000.0;
        let x = render(&[p(261.7, 0.5, 1.2, 0.3), p(523.9, 0.3, 2.0, 0.0)], sr, 6.0, 1e-5, 3);
        let spec = SegmentSpectrum::new(&x, sr, 0, x.len());
        let m = analyse_partial(&spec, 261.7, 261.7, 0.1, 4.0, Options::default());
        check(&m, &[(261.7, 0.5, 1.2)]);
    }

    #[test]
    fn separates_bichord_beating() {
        // Two strings 0.6 Hz apart (≈ 4 cents at 220 Hz), different decays.
        let sr = 48_000.0;
        let x = render(
            &[p(220.0, 0.4, 1.0, 0.0), p(220.6, 0.25, 0.4, 1.0), p(441.0, 0.3, 1.5, 0.5)],
            sr,
            8.0,
            1e-5,
            4,
        );
        let spec = SegmentSpectrum::new(&x, sr, 0, x.len());
        let m = analyse_partial(&spec, 220.3, 220.0, 0.1, 6.0, Options::default());
        check(&m, &[(220.0, 0.4, 1.0), (220.6, 0.25, 0.4)]);
    }

    #[test]
    fn separates_trichord() {
        let sr = 48_000.0;
        let x = render(
            &[p(440.0, 0.3, 2.0, 0.0), p(440.45, 0.2, 1.0, 2.0), p(440.9, 0.1, 0.5, 4.0)],
            sr,
            8.0,
            1e-5,
            5,
        );
        let spec = SegmentSpectrum::new(&x, sr, 0, x.len());
        let m = analyse_partial(&spec, 440.4, 440.0, 0.1, 6.0, Options::default());
        check(&m, &[(440.0, 0.3, 2.0), (440.45, 0.2, 1.0), (440.9, 0.1, 0.5)]);
    }

    #[test]
    fn separates_same_frequency_double_decay() {
        // Same frequency, two decay rates: only the damping differs.
        let sr = 48_000.0;
        let x = render(
            &[Partial { freq: 330.0, amp: 0.5, sigma: 3.0, slow: Some((0.02, 0.3)), phase: 0.0 }],
            sr,
            10.0,
            1e-6,
            6,
        );
        let spec = SegmentSpectrum::new(&x, sr, 0, x.len());
        let m = analyse_partial(&spec, 330.0, 330.0, 0.1, 8.0, Options::default());
        check(&m, &[(330.0, 0.5, 3.0), (330.0, 0.02, 0.3)]);
    }
}
