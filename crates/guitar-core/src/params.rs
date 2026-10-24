//! Physical description of the guitar, in SI units unless named otherwise.
//! A preset is this struct's serde form.

#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
pub struct StringParams {
    /// MIDI note of the open string.
    pub open_note: u8,
    /// Tension at pitch (N). With the scale length and the open pitch it
    /// gives the linear density: μ = T/(2Lf₀)².
    pub tension: f64,
    /// Outer diameter (m): air damping.
    pub diameter: f64,
    /// Diameter of the core that carries the bending stiffness (m): the
    /// whole string when plain, the core wire when wound.
    pub core_diameter: f64,
    /// Young's modulus of the core (Pa).
    pub young: f64,
    /// Viscoelastic loss factor of the bending (Valette & Cuesta's δ_ve):
    /// the mode loses δ_ve times its share of bending energy per radian.
    pub delta_ve: f64,
    /// Frequency-independent loss factor (internal friction under tension,
    /// friction between the winding and the core).
    pub eta_int: f64,
}

impl StringParams {
    /// Plain steel string.
    pub const fn plain(open_note: u8, tension: f64, diameter: f64) -> Self {
        Self { open_note, tension, diameter, core_diameter: diameter, young: 2.0e11, delta_ve: 2.0e-3, eta_int: 2.0e-5 }
    }

    /// Bronze-wound string on a steel core.
    pub const fn wound(open_note: u8, tension: f64, diameter: f64, core_diameter: f64) -> Self {
        Self { open_note, tension, diameter, core_diameter, young: 2.0e11, delta_ve: 2.0e-3, eta_int: 6.0e-5 }
    }
}

impl Default for StringParams {
    fn default() -> Self {
        Self::plain(64, 100.0, 0.30e-3)
    }
}

/// Admittance (velocity/force, s/kg) of one direction of the bridge, for
/// e^{iωt}: conductance (its real part, which takes energy from the string)
/// and susceptance (imaginary part: > 0 spring-like, which lowers the
/// partials; < 0 mass-like, which raises them).
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
pub struct Admittance {
    pub conductance: f64,
    /// Growth of the conductance per kHz. Stand-in for the rising density
    /// of body modes seen by the bridge (the average trend of measured
    /// guitar bridge mobilities) until the body gives Y(ω) itself.
    pub conductance_per_khz: f64,
    pub susceptance: f64,
}

impl Admittance {
    pub const ZERO: Self = Self { conductance: 0.0, conductance_per_khz: 0.0, susceptance: 0.0 };
}

impl Default for Admittance {
    fn default() -> Self {
        Self::ZERO
    }
}

/// What the strings are tied to at the bridge, in the two directions of
/// their motion: perpendicular to the top (which moves it most) and
/// parallel to it (which hardly does), and the coupling of the two through
/// the saddle's rocking.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
pub struct SupportParams {
    pub perpendicular: Admittance,
    pub parallel: Admittance,
    pub cross: Admittance,
}

