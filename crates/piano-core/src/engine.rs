//! Polyphonic engine: voice management, events, pedals and block rendering.
//!
//! Each key is a [`Voice`]: 1–3 stiff strings coupled through the bridge
//! (double decay, beating), struck by a felt hammer through the passive
//! contact solver, damped by a felt damper driven by the key and the pedals.
//! The bridge forces, grouped by bridge region, drive the modal soundboard,
//! which radiates to two microphones and moves the bridge under every other
//! string (sympathetic resonance).

use std::sync::Arc;

use crate::board::BoardBank;
use crate::denormal::DenormalGuard;
use crate::instrument::{region_of, Instrument, MICS, REGIONS};
use crate::params::PianoModel;
use crate::voice::{Voice, RING};

pub use crate::voice::MAX_MODES;
/// Internal processing granularity; also the fixed delay (samples) of the
/// bridge displacement fed back to the strings.
pub const SUB_BLOCK: usize = 64;
/// Delay (samples) of the bridge acceleration seen by the strings: the ring
/// delay plus the half sample of the backward difference.
const INPUT_DELAY: f64 = SUB_BLOCK as f64 + 0.5;
/// Output scaling without soundboard: bridge force (N) to full scale.
const FORCE_GAIN: f32 = 1.0 / 440.0;
/// Output scaling with soundboard: pressure at the mics (Pa) to full scale.
const PRESSURE_GAIN: f32 = 1.0 / 22.0;
/// Voices whose bridge force falls below ≈ 1e-5·240 N (−100 dB of a loud
/// note) are freed, unless their strings are free to resonate.
const SILENCE_ENERGY: f32 = (1e-5 / FORCE_GAIN) * (1e-5 / FORCE_GAIN);
/// Below this mic energy and without input, the soundboard sleeps.
const BOARD_SILENCE: f32 = (1e-6 / PRESSURE_GAIN) * (1e-6 / PRESSURE_GAIN);
/// Listening strings set up per block when the dampers lift (spreads cost).
const LISTENERS_PER_BLOCK: usize = 8;
/// Pending frame knocks (keybed, pedal) at a time.
const MAX_KNOCKS: usize = 32;

/// A half-sine force pulse into the board at one region.
#[derive(Clone, Copy, Default)]
struct Knock {
    region: usize,
    pos: usize,
    len: usize,
    peak: f32,
}

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
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            sample_rate: 48_000.0,
            max_block: 128,
            polyphony: 128,
            model: PianoModel::default(),
        }
    }
}

pub struct Engine {
    cfg: EngineConfig,
    inst: Arc<Instrument>,
    voices: Vec<Voice>,
    /// Pedal positions, 0–1.
    sustain: f64,
    soft: f64,
    sostenuto_down: bool,
    /// Bridge force per region for the current block: primary (hammer
    /// response) and sympathetic.
    region: Vec<Vec<f32>>,
    region_sym: Vec<Vec<f32>>,
    /// Force on the agraffes per region (primary strings), for the frame.
    region_agraffe: Vec<Vec<f32>>,
    frame: Option<BoardBank>,
    frame_awake: bool,
    /// Rim modes and the board's edge-force projections that drive them.
    rim: Option<crate::structure::RimBank>,
    rim_force: Vec<Vec<f32>>,
    rim_awake: bool,
    /// Soundboard driven by the primary forces (radiates and moves the
    /// bridge for the strings), and by the sympathetic ones (radiates only).
    board: Option<BoardBank>,
    board_sym: Option<BoardBank>,
    board_sym_awake: bool,
    board_awake: bool,
    /// Bridge velocity per region (this sub-block) and its last sample; the
    /// acceleration history (ring by absolute sample) and its delayed copy
    /// fed to the strings.
    disp: Vec<Vec<f32>>,
    last_vel: [f32; REGIONS],
    disp_ring: Vec<Vec<f32>>,
    disp_in: Vec<Vec<f32>>,
    /// Absolute sample index of the next sub-block.
    clock: u64,
    stereo: [Vec<f32>; MICS],
    listen_cursor: u8,
    knocks: [Knock; MAX_KNOCKS],
    room: Option<crate::room::Room>,
    /// Damper engagement below which a landing damper counts as an impact.
    sustain_prev: f64,
    /// Last summed mic outputs, for the differentiator.
    mic_last: [f32; MICS],
}

impl Engine {
    /// Builds the instrument (≈ 0.5 s): not for the audio thread.
    pub fn new(cfg: EngineConfig) -> Self {
        Self::with_instrument(cfg, Arc::new(Instrument::new(cfg.model, cfg.sample_rate)))
    }

