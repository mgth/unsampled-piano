//! The body ringing after each strike, heard directly: as the hammer meets
//! the string, its reaction runs back through the action into the keybed,
//! the keyframe, the case and the rim, and the board's low modes answer the
//! jolt. What it radiates fills the space between the partials with a dense,
//! continuous halo that lasts well after the action's own knock
//! ([`crate::action`]) has died: on real grands, 30 dB more than the model
//! without it at 125 Hz–1 kHz from 60 ms on, dying within 0.6–1 s
//! (docs/body.md).
//!
//! A bank of [`MODES`] damped resonances shared by all the keys, dense enough
//! to overlap (one every 3.5 Hz at the bottom, 5 Hz at 1 kHz, 7 at 2 kHz,
//! sparser above), so that
//! their sum is a halo rather than a chord. Each key strikes it with its own
//! pattern of amplitudes and signs, part shared by every key (the same body)
//! and part its own. Level and decay are set per octave band
//! ([`crate::params::BodyLaw`]); the two microphones hear each resonance
//! with their own weight and sign.
//!
//! The halo can settle in over [`BodyLaw::rise`] instead of starting at once:
//! on real grands the first 20–40 ms between the partials are the strings'
//! own attack, already right in the model, and a halo struck at full level
//! there doubled it into a "chtac" of its own; the body's share comes after
//! (docs/body.md, the halo's entry). Each resonance then takes its strike in
//! steps of up to [`SUB`] samples along a raised-cosine ramp, in phase with
//! its own ringing, so its spectrum and its tail stay those of the struck
//! halo.

use crate::params::BodyLaw;

/// Resonances of the bank (a multiple of [`LANES`]).
pub const MODES: usize = 1024;
const LANES: usize = 16;
const KEYS: usize = 88;
/// Strikes waiting for their delay or under way (a strike that settles in
/// over 0.1 s stays here that long).
const PENDING: usize = 32;
/// Most samples between two steps of a settling strike. A rise takes the
/// largest power of two up to this that leaves at least [`STEPS`] steps, so
/// that each step adds at most 2/STEPS (0.8 %) of the strike and the first
/// under 1e-6 of it, whatever the length: a rise of 0.1 s at 48 kHz steps
/// every 16 samples, one under 512 samples every sample.
const SUB: u32 = 16;
const STEPS: u32 = 256;
/// Step tables kept: 1, 2, 4, 8 and 16 samples.
const SUB_TABLES: usize = SUB.trailing_zeros() as usize + 1;
/// Length (s) of a strike: a short Hann pulse, so that the halo starts
/// without a step; its spectrum stays flat well past the bank's top.
const PULSE: f64 = 2.5e-4;
/// Lowest and highest resonance (Hz).
const F_LO: f64 = 40.0;
const F_HI: f64 = 10_000.0;
/// Below this the density is uniform in Hz; above it falls as 1/f.
const F_KNEE: f64 = 2000.0;
/// Octave band centres of the law's levels and decay times (Hz).
pub const BANDS: [f64; 8] = [63.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0];
/// Keys of the speed law's values ([`BodyLaw::speed_low`]): A0, C3, C5, C7
/// (linear in between, constant beyond).
pub const SPEED_KEYS: [f64; 4] = [21.0, 48.0, 72.0, 96.0];
/// Largest speed ratio the speed law follows above `speed_ref`.
const SPEED_RATIO_MAX: f64 = 2.0;

#[derive(Clone, Copy, Default)]
struct Pending {
    key: u8,
    amp: f32,
    wait: u32,
    age: u32,
    /// Its rise (samples) and step, latched when struck: a live change of
    /// [`BodyLaw::rise`] applies to the next strikes, never to one under
    /// way. A rise of 0 is the short pulse.
    rise: u32,
    sub: u32,
}

