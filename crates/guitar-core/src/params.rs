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
    /// through a spring–dashpot contact and lets go at once. The slip of a
    /// real release comes with G2.
    Finger,
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
    /// Displacement at velocity 127 (m); proportional below. For the finger,
    /// how far the fingertip travels.
    pub max_displacement: f64,
    /// Stiffness (N/m) and damping (N·s/m) of the fingertip's flesh.
    pub finger_stiffness: f64,
    pub finger_damping: f64,
    /// Time the finger takes to drag the string out (s).
    pub drag_time: f64,
}

impl Default for PluckParams {
    fn default() -> Self {
        Self {
            model: PluckModel::Finger,
            position: 0.12,
            angle: std::f64::consts::FRAC_PI_4,
            width: 8.0e-3,
            max_displacement: 3.0e-3,
            finger_stiffness: 5.0e3,
            finger_damping: 5.0,
            drag_time: 0.02,
        }
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
    /// Highest mode frequency (Hz), also capped below Nyquist.
    pub max_freq: f64,
    /// Output per newton of bridge force (perpendicular).
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
            max_freq: 16_000.0,
            output_gain: 0.1,
        }
    }
}

impl Default for GuitarModel {
    fn default() -> Self {
        Self::steel_acoustic()
    }
}
