//! Polyphonic engine: voice management, events and block rendering.
//!
//! Each key is a [`Voice`]: 1–3 stiff strings coupled through the bridge
//! (double decay, beating), struck by a felt hammer through the passive
//! contact solver. The soundboard and radiation come in Phase 4.

use crate::denormal::DenormalGuard;
use crate::note_model::PianoTable;
use crate::params::PianoModel;
use crate::voice::Voice;

pub use crate::voice::MAX_MODES;
/// Sustain pedal threshold on CC64.
const SUSTAIN_ON: u8 = 64;
/// Highest note that has a damper (dampers stop around E6/F6 on grands).
const HIGHEST_DAMPED_NOTE: u8 = 88;
/// Placeholder damper: extra decay rate (T60 ≈ 0.3 s). Phase 3 replaces it.
const DAMPER_SIGMA: f64 = 23.0;
/// Placeholder output scaling from bridge force (N) to full scale, until
/// the soundboard and radiation exist.
const OUTPUT_GAIN: f32 = 1.0 / 240.0;
/// Voices below −100 dBFS (in bridge force², before OUTPUT_GAIN) are freed.
const SILENCE_ENERGY: f32 = (1e-5 / OUTPUT_GAIN) * (1e-5 / OUTPUT_GAIN);

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

pub struct Engine {
    cfg: EngineConfig,
    table: PianoTable,
    voices: Vec<Voice>,
    sustain: bool,
}

impl Engine {
    pub fn new(cfg: EngineConfig) -> Self {
        let voices = (0..cfg.polyphony).map(|_| Voice::new(cfg.sample_rate, cfg.max_block)).collect();
        let table = PianoTable::new(&cfg.model, cfg.sample_rate, MAX_MODES);
        Self { cfg, table, voices, sustain: false }
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
            if v.active && v.energy() < SILENCE_ENERGY {
                v.active = false;
            }
        }
    }

    fn render(&mut self, out: &mut [f32]) {
        if out.is_empty() {
            return;
        }
        for v in self.voices.iter_mut().filter(|v| v.active) {
            v.render(self.table.get(v.note), out);
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
                    let ve = v.energy();
                    if ve < e {
                        e = ve;
                        best = i;
                    }
                }
                best
            });
        let nm = self.table.get(note);
        let v = &mut self.voices[idx];
        if !(v.active && v.note == note) {
            v.setup(nm);
        } else if v.damped {
            v.set_damping(0.0);
        }
        v.active = true;
        v.key_down = true;
        v.strike(nm, hammer_velocity(&cfg, velocity), contact_oversampling(&cfg, note));
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
        v.set_damping(DAMPER_SIGMA);
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

/// Contact duration (s) and peak force (N) of one strike on a string at
/// rest. Allocates: analysis use only.
pub fn strike_stats(cfg: &EngineConfig, note: u8, velocity: u8, oversample: Option<usize>) -> (f64, f64) {
    let nm = crate::note_model::NoteModel::build(&cfg.model, note, cfg.sample_rate, MAX_MODES);
    let mut v = Voice::new(cfg.sample_rate, cfg.max_block);
    v.setup(&nm);
    let os = oversample.unwrap_or_else(|| contact_oversampling(cfg, note));
    v.strike(&nm, hammer_velocity(cfg, velocity), os);
    let mut out = vec![0.0f32; 256];
    while v.contact.is_active() {
        v.contact.run(&mut out);
    }
    (v.contact.contact_time(), v.contact.max_force())
}