pub struct BodyBank {
    freq: Vec<f64>,
    pr: Vec<f32>,
    pi: Vec<f32>,
    zr: Vec<f32>,
    zi: Vec<f32>,
    /// Output weight per microphone and resonance (level law included).
    w: [Vec<f32>; 2],
    /// Per key and resonance: amplitude and sign of the strike, unit rms.
    pattern: Vec<f32>,
    pending: [Pending; PENDING],
    /// Per pending strike and resonance: the speed law's weight.
    speed_w: Vec<f32>,
    pulse: Vec<f32>,
    /// Excitation of the current sample, per resonance (scratch).
    exc: Vec<f32>,
    /// The short pulse's spectrum at each resonance, through its decaying
    /// pole (real and imaginary parts): a settling strike takes it too, so
    /// that once settled the halo is the struck one, sample for sample.
    pulse_hr: Vec<f32>,
    pulse_hi: Vec<f32>,
    /// Per step length 2^k (k < [`SUB_TABLES`]) and resonance, its rotation
    /// and decay over that many samples.
    step_r: [Vec<f32>; SUB_TABLES],
    step_i: [Vec<f32>; SUB_TABLES],
    /// Per pending strike and resonance: the resonance's rotation since the
    /// strike began, so that each step adds in phase with what rings.
    ph_r: Vec<f32>,
    ph_i: Vec<f32>,
    /// Length of the rise in samples for the next strikes; 0: struck at
    /// once by the pulse, as is any rise no longer than the pulse itself.
    rise: u32,
    awake: bool,
    sample_rate: f64,
    law: BodyLaw,
}

fn hash(a: f64, b: f64) -> f64 {
    ((a * 12.9898 + b * 78.233).sin() * 43_758.545_3).rem_euclid(1.0)
}

/// Value of a per-band law at `f`, interpolated in log frequency.
fn band_value(v: &[f64; 8], f: f64) -> f64 {
    let x = (f / BANDS[0]).log2().clamp(0.0, 7.0);
    let i = (x.floor() as usize).min(6);
    let t = x - i as f64;
    v[i] * (1.0 - t) + v[i + 1] * t
}

/// Frequencies of the bank: the inverse of the cumulative density
/// 1/(1 + f/F_KNEE), each jittered within its own slot.
fn frequencies() -> Vec<f64> {
    let cdf = |f: f64| F_KNEE * (1.0 + f / F_KNEE).ln();
    let (c0, c1) = (cdf(F_LO), cdf(F_HI));
    (0..MODES)
        .map(|i| {
            let u = (i as f64 + 0.5 + 0.8 * (hash(i as f64, 3.0) - 0.5)) / MODES as f64;
            let c = c0 + u * (c1 - c0);
            F_KNEE * ((c / F_KNEE).exp() - 1.0)
        })
        .collect()
}

/// The speed law of one blow ([`BodyLaw::speed_ref`]): the further gain of
/// a resonance at f, (speed/speed_ref)^γ(f). Shared by the body's halo and
/// the action's knock: the same blow jolts both.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SpeedTilt {
    /// ln(speed/speed_ref); 0: no tilt.
    lr: f64,
    lo: f64,
    hi: f64,
    /// Corner squared (Hz²).
    c2: f64,
}

impl SpeedTilt {
    pub fn of(law: &BodyLaw, note: u8, speed: f64) -> Self {
        if law.speed_ref <= 0.0 || speed <= 0.0 {
            return Self::default();
        }
        Self {
            lr: (speed / law.speed_ref).clamp(1e-3, SPEED_RATIO_MAX).ln(),
            lo: at_key(&law.speed_low, note),
            hi: at_key(&law.speed_high, note),
            c2: law.speed_corner.max(1.0).powi(2),
        }
    }

    /// The same law with its exponents times `k`.
    pub fn scaled(self, k: f64) -> Self {
        Self { lr: self.lr * k.max(0.0), ..self }
    }