    /// Engine on a prebuilt (shared) instrument; `cfg.model` is ignored.
    pub fn with_instrument(cfg: EngineConfig, inst: Arc<Instrument>) -> Self {
        assert_eq!(inst.sample_rate, cfg.sample_rate);
        let voices = (0..cfg.polyphony).map(|_| Voice::new(cfg.sample_rate, cfg.max_block)).collect();
        let rim_couple = inst.rim.as_ref().map_or(&[][..], |r| &r.couple[..]);
        let board = inst.board.as_ref().map(|m| BoardBank::with_extra(cfg.sample_rate, m, rim_couple, SUB_BLOCK));
        let frame = inst.frame.as_ref().map(|m| BoardBank::new(cfg.sample_rate, m, SUB_BLOCK));
        let rim = inst.rim.as_ref().map(|r| crate::structure::RimBank::new(r, cfg.sample_rate));
        let n_proj = inst.rim.as_ref().map_or(0, |r| r.projections.len());
        let board_sym = inst
            .board
            .as_ref()
            .filter(|_| inst.model.sympathetic.enabled)
            .map(|m| BoardBank::new(cfg.sample_rate, m, SUB_BLOCK));
        let room = match (inst.model.soundboard, inst.board.is_some()) {
            (Some(sb), true) if inst.model.room.enabled => {
                Some(crate::room::Room::new(&inst.model.room, &sb.mics, (sb.a, sb.b), cfg.sample_rate))
            }
            _ => None,
        };
        let mut e = Self {
            cfg,
            inst,
            voices,
            sustain: 0.0,
            soft: 0.0,
            sostenuto_down: false,
            region: vec![vec![0.0; SUB_BLOCK]; REGIONS],
            region_sym: vec![vec![0.0; SUB_BLOCK]; REGIONS],
            region_agraffe: vec![vec![0.0; SUB_BLOCK]; REGIONS],
            frame,
            frame_awake: false,
            rim,
            rim_force: vec![vec![0.0; SUB_BLOCK]; n_proj],
            rim_awake: false,
            board,
            board_sym,
            board_sym_awake: false,
            board_awake: false,
            disp: vec![vec![0.0; SUB_BLOCK]; REGIONS],
            last_vel: [0.0; REGIONS],
            disp_ring: vec![vec![0.0; RING]; REGIONS],
            disp_in: vec![vec![0.0; SUB_BLOCK]; REGIONS],
            clock: 0,
            stereo: std::array::from_fn(|_| vec![0.0; cfg.max_block]),
            listen_cursor: 21,
            knocks: [Knock::default(); MAX_KNOCKS],
            room,
            sustain_prev: 0.0,
            mic_last: [0.0; MICS],
        };
        // Strings without dampers always resonate.
        if e.sympathy() {
            for note in e.inst.model.damper.highest_note.saturating_add(1)..=108 {
                e.add_listener(note);
            }
        }
        e
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
        let board = if self.board_awake { self.board.as_ref().map_or(0, |b| b.len()) } else { 0 };
        let board_sym = if self.board_sym_awake { self.board_sym.as_ref().map_or(0, |b| b.len()) } else { 0 };
        self.voices.iter().filter(|v| v.active).map(|v| v.bank.len() + v.sym.len()).sum::<usize>() + board + board_sym
    }

    /// (note, bridge-force energy) of the active voices, for diagnostics.
    pub fn voice_energies(&self) -> Vec<(u8, f32)> {
        self.voices.iter().filter(|v| v.active).map(|v| (v.note, v.energy())).collect()
    }

    /// Bridge displacement fed to the strings at region `r` (diagnostics).
    pub fn bridge_input(&self, r: usize) -> &[f32] {
        &self.disp_in[r]
    }

    pub fn board_awake(&self) -> bool {
        self.board_awake
    }

    /// (note, primary energy, sympathetic energy) of the active voices.
    pub fn voice_energies_split(&self) -> Vec<(u8, f32, f32)> {
        self.voices.iter().filter(|v| v.active).map(|v| (v.note, v.bank.energy(), v.sym.energy())).collect()
    }

    fn sympathy(&self) -> bool {
        self.board.is_some() && self.inst.model.sympathetic.enabled
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
        let mut ev = 0;
        let mut s0 = 0;
        while s0 < len {
            let s1 = (s0 + SUB_BLOCK).min(len);
            let n_ev = events[ev..].iter().take_while(|e| (e.frame as usize) < s1 || s1 == len).count();
            self.sub_block(&events[ev..ev + n_ev], s0, &mut left[s0..s1], &mut right[s0..s1]);
            ev += n_ev;
            s0 = s1;
        }
    }

