//! What the interface needs from a running instrument. Implemented by the
//! standalone player and by the plugin wrapper (where edits go through the
//! host, which records automation between `begin_edit` and `end_edit`).

use std::time::Duration;

use piano_core::params::PianoModel;

use crate::Value;

pub trait PianoControl {
    /// The model as edited: ahead of the sound while a rebuild is pending.
    fn model(&self) -> PianoModel;
    /// A gesture on one parameter (slider drag) starts.
    fn begin_edit(&mut self, path: &str);
    /// Live parameters reach the engine at once; the others schedule a
    /// rebuild once the edits pause.
    fn set(&mut self, path: &str, value: Value);
    fn end_edit(&mut self, path: &str);
    /// Replaces the whole model (preset load): a single rebuild.
    fn load(&mut self, model: PianoModel);
    fn status(&self) -> Status;
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Status {
    /// An instrument is being built, or edits wait for one.
    pub rebuilding: bool,
    /// Duration of the last instrument build.
    pub last_build: Option<Duration>,
    pub voices: u32,
    /// Audio callback time over buffer duration (max of the last period).
    pub load: f32,
    /// Last error (preset, build), for display.
    pub error: Option<String>,
}
