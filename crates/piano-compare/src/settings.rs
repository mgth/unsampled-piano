//! What the window remembers from one start to the next: the key, the
//! layer, the piano played, which pianos are drawn and which make the
//! two bands, the measures shown and the playback options. Pianos and measures
//! are stored by name, so a set added later appears with its defaults.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub note: Option<String>,
    pub layer: Option<String>,
    /// Key of the piano the keyboard plays.
    pub current: Option<String>,
    /// Pianos not drawn.
    pub hidden: Vec<String>,
    /// Pianos put in the band, or taken out of it, against their default
    /// (the real pianos in, the others out).
    pub band_in: Vec<String>,
    pub band_out: Vec<String>,
    /// Pianos in the second band (empty by default).
    pub band2: Vec<String>,
    /// Measures not shown, by name.
    pub hidden_measures: Vec<String>,
    pub band_mode: Option<String>,
    pub match_levels: Option<bool>,
    pub autoplay: Option<bool>,
    pub seconds: Option<f32>,
    pub volume_db: Option<f32>,
    /// Height of the part that does not scroll (keyboard, table, pianos,
    /// partials), dragged by its lower edge.
    pub fixed_height: Option<f32>,
}

/// `PIANO_COMPARE_SETTINGS`, else compare.toml in the instrument's
/// configuration folder (~/.config/unsampled-piano on Linux).
pub fn path() -> Option<PathBuf> {
    std::env::var_os("PIANO_COMPARE_SETTINGS").map(PathBuf::from).or_else(|| piano_host::config_dir().map(|d| d.join("compare.toml")))
}

pub fn load() -> Settings {
    path().and_then(|p| std::fs::read_to_string(p).ok()).and_then(|t| toml::from_str(&t).ok()).unwrap_or_default()
}

pub fn save(s: &Settings) {
    let Some(p) = path() else { return };
    if let Some(d) = p.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    if let Ok(text) = toml::to_string(s) {
        let tmp = p.with_extension("part");
        if std::fs::write(&tmp, text).is_ok() {
            let _ = std::fs::rename(&tmp, &p);
        }
    }
}
