//! Physics-based guitar engine. Framework-agnostic: hosts wrap
//! [`engine::Engine`].
//!
//! Stage G1 (see docs/guitar_plan.md): six stiff, lossy strings in two
//! polarisations on the bridge's admittance, plucked by a fingertip through
//! a per-sample modal contact solver, their motion carried over when the
//! stopped length changes. G5/G2 basics: magnetic pickups (single coils,
//! humbuckers) through the guitar's passive circuit, a piezo, and finger
//! and plectrum plucks that slip off at their force. G3: the frets under the
//! strings (buzz), slap, pop, tapping, hammer-ons and pull-offs. G4: live
//! tension (bends, vibrato, the tremolo arm, the glide of hard plucks).
//! Acoustic bodies come next.
//!
//! The DSP core is piano-core's: [`piano_core::modal::ModalBank`] runs the
//! string modes, [`piano_core::denormal`] guards the audio callback. Nothing
//! piano-specific (hammer, dampers, unisons, soundboard) is used.

pub mod contact;
pub mod engine;
pub mod frets;
pub mod instrument;
pub mod output;
pub mod params;
pub mod project;
pub mod string;

pub use engine::{Engine, Event};
pub use instrument::Instrument;
pub use params::GuitarModel;
