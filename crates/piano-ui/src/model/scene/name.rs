//! The maker's name on a grand's fallboard, as makers inlay theirs: the
//! logo (assets/icon.svg: a string between its agraffe and its bridge, its
//! first two modes, an octave of keyboard) and "Unsampled" (Noto Serif Display,
//! [`lettering`]) in gold, standing a fraction of a millimetre proud of
//! the lacquer.

use super::lettering;
use super::*;

/// Gold leaf.
pub(super) const GILT: Color32 = Color32::from_rgb(214, 172, 84);
/// The word's size (m to the em: capitals some 2.1 cm high), the logo's
/// (m to its drawing's unit, 100 to its square) and the gap between them.
const EM: f64 = 0.03;
const LOGO: f64 = 0.00045;
const GAP: f64 = 0.012;
/// How far the gold stands proud of the wood; overlapping parts of the
/// logo a little more each, the one over the other.
const RAISE: f64 = 0.00025;
const STACK: f64 = 0.00005;

/// A flat piece of inlay from `points` (u across, v up, m), its `loops`
/// and its face's `tris` ([`Builder::prism`]), raised `h`; `at(u, v, h)`
/// places it, h out of the panel.
fn inlay(b: &mut Builder, points: &[(f64, f64)], loops: &[(usize, usize)], tris: &[[usize; 3]], h: f64, at: &dyn Fn(f64, f64, f64) -> V3) {
    b.prism(points, loops, tris, (0.0, h), at, GILT);
}

/// A simple polygon inlaid, raised `h`.
fn piece(b: &mut Builder, p: &[(f64, f64)], h: f64, at: &dyn Fn(f64, f64, f64) -> V3) {
    let mut p = p.to_vec();
    if signed_area(&p) < 0.0 {
        p.reverse();
    }
    inlay(b, &p, &[(0, p.len())], &triangulate(&p), h, at);
}

/// The logo's quadratic curve from `a` over `c` to `z` (its drawing's
/// units, y down), stroked `w` wide, its ends square (under the dots).
fn stroke(a: (f64, f64), c: (f64, f64), z: (f64, f64), w: f64) -> Vec<(f64, f64)> {
    const N: usize = 24;
    let at = |t: f64| {
        let u = 1.0 - t;
        (u * u * a.0 + 2.0 * u * t * c.0 + t * t * z.0, u * u * a.1 + 2.0 * u * t * c.1 + t * t * z.1)
    };
    let side = |t: f64| {
        let (d0, d1) = (2.0 * (1.0 - t) * (c.0 - a.0) + 2.0 * t * (z.0 - c.0), 2.0 * (1.0 - t) * (c.1 - a.1) + 2.0 * t * (z.1 - c.1));
        let l = d0.hypot(d1).max(1e-12);
        (-d1 / l * w / 2.0, d0 / l * w / 2.0)
    };
    let mut left: Vec<_> = (0..=N).map(|k| k as f64 / N as f64).map(|t| (at(t).0 + side(t).0, at(t).1 + side(t).1)).collect();
    let right = (0..=N).rev().map(|k| k as f64 / N as f64).map(|t| (at(t).0 - side(t).0, at(t).1 - side(t).1));
    left.extend(right);
    left
}

/// The name, centred on the panel `at` places (u across, v up, h out of
/// it, metres): the logo on the left, the word on the right, both centred
/// on the capitals' height.
pub(super) fn name(b: &mut Builder, at: &dyn Fn(f64, f64, f64) -> V3) {
    let word = lettering::WIDTH as f64 * EM;
    // The logo's drawing spans x 13–87 (its strokes and dots).
    let logo = 74.0 * LOGO;
    let left = -(logo + GAP + word) / 2.0;
    let mid = 0.35 * EM;
    // The drawing's units (y down, centred on 50, 50) to the panel.
    let lx = left + logo / 2.0;
    let svg = |(x, y): (f64, f64)| (lx + (x - 50.0) * LOGO, mid - (y - 50.0) * LOGO);
    let to = |p: Vec<(f64, f64)>| p.into_iter().map(svg).collect::<Vec<_>>();
    let mut h = RAISE;
    // The fundamental's envelope, the second mode over it.
    for (a, c, z, w) in [((18.0, 50.0), (50.0, 14.0), (82.0, 50.0), 2.0), ((18.0, 50.0), (50.0, 86.0), (82.0, 50.0), 2.0)] {
        piece(b, &to(stroke(a, c, z, w)), h, at);
    }
    h += STACK;
    for (a, c, z) in [((18.0, 50.0), (34.0, 30.0), (50.0, 50.0)), ((50.0, 50.0), (66.0, 70.0), (82.0, 50.0))] {
        piece(b, &to(stroke(a, c, z, 3.5)), h, at);
    }
    h += STACK;
    // The string's ends.
    for x in [18.0, 82.0] {
        let dot: Vec<_> = (0..24).map(|k| (std::f64::consts::TAU * k as f64 / 24.0).sin_cos()).map(|(s, c)| (x + 5.0 * c, 50.0 + 5.0 * s)).collect();
        piece(b, &to(dot), h, at);
    }
    // The keyboard, an octave: a bar, its black keys cut into its top,
    // two then three.
    let mut bar = vec![(18.0, 84.0), (82.0, 84.0), (82.0, 74.0)];
    for x in [70.4, 61.2, 52.1, 33.8, 24.6] {
        bar.extend([(x + 5.0, 74.0), (x + 5.0, 80.0), (x, 80.0), (x, 74.0)]);
    }
    bar.push((18.0, 74.0));
    piece(b, &to(bar), RAISE, at);
    // The word.
    let x0 = left + logo + GAP;
    let points: Vec<(f64, f64)> = lettering::POINTS.iter().map(|&(x, y)| (x0 + x as f64 * EM, y as f64 * EM)).collect();
    let loops: Vec<(usize, usize)> = lettering::LOOPS.iter().map(|&(s, n)| (s as usize, n as usize)).collect();
    let tris: Vec<[usize; 3]> = lettering::TRIS.iter().map(|t| t.map(|i| i as usize)).collect();
    inlay(b, &points, &loops, &tris, RAISE, at);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_name_stands_out_of_its_board() {
        // On a board sloping back, as the shut fallboard.
        let (dy, dz) = (0.9f64, 0.4f64);
        let l = dy.hypot(dz);
        let (dy, dz) = (dy / l, dz / l);
        let at = |u: f64, v: f64, h: f64| [u as f32, (dy * v - dz * h) as f32, (dz * v + dy * h) as f32];
        let out = [0.0, -dz as f32, dy as f32];
        let mut b = Builder::new(Place { vertical: false, top: 0.0 });
        name(&mut b, &at);
        let (mut faces, mut walls) = (0, 0);
        for t in b.s.tris.chunks(3) {
            let d = dot(t[0].normal, out);
            if d > 0.999 {
                faces += 1;
                // Every face just over the board.
                assert!(t.iter().all(|v| (dot(v.pos, out) as f64 - RAISE).abs() < 3.0 * STACK), "{t:?}");
            } else {
                assert!(d.abs() < 1e-3, "a wall leaning out of the board: {t:?}");
                walls += 1;
            }
        }
        assert!(faces > 1000 && walls > 2000, "{faces} {walls}");
        // As wide as the logo, the gap and the word.
        let (lo, hi) = b.bounds();
        assert!(((hi[0] - lo[0]) as f64 - (74.0 * LOGO + GAP + lettering::WIDTH as f64 * EM)).abs() < 0.004, "{lo:?} {hi:?}");
    }
}