    /// γ = low + high·x/(1 + x), x = (f/corner)².
    pub fn gain(&self, f: f64) -> f64 {
        if self.lr == 0.0 {
            return 1.0;
        }
        let x = f * f / self.c2;
        ((self.lo + self.hi * x / (1.0 + x)) * self.lr).exp()
    }
}

/// A speed-law value at a key ([`SPEED_KEYS`]).
fn at_key(v: &[f64; 4], note: u8) -> f64 {
    let k = note as f64;
    if k <= SPEED_KEYS[0] {
        return v[0];
    }
    for i in 0..3 {
        if k <= SPEED_KEYS[i + 1] {
            let t = (k - SPEED_KEYS[i]) / (SPEED_KEYS[i + 1] - SPEED_KEYS[i]);
            return v[i] + t * (v[i + 1] - v[i]);
        }
    }
    v[3]
}

impl BodyBank {
    pub fn new(sample_rate: f64, law: &BodyLaw) -> Self {
        let freq = frequencies();
        let n = ((PULSE * sample_rate).round() as usize).max(2);
        let w: Vec<f64> = (0..n).map(|i| (std::f64::consts::PI * (i as f64 + 0.5) / n as f64).sin().powi(2)).collect();
        let sum: f64 = w.iter().sum();
        let mut pattern = vec![0.0f32; KEYS * MODES];
        let mut s = Self {
            freq,
            pr: vec![0.0; MODES],
            pi: vec![0.0; MODES],
            zr: vec![0.0; MODES],
            zi: vec![0.0; MODES],
            w: [vec![0.0; MODES], vec![0.0; MODES]],
            pattern: Vec::new(),
            pending: [Pending::default(); PENDING],
            speed_w: vec![1.0; PENDING * MODES],
            pulse: w.iter().map(|x| (x / sum) as f32).collect(),
            exc: vec![0.0; MODES],
            pulse_hr: vec![0.0; MODES],
            pulse_hi: vec![0.0; MODES],
            step_r: std::array::from_fn(|_| vec![1.0; MODES]),
            step_i: std::array::from_fn(|_| vec![0.0; MODES]),
            ph_r: vec![1.0; PENDING * MODES],
            ph_i: vec![0.0; PENDING * MODES],
            rise: 0,
            awake: false,
            sample_rate,
            law: *law,
        };
        s.fill_pattern(&mut pattern, law.shared);
        s.pattern = pattern;
        s.set_law(law);
        s
    }

    fn fill_pattern(&self, pattern: &mut [f32], shared: f64) {
        let shared = shared.clamp(0.0, 1.0);
        let (a, b) = (shared.sqrt(), (1.0 - shared).sqrt());
        for k in 0..KEYS {
            for i in 0..MODES {
                // Gaussian-ish draws (sum of two uniforms), unit rms each.
                let g = |x: f64, y: f64| ((hash(x, y) + hash(x + 0.37, y + 11.0)) - 1.0) * 6f64.sqrt();
                let common = g(1000.0, i as f64);
                let own = g(k as f64 + 21.0, i as f64 + 5000.0);
                pattern[k * MODES + i] = (a * common + b * own) as f32;
            }
        }
    }

