//! The fingerboard's geometry: where each fret's crown lies under a string
//! stopped at a given length, and whether the string's motion can reach it.
//!
//! Heights are measured up from the board at the nut. The board sags by the
//! neck relief, a parabola between the nut and the body fret (flat beyond).
//! Each fret's crown stands `fret_height` above the board. The string runs
//! straight from the saddle to its stop (the nut, or the crown of the fret
//! pressed). The saddle height comes from the action: the open string's
//! height above the 12th fret's crown. The gap to each fret under the
//! vibrating part is the string's rest height there minus the crown's.
//!
//! The string's perpendicular displacement counts positive toward the
//! board, so a fret is an obstacle at +gap.
//!
//! Reach: a freely decaying string, y(x, t) = Σ Re(ζ_n·e^{(iω−σ)t})·sin(nπx/L),
//! never exceeds Σ |sin(nπx/L)|·|ζ_n| at x. Below a fret's gap, that fret
//! cannot be touched before something drives the string again: the string
//! stays in the modal bank, and only the frets within reach become contacts.

use crate::params::GuitarModel;
use crate::string::fret_length;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FretGap {
    pub fret: u8,
    /// Distance from the bridge (m).
    pub x: f64,
    /// String's rest height above the crown (m).
    pub gap: f64,
}

/// Board sag at `x` from the bridge (m, positive down).
fn sag(model: &GuitarModel, x: f64) -> f64 {
    let fb = &model.fretboard;
    let (l0, xb) = (model.scale_length, fret_length(model.scale_length, fb.body_fret));
    if x <= xb || x >= l0 {
        return 0.0;
    }
    let s = (l0 - x) / (l0 - xb);
    fb.relief * 4.0 * s * (1.0 - s)
}

/// Height of fret `j`'s crown.
pub fn crown(model: &GuitarModel, j: u8) -> f64 {
    model.fretboard.fret_height - sag(model, fret_length(model.scale_length, j))
}

/// Saddle height of `string`, from its action at the 12th fret.
fn saddle(model: &GuitarModel, string: usize) -> f64 {
    let fb = &model.fretboard;
    let n = model.strings.len().max(2) - 1;
    let action = fb.action_bass + (fb.action_treble - fb.action_bass) * (string.min(n) as f64 / n as f64);
    let z_nut = fb.fret_height + fb.nut_clearance;
    let r = fret_length(model.scale_length, 12) / model.scale_length;
    (crown(model, 12) + action - z_nut * r) / (1.0 - r)
}

/// The frets under `string` stopped at `stop` (None: open), with their gaps.
pub fn gaps(model: &GuitarModel, string: usize, stop: Option<u8>) -> Vec<FretGap> {
    let fb = &model.fretboard;
    let z_s = saddle(model, string);
    let (x_end, z_end, first) = match stop {
        None | Some(0) => (model.scale_length, fb.fret_height + fb.nut_clearance, 1),
        Some(f) => (fret_length(model.scale_length, f), crown(model, f), f + 1),
    };
    (first..=model.frets)
        .map(|j| {
            let x = fret_length(model.scale_length, j);
            let line = z_s + (z_end - z_s) * x / x_end;
            // A badly set-up neck could put the string on a fret: keep it
            // just above, buzzing.
            FretGap { fret: j, x, gap: (line - crown(model, j)).max(1e-5) }
        })
        .collect()
}

/// Slope of `string`'s rest line stopped at `stop` (height change per metre
/// from the saddle toward the stop; None: open).
pub fn rest_slope(model: &GuitarModel, string: usize, stop: Option<u8>) -> f64 {
    let fb = &model.fretboard;
    let z_s = saddle(model, string);
    let (x_end, z_end) = match stop {
        None | Some(0) => (model.scale_length, fb.fret_height + fb.nut_clearance),
        Some(f) => (fret_length(model.scale_length, f), crown(model, f)),
    };
    (z_end - z_s) / x_end
}

/// Upper bound of the perpendicular displacement at `x` of a string of
/// `length` decaying freely from amplitudes `amps` (|ζ_n|, n from 1).
pub fn reach(length: f64, x: f64, amps: impl Iterator<Item = f64>) -> f64 {
    let th = std::f64::consts::PI * x / length;
    let c2 = 2.0 * th.cos();
    // sin(nθ) by its recurrence.
    let (mut s0, mut s1) = (0.0, th.sin());
    let mut r = 0.0;
    for a in amps {
        r += s1.abs() * a;
        (s0, s1) = (s1, c2 * s1 - s0);
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_is_the_gap_at_the_12th_fret() {
        for m in [GuitarModel::steel_acoustic(), GuitarModel::electric_solid(), GuitarModel::electric_bass()] {
            let n = m.strings.len();
            for (s, action) in [(0, m.fretboard.action_bass), (n - 1, m.fretboard.action_treble)] {
                let g = gaps(&m, s, None);
                let twelfth = g.iter().find(|g| g.fret == 12).unwrap();
                assert!((twelfth.gap - action).abs() < 1e-9, "string {s}: {} vs {action}", twelfth.gap);
                // Open: the first fret sits just under the nut's height.
                assert!(g[0].gap < 0.8e-3 && g.iter().all(|g| g.gap > 1e-4), "{g:?}");
            }
        }
    }

    #[test]
    fn fretted_string_clears_the_frets_beyond_and_rises_toward_the_bridge() {
        let m = GuitarModel::electric_solid();
        let g = gaps(&m, 0, Some(5));
        assert_eq!(g.first().unwrap().fret, 6);
        assert_eq!(g.len(), (m.frets - 5) as usize);
        assert!(g.windows(2).all(|w| w[1].gap > w[0].gap), "{g:?}");
    }

    #[test]
    fn reach_bounds_the_displacement() {
        let l = 0.65;
        let amps: Vec<f64> = (1..=40).map(|n| 1.0 / (n * n) as f64).collect();
        for x in [0.05, 0.2, 0.4] {
            let bound = reach(l, x, amps.iter().copied());
            let direct: f64 = amps.iter().enumerate().map(|(n, a)| ((n + 1) as f64 * std::f64::consts::PI * x / l).sin().abs() * a).sum();
            assert!((bound - direct).abs() < 1e-12);
        }
    }
}
