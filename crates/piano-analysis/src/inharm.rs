//! Partial tracking and inharmonicity estimation for a single note.
//!
//! Partials are searched one by one around the prediction
//! f_k = k·f0·√(1+B·k²) of the current fit, and (f0, B) is refitted by
//! least squares on (f_k/k)² = f0² + f0²·B·k² after each new partial, in the
//! spirit of Rauhala et al. (2007) and Rigaud et al. (2013).

use crate::spectrum::Spectrum;

#[derive(Clone, Copy, Debug)]
pub struct Options {
    /// Analysis segment start (s) after the onset, to skip the attack.
    pub skip: f64,
    /// Segment length (s).
    pub len: f64,
    /// Highest frequency examined.
    pub fmax: f64,
    /// Minimum peak level above the local median, dB.
    pub min_snr_db: f64,
    pub max_partials: u32,
}

impl Default for Options {
    fn default() -> Self {
        Self { skip: 0.05, len: 1.0, fmax: 20_000.0, min_snr_db: 20.0, max_partials: 120 }
    }
}

#[derive(Clone, Debug)]
pub struct TrackedPartial {
    pub k: u32,
    pub freq: f64,
    pub level_db: f64,
}

#[derive(Clone, Debug)]
pub struct InharmonicityFit {
    pub f0: f64,
    pub b: f64,
    pub partials: Vec<TrackedPartial>,
    pub onset: usize,
}

impl InharmonicityFit {
    pub fn predict(&self, k: u32) -> f64 {
        let k = k as f64;
        k * self.f0 * (1.0 + self.b * k * k).sqrt()
    }
}

/// First sample whose magnitude reaches 10% of the maximum.
pub fn onset(x: &[f32]) -> usize {
    let peak = x.iter().fold(0.0f32, |m, &v| m.max(v.abs()));
    x.iter().position(|&v| v.abs() >= 0.1 * peak).unwrap_or(0)
}

pub fn estimate(x: &[f32], sr: f64, f0_nominal: f64, opt: Options) -> Option<InharmonicityFit> {
    let on = onset(x);
    let start = on + (opt.skip * sr) as usize;
    // Treble partials die fast: shorten the segment, keeping ≥ 300 periods.
    let len = opt.len.min((300.0 / f0_nominal).max(0.15));
    let spec = Spectrum::new(x, sr, start, (len * sr) as usize, 8);
    estimate_spectrum(spec, on, sr, f0_nominal, opt)
}

/// Shared onset from RMS pressure across channels. This signal is used
/// only for timing, never as input to a Fourier transform.
pub fn onset_channels(channels: &[&[f32]]) -> usize {
    assert!(!channels.is_empty());
    assert!(channels.iter().all(|c| c.len() == channels[0].len()));
    let powers: Vec<f64> = (0..channels[0].len()).map(|i| channels.iter().map(|c| (c[i] as f64).powi(2)).sum::<f64>()).collect();
    let peak = powers.iter().copied().fold(0.0, f64::max);
    powers.iter().position(|p| *p >= 0.01 * peak).unwrap_or(0)
}

pub fn estimate_channels(channels: &[&[f32]], sr: f64, f0_nominal: f64, opt: Options) -> Option<InharmonicityFit> {
    let on = onset_channels(channels);
    let start = on + (opt.skip * sr) as usize;
    let len = opt.len.min((300.0 / f0_nominal).max(0.15));
    let spec = Spectrum::channel_power(channels, sr, start, (len * sr) as usize, 8);
    estimate_spectrum(spec, on, sr, f0_nominal, opt)
}

