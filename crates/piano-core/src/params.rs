//! Per-note physical parameters of the model.
//!
//! Default laws are fitted on Salamander Grand V3 (Yamaha C5, see
//! docs/salamander.md); literature laws are kept for comparison. The string
//! scale (lengths, tensions) is a generic grand-piano scale, not measured.

/// Fundamental of a MIDI note in equal temperament, A4 = `a4` Hz.
pub fn note_freq(note: u8, a4: f64) -> f64 {
    a4 * 2f64.powf((note as f64 - 69.0) / 12.0)
}

/// Inharmonicity B across the keyboard as the sum of a bass and a treble
/// exponential (Rigaud, David & Daudet, JASA 2013).
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
pub struct InharmonicityLaw {
    pub a_bass: f64,
    pub b_bass: f64,
    pub a_treble: f64,
    pub b_treble: f64,
}

impl InharmonicityLaw {
    /// Average law with the constants quoted by Renault et al. (DAFx 2022).
    pub const RIGAUD_2013: Self = Self { a_bass: -0.0847, b_bass: -5.82, a_treble: 0.0926, b_treble: -13.64 };
    /// Fitted on Salamander (rms 0.07 in log B).
    pub const SALAMANDER_C5: Self =
        Self { a_bass: -0.111_714, b_bass: -6.107_058, a_treble: 0.088_550, b_treble: -13.495_151 };

    pub fn b(&self, note: u8) -> f64 {
        let m = note as f64;
        (self.a_bass * m + self.b_bass).exp() + (self.a_treble * m + self.b_treble).exp()
    }
}

/// Literature average law, kept for comparison.
pub fn inharmonicity_rigaud(note: u8) -> f64 {
    InharmonicityLaw::RIGAUD_2013.b(note)
}

/// Stretch tuning in cents relative to equal temperament:
/// `a_t·e^{k_t(m−69)} − a_b·e^{−k_b(m−69)} + c`.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
pub struct StretchLaw {
    pub a_t: f64,
    pub k_t: f64,
    pub a_b: f64,
    pub k_b: f64,
    pub c: f64,
}

impl StretchLaw {
    pub const NONE: Self = Self { a_t: 0.0, k_t: 0.0, a_b: 0.0, k_b: 0.0, c: 0.0 };
    /// Fitted on Salamander, C8 excluded (rms 2 cents): −16 c at A0, +25 c at C8.
    pub const SALAMANDER_C5: Self = Self { a_t: 4.224_489, k_t: 0.049_132, a_b: 0.019_494, k_b: 0.135_202, c: -3.708_095 };

    pub fn cents(&self, note: u8) -> f64 {
        let x = note as f64 - 69.0;
        self.a_t * (self.k_t * x).exp() - self.a_b * (-self.k_b * x).exp() + self.c
    }
}

/// Decay rate σ_k = b1(note) + b2·f_k² (1/s) with log b1 linear in the note,
/// plus a constant floor `b1_floor` (internal friction of the wound bass
/// strings, which the log-linear law misses).
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
pub struct LossLaw {
    pub b1_slope: f64,
    pub b1_offset: f64,
    pub b2: f64,
    /// Added to b1 below `wound_until` (wound strings: friction of the
    /// copper winding), fading out over the octave above it. 0.2 s⁻¹ puts
    /// the bass decay envelopes within 1 dB of Salamander (docs/sustain.md),
    /// of the order of its ESPRIT aftersound rates there (0.15–0.18). 0
    /// since the bass side of the board is as mobile as the rest: the
    /// board takes that loss, and the bass sounds as loud as on the real
    /// grands against the middle (docs/bass_decay.md).
    pub b1_floor: f64,
    pub wound_until: u8,
    /// Steeper growth of b1 above `top_from`: b1 × e^{top_slope·(key −
    /// top_from)}. The top strings (no dampers) otherwise rang on too long
    /// after release (docs/release_ring.md).
    pub top_slope: f64,
    pub top_from: u8,
}


impl LossLaw {
    /// Early ("prompt") decay of single partials, Salamander survey. A single
    /// string per note cannot reproduce the two-stage decay (Phase 2).
    pub const SALAMANDER_EARLY: Self =
        Self { b1_slope: 0.0493, b1_offset: -1.737, b2: 2.26e-7, b1_floor: 0.0, wound_until: 48, top_slope: 0.0, top_from: 108 };
    /// Intrinsic string losses: law of the ESPRIT aftersound (no bridge
    /// losses), scaled down by the decay calibration of the full instrument
    /// (tools/decay_grid.py): the raw fit (R² 0.31) overestimated them. b1
    /// was lowered again (×0.65 at C4, ×0.5 at F#6) on the sustain after
    /// 1 s of a commercial modelled piano, Iowa and Salamander (docs/long_sustain.md): the
    /// slow components of the treble fundamentals last 7–9 s there. The
    /// frequency term b2 = 6e-8: 1e-7, set with the bridge spreading
    /// against Salamander (docs/sustain.md), left the upper partials of the
    /// treble and top 15–40 dB short in the sustain against eight real
    /// grands (tools/bench.py: 1.32 → 1.16 without the top's 1 kHz band,
    /// docs/top_treble.md). No faster growth above C6 (top_slope 0): the extra
    /// losses set against a commercial modelled piano's release ring
    /// (docs/release_ring.md) cut short the slow aftersound that real top
    /// strings keep, −11 to −21 dB/s at C7 after the first 0.4 s on four
    /// real grands (docs/top_treble.md).
    pub const SALAMANDER_STRING: Self =
        Self { b1_slope: 0.055, b1_offset: -5.1, b2: 6.0e-8, b1_floor: 0.0, wound_until: 48, top_slope: 0.0, top_from: 84 };

    pub fn sigma(&self, note: u8, freq: f64) -> f64 {
        let wound = ((self.wound_until as f64 + 12.0 - note as f64) / 12.0).clamp(0.0, 1.0);
        let top = self.top_slope * (note as f64 - self.top_from as f64).max(0.0);
        self.b1_floor * wound + (self.b1_slope * note as f64 + self.b1_offset + top).exp() + self.b2 * freq * freq
    }
}