    /// One sub-block (≤ SUB_BLOCK samples); event frames are relative to the
    /// host block, which starts `offset` samples before this sub-block.
    fn sub_block(&mut self, events: &[TimedEvent], offset: usize, left: &mut [f32], right: &mut [f32]) {
        let len = left.len();
        let out_gain = 10f32.powf(self.inst.model.output_gain_db as f32 / 20.0);
        self.update_dampers(len);
        self.add_listeners();
        for r in self.region.iter_mut().chain(self.region_sym.iter_mut()).chain(self.region_agraffe.iter_mut()) {
            r[..len].fill(0.0);
        }
        // Bridge displacement SUB_BLOCK samples ago, for the strings.
        let t0 = self.clock;
        if self.sympathy() {
            for (ring, inp) in self.disp_ring.iter().zip(self.disp_in.iter_mut()) {
                for (i, x) in inp[..len].iter_mut().enumerate() {
                    *x = ring[((t0 + i as u64).wrapping_sub(SUB_BLOCK as u64) as usize) & (RING - 1)];
                }
            }
        }
        let mut pos = 0;
        for ev in events {
            let f = (ev.frame as usize).saturating_sub(offset).min(len);
            self.render(pos, f);
            pos = f;
            self.handle(ev.event);
        }
        self.render(pos, len);

        self.render_knocks(len);
        left.fill(0.0);
        right.fill(0.0);
        for d in &mut self.disp {
            d[..len].fill(0.0);
        }
        match self.board.as_mut() {
            None => {
                for r in &self.region {
                    for ((l, rr), x) in left.iter_mut().zip(right.iter_mut()).zip(&r[..len]) {
                        *l += out_gain * FORCE_GAIN * x;
                        *rr += out_gain * FORCE_GAIN * x;
                    }
                }
            }
            Some(board) => {
                let has_input = self.region.iter().any(|r| r[..len].iter().any(|&x| x != 0.0));
                for f in &mut self.rim_force {
                    f[..len].fill(0.0);
                }
                if has_input || self.board_awake {
                    let rim_force = self.rim.is_some().then_some(&mut self.rim_force[..]);
                    board.process(&self.region, len, [&mut *left, &mut *right], Some(&mut self.disp), rim_force);
                    self.board_awake = has_input || board.energy() > BOARD_SILENCE;
                }
                if let Some(rim) = self.rim.as_mut().filter(|_| self.board_awake || self.rim_awake) {
                    rim.process(&self.rim_force, len, [&mut *left, &mut *right]);
                    self.rim_awake = self.board_awake || rim.energy() > BOARD_SILENCE;
                }
                if let Some(frame) = self.frame.as_mut() {
                    let has = self.region_agraffe.iter().any(|r| r[..len].iter().any(|&x| x != 0.0));
                    if has || self.frame_awake {
                        frame.process(&self.region_agraffe, len, [&mut *left, &mut *right], None, None);
                        self.frame_awake = has || frame.energy() > BOARD_SILENCE;
                    }
                }
                if let Some(bs) = self.board_sym.as_mut() {
                    let has_sym = self.region_sym.iter().any(|r| r[..len].iter().any(|&x| x != 0.0));
                    if has_sym || self.board_sym_awake {
                        bs.process(&self.region_sym, len, [&mut *left, &mut *right], None, None);
                        self.board_sym_awake = has_sym || bs.energy() > BOARD_SILENCE;
                    }
                }
                // The banks carry acceleration weights: the pressure is the
                // derivative of their sum (board::to_acceleration_weights).
                let fs = self.cfg.sample_rate as f32;
                for (ch, last) in [&mut *left, &mut *right].into_iter().zip(self.mic_last.iter_mut()) {
                    for x in ch.iter_mut() {
                        let v = *x;
                        *x = (v - *last) * fs;
                        *last = v;
                    }
                }
                for x in left.iter_mut().chain(right.iter_mut()) {
                    *x *= out_gain * PRESSURE_GAIN;
                }
                if let Some(room) = self.room.as_mut() {
                    room.process(left, right);
                }
            }
        }
        // Bridge acceleration by backward difference of the velocity (its
        // half-sample lag is compensated in the strings' input phase).
        let fs = self.cfg.sample_rate as f32;
        for ((ring, d), last) in self.disp_ring.iter_mut().zip(&self.disp).zip(self.last_vel.iter_mut()) {
            for (i, &v) in d[..len].iter().enumerate() {
                ring[((t0 + i as u64) as usize) & (RING - 1)] = (v - *last) * fs;
                *last = v;
            }
        }
        self.clock += len as u64;

        let sympathy = self.sympathy();
        for v in &mut self.voices {
            if !v.active {
                continue;
            }
            let listening = sympathy && v.engagement < 0.5;
            if !listening && v.energy() < SILENCE_ENERGY {
                v.active = false;
            }
        }
    }

