//! Polyphonic engine: voice management, events and block rendering.
//!
//! Phase 1 voice: one stiff string per key (modal, parameters from
//! [`PianoModel`]) struck by a felt hammer through the passive contact
//! solver of [`crate::hammer`]. During contact the solver owns the string;
//! afterwards the modes run freely in the SIMD [`ModalBank`]. Unisons,
//! bridge coupling and the soundboard come in Phase 2.

use crate::denormal::DenormalGuard;
use crate::hammer::ContactSolver;
use crate::modal::{ModalBank, ModeSpec};
use crate::params::{self, NoteParams, PianoModel};

pub const MAX_MODES: usize = 512;
/// Sustain pedal threshold on CC64.
const SUSTAIN_ON: u8 = 64;
/// Highest note that has a damper (dampers stop around E6/F6 on grands).
const HIGHEST_DAMPED_NOTE: u8 = 88;
/// Placeholder damper: extra decay rate (T60 ≈ 0.3 s). Phase 3 replaces it.
const DAMPER_SIGMA: f64 = 23.0;
/// Placeholder output scaling from bridge force (N) to full scale, until
/// the soundboard and radiation exist.
const OUTPUT_GAIN: f32 = 1.0 / 120.0;
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
    pub model: PianoModel,
    /// Hammer speed (m/s) at MIDI velocity 1 and 127.
    pub hammer_vmin: f64,
    pub hammer_vmax: f64,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            sample_rate: 48_000.0,
            max_block: 128,
            polyphony: 128,
            model: PianoModel::default(),
            hammer_vmin: 0.4,
            // Fits the Salamander dynamics curve (1.4 dB rms over 16 layers).
            hammer_vmax: 4.0,
        }
    }
}

struct Voice {
    note: u8,
    active: bool,
    key_down: bool,
    damped: bool,
    params: Option<NoteParams>,
    bank: ModalBank,
    contact: ContactSolver,
}

pub struct Engine {
    cfg: EngineConfig,
    voices: Vec<Voice>,
    sustain: bool,
}

impl Engine {
    pub fn new(cfg: EngineConfig) -> Self {
        let voices = (0..cfg.polyphony)
            .map(|_| Voice {
                note: 0,
                active: false,
                key_down: false,
                damped: false,
                params: None,
                bank: ModalBank::new(cfg.sample_rate, MAX_MODES, cfg.max_block),
                contact: ContactSolver::new(MAX_MODES, cfg.sample_rate),
            })
            .collect();
        Self { cfg, voices, sustain: false }
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
            if v.active && !v.contact.is_active() && v.bank.energy() < SILENCE_ENERGY {
                v.active = false;
            }
        }
    }

    fn render(&mut self, out: &mut [f32]) {
        if out.is_empty() {
            return;
        }
        for v in self.voices.iter_mut().filter(|v| v.active) {
            let mut start = 0;
            if v.contact.is_active() {
                start = v.contact.run(out);
                if !v.contact.is_active() {
                    v.contact.hand_over(&mut v.bank);
                }
            }
            if start < out.len() {
                v.bank.process(None, &mut out[start..]);
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
            let np = cfg.model.note(note);
            v.bank.reset();
            v.bank.set_modes(string_modes(&cfg, &np, note));
            v.note = note;
            v.params = Some(np);
        } else {
            if v.contact.is_active() {
                v.contact.hand_over(&mut v.bank);
            }
            if v.damped {
                v.bank.update_poles(0.0);
            }
        }
        v.active = true;
        v.key_down = true;
        v.damped = false;
        let np = v.params.expect("set above");
        v.contact.strike(&v.bank, &np, hammer_velocity(&cfg, velocity), contact_oversampling(&cfg, note));
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

/// Sub-steps per sample during contact so that the shortest treble contacts
/// (≈ 0.4 ms) still span ~80 steps: 1 below C6 at 48 kHz, up to 4 at C8.
pub fn contact_oversampling(cfg: &EngineConfig, note: u8) -> usize {
    let per_ms = cfg.sample_rate / 1000.0;
    let need = match note {
        0..=83 => 40.0,
        84..=95 => 80.0,
        _ => 200.0,
    };
    ((need / per_ms).ceil() as usize).clamp(1, 8)
}

/// MIDI velocity → hammer velocity, exponential between vmin and vmax.
pub fn hammer_velocity(cfg: &EngineConfig, velocity: u8) -> f64 {
    let t = (velocity.clamp(1, 127) as f64 - 1.0) / 126.0;
    cfg.hammer_vmin * (cfg.hammer_vmax / cfg.hammer_vmin).powf(t)
}

/// Transverse modes of one string, up to 0.45·fs.
pub fn string_modes(cfg: &EngineConfig, np: &NoteParams, note: u8) -> impl Iterator<Item = ModeSpec> + use<> {
    let nyq = 0.45 * cfg.sample_rate;
    let (f0, b, losses) = (np.f0, np.b, cfg.model.losses);
    (1..=MAX_MODES as u32).map_while(move |k| {
        let f = params::partial_freq(f0, b, k);
        (f < nyq).then(|| ModeSpec { freq_hz: f, sigma: losses.sigma(note, f), gain_re: 0.0, gain_im: 0.0 })
    })
}

/// Contact duration (s) and peak force (N) of one strike on a string at
/// rest. Allocates: analysis use only.
pub fn strike_stats(cfg: &EngineConfig, note: u8, velocity: u8, oversample: Option<usize>) -> (f64, f64) {
    let np = cfg.model.note(note);
    let mut bank = ModalBank::new(cfg.sample_rate, MAX_MODES, cfg.max_block);
    bank.set_modes(string_modes(cfg, &np, note));
    let mut s = ContactSolver::new(MAX_MODES, cfg.sample_rate);
    let os = oversample.unwrap_or_else(|| contact_oversampling(cfg, note));
    s.strike(&bank, &np, hammer_velocity(cfg, velocity), os);
    let mut out = vec![0.0f32; 256];
    while s.is_active() {
        s.run(&mut out);
    }
    (s.contact_time(), s.max_force())
}
