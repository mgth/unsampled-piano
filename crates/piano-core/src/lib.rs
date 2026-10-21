//! Physics-based piano engine. Framework-agnostic: hosts (plugin, standalone,
//! offline renderer) wrap [`engine::Engine`].

pub mod complex;
pub mod denormal;
pub mod engine;
pub mod hammer;
pub mod instrument;
pub mod modal;
pub mod note_model;
pub mod params;
pub mod soundboard;
pub mod unison;
pub mod voice;

pub use engine::{Engine, EngineConfig, Event, TimedEvent};
