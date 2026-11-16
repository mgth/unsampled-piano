//! The front duplex: a string that runs on under the capo bar to its
//! bearing bar.
//!
//! Under a capo bar the string is held down on an edge: its displacement is
//! zero there, but the wire is one piece and stiff, so its slope and its
//! bending moment carry through to the short length in front of the bar
//! (34 to 48 mm on a concert grand), which ends on the bearing bar. The two
//! lengths are one stiff string on three supports:
//!
//! ```text
//!   EI·y'''' − T·y'' = μ·ω²·y   on each span,
//!   bridge (x = 0) and bearing bar: y = 0, y'' = 0,
//!   capo (x = L): y = 0 on both sides, y' and y'' continuous.
//! ```
//!
//! With k the wavenumber of the travelling part and κ² = k² + T/EI that of
//! the part that dies away from the supports, the modes are the roots of
//!
//! ```text
//!   k·(cot kL + cot kl) = κ·(coth κL + coth κl).
//! ```
//!
//! A wave of the speaking length passes the capo with an amplitude
//! k/√(k² + κ²): a twentieth at 1 kHz, a quarter at 5 kHz on a top string
//! (the capo's own motion passes ten thousand times less: the frame's
//! mobility there against the string's wave impedance). So the speaking
//! length's partials keep nearly all of their motion on their side, a little
//! higher than between two plain supports, and each mode of the short
//! length has a share of its motion in the speaking length, by which the
//! hammer rings it and the bridge hears it: the larger the nearer it falls
//! to a partial, where the two share the motion and push each other apart.
//!
//! [`modes`] gives, for one string, its partials as that string has them
//! (frequency re the first, shape at the hammer and at the bridge, share of
//! the motion in the speaking length) and the modes of the short length the
//! same way. The first partial is the one the tuner sets: every frequency is
//! given re it. docs/capo_duplex.md.

use crate::hammer::ModeShape;
use std::f64::consts::PI;

/// One mode of the string on its three supports.
#[derive(Clone, Copy, Debug)]
pub struct DuplexMode {
    /// Frequency re the string's first partial.
    pub ratio: f64,
    /// Its shape as the hammer and the bridge take it, for the modal mass
    /// μL/2 of a plain partial.
    pub shape: ModeShape,
    /// Share of its motion (of its modal mass) in the speaking length.
    pub speaking: f64,
}

/// The modes of one string, see [`modes`].
#[derive(Clone, Debug, Default)]
pub struct StringModes {
    /// The speaking length's partials from the first; None where none was
    /// found (the plain partial then).
    pub partials: Vec<Option<DuplexMode>>,
    /// The short length's modes, ascending.
    pub duplex: Vec<DuplexMode>,
}

/// Step of the search for the modes (first partials) and steps of the
/// shapes' integrals.
const SCAN: f64 = 0.002;
const QUAD: usize = 256;

