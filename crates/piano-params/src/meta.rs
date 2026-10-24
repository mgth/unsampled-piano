//! Display metadata: label, unit and range. Curated for the parameters a
//! player touches; the others get a range around their default value,
//! marked as guessed.

use crate::Value;

#[derive(Clone, Debug, PartialEq)]
pub struct Meta {
    pub label: String,
    pub unit: &'static str,
    pub min: f64,
    pub max: f64,
    /// Logarithmic slider (positive ranges spanning decades).
    pub log: bool,
    /// False when the range is derived from the default, not chosen.
    pub curated: bool,
}

/// (path, label, unit, min, max, log)
const CURATED: &[(&str, &str, &str, f64, f64, bool)] = &[
    ("a4", "A4 pitch", "Hz", 400.0, 470.0, false),
    ("output_gain_db", "Output gain", "dB", -40.0, 20.0, false),
    ("una_corda_softening", "Una corda softening", "", 0.0, 1.0, false),
    ("una_corda_shift", "Una corda shift (1 grand, 0 upright)", "", 0.0, 1.0, false),
    ("half_blow", "Soft pedal half blow (upright)", "", 0.0, 0.5, false),
    ("mute_felt", "Practice mute felt (0: sostenuto)", "mm", 0.0, 6.0, false),
    ("repetition_time", "Repetition reset time (0: grand)", "s", 0.0, 0.2, false),
    ("repetition_floor", "Immediate re-strike speed", "", 0.0, 1.0, false),
    ("soundboard.wall_distance", "Wall behind the board (0: none)", "m", 0.0, 1.0, false),
    ("soundboard.wall_reflection", "Wall reflection", "", 0.0, 1.0, false),
    ("soundboard.case_open", "Case openings (1: no case)", "", 0.0, 1.0, false),
    ("soundboard.case_panel_mass", "Case panel mass", "kg/m²", 1.0, 30.0, true),
    ("velocity.vmin", "Hammer speed at pp", "m/s", 0.05, 2.0, true),
    ("velocity.vmax", "Hammer speed at ff", "m/s", 1.0, 8.0, true),
    ("velocity.curve", "Velocity curve (soft end)", "", 0.3, 1.5, false),
    ("damper.strength", "Damper strength", "", 0.01, 4.0, true),
    ("damper.horizontal", "Damper grip on sideways motion", "", 0.0, 1.0, false),
    ("damper.contact_exponent", "Damper felt exponent", "", 1.0, 5.0, false),
    ("damper.pedal_free", "Pedal lost motion", "", 0.0, 0.8, false),
    ("damper.pedal_contact", "Pedal lift point", "", 0.1, 1.0, false),
    ("damper.pedal_curve", "Half-pedal curve", "", 0.3, 4.0, true),
    ("damper.fall_time", "Damper fall time", "s", 0.001, 0.2, true),
    ("damper.lift_time", "Damper lift time", "s", 0.001, 0.1, true),
    ("noise.keybed", "Keybed knock", "N·s/m", 0.0, 2.0, false),
    ("noise.damper_mass", "Damper felt mass", "kg", 0.0, 0.05, false),
    ("noise.damper_speed", "Damper fall speed", "m/s", 0.0, 1.0, false),
    ("noise.damper_landing", "Damper landing time", "s", 0.0005, 0.02, true),
    ("noise.pedal", "Pedal knock", "N", 0.0, 3.0, false),
    ("noise.knock_time", "Knock duration", "s", 0.0005, 0.02, true),
    ("noise.action", "Action knock", "Pa·s/m", 0.0, 0.2, false),
    ("noise.action_time", "Action knock T60", "s", 0.02, 1.0, true),
    ("noise.action_delay", "Action knock delay", "s", 0.0, 0.03, false),
    ("hardness", "Felt hardness", "", 0.02, 2.0, true),
    ("hardness_slope", "Hardness slope (treble)", "/key", -0.05, 0.08, false),
    ("hardness_slope_bass", "Hardness slope (bass)", "/key", -0.05, 0.08, false),
    ("hardness_slope_top", "Hardness slope (top)", "/key", 0.0, 0.4, false),
    ("shank_share", "Shank share of hammer mass", "", 0.0, 0.6, false),
    ("shank_freq", "Shank frequency", "Hz", 50.0, 1500.0, true),
    ("core_hardness", "Inner felt hardness", "", 0.0, 100.0, false),
    ("core_p", "Inner felt exponent", "", 1.5, 10.0, false),
    ("core_depth_bass", "Top felt depth (bass)", "mm", 0.0, 3.0, false),
    ("core_depth_mid", "Top felt depth (middle)", "mm", 0.0, 3.0, false),
    ("core_depth_treble", "Top felt depth (treble)", "mm", 0.0, 3.0, false),
    ("p_bass", "Extra felt exponent (bass)", "", 0.0, 2.0, false),
    ("p_offset", "Felt exponent offset", "", -1.5, 1.5, false),
    ("hysteresis", "Felt hysteresis", "", 0.0, 3.0, false),
    ("contact_width_scale", "Felt contact width", "", 0.25, 4.0, true),
    ("unison.height_spread", "String height spread", "m", 0.0, 0.0005, false),
    ("losses.b1_floor", "Wound-string loss floor", "1/s", 0.0, 1.0, false),
    ("soundboard.impedance_slope", "Board impedance slope (treble)", "/key", -0.05, 0.1, false),
    ("soundboard.impedance_slope_bass", "Board impedance slope (bass)", "/key", -0.1, 0.1, false),
    ("hammer_mass_scale", "Hammer mass", "", 0.25, 2.0, true),
    ("strike_ratio", "Strike point", "", 0.06, 0.2, false),
    ("soundboard.mics[0][0]", "Left mic x (along)", "", -3.0, 5.0, false),
    ("soundboard.mics[0][1]", "Left mic y (across)", "", -5.0, 5.0, false),
    ("soundboard.mics[0][2]", "Left mic height", "m", 0.05, 4.0, false),
    ("soundboard.mics[1][0]", "Right mic x (along)", "", -3.0, 5.0, false),
    ("soundboard.mics[1][1]", "Right mic y (across)", "", -5.0, 5.0, false),
    ("soundboard.mics[1][2]", "Right mic height", "m", 0.05, 4.0, false),
    ("soundboard.mics_upright[0][0]", "Upright: left mic x (down)", "", -3.0, 5.0, false),
    ("soundboard.mics_upright[0][1]", "Upright: left mic y (across)", "", -5.0, 5.0, false),
    ("soundboard.mics_upright[0][2]", "Upright: left mic in front", "m", 0.05, 4.0, false),
    ("soundboard.mics_upright[1][0]", "Upright: right mic x (down)", "", -3.0, 5.0, false),
    ("soundboard.mics_upright[1][1]", "Upright: right mic y (across)", "", -5.0, 5.0, false),
    ("soundboard.mics_upright[1][2]", "Upright: right mic in front", "m", 0.05, 4.0, false),
    ("soundboard.mics_relative", "Positions relative to the board", "", 0.0, 1.0, false),
    ("soundboard.mic_gain_db", "Microphone gain", "dB", -30.0, 20.0, false),
    ("soundboard.mic_pattern", "Mic pattern (1 omni, 0.5 cardioid)", "", 0.0, 1.0, false),
    ("room.directivity", "Piano directivity Q (0: from the piano)", "", 0.0, 8.0, false),
    ("room.size[0]", "Room length (x)", "m", 2.0, 60.0, false),
    ("room.size[1]", "Room width (y)", "m", 2.0, 60.0, false),
    ("room.size[2]", "Room height", "m", 2.0, 30.0, false),
    ("room.piano[0]", "Piano position x", "m", 0.0, 60.0, false),
    ("room.piano[1]", "Piano position y", "m", 0.0, 60.0, false),
    ("room.piano[2]", "Board height", "m", 0.3, 2.0, false),
    ("soundboard.lid_angle", "Lid angle", "°", 0.0, 60.0, false),
    ("soundboard.lid_hinge_height", "Lid hinge height", "m", 0.05, 0.6, false),
    ("soundboard.lid_reflection", "Lid reflection", "", 0.0, 1.0, false),
    ("soundboard.lid_transmission", "Sound through the lid", "", 0.0, 1.0, false),
];

