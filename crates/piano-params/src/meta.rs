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
    ("temperament.kind", "Temperament", "", 0.0, 8.0, false),
    ("temperament.key", "Temperament laid on", "", 0.0, 11.0, false),
    ("temperament.cents[0]", "Custom temperament: C", "c", -50.0, 50.0, false),
    ("temperament.cents[1]", "Custom temperament: C#", "c", -50.0, 50.0, false),
    ("temperament.cents[2]", "Custom temperament: D", "c", -50.0, 50.0, false),
    ("temperament.cents[3]", "Custom temperament: Eb", "c", -50.0, 50.0, false),
    ("temperament.cents[4]", "Custom temperament: E", "c", -50.0, 50.0, false),
    ("temperament.cents[5]", "Custom temperament: F", "c", -50.0, 50.0, false),
    ("temperament.cents[6]", "Custom temperament: F#", "c", -50.0, 50.0, false),
    ("temperament.cents[7]", "Custom temperament: G", "c", -50.0, 50.0, false),
    ("temperament.cents[8]", "Custom temperament: G#", "c", -50.0, 50.0, false),
    ("temperament.cents[9]", "Custom temperament: A", "c", -50.0, 50.0, false),
    ("temperament.cents[10]", "Custom temperament: Bb", "c", -50.0, 50.0, false),
    ("temperament.cents[11]", "Custom temperament: B", "c", -50.0, 50.0, false),
    ("output_gain_db", "Output gain", "dB", -40.0, 20.0, false),
    ("limiter", "Limiter", "", 0.0, 1.0, false),
    ("limiter_ceiling_db", "Limiter ceiling", "dBFS", -12.0, 0.0, false),
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
    ("macros.tone", "Tone (soft – bright)", "", -1.0, 1.0, false),
    ("macros.touch", "Touch (even – contrasted)", "", -1.0, 1.0, false),
    ("macros.sustain", "Sustain (short – long)", "", -1.0, 1.0, false),
    ("macros.resonance", "Resonance (dry – rich)", "", -1.0, 1.0, false),
    ("macros.release", "Note endings (clean – soft)", "", -1.0, 1.0, false),
    ("macros.unisons", "Unisons (pure – lively)", "", -1.0, 1.0, false),
    ("macros.mechanism", "Action noises (discreet – present)", "", -1.0, 1.0, false),
    ("macros.size", "Size (baby – concert grand)", "", -1.0, 1.0, false),
    ("macros.lid", "Lid (closed – open)", "", -1.0, 1.0, false),
    ("macros.space", "Space (close – roomy)", "", -1.0, 1.0, false),
    ("macros.stretch", "Tuning (strict – stretched)", "", -1.0, 1.0, false),
    ("damper.strength", "Damper strength", "", 0.01, 4.0, true),
    ("damper.strength_slope", "Damper strength falling up the keyboard", "", 0.0, 2.0, false),
    ("damper.horizontal", "Damper grip on sideways motion", "", 0.0, 1.0, false),
    ("damper.grip_rise", "Damper grip rising with frequency", "", 0.0, 1.0, false),
    ("damper.horizontal_high", "Sideways grip on the high partials", "", 0.0, 1.0, false),
    ("damper.node_floor_high", "Node floor of the high partials", "", 0.0, 1.0, false),
    ("damper.grip_corner", "Damper grip corner", "Hz", 50.0, 3000.0, true),
    ("damper.wedge_strength", "Bass wedge strength", "", 0.01, 4.0, true),
    ("damper.width_bass", "Pad length at A0 (0: as the others)", "m", 0.0, 0.15, false),
    ("damper.dodge", "Dampers set forward at the bass break", "", 0.0, 1.0, false),
    ("damper.dodge_from", "First damper set forward", "", 21.0, 108.0, false),
    ("damper.dodge_until", "Last damper set forward", "", 21.0, 108.0, false),
    ("damper.dodge_by", "How far forward", "m", 0.0, 0.04, false),
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
    ("noise.case", "Case knock", "Pa·s/m", 0.0, 1.0, false),
    ("noise.case_low", "Case knock lowest resonance", "Hz", 50.0, 150.0, false),
    ("noise.case_time", "Case knock T60", "s", 0.1, 2.0, true),
    ("body.gain", "Body halo level", "Pa·s/m", 0.0, 0.5, false),
    ("body.slope", "Body halo slope", "dB/oct", -12.0, 12.0, false),
    ("body.delay", "Body halo delay", "s", 0.0, 0.03, false),
    ("body.rise", "Body halo rise", "s", 0.0, 0.3, false),
    ("body.shared", "Body pattern shared by all keys", "", 0.0, 1.0, false),
    ("body.levels[0]", "Body halo level, 63 Hz", "dB", -40.0, 20.0, false),
    ("body.levels[1]", "Body halo level, 125 Hz", "dB", -40.0, 20.0, false),
    ("body.levels[2]", "Body halo level, 250 Hz", "dB", -40.0, 20.0, false),
    ("body.levels[3]", "Body halo level, 500 Hz", "dB", -40.0, 20.0, false),
    ("body.levels[4]", "Body halo level, 1 kHz", "dB", -40.0, 20.0, false),
    ("body.levels[5]", "Body halo level, 2 kHz", "dB", -40.0, 20.0, false),
    ("body.levels[6]", "Body halo level, 4 kHz", "dB", -40.0, 20.0, false),
    ("body.levels[7]", "Body halo level, 8 kHz", "dB", -40.0, 20.0, false),
    ("body.t60[0]", "Body halo T60, 63 Hz", "s", 0.02, 3.0, true),
    ("body.t60[1]", "Body halo T60, 125 Hz", "s", 0.02, 3.0, true),
    ("body.t60[2]", "Body halo T60, 250 Hz", "s", 0.02, 3.0, true),
    ("body.t60[3]", "Body halo T60, 500 Hz", "s", 0.02, 3.0, true),
    ("body.t60[4]", "Body halo T60, 1 kHz", "s", 0.02, 3.0, true),
    ("body.t60[5]", "Body halo T60, 2 kHz", "s", 0.02, 3.0, true),
    ("body.t60[6]", "Body halo T60, 4 kHz", "s", 0.02, 3.0, true),
    ("body.t60[7]", "Body halo T60, 8 kHz", "s", 0.02, 3.0, true),
    ("body.speed_ref", "Body halo reference speed", "m/s", 0.0, 6.0, false),
    ("body.speed_corner", "Body halo speed corner", "Hz", 200.0, 8000.0, true),
    ("body.speed_halo[0]", "Share of the speed law in the halo, A0", "", 0.0, 1.0, false),
    ("body.speed_halo[1]", "Share of the speed law in the halo, C3", "", 0.0, 1.0, false),
    ("body.speed_halo[2]", "Share of the speed law in the halo, C5", "", 0.0, 1.0, false),
    ("body.speed_halo[3]", "Share of the speed law in the halo, C7", "", 0.0, 1.0, false),
    ("body.speed_low[0]", "Body halo growth with speed, A0", "", -1.0, 4.0, false),
    ("body.speed_low[1]", "Body halo growth with speed, C3", "", -1.0, 4.0, false),
    ("body.speed_low[2]", "Body halo growth with speed, C5", "", -1.0, 4.0, false),
    ("body.speed_low[3]", "Body halo growth with speed, C7", "", -1.0, 4.0, false),
    ("body.speed_high[0]", "Body halo top growth with speed, A0", "", -1.0, 4.0, false),
    ("body.speed_high[1]", "Body halo top growth with speed, C3", "", -1.0, 4.0, false),
    ("body.speed_high[2]", "Body halo top growth with speed, C5", "", -1.0, 4.0, false),
    ("body.speed_high[3]", "Body halo top growth with speed, C7", "", -1.0, 4.0, false),
    // Sideways motion: the cross term and the bass's own values reach 0 and
    // the presets' settings (the inferred ranges, ×0.1–×10 the default's,
    // did not: the Concert grand's 0.6 tilt and 0.02 cross in the bass).
    ("polarization.h_coupling", "Sideways coupling", "", 0.0, 1.0, false),
    ("polarization.cross", "Cross coupling of the motions", "", 0.0, 1.0, false),
    ("polarization.radiation", "Sideways radiation", "", 0.0, 2.0, false),
    ("polarization.hammer_tilt", "Hammer tilt", "", 0.0, 1.0, false),
    ("polarization.smooth", "Sideways admittance smoothing", "", 0.0, 1.0, false),
    ("polarization.slope_from", "Sideways slope from (0: all partials)", "Hz", 0.0, 1000.0, false),
    ("polarization.slope_to", "Sideways slope up to (0: all partials)", "Hz", 0.0, 8000.0, false),
    ("polarization.bass_until", "Last key with the bass's sideways values", "", 21.0, 108.0, false),
    ("polarization.bass_fade", "Keys over which they blend", "", 1.0, 24.0, false),
    ("polarization.hammer_tilt_bass", "Hammer tilt, bass", "", 0.0, 1.0, false),
    ("polarization.radiation_bass", "Sideways radiation, bass", "", 0.0, 2.0, false),
    ("polarization.cross_bass", "Cross coupling, bass", "", 0.0, 1.0, false),
    ("losses.b2_high", "Overtone losses, top octaves (0: none)", "s", 0.0, 5e-7, false),
    ("hardness", "Felt hardness", "", 0.02, 2.0, true),
    ("hardness_slope", "Hardness slope (treble)", "/key", -0.05, 0.08, false),
    ("hardness_slope_bass", "Hardness slope (bass)", "/key", -0.05, 0.08, false),
    ("hardness_slope_top", "Hardness slope (top)", "/key", 0.0, 0.4, false),
    ("shank_share", "Shank share of hammer mass", "", 0.0, 0.6, false),
    ("shank_freq", "Shank frequency (top)", "Hz", 50.0, 1500.0, true),
    ("shank_freq_mid", "Shank frequency (middle)", "Hz", 50.0, 1500.0, true),
    ("core_hardness", "Inner felt hardness", "", 0.0, 100.0, false),
    ("core_p", "Inner felt exponent", "", 1.5, 10.0, false),
    ("core_depth_bass", "Top felt depth (bass)", "mm", 0.0, 3.0, false),
    ("core_depth_mid", "Top felt depth (middle)", "mm", 0.0, 3.0, false),
    ("core_depth_treble", "Top felt depth (treble)", "mm", 0.0, 3.0, false),
    ("p_bass", "Extra felt exponent (bass)", "", -2.0, 2.0, false),
    ("p_offset", "Felt exponent offset", "", -1.5, 1.5, false),
    ("hysteresis", "Felt hysteresis", "", 0.0, 3.0, false),
    ("contact_width_scale", "Felt contact width", "", 0.25, 4.0, true),
    ("unison.height_spread", "String height spread", "m", 0.0, 0.0005, false),
    ("unison.strike_spread", "Strike point spread across a unison", "", 0.0, 0.002, false),
    ("unison.b_spread", "Inharmonicity spread across a unison", "", 0.0, 0.2, false),
    ("unison.load_ramp", "Unison tension ramp", "keys", 0.0, 36.0, false),
    ("losses.b1_floor", "Wound-string loss floor", "1/s", 0.0, 1.0, false),
    ("losses.b2_top", "Top strings' overtone loss", "s", 0.0, 6.0e-7, false),
    ("losses.physical", "Physical string losses", "", 0.0, 1.0, false),
    ("losses.air_scale", "Air friction on the strings", "", 0.0, 4.0, false),
    ("losses.delta_ve", "Steel's bending loss angle", "", 0.0, 0.02, false),
    ("losses.q_te", "Steel's constant loss factor", "", 0.0, 1.0e-3, false),
    ("soundboard.impedance_slope", "Board impedance slope (treble)", "/key", -0.05, 0.1, false),
    ("soundboard.bridge_yield", "Bridge yield at high frequency (fitted)", "", 0.0, 1.0, false),
    ("soundboard.forced_radiation", "Board radiation at the partials' own frequencies", "", 0.0, 1.0, false),
    ("soundboard.yield_conductance", "Bridge yield conductance", "s/kg", 0.0, 1.0e-2, false),
    ("soundboard.yield_corner", "Bridge yield corner", "Hz", 500.0, 10000.0, true),
    ("soundboard.impedance_slope_bass", "Board impedance slope (bass)", "/key", -0.1, 0.1, false),
    // The rim's glued ledge: 0 lets the edge turn freely, the Concert grand's 1500
    // is fitted on a real D's board, 10000 is all but clamped.
    ("soundboard.structure.edge_spring", "Rim ledge stiffness", "N·m/rad/m", 0.0, 10000.0, false),
    ("hammer_mass_scale", "Hammer mass", "", 0.25, 2.0, true),
    ("strike_ratio", "Strike point", "", 0.06, 0.2, false),
    ("strike_ratio_spread", "Strike point spread", "", 0.0, 0.003, false),
    ("soundboard.mics_relative", "Positions relative to the board", "", 0.0, 1.0, false),
    ("soundboard.mic_gain_db", "Microphone set-up gain", "dB", -30.0, 20.0, false),
    ("soundboard.outputs_per_mic", "One output per microphone", "", 0.0, 1.0, false),
    ("room.directivity", "Piano directivity Q (0: from the piano)", "", 0.0, 8.0, false),
    ("room.size[0]", "Room length (x)", "m", 2.0, 60.0, false),
    ("room.size[1]", "Room width (y)", "m", 2.0, 60.0, false),
    ("room.size[2]", "Room height", "m", 2.0, 30.0, false),
    ("room.piano[0]", "Piano position x", "m", 0.0, 60.0, false),
    ("room.piano[1]", "Piano position y", "m", 0.0, 60.0, false),
    ("room.piano[2]", "Board height", "m", 0.3, 2.0, false),
    ("soundboard.main_string_slant", "Main strings' slant", "°", 0.0, 12.0, false),
    // The sections' lowest keys are MIDI notes kept as floats (pairs with
    // their gaps): 22–108 start a section, anything lower is none.
    ("soundboard.string_sections[0][0]", "2nd section's lowest key (below 22: none)", "", 0.0, 108.0, false),
    ("soundboard.string_sections[0][1]", "Gap below the 2nd section", "m", 0.0, 0.03, false),
    ("soundboard.string_sections[1][0]", "3rd section's lowest key (below 22: none)", "", 0.0, 108.0, false),
    ("soundboard.string_sections[1][1]", "Gap below the 3rd section", "m", 0.0, 0.03, false),
    ("soundboard.string_sections[2][0]", "4th section's lowest key (below 22: none)", "", 0.0, 108.0, false),
    ("soundboard.string_sections[2][1]", "Gap below the 4th section", "m", 0.0, 0.03, false),
    ("soundboard.string_sections[3][0]", "5th section's lowest key (below 22: none)", "", 0.0, 108.0, false),
    ("soundboard.string_sections[3][1]", "Gap below the 5th section", "m", 0.0, 0.03, false),
    ("soundboard.lid_angle", "Lid angle (0: shut)", "°", 0.0, 60.0, false),
    ("soundboard.lid_removed", "Lid removed", "", 0.0, 1.0, false),
    ("frame.capo_bar", "Capo bar in the treble", "", 0.0, 1.0, false),
    ("frame.capo_from", "Lowest key under the capo bar", "", 21.0, 108.0, false),
    ("sympathetic.rear", "Rear duplex sounds", "", 0.0, 1.0, false),
    ("sympathetic.rear_from", "Lowest key with a bare rear length", "", 21.0, 108.0, false),
    ("sympathetic.rear_loss", "Rear duplex decay", "1/s", 0.0, 60.0, false),
    ("sympathetic.bridge_cut", "Board's top modes only radiate (less CPU)", "", 0.0, 1.0, false),
    ("sympathetic.bridge_below", "Where the board's modes stop moving the bridge", "Hz", 3000.0, 22000.0, true),
    ("frame.duplex", "Front duplex sounds", "", 0.0, 1.0, false),
    ("frame.duplex_group", "Unisons to a bearing bar", "", 2.0, 12.0, false),
    ("frame.duplex_far", "Bearing bar to capo, a group's lowest key", "m", 0.015, 0.12, false),
    ("frame.duplex_near", "Bearing bar to capo, a group's highest key", "m", 0.015, 0.12, false),
    ("frame.duplex_loss", "Front duplex decay", "1/s", 0.0, 60.0, false),
    ("soundboard.lid_hinge_height", "Lid hinge height", "m", 0.05, 0.6, false),
    ("soundboard.lid_reflection", "Lid reflection", "", 0.0, 1.0, false),
    ("soundboard.lid_transmission", "Sound through the lid", "", 0.0, 1.0, false),
    ("soundboard.lid_leak", "Sound out of the shut lid", "", 0.0, 1.0, false),
    ("nonlinear.coupling_plain", "Phantom loudness, plain strings (0: as the wound)", "", 0.0, 1.0, false),
];