impl Default for SupportParams {
    /// Orders of magnitude of an acoustic steel-string bridge: parallel an
    /// order of magnitude stiffer than perpendicular. To be fitted.
    fn default() -> Self {
        Self {
            perpendicular: Admittance { conductance: 3.0e-3, conductance_per_khz: 2.0e-2, susceptance: 3.0e-3 },
            parallel: Admittance { conductance: 3.0e-4, conductance_per_khz: 2.0e-3, susceptance: 0.0 },
            cross: Admittance { conductance: 5.0e-4, conductance_per_khz: 0.0, susceptance: 0.0 },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PluckModel {
    /// The string released at rest from a triangle, its previous motion
    /// replaced.
    Ideal,
    /// A fingertip lands on the string (stopping what it does), drags it
    /// through a spring–dashpot contact, and slips off once its force
    /// reaches the pluck's.
    Finger,
    /// The same with a plectrum: stiffer, and it slips off faster.
    Plectrum,
}

/// Where and how the right hand plucks.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
pub struct PluckParams {
    pub model: PluckModel,
    /// Distance from the bridge (m). It stays put when the left hand frets:
    /// the relative position grows with the fret.
    pub position: f64,
    /// Direction of the pull, from the perpendicular to the top (rad): 0
    /// pushes toward the top, π/2 pulls along it.
    pub angle: f64,
    /// Width of the contact (m) of the ideal pluck: a fingertip rounds the
    /// corner of the plucked shape, a plectrum hardly.
    pub width: f64,
    /// Force on the string when it slips off, at velocity 127 (N);
    /// proportional below (placeholder law). The released displacement is
    /// F/k with k = T·L/(x₀(L − x₀)) the string's stiffness at the pluck point.
    pub max_force: f64,
    /// Stiffness (N/m), damping (N·s/m) and slip time (s) of the fingertip.
    pub finger_stiffness: f64,
    pub finger_damping: f64,
    pub finger_release: f64,
    /// The same for the plectrum: its bending stiffness, and a quicker slip
    /// off its edge.
    pub plectrum_stiffness: f64,
    pub plectrum_damping: f64,
    pub plectrum_release: f64,
    /// Time the hand takes to drag the string out (s).
    pub drag_time: f64,
}

impl Default for PluckParams {
    /// Slip times and contact stiffnesses are orders of magnitude (flesh
    /// slipping in ≈ 0.5 ms, a plectrum in ≈ 0.05 ms), to be measured.
    fn default() -> Self {
        Self {
            model: PluckModel::Finger,
            position: 0.12,
            angle: std::f64::consts::FRAC_PI_4,
            width: 8.0e-3,
            max_force: 3.0,
            finger_stiffness: 5.0e3,
            finger_damping: 5.0,
            finger_release: 4.0e-4,
            plectrum_stiffness: 2.0e4,
            plectrum_damping: 0.5,
            plectrum_release: 5.0e-5,
            drag_time: 0.02,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PickupKind {
    SingleCoil,
    /// Two coils side by side along the string, in series, wound and
    /// magnetised in opposite senses: hum cancels, the string's signals add.
    Humbucker,
}

/// A magnetic pickup: the string's velocity toward its poles, seen over the
/// window of its magnets, makes the EMF of a coil (L, R, C).
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
pub struct Pickup {
    pub kind: PickupKind,
    /// Centre of the pickup from the bridge (m).
    pub position: f64,
    /// Width of each coil's window along the string (m).
    pub aperture: f64,
    /// Centre-to-centre distance of a humbucker's coils (m).
    pub coil_spacing: f64,
    /// EMF per string velocity toward the poles (V per m/s).
    pub sensitivity: f64,
    /// Inductance (H), resistance (Ω) and self-capacitance (F) of the coil(s).
    pub inductance: f64,
    pub resistance: f64,
    pub capacitance: f64,
}

impl Pickup {
    /// Vintage-style single coil (≈ 2.5 H, 6 kΩ).
    pub const fn single_coil(position: f64) -> Self {
        Self {
            kind: PickupKind::SingleCoil,
            position,
            aperture: 0.012,
            coil_spacing: 0.0,
            sensitivity: 0.3,
            inductance: 2.5,
            resistance: 6.0e3,
            capacitance: 100e-12,
        }
    }

    /// Classic humbucker (≈ 4.5 H, 8.5 kΩ).
    pub const fn humbucker(position: f64) -> Self {
        Self {
            kind: PickupKind::Humbucker,
            position,
            aperture: 0.012,
            coil_spacing: 0.018,
            sensitivity: 0.6,
            inductance: 4.5,
            resistance: 8.5e3,
            capacitance: 150e-12,
        }
    }
}

impl Default for Pickup {
    fn default() -> Self {
        Self::single_coil(0.1)
    }
}

/// The guitar's passive circuit and what the jack feeds.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
pub struct Circuit {
    /// Volume and tone pots (Ω), audio taper; tone capacitor (F).
    pub volume_pot: f64,
    pub tone_pot: f64,
    pub tone_cap: f64,
    /// Cable capacitance (F) and the input resistance it feeds (Ω).
    pub cable_capacitance: f64,
    pub load_resistance: f64,
}

impl Default for Circuit {
    fn default() -> Self {
        Self { volume_pot: 250e3, tone_pot: 250e3, tone_cap: 47e-9, cable_capacitance: 500e-12, load_resistance: 1e6 }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
pub struct MagneticOutput {
    pub pickups: Vec<Pickup>,
    /// Pickups switched in by the selector (in parallel).
    pub selected: Vec<usize>,
    pub circuit: Circuit,
    /// Volume and tone knobs, 0 … 1 (1: full volume, tone wide open).
    pub volume: f64,
    pub tone: f64,
}

impl Default for MagneticOutput {
    fn default() -> Self {
        Self { pickups: vec![Pickup::default()], selected: vec![0], circuit: Circuit::default(), volume: 1.0, tone: 1.0 }
    }
}

/// What the instrument's output is.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Output {
    /// The perpendicular force on the bridge, times `gain` (per N). Stand-in
    /// for an acoustic instrument until its body radiates (G6).
    BridgeForce { gain: f64 },
    /// A piezo under the saddle: the perpendicular force (V/N), high-passed
    /// by the element's capacitance into its preamp.
    Piezo { sensitivity: f64, highpass_hz: f64 },
    /// Magnetic pickups through the guitar's circuit (V).
    Magnetic(MagneticOutput),
}

impl Default for Output {
    fn default() -> Self {
        Output::BridgeForce { gain: 0.1 }
    }
}

/// What stops a string at note-off: the fretting finger lifting off (or the
/// hand laid on an open string), as a dashpot spread over the fingertip.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
pub struct MuteParams {
    /// Distance of the fingertip's centre from the fret it presses behind
    /// (on the nut side), or from the nut on an open string (m).
    pub distance: f64,
    /// Width of the fingertip on the string (m).
    pub width: f64,
    /// Damping coefficient of the flesh (N·s/m).
    pub resistance: f64,
}

impl Default for MuteParams {
    fn default() -> Self {
        // Chosen so that a muted low E falls 60 dB in ≈ 0.35 s (σ ≈ 20 /s on
        // its fundamental; lighter strings stop faster); to be measured.
        Self { distance: 0.01, width: 0.012, resistance: 60.0 }
    }
}

/// The fingerboard under the strings: what they can hit.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
pub struct FretboardParams {
    /// Height of the fret crowns above the board (m).
    pub fret_height: f64,
    /// Neck relief: the board's sag midway between the nut and the fret
    /// where the neck meets the body (m); flat beyond that fret.
    pub relief: f64,
    pub body_fret: u8,
    /// String height above the 12th fret's crown, open string, on the
    /// lowest and the highest string (m); in between, interpolated.
    pub action_bass: f64,
    pub action_treble: f64,
    /// String height above the first fret's crown at the nut (m).
    pub nut_clearance: f64,
    /// String–fret contact: K·u^α (N, m) and a dashpot (N·s/m) while
    /// touching. Stiff, as the modal string resolves (Issanchou et al. use
    /// the same law).
    pub stiffness: f64,
    pub exponent: f64,
    pub damping: f64,
}

impl Default for FretboardParams {
    /// A steel-string acoustic set-up (orders of magnitude).
    fn default() -> Self {
        Self {
            fret_height: 1.0e-3,
            relief: 0.25e-3,
            body_fret: 14,
            action_bass: 2.4e-3,
            action_treble: 1.8e-3,
            nut_clearance: 0.5e-3,
            stiffness: 1e9,
            exponent: 1.5,
            damping: 0.0,
        }
    }
}

/// How a note is played.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Technique {
    /// The right hand plucks ([`PluckParams::model`]).
    #[default]
    Pluck,
    /// The thumb strikes the string toward the frets, near the end of the
    /// neck.
    Slap,
    /// A finger pulls the string away from the board and lets it snap back
    /// onto the frets.
    Pop,
    /// A finger drives the string onto the fret: a hammer-on on a string
    /// that is held, tapping otherwise. No right-hand pluck.
    Tap,
    /// The fretting finger leaves for a lower fret on the same string,
    /// plucking it sideways as it goes.
    PullOff,
}

/// The techniques' contacts (orders of magnitude, to be measured).
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
pub struct TechniqueParams {
    /// Where the thumb strikes, from the bridge (m).
    pub slap_position: f64,
    /// Thumb flesh: stiffness (N/m), damping (N·s/m).
    pub thumb_stiffness: f64,
    pub thumb_damping: f64,
    /// Thumb speed at velocity 127 (m/s, proportional below), how far past
    /// the string's rest it goes (m), and how long it takes to come back (s).
    pub slap_speed: f64,
    pub slap_depth: f64,
    pub slap_return: f64,
    /// Force a pop lets go at, at velocity 127 (N).
    pub pop_force: f64,
    /// Tapping or hammering finger: speed at velocity 127 (m/s), distance
    /// behind the fret (m), how far past the fret's crown it presses (m),
    /// stiffness (N/m) and damping (N·s/m).
    pub tap_speed: f64,
    pub tap_offset: f64,
    pub tap_press: f64,
    pub tap_stiffness: f64,
    pub tap_damping: f64,
    /// Force a pull-off lets go at, at velocity 127 (N).
    pub pulloff_force: f64,
}

impl Default for TechniqueParams {
    fn default() -> Self {
        Self {
            slap_position: 0.2,
            thumb_stiffness: 3e4,
            thumb_damping: 3.0,
            slap_speed: 2.0,
            slap_depth: 3e-3,
            slap_return: 5e-3,
            pop_force: 6.0,
            tap_speed: 1.5,
            tap_offset: 6e-3,
            tap_press: 1e-3,
            tap_stiffness: 2e4,
            tap_damping: 2.0,
            pulloff_force: 1.5,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
pub struct GuitarModel {
    /// Nut-to-saddle length of the open strings (m).
    pub scale_length: f64,
    pub frets: u8,
    /// Low string first.
    pub strings: Vec<StringParams>,
    pub support: SupportParams,
    pub pluck: PluckParams,
    pub mute: MuteParams,
    pub fretboard: FretboardParams,
    pub techniques: TechniqueParams,
    pub output: Output,
    /// Highest mode frequency (Hz), also capped below Nyquist.
    pub max_freq: f64,
    /// Digital level per unit of the output (V, or what `Output` says):
    /// full scale is 1/output_gain volts at the jack.
    pub output_gain: f64,
}

impl GuitarModel {
    /// Steel-string acoustic, light gauge .012–.053 phosphor bronze, 650 mm
    /// scale. Tensions and core diameters are approximate catalogue
    /// values, to be fitted on recordings.
    pub fn steel_acoustic() -> Self {
        Self {
            scale_length: 0.650,
            frets: 20,
            strings: vec![
                StringParams::wound(40, 120.0, 1.35e-3, 0.46e-3),
                StringParams::wound(45, 133.0, 1.07e-3, 0.41e-3),
                StringParams::wound(50, 132.0, 0.81e-3, 0.33e-3),
                StringParams::wound(55, 116.0, 0.61e-3, 0.28e-3),
                StringParams::plain(59, 104.0, 0.41e-3),
                StringParams::plain(64, 102.0, 0.30e-3),
            ],
            support: SupportParams::default(),
            pluck: PluckParams::default(),
            mute: MuteParams::default(),
            fretboard: FretboardParams::default(),
            techniques: TechniqueParams::default(),
            output: Output::default(),
            max_freq: 16_000.0,
            output_gain: 1.0,
        }
    }

    /// Solid-body electric, 648 mm (25.5") scale, nickel-wound .010–.046,
    /// three single coils (bridge, middle, neck; the neck one selected),
    /// played with a plectrum. A heavy, stiff body: small admittances, long
    /// sustain. Approximate catalogue values.
    pub fn electric_solid() -> Self {
        Self {
            scale_length: 0.648,
            frets: 22,
            strings: vec![
                StringParams::wound(40, 77.8, 1.17e-3, 0.46e-3),
                StringParams::wound(45, 86.7, 0.91e-3, 0.41e-3),
                StringParams::wound(50, 81.8, 0.66e-3, 0.36e-3),
                StringParams::plain(55, 73.8, 0.43e-3),
                StringParams::plain(59, 68.5, 0.33e-3),
                StringParams::plain(64, 72.1, 0.254e-3),
            ],
            support: SupportParams {
                perpendicular: Admittance { conductance: 5.0e-4, conductance_per_khz: 2.0e-3, susceptance: 0.0 },
                parallel: Admittance { conductance: 1.0e-4, conductance_per_khz: 5.0e-4, susceptance: 0.0 },
                cross: Admittance { conductance: 1.0e-4, conductance_per_khz: 0.0, susceptance: 0.0 },
            },
            pluck: PluckParams { model: PluckModel::Plectrum, position: 0.09, max_force: 2.0, ..PluckParams::default() },
            mute: MuteParams::default(),
            fretboard: FretboardParams { fret_height: 1.1e-3, relief: 0.2e-3, body_fret: 16, action_bass: 1.8e-3, action_treble: 1.5e-3, ..FretboardParams::default() },
            techniques: TechniqueParams { slap_position: 0.15, pop_force: 4.0, pulloff_force: 1.0, ..TechniqueParams::default() },
            output: Output::Magnetic(MagneticOutput {
                pickups: vec![Pickup::single_coil(0.041), Pickup::single_coil(0.098), Pickup::single_coil(0.160)],
                selected: vec![2],
                ..MagneticOutput::default()
            }),
            max_freq: 16_000.0,
            output_gain: 0.3,
        }
    }

    /// Four-string electric bass, 864 mm (34") scale, .045–.100 nickel
    /// round-wound, two single coils (both on), plucked with the fingers
    /// between them. Approximate catalogue values.
    pub fn electric_bass() -> Self {
        let bass = |note, t, d, core| StringParams { delta_ve: 3.0e-3, ..StringParams::wound(note, t, d, core) };
        Self {
            scale_length: 0.864,
            frets: 20,
            strings: vec![
                bass(28, 177.0, 2.54e-3, 0.80e-3),
                bass(33, 200.0, 2.03e-3, 0.70e-3),
                bass(38, 198.0, 1.65e-3, 0.60e-3),
                bass(43, 177.0, 1.14e-3, 0.50e-3),
            ],
            support: SupportParams {
                perpendicular: Admittance { conductance: 4.0e-4, conductance_per_khz: 2.0e-3, susceptance: 0.0 },
                parallel: Admittance { conductance: 1.0e-4, conductance_per_khz: 5.0e-4, susceptance: 0.0 },
                cross: Admittance { conductance: 1.0e-4, conductance_per_khz: 0.0, susceptance: 0.0 },
            },
            pluck: PluckParams { model: PluckModel::Finger, position: 0.12, max_force: 6.0, drag_time: 0.03, ..PluckParams::default() },
            mute: MuteParams::default(),
            fretboard: FretboardParams { fret_height: 1.2e-3, relief: 0.3e-3, body_fret: 16, action_bass: 2.4e-3, action_treble: 2.0e-3, ..FretboardParams::default() },
            techniques: TechniqueParams { slap_position: 0.27, pop_force: 12.0, slap_depth: 4e-3, pulloff_force: 3.0, ..TechniqueParams::default() },
            output: Output::Magnetic(MagneticOutput {
                pickups: vec![
                    Pickup { inductance: 3.5, resistance: 7.0e3, ..Pickup::single_coil(0.092) },
                    Pickup { inductance: 3.5, resistance: 7.0e3, ..Pickup::single_coil(0.165) },
                ],
                selected: vec![0, 1],
                circuit: Circuit { tone_cap: 47e-9, ..Circuit::default() },
                ..MagneticOutput::default()
            }),
            max_freq: 12_000.0,
            output_gain: 0.5,
        }
    }
}

impl Default for GuitarModel {
    fn default() -> Self {
        Self::steel_acoustic()
    }
}