/// Strings per key and their mistuning.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
pub struct UnisonLaw {
    /// First bichord and first trichord key.
    pub bichord_from: u8,
    pub trichord_from: u8,
    /// Spread between the extreme strings (cents) up to C4, and at C8
    /// (quadratic in between).
    pub width_mid: f64,
    pub width_treble: f64,
    /// The hammer never strikes the strings of a unison equally (uneven
    /// felt, strings not quite level): relative spread of the force each
    /// string receives (0 = equal). The uneven part directly excites the
    /// slow out-of-phase modes, i.e. the aftersound.
    pub hit_spread: f64,
    /// The spread grows as the hammer slows, hit_spread·(soft_speed/v)^soft_exponent
    /// (capped at 1; exponent 0 keeps it constant): a light blow meets the
    /// strings with the felt's surface only, unevenly. On eight real grands
    /// the upper partials of the middle decay at about −20 dB/s over the
    /// first 0.3 s at pp and −30 at ff; the model's decay did not depend on
    /// the velocity (docs/release.md).
    pub soft_exponent: f64,
    pub soft_speed: f64,
    /// Height spread of a unison's strings under the hammer (m): a blow at
    /// v meets the lowest string up to height_spread/v after the highest,
    /// which takes no force until then. 0.1 mm: 0.2 ms at pp, short against
    /// a 2–3 ms contact, so the strings' partials barely part
    /// (docs/release.md).
    pub height_spread: f64,
}

impl UnisonLaw {
    /// Generic grand layout; treble width calibrated on the ESPRIT
    /// statistics of Salamander (tools/unison_grid.py); width up to C4 1
    /// cent: the aftersound of the bass and middle then sits 1–1.5 cent from
    /// the prompt sound and 13–17 dB below it, as in the commercial piano, Iowa and
    /// Salamander (docs/long_sustain.md; 0.35 cent left it 10 dB too weak
    /// and the notes died after 1 s); hit spread ±20 % set on the
    /// aftersound level of the treble (docs/sustain.md).
    pub const GRAND: Self = Self { bichord_from: 28, trichord_from: 45, width_mid: 1.0, width_treble: 5.0, hit_spread: 0.2, soft_exponent: 1.0, soft_speed: 2.75, height_spread: 1e-4 };
    /// Never let strings coincide exactly (degenerate coupled subspace).
    const MIN_SPREAD: f64 = 0.02;

    pub fn strings(&self, note: u8) -> usize {
        if note >= self.trichord_from {
            3
        } else if note >= self.bichord_from {
            2
        } else {
            1
        }
    }

    pub fn width(&self, note: u8) -> f64 {
        let t = ((note as f64 - 60.0) / 48.0).max(0.0);
        (self.width_mid + (self.width_treble - self.width_mid) * t * t).max(Self::MIN_SPREAD)
    }

    /// Detuning of each string (cents), deterministic but irregular per key.
    pub fn detune(&self, note: u8) -> [f64; 3] {
        let w = self.width(note);
        let h = ((note as f64 * 12.9898).sin() * 43_758.545_3).rem_euclid(1.0);
        match self.strings(note) {
            1 => [0.0; 3],
            2 => [-0.5 * w * (0.7 + 0.6 * h), 0.5 * w, 0.0],
            _ => [-0.5 * w, (h - 0.5) * 0.6 * w, 0.5 * w],
        }
    }

    /// Share of the hammer force each string receives (mean 1),
    /// deterministic but irregular per key and independent of the detuning.
    pub fn hits(&self, note: u8) -> [f64; 3] {
        self.pattern(note).map(|d| 1.0 + self.hit_spread * d)
    }

    /// Uneven part of [`Self::hits`] per unit spread: zero mean, extreme ±1.
    pub fn pattern(&self, note: u8) -> [f64; 3] {
        let g = ((note as f64 * 78.233).sin() * 12_345.678_9).rem_euclid(1.0);
        match self.strings(note) {
            1 => [0.0; 3],
            2 => {
                let d = if g < 0.5 { -1.0 } else { 1.0 } * (0.5 + g.min(1.0 - g));
                [d, -d, 0.0]
            }
            _ => {
                // Rotated by key.
                let base = [1.0, -0.5 + (g - 0.5), -0.5 - (g - 0.5)];
                let r = (g * 3.0) as usize % 3;
                [base[r], base[(r + 1) % 3], base[(r + 2) % 3]]
            }
        }
    }

    /// Height of each string under the hammer, in units of `height_spread`
    /// below the highest (0 … 1), irregular per key and independent of the
    /// detuning and the force pattern.
    pub fn heights(&self, note: u8) -> [f64; 3] {
        let g = ((note as f64 * 51.719).sin() * 31_415.926_5).rem_euclid(1.0);
        match self.strings(note) {
            1 => [0.0; 3],
            2 => if g < 0.5 { [0.0, 0.5 + g, 0.0] } else { [g, 0.0, 0.0] },
            _ => {
                let base = [0.0, 1.0, g];
                let r = (g * 7.0) as usize % 3;
                [base[r], base[(r + 1) % 3], base[(r + 2) % 3]]
            }
        }
    }

    /// Spread of the hammer force at hammer speed `v` (m/s).
    pub fn spread_at(&self, v: f64) -> f64 {
        if self.soft_exponent == 0.0 {
            return self.hit_spread;
        }
        (self.hit_spread * (self.soft_speed / v.max(1e-3)).powf(self.soft_exponent)).min(1.0)
    }
}

/// Bridge admittance seen by the strings: G(f) = g_1k·(f/1 kHz)^slope (s/kg),
/// Im Y = b_over_g·G. Measured on Salamander from the ESPRIT prompt/aftersound
/// decay difference (tools/bridge_conductance.py): G ≈ 4–6e-4 s/kg, nearly
/// flat, with ±2.5× modal scatter left to the soundboard model (Phase 4).
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
pub struct BridgeLaw {
    pub g_1k: f64,
    pub slope: f64,
    pub b_over_g: f64,
}

