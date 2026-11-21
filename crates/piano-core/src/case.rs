//! The case's knock, heard directly: the blow of each key shakes the keybed
//! and the case, and their wood answers with its own few resonances, the
//! same under every key. On real grands it is what the keys from C6 up have
//! below their fundamental from 80 to 630 Hz: 24–30 dB under the note's
//! first 100 ms per third octave, at about the same level from 80 to 630 Hz,
//! next to nothing below 75 Hz, dying in 0.8 s at 80 Hz and 0.3 s at 500 Hz
//! (docs/case_knock.md). The action's knock ([`crate::action`]) is shorter
//! and brighter; the body's halo ([`crate::body`]) comes in later and is
//! dense. Neither rings as a box that was knocked on.
//!
//! A sparse bank of damped resonances shared by all the keys, one every
//! 34 Hz on average from [`NoiseLaw::case_low`](crate::params::NoiseLaw)
//! up, as the modes of thick wooden panels are; each key strikes it with its
//! own fixed amplitudes, the lowest resonances with one sign for every key.
//! The source is small against the wavelengths it radiates: the two sides
//! hear the same knock, the second [`FAR`] later, so that they agree at
//! 80–160 Hz and drift apart above 200 Hz as two microphones 0.3 m further
//! one than the other do.

/// Resonances of the bank.
const K: usize = 28;
/// Strikes under way.
const PENDING: usize = 16;
/// Length (s) of a strike: a Hann pulse, so that the knock starts without a
/// step.
const PULSE: f64 = 1e-3;
/// Mean spacing (Hz) of the resonances.
const SPACING: f64 = 34.0;
/// Above this the knock is left to the action's ([`crate::action`]): it
/// falls at 18 dB per octave.
const TOP: f64 = 600.0;
/// How much later (s) the second side hears the knock: 0.3 m of path.
const FAR: f64 = 0.3 / 343.0;

#[derive(Clone, Copy, Default)]
struct Pending {
    note: u8,
    amp: f32,
    wait: u32,
    /// Samples since the strike began.
    age: u32,
}

pub struct CaseKnock {
    /// Pole per resonance (rotation times decay), as (re, im).
    pole: [(f32, f32); K],
    weight: [f32; K],
    freq: [f64; K],
    /// State per side and resonance.
    state: [[(f32, f32); K]; 2],
    pending: [Pending; PENDING],
    /// The pulse, summing to 1.
    pulse: Vec<f32>,
    /// Samples by which the second side lags.
    far: u32,
    awake: bool,
    sample_rate: f64,
    low: f64,
    time: f64,
}

fn hash(a: f64, b: f64) -> f64 {
    ((a * 12.9898 + b * 78.233).sin() * 43_758.545_3).rem_euclid(1.0)
}

impl CaseKnock {
    /// Resonances from `low` (Hz) up; `time` is their T60 (s) at 200 Hz,
    /// shorter above and longer below as (200/f)^0.5.
    pub fn new(sample_rate: f64, low: f64, time: f64) -> Self {
        let mut s = Self {
            pole: [(0.0, 0.0); K],
            weight: [0.0; K],
            freq: [0.0; K],
            state: [[(0.0, 0.0); K]; 2],
            pending: [Pending::default(); PENDING],
            pulse: Vec::new(),
            far: (FAR * sample_rate).round() as u32,
            awake: false,
            sample_rate,
            low: 0.0,
            time: 0.0,
        };
        let n = ((PULSE * sample_rate) as usize).max(2);
        let w: Vec<f64> = (0..n).map(|i| (std::f64::consts::PI * (i as f64 + 0.5) / n as f64).sin().powi(2)).collect();
        let sum: f64 = w.iter().sum();
        s.pulse = w.iter().map(|x| (x / sum) as f32).collect();
        s.set(low, time);
        s
    }

