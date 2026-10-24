//! The action's own knock, heard directly: the key bottoming on the keybed,
//! the hammer shank and the wippen clattering, radiated by the keyframe and
//! the action rather than through the soundboard. On eight real grands it is
//! what the top keys have below their fundamental: a knock of about the same
//! level at every key, 20–27 dB under a middle note's attack at 125–500 Hz
//! and 16–20 dB at 1–2 kHz, peaking some 10 ms after the hammer reaches the
//! string and dying within about 0.1 s (docs/action_noise.md).
//!
//! A bank of damped resonances shared by all the keys (the keyframe and the
//! action's wooden parts), struck by each key with its own fixed pattern of
//! amplitudes and signs, and heard by each microphone according to where
//! the key lies along the keyboard.

/// Resonances of the bank.
const K: usize = 24;
/// Strikes waiting for their delay or under way.
const PENDING: usize = 16;
/// Length (s) of a strike: a Hann pulse, so that the knock starts without a
/// step (a one-sample impulse clicked).
const PULSE: f64 = 6e-4;

#[derive(Clone, Copy, Default)]
struct Pending {
    note: u8,
    amp: f32,
    wait: u32,
    /// Samples of the pulse already given.
    age: u32,
}

pub struct ActionNoise {
    /// Pole per resonance (rotation times decay), as (re, im).
    pole: [(f32, f32); K],
    /// Spectral weight of each resonance.
    weight: [f32; K],
    freq: [f64; K],
    /// State per microphone and resonance.
    state: [[(f32, f32); K]; 2],
    pending: [Pending; PENDING],
    /// The pulse, summing to 1.
    pulse: Vec<f32>,
    awake: bool,
    sample_rate: f64,
}

fn hash(a: f64, b: f64) -> f64 {
    ((a * 12.9898 + b * 78.233).sin() * 43_758.545_3).rem_euclid(1.0)
}

impl ActionNoise {
    /// Resonances log-spaced from 80 Hz to 2.6 kHz; `t60` (s) at 500 Hz,
    /// shorter above as (500/f)^0.3. Spectral weight flat to 500 Hz, +6 dB
    /// at 1–1.5 kHz, then falling at 36 dB per octave: the real knocks have
    /// next to nothing at 4–8 kHz.
    pub fn new(sample_rate: f64, t60: f64) -> Self {
        let mut s = Self {
            pole: [(0.0, 0.0); K],
            weight: [0.0; K],
            freq: [0.0; K],
            state: [[(0.0, 0.0); K]; 2],
            pending: [Pending::default(); PENDING],
            pulse: Vec::new(),
            awake: false,
            sample_rate,
        };
        let n = ((PULSE * sample_rate) as usize).max(2);
        let w: Vec<f64> = (0..n).map(|i| (std::f64::consts::PI * (i as f64 + 0.5) / n as f64).sin().powi(2)).collect();
        let sum: f64 = w.iter().sum();
        s.pulse = w.iter().map(|x| (x / sum) as f32).collect();
        for i in 0..K {
            let t = i as f64 / (K - 1) as f64;
            let f = 80.0 * (2600.0f64 / 80.0).powf(t) * (1.0 + 0.08 * (2.0 * hash(i as f64, 1.0) - 1.0));
            s.freq[i] = f;
            let lift = 6.0 * (f / 500.0).log2().clamp(0.0, 1.0);
            let fall = -36.0 * (f / 1500.0).log2().max(0.0);
            s.weight[i] = 10f64.powf((lift + fall) / 20.0) as f32;
        }
        s.set_time(t60);
        s
    }

    pub fn set_time(&mut self, t60: f64) {
        for i in 0..K {
            let f = self.freq[i];
            let sigma = 6.91 / (t60.max(1e-3) * (500.0 / f).powf(0.3));
            let r = (-sigma / self.sample_rate).exp();
            let w = std::f64::consts::TAU * f / self.sample_rate;
            self.pole[i] = ((r * w.cos()) as f32, (r * w.sin()) as f32);
        }
    }

    /// A key reaches the bottom of its travel `delay` s from now, with a
    /// knock of peak pressure `amp` (Pa at the microphones).
    pub fn strike(&mut self, note: u8, amp: f64, delay: f64) {
        if amp <= 0.0 {
            return;
        }
        let wait = (delay * self.sample_rate).max(0.0) as u32;
        let slot = self.pending.iter_mut().find(|p| p.amp == 0.0);
        let slot = match slot {
            Some(p) => p,
            // All busy: the soonest replaces itself (dense chords).
            None => self.pending.iter_mut().min_by_key(|p| p.wait).unwrap(),
        };
        *slot = Pending { note, amp: amp as f32, wait, age: 0 };
    }

    fn excite(&mut self, note: u8, amp: f32) {
        // Where the key lies: the left microphone over the bass, the right
        // over the treble; both hear the whole action.
        let x = ((note as f32 - 21.0) / 87.0).clamp(0.0, 1.0);
        let gains = [1.0 - 0.5 * x, 0.5 + 0.5 * x];
        // Unit peak for K resonances of random sign and weight ~1.
        let norm = amp / (K as f32).sqrt();
        for i in 0..K {
            let h = hash(note as f64, i as f64 + 7.0);
            // ±4 dB per key and resonance, random sign.
            let a = norm * self.weight[i] * 10f32.powf((4.0 * (2.0 * h as f32 - 1.0)) / 20.0);
            let a = if hash(note as f64 + 0.5, i as f64) < 0.5 { -a } else { a };
            for (ch, g) in gains.iter().enumerate() {
                self.state[ch][i].0 += a * g;
            }
        }
        self.awake = true;
    }

    /// Adds the knocks to the two microphone signals (pressure, Pa, times
    /// `gain`). Real-time safe.
    pub fn render(&mut self, out: [&mut [f32]; 2], gain: f32) {
        let len = out[0].len();
        if !self.awake && self.pending.iter().all(|p| p.amp == 0.0) {
            return;
        }
        let [l, r] = out;
        for n in 0..len {
            for j in 0..PENDING {
                let p = self.pending[j];
                if p.amp != 0.0 {
                    if p.wait > 0 {
                        self.pending[j].wait -= 1;
                    } else {
                        self.excite(p.note, p.amp * self.pulse[p.age as usize]);
                        self.pending[j].age += 1;
                        if p.age as usize + 1 >= self.pulse.len() {
                            self.pending[j].amp = 0.0;
                        }
                    }
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
        if self.awake {
            let e: f32 = self.state.iter().flatten().map(|(a, b)| a * a + b * b).sum();
            if e < 1e-14 {
                self.state = [[(0.0, 0.0); K]; 2];
                self.awake = false;
            }
        }
    }
}