    /// Applies a new law (levels, decays, pattern); real-time safe.
    pub fn set_law(&mut self, law: &BodyLaw) {
        if law.shared != self.law.shared {
            let mut p = std::mem::take(&mut self.pattern);
            self.fill_pattern(&mut p, law.shared);
            self.pattern = p;
        }
        self.law = *law;
        let rise = (law.rise.max(0.0) * self.sample_rate).round() as u32;
        self.rise = if rise as usize <= self.pulse.len() { 0 } else { rise };
        // Unit peak for MODES resonances of random sign and unit rms.
        let norm = 1.0 / (MODES as f64).sqrt();
        for i in 0..MODES {
            let f = self.freq[i];
            let t60 = band_value(&law.t60, f).max(1e-3);
            let r = (-6.91 / (t60 * self.sample_rate)).exp();
            let wv = std::f64::consts::TAU * f / self.sample_rate;
            self.pr[i] = (r * wv.cos()) as f32;
            self.pi[i] = (r * wv.sin()) as f32;
            for k in 0..SUB_TABLES {
                let n = 1i32 << k;
                let rn = r.powi(n);
                self.step_r[k][i] = (rn * (wv * n as f64).cos()) as f32;
                self.step_i[k][i] = (rn * (wv * n as f64).sin()) as f32;
            }
            // The pulse added sample by sample to a decaying, turning
            // resonance amounts to one kick of Σ_k w_k·p^(−k), p = r·e^(iω).
            let (mut hr, mut hi) = (0.0, 0.0);
            for (k, &x) in self.pulse.iter().enumerate() {
                let g = x as f64 * r.powi(-(k as i32));
                hr += g * (wv * k as f64).cos();
                hi -= g * (wv * k as f64).sin();
            }
            self.pulse_hr[i] = hr as f32;
            self.pulse_hi[i] = hi as f32;
            let level = 10f64.powf(band_value(&law.levels, f) / 20.0) * norm;
            for (ch, w) in self.w.iter_mut().enumerate() {
                let h = hash(i as f64 + 0.5, ch as f64 + 41.0);
                let sign = if hash(i as f64, ch as f64 + 17.0) < 0.5 { -1.0 } else { 1.0 };
                w[i] = (level * sign * (0.7 + 0.6 * h)) as f32;
            }
        }
    }

    pub fn law(&self) -> &BodyLaw {
        &self.law
    }

    /// The hammer of `note` meets its string `offset` s after the start of
    /// the next [`Self::render`] call, at `speed` m/s.
    pub fn strike(&mut self, note: u8, speed: f64, offset: f64) {
        let law = self.law;
        if !law.enabled || law.gain <= 0.0 || !(21..=108).contains(&note) {
            return;
        }
        let amp = law.gain * speed * 10f64.powf(law.slope * (note as f64 - 60.0) / 12.0 / 20.0);
        let wait = ((offset + law.delay) * self.sample_rate).round().max(0.0) as u32;
        // No room: the strike furthest along gives up what it had left.
        let j = match self.pending.iter().position(|p| p.amp == 0.0) {
            Some(j) => j,
            None => (0..PENDING).max_by_key(|&j| (self.pending[j].wait == 0, self.pending[j].age)).unwrap(),
        };
        let sub = if self.rise >= STEPS { (1u32 << (self.rise / STEPS).ilog2()).min(SUB) } else { 1 };
        self.pending[j] = Pending { key: note - 21, amp: amp as f32, wait, age: 0, rise: self.rise, sub };
        self.ph_r[j * MODES..(j + 1) * MODES].fill(1.0);
        self.ph_i[j * MODES..(j + 1) * MODES].fill(0.0);
        let tilt = SpeedTilt::of(&law, note, speed).scaled(at_key(&law.speed_halo, note));
        for (w, &f) in self.speed_w[j * MODES..(j + 1) * MODES].iter_mut().zip(&self.freq) {
            *w = tilt.gain(f) as f32;
        }
    }

