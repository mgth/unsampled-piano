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

/// Where and how the right hand plucks.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
pub struct PluckParams {
    /// Distance from the bridge (m). It stays put when the left hand frets:
    /// the relative position grows with the fret.
    pub position: f64,
    /// Width of the contact (m): a fingertip rounds the corner of the
    /// plucked shape, a plectrum hardly.
    pub width: f64,
    /// Displacement released at velocity 127 (m); proportional below.
    pub max_displacement: f64,
}

impl Default for PluckParams {
    fn default() -> Self {
        Self { position: 0.12, width: 8.0e-3, max_displacement: 3.0e-3 }
    }
}

/// What stops a string at note-off: the fretting finger lifting off (or the
/// hand laid on an open string), as a dashpot touching the string.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
pub struct MuteParams {
    /// Distance of the finger from the fret (or nut) end of the vibrating
    /// length (m).
    pub distance: f64,
    /// Damping coefficient of the flesh (N·s/m).
    pub resistance: f64,
}

impl Default for MuteParams {
    fn default() -> Self {
        // Chosen so that a muted low E falls 60 dB in ≈ 0.35 s (σ ≈ 20 /s on
        // its fundamental; lighter strings stop faster); to be measured.
        Self { distance: 0.01, resistance: 60.0 }
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
    pub pluck: PluckParams,
    pub mute: MuteParams,
    /// Loss factor standing in for the bridge until the body is modelled
    /// (energy the string gives to the top, per radian).
    pub eta_bridge: f64,
    /// Highest mode frequency (Hz), also capped below Nyquist.
    pub max_freq: f64,
    /// Output per newton of bridge force.
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
            pluck: PluckParams::default(),
            mute: MuteParams::default(),
            eta_bridge: 1.0e-3,
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