    /// The lowest resonance (Hz) and the decay time (s) at 200 Hz. Real-time
    /// safe; what rings keeps ringing at the new frequencies.
    pub fn set(&mut self, low: f64, time: f64) {
        if low == self.low && time == self.time {
            return;
        }
        (self.low, self.time) = (low, time);
        let mut f = low.clamp(20.0, 400.0);
        for i in 0..K {
            if i > 0 {
                f += SPACING * (0.6 + 0.8 * hash(i as f64, 3.0));
            }
            self.freq[i] = f;
            self.weight[i] = 10f64.powf(-18.0 * (f / TOP).log2().max(0.0) / 20.0) as f32;
            let sigma = 6.91 / (time.max(1e-3) * (200.0 / f).sqrt());
            let r = (-sigma / self.sample_rate).exp();
            let w = std::f64::consts::TAU * f / self.sample_rate;
            self.pole[i] = ((r * w.cos()) as f32, (r * w.sin()) as f32);
        }
    }

    /// A key's blow `delay` s after the start of the next [`Self::render`]
    /// call, with a knock of peak pressure `amp` (Pa at the microphones).
    pub fn strike(&mut self, note: u8, amp: f64, delay: f64) {
        if amp <= 0.0 {
            return;
        }
        let wait = (delay * self.sample_rate).max(0.0) as u32;
        let slot = self.pending.iter_mut().find(|p| p.amp == 0.0);
        let slot = match slot {
            Some(p) => p,
            // All busy: the one furthest on gives way (dense chords).
            None => self.pending.iter_mut().max_by_key(|p| p.age).unwrap(),
        };
        *slot = Pending { note, amp: amp as f32, wait, age: 0 };
    }

    fn excite(&mut self, side: usize, note: u8, amp: f32) {
        let norm = amp / (K as f32).sqrt();
        for i in 0..K {
            // ±3 dB per key and resonance. The lowest resonances move the
            // same way under every key; higher up a key's place decides.
            let a = norm * self.weight[i] * 10f32.powf(3.0 * (2.0 * hash(note as f64, i as f64 + 7.0) as f32 - 1.0) / 20.0);
            let flip = hash(note as f64 + 0.5, i as f64) < 0.5 * (i as f64 / 8.0).min(1.0);
            self.state[side][i].0 += if flip { -a } else { a };
        }
        self.awake = true;
    }