/// The modes below `f_max` re the first partial (a ratio) of a string of
/// speaking length `length` (m), inharmonicity `b` (its plain partials at
/// k·f0·√(1 + b·k²)), struck at `strike_pos`·L from the capo, with `front`
/// metres between the capo and its bearing bar. The tension and the linear
/// mass only set the scale: the modes depend on b, on the two lengths and
/// on the strike point.
pub fn modes(length: f64, b: f64, strike_pos: f64, front: f64, f_max: f64) -> StringModes {
    let mut out = StringModes::default();
    if !(length > 0.0 && front > 0.0 && b > 0.0) {
        return out;
    }
    // Units: L = 1, T = 1, μ = 1 (a plain string's partial k at
    // k/2·√(1 + b·k²) "Hz"); EI = b/π².
    let ei = b / (PI * PI);
    let l = front / length;
    let k_of = |f: f64| {
        let w2 = (2.0 * PI * f).powi(2);
        ((-1.0 + (1.0 + 4.0 * ei * w2).sqrt()) / (2.0 * ei)).sqrt()
    };
    let kappa = |k: f64| (k * k + 1.0 / ei).sqrt();
    // The characteristic equation times sin kL · sin kl (no poles).
    let g = |f: f64| {
        let k = k_of(f);
        let ka = kappa(k);
        k * (k * (1.0 + l)).sin() - ka * (1.0 / ka.tanh() + 1.0 / (ka * l).tanh()) * k.sin() * (k * l).sin()
    };
    let plain = |k: u32| 0.5 * k as f64 * (1.0 + b * (k * k) as f64).sqrt();
    // Every mode up to f_max (re the plain first partial, a little over: the
    // real first partial is up to 4 % higher, C8 with 120 mm in front).
    let top = f_max * plain(1) * 1.1;
    let step = SCAN * plain(1);
    let mut found: Vec<(f64, DuplexMode)> = Vec::new();
    let (mut f0, mut g0) = (step, g(step));
    while f0 < top {
        let f1 = f0 + step;
        let g1 = g(f1);
        if g0 == 0.0 || g0 * g1 < 0.0 {
            let (mut a, mut c, mut ga) = (f0, f1, g0);
            for _ in 0..60 {
                let m = 0.5 * (a + c);
                let gm = g(m);
                if ga * gm <= 0.0 {
                    c = m;
                } else {
                    (a, ga) = (m, gm);
                }
            }
            let f = 0.5 * (a + c);
            found.push((f, shape(k_of(f), kappa(k_of(f)), l, strike_pos)));
        }
        (f0, g0) = (f1, g1);
    }
    // Partial k: of the modes near `at` (none taken yet), the one with most
    // of its motion in the speaking length, if it has more than half of it.
    let mut taken = vec![false; found.len()];
    let partial = |at: f64, near: f64, taken: &mut [bool]| {
        let best = (0..found.len())
            .filter(|&i| !taken[i] && (found[i].0 - at).abs() < near)
            .max_by(|&i, &j| found[i].1.speaking.total_cmp(&found[j].1.speaking))
            .filter(|&i| found[i].1.speaking > 0.5)?;
        taken[best] = true;
        Some(best)
    };
    // The first is the one the tuner sets, near the plain string's first:
    // not the lowest mode, which is the short length's once it is the
    // longer of the two.
    let Some(i1) = partial(plain(1), 0.45 * (plain(2) - plain(1)), &mut taken) else { return out };
    let first = found[i1].0;
    out.partials.push(Some(DuplexMode { ratio: 1.0, ..found[i1].1 }));
    // Each further partial near its place re the first; what is left is
    // the short length's.
    let mut k = 2u32;
    loop {
        let at = first * plain(k) / plain(1);
        if at > first * f_max {
            break;
        }
        let near = 0.45 * (plain(k + 1) - plain(k)) / plain(1) * first;
        out.partials.push(partial(at, near, &mut taken).map(|i| DuplexMode { ratio: found[i].0 / first, ..found[i].1 }));
        k += 1;
    }
    out.duplex = (0..found.len())
        .filter(|&i| !taken[i] && found[i].0 < first * f_max)
        .map(|i| DuplexMode { ratio: found[i].0 / first, ..found[i].1 })
        .collect();
    out
}

