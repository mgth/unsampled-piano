//! Physics-based guitar engine. Framework-agnostic: hosts wrap
//! [`engine::Engine`].
//!
//! Skeleton stage (see docs/guitar_plan.md): six stiff, lossy strings on a
//! rigid bridge, released from an ideal pluck shape, heard as the force they
//! put on the bridge. The body, the two polarisations and a real finger or
//! plectrum contact come next.
//!
//! The DSP core is piano-core's: [`piano_core::modal::ModalBank`] runs the
//! string modes, [`piano_core::denormal`] guards the audio callback. Nothing
//! piano-specific (hammer, dampers, unisons, soundboard) is used.

pub mod engine;
pub mod instrument;
pub mod params;
pub mod string;

pub use engine::{Engine, Event};
pub use instrument::Instrument;
pub use params::GuitarModel;
