//! A control with no engine behind it, to work on the interface: edits
//! change the model, rebuild parameters fake a build after a pause.

use std::time::{Duration, Instant};

use piano_core::params::PianoModel;
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
}

impl MockControl {
    pub fn new(model: PianoModel) -> Self {
        Self { registry: Registry::new(), model, pending: None, error: None }
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

    fn status(&self) -> Status {
        let rebuilding = self.pending.is_some_and(|t| t.elapsed() < DEBOUNCE + BUILD);
        Status {
            rebuilding,
            last_build: self.pending.filter(|_| !rebuilding).map(|_| BUILD),
            voices: 0,
            load: 0.0,
            error: self.error.clone(),
        }
    }
}
