//! Per-partial amplitude envelopes and decay fits (single and double decay).

use crate::spectrum::hann;

/// Amplitude envelope of the component at `freq`: windowed heterodyne frames.
/// Returns (time s, level dB) with time at the frame centre.
pub fn envelope(x: &[f32], sr: f64, freq: f64, win_s: f64, hop_s: f64) -> Vec<(f64, f64)> {
    let wlen = ((win_s * sr) as usize).max(16);
    let hop = ((hop_s * sr) as usize).max(1);
    let w = hann(wlen);
    let norm = 2.0 / w.iter().sum::<f64>();
    let dphi = std::f64::consts::TAU * freq / sr;
    // Precomputed rotator per window sample; phase offset per frame cancels in |·|.
    let rot: Vec<(f64, f64)> = (0..wlen).map(|i| (dphi * i as f64).sin_cos()).collect();
    let mut out = Vec::new();
    let mut start = 0;
    while start + wlen <= x.len() {
        let (mut re, mut im) = (0.0, 0.0);
        for i in 0..wlen {
            let v = x[start + i] as f64 * w[i];
            re += v * rot[i].1;
            im -= v * rot[i].0;
        }
        let a = norm * (re * re + im * im).sqrt();
        out.push(((start + wlen / 2) as f64 / sr, 20.0 * (a + 1e-300).log10()));
        start += hop;
    }
    out
}

#[derive(Clone, Copy, Debug)]
pub struct LineFit {
    /// Level at t = 0, dB.
    pub intercept_db: f64,
    /// dB per second (negative when decaying).
    pub slope_db_s: f64,
    pub rms_err_db: f64,
}

impl LineFit {
    pub fn sigma(&self) -> f64 {
        -self.slope_db_s * std::f64::consts::LN_10 / 20.0
    }
    pub fn t60(&self) -> f64 {
        -60.0 / self.slope_db_s
    }
}

pub fn line_fit(pts: &[(f64, f64)]) -> Option<LineFit> {
    if pts.len() < 2 {
        return None;
    }
    let n = pts.len() as f64;
    let (sx, sy) = pts.iter().fold((0.0, 0.0), |(a, b), &(x, y)| (a + x, b + y));
    let (mx, my) = (sx / n, sy / n);
    let (mut sxx, mut sxy) = (0.0, 0.0);
    for &(x, y) in pts {
        sxx += (x - mx) * (x - mx);
        sxy += (x - mx) * (y - my);
    }
    if sxx == 0.0 {
        return None;
    }
    let slope = sxy / sxx;
    let icpt = my - slope * mx;
    let err = (pts.iter().map(|&(x, y)| (y - icpt - slope * x).powi(2)).sum::<f64>() / n).sqrt();
    Some(LineFit { intercept_db: icpt, slope_db_s: slope, rms_err_db: err })
}

/// Part of the envelope usable for fitting: from its maximum until it first
/// gets within `margin_db` of the noise floor (5th percentile level).
pub fn usable(env: &[(f64, f64)], margin_db: f64) -> &[(f64, f64)] {
    usable_within(env, margin_db, f64::INFINITY)
}

/// Same, the floor held at least `range_db` below the maximum: a render
/// has no noise floor, and its partials fall to digital silence, which no
/// recording shows (their usable range is 58–73 dB, median per set).
pub fn usable_within(env: &[(f64, f64)], margin_db: f64, range_db: f64) -> &[(f64, f64)] {
    if env.is_empty() {
        return env;
    }
    let mut lv: Vec<f64> = env.iter().map(|p| p.1).collect();
    lv.sort_by(f64::total_cmp);
    let imax = env.iter().enumerate().max_by(|a, b| a.1.1.total_cmp(&b.1.1)).unwrap().0;
    let floor = lv[lv.len() / 20].max(env[imax].1 - range_db - margin_db);
    let end = env[imax..].iter().position(|p| p.1 < floor + margin_db).map_or(env.len(), |i| imax + i);
    &env[imax..end]
}