impl BridgeLaw {
    /// b_over_g calibrated on the ESPRIT statistics (tools/unison_grid.py).
    pub const SALAMANDER_C5: Self = Self { g_1k: 4.6e-4, slope: 0.109, b_over_g: 1.5 };

    pub fn admittance(&self, f: f64) -> crate::complex::C64 {
        let g = self.g_1k * (f / 1000.0).powf(self.slope);
        crate::complex::C64::new(g, self.b_over_g * g)
    }
}

/// Horizontal polarisation of the strings, relative to the vertical one.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
pub struct PolarizationLaw {
    pub enabled: bool,
    /// ε_H / ε_V: the bridge is much stiffer sideways.
    pub h_coupling: f64,
    /// ε_X / ε_V: transfer between polarisations (bridge rocking). The
    /// bridge stays passive only if ε_X² ≤ ε_V·ε_H, i.e. cross ≤ √h_coupling;
    /// larger values are clamped.
    pub cross: f64,
    /// Horizontal minus vertical frequency (cents), from the terminations.
    pub detune_cents: f64,
    /// Share of the horizontal bridge force reaching the output.
    pub radiation: f64,
    /// Share of the hammer force acting horizontally.
    pub hammer_tilt: f64,
}

impl PolarizationLaw {
    pub const OFF: Self =
        Self { enabled: false, h_coupling: 0.0, cross: 0.0, detune_cents: 0.0, radiation: 0.0, hammer_tilt: 0.0 };
    /// Calibrated on the ESPRIT statistics of Salamander with the full
    /// instrument (tools/decay_grid.py): weak sideways coupling, cross term
    /// at the passivity limit √h; gives single strings their measured
    /// aftersound (≈ −27 dB, slow).
    pub const DEFAULT: Self =
        Self { enabled: true, h_coupling: 0.03, cross: 0.173, detune_cents: 0.3, radiation: 0.1, hammer_tilt: 0.05 };

    /// Cross coupling actually used: clamped to keep the bridge passive.
    pub fn passive_cross(&self) -> f64 {
        self.cross.min(self.h_coupling.sqrt())
    }
}

/// Felt dampers: a dashpot of coefficient c = e·strength·2Z0 at
/// `position`·L adds to partial k the decay rate
/// σ_k = c·⟨sin²(kπx/L)⟩/(μL)·(2Z0/(c + 2Z0))², averaged over the pad width
/// (impedance mismatch makes a too hard felt reflect instead of absorb).
/// e ∈ [0, 1] is the contact.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
pub struct DamperLaw {
    /// Contact point relative to the string length, from the agraffe end.
    pub position: f64,
    /// Width of the felt pad (m): partials with a node under the damper
    /// are still damped by the rest of the pad.
    pub width: f64,
    /// Share of the full rate every partial gets at least: the felt is
    /// crushed over a length of string, presses on 2–3 strings whose
    /// contact points differ, and brakes the sideways motion too, so no
    /// partial escapes it. Without it the partials with a node under the
    /// pad (7, 14, 21 at 0.14·L) rang on for seconds after the release in
    /// the bass (docs/release_ring.md).
    pub node_floor: f64,
    /// c_max / (2·Z0): 1 is the impedance-matched, most absorbing felt.
    /// 0.1: a released note loses 15–30 dB in its first 0.25 s, as on a
    /// Disklavier grand; at 1 the dampers stopped notes 2–4 times too fast
    /// and left holes in pedalled passages (docs/release.md).
    pub strength: f64,
    /// Felt nonlinearity: the dashpot grows as contact^exponent (felt
    /// compression, as Stulov's p for hammers). With 1, a damper barely
    /// touching already absorbed 89 % of the full rate at half contact:
    /// no real half pedal, and an abrupt stop as the damper lands. 1.5
    /// with the softer felt: a released note starts losing within 30 ms, as
    /// on a Disklavier grand (docs/release.md).
    pub contact_exponent: f64,
    /// Share of the rate the horizontal polarisation gets: the felt rests
    /// on the strings and brakes their sideways motion by friction only.
    /// A released note keeps a slow tail 25–40 dB down, as on a Disklavier
    /// grand (docs/release.md).
    pub horizontal: f64,
    /// Highest key with a damper: D6 (86), the top 22 keys ring freely. In
    /// the sustain of the top notes, real grands have lines at 0.7–1.4 kHz,
    /// the undamped strings answering sympathetically; with F#6 (90), set
    /// on a commercial modelled piano (docs/release_ring.md), the model had
    /// nothing there (bench: top 1 kHz sustain band z −8.3 → −0.2, score
    /// 1.31 → 1.09, docs/top_treble.md).
    pub highest_note: u8,
    /// Sustain-pedal travel (0–1): lost motion before the dampers start to
    /// lift (`pedal_free`), and travel at which they have left the strings
    /// (`pedal_contact`). Pianists rest the foot on the pedal: recorded
    /// performances (Disklavier, MAESTRO) seldom drop below CC64 ≈ 40.
    pub pedal_free: f64,
    pub pedal_contact: f64,
    /// Shape of the contact versus pedal travel below that point.
    pub pedal_curve: f64,
    /// Time constants (s) of the damper falling on / lifting off the strings.
    pub fall_time: f64,
    pub lift_time: f64,
}

impl DamperLaw {
    pub const GRAND: Self = Self {
        position: 0.14,
        width: 0.02,
        // 15 % of a fully exposed partial's rate: the node partials of the
        // bass (7 at A0–C3) die 23–45 dB down within 1.3 s of the release,
        // as on a commercial modelled piano (−25 to −39 dB).
        node_floor: 0.15,
        strength: 0.1,
        contact_exponent: 1.5,
        horizontal: 0.03,
        highest_note: 86,
        // Dampers resting up to CC64 ≈ 38, off the strings from ≈ 76: on a
        // Disklavier grand (Liang's pedalling dataset, tools/pedal_bench.py)
        // CC64 = 64 is still a half pedal, a released note losing 7–28 dB
        // more in its first second than with the pedal down (bass to
        // treble; the model 10–25 dB), and 96 is a full pedal. Curve 1.2
        // with the softer felt (docs/release.md).
        pedal_free: 0.3,
        pedal_contact: 0.6,
        pedal_curve: 1.2,
        fall_time: 0.015,
        lift_time: 0.005,
    };

