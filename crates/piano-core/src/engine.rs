//! Polyphonic engine: voice management, events and block rendering.
//!
//! Phase 0 voice: one modal string per note excited by a *prescribed*
//! half-sine force whose duration comes from Stulov's rigid-surface contact
//! law. This is a placeholder for the coupled hammer–string contact of
//! Phase 1; the string side (modal projection, bridge force) is already the
//! physical formulation.

use crate::denormal::DenormalGuard;
use crate::modal::{ModalBank, ModeSpec};
use crate::params;

pub const MAX_MODES: usize = 512;
/// Sustain pedal threshold on CC64.
const SUSTAIN_ON: u8 = 64;
/// Highest note that has a damper (dampers stop around E6/F6 on grands).
const HIGHEST_DAMPED_NOTE: u8 = 88;
/// Placeholder damper: extra decay rate (T60 ≈ 0.3 s). Phase 3 replaces it.
const DAMPER_SIGMA: f64 = 23.0;
/// Placeholder output scaling from bridge force (N) to full scale, until
/// the soundboard and radiation exist.
const OUTPUT_GAIN: f32 = 1.0 / 400.0;
const SILENCE_ENERGY: f32 = 1e-14;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    NoteOn { note: u8, velocity: u8 },
    NoteOff { note: u8, velocity: u8 },
    Control { cc: u8, value: u8 },
    AllNotesOff,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimedEvent {
    /// Offset in samples from the start of the block.
    pub frame: u32,
    pub event: Event,
}

#[derive(Clone, Copy, Debug)]
pub struct EngineConfig {
    pub sample_rate: f64,
    pub max_block: usize,
    pub polyphony: usize,
    pub a4: f64,
    /// Relative striking position along the string.
    pub strike_pos: f64,
    pub hammer_vmin: f64,
    pub hammer_vmax: f64,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            sample_rate: 48_000.0,
            max_block: 128,
            polyphony: 64,
            a4: 440.0,
            strike_pos: 1.0 / 8.0,
            hammer_vmin: 0.4,
            hammer_vmax: 6.0,
        }
    }
}

struct Voice {
    note: u8,
    active: bool,
    key_down: bool,
    damped: bool,
    started_at: u64,
    bank: ModalBank,
    force: Vec<f32>,
    pulse_pos: usize,
    pulse_len: usize,
    pulse_peak: f32,
}

pub struct Engine {
    cfg: EngineConfig,
    voices: Vec<Voice>,
    sustain: bool,
    clock: u64,
}

impl Engine {
    pub fn new(cfg: EngineConfig) -> Self {
        let voices = (0..cfg.polyphony)
            .map(|_| Voice {
                note: 0,
                active: false,
                key_down: false,
                damped: false,
                started_at: 0,
                bank: ModalBank::new(cfg.sample_rate, MAX_MODES, cfg.max_block),
                force: vec![0.0; cfg.max_block],
                pulse_pos: 0,
                pulse_len: 0,
                pulse_peak: 0.0,
            })
            .collect();
        Self { cfg, voices, sustain: false, clock: 0 }
    }

    pub fn config(&self) -> &EngineConfig {
        &self.cfg
    }

    pub fn active_voices(&self) -> usize {
        self.voices.iter().filter(|v| v.active).count()
    }

    pub fn active_modes(&self) -> usize {
        self.voices.iter().filter(|v| v.active).map(|v| v.bank.len()).sum()
    }

    /// Renders one block into `out` (overwritten). `events` must be sorted by
    /// frame and lie inside the block. Real-time safe: no allocation, no lock.
    pub fn process(&mut self, events: &[TimedEvent], out: &mut [f32]) {
        assert!(out.len() <= self.cfg.max_block);
        let _ftz = DenormalGuard::new();
        out.fill(0.0);
        let mut pos = 0;
        for ev in events {
            let f = (ev.frame as usize).min(out.len());
            self.render(&mut out[pos..f]);
            pos = f;
            self.handle(ev.event);
        }
        self.render(&mut out[pos..]);
        for o in out.iter_mut() {
            *o *= OUTPUT_GAIN;
        }
        for v in &mut self.voices {
            if v.active && v.pulse_pos >= v.pulse_len && v.bank.energy() < SILENCE_ENERGY {
                v.active = false;
            }
        }
    }

    fn render(&mut self, out: &mut [f32]) {
        if out.is_empty() {
            return;
        }
        self.clock += out.len() as u64;
        for v in self.voices.iter_mut().filter(|v| v.active) {
            if v.pulse_pos < v.pulse_len {
                let force = &mut v.force[..out.len()];
                for f in force.iter_mut() {
                    *f = if v.pulse_pos < v.pulse_len {
                        let ph = std::f32::consts::PI * (v.pulse_pos as f32 + 0.5) / v.pulse_len as f32;
                        v.pulse_pos += 1;
                        v.pulse_peak * ph.sin()
                    } else {
                        0.0
                    };
                }
                v.bank.process(Some(force), out);
            } else {
                v.bank.process(None, out);
            }
        }
    }

