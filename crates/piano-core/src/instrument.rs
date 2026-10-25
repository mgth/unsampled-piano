//! Everything precomputed from a [`PianoModel`] at a given sample rate:
//! the coupled modes of every key and the soundboard's modal bank data.
//! Built off the audio thread (≈ 0.5 s) and shared read-only by engines.

use crate::board::{self, BoardMode};
use crate::note_model::{PianoTable, Resolution};
use crate::params::PianoModel;
use crate::soundboard::Soundboard;
use crate::structure::RimModel;

pub use crate::board::{region_of, MICS, REGIONS};

pub struct Instrument {
    pub model: PianoModel,
    pub sample_rate: f64,
    pub table: PianoTable,
    /// Soundboard modes when it is rendered (radiation and sympathy).
    pub board: Option<Vec<BoardMode>>,
    /// Frame modes (driven at the agraffes) and rim model, when enabled
    /// with a rendered soundboard.
    pub frame: Option<Vec<BoardMode>>,
    pub rim: Option<RimModel>,
}

impl Instrument {
    pub fn new(model: PianoModel, sample_rate: f64) -> Self {
        Self::with_resolution(model, sample_rate, Resolution::DEFAULT)
    }

    pub fn with_resolution(model: PianoModel, sample_rate: f64, res: Resolution) -> Self {
        // The musician's controls applied, microphones in metres from here on
        // (the engine's room uses them too).
        let model = model.with_macros();
        let model = PianoModel { soundboard: model.soundboard.map(|s| s.resolved()), ..model };
        let sb = model.soundboard.map(Soundboard::new);
        let modes = sb.as_ref().filter(|s| s.params.radiate).map(|s| board::board_modes(s, sample_rate));
        let table = PianoTable::with_resolution(&model, sb.as_ref(), modes.as_deref(), sample_rate, res);
        let rendered = sb.as_ref().zip(modes.as_ref());
        let frame = rendered
            .filter(|_| model.frame.enabled)
            .map(|(s, _)| crate::structure::frame_modes(&model.frame, &s.params, sample_rate));
        let mut rim = rendered
            .filter(|_| model.rim.enabled)
            .map(|(s, m)| RimModel::new(&model.rim, s, m.len(), sample_rate));
        // The engine differentiates the summed mic outputs (pressure follows
        // the acceleration): every radiating bank gets acceleration weights.
        let mut modes = modes;
        let mut frame = frame;
        // The listening set-up's gain scales every mic weight.
        let mic_gain = 10f64.powf(model.soundboard.map_or(0.0, |s| s.mic_gain_db) / 20.0);
        for m in modes.iter_mut().chain(frame.iter_mut()) {
            board::to_acceleration_weights(m, sample_rate);
            for bm in m.iter_mut() {
                for o in &mut bm.outputs[..board::MICS] {
                    *o = (o.0 * mic_gain, o.1 * mic_gain);
                }
            }
        }
        if let Some(r) = rim.as_mut() {
            for m in &mut r.modes {
                let d = board::mic_differentiator(std::f64::consts::TAU * m.freq_hz, sample_rate);
                m.mics = m.mics.map(|(re, im)| {
                    let w = crate::complex::C64::new(re, im).scale(mic_gain) / d;
                    (w.re, w.im)
                });
            }
        }
        Self { model, sample_rate, table, board: modes, frame, rim }
    }
}