    /// Adds the halo to the two microphone signals (pressure, Pa, times
    /// `gain`). Real-time safe.
    pub fn render(&mut self, out: [&mut [f32]; 2], gain: f32) {
        if !self.awake && self.pending.iter().all(|p| p.amp == 0.0) {
            return;
        }
        let [l, r] = out;
        for n in 0..l.len() {
            // Strikes under way this sample.
            let mut struck = false;
            for j in 0..PENDING {
                let p = self.pending[j];
                if p.amp == 0.0 {
                    continue;
                }
                if p.wait > 0 {
                    self.pending[j].wait -= 1;
                    continue;
                }
                let pat = &self.pattern[p.key as usize * MODES..(p.key as usize + 1) * MODES];
                let sw = &self.speed_w[j * MODES..(j + 1) * MODES];
                if p.rise > 0 {
                    // Settling in: every `sub` samples the next step of the
                    // ramp, turned as the resonance has since the strike.
                    if p.age.is_multiple_of(p.sub) {
                        let ramp = |a: u32| {
                            let x = (a as f64 / p.rise as f64).min(1.0);
                            x - (std::f64::consts::TAU * x).sin() / std::f64::consts::TAU
                        };
                        let d = (p.amp as f64 * (ramp(p.age + p.sub) - ramp(p.age))) as f32;
                        let t = p.sub.trailing_zeros() as usize;
                        let (sr, si) = (&self.step_r[t], &self.step_i[t]);
                        let phr = &mut self.ph_r[j * MODES..(j + 1) * MODES];
                        let phi = &mut self.ph_i[j * MODES..(j + 1) * MODES];
                        for i in 0..MODES {
                            let v = d * pat[i] * sw[i];
                            let (a, b) = (phr[i], phi[i]);
                            let (hr, hi) = (self.pulse_hr[i], self.pulse_hi[i]);
                            self.zr[i] += v * (a * hr - b * hi);
                            self.zi[i] += v * (a * hi + b * hr);
                            phr[i] = a * sr[i] - b * si[i];
                            phi[i] = a * si[i] + b * sr[i];
                        }
                        self.awake = true;
                        if p.age + p.sub >= p.rise {
                            self.pending[j].amp = 0.0;
                        }
                    }
                    self.pending[j].age += 1;
                    continue;
                }
                let a = p.amp * self.pulse[p.age as usize];
                if !struck {
                    self.exc.iter_mut().for_each(|e| *e = 0.0);
                    struck = true;
                }
                for ((e, &q), &w) in self.exc.iter_mut().zip(pat).zip(sw) {
                    *e += a * q * w;
                }
                self.pending[j].age += 1;
                if p.age as usize + 1 >= self.pulse.len() {
                    self.pending[j].amp = 0.0;
                }
            }
            if struck {
                for (z, &e) in self.zr.iter_mut().zip(&self.exc) {
                    *z += e;
                }
                self.awake = true;
            }
            if !self.awake {
                continue;
            }
            let mut acc_l = [0.0f32; LANES];
            let mut acc_r = [0.0f32; LANES];
            for c in 0..MODES / LANES {
                let s = c * LANES;
                let zr: &mut [f32; LANES] = (&mut self.zr[s..s + LANES]).try_into().unwrap();
                let zi: &mut [f32; LANES] = (&mut self.zi[s..s + LANES]).try_into().unwrap();
                let pr: &[f32; LANES] = self.pr[s..s + LANES].try_into().unwrap();
                let pi: &[f32; LANES] = self.pi[s..s + LANES].try_into().unwrap();
                let wl: &[f32; LANES] = self.w[0][s..s + LANES].try_into().unwrap();
                let wr: &[f32; LANES] = self.w[1][s..s + LANES].try_into().unwrap();
                for k in 0..LANES {
                    let (re, im) = (zr[k], zi[k]);
                    zr[k] = re * pr[k] - im * pi[k];
                    zi[k] = re * pi[k] + im * pr[k];
                    acc_l[k] += re * wl[k];
                    acc_r[k] += re * wr[k];
                }
            }
            l[n] += gain * acc_l.iter().sum::<f32>();
            r[n] += gain * acc_r.iter().sum::<f32>();
        }
        if self.awake && self.pending.iter().all(|p| p.amp == 0.0) {
            let e: f32 = self.zr.iter().zip(&self.zi).map(|(a, b)| a * a + b * b).sum();
            if e < 1e-16 {
                self.zr.iter_mut().for_each(|z| *z = 0.0);
                self.zi.iter_mut().for_each(|z| *z = 0.0);
                self.awake = false;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn law() -> BodyLaw {
        BodyLaw { enabled: true, gain: 1.0, ..BodyLaw::DEFAULT }
    }

    #[test]
    fn dense_below_the_knee() {
        let f = frequencies();
        assert!(f.windows(2).all(|w| w[1] > w[0]));
        let below = f.iter().filter(|&&x| (125.0..1000.0).contains(&x)).count();
        // One resonance every 3.5–5 Hz: they overlap at T60 ≤ 1 s.
        assert!((180..260).contains(&below), "{below}");
    }

    #[test]
    fn silent_until_struck_then_decays_per_band() {
        let sr = 48_000.0;
        let mut b = BodyBank::new(sr, &law());
        let (mut l, mut r) = (vec![0.0f32; 4800], vec![0.0f32; 4800]);
        b.render([&mut l, &mut r], 1.0);
        assert!(l.iter().chain(&r).all(|&x| x == 0.0));
        b.strike(60, 1.0, 0.0);
        let n = (sr * 1.2) as usize;
        let (mut l, mut r) = (vec![0.0f32; n], vec![0.0f32; n]);
        b.render([&mut l, &mut r], 1.0);
        let rms = |a: usize, b: usize| (l[a..b].iter().map(|x| x * x).sum::<f32>() / (b - a) as f32).sqrt();
        let early = rms(4800, 9600);
        let late = rms(52800, 57600);
        assert!(early > 0.0 && l.iter().all(|x| x.is_finite()));
        // T60 0.6–0.1 s across the bands: 1 s later, at least 40 dB down.
        assert!(late < early * 0.01, "{early} {late}");
        // The two microphones differ.
        assert!(l.iter().zip(&r).any(|(a, b)| (a - b).abs() > 1e-6));
    }

    #[test]
    fn a_soft_blow_darkens_the_halo() {
        // Energy of the halo above and below 1.3 kHz after one strike.
        let bands = |law: &BodyLaw, speed: f64| {
            let sr = 48_000.0;
            let mut b = BodyBank::new(sr, law);
            b.strike(40, speed, 0.0);
            let n = 4096;
            let (mut l, mut r) = (vec![0.0f32; n], vec![0.0f32; n]);
            b.render([&mut l, &mut r], 1.0);
            let (mut lo, mut hi) = (0.0, 0.0);
            for k in 1..n / 2 {
                let f = k as f64 * sr / n as f64;
                let w = std::f64::consts::TAU * k as f64 / n as f64;
                let (mut re, mut im) = (0.0, 0.0);
                for (i, &x) in l.iter().enumerate() {
                    re += x as f64 * (w * i as f64).cos();
                    im += x as f64 * (w * i as f64).sin();
                }
                if (60.0..300.0).contains(&f) {
                    lo += re * re + im * im;
                } else if (2000.0..6000.0).contains(&f) {
                    hi += re * re + im * im;
                }
            }
            (lo, hi)
        };
        let plain = law();
        let fitted = BodyLaw { speed_ref: 2.8, speed_corner: 1300.0, speed_low: [0.0; 4], speed_high: [2.5, 1.5, 1.0, 0.0], ..plain };
        // At the reference speed the law changes nothing.
        let (a, b) = (bands(&plain, 2.8), bands(&fitted, 2.8));
        assert!((a.0 / b.0 - 1.0).abs() < 1e-3 && (a.1 / b.1 - 1.0).abs() < 1e-3, "{a:?} {b:?}");
        // Three times slower (9.6 dB): the low band follows the speed alone,
        // the top (E2: γ ≈ 1.8 well above the corner, 1.4–1.7 over 2–6 kHz)
        // falls some 14 dB further.
        let (p, q) = (bands(&plain, 0.93), bands(&fitted, 0.93));
        assert!((q.0 / p.0 - 1.0).abs() < 0.12, "{p:?} {q:?}");
        let drop = 10.0 * (q.1 / p.1).log10();
        assert!((-17.0..-11.0).contains(&drop), "top {drop:.1} dB");
    }

    #[test]
    fn a_rise_keeps_the_tail_and_spares_the_attack() {
        // No delay: the strike's first sample is the bank's first sample.
        let sr = 48_000.0;
        // (T60 of every band, rise, attack window, tail window): the
        // Concert grand's rise, and a short one at the shortest T60.
        for (t60, rise, attack, tail) in [(1.0, 0.1, (0.0, 0.01), (0.15, 0.6)), (0.02, 0.005, (0.0, 0.0005), (0.0055, 0.01))] {
            let render = |rise: f64| {
                let law = BodyLaw { t60: [t60; 8], rise, delay: 0.0, ..law() };
                let mut b = BodyBank::new(sr, &law);
                b.strike(45, 1.0, 0.0);
                let n = (sr * tail.1) as usize;
                let (mut l, mut r) = (vec![0.0f32; n], vec![0.0f32; n]);
                b.render([&mut l, &mut r], 1.0);
                l
            };
            let (struck, settled) = (render(0.0), render(rise));
            let db = |x: &[f32], (a, b): (f64, f64)| {
                let s = &x[(a * sr) as usize..(b * sr) as usize];
                10.0 * (s.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / s.len() as f64).log10()
            };
            // Over the attack the settling halo is far under the struck one…
            let early = db(&settled, attack) - db(&struck, attack);
            assert!(early < -25.0, "T60 {t60}: {early:.1} dB");
            // …and once settled it is the same halo, sample for sample.
            let level = (db(&settled, tail) - db(&struck, tail)).abs();
            let diff: Vec<f32> = struck.iter().zip(&settled).map(|(a, b)| a - b).collect();
            assert!(level < 0.1, "T60 {t60}: {level:.2} dB");
            let rest = db(&diff, tail) - db(&struck, tail);
            assert!(rest < -30.0, "T60 {t60}: {rest:.1} dB");
            // No step at its start: over the rise's first hundredth the
            // halo is still tiny.
            let peak = struck.iter().fold(0.0f32, |m, v| m.max(v.abs()));
            let n = (rise * sr / 100.0) as usize;
            assert!(settled[..n].iter().all(|v| v.abs() < 1e-3 * peak), "T60 {t60}");
        }
    }

    #[test]
    fn a_rise_of_any_length_starts_as_gently_as_the_pulse() {
        // The strike's first sample (no delay), re the peak, for rises on
        // both sides of every change of step (256, 512, 1024, 2048, 4096
        // samples) and of the pulse's own length (12 samples at 48 kHz).
        let sr = 48_000.0;
        let first = |n: u32| {
            let law = BodyLaw { t60: [0.6; 8], rise: n as f64 / sr, delay: 0.0, ..law() };
            let mut b = BodyBank::new(sr, &law);
            b.strike(52, 1.0, 0.0);
            let len = 9600;
            let (mut l, mut r) = (vec![0.0f32; len], vec![0.0f32; len]);
            b.render([&mut l, &mut r], 1.0);
            let peak = l.iter().fold(0.0f32, |m, v| m.max(v.abs()));
            l[0].abs() / peak
        };
        let pulse = first(0);
        assert!(pulse > 0.0 && pulse < 0.01, "{pulse}");
        for n in [5, 12, 13, 24, 127, 128, 255, 256, 511, 512, 1023, 1024, 2047, 2048, 4095, 4096, 4800] {
            let x = first(n);
            assert!(x <= 1.5 * pulse, "rise {n} samples: first sample {x:.5} of the peak, the pulse's {pulse:.5}");
        }
    }

    #[test]
    fn off_or_zero_gain_is_silent() {
        let mut b = BodyBank::new(48_000.0, &BodyLaw::DEFAULT);
        b.strike(60, 3.0, 0.0);
        let (mut l, mut r) = (vec![0.0f32; 2000], vec![0.0f32; 2000]);
        b.render([&mut l, &mut r], 1.0);
        assert!(l.iter().chain(&r).all(|&x| x == 0.0));
    }
}
