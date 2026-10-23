//! Host-side helpers shared by the renderer and the real-time player:
//! instrument presets (TOML), decoding of raw MIDI messages and reading of
//! standard MIDI files.

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

/// Fields of a listening set-up (microphones, their gain, the lid opening,
/// the room): what a listening preset sets, and what loading an instrument
/// keeps.
const LISTENING: &[(&str, &[&str])] = &[("soundboard", &["mics", "mics_relative", "mic_gain_db", "lid_angle"]), ("room", &[])];

fn to_table(model: &PianoModel) -> toml::Table {
    toml::Table::try_from(model).expect("model serialises")
}

/// Deep merge: every value of `top` replaces the one of `base`.
fn merge(base: &mut toml::Table, top: toml::Table) {
    for (k, v) in top {
        match (base.get_mut(&k), v) {
            (Some(toml::Value::Table(b)), toml::Value::Table(t)) => merge(b, t),
            (_, v) => {
                base.insert(k, v);
            }
        }
    }
}

/// `base` with the fields a preset text sets (the others keep their value).
pub fn overlay_preset(base: &PianoModel, text: &str) -> Result<PianoModel, String> {
    let top: toml::Table = text.parse().map_err(|e| format!("{e}"))?;
    let mut t = to_table(base);
    merge(&mut t, top);
    t.try_into().map_err(|e| format!("{e}"))
}

/// The listening part of a model (microphones, their gain, the room).
pub fn listening_part(model: &PianoModel) -> toml::Table {
    let t = to_table(model);
    let mut out = toml::Table::new();
    for (section, keys) in LISTENING {
        let Some(toml::Value::Table(s)) = t.get(*section) else { continue };
        let part: toml::Table = if keys.is_empty() {
            s.clone()
        } else {
            s.iter().filter(|(k, _)| keys.contains(&k.as_str())).map(|(k, v)| (k.clone(), v.clone())).collect()
        };
        out.insert(section.to_string(), toml::Value::Table(part));
    }
    out
}

/// `model` with the listening set-up of `current`.
pub fn with_listening(model: &PianoModel, current: &PianoModel) -> PianoModel {
    let mut t = to_table(model);
    merge(&mut t, listening_part(current));
    t.try_into().expect("a model with another model's listening fields is valid")
}

/// Loads an instrument preset on top of the default instrument, keeping
/// the current listening set-up.
pub fn load_instrument(path: &Path, current: &PianoModel) -> Result<PianoModel, String> {
    Ok(with_listening(&load_preset(path)?, current))
}

/// Applies a listening preset (microphones, their gain, the room) to the
/// current instrument.
pub fn load_listening(path: &Path, current: &PianoModel) -> Result<PianoModel, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    overlay_preset(current, &text).map_err(|e| format!("{}: {e}", path.display()))
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

/// Reads a standard MIDI file: all tracks and channels merged, tempo map
/// applied; returns (time in seconds, event), sorted by time.
pub fn read_midi_file(path: &Path) -> Result<Vec<(f64, Event)>, String> {
    use midly::{MetaMessage, MidiMessage, Smf, Timing, TrackEventKind};
    let data = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let smf = Smf::parse(&data).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut evs: Vec<(u64, usize, TrackEventKind)> = Vec::new();
    for (ti, track) in smf.tracks.iter().enumerate() {
        let mut tick = 0u64;
        for e in track {
            tick += e.delta.as_int() as u64;
            evs.push((tick, ti, e.kind));
        }
    }
    evs.sort_by_key(|e| (e.0, e.1));
    let (tpb, fixed_tick_s) = match smf.header.timing {
        Timing::Metrical(t) => (t.as_int() as f64, None),
        Timing::Timecode(fps, sub) => (1.0, Some(1.0 / (fps.as_f32() as f64 * sub as f64))),
    };
    let mut us_per_beat = 500_000.0;
    let (mut last_tick, mut time) = (0u64, 0.0f64);
    let mut out = Vec::new();
    for (tick, _, kind) in evs {
        let dt = (tick - last_tick) as f64;
        time += fixed_tick_s.unwrap_or(us_per_beat * 1e-6 / tpb) * dt;
        last_tick = tick;
        match kind {
            TrackEventKind::Meta(MetaMessage::Tempo(t)) => us_per_beat = t.as_int() as f64,
            TrackEventKind::Midi { message, .. } => {
                let ev = match message {
                    MidiMessage::NoteOn { key, vel } => Event::NoteOn { note: key.as_int(), velocity: vel.as_int() },
                    MidiMessage::NoteOff { key, vel } => Event::NoteOff { note: key.as_int(), velocity: vel.as_int() },
                    MidiMessage::Controller { controller, value } => {
                        Event::Control { cc: controller.as_int(), value: value.as_int() }
                    }
                    _ => continue,
                };
                out.push((time, ev));
            }
            _ => {}
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {

    #[test]
    fn listening_overlay_keeps_the_instrument() {
        let mut inst = PianoModel { hardness: 0.77, ..Default::default() };
        inst.scale.len_a0 = 2.0;
        let l = "[soundboard]\nmics = [[0.3, 0.3, 0.4], [0.7, 0.7, 0.4]]\nmics_relative = true\nmic_gain_db = -9.0\n[room]\nenabled = false\n";
        let m = overlay_preset(&inst, l).unwrap();
        assert_eq!((m.hardness, m.scale.len_a0), (0.77, 2.0));
        let sb = m.soundboard.unwrap();
        assert!(sb.mics_relative && sb.mic_gain_db == -9.0 && !m.room.enabled);
        // An instrument loaded afterwards keeps that listening set-up.
        let dir = std::env::temp_dir().join(format!("piano-host-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("i.toml");
        std::fs::write(&path, "hardness = 0.4\n[soundboard]\na = 2.0\nmics = [[1.0, 1.0, 2.0], [1.0, 1.0, 2.0]]\n").unwrap();
        let n = load_instrument(&path, &m).unwrap();
        let sb = n.soundboard.unwrap();
        assert_eq!((n.hardness, sb.a, sb.mics[0].0, sb.mic_gain_db), (0.4, 2.0, 0.3, -9.0));
        assert!(!n.room.enabled);
        assert_eq!(n.scale.len_a0, PianoModel::default().scale.len_a0);
        std::fs::remove_dir_all(dir).ok();
    }

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
