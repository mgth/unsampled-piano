//! Musician's controls: each moves a group of the model's parameters
//! together, so that one gesture gives one audible effect (brighter,
//! longer, more room…). A preset keeps its own (advanced) values; the
//! controls are offsets on top of them, from −1 to +1, 0 leaving the
//! preset as it is. [`PianoModel::with_macros`] applies them where the
//! model is turned into an instrument or sent to a playing engine.
//!
//! The spans keep to what real grands do (docs/macros.md): a control at
//! either end stays a plausible piano.

use crate::params::PianoModel;

/// Offsets in −1…+1 (0: the preset as it is).
#[derive(Clone, Copy, Debug, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
pub struct Macros {
    /// Soft ↔ bright: the felt's hardness, top felt and inner felt.
    pub tone: f64,
    /// Even ↔ contrasted: the spread of hammer speeds around mezzo forte.
    pub touch: f64,
    /// Short ↔ long: the strings' losses and the board's edge loss.
    pub sustain: f64,
    /// Dry ↔ rich: the strings answering each other (sympathy) and how
    /// many top strings have no damper.
    pub resonance: f64,
    /// Clean ↔ soft note endings: how hard and how fast the dampers stop
    /// a released note.
    pub release: f64,
    /// Pure ↔ lively unisons: how far apart the strings of a key are tuned
    /// and how unevenly the hammer meets them.
    pub unisons: f64,
    /// Discreet ↔ present action noises: the action's knock, the dampers'
    /// and the pedal's.
    pub mechanism: f64,
    /// Baby grand ↔ concert grand: string lengths, board and frame size,
    /// hammer mass.
    pub size: f64,
    /// Closed ↔ open lid.
    pub lid: f64,
    /// Close ↔ roomy: the level of the room's reverberation.
    pub space: f64,
    /// Strict ↔ stretched tuning: the octaves' stretch against equal
    /// temperament.
    pub stretch: f64,
}

impl Macros {
    pub const NONE: Self = Self {
        tone: 0.0,
        touch: 0.0,
        sustain: 0.0,
        resonance: 0.0,
        release: 0.0,
        unisons: 0.0,
        mechanism: 0.0,
        size: 0.0,
        lid: 0.0,
        space: 0.0,
        stretch: 0.0,
    };
}

/// Factor e^{ln(span)·v}: span at v = +1, 1/span at v = −1.
fn scale(span: f64, v: f64) -> f64 {
    span.powf(v)
}

// Spans at the ends of each control (calibrated in docs/macros.md).
/// Felt hardness factor (top felt and core).
const TONE_HARDNESS: f64 = 4.0;
const TONE_CORE: f64 = 12.0;
/// Log-distance of the hammer speeds from mezzo forte's.
const TOUCH_CONTRAST: f64 = 1.5;
/// String and edge losses.
const SUSTAIN_LOSS: f64 = 3.0;
/// Sympathetic gain, and top keys without damper added (+) or removed (−).
const RESONANCE_GAIN: f64 = 2.0;
const RESONANCE_KEYS: f64 = 4.0;
/// Damper strength (and the fall time, half as much in log).
const RELEASE_STRENGTH: f64 = 3.0;
/// Unison spread, hit unevenness.
const UNISON_WIDTH: f64 = 3.0;
const UNISON_PURE: f64 = 1.6;
const UNISON_HITS: f64 = 1.5;
/// Mechanical noise levels.
const MECHANISM_LEVEL: f64 = 4.0;
/// Bass string length, board and frame size, hammer mass.
const SIZE_BASS: f64 = 1.25;
const SIZE_BOARD: f64 = 1.12;
const SIZE_HAMMER: f64 = 1.1;
/// Lid angle at either end (degrees): closed, fully open.
const LID_MIN: f64 = 0.0;
const LID_MAX: f64 = 45.0;
/// Room tail level.
const SPACE_TAIL: f64 = 3.0;

