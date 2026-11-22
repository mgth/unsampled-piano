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

use crate::modal::Isa;
use crate::params::BodyLaw;
use crate::simd::V16;

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

/// A strike on its way: it takes a slot at its own event frame, `at`
/// samples into the render, whatever the slots hold then (a free one, or
/// the one whose strike has the least excitation left). Choosing then, not
/// when it is queued, makes the halo the same however the render is cut
/// into blocks.
#[derive(Clone, Copy, Default)]
struct Arrival {
    at: u32,
    next: Pending,
    tilt: SpeedTilt,
}

/// Strikes due later within one render (an offset). A strike due at once
/// (offset 0, as the engine gives them, rendering the bank up to each
/// event) takes its slot straight away and never waits here; beyond this
/// many later ones, a strike is lost.
const ARRIVALS: usize = 64;

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
    /// Strikes on their way, in the order they came; `arrived` of them.
    arrivals: [Arrival; ARRIVALS],
    arrived: usize,
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
    /// The tables of the resonances (poles, steps, pulse) and of the
    /// microphones' weights hold for `law`: a live change that leaves the
    /// decays or the levels as they were skips their rebuild.
    built: bool,
    awake: bool,
    sample_rate: f64,
    law: BodyLaw,
    isa: Isa,
}

/// The share of a strike settled in after `age` samples of a rise of `rise`
/// samples: a raised-cosine ramp from 0 to 1.
fn ramp(age: u32, rise: u32) -> f64 {
    let x = (age as f64 / rise as f64).min(1.0);
    x - (std::f64::consts::TAU * x).sin() / std::f64::consts::TAU
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
            arrivals: [Arrival::default(); ARRIVALS],
            arrived: 0,
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
            built: false,
            awake: false,
            sample_rate,
            law: *law,
            isa: Isa::detect(),
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
        let poles = !self.built || law.t60 != self.law.t60;
        let weights = !self.built || law.levels != self.law.levels;
        self.built = true;
        self.law = *law;
        let rise = (law.rise.max(0.0) * self.sample_rate).round() as u32;
        self.rise = if rise as usize <= self.pulse.len() { 0 } else { rise };
        if poles {
            for i in 0..MODES {
                let f = self.freq[i];
                let t60 = band_value(&law.t60, f).max(1e-3);
                let r = (-6.91 / (t60 * self.sample_rate)).exp();
                let wv = std::f64::consts::TAU * f / self.sample_rate;
                let (cr, ci) = (r * wv.cos(), r * wv.sin());
                self.pr[i] = cr as f32;
                self.pi[i] = ci as f32;
                // p^(2^k) by squaring.
                let (mut sr, mut si) = (cr, ci);
                for k in 0..SUB_TABLES {
                    self.step_r[k][i] = sr as f32;
                    self.step_i[k][i] = si as f32;
                    (sr, si) = (sr * sr - si * si, 2.0 * sr * si);
                }
                // The pulse added sample by sample to a decaying, turning
                // resonance amounts to one kick of Σ_k w_k·p^(−k), p = r·e^(iω).
                let (qr, qi) = (wv.cos() / r, -wv.sin() / r);
                let (mut gr, mut gi, mut hr, mut hi) = (1.0, 0.0, 0.0, 0.0);
                for &x in &self.pulse {
                    hr += x as f64 * gr;
                    hi += x as f64 * gi;
                    (gr, gi) = (gr * qr - gi * qi, gr * qi + gi * qr);
                }
                self.pulse_hr[i] = hr as f32;
                self.pulse_hi[i] = hi as f32;
            }
        }
        if weights {
            // Unit peak for MODES resonances of random sign and unit rms.
            let norm = 1.0 / (MODES as f64).sqrt();
            for i in 0..MODES {
                let level = 10f64.powf(band_value(&law.levels, self.freq[i]) / 20.0) * norm;
                for (ch, w) in self.w.iter_mut().enumerate() {
                    let h = hash(i as f64 + 0.5, ch as f64 + 41.0);
                    let sign = if hash(i as f64, ch as f64 + 17.0) < 0.5 { -1.0 } else { 1.0 };
                    w[i] = (level * sign * (0.7 + 0.6 * h)) as f32;
                }
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
        let sub = if self.rise >= STEPS { (1u32 << (self.rise / STEPS).ilog2()).min(SUB) } else { 1 };
        let next = Pending { key: note - 21, amp: amp as f32, wait, age: 0, rise: self.rise, sub };
        let tilt = SpeedTilt::of(&law, note, speed).scaled(at_key(&law.speed_halo, note));
        let at = (offset * self.sample_rate).round().max(0.0) as u32;
        let a = Arrival { at, next: Pending { wait: wait.saturating_sub(at), ..next }, tilt };
        if at == 0 {
            // Due now: it takes its slot at once, after those already due.
            self.arrive_due();
            self.arrive(a);
        } else if self.arrived < ARRIVALS {
            self.arrivals[self.arrived] = a;
            self.arrived += 1;
        }
    }

    /// The queued strikes due at this frame take their slots, in the order
    /// they came; the others count down one sample.
    fn arrive_due(&mut self) {
        let mut kept = 0;
        for i in 0..self.arrived {
            let a = self.arrivals[i];
            if a.at == 0 {
                self.arrive(a);
            } else {
                self.arrivals[kept] = a;
                kept += 1;
            }
        }
        self.arrived = kept;
    }

    /// A strike reaching its frame takes a free slot, or else the slot of
    /// the strike with the least excitation left, which first takes that
    /// rest at once.
    fn arrive(&mut self, a: Arrival) {
        let j = match self.pending.iter().position(|p| p.amp == 0.0) {
            Some(j) => j,
            None => {
                let j = self.victim();
                self.settle_rest(j);
                j
            }
        };
        self.install(j, a.next, a.tilt);
    }

    fn install(&mut self, j: usize, p: Pending, tilt: SpeedTilt) {
        self.pending[j] = p;
        self.ph_r[j * MODES..(j + 1) * MODES].fill(1.0);
        self.ph_i[j * MODES..(j + 1) * MODES].fill(0.0);
        for (w, &f) in self.speed_w[j * MODES..(j + 1) * MODES].iter_mut().zip(&self.freq) {
            *w = tilt.gain(f) as f32;
        }
    }

    /// The slot to give up when all are taken: the strike under way with the
    /// least of its excitation left (one still waiting only if all are).
    fn victim(&self) -> usize {
        let left = |p: &Pending| -> f64 {
            let done = if p.rise > 0 {
                ramp(p.age.div_ceil(p.sub) * p.sub, p.rise)
            } else {
                self.pulse[..(p.age as usize).min(self.pulse.len())].iter().map(|&x| x as f64).sum()
            };
            p.amp as f64 * (1.0 - done).max(0.0)
        };
        (0..PENDING)
            .min_by(|&a, &b| {
                let (pa, pb) = (&self.pending[a], &self.pending[b]);
                (pa.wait > 0).cmp(&(pb.wait > 0)).then(left(pa).total_cmp(&left(pb)))
            })
            .unwrap_or(0)
    }

    /// A settling strike giving its slot away takes the rest of its ramp at
    /// once, in phase with what it already rings, so that its tail is the
    /// same as if it had settled in: only the rest's onset comes early. A
    /// pulse's last samples (under 0.25 ms) are dropped. Real-time safe.
    fn settle_rest(&mut self, j: usize) {
        let p = self.pending[j];
        if p.amp == 0.0 || p.rise == 0 || p.wait > 0 {
            return;
        }
        // What has been added so far reaches the next step `next`, with the
        // phasor at p^next; the rest goes in now, at `age`: turned back by
        // p^(next − age).
        let next = p.age.div_ceil(p.sub) * p.sub;
        let d = p.amp as f64 * (1.0 - ramp(next, p.rise));
        let m = next - p.age;
        let pat = &self.pattern[p.key as usize * MODES..(p.key as usize + 1) * MODES];
        let sw = &self.speed_w[j * MODES..(j + 1) * MODES];
        for i in 0..MODES {
            // p^m from the tables of p^(2^k), then its inverse.
            let (mut mr, mut mi) = (1.0f64, 0.0f64);
            for k in 0..SUB_TABLES {
                if m >> k & 1 == 1 {
                    let (sr, si) = (self.step_r[k][i] as f64, self.step_i[k][i] as f64);
                    (mr, mi) = (mr * sr - mi * si, mr * si + mi * sr);
                }
            }
            let n2 = mr * mr + mi * mi;
            let (qr, qi) = (mr / n2, -mi / n2);
            let (a, b) = (self.ph_r[j * MODES + i] as f64, self.ph_i[j * MODES + i] as f64);
            let (hr, hi) = (self.pulse_hr[i] as f64, self.pulse_hi[i] as f64);
            let (ur, ui) = (a * hr - b * hi, a * hi + b * hr);
            let (er, ei) = (ur * qr - ui * qi, ur * qi + ui * qr);
            let v = d * pat[i] as f64 * sw[i] as f64;
            self.zr[i] += (v * er) as f32;
            self.zi[i] += (v * ei) as f32;
        }
        self.awake = true;
    }

    /// Panics on an ISA this CPU does not have ([`Isa::available`]).
    pub fn set_isa(&mut self, isa: Isa) {
        assert!(Isa::available().contains(&isa), "{isa:?} is not available on this CPU");
        self.isa = isa;
    }

    /// Adds the halo to the two microphone signals (pressure, Pa, times
    /// `gain`). Real-time safe. The same result on every ISA: plain
    /// multiplies and adds, resonance by resonance.
    pub fn render(&mut self, out: [&mut [f32]; 2], gain: f32) {
        match self.isa {
            #[cfg(target_arch = "aarch64")]
            Isa::Generic => self.render_v::<crate::simd::arm::N>(out, gain),
            #[cfg(not(target_arch = "aarch64"))]
            Isa::Generic => self.render_v::<[f32; LANES]>(out, gain),
            // SAFETY: only selectable when detected.
            #[cfg(target_arch = "x86_64")]
            Isa::Avx2 => unsafe { self.render_avx2(out, gain) },
            #[cfg(target_arch = "x86_64")]
            Isa::Avx512 => unsafe { self.render_avx512(out, gain) },
        }
    }

    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "avx2")]
    fn render_avx2(&mut self, out: [&mut [f32]; 2], gain: f32) {
        self.render_v::<crate::simd::x86::Y>(out, gain)
    }

    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "avx512f")]
    fn render_avx512(&mut self, out: [&mut [f32]; 2], gain: f32) {
        self.render_v::<crate::simd::x86::Z>(out, gain)
    }

    #[inline(always)]
    fn render_v<V: V16>(&mut self, out: [&mut [f32]; 2], gain: f32) {
        if !self.awake && self.arrived == 0 && self.pending.iter().all(|p| p.amp == 0.0) {
            return;
        }
        let [l, r] = out;
        for n in 0..l.len() {
            // Strikes reaching their frame take their slots, in the order
            // they came; the others come one sample nearer.
            if self.arrived > 0 {
                self.arrive_due();
                for a in &mut self.arrivals[..self.arrived] {
                    a.at -= 1;
                }
            }
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
                let pat = lanes(&self.pattern[p.key as usize * MODES..(p.key as usize + 1) * MODES]);
                let sw = lanes(&self.speed_w[j * MODES..(j + 1) * MODES]);
                if p.rise > 0 {
                    // Settling in: every `sub` samples the next step of the
                    // ramp, turned as the resonance has since the strike.
                    if p.age.is_multiple_of(p.sub) {
                        let d = V::splat((p.amp as f64 * (ramp(p.age + p.sub, p.rise) - ramp(p.age, p.rise))) as f32);
                        let t = p.sub.trailing_zeros() as usize;
                        let (sr, si) = (lanes(&self.step_r[t]), lanes(&self.step_i[t]));
                        let (hr, hi) = (lanes(&self.pulse_hr), lanes(&self.pulse_hi));
                        let phr = lanes_mut(&mut self.ph_r[j * MODES..(j + 1) * MODES]);
                        let phi = lanes_mut(&mut self.ph_i[j * MODES..(j + 1) * MODES]);
                        let (zr, zi) = (lanes_mut(&mut self.zr), lanes_mut(&mut self.zi));
                        for c in 0..MODES / LANES {
                            let v = d.mul(V::ld(&pat[c])).mul(V::ld(&sw[c]));
                            let (a, b) = (V::ld(&phr[c]), V::ld(&phi[c]));
                            let (hr, hi) = (V::ld(&hr[c]), V::ld(&hi[c]));
                            V::ld(&zr[c]).add(v.mul(a.mul(hr).sub(b.mul(hi)))).st(&mut zr[c]);
                            V::ld(&zi[c]).add(v.mul(a.mul(hi).add(b.mul(hr)))).st(&mut zi[c]);
                            let (sr, si) = (V::ld(&sr[c]), V::ld(&si[c]));
                            a.mul(sr).sub(b.mul(si)).st(&mut phr[c]);
                            a.mul(si).add(b.mul(sr)).st(&mut phi[c]);
                        }
                        self.awake = true;
                        if p.age + p.sub >= p.rise {
                            self.pending[j].amp = 0.0;
                        }
                    }
                    self.pending[j].age += 1;
                    continue;
                }
                let a = V::splat(p.amp * self.pulse[p.age as usize]);
                let exc = lanes_mut(&mut self.exc);
                if !struck {
                    exc.fill([0.0; LANES]);
                    struck = true;
                }
                for c in 0..MODES / LANES {
                    V::ld(&exc[c]).add(a.mul(V::ld(&pat[c])).mul(V::ld(&sw[c]))).st(&mut exc[c]);
                }
                self.pending[j].age += 1;
                if p.age as usize + 1 >= self.pulse.len() {
                    self.pending[j].amp = 0.0;
                }
            }
            let (zr, zi) = (lanes_mut(&mut self.zr), lanes_mut(&mut self.zi));
            if struck {
                let exc = lanes(&self.exc);
                for c in 0..MODES / LANES {
                    V::ld(&zr[c]).add(V::ld(&exc[c])).st(&mut zr[c]);
                }
                self.awake = true;
            }
            if !self.awake {
                continue;
            }
            let (pr, pi) = (lanes(&self.pr), lanes(&self.pi));
            let (wl, wr) = (lanes(&self.w[0]), lanes(&self.w[1]));
            let (mut acc_l, mut acc_r) = (V::zero(), V::zero());
            for c in 0..MODES / LANES {
                let (re, im) = (V::ld(&zr[c]), V::ld(&zi[c]));
                let (pr, pi) = (V::ld(&pr[c]), V::ld(&pi[c]));
                re.mul(pr).sub(im.mul(pi)).st(&mut zr[c]);
                re.mul(pi).add(im.mul(pr)).st(&mut zi[c]);
                acc_l = acc_l.add(re.mul(V::ld(&wl[c])));
                acc_r = acc_r.add(re.mul(V::ld(&wr[c])));
            }
            let (mut sl, mut sr) = ([0.0f32; LANES], [0.0f32; LANES]);
            acc_l.st(&mut sl);
            acc_r.st(&mut sr);
            l[n] += gain * sl.iter().sum::<f32>();
            r[n] += gain * sr.iter().sum::<f32>();
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

/// The bank's arrays by chunks of [`LANES`] resonances ([`MODES`] is a
/// multiple of it).
#[inline(always)]
fn lanes(a: &[f32]) -> &[[f32; LANES]] {
    a.as_chunks::<LANES>().0
}

#[inline(always)]
fn lanes_mut(a: &mut [f32]) -> &mut [[f32; LANES]] {
    a.as_chunks_mut::<LANES>().0
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
    fn a_full_queue_keeps_every_strike_whole() {
        // Four eight-note chords 40 ms apart fill the 32 slots while they
        // settle in over 0.3 s; one quiet note at 160 ms must free one. Once
        // every ramp is over, the halo is the sum of each strike's own.
        let sr = 48_000.0;
        let law = BodyLaw { t60: [1.0; 8], rise: 0.3, delay: 0.0, ..law() };
        let chords: Vec<(usize, u8, f64)> = (0..4).flat_map(|c| (0..8).map(move |k| (c * 1920, 36 + 3 * k as u8 + c as u8, 2.0))).collect();
        let quiet = (7680, 70, 0.3);
        let play = |strikes: &[(usize, u8, f64)]| {
            let mut b = BodyBank::new(sr, &law);
            let n = (sr * 0.6) as usize;
            let (mut l, mut r) = (vec![0.0f32; n], vec![0.0f32; n]);
            let mut at = 0;
            let mut times: Vec<usize> = strikes.iter().map(|s| s.0).collect();
            times.push(n);
            times.sort();
            times.dedup();
            for t in times {
                b.render([&mut l[at..t], &mut r[at..t]], 1.0);
                for &(_, note, speed) in strikes.iter().filter(|s| s.0 == t) {
                    b.strike(note, speed, 0.0);
                }
                at = t;
            }
            l
        };
        let mut all = chords.clone();
        all.push(quiet);
        let (together, apart, alone) = (play(&all), play(&chords), play(&[quiet]));
        let db = |x: &[f32]| 10.0 * (x.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / x.len() as f64).log10();
        let w = (0.5 * sr) as usize..(0.6 * sr) as usize;
        let err: Vec<f32> = w.clone().map(|i| together[i] - apart[i] - alone[i]).collect();
        let rel = db(&err) - db(&apart[w]);
        assert!(rel < -40.0, "{rel:.1} dB");
    }

    #[test]
    fn a_strike_that_needs_a_slot_changes_nothing_before_its_frame() {
        // All 32 slots settling in; one more note 48 frames into the next
        // render. Struck with that offset, or after rendering those 48
        // frames first: the same output, and before the note the same as
        // without it.
        let sr = 48_000.0;
        let law = BodyLaw { t60: [1.0; 8], rise: 0.3, delay: 0.0, ..law() };
        let full = || {
            let mut b = BodyBank::new(sr, &law);
            for k in 0..32u8 {
                b.strike(30 + 2 * k, 1.0 + 0.05 * k as f64, 0.0);
            }
            let (mut l, mut r) = (vec![0.0f32; 64], vec![0.0f32; 64]);
            b.render([&mut l, &mut r], 1.0);
            b
        };
        let n = 2400;
        let run = |split: Option<usize>| {
            let mut b = full();
            let (mut l, mut r) = (vec![0.0f32; n], vec![0.0f32; n]);
            match split {
                None => b.render([&mut l, &mut r], 1.0),
                Some(0) => {
                    b.strike(80, 2.0, 48.0 / sr);
                    b.render([&mut l, &mut r], 1.0);
                }
                Some(k) => {
                    b.render([&mut l[..k], &mut r[..k]], 1.0);
                    b.strike(80, 2.0, 0.0);
                    b.render([&mut l[k..], &mut r[k..]], 1.0);
                }
            }
            (l, r)
        };
        let (without, offset, split) = (run(None), run(Some(0)), run(Some(48)));
        let diff = |a: &[f32], b: &[f32]| a.iter().zip(b).map(|(x, y)| (x - y).abs()).fold(0.0f32, f32::max);
        assert_eq!(diff(&offset.0[..48], &without.0[..48]), 0.0);
        assert_eq!(diff(&offset.1[..48], &without.1[..48]), 0.0);
        assert_eq!(diff(&offset.0, &split.0), 0.0);
        assert_eq!(diff(&offset.1, &split.1), 0.0);
        // The note does sound from its frame on.
        assert!(diff(&offset.0[48..], &without.0[48..]) > 1e-3);
    }

    #[test]
    fn the_slot_is_chosen_when_the_note_takes_effect() {
        // The same notes rendered in one call, or cut into blocks at their
        // frames and struck at offset zero: the same halo, bit for bit.
        let sr = 48_000.0;
        let settling = BodyLaw { t60: [1.0; 8], rise: 0.3, delay: 0.0, ..law() };
        let n = 4800;
        let play = |setup: &dyn Fn(&mut BodyBank), notes: &[(usize, u8)], cut: bool| {
            let mut b = BodyBank::new(sr, &settling);
            setup(&mut b);
            let (mut l, mut r) = (vec![0.0f32; n], vec![0.0f32; n]);
            if cut {
                let mut at = 0;
                for &(t, note) in notes {
                    b.render([&mut l[at..t], &mut r[at..t]], 1.0);
                    b.strike(note, 1.5, 0.0);
                    at = t;
                }
                b.render([&mut l[at..], &mut r[at..]], 1.0);
            } else {
                for &(t, note) in notes {
                    b.strike(note, 1.5, t as f64 / sr);
                }
                b.render([&mut l, &mut r], 1.0);
            }
            (l, r)
        };
        let same = |a: &(Vec<f32>, Vec<f32>), b: &(Vec<f32>, Vec<f32>)| a.0 == b.0 && a.1 == b.1;
        // A slot frees during the block: 31 settling strikes and one 12-frame
        // pulse struck at its start, a new note at frame 48 (the pulse's
        // slot is free by then).
        let freeing = |b: &mut BodyBank| {
            for k in 0..31u8 {
                b.strike(30 + 2 * k, 1.0 + 0.05 * k as f64, 0.0);
            }
            b.set_law(&BodyLaw { rise: 0.0, ..settling });
            b.strike(95, 2.0, 0.0);
            b.set_law(&settling);
        };
        assert!(same(&play(&freeing, &[(48, 80)], false), &play(&freeing, &[(48, 80)], true)));
        // A full queue and two arrivals, at frames 8 and 48: the first one is
        // the least advanced when the second comes.
        let full = |b: &mut BodyBank| {
            for k in 0..32u8 {
                b.strike(30 + 2 * k, 1.0 + 0.05 * k as f64, 0.0);
            }
            let (mut l, mut r) = (vec![0.0f32; 64], vec![0.0f32; 64]);
            b.render([&mut l, &mut r], 1.0);
        };
        let two = [(8, 80), (48, 84)];
        assert!(same(&play(&full, &two, false), &play(&full, &two, true)));
    }

    /// The render as it was written before the vector kernels, resonance by
    /// resonance: the reference the kernels must match bit for bit.
    fn render_reference(bank: &mut BodyBank, out: [&mut [f32]; 2], gain: f32) {
        if !bank.awake && bank.arrived == 0 && bank.pending.iter().all(|p| p.amp == 0.0) {
            return;
        }
        let [l, r] = out;
        for n in 0..l.len() {
            // Strikes reaching their frame take their slots, in the order
            // they came; the others come one sample nearer.
            if bank.arrived > 0 {
                bank.arrive_due();
                for a in &mut bank.arrivals[..bank.arrived] {
                    a.at -= 1;
                }
            }
            // Strikes under way this sample.
            let mut struck = false;
            for j in 0..PENDING {
                let p = bank.pending[j];
                if p.amp == 0.0 {
                    continue;
                }
                if p.wait > 0 {
                    bank.pending[j].wait -= 1;
                    continue;
                }
                let pat = &bank.pattern[p.key as usize * MODES..(p.key as usize + 1) * MODES];
                let sw = &bank.speed_w[j * MODES..(j + 1) * MODES];
                if p.rise > 0 {
                    // Settling in: every `sub` samples the next step of the
                    // ramp, turned as the resonance has since the strike.
                    if p.age.is_multiple_of(p.sub) {
                        let d = (p.amp as f64 * (ramp(p.age + p.sub, p.rise) - ramp(p.age, p.rise))) as f32;
                        let t = p.sub.trailing_zeros() as usize;
                        let (sr, si) = (&bank.step_r[t], &bank.step_i[t]);
                        let phr = &mut bank.ph_r[j * MODES..(j + 1) * MODES];
                        let phi = &mut bank.ph_i[j * MODES..(j + 1) * MODES];
                        for i in 0..MODES {
                            let v = d * pat[i] * sw[i];
                            let (a, b) = (phr[i], phi[i]);
                            let (hr, hi) = (bank.pulse_hr[i], bank.pulse_hi[i]);
                            bank.zr[i] += v * (a * hr - b * hi);
                            bank.zi[i] += v * (a * hi + b * hr);
                            phr[i] = a * sr[i] - b * si[i];
                            phi[i] = a * si[i] + b * sr[i];
                        }
                        bank.awake = true;
                        if p.age + p.sub >= p.rise {
                            bank.pending[j].amp = 0.0;
                        }
                    }
                    bank.pending[j].age += 1;
                    continue;
                }
                let a = p.amp * bank.pulse[p.age as usize];
                if !struck {
                    bank.exc.iter_mut().for_each(|e| *e = 0.0);
                    struck = true;
                }
                for ((e, &q), &w) in bank.exc.iter_mut().zip(pat).zip(sw) {
                    *e += a * q * w;
                }
                bank.pending[j].age += 1;
                if p.age as usize + 1 >= bank.pulse.len() {
                    bank.pending[j].amp = 0.0;
                }
            }
            if struck {
                for (z, &e) in bank.zr.iter_mut().zip(&bank.exc) {
                    *z += e;
                }
                bank.awake = true;
            }
            if !bank.awake {
                continue;
            }
            let mut acc_l = [0.0f32; LANES];
            let mut acc_r = [0.0f32; LANES];
            for c in 0..MODES / LANES {
                let s = c * LANES;
                let zr: &mut [f32; LANES] = (&mut bank.zr[s..s + LANES]).try_into().unwrap();
                let zi: &mut [f32; LANES] = (&mut bank.zi[s..s + LANES]).try_into().unwrap();
                let pr: &[f32; LANES] = bank.pr[s..s + LANES].try_into().unwrap();
                let pi: &[f32; LANES] = bank.pi[s..s + LANES].try_into().unwrap();
                let wl: &[f32; LANES] = bank.w[0][s..s + LANES].try_into().unwrap();
                let wr: &[f32; LANES] = bank.w[1][s..s + LANES].try_into().unwrap();
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
        if bank.awake && bank.pending.iter().all(|p| p.amp == 0.0) {
            let e: f32 = bank.zr.iter().zip(&bank.zi).map(|(a, b)| a * a + b * b).sum();
            if e < 1e-16 {
                bank.zr.iter_mut().for_each(|z| *z = 0.0);
                bank.zi.iter_mut().for_each(|z| *z = 0.0);
                bank.awake = false;
            }
        }
    }

    /// Strikes of every kind (pulses, rises of every step length, a full
    /// queue, strikes due later, live law changes) rendered in blocks of
    /// every length: each ISA's kernels give the reference's bits.
    #[test]
    fn every_isa_matches_the_scalar_reference() {
        let sr = 48_000.0;
        for rise in [0.0, 0.004, 0.012, 0.03, 0.06, 0.1] {
            let law = BodyLaw { rise, speed_ref: 2.76, ..law() };
            for isa in Isa::available() {
                let mut a = BodyBank::new(sr, &law);
                let mut b = BodyBank::new(sr, &law);
                a.set_isa(isa);
                let mut seed = 0x2545_f491u32;
                let mut rnd = |n: u32| {
                    seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                    (seed >> 8) % n
                };
                for block in 0..400 {
                    let len = [1usize, 7, 16, 63, 64][block % 5];
                    // Bursts of strikes (more than the slots hold), then rests
                    // long enough for the bank to fall asleep.
                    let strikes = if block % 97 < 60 { rnd(4) } else { 0 } + if block % 131 == 5 { 40 } else { 0 };
                    for _ in 0..strikes {
                        let (note, speed) = (21 + rnd(88) as u8, 0.3 + 0.05 * rnd(100) as f64);
                        let offset = if rnd(3) == 0 { rnd(len as u32) as f64 / sr } else { 0.0 };
                        a.strike(note, speed, offset);
                        b.strike(note, speed, offset);
                    }
                    if block == 200 {
                        let l2 = BodyLaw { rise: 0.5 * rise, gain: 0.7 * law.gain, ..law };
                        a.set_law(&l2);
                        b.set_law(&l2);
                    }
                    let (mut al, mut ar) = (vec![0.25f32; len], vec![-0.5f32; len]);
                    let (mut bl, mut br) = (al.clone(), ar.clone());
                    a.render([&mut al, &mut ar], 0.8);
                    render_reference(&mut b, [&mut bl, &mut br], 0.8);
                    let same = |x: &[f32], y: &[f32]| x.iter().zip(y).all(|(p, q)| p.to_bits() == q.to_bits());
                    assert!(same(&al, &bl) && same(&ar, &br), "{isa:?}, rise {rise}, block {block}");
                    assert!(same(&a.zr, &b.zr) && same(&a.zi, &b.zi), "{isa:?}, rise {rise}, block {block}: states");
                }
                assert!(b.zr.iter().any(|&z| z != 0.0) || !b.awake);
            }
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
