//! Everything precomputed from a [`PianoModel`] at a given sample rate:
//! the coupled modes of every key and the soundboard's radiation banks.
//! Built off the audio thread (≈ 0.1–1 s) and shared read-only by engines.

use crate::modal::ModeSpec;
use crate::note_model::PianoTable;
use crate::params::PianoModel;
use crate::soundboard::Soundboard;
use crate::voice::MAX_MODES;

/// Keys are grouped into this many bridge regions feeding the soundboard.
pub const REGIONS: usize = 16;
pub const MICS: usize = 2;

pub struct BoardData {
    /// One-pole low-pass coefficient for the region forces (bridge mass),
    /// y[n] = a·y[n−1] + (1−a)·F[n]; None when the bridge mass is 0.
    pub input_lowpass: Option<f32>,
    /// Soundboard modes as driven by one bridge region, with the two
    /// microphones as the bank's two outputs: index `region`.
    pub banks: Vec<Vec<ModeSpec>>,
}

pub struct Instrument {
    pub model: PianoModel,
    pub sample_rate: f64,
    pub table: PianoTable,
    pub board: Option<BoardData>,
}

/// Bridge region of a key.
pub fn region_of(note: u8) -> usize {
    ((note.clamp(21, 108) - 21) as usize * REGIONS / 88).min(REGIONS - 1)
}

impl Instrument {
    pub fn new(model: PianoModel, sample_rate: f64) -> Self {
        let sb = model.soundboard.map(Soundboard::new);
        let table = PianoTable::new(&model, sb.as_ref(), sample_rate, MAX_MODES);
        let board = sb.filter(|sb| sb.params.radiate).map(|sb| {
            let rad = sb.radiation();
            let nyq = 0.45 * sample_rate;
            let t = 1.0 / sample_rate;
            let mut banks = Vec::with_capacity(REGIONS);
            for r in 0..REGIONS {
                // Bridge point of the region's middle key.
                let mid = 21 + ((2 * r + 1) * 88 / (2 * REGIONS)) as u8;
                let (x, y) = sb.params.bridge_point(mid);
                banks.push(
                    sb.modes
                        .iter()
                        .zip(&rad)
                        .filter(|(md, _)| md.omega / std::f64::consts::TAU < nyq)
                        .map(|(md, w)| ModeSpec {
                            freq_hz: md.omega / std::f64::consts::TAU,
                            sigma: sb.sigma(md),
                            // Modal velocity response to a force: Φ(x_r)·T per sample.
                            gain_re: sb.shape(md, x, y) * t,
                            gain_im: 0.0,
                            out_re: w[0].re,
                            out_im: w[0].im,
                            out2_re: w[1].re,
                            out2_im: w[1].im,
                        })
                        .collect(),
                );
            }
            let input_lowpass = sb.bridge_mass_corner().map(|wc| (-wc * t).exp() as f32);
            BoardData { input_lowpass, banks }
        });
        Self { model, sample_rate, table, board }
    }
}
