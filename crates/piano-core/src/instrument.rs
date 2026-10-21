//! Everything precomputed from a [`PianoModel`] at a given sample rate:
//! the coupled modes of every key and the soundboard's modal bank data.
//! Built off the audio thread (≈ 0.5 s) and shared read-only by engines.

use crate::board::{self, BoardMode};
use crate::note_model::PianoTable;
use crate::params::PianoModel;
use crate::soundboard::Soundboard;
use crate::voice::MAX_MODES;

pub use crate::board::{region_of, MICS, REGIONS};

pub struct Instrument {
    pub model: PianoModel,
    pub sample_rate: f64,
    pub table: PianoTable,
    /// Soundboard modes when it is rendered (radiation and sympathy).
    pub board: Option<Vec<BoardMode>>,
}

impl Instrument {
    pub fn new(model: PianoModel, sample_rate: f64) -> Self {
        let sb = model.soundboard.map(Soundboard::new);
        let modes = sb.as_ref().filter(|s| s.params.radiate).map(|s| board::board_modes(s, sample_rate));
        let table = PianoTable::new(&model, sb.as_ref(), modes.as_deref(), sample_rate, MAX_MODES);
        Self { model, sample_rate, table, board: modes }
    }
}