impl Meta {
    pub(crate) fn of(path: &str, default: Value) -> Self {
        if let Some(&(_, label, unit, min, max, log)) = CURATED.iter().find(|c| c.0 == path) {
            return Self { label: label.into(), unit, min, max, log, curated: true };
        }
        if let Some(m) = bridge_key(path).or_else(|| mic(path)) {
            return m;
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

/// The measured bridge points (`soundboard.bridge_keys[i][e]`, triples of
/// floats with a default of 0): a key over the keyboard, 21–108 placing a
/// point and anything lower none, then the point as shares of the board's
/// length and width.
fn bridge_key(path: &str) -> Option<Meta> {
    let (i, e) = path.strip_prefix("soundboard.bridge_keys[")?.strip_suffix(']')?.split_once("][")?;
    let n = i.parse::<usize>().ok()? + 1;
    let (label, max) = match e {
        "0" => (format!("Measured bridge point {n}: key (below 21: none)"), 108.0),
        "1" => (format!("Measured bridge point {n}: along the board"), 1.0),
        "2" => (format!("Measured bridge point {n}: across the board"), 1.0),
        _ => return None,
    };
    Some(Meta { label, unit: "", min: 0.0, max, log: false, curated: true })
}

/// A microphone's fields (`soundboard.mics[i].field` or `.field[j]`).
fn mic(path: &str) -> Option<Meta> {
    let (i, rest) = path.strip_prefix("soundboard.mics[")?.split_once("].")?;
    let n = i.parse::<usize>().ok()? + 1;
    let (field, j) = match rest.split_once('[') {
        Some((f, j)) => (f, j.strip_suffix(']')?.parse::<usize>().ok()),
        None => (rest, None),
    };
    let axis = |grand: [&'static str; 3], upright: bool| -> Option<&'static str> {
        let names = if upright { ["down", "across", "in front"] } else { grand };
        names.get(j?).copied()
    };
    let (label, unit, min, max) = match (field, j) {
        ("at" | "aim", Some(2)) => (format!("Mic {n} {}", if field == "at" { "height" } else { "aim: height" }), "m", -2.0, 8.0),
        ("at" | "aim", Some(_)) => (format!("Mic {n} {}{}", if field == "aim" { "aim: " } else { "" }, axis(["along", "across", ""], false)?), "", -8.0, 8.0),
        ("upright" | "aim_upright", Some(2)) => (format!("Upright: mic {n} in front"), "m", -1.0, 8.0),
        ("upright" | "aim_upright", Some(_)) => (format!("Upright: mic {n} {}", axis(["", "", ""], true)?), "", -8.0, 8.0),
        ("pattern", None) => (format!("Mic {n} pattern (1 omni, 0.5 cardioid, 0 figure of eight)"), "", 0.0, 1.0),
        ("gain_db", None) => (format!("Mic {n} gain"), "dB", -40.0, 40.0),
        ("pan", None) => (format!("Mic {n} pan (left – right)"), "", -1.0, 1.0),
        _ => return None,
    };
    Some(Meta { label, unit, min, max, log: false, curated: true })
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

    /// The shipped instruments' own settings are reachable with the sliders
    /// and a learned MIDI control.
    #[test]
    fn shipped_presets_lie_within_the_ranges() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../presets/instruments");
        let reg = crate::Registry::new();
        let mut out = Vec::new();
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            for file in std::fs::read_dir(entry.path()).unwrap().flatten() {
                let path = file.path();
                if path.extension().and_then(|e| e.to_str()) != Some("toml") {
                    continue;
                }
                let model: piano_core::params::PianoModel = toml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
                for (p, v) in reg.params().iter().zip(reg.read(&model)) {
                    let x = match v {
                        Some(crate::Value::Float(x)) => x,
                        Some(crate::Value::Int(i)) => i as f64,
                        _ => continue,
                    };
                    let checked = p.path.starts_with("polarization.") || p.path == "soundboard.structure.edge_spring";
                    if checked && (x < p.meta.min || x > p.meta.max) {
                        out.push(format!("{}: {} = {x} outside {}–{}", path.display(), p.path, p.meta.min, p.meta.max));
                    }
                }
            }
        }
        assert!(out.is_empty(), "{out:#?}");
    }

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

    /// The sections' lowest keys can be set over the keyboard with the
    /// standard controls, and their gaps in millimetres.
    #[test]
    fn the_sections_have_note_and_gap_ranges() {
        let reg = crate::Registry::new();
        for i in 0..4 {
            let key = &reg.get(&format!("soundboard.string_sections[{i}][0]")).unwrap().meta;
            assert!(key.min <= 22.0 && key.max == 108.0 && !key.log);
            let gap = &reg.get(&format!("soundboard.string_sections[{i}][1]")).unwrap().meta;
            assert!(gap.min == 0.0 && gap.max <= 0.05 && gap.unit == "m");
        }
    }

    /// The measured bridge points' keys can be set over the keyboard with
    /// the standard controls and MIDI learn, 0 still meaning none, and
    /// their points over the board.
    #[test]
    fn the_measured_bridge_points_have_key_and_share_ranges() {
        let reg = crate::Registry::new();
        let mut map = crate::midi::MidiMap::default();
        for i in 0..piano_core::soundboard::BRIDGE_KEYS {
            let path = format!("soundboard.bridge_keys[{i}][0]");
            let key = &reg.get(&path).unwrap_or_else(|| panic!("{path} is not a parameter")).meta;
            assert!(key.curated && key.min == 0.0 && key.max == 108.0 && !key.log, "{key:?}");
            map.assign(&reg, 1, &path);
            let m = map.for_path(&path).unwrap();
            let on = (0..=127u8).filter(|&cc| (21.0..=108.0).contains(&crate::midi::scale(m.min, m.max, false, cc))).count();
            assert!(on > 100, "{path}: {on} CC values place a point");
            for e in [1, 2] {
                let share = &reg.get(&format!("soundboard.bridge_keys[{i}][{e}]")).unwrap().meta;
                assert!(share.curated && share.min == 0.0 && share.max == 1.0, "{share:?}");
            }
        }
        assert!(reg.get(&format!("soundboard.bridge_keys[{}][0]", piano_core::soundboard::BRIDGE_KEYS)).is_none());
    }

    /// With a default of 0 the guessed range was −1 to 1: half of a MIDI
    /// controller's travel kept `coupling`, and step 64 dropped to 0.008.
    /// From 0 to 1, only step 0 keeps `coupling` on the plain strings.
    #[test]
    fn the_plain_strings_coupling_is_a_share() {
        let reg = crate::Registry::new();
        let path = "nonlinear.coupling_plain";
        let meta = &reg.get(path).unwrap().meta;
        assert!(meta.curated && meta.min == 0.0 && meta.max == 1.0 && !meta.log, "{meta:?}");
        let mut map = crate::midi::MidiMap::default();
        map.assign(&reg, 1, path);
        let m = map.for_path(path).unwrap();
        let set = (0..=127u8).filter(|&cc| crate::midi::scale(m.min, m.max, false, cc) > 0.0).count();
        assert_eq!(set, 127);
    }

    #[test]
    fn labels() {
        assert_eq!(label("soundboard.treble_bridge[2][1]"), "Treble bridge [2][1]");
        assert_eq!(label("damper.fall_time"), "Fall time");
        assert_eq!(label("nonlinear.max_long_hz"), "Max long");
    }
}
