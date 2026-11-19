//! The measures of one recorded note, the same on every piano and on the
//! model. Channels are never summed as pressures: spectra and envelopes
//! average the channels' powers, as the bench does (docs/bench.md).

use piano_analysis::{decay, inharm, spectrum::Spectrum};
use serde::{Deserialize, Serialize};

/// Bump when a measure changes: cached analyses of another version are
/// measured again.
pub const VERSION: u32 = 5;

/// Partials kept for the note's partial profile.
pub const PARTIALS: usize = 32;
/// Partials below this level (dB re the note's strongest) read at it: a
/// partial lost in the noise must not pull a mean by tens of dB.
pub const PARTIAL_FLOOR_DB: f32 = -80.0;
/// Above this the recordings hold mostly noise (the bench's exclusion).
const FMAX: f64 = 12_000.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Id {
    Tuning,
    Inharm,
    Rise,
    CentroidAttack,
    CentroidSustain,
    UpperSustain,
    Tilt,
    Between,
    DecayEarly,
    DecayLate,
    T60H1,
    Ripple,
    Dynamics,
}

pub struct Def {
    pub id: Id,
    pub name: &'static str,
    pub unit: &'static str,
    /// Smallest spread used for the model's z (two real pianos may agree by
    /// chance; the floor keeps that from becoming a target).
    pub floor: f64,
    /// Compared and plotted on a log scale.
    pub log: bool,
    pub decimals: usize,
    pub help: &'static str,
}

pub const DEFS: &[Def] = &[
    Def { id: Id::Tuning, name: "Tuning", unit: "cents", floor: 3.0, log: false, decimals: 1,
          help: "First partial against equal temperament at A4 = 440 Hz, from the inharmonic fit of the partials (0.05 to 1.05 s after the onset). Shows each piano's stretch and its tuner." },
    Def { id: Id::Inharm, name: "Inharmonicity B", unit: "", floor: 0.1, log: true, decimals: 1,
          help: "B of f_k = k f0 sqrt(1 + B k^2), fitted on every partial found 20 dB above its neighbourhood (0.05 to 1.05 s). Plotted on a log scale." },
    Def { id: Id::Rise, name: "Rise time", unit: "ms", floor: 3.0, log: false, decimals: 1,
          help: "Time from 10 % to 90 % of the peak amplitude (all channels' power, windows of 5 ms or one period, whichever is longer)." },
    Def { id: Id::CentroidAttack, name: "Brightness, attack", unit: "x f1", floor: 0.08, log: true, decimals: 2,
          help: "Spectral centroid of the first 60 ms, from half the first partial to 12 kHz, as a multiple of the first partial's frequency." },
    Def { id: Id::CentroidSustain, name: "Brightness, sustain", unit: "x f1", floor: 0.08, log: true, decimals: 2,
          help: "Spectral centroid from 0.5 to 1.0 s, from half the first partial to 12 kHz, as a multiple of the first partial's frequency." },
    Def { id: Id::UpperSustain, name: "Partials 2-4, sustain", unit: "dB re p1", floor: 3.0, log: false, decimals: 1,
          help: "Mean power of partials 2 to 4 against partial 1, 0.5 to 1.0 s: how far the overtones outweigh the fundamental as the note sounds on." },
    Def { id: Id::Tilt, name: "Spectral tilt", unit: "dB/oct", floor: 2.0, log: false, decimals: 1,
          help: "Slope of the partials' levels against log2 of their rank (partials 1 to 16 below 8 kHz, 0.02 to 0.32 s): how fast the partials fall off." },
    Def { id: Id::Between, name: "Between partials", unit: "dB", floor: 3.0, log: false, decimals: 1,
          help: "Median level between neighbouring partials, re the partials' level, below 5 kHz, 0.02 to 0.32 s: the halo of noise, beats and resonances around the partials." },
    Def { id: Id::DecayEarly, name: "Early decay", unit: "dB/s", floor: 3.0, log: false, decimals: 1,
          help: "Slope of the overall level from 0.1 to 1 s after the onset (all channels' power, 20 ms frames, above the noise floor)." },
    Def { id: Id::DecayLate, name: "Late decay", unit: "dB/s", floor: 2.0, log: false, decimals: 1,
          help: "Slope of the overall level from 1 to 3 s after the onset (above the noise floor)." },
    Def { id: Id::T60H1, name: "T60 of partial 1", unit: "s", floor: 0.1, log: true, decimals: 1,
          help: "Decay time of the first partial, a straight line fitted to its envelope from 0.1 to 3 s (above the noise floor)." },
    Def { id: Id::Ripple, name: "Partial 1 ripple", unit: "dB", floor: 1.0, log: false, decimals: 1,
          help: "RMS distance of the first partial's envelope from that straight decay: beats between the unison strings, double decay." },
    Def { id: Id::Dynamics, name: "Dynamics ff - pp", unit: "dB", floor: 3.0, log: false, decimals: 1,
          help: "Level of the ff layer minus the pp layer (first second). Only for the pianos recorded at known velocities (BiVib and the model); the sample sets were normalised." },
];

