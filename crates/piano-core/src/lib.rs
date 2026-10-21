//! Physics-based piano engine. Framework-agnostic: hosts (plugin, standalone,
//! offline renderer) wrap [`engine::Engine`].

pub mod denormal;
pub mod engine;
pub mod modal;
pub mod params;

pub use engine::{Engine, EngineConfig, Event, TimedEvent};
