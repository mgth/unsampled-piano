//! Host-side helpers shared by the renderer and the real-time player:
//! instrument presets (TOML) and decoding of raw MIDI messages.

use piano_core::params::PianoModel;
use piano_core::Event;
use std::path::Path;

/// Loads an instrument preset. Missing sections or fields keep the default
/// instrument's values, so a preset only needs what it changes.
pub fn load_preset(path: &Path) -> Result<PianoModel, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
}

/// The full preset text of a model (every parameter, with its value).
pub fn preset_toml(model: &PianoModel) -> String {
    let body = toml::to_string_pretty(model).expect("model serialises");
    format!(
        "# Physical piano preset. Every field is optional: omitted ones keep the\n\
         # default instrument's value. Units are SI unless stated otherwise.\n\n{body}"
    )
}

pub fn save_preset(model: &PianoModel, path: &Path) -> Result<(), String> {
    std::fs::write(path, preset_toml(model)).map_err(|e| format!("{}: {e}", path.display()))
}

/// Decodes a raw MIDI channel message (any channel) into an engine event.
pub fn decode_midi(msg: &[u8]) -> Option<Event> {
    let (&status, data) = msg.split_first()?;
    let (a, b) = (*data.first()?, data.get(1).copied().unwrap_or(0));
    match status & 0xF0 {
        0x90 => Some(Event::NoteOn { note: a, velocity: b }),
        0x80 => Some(Event::NoteOff { note: a, velocity: b }),
        0xB0 => Some(Event::Control { cc: a, value: b }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preset_round_trips_and_partial_presets_keep_defaults() {
        let d = PianoModel::default();
        let m = PianoModel { a4: 442.0, unison: piano_core::params::UnisonLaw { width_mid: 1.7, ..d.unison }, ..d };
        let text = preset_toml(&m);
        let back: PianoModel = toml::from_str(&text).unwrap();
        assert_eq!(back.a4, 442.0);
        assert_eq!(back.unison, m.unison);
        assert_eq!(back.soundboard, m.soundboard);
        let partial: PianoModel = toml::from_str("a4 = 415.0\n[damper]\nstrength = 0.5\n").unwrap();
        assert_eq!(partial.a4, 415.0);
        assert_eq!(partial.damper.strength, 0.5);
        assert_eq!(partial.damper.position, PianoModel::default().damper.position);
        assert_eq!(partial.inharmonicity, PianoModel::default().inharmonicity);
    }

    #[test]
    fn decodes_channel_messages() {
        assert_eq!(decode_midi(&[0x93, 60, 100]), Some(Event::NoteOn { note: 60, velocity: 100 }));
        assert_eq!(decode_midi(&[0x80, 60, 30]), Some(Event::NoteOff { note: 60, velocity: 30 }));
        assert_eq!(decode_midi(&[0xB0, 64, 127]), Some(Event::Control { cc: 64, value: 127 }));
        assert_eq!(decode_midi(&[0xF8]), None);
    }
}
