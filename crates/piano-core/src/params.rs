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

/// Decay rate σ_k = b1(note) + b2·f_k² (1/s) with log b1 linear in the note.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LossLaw {
    pub b1_slope: f64,
    pub b1_offset: f64,
    pub b2: f64,
}

impl LossLaw {
    /// Early ("prompt") decay of single partials, Salamander survey. A single
    /// string per note cannot reproduce the two-stage decay (Phase 2).
    pub const SALAMANDER_EARLY: Self = Self { b1_slope: 0.0493, b1_offset: -1.737, b2: 2.26e-7 };
    /// Intrinsic string losses from the ESPRIT aftersound (no bridge losses).
    pub const SALAMANDER_STRING: Self = Self { b1_slope: 0.0590, b1_offset: -3.983, b2: 6.7e-8 };

    pub fn sigma(&self, note: u8, freq: f64) -> f64 {
        (self.b1_slope * note as f64 + self.b1_offset).exp() + self.b2 * freq * freq
    }
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
    /// Striking point relative to the string length.
    pub strike_pos: f64,
    /// Width of the felt contact patch (m).
    pub contact_width: f64,
    pub hammer: HammerParams,
}

/// The whole instrument description: laws evaluated per key.
#[derive(Clone, Copy, Debug)]
pub struct PianoModel {
    pub a4: f64,
    pub inharmonicity: InharmonicityLaw,
    pub stretch: StretchLaw,
    pub losses: LossLaw,
    pub scale: StringScale,
    /// Multiplies Stulov's Q0 (felt hardness / voicing).
    pub hardness: f64,
    /// Added to Stulov's exponent p (Russell: 2–4, Stulov: 3.7–5).
    pub p_offset: f64,
    /// Multiplies Stulov's hysteresis time α.
    pub hysteresis: f64,
}

impl Default for PianoModel {
    fn default() -> Self {
        Self {
            a4: 440.0,
            inharmonicity: InharmonicityLaw::SALAMANDER_C5,
            stretch: StretchLaw::SALAMANDER_C5,
            losses: LossLaw::SALAMANDER_EARLY,
            scale: StringScale::GRAND_2M,
            // Calibrated on Salamander (tools/grid_hammer.py): a less
            // nonlinear felt than Stulov's fit (p ≈ 2.7–4, Russell's range)
            // matches the brightness-vs-velocity curve 2.6× better.
            hardness: 2.0,
            p_offset: -1.0,
            hysteresis: 1.0,
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
        let strike_pos = if m <= 84.0 { 0.12 } else { 0.12 - 0.05 * (m - 84.0) / 24.0 };
        let contact_width = 0.008 - 0.004 * (m - 21.0) / 87.0;
        let mut hammer = hammer_stulov(note);
        hammer.q0 *= self.hardness;
        hammer.p += self.p_offset;
        hammer.alpha *= self.hysteresis;
        NoteParams { f0, b: self.inharmonicity.b(note), length, tension, mu, strike_pos, contact_width, hammer }
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
}

pub fn hammer_stulov(note: u8) -> HammerParams {
    let n = (note as f64 - 20.0).clamp(1.0, 88.0);
    HammerParams {
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
