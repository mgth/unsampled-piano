//! The measures of one recorded note, the same on every piano and on the
//! model. Channels are never summed as pressures: spectra and envelopes
//! average the channels' powers, as the bench does (docs/bench.md).

use piano_analysis::{decay, inharm, spectrum::Spectrum};
use serde::{Deserialize, Serialize};

/// Bump when a measure changes: cached analyses of another version are
/// measured again.
pub const VERSION: u32 = 9;

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
    /// Late level of the partial lines of a band (see [`LATE_LINES`]).
    Lines1225Late,
    Lines2540Mid,
    Lines2540Late,
    Lines4070Mid,
    Lines4070Late,
    /// Early decay of the partial lines of a band (see [`EARLY_LINES`]).
    LinesEarly2540,
    LinesEarly4070,
    DecayEarly,
    DecayLate,
    T60H1,
    Ripple,
    /// Level drop of a band across the key's release (see [`RELEASE_BANDS`]).
    Release150,
    Release600,
    Release1200,
    Release2500,
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

impl Def {
    /// Measured on the notes released within their file (Liang's, MAPS's and
    /// the model's second render): judged against the pianos that have one,
    /// whatever band the other measures use.
    pub fn released(&self) -> bool {
        RELEASE_BANDS.iter().any(|b| b.0 == self.id)
    }

    /// Pianos needed at a key and layer for the model's z. Two, so that a
    /// gap as large as the one between two real pianos is not a fault; one
    /// for the release drops, which only Liang (every key, ff) and MAPS (40
    /// keys at ff, 34 at mf) can judge and which rarely both read at one
    /// key: there the floor alone stands for the spread.
    pub fn judges(&self) -> usize {
        if self.released() { 1 } else { 2 }
    }
}

/// The partial lines' late level: (measure, band lo, hi in Hz, window start,
/// end in s after the onset).
const LATE_LINES: [(Id, f64, f64, f64, f64); 5] = [
    (Id::Lines1225Late, 1200.0, 2500.0, 1.2, 2.2),
    (Id::Lines2540Mid, 2500.0, 4000.0, 0.6, 1.2),
    (Id::Lines2540Late, 2500.0, 4000.0, 1.2, 2.2),
    (Id::Lines4070Mid, 4000.0, 7000.0, 0.6, 1.2),
    (Id::Lines4070Late, 4000.0, 7000.0, 1.2, 2.2),
];
/// The partial lines' early decay: (measure, band lo, hi in Hz). Their power
/// in [`EARLY_WINDOWS`] windows of [`EARLY_WINDOW`] s every [`EARLY_HOP`] s
/// from 0.02 s after the onset (0.02 to 0.32 s), a straight line fitted to
/// the levels in dB.
const EARLY_LINES: [(Id, f64, f64); 2] = [(Id::LinesEarly2540, 2500.0, 4000.0), (Id::LinesEarly4070, 4000.0, 7000.0)];
const EARLY_WINDOW: f64 = 0.1;
const EARLY_HOP: f64 = 0.025;
const EARLY_WINDOWS: usize = 9;
/// Smallest spread of the early decay of the lines, dB/s: the five pianos of
/// the band differ by about 25 dB/s (sd) at the median key from C4 up (their
/// unisons' beats weigh in a 0.3 s fit); the floor only stops two pianos that
/// agree by chance from making a target.
const EARLY_LINES_FLOOR: f64 = 8.0;
/// The lines of a band must hold this much more power than the level between
/// them would put on their bins, dB: a line lost in the noise reads nothing.
const LINES_CLEAR_DB: f64 = 10.0;

/// Bands of the release drop, Hz.
const RELEASE_BANDS: [(Id, f64, f64); 4] =
    [(Id::Release150, 150.0, 600.0), (Id::Release600, 600.0, 1200.0), (Id::Release1200, 1200.0, 2500.0), (Id::Release2500, 2500.0, 5000.0)];
/// A release is measured when the key is released within this many seconds
/// of the onset: Liang's and MAPS's notes (`held` 1.85 s) and the model's
/// release renders (2 s), not the model's notes held 3.5 s.
pub const RELEASE_HELD_MAX: f64 = 2.5;
/// Before the release (as found, the start of the fall), and after it, s.
const RELEASE_PRE: (f64, f64) = (-0.15, -0.03);
const RELEASE_POST: (f64, f64) = (0.10, 0.25);
/// The band's level before the release must stand this far above the file's
/// noise floor in the band, dB.
const RELEASE_CLEAR_DB: f64 = 25.0;