impl Meta {
    pub(crate) fn of(path: &str, default: Value) -> Self {
        if let Some(&(_, label, unit, min, max, log)) = CURATED.iter().find(|c| c.0 == path) {
            return Self { label: label.into(), unit, min, max, log, curated: true };
        }
        let name = path.rsplit('.').next().unwrap_or(path);
        let unit = guess_unit(name);
        let (min, max, log) = match default {
            Value::Bool(_) => (0.0, 1.0, false),
            Value::Int(_) if is_note(name) => (21.0, 108.0, false),
            Value::Int(d) => (0.0, (2 * d).max(8) as f64, false),
            Value::Float(d) if d > 0.0 => (d / 10.0, d * 10.0, true),
            Value::Float(d) if d < 0.0 => (3.0 * d, -3.0 * d, false),
            Value::Float(_) => (-1.0, 1.0, false),
        };
        Self { label: label(path), unit, min, max, log, curated: false }
    }
}

/// "soundboard.treble_bridge[2][1]" → "Treble bridge [2][1]".
fn label(path: &str) -> String {
    let name = path.rsplit('.').next().unwrap_or(path);
    let (word, index) = name.split_at(name.find('[').unwrap_or(name.len()));
    // The unit is shown with the value.
    let word = ["_hz", "_db", "_cents"].iter().find_map(|u| word.strip_suffix(u)).unwrap_or(word);
    let mut s = word.replace('_', " ");
    if let Some(c) = s.get(..1) {
        s.replace_range(..1, &c.to_uppercase());
    }
    if !index.is_empty() {
        s.push(' ');
        s.push_str(index);
    }
    s
}

fn is_note(name: &str) -> bool {
    name.contains("note") || name.ends_with("_from") || name.ends_with("_until")
}

fn guess_unit(name: &str) -> &'static str {
    let name = name.split('[').next().unwrap_or(name);
    if name.ends_with("_db") {
        "dB"
    } else if name.ends_with("_hz") || name.contains("freq") {
        "Hz"
    } else if name.ends_with("_time") {
        "s"
    } else if name.ends_with("_cents") {
        "cents"
    } else {
        ""
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curated_paths_exist() {
        let reg = crate::Registry::new();
        for c in CURATED {
            let p = reg.get(c.0).unwrap_or_else(|| panic!("{} is not a parameter", c.0));
            assert!(p.meta.curated);
            let d = p.default.as_f64();
            assert!(c.3 <= d && d <= c.4, "{}: default {d} outside [{}, {}]", c.0, c.3, c.4);
        }
    }

    #[test]
    fn labels() {
        assert_eq!(label("soundboard.treble_bridge[2][1]"), "Treble bridge [2][1]");
        assert_eq!(label("damper.fall_time"), "Fall time");
        assert_eq!(label("nonlinear.max_long_hz"), "Max long");
    }
}
