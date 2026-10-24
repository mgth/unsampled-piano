//! A string changing length while it vibrates (a new fret, a finger lifting
//! off): its displacement and velocity carried over to the modes of the new
//! length.
//!
//! The string runs from the bridge (x = 0) to the stop. Going shorter
//! (L₂ < L₁), the part beyond the new stop is cut off: it now vibrates on
//! the other side of the fret, behind the left hand, and is taken as
//! silent. Going longer, the added part was at rest (held behind the old
//! stop). In both cases, on the new modes sin(b_m·x), b_m = mπ/L₂:
//!
//! ```text
//! c_m = (2/L₂)·Σ_n q_n·I(n, m),   I(n, m) = ∫₀^{min(L₁, L₂)} sin(a_n x)·sin(b_m x) dx,   a_n = nπ/L₁
//! ```
//!
//! and the same for the velocity. Because one end of the interval is a node
//! of one of the two families, I has a closed form with a single sine:
//! (−1)^m·sin(a_n L₂)·b_m/(a_n² − b_m²) going shorter,
//! −(−1)^n·sin(b_m L₁)·a_n/(a_n² − b_m²) going longer, min(L₁, L₂)/2 when
//! a_n = b_m. So a projection costs N + M sines and N·M divisions.
//!
//! Exact for the sine shapes, which the stiff string's modes are to within
//! its inharmonicity. Instant: the shape's corner at the new stop gives the
//! broadband click of a string landing on a fret.

use piano_core::complex::C64;

use crate::string::StringTable;

/// Writes into `out[..to.len()]` the per-polarisation amplitudes on `to`
/// of the state `zeta` (one per partial of `from`). ζ = q − i·v/ω (q = Re ζ,
/// ζ ∝ e^{iωt}). Real-time safe.
pub fn project(from: &StringTable, to: &StringTable, zeta: &[[C64; 2]], out: &mut [[C64; 2]]) {
    let (l1, l2) = (from.length, to.length);
    let (nn, mm) = (from.len().min(zeta.len()), to.len());
    // Signs: (−1)^m and −(−1)^n with m, n counted from 1.
    // Accumulate displacement (re) and velocity (im) per polarisation.
    for o in &mut out[..mm] {
        *o = [C64::ZERO; 2];
    }
    let term = |a: f64, b: f64, num: f64| {
        if (a - b).abs() <= 1e-9 * a {
            l1.min(l2) / 2.0
        } else {
            num / (a * a - b * b)
        }
    };
    let add = |o: &mut [C64; 2], z: &[C64; 2], omega: f64, w: f64| {
        for p in 0..2 {
            o[p].re += w * z[p].re;
            o[p].im += w * (-omega * z[p].im);
        }
    };
    if l2 <= l1 {
        for (pn, z) in from.partials.iter().zip(zeta).take(nn) {
            let a = pn.k;
            let s = (a * l2).sin();
            for (m, (pm, o)) in to.partials.iter().zip(out.iter_mut()).enumerate() {
                let b = pm.k;
                let sign = if m % 2 == 0 { -1.0 } else { 1.0 };
                add(o, z, pn.omega, term(a, b, sign * s * b));
            }
        }
    } else {
        for (pm, o) in to.partials.iter().zip(out.iter_mut()) {
            let b = pm.k;
            let s = (b * l1).sin();
            for (n, (pn, z)) in from.partials.iter().zip(zeta).enumerate().take(nn) {
                let a = pn.k;
                let sign = if n % 2 == 0 { 1.0 } else { -1.0 };
                add(o, z, pn.omega, term(a, b, sign * s * a));
            }
        }
    }
    for (pm, o) in to.partials.iter().zip(out.iter_mut()) {
        for z in o.iter_mut() {
            // Back from (displacement, velocity) to ζ on the new mode.
            *z = C64::new(z.re * 2.0 / l2, -z.im * 2.0 / (l2 * pm.omega));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::GuitarModel;
    use crate::string::{fret_length, PAR, PERP};

    const SR: f64 = 48_000.0;

    fn at(m: &GuitarModel, s: usize, fret: u8) -> StringTable {
        let l = fret_length(m.scale_length, fret);
        StringTable::new(m, s, l, l - m.mute.distance, SR)
    }

    fn y(t: &StringTable, zeta: &[[C64; 2]], x: f64, pol: usize) -> f64 {
        t.partials.iter().zip(zeta).map(|(p, z)| (p.k * x).sin() * z[pol].re).sum()
    }

    #[test]
    fn same_length_is_the_identity() {
        let m = GuitarModel::default();
        let t = at(&m, 1, 3);
        let zeta: Vec<[C64; 2]> = (0..t.len()).map(|n| [C64::new(1.0 / (n + 1) as f64, 0.3), C64::new(-0.2, 0.1 / (n + 1) as f64)]).collect();
        let mut out = vec![[C64::ZERO; 2]; t.len()];
        project(&t, &t, &zeta, &mut out);
        for (a, b) in zeta.iter().zip(&out) {
            for p in 0..2 {
                assert!((a[p] - b[p]).norm() < 1e-12, "{a:?} → {b:?}");
            }
        }
    }

    #[test]
    fn octave_harmonic_lands_on_the_fundamental_at_the_12th_fret() {
        // Partial 2 of the open string has its node at the 12th fret: the
        // fretted string carries on as its own fundamental.
        let m = GuitarModel::default();
        let (open, oct) = (at(&m, 3, 0), at(&m, 3, 12));
        let mut zeta = vec![[C64::ZERO; 2]; open.len()];
        zeta[1] = [C64::new(1e-3, -2e-4), C64::new(0.0, 5e-4)];
        let mut out = vec![[C64::ZERO; 2]; oct.len()];
        project(&open, &oct, &zeta, &mut out);
        // Same displacement; same velocity (ω₂ open = ω₁ at the octave).
        assert!((out[0][PERP] - zeta[1][PERP]).norm() < 1e-12 && (out[0][PAR] - zeta[1][PAR]).norm() < 1e-12, "{:?}", out[0]);
        assert!(out[1..].iter().all(|o| o[0].norm() + o[1].norm() < 1e-12));
    }

    #[test]
    fn lifting_a_finger_keeps_the_shape_and_adds_a_still_part() {
        // Fretted at 5, plucked; lifted to the open length: the same shape
        // up to the old stop, nothing beyond (to the modal truncation).
        let m = GuitarModel::default();
        let (f5, open) = (at(&m, 0, 5), at(&m, 0, 0));
        let zeta: Vec<[C64; 2]> = f5.shape.iter().map(|a| [C64::new(a * 1e-3, 0.0), C64::new(a * 5e-4, 0.0)]).collect();
        let mut out = vec![[C64::ZERO; 2]; open.len()];
        project(&f5, &open, &zeta, &mut out);
        let l1 = f5.length;
        for x in [0.05, 0.12, 0.25, 0.4] {
            let (before, after) = (y(&f5, &zeta, x, PERP), y(&open, &out, x, PERP));
            assert!((before - after).abs() < 0.02 * 1e-3, "x = {x}: {before} vs {after}");
        }
        for x in [l1 + 0.03, l1 + 0.1] {
            assert!(y(&open, &out, x, PERP).abs() < 0.02 * 1e-3, "x = {x}: {}", y(&open, &out, x, PERP));
        }
    }
}