    /// Adds the knocks to the two sides (pressure, Pa, times `gain`).
    /// Real-time safe.
    pub fn render(&mut self, out: [&mut [f32]; 2], gain: f32) {
        let len = out[0].len();
        if !self.awake && self.pending.iter().all(|p| p.amp == 0.0) {
            return;
        }
        let [l, r] = out;
        let n_pulse = self.pulse.len() as u32;
        for n in 0..len {
            for j in 0..PENDING {
                let p = self.pending[j];
                if p.amp == 0.0 {
                    continue;
                }
                if p.wait > 0 {
                    self.pending[j].wait -= 1;
                    continue;
                }
                if p.age < n_pulse {
                    self.excite(0, p.note, p.amp * self.pulse[p.age as usize]);
                }
                if p.age >= self.far && p.age - self.far < n_pulse {
                    self.excite(1, p.note, p.amp * self.pulse[(p.age - self.far) as usize]);
                }
                self.pending[j].age += 1;
                if p.age + 1 >= n_pulse + self.far {
                    self.pending[j].amp = 0.0;
                }
            }
            if !self.awake {
                continue;
            }
            let mut acc = [0.0f32; 2];
            for (ch, st) in self.state.iter_mut().enumerate() {
                for (z, &(pr, pi)) in st.iter_mut().zip(&self.pole) {
                    let (re, im) = *z;
                    *z = (re * pr - im * pi, re * pi + im * pr);
                    acc[ch] += re;
                }
            }
            l[n] += gain * acc[0];
            r[n] += gain * acc[1];
        }
        if self.awake && self.pending.iter().all(|p| p.amp == 0.0) {
            let e: f32 = self.state.iter().flatten().map(|(a, b)| a * a + b * b).sum();
            if e < 1e-14 {
                self.state = [[(0.0, 0.0); K]; 2];
                self.awake = false;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn knock(note: u8, secs: f64) -> [Vec<f32>; 2] {
        let sr = 48_000.0;
        let mut c = CaseKnock::new(sr, 80.0, 0.55);
        c.strike(note, 1.0, 0.0);
        let n = (secs * sr) as usize;
        let (mut l, mut r) = (vec![0.0f32; n], vec![0.0f32; n]);
        for (a, b) in l.chunks_mut(128).zip(r.chunks_mut(128)) {
            c.render([a, b], 1.0);
        }
        [l, r]
    }

    /// Power of `x` at `f` over `from..to` seconds (Hann window).
    fn power(x: &[f32], f: f64, from: f64, to: f64) -> f64 {
        let sr = 48_000.0;
        let (a, b) = ((from * sr) as usize, (to * sr) as usize);
        let n = b - a;
        let (mut re, mut im) = (0.0, 0.0);
        for (i, &v) in x[a..b].iter().enumerate() {
            let w = (std::f64::consts::PI * i as f64 / n as f64).sin().powi(2);
            let ph = std::f64::consts::TAU * f * i as f64 / sr;
            re += w * v as f64 * ph.cos();
            im += w * v as f64 * ph.sin();
        }
        (re * re + im * im) / (n * n) as f64
    }

    #[test]
    fn nothing_below_the_lowest_resonance_and_the_low_end_outlasts_the_top() {
        let [l, _] = knock(96, 1.0);
        // The lowest resonance stands out of what lies under it.
        assert!(power(&l, 80.0, 0.0, 0.5) > 30.0 * power(&l, 45.0, 0.0, 0.5));
        // 80 Hz loses less from the first 0.2 s to 0.4–0.6 s than the bank near 500 Hz.
        let fall = |f: f64| power(&l, f, 0.0, 0.2) / power(&l, f, 0.4, 0.6);
        let top: f64 = (0..8).map(|i| fall(450.0 + 15.0 * i as f64)).sum::<f64>() / 8.0;
        assert!(fall(80.0) * 30.0 < top, "{} {}", fall(80.0), top);
    }

    #[test]
    fn the_two_sides_agree_at_the_bottom_and_part_above() {
        let [l, r] = knock(84, 0.3);
        let corr = |lo: f64, hi: f64| {
            // Crude band-pass: a difference of two one-pole low-passes, twice.
            let band = |x: &[f32]| {
                let lp = |x: &[f64], fc: f64| {
                    let k = 1.0 - (-std::f64::consts::TAU * fc / 48_000.0).exp();
                    let mut y = 0.0;
                    x.iter().map(|&v| { y += k * (v - y); y }).collect::<Vec<f64>>()
                };
                let x: Vec<f64> = x.iter().map(|&v| v as f64).collect();
                let a = lp(&lp(&x, hi), hi);
                let b = lp(&lp(&a, lo), lo);
                a.iter().zip(&b).map(|(p, q)| p - q).collect::<Vec<f64>>()
            };
            let (a, b) = (band(&l), band(&r));
            let dot = |p: &[f64], q: &[f64]| p.iter().zip(q).map(|(x, y)| x * y).sum::<f64>();
            dot(&a, &b) / (dot(&a, &a) * dot(&b, &b)).sqrt()
        };
        assert!(corr(60.0, 110.0) > 0.7, "{}", corr(60.0, 110.0));
        assert!(corr(350.0, 500.0) < 0.2, "{}", corr(350.0, 500.0));
    }

    #[test]
    fn silent_when_not_struck_and_asleep_once_it_has_died() {
        let mut c = CaseKnock::new(48_000.0, 80.0, 0.55);
        let (mut l, mut r) = (vec![0.0f32; 256], vec![0.0f32; 256]);
        c.render([&mut l, &mut r], 1.0);
        assert!(l.iter().chain(&r).all(|&v| v == 0.0));
        c.strike(60, 1e-3, 0.0);
        for _ in 0..3000 {
            c.render([&mut l, &mut r], 1.0);
        }
        assert!(!c.awake);
    }
}