    /// Contact (0 lifted … 1 resting) imposed by the sustain pedal alone.
    pub fn pedal_engagement(&self, travel: f64) -> f64 {
        let span = (self.pedal_contact - self.pedal_free).max(1e-6);
        ((self.pedal_contact - travel) / span).clamp(0.0, 1.0).powf(self.pedal_curve)
    }

    /// Mean of sin²(kπx/L) over the pad: 1/2 − cos(2kπ·pos)·sinc(kπ·w/L)/2.
    pub fn shape(&self, k: u32, length: f64) -> f64 {
        let a = k as f64 * std::f64::consts::PI * self.width / length;
        let sinc = if a > 1e-9 { a.sin() / a } else { 1.0 };
        let pad = 0.5 - 0.5 * (2.0 * k as f64 * std::f64::consts::PI * self.position).cos() * sinc;
        // ⟨sin²⟩ peaks at 1: the floor is a share of a fully exposed partial.
        self.node_floor + (1.0 - self.node_floor) * pad
    }

    /// Decay-rate factor g(c) such that σ_k = g·shape(k).
    pub fn rate(&self, engagement: f64, tension: f64, mu: f64, length: f64) -> f64 {
        let z0 = (tension * mu).sqrt();
        let c = engagement.max(0.0).powf(self.contact_exponent) * self.strength * 2.0 * z0;
        let m = 2.0 * z0 / (c + 2.0 * z0);
        c / (mu * length) * m * m
    }
}

/// Mechanical noises, as impulses into the modelled structures: the key
/// bottoming on the keybed and the damper rail (pedal) knock the frame, whose
/// force reaches the soundboard; a damper falling on a ringing string kicks
/// its modes at the damper point.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
pub struct NoiseLaw {
    /// Peak force (N) into the board per m/s of hammer speed when the key
    /// bottoms (through keybed, rim and frame).
    pub keybed: f64,
    /// Effective felt mass (kg) and fall speed (m/s) at mid release
    /// velocity: most of the head's momentum is absorbed by the soft felt,
    /// only a small effective mass kicks the string.
    pub damper_mass: f64,
    pub damper_speed: f64,
    /// Duration (s) of the felt landing: the impulse is a half-sine of this
    /// length, whose spectrum falls quickly above 1/duration (soft thud).
    pub damper_landing: f64,
    /// Peak force (N) into the board for a full-speed pedal move.
    pub pedal: f64,
    /// Duration (s) of the keybed / pedal knocks.
    pub knock_time: f64,
    /// The action's knock heard directly (keybed, shanks, keyframe; not
    /// through the board): peak pressure (Pa) at the microphones per m/s of
    /// hammer speed, its decay time T60 (s) at 500 Hz and its delay after
    /// the hammer reaches the string (s). 0: none (crate::action). 0.05 Pa
    /// per m/s puts the knock below the top keys' fundamentals at the level
    /// of eight real grands' (docs/action_noise.md).
    pub action: f64,
    pub action_time: f64,
    pub action_delay: f64,
}

impl NoiseLaw {
    /// Levels set 20–30 dB below a mezzo-forte note (examples/noise_levels.rs).
    pub const DEFAULT: Self =
        Self { keybed: 0.25, damper_mass: 0.007, damper_speed: 0.1, damper_landing: 0.004, pedal: 0.35, knock_time: 0.003, action: 0.05, action_time: 0.2, action_delay: 0.008 };
    pub const OFF: Self =
        Self { keybed: 0.0, damper_mass: 0.0, damper_speed: 0.0, damper_landing: 0.004, pedal: 0.0, knock_time: 0.003, action: 0.0, action_time: 0.2, action_delay: 0.008 };

    /// |spectrum| of a unit-area half-sine pulse of duration τ at ω.
    pub fn half_sine_spectrum(omega: f64, tau: f64) -> f64 {
        let x = omega * tau / std::f64::consts::PI;
        if (x - 1.0).abs() < 1e-6 {
            std::f64::consts::FRAC_PI_4
        } else {
            ((omega * tau / 2.0).cos() / (1.0 - x * x)).abs()
        }
    }
}

/// Geometric nonlinearity of the strings (Bank & Sujbert 2005): a string
/// vibrating with a large amplitude stretches, which drives its
/// longitudinal modes with the products of the transverse partials (sum and
/// difference frequencies: the "phantom partials"); their force on the
/// bridge reaches the board. It grows as the amplitude squared: the bass
/// brightens and growls at ff. One-way (the transverse motion is linear).
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
pub struct StringNonlinearity {
    pub enabled: bool,
    /// Young's modulus of the steel core (Pa); the core diameter follows
    /// from the inharmonicity, EA = E·π·d²/4 with B = π³E·d⁴/(64·T·L²).
    pub young: f64,
    /// Highest key computed, partials and longitudinal modes kept (Hz).
    pub max_note: u8,
    pub max_partial_hz: f64,
    pub max_long_hz: f64,
    /// Loss factor of the longitudinal modes.
    pub long_loss: f64,
    /// Share of the longitudinal force at the bridge that drives the board
    /// (string angle over the bridge, bridge rocking).
    pub coupling: f64,
    /// The longitudinal force pulls the bridge in the board's plane, where
    /// the board is stiff; it bends the board through its moment about the
    /// bridge foot, which grows with the board's wavenumber: low-frequency
    /// (quasi-static) tension changes hardly reach it. First-order high-pass
    /// at this corner (Hz).
    pub coupling_corner: f64,
}

