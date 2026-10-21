//! Polyphonic engine: voice management, events and block rendering.
//!
//! Each key is a [`Voice`]: 1–3 stiff strings coupled through the bridge
//! (double decay, beating), struck by a felt hammer through the passive
//! contact solver. The bridge forces of the keys, grouped by bridge region,
//! drive the modal soundboard, whose modes radiate to two microphones.

use std::sync::Arc;

use crate::denormal::DenormalGuard;
use crate::instrument::{region_of, Instrument, MICS, REGIONS};
use crate::modal::ModalBank;
use crate::params::PianoModel;
use crate::voice::Voice;

pub use crate::voice::MAX_MODES;
/// Sustain pedal threshold on CC64.
const SUSTAIN_ON: u8 = 64;
/// Highest note that has a damper (dampers stop around E6/F6 on grands).
const HIGHEST_DAMPED_NOTE: u8 = 88;
/// Placeholder damper: extra decay rate (T60 ≈ 0.3 s). Phase 3 replaces it.
const DAMPER_SIGMA: f64 = 23.0;
/// Output scaling without soundboard: bridge force (N) to full scale.
const FORCE_GAIN: f32 = 1.0 / 240.0;
/// Output scaling with soundboard: pressure at the mics (Pa) to full scale.
const PRESSURE_GAIN: f32 = 1.0 / 24.0;
/// Voices whose bridge force falls below ≈ 1e-5·240 N (−100 dB of a loud
/// note) are freed.
const SILENCE_ENERGY: f32 = (1e-5 / FORCE_GAIN) * (1e-5 / FORCE_GAIN);
/// Soundboard regions below this output energy (Pa²) with no input sleep.
const BOARD_SILENCE: f32 = (1e-6 / PRESSURE_GAIN) * (1e-6 / PRESSURE_GAIN);

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
    inst: Arc<Instrument>,
    voices: Vec<Voice>,
    sustain: bool,
    /// Bridge force per region for the current block.
    region: Vec<Vec<f32>>,
    /// Soundboard banks, one per region, outputs = the two mics (empty
    /// without board).
    board: Vec<ModalBank>,
    board_awake: [bool; REGIONS],
    /// Bridge-mass low-pass state per region.
    lp_state: [f32; REGIONS],
    stereo: [Vec<f32>; MICS],
}

impl Engine {
    /// Builds the instrument (≈ 0.1–1 s): not for the audio thread.
    pub fn new(cfg: EngineConfig) -> Self {
        Self::with_instrument(cfg, Arc::new(Instrument::new(cfg.model, cfg.sample_rate)))
    }

    /// Engine on a prebuilt (shared) instrument; `cfg.model` is ignored.
    pub fn with_instrument(cfg: EngineConfig, inst: Arc<Instrument>) -> Self {
        assert_eq!(inst.sample_rate, cfg.sample_rate);
        let voices = (0..cfg.polyphony).map(|_| Voice::new(cfg.sample_rate, cfg.max_block)).collect();
        let board = inst
            .board
            .as_ref()
            .map(|b| {
                b.banks
                    .iter()
                    .map(|specs| {
                        let mut bank = ModalBank::new(cfg.sample_rate, specs.len(), cfg.max_block);
                        bank.set_modes(specs.iter().copied());
                        bank
                    })
                    .collect()
            })
            .unwrap_or_default();
        Self {
            cfg,
            inst,
            voices,
            sustain: false,
            region: vec![vec![0.0; cfg.max_block]; REGIONS],
            board,
            board_awake: [false; REGIONS],
            lp_state: [0.0; REGIONS],
            stereo: std::array::from_fn(|_| vec![0.0; cfg.max_block]),
        }
    }

    pub fn instrument(&self) -> &Arc<Instrument> {
        &self.inst
    }

    pub fn config(&self) -> &EngineConfig {
        &self.cfg
    }

    pub fn active_voices(&self) -> usize {
        self.voices.iter().filter(|v| v.active).count()
    }

    pub fn active_modes(&self) -> usize {
        let board: usize =
            self.board.iter().zip(&self.board_awake).filter(|(_, a)| **a).map(|(b, _)| b.len()).sum();
        self.voices.iter().filter(|v| v.active).map(|v| v.bank.len()).sum::<usize>() + board
    }

    /// Mono render: mean of the microphones (overwrites `out`).
    pub fn process(&mut self, events: &[TimedEvent], out: &mut [f32]) {
        let mut st = std::mem::take(&mut self.stereo);
        {
            let [l, r] = &mut st;
            self.process_stereo(events, &mut l[..out.len()], &mut r[..out.len()]);
            for ((o, a), b) in out.iter_mut().zip(l.iter()).zip(r.iter()) {
                *o = 0.5 * (a + b);
            }
        }
        self.stereo = st;
    }

    /// Renders one block into `left`/`right` (overwritten). `events` must be
    /// sorted by frame and lie inside the block. Real-time safe: no
    /// allocation, no lock.
    pub fn process_stereo(&mut self, events: &[TimedEvent], left: &mut [f32], right: &mut [f32]) {
        let len = left.len();
        assert!(len <= self.cfg.max_block && right.len() == len);
        let _ftz = DenormalGuard::new();
        for r in &mut self.region {
            r[..len].fill(0.0);
        }
        let mut pos = 0;
        for ev in events {
            let f = (ev.frame as usize).min(len);
            self.render(pos, f);
            pos = f;
            self.handle(ev.event);
        }
        self.render(pos, len);

        left.fill(0.0);
        right.fill(0.0);
        if self.board.is_empty() {
            for r in &self.region {
                for ((l, rr), x) in left.iter_mut().zip(right.iter_mut()).zip(&r[..len]) {
                    *l += FORCE_GAIN * x;
                    *rr += FORCE_GAIN * x;
                }
            }
        } else {
            let lp = self.inst.board.as_ref().and_then(|b| b.input_lowpass);
            for (ri, input) in self.region.iter_mut().enumerate() {
                let input = &mut input[..len];
                if let Some(a) = lp {
                    let mut y = self.lp_state[ri];
                    for x in input.iter_mut() {
                        y = a * y + (1.0 - a) * *x;
                        *x = y;
                    }
                    self.lp_state[ri] = y;
                }
                let input = &*input;
                let has_input = input.iter().any(|&x| x != 0.0);
                if !has_input && !self.board_awake[ri] {
                    continue;
                }
                let bank = &mut self.board[ri];
                bank.process2(Some(input), left, right);
                self.board_awake[ri] = has_input || bank.energy() > BOARD_SILENCE;
            }
            for x in left.iter_mut().chain(right.iter_mut()) {
                *x *= PRESSURE_GAIN;
            }
        }
        for v in &mut self.voices {
            if v.active && v.energy() < SILENCE_ENERGY {
                v.active = false;
            }
        }
    }

    fn render(&mut self, from: usize, to: usize) {
        if from >= to {
            return;
        }
        for v in self.voices.iter_mut().filter(|v| v.active) {
            v.render(self.inst.table.get(v.note), &mut self.region[region_of(v.note)][from..to]);
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
        let nm = self.inst.table.get(note);
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
    let board = cfg.model.soundboard.map(crate::soundboard::Soundboard::new);
    let nm = crate::note_model::NoteModel::build(&cfg.model, board.as_ref(), note, cfg.sample_rate, MAX_MODES);
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
