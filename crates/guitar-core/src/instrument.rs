//! Everything precomputed from a [`GuitarModel`] at a given sample rate:
//! the modes of every string at every fret, ringing, and of the open string
//! muted, by the hand near the nut or by a finger lifting off each fret.
//! Built off the audio thread and shared read-only by engines.

use crate::params::GuitarModel;
use crate::string::{fret_length, StringTable};

pub struct Instrument {
    pub model: GuitarModel,
    pub sample_rate: f64,
    /// `tables[string][fret]`, fret 0 the open string, ringing.
    tables: Vec<Vec<StringTable>>,
    /// `released[string][fret]`: the open length, muted (run at damper
    /// scale 1) by the fingertip that pressed behind that fret and now lifts
    /// off; fret 0, by the hand near the nut.
    released: Vec<Vec<StringTable>>,
    /// Most partials of any table: what a string's bank and solver hold.
    pub max_partials: usize,
}

impl Instrument {
    pub fn new(model: GuitarModel, sample_rate: f64) -> Self {
        let (l0, d) = (model.scale_length, model.mute.distance);
        let tables: Vec<Vec<StringTable>> = (0..model.strings.len())
            .map(|s| {
                (0..=model.frets)
                    .map(|f| {
                        let l = fret_length(l0, f);
                        StringTable::new(&model, s, l, l - d, sample_rate)
                    })
                    .collect()
            })
            .collect();
        let released: Vec<Vec<StringTable>> = (0..model.strings.len())
            .map(|s| {
                (0..=model.frets)
                    .map(|f| {
                        let at = if f == 0 { l0 - d } else { fret_length(l0, f) + d };
                        StringTable::muted(&model, s, l0, at, sample_rate)
                    })
                    .collect()
            })
            .collect();
        let max_partials = tables.iter().chain(&released).flatten().map(|t| t.len()).max().unwrap_or(0);
        Self { model, sample_rate, tables, released, max_partials }
    }

    pub fn table(&self, string: usize, fret: u8) -> &StringTable {
        &self.tables[string][fret as usize]
    }

    /// The open string, muted: after the finger at `fret` lifts (fret 0: by
    /// the hand).
    pub fn released(&self, string: usize, fret: u8) -> &StringTable {
        &self.released[string][fret as usize]
    }

    /// Strings and frets that can play `note`, lowest fret first.
    pub fn positions(&self, note: u8) -> Vec<(usize, u8)> {
        let mut p: Vec<(usize, u8)> = self
            .model
            .strings
            .iter()
            .enumerate()
            .filter(|(_, s)| note >= s.open_note && note - s.open_note <= self.model.frets)
            .map(|(i, s)| (i, note - s.open_note))
            .collect();
        p.sort_by_key(|&(_, fret)| fret);
        p
    }
}