impl Default for StringNonlinearity {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl StringNonlinearity {
    pub const DEFAULT: Self = Self {
        enabled: true,
        young: 2.0e11,
        // Above E4 the first longitudinal mode passes 4 kHz and little is
        // left; above 8 kHz the longitudinal modes add nothing audible.
        max_note: 64,
        max_partial_hz: 5000.0,
        max_long_hz: 8000.0,
        long_loss: 0.005,
        // Phantom partials at the level of the references (≈ −28 dB below
        // the transverse partials in the bass at ff, tools/phantom_levels.py).
        coupling: 0.3,
        coupling_corner: 1000.0,
    };
}

/// Sympathetic resonance through the bridge.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
pub struct SympatheticLaw {
    pub enabled: bool,
    /// Strings that only listen run their partials below this frequency.
    pub max_freq: f64,
    /// Scale on the bridge-to-string gain (1 = the one-way physical
    /// coupling). Measured against a Disklavier grand played with and
    /// without the pedal (Liang's pedalling dataset, tools/pedal_bench.py,
    /// close microphones): with 1, the energy the other strings add between
    /// the partials of a pedalled chord matches within 0.5–2.5 dB (middle
    /// −19.3 dB re the partials against −19.7, treble −24.1 against −21.8),
    /// and the chord's own partials sustain as on the Disklavier (no
    /// growth at the octaves); 0.25 left the halo 7–13 dB short.
    pub gain: f64,
    /// Cancel each key's own contribution to its bridge input.
    pub self_cancel: bool,
}

impl SympatheticLaw {
    pub const DEFAULT: Self = Self { enabled: true, max_freq: 5000.0, gain: 1.0, self_cancel: true };
}

/// Partial frequency of a stiff string: f_k = k·f0·sqrt(1 + B·k²).
pub fn partial_freq(f0: f64, b: f64, k: u32) -> f64 {
    let k = k as f64;
    k * f0 * (1.0 + b * k * k).sqrt()
}

/// Generic grand-piano string scale. Speaking lengths shrink by `ratio`
/// per octave from C8 down to `break_note`, then grow slowly to `len_a0`
/// (foreshortened, wound bass strings). Linear mass follows from the tension
/// and the fundamental. Placeholder until measured scales are digitised.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
pub struct StringScale {
    pub len_c8: f64,
    pub ratio_per_octave: f64,
    pub break_note: u8,
    pub len_a0: f64,
    pub tension: f64,
}

impl StringScale {
    pub const GRAND_2M: Self = Self { len_c8: 0.052, ratio_per_octave: 1.85, break_note: 48, len_a0: 1.45, tension: 750.0 };

    pub fn length(&self, note: u8) -> f64 {
        let free = |m: f64| self.len_c8 * self.ratio_per_octave.powf((108.0 - m) / 12.0);
        let m = note as f64;
        let brk = self.break_note as f64;
        if m >= brk {
            free(m)
        } else {
            let t = (brk - m) / (brk - 21.0);
            free(brk) + t * (self.len_a0 - free(brk))
        }
    }
}

/// Everything the voice needs for one key.
#[derive(Clone, Copy, Debug)]
pub struct NoteParams {
    pub f0: f64,
    pub b: f64,
    pub length: f64,
    pub tension: f64,
    /// Linear mass (kg/m).
    pub mu: f64,
    /// Striking point relative to the string length, from the agraffe end
    /// (the bridge is at the other end).
    pub strike_pos: f64,
    /// Width of the felt contact patch (m).
    pub contact_width: f64,
    pub hammer: HammerParams,
    pub strings: usize,
    /// Detuning of each string in cents (first `strings` entries).
    pub detune: [f64; 3],
}

impl NoteParams {
    /// Rate (1/s) coupling the strings through the bridge at frequency f:
    /// ε = (T/L)·Y(f). Re ε is one string's bridge loss.
    pub fn coupling(&self, bridge: &BridgeLaw, f: f64) -> crate::complex::C64 {
        bridge.admittance(f).scale(self.tension / self.length)
    }
}

/// MIDI velocity → hammer speed (m/s): vmin·(vmax/vmin)^(t^curve), t =
/// (velocity − 1)/126. With curve 1 the map is exponential, the
/// logarithmic map of the recording pianos (Goebl & Bresin 2003); below 1
/// it opens the soft end. Fitted on two Disklavier grands (tools/
/// velocity_fit.py): BiVib's, recorded at ten velocities per key, and
/// Liang's, at 49, 80 and 96. At velocity 12, BiVib's level re velocity
/// 100 falls 18 (bass) to 31 dB (treble), where the earlier exponential
/// 0.6–4.2 m/s fell 11–20 dB; 0.26–4.4 m/s with curve 0.75 follows both
/// within 1–2.5 dB from the bass to the treble (2.4 dB rms over every
/// third key, against 3.5). The two pianos differ by 20–25 % in their
/// mid-range slope, the fit sits between them. The speed at velocity 100
/// (2.8 m/s), where the felt and the brightness were set (docs/
/// brightness.md), is kept: [`VelocityLaw::PLAYBACK`].
///
/// The default follows instead the velocities a pianist's fingers produce
/// (what a keyboard sends, what a Disklavier records): within recorded
/// performances an onset gains 1.4–1.6 times as much per velocity step as
/// on the played-back scale. Fitted on Liang's and MAESTRO's performances
/// with velocity 60 kept at the played-back law's speed (tools/
/// velocity_fit_pieces.py, docs/bench.md).
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
pub struct VelocityLaw {
    pub vmin: f64,
    pub vmax: f64,
    pub curve: f64,
}

impl VelocityLaw {
    pub const DEFAULT: Self = Self { vmin: 0.205, vmax: 5.6, curve: 0.78 };
    /// The scale a Disklavier plays back (BiVib's and Liang's isolated notes).
    pub const PLAYBACK: Self = Self { vmin: 0.26, vmax: 4.4, curve: 0.75 };
    /// Fit of the Salamander layers (sampler velocity ranges), 1.4 dB rms.
    pub const SALAMANDER_LAYERS: Self = Self { vmin: 0.4, vmax: 4.0, curve: 1.0 };

