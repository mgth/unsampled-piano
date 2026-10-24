//! Physics-based guitar engine. Framework-agnostic: hosts wrap
//! [`engine::Engine`].
//!
//! Stage G1 (see docs/guitar_plan.md): six stiff, lossy strings in two
//! polarisations on the bridge's admittance, plucked by a fingertip through
//! a per-sample modal contact solver, their motion carried over when the
//! stopped length changes; heard as the force they put on the bridge. The
//! body, the pickups and the other excitations come next.
//!
//! The DSP core is piano-core's: [`piano_core::modal::ModalBank`] runs the
//! string modes, [`piano_core::denormal`] guards the audio callback. Nothing
//! piano-specific (hammer, dampers, unisons, soundboard) is used.

pub mod contact;
pub mod engine;
pub mod instrument;
pub mod params;
pub mod project;
pub mod string;

pub use engine::{Engine, Event};
pub use instrument::Instrument;
pub use params::GuitarModel;
