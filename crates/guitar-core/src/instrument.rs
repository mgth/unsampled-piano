//! Everything precomputed from a [`GuitarModel`] at a given sample rate:
//! the modes of every string at every fret. Built off the audio thread and
//! shared read-only by engines.

use crate::params::GuitarModel;
use crate::string::StringTable;

pub struct Instrument {
    pub model: GuitarModel,
    pub sample_rate: f64,
    /// `tables[string][fret]`, fret 0 the open string.
    pub tables: Vec<Vec<StringTable>>,
    /// Most modes of any table: the capacity of a string's bank.
    pub max_modes: usize,
}

impl Instrument {
    pub fn new(model: GuitarModel, sample_rate: f64) -> Self {
        let tables: Vec<Vec<StringTable>> = (0..model.strings.len())
            .map(|s| (0..=model.frets).map(|f| StringTable::new(&model, s, f, sample_rate)).collect())
            .collect();
        let max_modes = tables.iter().flatten().map(|t| t.shape.len()).max().unwrap_or(0);
        Self { model, sample_rate, tables, max_modes }
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