    pub fn hammer_speed(&self, velocity: u8) -> f64 {
        let t = (velocity.clamp(1, 127) as f64 - 1.0) / 126.0;
        self.vmin * (self.vmax / self.vmin).powf(t.powf(self.curve))
    }
}

/// The whole instrument description: laws evaluated per key.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
pub struct PianoModel {
    pub a4: f64,
    pub velocity: VelocityLaw,
    pub inharmonicity: InharmonicityLaw,
    pub stretch: StretchLaw,
    /// Intrinsic string losses (the bridge adds its own through coupling).
    pub losses: LossLaw,
    pub scale: StringScale,
    pub unison: UnisonLaw,
    /// Smooth bridge law, used when `soundboard` is None.
    pub bridge: BridgeLaw,
    pub polarization: PolarizationLaw,
    /// Modal soundboard: bridge admittance at each key and radiation.
    pub soundboard: Option<crate::soundboard::SoundboardParams>,
    pub damper: DamperLaw,
    pub sympathetic: SympatheticLaw,
    /// Longitudinal modes driven by the transverse motion (phantom partials).
    pub nonlinear: StringNonlinearity,
    pub noise: NoiseLaw,
    /// The room around the piano (early reflections and reverberation).
    pub room: crate::room::RoomParams,
    /// The cast-iron frame (driven by the agraffes) and the rim (driven by
    /// the board's edges); both need the soundboard.
    pub frame: crate::structure::FrameParams,
    pub rim: crate::structure::RimParams,
    /// Felt softening when the una corda pedal shifts the action fully.
    pub una_corda_softening: f64,
    /// Share of the sideways shift at full soft pedal: 1 on a grand (una
    /// corda); 0 on an upright, whose soft pedal moves the hammers closer
    /// to the strings instead (`half_blow`).
    pub una_corda_shift: f64,
    /// Hammer speed lost at full soft pedal: the half-blow rail of an
    /// upright shortens the hammers' throw (0 on a grand).
    pub half_blow: f64,
    /// Thickness (mm) of the felt strip that an upright's middle pedal
    /// lowers between hammers and strings (practice mute); 0: the middle
    /// pedal is a sostenuto.
    pub mute_felt: f64,
    /// Time (s) an upright's action needs after a key's release before it
    /// can throw the hammer at full speed again: without the grand's double
    /// escapement, the key must rise nearly all the way for the jack to
    /// reset under the hammer. 0: a grand (repetition at any time).
    pub repetition_time: f64,
    /// Share of the hammer speed left for a re-strike right at the release
    /// (smoothstep up to 1 over `repetition_time`): the hammer is thrown from
    /// closer and slower, never not at all.
    pub repetition_floor: f64,
    /// Output level (dB): close microphones hear the piano louder than
    /// distant ones; presets set it to keep a comparable loudness.
    pub output_gain_db: f64,
    /// Multiplies Stulov's Q0 (felt hardness / voicing).
    pub hardness: f64,
    /// Extra hardness growth per key around C4: Q0 ×= e^{slope·(n − 40)}
    /// (voicing; Stulov's fit grows as e^{0.045·n}).
    pub hardness_slope: f64,
    /// The same slope below key 40 (C4): bass hammers are large and soft.
    pub hardness_slope_bass: f64,
    /// Further growth above C6 (key 64): Q0 ×= e^{slope·(n − 64)}. With the
    /// flexible shank the top's fundamentals gain more than its partials
    /// 2–4; the top hammers are voiced harder (docs/top_level.md).
    pub hardness_slope_top: f64,
    /// Inner felt: Q_core·(u − depth)^core_p added to the top felt's
    /// Q0·u^p, Q_core relative to Stulov's Q0 of the key (0: single layer),
    /// with the depth (mm) of the top felt in the bass (A0), the middle
    /// (C4) and the treble (C8), linear in between. With depth 0 and
    /// core_p above p, the felt is graded: it stiffens continuously as it
    /// is compressed, as a felt whose density grows towards the core.
    pub core_hardness: f64,
    pub core_p: f64,
    pub core_depth_bass: f64,
    pub core_depth_mid: f64,
    pub core_depth_treble: f64,
    /// Extra felt exponent in the bass, growing linearly from C4 to A0:
    /// bass hammers have a soft surface over a very hard core, round at pp,
    /// biting at ff.
    pub p_bass: f64,
    /// Added to Stulov's exponent p (Russell: 2–4, Stulov: 3.7–5) of the
    /// top felt; the layered hammer is modelled by the core below.
    pub p_offset: f64,
    /// Multiplies Stulov's hysteresis time α.
    pub hysteresis: f64,
    /// Multiplies the felt contact width (spatial low-pass of the strike).
    pub contact_width_scale: f64,
    /// Multiplies Stulov's hammer masses (light leather-covered fortepiano
    /// hammers, small upright hammers).
    pub hammer_mass_scale: f64,
    /// The shank bends: of the hammer's effective mass, `shank_share` is
    /// the shank's, linked to the head by a spring whose mode (head against
    /// shank) is at `shank_freq` (Hz), with loss factor `shank_loss`. A
    /// contact much shorter than that mode's half period (the treble) meets
    /// the head alone; a long one (the bass) the whole hammer. 0: rigid.
    /// 0.3 and 300 Hz: the top's attack comes within 1 dB of eight real
    /// grands' against the middle (−10.6 dB before, docs/top_level.md).
    pub shank_share: f64,
    pub shank_freq: f64,
    pub shank_loss: f64,
    /// Strike point (fraction of the speaking length from the agraffe) up
    /// to C6, 0.05 closer to the agraffe at C8.
    pub strike_ratio: f64,
}

impl Default for PianoModel {
    fn default() -> Self {
        Self {
            a4: 440.0,
            velocity: VelocityLaw::DEFAULT,
            inharmonicity: InharmonicityLaw::SALAMANDER_C5,
            stretch: StretchLaw::SALAMANDER_C5,
            losses: LossLaw::SALAMANDER_STRING,
            scale: StringScale::GRAND_2M,
            unison: UnisonLaw::GRAND,
            bridge: BridgeLaw::SALAMANDER_C5,
            polarization: PolarizationLaw::DEFAULT,
            soundboard: Some(crate::soundboard::SoundboardParams::GRAND_2M),
            damper: DamperLaw::GRAND,
            sympathetic: SympatheticLaw::DEFAULT,
            nonlinear: StringNonlinearity::DEFAULT,
            noise: NoiseLaw::DEFAULT,
            room: crate::room::RoomParams::CONCERT_HALL,
            frame: crate::structure::FrameParams::GRAND_2M,
            rim: crate::structure::RimParams::GRAND_2M,
            una_corda_softening: 0.6,
            una_corda_shift: 1.0,
            half_blow: 0.0,
            mute_felt: 0.0,
            repetition_time: 0.0,
            repetition_floor: 0.5,
            output_gain_db: 0.0,
            // Top felt 0.3 × Stulov, slightly harder towards the treble, with
            // the core shallower at C4 and less hysteresis: the middle's ff
            // goes further into the core with a steeper pulse (C4 ff: 44 N,
            // 1.9 ms). 0.2 left the middle dull at 2–8 kHz
            // (docs/middle_highs.md).
            hardness: 0.3,
            hardness_slope: 0.02,
            hardness_slope_bass: 0.02,
            hardness_slope_top: 0.2,
            p_bass: 0.0,
            // Graded felt (docs/felt.md): Q0·u^p + 2·Q0_Stulov·u^7, no
            // threshold. The former two-layer hammer (a core 30 × Stulov,
            // 0.8/0.45/0.4 mm under the top felt, docs/presence.md) switched
            // the mf attack between too dark and too bright within 0.03 mm
            // of depth; scored against eight real grands (tools/bench.py)
            // the graded felt gives 1.58 against 1.66, the mf attack of the
            // bass within the real pianos (4–8 kHz share: z −4.5 → −0.5).
            core_hardness: 2.0,
            core_p: 7.0,
            core_depth_bass: 0.0,
            core_depth_mid: 0.0,
            core_depth_treble: 0.0,
            p_offset: 0.0,
            hysteresis: 0.3,
            contact_width_scale: 1.0,
            hammer_mass_scale: 1.0,
            shank_share: 0.3,
            shank_freq: 300.0,
            shank_loss: 0.1,
            strike_ratio: 0.12,
        }
    }
}

impl PianoModel {
    pub fn note(&self, note: u8) -> NoteParams {
        let f0 = note_freq(note, self.a4) * 2f64.powf(self.stretch.cents(note) / 1200.0);
        let length = self.scale.length(note);
        let tension = self.scale.tension;
        let mu = tension / (2.0 * length * f0).powi(2);
        // ≈1/8 of the length up to C6, closer to the agraffe in the high treble.
        let m = note as f64;
        let strike_pos = if m <= 84.0 { self.strike_ratio } else { self.strike_ratio - 0.05 * (m - 84.0) / 24.0 };
        let contact_width = (0.008 - 0.004 * (m - 21.0) / 87.0) * self.contact_width_scale;
        let mut hammer = hammer_stulov(note);
        hammer.mass_kg *= self.hammer_mass_scale;
        if self.shank_share > 0.0 && self.shank_freq > 0.0 {
            let total = hammer.mass_kg;
            let share = self.shank_share.clamp(0.0, 0.9);
            hammer.mass_kg = total * (1.0 - share);
            hammer.shank_mass = total * share;
            let reduced = hammer.mass_kg * hammer.shank_mass / total;
            let w = std::f64::consts::TAU * self.shank_freq;
            hammer.shank_k = reduced * w * w;
            hammer.shank_c = self.shank_loss * reduced * w;
        }
        let key = note as f64 - 20.0;
        let slope = if key < 40.0 { self.hardness_slope_bass } else { self.hardness_slope };
        // The core keeps Stulov's stiffness of the key, whatever the voicing
        // of the top felt.
        let base_q0 = hammer.q0;
        hammer.q0 *= self.hardness * (slope * (key - 40.0) + self.hardness_slope_top * (key - 64.0).max(0.0)).exp();
        hammer.p += self.p_offset + self.p_bass * ((40.0 - key) / 39.0).max(0.0);
        hammer.core_q = self.core_hardness * base_q0;
        hammer.core_p = self.core_p;
        let depth = if m <= 60.0 {
            let t = ((m - 21.0) / 39.0).clamp(0.0, 1.0);
            self.core_depth_bass + (self.core_depth_mid - self.core_depth_bass) * t
        } else {
            let t = ((m - 60.0) / 48.0).clamp(0.0, 1.0);
            self.core_depth_mid + (self.core_depth_treble - self.core_depth_mid) * t
        };
        hammer.core_depth = depth * 1e-3;
        hammer.alpha *= self.hysteresis;
        NoteParams {
            f0,
            b: self.inharmonicity.b(note),
            length,
            tension,
            mu,
            strike_pos,
            contact_width,
            hammer,
            strings: self.unison.strings(note),
            detune: self.unison.detune(note),
        }
    }
}

/// Hammer felt parameters for a key, from Stulov's fits over keys 1..88
/// (Stulov, ICA 2009). Static law F = Q0·u^p with u in mm.
#[derive(Clone, Copy, Debug)]
pub struct HammerParams {
    pub mass_kg: f64,
    pub p: f64,
    /// Stiffness Q0 in N/mm^p.
    pub q0: f64,
    /// Hysteresis time α (s) of F = Q0·[u^p + α·d(u^p)/dt].
    pub alpha: f64,
    /// Hard core under the top felt: once the felt is compressed by
    /// `core_depth` (m), it adds Q_core·(u − depth)^p_core (Q_core in
    /// N/mm^p_core). Q_core = 0: single-layer felt.
    pub core_q: f64,
    pub core_p: f64,
    pub core_depth: f64,
    /// Flexible shank: its effective mass (kg, 0: rigid hammer of
    /// `mass_kg`), and the stiffness (N/m) and damping (N·s/m) linking it to
    /// the head (`mass_kg`).
    pub shank_mass: f64,
    pub shank_k: f64,
    pub shank_c: f64,
}

pub fn hammer_stulov(note: u8) -> HammerParams {
    let n = (note as f64 - 20.0).clamp(1.0, 88.0);
    HammerParams {
        shank_mass: 0.0,
        shank_k: 0.0,
        shank_c: 0.0,
        core_q: 0.0,
        core_p: 2.5,
        core_depth: 1e-3,
        mass_kg: (11.074 - 0.074 * n + 1e-4 * n * n) * 1e-3,
        p: 3.7 + 0.015 * n,
        q0: 183.0 * (0.045 * n).exp(),
        // The published quadratic fit (JOS PASP, "Including Hysteresis")
        // turns negative above key ~85; floor it at 30 µs.
        alpha: (248.0 + 1.83 * n - 0.055 * n * n).max(30.0) * 1e-6,
    }
}

impl HammerParams {
    /// Stiffness in SI units (N/m^p), for u in metres.
    pub fn q_si(&self) -> f64 {
        self.q0 * 1000f64.powf(self.p)
    }