/// Smallest spreads of the late lines and of the release drops, dB, from the
/// real pianos' own spread (docs/compare_tool.md): the five pianos of the
/// band differ by 4.0 to 4.6 dB (sd) at the median key on the late lines, as
/// on partials 2-4 (4.3 dB, floor 3); Liang and MAPS by 4.3 to 4.7 dB at
/// their common keys on the release drops (600 Hz to 2.5 kHz).
const LINES_FLOOR: f64 = 3.0;
const RELEASE_FLOOR: f64 = 4.0;

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
    Def { id: Id::Lines1225Late, name: "Lines 1.2-2.5 kHz, 1.2-2.2 s", unit: "dB re p1-6", floor: LINES_FLOOR, log: false, decimals: 1,
          help: "Power on the partial lines between 1.2 and 2.5 kHz (each line whole, +-f1/4, the noise between them taken off), 1.2 to 2.2 s after the onset, against partials 1 to 6 in the first 5 to 45 ms. The control band of the four below. Nothing when the lines are not 10 dB clear of the level between them, or fewer than two lie in the band." },
    Def { id: Id::Lines2540Mid, name: "Lines 2.5-4 kHz, 0.6-1.2 s", unit: "dB re p1-6", floor: LINES_FLOOR, log: false, decimals: 1,
          help: "Power on the partial lines between 2.5 and 4 kHz, 0.6 to 1.2 s after the onset, against partials 1 to 6 in the first 5 to 45 ms: how long the upper partials ring." },
    Def { id: Id::Lines2540Late, name: "Lines 2.5-4 kHz, 1.2-2.2 s", unit: "dB re p1-6", floor: LINES_FLOOR, log: false, decimals: 1,
          help: "Power on the partial lines between 2.5 and 4 kHz, 1.2 to 2.2 s after the onset, against partials 1 to 6 in the first 5 to 45 ms." },
    Def { id: Id::Lines4070Mid, name: "Lines 4-7 kHz, 0.6-1.2 s", unit: "dB re p1-6", floor: LINES_FLOOR, log: false, decimals: 1,
          help: "Power on the partial lines between 4 and 7 kHz, 0.6 to 1.2 s after the onset, against partials 1 to 6 in the first 5 to 45 ms." },
    Def { id: Id::Lines4070Late, name: "Lines 4-7 kHz, 1.2-2.2 s", unit: "dB re p1-6", floor: LINES_FLOOR, log: false, decimals: 1,
          help: "Power on the partial lines between 4 and 7 kHz, 1.2 to 2.2 s after the onset, against partials 1 to 6 in the first 5 to 45 ms." },
    Def { id: Id::LinesEarly2540, name: "Lines 2.5-4 kHz, early decay", unit: "dB/s", floor: EARLY_LINES_FLOOR, log: false, decimals: 1,
          help: "Slope of the power on the partial lines between 2.5 and 4 kHz (each line +-f1/4, the noise between them taken off) over 0.02 to 0.32 s after the onset: nine 0.1 s windows every 25 ms, a straight line through their levels. The treble's upper partials lose their first 15-20 dB fast, then ring on: this is the first stage, the Lines measures the second. Nothing when a window's lines are not 10 dB clear of the level between them, fewer than two lie in the band, or the first partial is under 80 Hz (its lines are not apart in a 0.1 s window)." },
    Def { id: Id::LinesEarly4070, name: "Lines 4-7 kHz, early decay", unit: "dB/s", floor: EARLY_LINES_FLOOR, log: false, decimals: 1,
          help: "Slope of the power on the partial lines between 4 and 7 kHz over 0.02 to 0.32 s after the onset (nine 0.1 s windows every 25 ms), as the 2.5-4 kHz one." },
    Def { id: Id::DecayEarly, name: "Early decay", unit: "dB/s", floor: 3.0, log: false, decimals: 1,
          help: "Slope of the overall level from 0.1 to 1 s after the onset (all channels' power, 20 ms frames, above the noise floor)." },
    Def { id: Id::DecayLate, name: "Late decay", unit: "dB/s", floor: 2.0, log: false, decimals: 1,
          help: "Slope of the overall level from 1 to 3 s after the onset (above the noise floor)." },
    Def { id: Id::T60H1, name: "T60 of partial 1", unit: "s", floor: 0.1, log: true, decimals: 1,
          help: "Decay time of the first partial, a straight line fitted to its envelope from 0.1 to 3 s (above the noise floor)." },
    Def { id: Id::Ripple, name: "Partial 1 ripple", unit: "dB", floor: 1.0, log: false, decimals: 1,
          help: "RMS distance of the first partial's envelope from that straight decay: beats between the unison strings, double decay." },
    Def { id: Id::Release150, name: "Release drop 150-600 Hz", unit: "dB", floor: RELEASE_FLOOR, log: false, decimals: 1,
          help: "Level of the band 150 to 600 Hz 100 to 250 ms after the key's release, against 150 to 30 ms before it: what the damper leaves. Only on notes released within their file (Liang, MAPS, and the model rendered again with the key released 2 s after the onset), judged against Liang and MAPS. Nothing when the band stands less than 25 dB above the file's noise floor before the release." },
    Def { id: Id::Release600, name: "Release drop 0.6-1.2 kHz", unit: "dB", floor: RELEASE_FLOOR, log: false, decimals: 1,
          help: "Level of the band 600 to 1200 Hz 100 to 250 ms after the key's release, against 150 to 30 ms before it (Liang, MAPS and the model released at 2 s)." },
    Def { id: Id::Release1200, name: "Release drop 1.2-2.5 kHz", unit: "dB", floor: RELEASE_FLOOR, log: false, decimals: 1,
          help: "Level of the band 1.2 to 2.5 kHz 100 to 250 ms after the key's release, against 150 to 30 ms before it (Liang, MAPS and the model released at 2 s)." },
    Def { id: Id::Release2500, name: "Release drop 2.5-5 kHz", unit: "dB", floor: RELEASE_FLOOR, log: false, decimals: 1,
          help: "Level of the band 2.5 to 5 kHz 100 to 250 ms after the key's release, against 150 to 30 ms before it (Liang, MAPS and the model released at 2 s)." },
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
    // A window gone silent reads nothing: the spectrum of several channels
    // floors its empty bins at a small positive power, which would pass for
    // a note. Its signal power has to be above -100 dB re the first second's.
    let mean = |p: &[f64]| p.iter().sum::<f64>() / p.len().max(1) as f64;
    let window = &power[at(0.5).min(power.len())..at(1.0).min(power.len())];
    let sounding = !window.is_empty() && mean(window) > 1e-10 * mean(first);
    if sounding && on + at(1.0) <= chs[0].len() && fk(4) < fmax {
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

    // The partial lines' late level, band by band, against partials 1-6 at
    // the start: how long the upper partials ring, apart from their start.
    if let Some(f) = &fit
        && on + at(0.045) <= chs[0].len() {
            let start = Spectrum::channel_power(&chs, sr, on + at(0.005), at(0.04), 4);
            let reference = band_power(&start, at(0.04), 0.5 * f1, fk(6) + 0.5 * f1);
            // A line found by the fit sits where it was found; the others
            // where the fit puts them.
            let line = |k: usize| f.partials.iter().find(|p| p.k as usize == k).map_or(fk(k), |p| p.freq);
            for &(id, lo, hi, t0, t1) in &LATE_LINES {
                if t1 > until || on + at(t1) > chs[0].len() || hi > fmax || reference <= 0.0 {
                    continue;
                }
                let s = Spectrum::channel_power(&chs, sr, on + at(t0), at(t1 - t0), 2);
                let lines: Vec<f64> = (1..=400).map(line).filter(|&f| f >= lo && f < hi).collect();
                let v = lines_power(&s, at(t1 - t0), &lines, (1..=400).map(line).take_while(|&f| f < hi + f1), 0.25 * f1, lo, hi);
                set(id, v.map(|p| 10.0 * (p / reference).log10()));
            }
        }

    // The partial lines' early decay, band by band: the first stage of the
    // upper partials, before the Lines' late levels.
    if let Some(f) = &fit {
        let line = |k: usize| f.partials.iter().find(|p| p.k as usize == k).map_or(fk(k), |p| p.freq);
        let len = at(EARLY_WINDOW);
        let t_end = 0.02 + (EARLY_WINDOWS - 1) as f64 * EARLY_HOP + EARLY_WINDOW;
        // A line spreads over +-2/T in a Hann window of T s: the lines' own
        // half width, f1/4, must hold it.
        let resolved = 0.25 * f1 >= 2.0 / EARLY_WINDOW;
        for &(id, lo, hi) in &EARLY_LINES {
            if !resolved || t_end > until || on + at(t_end) > chs[0].len() || hi > fmax {
                continue;
            }
            let lines: Vec<f64> = (1..=400).map(line).filter(|&f| f >= lo && f < hi).collect();
            let pts: Vec<(f64, f64)> = (0..EARLY_WINDOWS)
                .map_while(|i| {
                    let t0 = 0.02 + i as f64 * EARLY_HOP;
                    let s = Spectrum::channel_power(&chs, sr, on + at(t0), len, 2);
                    let p = lines_power(&s, len, &lines, (1..=400).map(line).take_while(|&f| f < hi + f1), 0.25 * f1, lo, hi)?;
                    Some((t0 + 0.5 * EARLY_WINDOW, 10.0 * p.log10()))
                })
                .collect();
            if pts.len() == EARLY_WINDOWS {
                set(id, decay::line_fit(&pts).map(|l| l.slope_db_s));
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

    // What the damper leaves, on the notes released within their file.
    if let Some(h) = held.filter(|h| *h <= RELEASE_HELD_MAX) {
        for (id, v) in release_drops(&refs, sr, on, h, false) {
            set(id, v);
        }
    }
    a
}

/// The release drops alone, of a render released `held` s after its onset
/// (the model's second protocol). A render has no noise floor: only its own
/// range (90 dB) bounds the guard.
pub fn analyse_release(channels: &[Vec<f32>], sr: f64, held: f64) -> Analysis {
    let mut a = Analysis { values: vec![None; DEFS.len()], partials: vec![None; PARTIALS], level_db: None };
    let refs: Vec<&[f32]> = channels.iter().map(|c| c.as_slice()).collect();
    if refs.is_empty() || refs[0].len() < (0.5 * sr) as usize {
        return a;
    }
    let on = inharm::onset_channels(&refs);
    for (id, v) in release_drops(&refs, sr, on, held, true) {
        a.values[index(id)] = v.filter(|x| x.is_finite());
    }
    a
}

/// Mean power (signal units squared, mean of the channels) of the bins in
/// [lo, hi] Hz of a Hann spectrum of `len` samples: comparable between
/// windows of different lengths.
fn band_power(s: &Spectrum, len: usize, lo: f64, hi: f64) -> f64 {
    let (a, b) = (((lo / s.bin_hz).ceil() as usize).max(1), ((hi / s.bin_hz).floor() as usize).min(s.db.len() - 1));
    if a > b {
        return 0.0;
    }
    bins_to_power(s, len, s.db[a..=b].iter().map(|d| 10f64.powf(d / 10.0)).sum())
}

/// Summed |X|^2 of a Hann window of `len` samples (sum w^2 = 3 len / 8),
/// one-sided, as the mean power of the signal it holds.
fn bins_to_power(s: &Spectrum, len: usize, sum: f64) -> f64 {
    let nfft = (s.sr / s.bin_hz).round();
    sum * 16.0 / (3.0 * len as f64 * nfft)
}

/// Power on the partial lines `lines` (each its bins within `half` Hz) of
/// the band [lo, hi), the noise taken off: the median level of the band's
/// bins off every line (`all`, the lines up to past the band) stands for
/// it. None when fewer than two lines lie in the band, or when they are not
/// [`LINES_CLEAR_DB`] above that noise.
fn lines_power(s: &Spectrum, len: usize, lines: &[f64], all: impl Iterator<Item = f64>, half: f64, lo: f64, hi: f64) -> Option<f64> {
    if lines.len() < 2 {
        return None;
    }
    let n = s.db.len();
    let bins = |f: f64| (((f - half) / s.bin_hz).ceil().max(1.0) as usize, (((f + half) / s.bin_hz).floor() as usize).min(n - 1));
    let mut on_line = vec![false; n];
    for f in all {
        let (a, b) = bins(f);
        for x in on_line.iter_mut().take(b + 1).skip(a) {
            *x = true;
        }
    }
    let (ba, bb) = (((lo / s.bin_hz).ceil() as usize).max(1), ((hi / s.bin_hz) as usize).min(n - 1));
    let mut off: Vec<f64> = (ba..=bb).filter(|&i| !on_line[i]).map(|i| s.db[i]).collect();
    if off.len() < 8 {
        return None;
    }
    off.sort_by(f64::total_cmp);
    // The median of a noise's power is ln 2 of its mean.
    let noise_bin = 10f64.powf(off[off.len() / 2] / 10.0) / std::f64::consts::LN_2;
    let (mut sum, mut count) = (0.0, 0usize);
    for &f in lines {
        let (a, b) = bins(f);
        for i in a..=b {
            sum += 10f64.powf(s.db[i] / 10.0);
            count += 1;
        }
    }
    let noise = noise_bin * count as f64;
    (sum > 0.0 && sum >= 10f64.powf(LINES_CLEAR_DB / 10.0) * noise).then(|| bins_to_power(s, len, sum - noise))
}

/// The drop of each band of [`RELEASE_BANDS`] across the key's release, dB.
/// The release is found, not assumed (the start of the overall level's
/// fall, from 0.35 s before `held` to 0.8 s after it).
/// `noiseless`: a render, whose floor is only its 90 dB range.
fn release_drops(refs: &[&[f32]], sr: f64, on: usize, held: f64, noiseless: bool) -> Vec<(Id, Option<f64>)> {
    let mut out: Vec<(Id, Option<f64>)> = RELEASE_BANDS.iter().map(|b| (b.0, None)).collect();
    let end = (on + (12.0 * sr) as usize).min(refs[0].len());
    let chs: Vec<&[f32]> = refs.iter().map(|c| &c[on..end]).collect();
    let n = chs[0].len();
    let at = |s: f64| (s * sr).round() as usize;
    let power: Vec<f64> = (0..n).map(|i| chs.iter().map(|c| (c[i] as f64).powi(2)).sum::<f64>() / chs.len() as f64).collect();
    let (win, hop) = (at(0.02), at(0.01));
    let lv: Vec<f64> = (0..)
        .map(|i| i * hop)
        .take_while(|&i| i + win <= n)
        .map(|i| 10.0 * (power[i..i + win].iter().sum::<f64>() / win as f64).max(1e-30).log10())
        .collect();
    // The steepest 60 ms fall from 0.35 s before `held` to 0.8 s after it,
    // at least 3 dB, anchors the release. The release is then where the
    // level leaves its decay, from 0.4 s before the anchor to its end: the
    // first frame 1.5 dB under the line of the 200 ms before it (a decay),
    // still 3 dB under it 60 ms later, 6 dB under it 150 and 250 ms later,
    // and 3 dB under the lowest level of those 200 ms (a beat's dip comes
    // back up). A felt that stops the strings at once and one that damps
    // them slowly are timed alike.
    let (step, t_of) = (6, |i: usize| (i * hop) as f64 / sr);
    let anchor = (0..lv.len().saturating_sub(step))
        .filter(|&i| (held - 0.35..=held + 0.8).contains(&t_of(i)))
        .map(|i| (i, lv[i] - lv[i + step]))
        .max_by(|a, b| a.1.total_cmp(&b.1));
    let Some((anchor, _)) = anchor.filter(|a| a.1 >= 3.0) else { return out };
    let leaves = |k: usize| {
        let t = t_of(k);
        if t < 0.4 || t < held - 0.35 || k + 25 >= lv.len() {
            return false;
        }
        let pts: Vec<(f64, f64)> = (0..k).map(|j| (t_of(j), lv[j])).filter(|&(u, _)| u >= t - 0.2 && u <= t - 0.03).collect();
        let Some(line) = decay::line_fit(&pts) else { return false };
        // A decay, not the rise out of a beat's dip; and the fall goes under
        // the lowest level of those 200 ms.
        let lowest = pts.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
        let under = |j: usize| line.intercept_db + line.slope_db_s * t_of(j) - lv[j];
        line.slope_db_s <= 5.0
            && under(k) > 1.5
            && under(k + 6) > 3.0
            && under(k + 15) > 6.0
            && under(k + 25) > 6.0
            && lv[k + 15].max(lv[k + 25]) < lowest - 3.0
    };
    let Some(start) = (anchor.saturating_sub(40)..=anchor + step).find(|&k| leaves(k)) else { return out };
    let tr = t_of(start);
    if tr + RELEASE_POST.1 > n as f64 / sr {
        return out;
    }
    let window = |(a, b): (f64, f64)| {
        let (start, len) = (at(tr + a), at(b - a));
        (Spectrum::channel_power(&chs, sr, start, len, 2), len)
    };
    let (pre, pre_len) = window(RELEASE_PRE);
    let (post, post_len) = window(RELEASE_POST);
    // The noise floor of each band: its 2nd percentile over 50 ms frames of
    // the whole file (to 12 s), within 90 dB of the band's loudest frame.
    let (fwin, fhop) = (at(0.05), at(0.025));
    let frames: Vec<Spectrum> = (0..).map(|k| k * fhop).take_while(|&s| s + fwin <= n).map(|s| Spectrum::channel_power(&chs, sr, s, fwin, 1)).collect();
    for (o, &(_, lo, hi)) in out.iter_mut().zip(&RELEASE_BANDS) {
        if hi > 0.45 * sr {
            continue;
        }
        let mut lvl: Vec<f64> = frames.iter().map(|f| band_power(f, fwin, lo, hi)).filter(|p| *p > 1e-18).map(|p| 10.0 * p.log10()).collect();
        if lvl.is_empty() {
            continue;
        }
        lvl.sort_by(f64::total_cmp);
        let range = lvl[lvl.len() - 1] - 90.0;
        let floor_db = if noiseless { range } else { lvl[lvl.len() / 50].max(range) };
        let floor = 10f64.powf(floor_db / 10.0);
        let (p0, p1) = (band_power(&pre, pre_len, lo, hi), band_power(&post, post_len, lo, hi));
        if p0 <= 0.0 || 10.0 * p0.log10() < floor_db + RELEASE_CLEAR_DB {
            continue;
        }
        // The floor is taken off what is left; within 3 dB of it, the drop
        // is only a bound and reads nothing.
        if p1 >= 2.0 * floor {
            o.1 = Some(10.0 * ((p1 - floor) / p0).log10());
        }
    }
    out
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
    fn a_silent_sustain_reads_nothing_in_mono_or_stereo() {
        let sr = 48_000.0;
        let mut x = split_note(f_et(81), false, sr);
        // The note stops 0.4 s after its onset (0.1 s of silence before it).
        for v in &mut x[(0.5 * sr) as usize..] {
            *v = 0.0;
        }
        let mono = analyse(&[x.clone()], sr, 81, None);
        let stereo = analyse(&[x.clone(), x], sr, 81, None);
        assert_eq!(mono.get(Id::UpperSustain), None);
        assert_eq!(stereo.get(Id::UpperSustain), None);
    }

    /// A partial: frequency, amplitude, T60 (s), and its gain at the release
    /// (dB, over 10 ms), after which it decays with T60 `after`.
    struct Line {
        f: f64,
        amp: f64,
        t60: f64,
        gain_db: f64,
    }

    /// 0.1 s of silence, then the lines from their peak, released `release`
    /// s after the onset, plus white noise of RMS `noise`.
    fn tone(lines: &[Line], release: Option<f64>, after: f64, sr: f64, dur: f64, noise: f64) -> Vec<f32> {
        let sigma = |t60: f64| 3.0 * std::f64::consts::LN_10 / t60;
        let pre = (0.1 * sr) as usize;
        let n = (dur * sr) as usize;
        let mut x = vec![0.0f64; pre + n];
        for l in lines {
            let (w, s) = (std::f64::consts::TAU * l.f / sr, sigma(l.t60));
            for i in 0..n {
                let t = i as f64 / sr;
                let env = match release {
                    Some(tr) if t >= tr => {
                        let ramp = ((t - tr) / 0.01).min(1.0);
                        (-s * tr).exp() * 10f64.powf(ramp * l.gain_db / 20.0) * (-sigma(after) * (t - tr)).exp()
                    }
                    _ => (-s * t).exp(),
                };
                x[pre + i] += l.amp * env * (w * i as f64 + 0.3 * l.f).sin();
            }
        }
        let mut rng = piano_analysis::synth::Lcg(7);
        x.iter().map(|&v| (v + noise * rng.gauss()) as f32).collect()
    }

    /// Mean of exp(-2 sigma t) over [a, b], weighted as a Hann window weighs
    /// a spectrum's power (w^2): what the measures read of a decay.
    fn mean_decay(t60: f64, a: f64, b: f64) -> f64 {
        let s2 = 2.0 * 3.0 * std::f64::consts::LN_10 / t60;
        let n = 4000;
        let (mut sum, mut weight) = (0.0, 0.0);
        for i in 0..n {
            let u = (i as f64 + 0.5) / n as f64;
            let w = (0.5 - 0.5 * (std::f64::consts::TAU * u).cos()).powi(2);
            sum += w * (-s2 * (a + u * (b - a))).exp();
            weight += w;
        }
        sum / weight
    }

    /// Partials of A3 with B = 2e-4 at 1/k, ringing 8 s below 2.5 kHz,
    /// 3 s in 2.5-4 kHz and 1.5 s above.
    fn ringing(f0: f64) -> Vec<Line> {
        (1..=40)
            .map(|k| {
                let kf = k as f64;
                let f = kf * f0 * (1.0 + 2e-4 * kf * kf).sqrt();
                let t60 = if f < 2500.0 { 8.0 } else if f < 4000.0 { 3.0 } else { 1.5 };
                Line { f, amp: 0.1 / kf, t60, gain_db: 0.0 }
            })
            .filter(|l| l.f < 7500.0)
            .collect()
    }

    /// The expected late level of a band: its lines against partials 1-6 at
    /// 5-45 ms, from the known amplitudes and decays.
    fn expected_lines(lines: &[Line], lo: f64, hi: f64, t0: f64, t1: f64) -> f64 {
        let p = |l: &Line, a, b| l.amp * l.amp / 2.0 * mean_decay(l.t60, a, b);
        let reference: f64 = lines[..6].iter().map(|l| p(l, 0.005, 0.045)).sum();
        let band: f64 = lines.iter().filter(|l| l.f >= lo && l.f < hi).map(|l| p(l, t0, t1)).sum();
        10.0 * (band / reference).log10()
    }

    #[test]
    fn the_late_lines_read_their_known_levels() {
        let sr = 48_000.0;
        let f0 = f_et(57);
        let lines = ringing(f0);
        let x = tone(&lines, None, 1.0, sr, 3.5, 0.0);
        let a = analyse(&[x.clone(), x], sr, 57, None);
        for &(id, lo, hi, t0, t1) in &LATE_LINES {
            let want = expected_lines(&lines, lo, hi, t0, t1);
            let got = a.get(id).unwrap_or(f64::NAN);
            assert!((got - want).abs() < 1.0, "{}: {got} against {want}", def(id).name);
        }
        // The 4-7 kHz lines have lost 24 dB more than the control band by
        // 1.2-2.2 s: a known gap the measures must keep.
        assert!(a.get(Id::Lines4070Late).unwrap() < a.get(Id::Lines1225Late).unwrap() - 30.0);
    }

    #[test]
    fn late_lines_lost_in_noise_or_after_the_release_read_nothing() {
        let sr = 48_000.0;
        let f0 = f_et(57);
        let lines = ringing(f0);
        // Noise that buries the 4-7 kHz lines by 1.2 s (they are near -75 dB
        // there), not the 2.5-4 kHz ones at 0.6-1.2 s.
        let x = tone(&lines, None, 1.0, sr, 3.5, 1e-4);
        let a = analyse(&[x.clone(), x.clone()], sr, 57, None);
        assert_eq!(a.get(Id::Lines4070Late), None);
        let want = expected_lines(&lines, 2500.0, 4000.0, 0.6, 1.2);
        let got = a.get(Id::Lines2540Mid).unwrap();
        assert!((got - want).abs() < 1.0, "2.5-4 kHz with noise: {got} against {want}");
        // A key released at 1.85 s leaves 1.2-2.2 s out, not 0.6-1.2 s.
        let held = analyse(std::slice::from_ref(&x), sr, 57, Some(1.85));
        assert_eq!(held.get(Id::Lines2540Late), None);
        assert!(held.get(Id::Lines2540Mid).is_some());
        // A file too short for the window.
        let short = analyse(&[x[..(2.0 * sr) as usize].to_vec()], sr, 57, None);
        assert_eq!(short.get(Id::Lines2540Late), None);
        assert!(short.get(Id::Lines2540Mid).is_some());
        // Fewer than two lines in a band: C7's lines are 2 kHz apart.
        let top: Vec<Line> = (1..=3).map(|k| Line { f: k as f64 * f_et(96), amp: 0.1 / k as f64, t60: 4.0, gain_db: 0.0 }).collect();
        let y = tone(&top, None, 1.0, sr, 3.5, 0.0);
        assert_eq!(analyse(&[y], sr, 96, None).get(Id::Lines2540Late), None);
    }

    /// Lines that lose 60 dB/s over their first 0.3 s, then ring on, read
    /// their first stage; the late lines read the second.
    #[test]
    fn the_early_lines_read_their_first_stage() {
        let sr = 48_000.0;
        let f0 = f_et(72);
        // Two stages per line above 2.5 kHz: a fast part (T60 1 s, 60 dB/s)
        // and a slow one 20 dB under it (T60 3 s, 20 dB/s); the lines below
        // ring 8 s.
        let build = |two: bool| {
            let mut lines = Vec::new();
            for k in 1..=14 {
                let kf = k as f64;
                let f = kf * f0 * (1.0 + 9e-4 * kf * kf).sqrt();
                if f >= 7500.0 {
                    break;
                }
                let amp = 0.1 / kf;
                if f < 2500.0 {
                    lines.push(Line { f, amp, t60: 8.0, gain_db: 0.0 });
                } else if two {
                    lines.push(Line { f, amp, t60: 1.0, gain_db: 0.0 });
                    lines.push(Line { f: f + 0.05, amp: 0.1 * amp, t60: 3.0, gain_db: 0.0 });
                } else {
                    lines.push(Line { f, amp, t60: 3.0, gain_db: 0.0 });
                }
            }
            lines
        };
        let x = tone(&build(true), None, 1.0, sr, 3.5, 0.0);
        let a = analyse(&[x.clone(), x.clone()], sr, 72, None);
        // The fast part's 60 dB/s, a little less as the slow part comes up
        // by 0.32 s (it is then 2 dB under it).
        for id in [Id::LinesEarly2540, Id::LinesEarly4070] {
            let got = a.get(id).unwrap_or(f64::NAN);
            assert!(got < -45.0 && got > -62.0, "{}: {got}", def(id).name);
        }
        // One stage of 20 dB/s reads 20 dB/s.
        let y = tone(&build(false), None, 1.0, sr, 3.5, 0.0);
        let b = analyse(&[y.clone(), y], sr, 72, None);
        let got = b.get(Id::LinesEarly4070).unwrap();
        assert!((got + 20.0).abs() < 2.0, "one stage: {got}");
        // Released before 0.32 s, or lines not apart (A1, 55 Hz): nothing.
        assert_eq!(analyse(std::slice::from_ref(&x), sr, 72, Some(0.3)).get(Id::LinesEarly2540), None);
        let low: Vec<Line> = ringing(f_et(33));
        let z = tone(&low, None, 1.0, sr, 3.5, 0.0);
        assert_eq!(analyse(&[z], sr, 33, None).get(Id::LinesEarly2540), None);
    }

    /// A3 released 2 s after its onset: each band of the release drops by its
    /// own gain, then everything decays at 40 dB/s into the noise.
    fn released(noise: f64, sr: f64) -> (Vec<f32>, Vec<Line>) {
        let f0 = f_et(57);
        let lines: Vec<Line> = (1..=20)
            .map(|k| {
                let kf = k as f64;
                let f = kf * f0 * (1.0 + 2e-4 * kf * kf).sqrt();
                let (amp, gain_db) = match f {
                    f if f < 600.0 => (0.1 / kf, -25.0),
                    f if f < 1200.0 => (0.1 / kf, -20.0),
                    f if f < 2500.0 => (0.1 / kf, -30.0),
                    // 40 dB weaker: lost in the noise floor when there is one.
                    _ => (0.001 / kf, -12.0),
                };
                Line { f, amp, t60: 10.0, gain_db }
            })
            .collect();
        (tone(&lines, Some(2.0), 1.5, sr, 4.0, noise), lines)
    }

    /// The drop of a band: its gain, the decay of 10 s before, of 1.5 s
    /// after, the windows placed on a release found 10 ms early (the frame
    /// the fall starts in).
    fn expected_drop(gain_db: f64) -> f64 {
        let tr = 1.99;
        let pre = mean_decay(10.0, tr - 0.15, tr - 0.03);
        let post = (-2.0 * 3.0 * std::f64::consts::LN_10 / 10.0 * 2.0f64).exp() * mean_decay(1.5, tr + 0.10 - 2.0, tr + 0.25 - 2.0);
        gain_db + 10.0 * (post / pre).log10()
    }

    #[test]
    fn the_release_drop_reads_each_band_s_gain() {
        let sr = 48_000.0;
        let (x, _) = released(0.0, sr);
        // A render: no noise, every band read.
        let a = analyse_release(&[x.clone(), x.clone()], sr, 2.0);
        for (&(id, ..), gain) in RELEASE_BANDS.iter().zip([-25.0, -20.0, -30.0, -12.0]) {
            let got = a.get(id).unwrap_or(f64::NAN);
            let want = expected_drop(gain);
            assert!((got - want).abs() < 1.0, "{}: {got} against {want}", def(id).name);
        }
        // A recording with noise, released after `held` (1.85 s): the band
        // that stands 11 dB above its floor reads nothing, the others their
        // drop.
        let (y, _) = released(1e-4, sr);
        let r = analyse(&[y.clone(), y.clone()], sr, 57, Some(1.85));
        assert_eq!(r.get(Id::Release2500), None);
        for (id, gain) in [(Id::Release600, -20.0), (Id::Release1200, -30.0)] {
            let got = r.get(id).unwrap_or(f64::NAN);
            assert!((got - expected_drop(gain)).abs() < 1.0, "{} with noise: {got}", def(id).name);
        }
        // Held through the file (the model's 3.5 s renders) or not released:
        // no release measure.
        assert_eq!(analyse(std::slice::from_ref(&y), sr, 57, Some(3.5)).get(Id::Release600), None);
        assert_eq!(analyse(&[y], sr, 57, None).get(Id::Release600), None);
        let z = tone(&ringing(f_et(57)), None, 1.0, sr, 4.0, 0.0);
        assert_eq!(analyse_release(&[z], sr, 2.0).get(Id::Release600), None);
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