pub fn def(id: Id) -> &'static Def {
    DEFS.iter().find(|d| d.id == id).unwrap()
}

pub fn index(id: Id) -> usize {
    DEFS.iter().position(|d| d.id == id).unwrap()
}

/// Everything measured on one recording.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Analysis {
    /// One value per entry of [`DEFS`] (Dynamics is filled by the caller,
    /// from two layers).
    pub values: Vec<Option<f64>>,
    /// Partials 1.. in dB re the strongest, 0.02 to 0.32 s.
    pub partials: Vec<Option<f32>>,
    /// Mean power of the first second, dB.
    pub level_db: Option<f64>,
}

impl Analysis {
    pub fn get(&self, id: Id) -> Option<f64> {
        self.values.get(index(id)).copied().flatten()
    }
}

pub fn f_et(midi: u8) -> f64 {
    440.0 * 2f64.powf((midi as f64 - 69.0) / 12.0)
}

/// Measures of one note. `channels` all have the same length. `held`:
/// seconds after the onset at which the key was released, if within the
/// measures (the decays are fitted before it).
pub fn analyse(channels: &[Vec<f32>], sr: f64, midi: u8, held: Option<f64>) -> Analysis {
    let until = held.map_or(f64::INFINITY, |h| h - 0.05);
    let mut a = Analysis { values: vec![None; DEFS.len()], partials: vec![None; PARTIALS], level_db: None };
    let refs: Vec<&[f32]> = channels.iter().map(|c| c.as_slice()).collect();
    if refs.is_empty() || refs[0].len() < (0.5 * sr) as usize {
        return a;
    }
    let on = inharm::onset_channels(&refs);
    // Everything is measured within 3.3 s of the onset; the noise floors
    // are read up to 12 s, where a recording has mostly decayed into them
    // (within 3.3 s the quietest part is still the note).
    let end = (on + (3.3 * sr) as usize).min(refs[0].len());
    let long_end = (on + (12.0 * sr) as usize).min(refs[0].len());
    let chs: Vec<&[f32]> = refs.iter().map(|c| &c[..end]).collect();
    let after: Vec<&[f32]> = chs.iter().map(|c| &c[on..]).collect();
    let long: Vec<&[f32]> = refs.iter().map(|c| &c[on..long_end]).collect();
    let n_after = after[0].len();
    let at = |s: f64| (s * sr) as usize;
    let mut set = |id: Id, v: Option<f64>| a.values[index(id)] = v.filter(|x| x.is_finite());

    let f0n = f_et(midi);
    let fmax = FMAX.min(0.45 * sr);
    let fit = inharm::estimate_channels(&chs, sr, f0n, inharm::Options::default());
    let f1 = fit.as_ref().map_or(f0n, |f| f.predict(1));
    let fk = |k: usize| fit.as_ref().map_or(k as f64 * f0n, |f| f.predict(k as u32));
    if let Some(f) = &fit {
        let cents = 1200.0 * (f.predict(1) / f0n).log2();
        set(Id::Tuning, (cents.abs() < 60.0).then_some(cents));
        set(Id::Inharm, (f.b > 1e-7 && f.partials.len() >= 3).then_some(f.b));
    }

    // Overall power, all channels (to 12 s for the noise floor).
    let power: Vec<f64> = (0..long[0].len()).map(|i| long.iter().map(|c| (c[i] as f64).powi(2)).sum::<f64>() / long.len() as f64).collect();
    let first = &power[..at(1.0).min(power.len())];
    a.level_db = (!first.is_empty()).then(|| 10.0 * (first.iter().sum::<f64>() / first.len() as f64).max(1e-30).log10());

    // Rise time on a short RMS envelope.
    {
        let win = at((1.0 / f1).max(0.005)).max(1);
        let hop = at(0.0005).max(1);
        let limit = n_after.min(at(0.6));
        let mut env = Vec::new();
        let mut i = 0;
        while i + win <= limit {
            env.push(power[i..i + win].iter().sum::<f64>() / win as f64);
            i += hop;
        }
        if let Some(peak) = env.iter().copied().reduce(f64::max).filter(|p| *p > 0.0) {
            let t10 = env.iter().position(|p| *p >= 0.01 * peak);
            let t90 = env.iter().position(|p| *p >= 0.81 * peak);
            if let (Some(t10), Some(t90)) = (t10, t90) {
                set(Id::Rise, Some((t90 - t10) as f64 * hop as f64 / sr * 1000.0));
            }
        }
    }

    // Brightness: centroid against f1.
    let centroid = |start: usize, len: usize| -> Option<f64> {
        if start + len / 2 > chs[0].len() {
            return None;
        }
        let s = Spectrum::channel_power(&chs, sr, start, len, 2);
        let (lo, hi) = ((0.5 * f1 / s.bin_hz) as usize, ((fmax / s.bin_hz) as usize).min(s.db.len() - 1));
        let (mut w, mut fw) = (0.0, 0.0);
        for i in lo.max(1)..=hi {
            let p = 10f64.powf(s.db[i] / 10.0);
            w += p;
            fw += p * i as f64 * s.bin_hz;
        }
        (w > 0.0).then(|| fw / w / f1)
    };
    set(Id::CentroidAttack, centroid(on, at(0.06)));
    set(Id::CentroidSustain, centroid(on + at(0.5), at(0.5)));

    // Partials 2-4 against partial 1 in the sustain.
    if on + at(1.0) <= chs[0].len() && fk(4) < fmax {
        let s = Spectrum::channel_power(&chs, sr, on + at(0.5), at(0.5), 4);
        // The power summed over each partial's band: a unison's partial split
        // into several lines counts whole, as one line would.
        let half = 0.25 * f1;
        let power = |k: usize| {
            let (lo, hi) = (((fk(k) - half) / s.bin_hz).ceil() as usize, ((fk(k) + half) / s.bin_hz) as usize);
            (lo >= 1 && hi < s.db.len() && lo < hi).then(|| s.db[lo..=hi].iter().map(|d| 10f64.powf(d / 10.0)).sum::<f64>())
        };
        if let (Some(p1), Some(p2), Some(p3), Some(p4)) = (power(1), power(2), power(3), power(4)) {
            set(Id::UpperSustain, (p1 > 0.0).then(|| 10.0 * ((p2 + p3 + p4) / 3.0 / p1).log10()));
        }
    }

    // Partials, tilt and the level between them.
    if on + at(0.32) <= chs[0].len() {
        let s = Spectrum::channel_power(&chs, sr, on + at(0.02), at(0.3), 4);
        let mut lv: Vec<Option<f64>> = vec![None; PARTIALS];
        for (k, l) in lv.iter_mut().enumerate().map(|(i, l)| (i + 1, l)) {
            let f = fk(k);
            if f > fmax {
                break;
            }
            let half = 0.25 * f1;
            *l = s.peak_in(f - half, f + half).map(|p| p.1);
        }
        if let Some(top) = lv.iter().flatten().copied().reduce(f64::max) {
            for (out, l) in a.partials.iter_mut().zip(&lv) {
                *out = l.map(|l| ((l - top) as f32).max(PARTIAL_FLOOR_DB));
            }
            let pts: Vec<(f64, f64)> = (1..=16)
                .filter(|&k| fk(k) < 8000.0)
                .filter_map(|k| a.partials[k - 1].map(|l| ((k as f64).log2(), l as f64)))
                .collect();
            if pts.len() >= 3 {
                set(Id::Tilt, decay::line_fit(&pts).map(|f| f.slope_db_s));
            }
            let mut gaps = Vec::new();
            for k in 1..PARTIALS {
                let (fa, fb) = (fk(k), fk(k + 1));
                if fb > 5000.0 {
                    break;
                }
                if let (Some(la), Some(lb)) = (lv[k - 1], lv[k]) {
                    let sp = fb - fa;
                    let mid = s.median_in(fa + 0.25 * sp, fb - 0.25 * sp);
                    gaps.push(mid - 0.5 * (la + lb));
                }
            }
            if !gaps.is_empty() {
                set(Id::Between, Some(gaps.iter().sum::<f64>() / gaps.len() as f64));
            }
        }
    }

    // Overall decay, 20 ms frames, above the noise floor.
    {
        let (win, hop) = (at(0.02), at(0.01));
        let mut env = Vec::new();
        let mut i = 0;
        while i + win <= power.len() {
            let p = power[i..i + win].iter().sum::<f64>() / win as f64;
            env.push(((i + win / 2) as f64 / sr, 10.0 * p.max(1e-30).log10()));
            i += hop;
        }
        let floor = noise_floor(&env);
        let slope = |t0: f64, t1: f64, min: usize| {
            let pts: Vec<(f64, f64)> = env.iter().copied().filter(|&(t, l)| t >= t0 && t <= t1 && l > floor + 6.0).collect();
            (pts.len() >= min).then(|| decay::line_fit(&pts)).flatten().map(|f| f.slope_db_s)
        };
        set(Id::DecayEarly, slope(0.1, until.min(1.0), 20));
        if power.len() as f64 / sr >= 2.9 {
            set(Id::DecayLate, slope(1.0, until.min(3.0), 40));
        }
    }

    // First partial's envelope.
    if f1 < fmax && n_after as f64 / sr >= 1.0 {
        let env = decay::envelope_channels(&long, sr, f1, (6.0 / f1).max(0.05), 0.01);
        let floor = noise_floor(&env);
        let pts: Vec<(f64, f64)> = env.iter().copied().filter(|&(t, l)| t >= 0.1 && t <= until.min(3.0) && l > floor + 6.0).collect();
        if pts.len() >= 30
            && let Some(f) = decay::line_fit(&pts) {
                if f.slope_db_s < 0.0 {
                    set(Id::T60H1, Some(f.t60().min(200.0)));
                }
                set(Id::Ripple, Some(f.rms_err_db));
            }
    }
    a
}