    /// Maximum felt compression (m) against a rigid surface at speed `v` (m/s).
    pub fn max_compression_rigid(&self, v: f64) -> f64 {
        // Energy balance m·V²/2 = Q0·u^(p+1)/(p+1), with Q0 in N/mm^p:
        // N·m / (N/mm^p) = 1000·mm^(p+1).
        let u_mm = (1000.0 * (self.p + 1.0) * self.mass_kg * v * v / (2.0 * self.q0)).powf(1.0 / (self.p + 1.0));
        u_mm * 1e-3
    }

    /// Contact duration (s) against a rigid surface, closed form for the
    /// power law: t0 = 2√π·u_max/V · Γ(1+a)/Γ(a+½), a = 1/(p+1).
    pub fn contact_time_rigid(&self, v: f64) -> f64 {
        let a = 1.0 / (self.p + 1.0);
        let ratio = (ln_gamma(1.0 + a) - ln_gamma(a + 0.5)).exp();
        2.0 * std::f64::consts::PI.sqrt() * self.max_compression_rigid(v) / v * ratio
    }
}

/// Lanczos approximation (g = 7, n = 9), |error| < 1e-13 for x > 0.
pub fn ln_gamma(x: f64) -> f64 {
    const C: [f64; 9] = [
        0.999_999_999_999_809_9,
        676.520_368_121_885_1,
        -1_259.139_216_722_402_8,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_12,
        9.984_369_578_019_572e-6,
        1.505_632_735_149_311_6e-7,
    ];
    if x < 0.5 {
        let pi = std::f64::consts::PI;
        return (pi / (pi * x).sin()).ln() - ln_gamma(1.0 - x);
    }
    let x = x - 1.0;
    let mut s = C[0];
    for (i, c) in C.iter().enumerate().skip(1) {
        s += c / (x + i as f64);
    }
    let t = x + 7.5;
    0.5 * (2.0 * std::f64::consts::PI).ln() + (x + 0.5) * t.ln() - t + s.ln()
}

impl Default for InharmonicityLaw {
    fn default() -> Self {
        Self::SALAMANDER_C5
    }
}

impl Default for StretchLaw {
    fn default() -> Self {
        Self::SALAMANDER_C5
    }
}

impl Default for LossLaw {
    fn default() -> Self {
        Self::SALAMANDER_STRING
    }
}

impl Default for StringScale {
    fn default() -> Self {
        Self::GRAND_2M
    }
}

impl Default for UnisonLaw {
    fn default() -> Self {
        Self::GRAND
    }
}

impl Default for BridgeLaw {
    fn default() -> Self {
        Self::SALAMANDER_C5
    }
}

impl Default for PolarizationLaw {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl Default for DamperLaw {
    fn default() -> Self {
        Self::GRAND
    }
}

impl Default for NoiseLaw {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl Default for SympatheticLaw {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl Default for VelocityLaw {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rigaud_b_values() {
        let a0 = inharmonicity_rigaud(21);
        assert!((a0 - 5.1e-4).abs() < 0.2e-4, "{a0}");
        let c8 = inharmonicity_rigaud(108);
        assert!((c8 - 2.6e-2).abs() < 0.2e-2, "{c8}");
    }

