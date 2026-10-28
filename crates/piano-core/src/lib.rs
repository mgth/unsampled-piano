//! Physics-based piano engine. Framework-agnostic: hosts (plugin, standalone,
//! offline renderer) wrap [`engine::Engine`].

pub mod action;
pub mod board;
pub mod complex;
pub mod denormal;
pub mod eigen;
pub mod engine;
pub mod hammer;
pub mod instrument;
pub mod limiter;
pub mod live;
pub mod macros;
pub mod modal;
pub mod note_model;
pub mod observe;
pub mod par;
pub mod params;
pub mod room;
mod simd;
pub mod soundboard;
pub mod structure;
pub mod unison;
pub mod upsample;
pub mod voice;

pub use engine::{Engine, EngineConfig, Event, Quality, TimedEvent};
pub use live::LiveParam;
pub use observe::Observation;