fn estimate_spectrum(spec: Spectrum, on: usize, sr: f64, f0_nominal: f64, opt: Options) -> Option<InharmonicityFit> {
    let fmax = opt.fmax.min(0.95 * sr / 2.0);

    let (mut f0, mut b) = (f0_nominal, 0.0);
    let mut found: Vec<TrackedPartial> = Vec::new();
    let mut misses = 0;
    for k in 1..=opt.max_partials {
        let kf = k as f64;
        let pred = kf * f0 * (1.0 + b * kf * kf).sqrt();
        if pred > fmax {
            break;
        }
        // Local spacing d f_k / dk, the search half-width is a fraction of it.
        let spacing = f0 * (1.0 + 2.0 * b * kf * kf) / (1.0 + b * kf * kf).sqrt();
        // Before B is known, the uncertainty grows with k.
        let half = if found.len() < 4 { 0.35 * spacing } else { 0.2 * spacing };
        let accepted = spec
            .peak_in(pred - half, pred + half)
            .filter(|&(_, lvl)| lvl - spec.median_in(pred - spacing, pred + spacing) >= opt.min_snr_db);
        match accepted {
            Some((f, lvl)) => {
                misses = 0;
                found.push(TrackedPartial { k, freq: f, level_db: lvl });
                match fit(&found) {
                    Some((nf0, nb)) if nb >= 0.0 => (f0, b) = (nf0, nb),
                    _ => f0 = found[0].freq / found[0].k as f64,
                }
            }
            None => {
                misses += 1;
                if misses > 6 {
                    break;
                }
            }
        }
    }
    // Two partials still give (f0, B), poorly conditioned: high treble only.
    if found.len() < 2 {
        return None;
    }
    // One pass of outlier rejection on the frequency residual.
    for _ in 0..2 {
        let res: Vec<f64> = found
            .iter()
            .map(|p| {
                let kf = p.k as f64;
                1200.0 * (p.freq / (kf * f0 * (1.0 + b * kf * kf).sqrt())).log2()
            })
            .collect();
        let mut abs: Vec<f64> = res.iter().map(|r| r.abs()).collect();
        abs.sort_by(f64::total_cmp);
        // Bridge coupling shifts partials by a few cents: do not reject that.
        let mad = abs[abs.len() / 2].max(0.5);
        let keep: Vec<TrackedPartial> = found.iter().zip(&res).filter(|(_, r)| r.abs() <= 4.0 * mad).map(|(p, _)| p.clone()).collect();
        if keep.len() >= 2 {
            found = keep;
        }
        (f0, b) = fit(&found)?;
    }
    Some(InharmonicityFit { f0, b, partials: found, onset: on })
}

/// Least squares of y = (f/k)² against x = k²: y = a + s·x, f0 = √a, B = s/a.
fn fit(p: &[TrackedPartial]) -> Option<(f64, f64)> {
    if p.is_empty() {
        return None;
    }
    if p.len() == 1 {
        return Some((p[0].freq / p[0].k as f64, 0.0));
    }
    let n = p.len() as f64;
    let (mut sx, mut sy, mut sxx, mut sxy) = (0.0, 0.0, 0.0, 0.0);
    for q in p {
        let x = (q.k * q.k) as f64;
        let y = (q.freq / q.k as f64).powi(2);
        sx += x;
        sy += y;
        sxx += x * x;
        sxy += x * y;
    }
    let det = n * sxx - sx * sx;
    if det.abs() < 1e-12 {
        return Some(((sy / n).sqrt(), 0.0));
    }
    let s = (n * sxy - sx * sy) / det;
    let a = (sy - s * sx) / n;
    if a <= 0.0 {
        return None;
    }
    Some((a.sqrt(), s / a))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synth::{Partial, render};

    fn tone(f0: f64, b: f64, sr: f64) -> Vec<f32> {
        let parts: Vec<Partial> = (1..=200u32)
            .map(|k| {
                let kf = k as f64;
                Partial {
                    freq: kf * f0 * (1.0 + b * kf * kf).sqrt(),
                    amp: 0.3 / kf * (kf * std::f64::consts::PI / 8.0).sin().abs().max(0.05),
                    sigma: 0.5 + 1e-7 * (kf * f0).powi(2),
                    slow: None,
                    phase: 0.1 * kf,
                }
            })
            .filter(|p| p.freq < 0.45 * sr)
            .collect();
        let mut x = render(&parts, sr, 1.5, 1e-4, 7);
        x[..10].fill(0.0);
        x
    }

    #[test]
    fn recovers_b_across_keyboard() {
        let sr = 48_000.0;
        for (f0, b) in [(27.5, 5.1e-4), (65.4, 2.4e-4), (261.6, 3.3e-4), (1046.5, 2.5e-3), (2093.0, 7e-3)] {
            let x = tone(f0, b, sr);
            let fit = estimate(&x, sr, f0 * 1.01, Options::default()).unwrap_or_else(|| panic!("no fit at f0={f0}"));
            let rel = (fit.b - b).abs() / b;
            let cents = 1200.0 * (fit.f0 / f0).log2();
            assert!(rel < 0.03, "f0={f0}: B {} vs {b} ({} partials)", fit.b, fit.partials.len());
            assert!(cents.abs() < 0.2, "f0={f0}: {cents} cents");
        }
    }
}