    #[test]
    fn salamander_laws_match_survey() {
        let b = InharmonicityLaw::SALAMANDER_C5;
        assert!((b.b(60) - 2.9e-4).abs() < 0.3e-4, "{}", b.b(60));
        assert!((StretchLaw::SALAMANDER_C5.cents(21) + 16.1).abs() < 0.2);
        let m = PianoModel::default().note(60);
        assert!(m.mu > 3e-3 && m.mu < 1e-2, "C4 linear mass {}", m.mu);
        let a0 = PianoModel::default().note(21);
        assert!(a0.mu > 0.05 && a0.mu < 0.3, "A0 linear mass {}", a0.mu);
    }

    #[test]
    fn gamma_known_values() {
        assert!((ln_gamma(0.5).exp() - std::f64::consts::PI.sqrt()).abs() < 1e-12);
        assert!((ln_gamma(5.0).exp() - 24.0).abs() < 1e-10);
    }

    #[test]
    fn stulov_contact_time_matches_paper() {
        // Stulov reports ≈1.7 ms for key 1 at 1 m/s.
        let t = hammer_stulov(21).contact_time_rigid(1.0);
        assert!((t - 1.7e-3).abs() < 0.1e-3, "{t}");
        // Contact shortens with speed and up the keyboard.
        assert!(hammer_stulov(60).contact_time_rigid(3.0) < t);
        assert!(hammer_stulov(60).contact_time_rigid(5.0) < hammer_stulov(60).contact_time_rigid(1.0));
    }
}