/// The quietest level of an envelope read up to 12 s (its 2nd percentile):
/// the recording's noise, or the end of its decay on a short file. A render
/// has no noise: at least 90 dB of range are kept.
fn noise_floor(env: &[(f64, f64)]) -> f64 {
    let mut v: Vec<f64> = env.iter().map(|p| p.1).collect();
    if v.is_empty() {
        return f64::NEG_INFINITY;
    }
    v.sort_by(f64::total_cmp);
    let max = v[v.len() - 1];
    v[v.len() / 50].max(max - 90.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use piano_analysis::synth::{render, Partial};

    fn note(f0: f64, b: f64, t60: f64, sr: f64) -> Vec<f32> {
        let partials: Vec<Partial> = (1..=20)
            .map(|k| {
                let kf = k as f64;
                // T60 = 3 ln 10 / sigma.
                Partial { freq: kf * f0 * (1.0 + b * kf * kf).sqrt(), amp: 0.1 / kf, sigma: 3.0 * std::f64::consts::LN_10 / t60, slow: None, phase: 0.0 }
            })
            .filter(|p| p.freq < 0.45 * sr)
            .collect();
        let mut x = vec![0.0f32; (0.1 * sr) as usize];
        x.extend(render(&partials, sr, 3.5, 0.0, 1));
        x
    }

    /// Partial 1 alone, then partials 2-4 at 1/k, each as one line or split
    /// into three lines 8 Hz apart carrying the same power.
    fn split_note(f0: f64, split: bool, sr: f64) -> Vec<f32> {
        let sigma = 3.0 * std::f64::consts::LN_10 / 8.0;
        let mut partials = vec![Partial { freq: f0, amp: 0.1, sigma, slow: None, phase: 0.0 }];
        for k in 2..=4 {
            let (kf, amp) = (k as f64 * f0, 0.1 / k as f64);
            if split {
                for d in [-8.0, 0.0, 8.0] {
                    partials.push(Partial { freq: kf + d, amp: amp / 3f64.sqrt(), sigma, slow: None, phase: 0.7 * d });
                }
            } else {
                partials.push(Partial { freq: kf, amp, sigma, slow: None, phase: 0.0 });
            }
        }
        let mut x = vec![0.0f32; (0.1 * sr) as usize];
        x.extend(render(&partials, sr, 3.5, 0.0, 1));
        x
    }

    #[test]
    fn a_split_unison_partial_counts_whole() {
        let sr = 48_000.0;
        let read = |split| {
            let x = split_note(f_et(81), split, sr);
            analyse(&[x.clone(), x], sr, 81, None).get(Id::UpperSustain).unwrap()
        };
        let (one, three) = (read(false), read(true));
        assert!((one + 8.50).abs() < 0.5, "one line {one}");
        assert!((three - one).abs() < 0.5, "split {three} against {one}");
    }

    #[test]
    fn synthetic_note_reads_back() {
        let sr = 48_000.0;
        let x = note(f_et(57) * 2f64.powf(5.0 / 1200.0), 2e-4, 8.0, sr);
        let a = analyse(&[x.clone(), x.clone()], sr, 57, None);
        // A release at 2 s leaves the fits of what comes before alone.
        let cut = analyse(&[x], sr, 57, Some(2.0));
        assert!((cut.get(Id::T60H1).unwrap() / 8.0 - 1.0).abs() < 0.1);
        assert!(cut.get(Id::DecayLate).is_some());
        let tuning = a.get(Id::Tuning).unwrap();
        assert!((tuning - 5.0).abs() < 1.0, "tuning {tuning}");
        let b = a.get(Id::Inharm).unwrap();
        assert!((b / 2e-4 - 1.0).abs() < 0.1, "B {b}");
        let t60 = a.get(Id::T60H1).unwrap();
        assert!((t60 / 8.0 - 1.0).abs() < 0.1, "T60 {t60}");
        // 1/k amplitudes: −6 dB per octave of rank.
        // 1/k amplitudes: partials 2-4 at (1/4 + 1/9 + 1/16)/3 of partial 1's power.
        let upper = a.get(Id::UpperSustain).unwrap();
        assert!((upper + 8.50).abs() < 0.5, "partials 2-4 {upper}");
        let tilt = a.get(Id::Tilt).unwrap();
        assert!((tilt + 6.0).abs() < 1.0, "tilt {tilt}");
        assert!(a.get(Id::Ripple).unwrap() < 0.5);
        assert_eq!(a.partials[0], Some(0.0));
    }
}