    fn render(&mut self, from: usize, to: usize) {
        if from >= to {
            return;
        }
        let sympathy = self.sympathy();
        let with_frame = self.frame.is_some();
        let t0 = self.clock;
        for v in self.voices.iter_mut().filter(|v| v.active) {
            let nm = self.inst.table.get(v.note);
            let r = region_of(v.note);
            // Fully damped strings do not resonate: skip their input.
            let bridge = (sympathy && v.engagement < 0.99).then(|| (&self.disp_in[r][from..to], SUB_BLOCK));
            v.render(
                nm,
                &mut self.region[r][from..to],
                &mut self.region_sym[r][from..to],
                with_frame.then(|| &mut self.region_agraffe[r][from..to]),
                t0,
                from,
                to,
                bridge,
            );
        }
    }

    /// Adds the pending frame knocks to the region forces.
    fn render_knocks(&mut self, len: usize) {
        for k in self.knocks.iter_mut().filter(|k| k.pos < k.len) {
            let r = &mut self.region[k.region];
            for x in r[..len].iter_mut() {
                if k.pos >= k.len {
                    break;
                }
                *x += k.peak * (std::f32::consts::PI * (k.pos as f32 + 0.5) / k.len as f32).sin();
                k.pos += 1;
            }
        }
    }

    fn knock(&mut self, region: usize, peak: f64) {
        if peak <= 0.0 || self.board.is_none() {
            return;
        }
        let len = ((self.inst.model.noise.knock_time * self.cfg.sample_rate) as usize).max(2);
        if let Some(k) = self.knocks.iter_mut().find(|k| k.pos >= k.len) {
            *k = Knock { region, pos: 0, len, peak: peak as f32 };
        }
    }

    /// Moves every damper towards its target contact and sets the decay.
    fn update_dampers(&mut self, len: usize) {
        let law = self.inst.model.damper;
        let dt = len as f64 / self.cfg.sample_rate;
        let pedal = law.pedal_engagement(self.sustain);
        let (k_fall, k_lift) = (1.0 - (-dt / law.fall_time).exp(), 1.0 - (-dt / law.lift_time).exp());
        for v in self.voices.iter_mut().filter(|v| v.active) {
            let nm = self.inst.table.get(v.note);
            let target = if v.key_down || v.sostenuto || !nm.has_damper { 0.0 } else { pedal };
            let k = if target > v.engagement { k_fall } else { k_lift };
            let e = v.engagement + (target - v.engagement) * k;
            let e = if (e - target).abs() < 1e-3 { target } else { e };
            // The felt lands on the strings: momentum kick at the damper point.
            if v.engagement < 0.05 && e >= 0.05 {
                let noise = self.inst.model.noise;
                let impulse = noise.damper_mass * noise.damper_speed * v.damper_speed;
                v.damper_kick(nm, impulse, law.position, law.width, noise.damper_landing);
            }
            v.engagement = e;
            v.set_damper_rate(nm.damper_rate(&self.inst.model, v.engagement));
        }
    }

    /// While the dampers are lifted, give every damped key a listening string.
    fn add_listeners(&mut self) {
        let law = self.inst.model.damper;
        if !self.sympathy() || law.pedal_engagement(self.sustain) >= 0.5 {
            return;
        }
        let mut added = 0;
        for _ in 21..=law.highest_note.min(108) {
            let note = self.listen_cursor;
            self.listen_cursor = if note >= law.highest_note.min(108) { 21 } else { note + 1 };
            if !self.voices.iter().any(|v| v.active && v.note == note) && self.add_listener(note) {
                added += 1;
                if added == LISTENERS_PER_BLOCK {
                    break;
                }
            }
        }
    }

    /// Sets up a free voice as a listening string; false when none is free.
    fn add_listener(&mut self, note: u8) -> bool {
        let Some(v) = self.voices.iter_mut().find(|v| !v.active) else { return false };
        let nm = self.inst.table.get(note);
        v.setup(nm, true, INPUT_DELAY / self.cfg.sample_rate);
        v.active = true;
        v.key_down = false;
        let law = self.inst.model.damper;
        v.engagement = if nm.has_damper { law.pedal_engagement(self.sustain) } else { 0.0 };
        v.set_damper_rate(nm.damper_rate(&self.inst.model, v.engagement));
        true
    }

