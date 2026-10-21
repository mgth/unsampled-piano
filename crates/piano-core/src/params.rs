//! Default per-note physical parameters.
//!
//! Only the inharmonicity law is taken from the literature; the loss and
//! excitation values are placeholders to be replaced by fitted data
//! (Phase 1 / Phase 5 of the plan).

/// Fundamental of a MIDI note in equal temperament, A4 = `a4` Hz.
pub fn note_freq(note: u8, a4: f64) -> f64 {
    a4 * 2f64.powf((note as f64 - 69.0) / 12.0)
}

/// Inharmonicity coefficient B across the keyboard: sum of a bass and a
/// treble exponential (Rigaud, David & Daudet, JASA 2013), with the fitted
/// constants quoted by Renault et al. (DAFx 2022). B ≈ 5e-4 at A0, minimum
/// ≈ 1.4e-4 around MIDI 44, ≈ 2.6e-2 at C8.
pub fn inharmonicity_rigaud(note: u8) -> f64 {
    let m = note as f64;
    (-0.0847 * m - 5.82).exp() + (0.0926 * m - 13.64).exp()
}

/// Partial frequency of a stiff string: f_k = k·f0·sqrt(1 + B·k²).
pub fn partial_freq(f0: f64, b: f64, k: u32) -> f64 {
    let k = k as f64;
    k * f0 * (1.0 + b * k * k).sqrt()
}

/// Placeholder loss law σ_k = b1(note) + b2·f_k², in 1/s.
/// Gives T60 ≈ 28 s for A0's fundamental and ≈ 0.9 s at C8. Not fitted.
pub fn decay_rate_placeholder(note: u8, freq_hz: f64) -> f64 {
    let b1 = 0.25 * (0.04 * (note as f64 - 21.0)).exp();
    let b2 = 8e-7;
    b1 + b2 * freq_hz * freq_hz
}

/// Hammer felt parameters for a key, from Stulov's fits over keys 1..88
/// (Stulov, ICA 2009). Static law F = Q0·u^p with u in mm.
#[derive(Clone, Copy, Debug)]
pub struct HammerParams {
    pub mass_kg: f64,
    pub p: f64,
    /// Stiffness Q0 in N/mm^p.
    pub q0: f64,
}

pub fn hammer_stulov(note: u8) -> HammerParams {
    let n = (note as f64 - 20.0).clamp(1.0, 88.0);
    HammerParams {
        mass_kg: (11.074 - 0.074 * n + 1e-4 * n * n) * 1e-3,
        p: 3.7 + 0.015 * n,
        q0: 183.0 * (0.045 * n).exp(),
    }
}

impl HammerParams {
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