/// The mode of wavenumber k (L = 1): its shape and its share in the
/// speaking length. On the speaking length, from the bridge,
/// y = A·(sin kx − sin k·sinh κx / sinh κ); on the short one, from the
/// bearing bar, y = C·(sin ks − sin kl·sinh κs / sinh κl), with
/// A·sin k = C·sin kl (the bending moment at the capo).
fn shape(k: f64, ka: f64, l: f64, strike_pos: f64) -> DuplexMode {
    let (sl, sf) = (k.sin(), (k * l).sin());
    // (A, C) ∝ (sin kl, sin kL), the bridge's slope positive.
    let norm = sl.hypot(sf).max(1e-300);
    let (a, c) = if sf >= 0.0 { (sf / norm, sl / norm) } else { (-sf / norm, -sl / norm) };
    // sinh κx / sinh κX, stable for large κX.
    let sh = |x: f64, len: f64| ((ka * (x - len)).exp() * (1.0 - (-2.0 * ka * x).exp()) / (1.0 - (-2.0 * ka * len).exp())).max(0.0);
    let y1 = |x: f64| a * ((k * x).sin() - sl * sh(x, 1.0));
    let y2 = |s: f64| c * ((k * s).sin() - sf * sh(s, l));
    let simpson = |f: &dyn Fn(f64) -> f64, len: f64| {
        let h = len / QUAD as f64;
        (0..=QUAD).map(|i| f(i as f64 * h).powi(2) * if i == 0 || i == QUAD { 1.0 } else if i % 2 == 1 { 4.0 } else { 2.0 }).sum::<f64>() * h / 3.0
    };
    let (m1, m2) = (simpson(&y1, 1.0), simpson(&y2, l));
    // For a plain partial's modal mass, ∫ sin² = L/2.
    let scale = (0.5 / (m1 + m2)).sqrt();
    DuplexMode {
        ratio: 1.0,
        shape: ModeShape { nu: k / PI, strike: scale * y1(1.0 - strike_pos), bridge: scale * a * k / PI },
        speaking: m1 / (m1 + m2),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // C7 on a concert grand: 94 mm, b 0.0077, struck at 0.09, 37 mm in front.
    fn c7() -> StringModes {
        modes(0.0944, 0.0077, 0.09, 0.0368, 9.5)
    }

    #[test]
    fn the_partials_stay_nearly_plain_and_the_short_length_adds_its_modes() {
        let m = c7();
        assert!(m.partials.len() >= 6 && m.partials.iter().all(|p| p.is_some()));
        let plain = |k: u32| k as f64 * ((1.0 + 0.0077 * (k * k) as f64) / 1.0077).sqrt();
        for (i, p) in m.partials.iter().enumerate() {
            let (k, p) = (i as u32 + 1, p.unwrap());
            // Within a third of a semitone of the plain partial, most of
            // its motion in the speaking length, its shape near sin(kπx/L).
            assert!((1200.0 * (p.ratio / plain(k)).log2()).abs() < 35.0, "partial {k}: {} against {}", p.ratio, plain(k));
            assert!(p.speaking > 0.5 && p.speaking <= 1.0);
            assert!((p.shape.nu - k as f64).abs() < 0.2 && p.shape.bridge > 0.0);
        }
        assert_eq!(m.partials[0].unwrap().ratio, 1.0);
        // Three modes of the short length below 20 kHz, the first near
        // c/2l·(1 + stiffness), with a few percent of its motion in the
        // speaking length.
        assert_eq!(m.duplex.len(), 3);
        let d = m.duplex[0];
        let wave = 0.0944 / 0.0368;
        assert!(d.ratio > wave && d.ratio < 1.15 * wave, "{}", d.ratio);
        assert!(d.speaking > 0.005 && d.speaking < 0.2, "{}", d.speaking);
    }

    #[test]
    fn the_shares_add_up() {
        // As many modes as the two lengths have apart, and their shares in
        // the speaking length add up to the speaking length's own count.
        let m = c7();
        let total: f64 = m.partials.iter().flatten().chain(&m.duplex).map(|p| p.speaking).sum();
        assert!((total - m.partials.len() as f64).abs() < 0.6, "{total} for {} partials", m.partials.len());
    }

    #[test]
    fn a_mode_of_the_short_length_on_a_partial_shares_it() {
        // G7 with 34 mm in front: the short length's first mode falls on
        // the second partial, and the two share the motion.
        let m = modes(0.0647, 0.0142, 0.07, 0.034, 5.0);
        let (p2, d) = (m.partials[1].unwrap(), m.duplex[0]);
        assert!(d.speaking > 0.15 && p2.speaking < 0.85, "{} {}", d.speaking, p2.speaking);
        assert!((p2.speaking + d.speaking - 1.0).abs() < 0.1);
    }

    #[test]
    fn a_front_length_longer_than_the_speaking_one_keeps_the_tuned_first() {
        // C8 on a concert grand, 49 mm, with 60 and 120 mm in front (inside
        // the controls' range): the short length's first modes fall below
        // the speaking length's first partial, which stays the tuned one.
        for front in [0.06, 0.12] {
            let m = modes(0.0494, 0.023, 0.07, front, 5.0);
            assert!(m.partials.len() >= 4 && m.partials.iter().all(|p| p.is_some()));
            assert_eq!(m.partials[0].unwrap().ratio, 1.0);
            let plain = |k: u32| k as f64 * ((1.0 + 0.023 * (k * k) as f64) / 1.023).sqrt();
            for (i, p) in m.partials.iter().enumerate() {
                let (k, p) = (i as u32 + 1, p.unwrap());
                assert!(p.speaking > 0.5, "partial {k}: {} in the speaking length", p.speaking);
                assert!((1200.0 * (p.ratio / plain(k)).log2()).abs() < 35.0, "partial {k} with {front} m in front: {} against {}", p.ratio, plain(k));
            }
            let below = m.duplex.iter().take_while(|d| d.ratio < 1.0).count();
            assert!(below >= 1 && m.duplex[..below].iter().all(|d| d.speaking < 0.05), "{:?}", m.duplex);
        }
    }

    #[test]
    fn nothing_without_a_front_length() {
        assert!(modes(0.1, 0.005, 0.1, 0.0, 10.0).partials.is_empty());
    }
}