    fn handle(&mut self, ev: Event) {
        match ev {
            Event::NoteOn { velocity: 0, note } => self.note_off(note),
            Event::NoteOn { note, velocity } => self.note_on(note, velocity),
            Event::NoteOff { note, .. } => self.note_off(note),
            Event::Control { cc: 64, value } => {
                self.sustain = value >= SUSTAIN_ON;
                if !self.sustain {
                    for v in self.voices.iter_mut().filter(|v| v.active && !v.key_down) {
                        damp(v);
                    }
                }
            }
            Event::Control { cc: 120 | 123, .. } | Event::AllNotesOff => {
                for v in self.voices.iter_mut().filter(|v| v.active) {
                    v.key_down = false;
                    damp(v);
                }
            }
            Event::Control { .. } => {}
        }
    }

    fn note_on(&mut self, note: u8, velocity: u8) {
        let clock = self.clock;
        let cfg = self.cfg;
        // A key re-struck while its string still rings hits the moving string.
        let idx = self
            .voices
            .iter()
            .position(|v| v.active && v.note == note)
            .or_else(|| self.voices.iter().position(|v| !v.active))
            .unwrap_or_else(|| {
                // Steal the quietest voice.
                let mut best = 0;
                let mut e = f32::INFINITY;
                for (i, v) in self.voices.iter().enumerate() {
                    let ve = v.bank.energy();
                    if ve < e {
                        e = ve;
                        best = i;
                    }
                }
                best
            });
        let v = &mut self.voices[idx];
        if !(v.active && v.note == note) {
            v.bank.reset();
            v.note = note;
            v.bank.set_modes(string_modes(&cfg, note));
        } else if v.damped {
            v.bank.update_poles(0.0);
        }
        v.active = true;
        v.key_down = true;
        v.damped = false;
        v.started_at = clock;

        let vh = hammer_velocity(&cfg, velocity);
        let h = params::hammer_stulov(note);
        let tc = h.contact_time_rigid(vh);
        v.pulse_len = ((tc * cfg.sample_rate).round() as usize).max(2);
        v.pulse_pos = 0;
        // Half-sine whose impulse equals the rebound momentum 2·m·V.
        let impulse = 2.0 * h.mass_kg * vh;
        v.pulse_peak = (std::f64::consts::PI * impulse / (2.0 * tc)) as f32;
    }

    fn note_off(&mut self, note: u8) {
        let sustain = self.sustain;
        for v in self.voices.iter_mut().filter(|v| v.active && v.note == note && v.key_down) {
            v.key_down = false;
            if !sustain {
                damp(v);
            }
        }
    }
}

fn damp(v: &mut Voice) {
    if !v.damped && v.note <= HIGHEST_DAMPED_NOTE {
        v.damped = true;
        v.bank.update_poles(DAMPER_SIGMA);
    }
}

/// MIDI velocity → hammer velocity, exponential between vmin and vmax.
pub fn hammer_velocity(cfg: &EngineConfig, velocity: u8) -> f64 {
    let t = (velocity.clamp(1, 127) as f64 - 1.0) / 126.0;
    cfg.hammer_vmin * (cfg.hammer_vmax / cfg.hammer_vmin).powf(t)
}

/// Transverse modes of one string seen as force at the bridge.
///
/// With modal mass μL/2 and μ = T/(2L·f0)², the bridge force T·∂y/∂x(0) for
/// a unit force impulse at x0 is `4·f0·Σ sin(kπx0)/√(1+Bk²)·e^{-σt}·sin(ω_k t)`,
/// independent of T and L.
pub fn string_modes(cfg: &EngineConfig, note: u8) -> impl Iterator<Item = ModeSpec> + use<> {
    let f0 = params::note_freq(note, cfg.a4);
    let b = params::inharmonicity_rigaud(note);
    let nyq = 0.45 * cfg.sample_rate;
    let t = 1.0 / cfg.sample_rate;
    let x0 = cfg.strike_pos;
    (1..=MAX_MODES as u32)
        .map(move |k| (k, params::partial_freq(f0, b, k)))
        .take_while(move |&(_, f)| f < nyq)
        .map(move |(k, f)| {
            let kf = k as f64;
            let amp = 4.0 * f0 * (kf * std::f64::consts::PI * x0).sin() / (1.0 + b * kf * kf).sqrt() * t;
            ModeSpec::driven(f, params::decay_rate_placeholder(note, f), amp)
        })
}
