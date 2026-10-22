//! Physics-based piano engine. Framework-agnostic: hosts (plugin, standalone,
//! offline renderer) wrap [`engine::Engine`].

pub mod board;
pub mod complex;
pub mod denormal;
pub mod engine;
pub mod hammer;
pub mod instrument;
pub mod live;
pub mod modal;
pub mod note_model;
pub mod par;
pub mod params;
pub mod room;
pub mod soundboard;
pub mod structure;
pub mod unison;
pub mod voice;

pub use engine::{Engine, EngineConfig, Event, TimedEvent};
pub use live::LiveParam;