impl PianoModel {
    /// The model with its musician's controls applied to the parameters
    /// they move, and the controls themselves back to 0 (applying twice
    /// changes nothing).
    pub fn with_macros(&self) -> PianoModel {
        let c = self.macros;
        if c == Macros::NONE {
            return *self;
        }
        let v = |x: f64| x.clamp(-1.0, 1.0);
        let mut m = *self;
        m.macros = Macros::NONE;

        // Tone: both layers of the felt, harder or softer together.
        let t = v(c.tone);
        m.hardness *= scale(TONE_HARDNESS, t);
        m.core_hardness *= scale(TONE_CORE, t);

        // Touch: hammer speeds spread further from (or closer to) the
        // mezzo forte's, which stays put: log v' = log v60 + g·(log v − log v60).
        let g = scale(TOUCH_CONTRAST, v(c.touch));
        let v60 = m.velocity.hammer_speed(60);
        m.velocity.vmin = v60 * (m.velocity.vmin / v60).powf(g);
        m.velocity.vmax = v60 * (m.velocity.vmax / v60).powf(g);

        // Sustain: the strings' own losses and the energy the bass leaves in
        // the rim, less for a longer sustain.
        let s = scale(SUSTAIN_LOSS, -v(c.sustain));
        m.losses.b1_offset += s.ln();
        m.losses.b1_floor *= s;
        m.losses.b2 *= s;
        if let Some(sb) = m.soundboard.as_mut() {
            sb.edge_loss *= s;
        }

        // Resonance: louder sympathy, and the dampers stopping lower or
        // higher up the keyboard.
        let r = v(c.resonance);
        m.sympathetic.gain *= scale(RESONANCE_GAIN, r);
        let top = m.damper.highest_note as f64 - (RESONANCE_KEYS * r).round();
        m.damper.highest_note = top.clamp(60.0, 108.0) as u8;

        // Release: a softer, slower felt lets the released note fade.
        let rl = v(c.release);
        m.damper.strength *= scale(RELEASE_STRENGTH, -rl);
        m.damper.fall_time *= scale(RELEASE_STRENGTH, 0.5 * rl);

        // Unisons: strings further apart, struck less evenly.
        // Purer only so far: much tighter unisons lose the slow modes that
        // carry the sustain (−12 dB at 1.5 s with a third of the spread).
        let u = v(c.unisons);
        let w = if u < 0.0 { scale(UNISON_PURE, u) } else { scale(UNISON_WIDTH, u) };
        m.unison.width_mid *= w;
        m.unison.width_treble *= w;
        m.unison.hit_spread *= scale(UNISON_HITS, u);

        // Mechanism: every mechanical noise together.
        let k = scale(MECHANISM_LEVEL, v(c.mechanism));
        m.noise.action *= k;
        m.noise.keybed *= k;
        m.noise.damper_mass *= k;
        m.noise.pedal *= k;

        // Size: a longer bass scale (less inharmonic), a larger board and
        // frame, heavier hammers.
        let z = v(c.size);
        let bass = scale(SIZE_BASS, z);
        m.scale.len_a0 *= bass;
        // B ∝ 1/L² for strings scaled alike.
        m.inharmonicity.b_bass -= 2.0 * bass.ln();
        let board = scale(SIZE_BOARD, z);
        if let Some(sb) = m.soundboard.as_mut() {
            sb.a *= board;
            sb.b *= board;
        }
        m.frame.a *= board;
        m.frame.b *= board;
        m.hammer_mass_scale *= scale(SIZE_HAMMER, z);

        // Lid: towards closed (−1) or fully open (+1) from the preset's angle.
        let l = v(c.lid);
        if let Some(sb) = m.soundboard.as_mut() {
            let end = if l < 0.0 { LID_MIN } else { LID_MAX };
            sb.lid_angle += (end - sb.lid_angle) * l.abs();
        }

        // Space: more or less of the room's reverberation.
        m.room.tail_gain *= scale(SPACE_TAIL, v(c.space));

        // Stretch: from equal temperament (−1) to twice the preset's
        // stretch (+1).
        let st = 1.0 + v(c.stretch);
        m.stretch.a_t *= st;
        m.stretch.a_b *= st;
        m.stretch.c *= st;
        m
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// PianoModel has no PartialEq (floats, Options): compare its Debug form.
    fn same(a: &PianoModel, b: &PianoModel) -> bool {
        format!("{a:?}") == format!("{b:?}")
    }

    #[test]
    fn neutral_controls_change_nothing_and_apply_once() {
        let m = PianoModel::default();
        assert!(same(&m.with_macros(), &m));
        let mut c = m;
        c.macros = Macros { tone: 0.7, touch: -0.4, sustain: 0.5, resonance: 1.0, release: -1.0, unisons: 0.3, mechanism: -0.6, size: 0.2, lid: 0.5, space: -0.8, stretch: -1.0 };
        let once = c.with_macros();
        assert_eq!(once.macros, Macros::NONE);
        assert!(same(&once.with_macros(), &once));
        // Out-of-range values are held at the ends.
        let mut far = c;
        far.macros.tone = 5.0;
        let mut end = c;
        end.macros.tone = 1.0;
        assert!(same(&far.with_macros(), &end.with_macros()));
    }

    #[test]
    fn touch_keeps_mezzo_forte_and_stretch_reaches_equal_temperament() {
        let mut m = PianoModel::default();
        m.macros.touch = 1.0;
        m.macros.stretch = -1.0;
        let r = m.with_macros();
        let v60 = PianoModel::default().velocity.hammer_speed(60);
        assert!((r.velocity.hammer_speed(60) - v60).abs() < 1e-9 * v60);
        assert!(r.velocity.hammer_speed(127) > PianoModel::default().velocity.hammer_speed(127));
        assert!(r.stretch.cents(108).abs() < 1e-9 && r.stretch.cents(21).abs() < 1e-9);
    }
}