#[derive(Clone, Copy, Debug)]
pub struct DoubleDecay {
    /// Fast component: linear amplitude at t=0 and decay rate (1/s).
    pub a1: f64,
    pub sigma1: f64,
    /// Slow component ("aftersound").
    pub a2: f64,
    pub sigma2: f64,
    /// Time where the slow component takes over.
    pub knee_s: f64,
    pub rms_err_db: f64,
}

/// Fits a·e^{-σ1 t} + b·e^{-σ2 t} to an envelope: best two-line split in dB,
/// then alternate refits of each exponential with the other subtracted.
pub fn fit_double(pts: &[(f64, f64)]) -> Option<DoubleDecay> {
    let n = pts.len();
    if n < 10 {
        return None;
    }
    let mut best: Option<(f64, usize)> = None;
    for j in 4..n - 4 {
        let (Some(l), Some(r)) = (line_fit(&pts[..j]), line_fit(&pts[j..])) else { continue };
        let sse = l.rms_err_db.powi(2) * j as f64 + r.rms_err_db.powi(2) * (n - j) as f64;
        if best.is_none_or(|b| sse < b.0) {
            best = Some((sse, j));
        }
    }
    let (_, j) = best?;
    let to_exp = |f: LineFit| (10f64.powf(f.intercept_db / 20.0), f.sigma());
    let (mut a1, mut s1) = to_exp(line_fit(&pts[..j])?);
    let (mut a2, mut s2) = to_exp(line_fit(&pts[j..])?);
    let lin: Vec<(f64, f64)> = pts.iter().map(|&(t, d)| (t, 10f64.powf(d / 20.0))).collect();
    let residual_fit = |range: &[(f64, f64)], a: f64, s: f64| {
        let pts: Vec<(f64, f64)> = range
            .iter()
            .filter_map(|&(t, v)| {
                let r = v - a * (-s * t).exp();
                (r > 0.0).then(|| (t, 20.0 * r.log10()))
            })
            .collect();
        line_fit(&pts).map(to_exp)
    };
    for _ in 0..4 {
        if let Some(e) = residual_fit(&lin[..j], a2, s2) {
            (a1, s1) = e;
        }
        if let Some(e) = residual_fit(&lin[j..], a1, s1) {
            (a2, s2) = e;
        }
    }
    let err = (pts
        .iter()
        .map(|&(t, d)| (d - 20.0 * (a1 * (-s1 * t).exp() + a2 * (-s2 * t).exp()).log10()).powi(2))
        .sum::<f64>()
        / n as f64)
        .sqrt();
    Some(DoubleDecay { a1, sigma1: s1, a2, sigma2: s2, knee_s: pts[j].0, rms_err_db: err })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synth::{render, Partial};

    #[test]
    fn single_decay_t60() {
        let sr = 48_000.0;
        let x = render(
            &[Partial { freq: 440.0, amp: 0.5, sigma: 1.3, slow: None, phase: 0.0 }],
            sr,
            6.0,
            1e-5,
            1,
        );
        let env = envelope(&x, sr, 440.0, 0.05, 0.01);
        let fit = line_fit(usable(&env, 10.0)).unwrap();
        assert!((fit.sigma() - 1.3).abs() / 1.3 < 0.01, "{}", fit.sigma());
    }

    #[test]
    fn double_decay_parameters() {
        let sr = 48_000.0;
        // Fast 3 1/s, slow 0.3 1/s at −26 dB, with a neighbouring partial.
        let x = render(
            &[
                Partial { freq: 220.0, amp: 1.0, sigma: 3.0, slow: Some((0.05, 0.3)), phase: 0.0 },
                Partial { freq: 441.0, amp: 0.5, sigma: 2.0, slow: None, phase: 1.0 },
            ],
            sr,
            15.0,
            1e-6,
            2,
        );
        let env = envelope(&x, sr, 220.0, 0.08, 0.02);
        let d = fit_double(usable(&env, 10.0)).unwrap();
        assert!((d.sigma1 - 3.0).abs() / 3.0 < 0.05, "{d:?}");
        assert!((d.sigma2 - 0.3).abs() / 0.3 < 0.05, "{d:?}");
        assert!((20.0 * (d.a2 / d.a1).log10() + 26.0).abs() < 1.0, "{d:?}");
    }
}