    fn handle(&mut self, ev: Event) {
        match ev {
            Event::NoteOn { velocity: 0, note } => self.note_off(note),
            Event::NoteOn { note, velocity } => self.note_on(note, velocity),
            Event::NoteOff { note, velocity } => {
                // Release velocity sets how fast the damper falls.
                let speed = if velocity == 0 { 1.0 } else { velocity as f64 / 64.0 };
                for v in self.voices.iter_mut().filter(|v| v.active && v.note == note) {
                    v.damper_speed = speed;
                }
                self.note_off(note);
            }
            Event::Control { cc: 64, value } => {
                let new = value as f64 / 127.0;
                let delta = (new - self.sustain_prev).abs();
                // The damper rail knocks the frame near the middle of the
                // keyboard; on release the rail lowers the dampers, slower
                // than a key does (its travel is cushioned by felts).
                let peak = self.inst.model.noise.pedal * delta;
                self.knock(region_of(54), 0.6 * peak);
                self.knock(region_of(66), 0.4 * peak);
                if new < self.sustain {
                    let speed = (delta * 0.3).clamp(0.02, 0.2);
                    for v in self.voices.iter_mut().filter(|v| v.active && !v.key_down) {
                        v.damper_speed = speed;
                    }
                }
                self.sustain = new;
                self.sustain_prev = new;
            }
            Event::Control { cc: 66, value } => {
                let down = value >= 64;
                if down && !self.sostenuto_down {
                    for v in self.voices.iter_mut().filter(|v| v.active) {
                        v.sostenuto = v.key_down;
                    }
                } else if !down {
                    for v in &mut self.voices {
                        v.sostenuto = false;
                    }
                }
                self.sostenuto_down = down;
            }
            Event::Control { cc: 67, value } => self.soft = value as f64 / 127.0,
            Event::Control { cc: 120 | 123, .. } | Event::AllNotesOff => {
                for v in &mut self.voices {
                    v.key_down = false;
                    v.sostenuto = false;
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
            v.setup(nm, false, INPUT_DELAY / cfg.sample_rate);
            v.engagement = 0.0;
        }
        v.active = true;
        v.key_down = true;
        let softening = self.inst.model.una_corda_softening;
        let vh = hammer_velocity(&cfg, velocity);
        v.strike(nm, vh, contact_oversampling(&cfg, note), self.soft, softening);
        v.damper_speed = 1.0;
        // The key bottoms on the keybed as the hammer reaches the string.
        self.knock(region_of(note), self.inst.model.noise.keybed * vh);
    }

    fn note_off(&mut self, note: u8) {
        for v in self.voices.iter_mut().filter(|v| v.active && v.note == note) {
            v.key_down = false;
        }
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

/// MIDI velocity → hammer velocity (see [`crate::params::VelocityLaw`]).
pub fn hammer_velocity(cfg: &EngineConfig, velocity: u8) -> f64 {
    cfg.model.velocity.hammer_speed(velocity)
}

/// Contact duration (s) and peak force (N) of one strike on a string at
/// rest. Allocates: analysis use only.
pub fn strike_stats(cfg: &EngineConfig, note: u8, velocity: u8, oversample: Option<usize>) -> (f64, f64) {
    let (t, f, _) = strike_stats_full(cfg, note, velocity, oversample);
    (t, f)
}

/// Contact duration (s), peak force (N) and largest felt compression (m).
pub fn strike_stats_full(cfg: &EngineConfig, note: u8, velocity: u8, oversample: Option<usize>) -> (f64, f64, f64) {
    let board = cfg.model.soundboard.map(crate::soundboard::Soundboard::new);
    let nm = crate::note_model::NoteModel::build(&cfg.model, board.as_ref(), None, note, cfg.sample_rate, MAX_MODES);
    let mut v = Voice::new(cfg.sample_rate, cfg.max_block);
    v.setup(&nm, false, 0.0);
    let os = oversample.unwrap_or_else(|| contact_oversampling(cfg, note));
    v.strike(&nm, hammer_velocity(cfg, velocity), os, 0.0, 0.0);
    let mut out = vec![0.0f32; 256];
    while v.contact.is_active() {
        v.contact.run(&mut out, None);
    }
    (v.contact.contact_time(), v.contact.max_force(), v.contact.max_compression())
}
