//! Everything precomputed from a [`PianoModel`] at a given sample rate:
//! the coupled modes of every key and the soundboard's modal bank data.
//! Built off the audio thread (≈ 0.5 s) and shared read-only by engines.

use crate::board::{self, BoardMode};
use crate::note_model::PianoTable;
use crate::params::PianoModel;
use crate::soundboard::Soundboard;
use crate::structure::RimModel;
use crate::voice::MAX_MODES;

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
        let sb = model.soundboard.map(Soundboard::new);
        let modes = sb.as_ref().filter(|s| s.params.radiate).map(|s| board::board_modes(s, sample_rate));
        let table = PianoTable::new(&model, sb.as_ref(), modes.as_deref(), sample_rate, MAX_MODES);
        let rendered = sb.as_ref().zip(modes.as_ref());
        let frame = rendered
            .filter(|_| model.frame.enabled)
            .map(|(s, _)| crate::structure::frame_modes(&model.frame, &s.params, sample_rate));
        let rim = rendered
            .filter(|_| model.rim.enabled)
            .map(|(s, m)| RimModel::new(&model.rim, s, m.len(), sample_rate));
        Self { model, sample_rate, table, board: modes, frame, rim }
    }
}
