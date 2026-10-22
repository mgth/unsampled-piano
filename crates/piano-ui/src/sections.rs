//! Pages of the interface: which parameters each shows, and the per-key
//! curves that summarise its laws. A parameter goes to the first page whose
//! prefixes match its path; new model fields land in their section's page,
//! or in "Other" at the top level.

use piano_core::params::PianoModel;

pub struct Curve {
    pub title: &'static str,
    pub unit: &'static str,
    /// x range: MIDI notes (21–108) or velocities (1–127).
    pub x: (u8, u8),
    pub log: bool,
    pub f: fn(&PianoModel, u8) -> f64,
}

pub struct Section {
    pub title: &'static str,
    /// Exact paths or prefixes ending in '.', '_' or '['.
    pub prefixes: &'static [&'static str],
    pub curves: &'static [Curve],
    /// Drawing shown above the parameters.
    pub view: View,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum View {
    None,
    /// The microphones placed around the piano ([`crate::mics`]).
    Mics,
    /// The action, the hovered parameter's part lit ([`crate::sketches`]).
    Action,
}

const KEYS: (u8, u8) = (21, 108);

pub const SECTIONS: &[Section] = &[
    Section {
        title: "Player",
        prefixes: &["output_gain_db", "velocity.", "una_corda_softening", "damper.pedal_"],
        curves: &[Curve {
            title: "Hammer speed by velocity",
            unit: "m/s",
            x: (1, 127),
            log: true,
            f: |m, v| m.velocity.hammer_speed(v),
        }],
        view: View::Action,
    },
    Section {
        title: "Microphones",
        prefixes: &["soundboard.mics[", "soundboard.lid_"],
        curves: &[],
        view: View::Mics,
    },
    Section {
        title: "Hammer",
        prefixes: &[
            "hardness",
            "hardness_slope",
            "hardness_slope_bass",
            "core_",
            "p_bass",
            "p_offset",
            "hysteresis",
            "contact_width_scale",
        ],
        curves: &[
            Curve { title: "Felt stiffness Q0", unit: "N/mm^p", x: KEYS, log: true, f: |m, n| m.note(n).hammer.q0 },
            Curve { title: "Felt exponent p", unit: "", x: KEYS, log: false, f: |m, n| m.note(n).hammer.p },
        ],
        view: View::Action,
    },
    Section {
        title: "Tuning",
        prefixes: &["a4", "stretch.", "inharmonicity."],
        curves: &[
            Curve { title: "Stretch", unit: "cents", x: KEYS, log: false, f: |m, n| m.stretch.cents(n) },
            Curve { title: "Inharmonicity B", unit: "", x: KEYS, log: true, f: |m, n| m.inharmonicity.b(n) },
        ],
        view: View::None,
    },
    Section {
        title: "Strings",
        prefixes: &["losses.", "scale.", "unison.", "polarization.", "nonlinear."],
        curves: &[
            Curve {
                title: "Fundamental T60 (string losses alone)",
                unit: "s",
                x: KEYS,
                log: true,
                f: |m, n| 3.0 * std::f64::consts::LN_10 / m.losses.sigma(n, m.note(n).f0),
            },
            Curve { title: "String length", unit: "m", x: KEYS, log: true, f: |m, n| m.scale.length(n) },
            Curve { title: "Unison spread", unit: "cents", x: KEYS, log: false, f: |m, n| m.unison.width(n) },
        ],
        view: View::None,
    },
    Section { title: "Dampers", prefixes: &["damper.", "sympathetic."], curves: &[], view: View::Action },
    Section { title: "Noises", prefixes: &["noise."], curves: &[], view: View::Action },
    Section { title: "Soundboard", prefixes: &["soundboard.", "bridge."], curves: &[], view: View::None },
    Section { title: "Frame & rim", prefixes: &["frame.", "rim."], curves: &[], view: View::None },
    Section { title: "Room", prefixes: &["room."], curves: &[], view: View::None },
    Section { title: "Other", prefixes: &[""], curves: &[], view: View::None },
];

pub fn section_of(path: &str) -> usize {
    SECTIONS
        .iter()
        .position(|s| {
            s.prefixes.iter().any(|p| if p.is_empty() || p.ends_with(['.', '_', '[']) { path.starts_with(p) } else { path == *p })
        })
        .unwrap_or(SECTIONS.len() - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placement() {
        assert_eq!(SECTIONS[section_of("damper.pedal_free")].title, "Player");
        assert_eq!(SECTIONS[section_of("damper.fall_time")].title, "Dampers");
        assert_eq!(SECTIONS[section_of("hardness_slope")].title, "Hammer");
        assert_eq!(SECTIONS[section_of("core_p")].title, "Hammer");
        assert_eq!(SECTIONS[section_of("room.size[1]")].title, "Room");
        assert_eq!(SECTIONS[section_of("a4")].title, "Tuning");
        assert_eq!(SECTIONS[section_of("soundboard.mics[1][2]")].title, "Microphones");
        assert_eq!(SECTIONS[section_of("soundboard.lid_angle")].title, "Microphones");
        assert_eq!(SECTIONS[section_of("soundboard.back_gain")].title, "Soundboard");
        assert_eq!(SECTIONS[section_of("some_new_field")].title, "Other");
    }

    #[test]
    fn curves_are_finite() {
        let m = PianoModel::default();
        for s in SECTIONS {
            for c in s.curves {
                for x in c.x.0..=c.x.1 {
                    let y = (c.f)(&m, x);
                    assert!(y.is_finite() && (!c.log || y > 0.0), "{} at {x}: {y}", c.title);
                }
            }
        }
    }
}
