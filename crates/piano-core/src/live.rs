//! Parameters the engine applies while playing, without rebuilding the
//! instrument: those read at event time or per block, never by
//! [`crate::instrument::Instrument::new`]. Every other [`PianoModel`] field
//! shapes the precomputed modes and needs a new instrument.
//!
//! A live change is an [`crate::Event::Param`] in the event stream, so the
//! interface, MIDI controllers and host automation reach the engine through
//! the same sample-accurate path.

use crate::params::PianoModel;

macro_rules! live_params {
    ($($name:ident => $path:literal, $($field:ident).+;)*) => {
        /// A parameter changeable while playing; its [`Self::path`] is the
        /// field path in the preset (TOML) and the parameter's stable name.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum LiveParam {
            $($name,)*
        }

        impl LiveParam {
            pub const ALL: &[LiveParam] = &[$(LiveParam::$name,)*];

            /// Field path in [`PianoModel`], as in the preset files.
            pub fn path(self) -> &'static str {
                match self {
                    $(LiveParam::$name => $path,)*
                }
            }

            pub fn from_path(path: &str) -> Option<Self> {
                match path {
                    $($path => Some(LiveParam::$name),)*
                    _ => None,
                }
            }

            pub fn get(self, model: &PianoModel) -> f64 {
                match self {
                    $(LiveParam::$name => model.$($field).+ as f64,)*
                }
            }

            pub fn set(self, model: &mut PianoModel, value: f64) {
                match self {
                    $(LiveParam::$name => model.$($field).+ = value,)*
                }
            }
        }
    };
}

live_params! {
    OutputGainDb => "output_gain_db", output_gain_db;
    Limiter => "limiter", limiter;
    LimiterCeilingDb => "limiter_ceiling_db", limiter_ceiling_db;
    UnaCordaSoftening => "una_corda_softening", una_corda_softening;
    UnaCordaShift => "una_corda_shift", una_corda_shift;
    HalfBlow => "half_blow", half_blow;
    MuteFelt => "mute_felt", mute_felt;
    RepetitionTime => "repetition_time", repetition_time;
    RepetitionFloor => "repetition_floor", repetition_floor;
    VelocityMin => "velocity.vmin", velocity.vmin;
    VelocityMax => "velocity.vmax", velocity.vmax;
    DamperStrength => "damper.strength", damper.strength;
    DamperContactExponent => "damper.contact_exponent", damper.contact_exponent;
    DamperPedalFree => "damper.pedal_free", damper.pedal_free;
    DamperPedalContact => "damper.pedal_contact", damper.pedal_contact;
    DamperPedalCurve => "damper.pedal_curve", damper.pedal_curve;
    DamperFallTime => "damper.fall_time", damper.fall_time;
    DamperLiftTime => "damper.lift_time", damper.lift_time;
    NoiseKeybed => "noise.keybed", noise.keybed;
    NoiseDamperMass => "noise.damper_mass", noise.damper_mass;
    NoiseDamperSpeed => "noise.damper_speed", noise.damper_speed;
    NoiseDamperLanding => "noise.damper_landing", noise.damper_landing;
    NoisePedal => "noise.pedal", noise.pedal;
    NoiseKnockTime => "noise.knock_time", noise.knock_time;
    NoiseAction => "noise.action", noise.action;
    NoiseActionTime => "noise.action_time", noise.action_time;
    NoiseActionDelay => "noise.action_delay", noise.action_delay;
    // The musician's controls that move live parameters only (crate::macros).
    MacroTouch => "macros.touch", macros.touch;
    MacroRelease => "macros.release", macros.release;
    MacroMechanism => "macros.mechanism", macros.mechanism;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_round_trip() {
        let mut m = PianoModel::default();
        for &p in LiveParam::ALL {
            assert_eq!(LiveParam::from_path(p.path()), Some(p));
            let v = p.get(&m) * 1.5 + 0.25;
            p.set(&mut m, v);
            assert_eq!(p.get(&m), v);
        }
    }
}
