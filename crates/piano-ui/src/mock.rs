//! A control with no engine behind it, to work on the interface: edits
//! change the model, rebuild parameters fake a build after a pause, and
//! the live view sees it play a slow arpeggio by itself.

use std::time::{Duration, Instant};

use piano_core::params::PianoModel;
use piano_core::Observation;
use piano_params::midi::{CcAction, MidiLearn, MidiMap};
use piano_params::{Kind, PianoControl, Registry, Status, Value};

/// Pause after the last rebuild edit, then simulated build time.
const DEBOUNCE: Duration = Duration::from_millis(150);
const BUILD: Duration = Duration::from_millis(120);

pub struct MockControl {
    registry: Registry,
    model: PianoModel,
    /// Last rebuild edit, while a (fake) build is pending.
    pending: Option<Instant>,
    error: Option<String>,
    midi: MidiLearn,
    start: Instant,
}

impl MockControl {
    pub fn new(model: PianoModel) -> Self {
        Self { registry: Registry::new(), model, pending: None, error: None, midi: MidiLearn::new(MidiMap::default()), start: Instant::now() }
    }

    /// A controller moved (what the audio side reports to a real host).
    pub fn on_cc(&mut self, cc: u8, value: u8) {
        if let CcAction::Set { path, value, .. } = self.midi.on_cc(cc, value) {
            self.set(&path, value);
        }
    }

    fn rebuild(&mut self) {
        self.pending = Some(Instant::now());
    }
}

impl PianoControl for MockControl {
    fn model(&self) -> PianoModel {
        self.model
    }

    fn begin_edit(&mut self, _path: &str) {}

    fn set(&mut self, path: &str, value: Value) {
        match piano_params::set(&self.model, path, value) {
            Ok(m) => {
                self.model = m;
                self.error = None;
                if self.registry.get(path).is_some_and(|p| p.kind == Kind::Rebuild) {
                    self.rebuild();
                }
            }
            Err(e) => self.error = Some(e),
        }
    }

    fn end_edit(&mut self, _path: &str) {}

    fn load(&mut self, model: PianoModel) {
        self.model = model;
        self.rebuild();
    }

    fn midi(&mut self) -> Option<&mut MidiLearn> {
        Some(&mut self.midi)
    }

    /// An arpeggio of C major over two octaves, a note every 0.4 s held
    /// 0.3 s, the sustain pedal down every other bar.
    fn observe(&self) -> Option<Observation> {
        const NOTES: [u8; 8] = [48, 52, 55, 60, 64, 67, 72, 76];
        const STEP: f32 = 0.4;
        let t = self.start.elapsed().as_secs_f32();
        let bar = (t / (STEP * NOTES.len() as f32)) as u32;
        let mut o = Observation { sustain: if bar % 2 == 1 { 1.0 } else { 0.0 }, ..Default::default() };
        for (i, &n) in NOTES.iter().enumerate() {
            // Time since this note was last struck.
            let period = STEP * NOTES.len() as f32;
            let age = (t - i as f32 * STEP).rem_euclid(period);
            let k = &mut o.keys[n as usize];
            k.down = age < 0.3;
            k.hammer = age < 0.004;
            k.damper = if k.down { 0.0 } else { 1.0 - o.sustain };
            // Faster decay once the damper is back.
            let rate = if k.damper > 0.5 { 12.0 } else { 1.5 };
            k.energy = (-(age.min(0.3)) * 1.5 - (age - 0.3).max(0.0) * rate).exp() * (1.0 + 0.1 * n as f32);
            // Each note drives a few board modes of its own.
            for m in 0..24 {
                let w = ((n as usize * 7 + m * 13) % 17) as f32 / 17.0;
                o.board[m] = o.board[m].hypot(k.energy.sqrt() * w / (1.0 + m as f32 * 0.3));
            }
        }
        o.board_modes = 24;
        Some(o)
    }

    fn status(&self) -> Status {
        let rebuilding = self.pending.is_some_and(|t| t.elapsed() < DEBOUNCE + BUILD);
        Status {
            rebuilding,
            last_build: self.pending.filter(|_| !rebuilding).map(|_| BUILD),
            voices: 0,
            load: 0.0,
            doubled_notes: 0,
            clipped: 0,
            peak: 0.0,
            limiting_db: 0.0,
            error: self.error.clone(),
        }
    }
}
