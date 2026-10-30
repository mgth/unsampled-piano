//! The instrument as the model describes it, in three dimensions and to
//! scale: one list of triangles and lines that the plan (egui) and the 3D
//! view (GPU) both draw. Built from the model alone (static and editor
//! views) or with the engine's state laid on it (live view).
//!
//! World axes: x across the keyboard (treble at 0, bass towards −x), y
//! away from the pianist, z up; metres. A grand's board lies in z = 0, its
//! plate coordinates (x along the strings, y across, z above) turned into
//! (−y, x, z). An upright's board stands in y = 0, its x pointing down
//! from its top and its z towards the pianist.
//!
//! What the model has no geometry for is drawn plainly: the board and the
//! case are the model's rectangles (not a grand's curved outline); the
//! strings run from their key's place across the agraffes to their bridge
//! point, which crosses the bass over the tenor as on overstrung pianos;
//! legs, keyboard and pedals only situate the instrument.

use egui::Color32;
use piano_core::params::PianoModel;
use piano_core::soundboard::{Outline, SoundboardParams};
use piano_core::Observation;

use super::cam::{add, cross, dot, norm, scale, sub, V3};
use crate::sketches::{self, Part};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct Vertex {
    pub pos: V3,
    pub normal: V3,
    /// sRGB, 0–1, straight alpha.
    pub color: [f32; 4],
}

/// A named part: where its label goes, and the page that sets it.
#[derive(Clone, Debug, PartialEq)]
pub struct Label {
    pub at: V3,
    pub text: String,
    pub page: &'static str,
}

#[derive(Default)]
pub struct Scene {
    /// Opaque triangles, three vertices each.
    pub tris: Vec<Vertex>,
    /// Translucent ones (frame, lid), drawn last.
    pub glass: Vec<Vertex>,
    /// Lines, two vertices each.
    pub lines: Vec<Vertex>,
    pub labels: Vec<Label>,
    pub min: V3,
    pub max: V3,
    /// The instrument's own extent, without the microphones (what the 3D
    /// view frames).
    pub body: (V3, V3),
    /// The board stands (an upright).
    pub vertical: bool,
}

/// What the live view lays on the instrument: the engine's state, and the
/// levels the view derived from it (0 at rest … 1 loudest).
pub struct Live<'a> {
    pub obs: &'a Observation,
    /// Per key, its strings' level.
    pub keys: &'a [f32; 128],
    /// The board's motion on its grid ([`BOARD_GRID`]), row by row along y.
    pub board: Option<&'a [f32]>,
    /// Seconds, for the strings' (slowed, exaggerated) motion.
    pub time: f32,
}

#[derive(Clone, Copy)]
pub struct Options<'a> {
    pub key: u8,
    pub lit: &'a [Part],
    pub lid: bool,
    pub frame: bool,
    pub mics: bool,
    /// Text colour of the theme (the key shown).
    pub ink: Color32,
}

/// Board grid: points along x (the strings) and across.
pub const BOARD_GRID: (usize, usize) = (36, 28);

/// Bridge height above the board, the strings' height on it, and the bass
/// strings' rise over the others (they cross over them).
const BRIDGE_H: f64 = 0.03;
const STRING_Z: f64 = 0.035;
const BASS_RISE: f64 = 0.02;
/// A grand's hammer line in front of its board (half a hammer and a gap).
const HAMMER_CLEAR: f64 = 0.015;
/// The tuning pins (7 mm), one per string in two staggered rows in front
/// of the agraffes: the first row's distance, the rows' spacing, a key's
/// pins' spacing across (wider than its strings', which fan out to them).
const PIN_R: f64 = 0.0035;
const PIN_GAP: f64 = 0.03;
const PIN_ROW: f64 = 0.014;
const PIN_ACROSS: f64 = 0.0038;
/// The pin block's top (under the plate) and thickness; the case's front
/// panel and the belly rail's thickness.
const PIN_BLOCK_TOP: f64 = 0.02;
const PIN_BLOCK_T: f64 = 0.05;
const FRONT_T: f64 = 0.02;
const BELLY_RAIL: f64 = 0.05;
/// The keyboard: depth of the white keys. A grand's keybed, its underside
/// flush with the case's, its thickness and the keys' bottom above it; the
/// keys' top above the floor; the key slip's thickness.
const KEY_DEPTH: f64 = 0.15;
const KEYBED_T: f64 = 0.035;
const KEY_FRAME: f64 = 0.04;
const KEY_HEIGHT: f64 = 0.72;
const KEY_SLIP: f64 = 0.018;
/// An upright's board bottom, keyboard height and distance in front.
const UPRIGHT_BASE: f64 = 0.2;
const UPRIGHT_KEYS_Z: f64 = 0.72;
const UPRIGHT_KEYS_Y: f64 = -0.27;
/// The pedals' spacing across (m).
const PEDAL_PITCH: f32 = 0.07;

/// Bends up to this angle (degrees) are shaded as a curve, sharper ones as
/// corners.
const SMOOTH_DEG: f64 = 40.0;

const STEEL: Color32 = Color32::from_rgb(170, 172, 178);
const COPPER: Color32 = Color32::from_rgb(184, 115, 70);
const IRON: Color32 = Color32::from_rgb(190, 160, 90);
const BRASS: Color32 = Color32::from_rgb(222, 184, 98);
const PIN: Color32 = Color32::from_rgb(132, 134, 142);
const CASE: Color32 = Color32::from_rgb(40, 34, 32);
const IVORY: Color32 = Color32::from_rgb(236, 232, 222);
const EBONY: Color32 = Color32::from_rgb(28, 26, 26);
/// The strip of felt behind the keys.
const FELT: Color32 = Color32::from_rgb(122, 28, 36);
const MIC: [Color32; 2] = [Color32::from_rgb(70, 140, 220), Color32::from_rgb(220, 80, 90)];

fn rgba(c: Color32, a: f32) -> [f32; 4] {
    [c.r() as f32 / 255.0, c.g() as f32 / 255.0, c.b() as f32 / 255.0, a]
}

fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()))
}

/// The board's motion on its wood: lighter and warmer where it moves, a
/// glow rather than a flare (the low levels barely tint it).
pub fn heat(level: f32) -> Color32 {
    let l = level.clamp(0.0, 1.0);
    mix(sketches::WOOD.gamma_multiply(0.8), Color32::from_rgb(244, 200, 128), 0.75 * l * l)
}

/// Plate coordinates to world.
#[derive(Clone, Copy)]
struct Place {
    vertical: bool,
    top: f64,
}

impl Place {
    /// A plate direction in world coordinates.
    fn dir(&self, x: f64, y: f64, z: f64) -> V3 {
        norm(sub(self.at(x, y, z), self.at(0.0, 0.0, 0.0)))
    }

    fn at(&self, x: f64, y: f64, z: f64) -> V3 {
        if self.vertical { [-y as f32, -z as f32, (self.top - x) as f32] } else { [-y as f32, x as f32, z as f32] }
    }
}

struct Builder {
    s: Scene,
    place: Place,
}

impl Builder {
    fn tri(&mut self, glass: bool, p: [V3; 3], c: [[f32; 4]; 3]) {
        let n = norm(cross(sub(p[1], p[0]), sub(p[2], p[0])));
        let out = if glass { &mut self.s.glass } else { &mut self.s.tris };
        for (pos, color) in p.into_iter().zip(c) {
            out.push(Vertex { pos, normal: n, color });
        }
    }

    /// A quad with a normal at each corner (smooth shading).
    fn quad_n(&mut self, p: [V3; 4], n: [V3; 4], c: [f32; 4]) {
        for k in [0, 1, 2, 0, 2, 3] {
            self.s.tris.push(Vertex { pos: p[k], normal: n[k], color: c });
        }
    }

    /// A wall of constant section along the plate polyline `line`, between
    /// `a` and `b` metres to its right (outwards on a counter-clockwise
    /// outline), from `z0` up to each segment's `top`: one piece, mitred at
    /// the joins, shaded smooth where the line bends by less than
    /// [`SMOOTH_DEG`], with a step where two segments' tops differ.
    #[allow(clippy::too_many_arguments)]
    fn wall(&mut self, line: &[(f64, f64)], closed: bool, a: f64, b: f64, z0: f64, top: &dyn Fn(usize) -> f64, c: Color32) {
        let n = line.len();
        let segs = if closed { n } else { n - 1 };
        if n < 2 {
            return;
        }
        let c = rgba(c, 1.0);
        let right = |k: usize| {
            let (p, q) = (line[k], line[(k + 1) % n]);
            let (dx, dy) = (q.0 - p.0, q.1 - p.1);
            let l = dx.hypot(dy).max(1e-12);
            (dy / l, -dx / l)
        };
        // Per vertex: the segments on either side (None at an open end),
        // the mitre direction and its length factor, and whether the bend
        // is smooth.
        let joint = |i: usize| -> (Option<usize>, Option<usize>) {
            let prev = if i > 0 { Some(i - 1) } else if closed { Some(n - 1) } else { None };
            let next = if i < segs { Some(i) } else { None };
            (prev, next)
        };
        let mut mitre = Vec::with_capacity(n);
        for i in 0..n {
            let (prev, next) = joint(i);
            let (m, smooth) = match (prev.map(right), next.map(right)) {
                (Some(u), Some(v)) => {
                    let (mx, my) = (u.0 + v.0, u.1 + v.1);
                    let l = mx.hypot(my).max(1e-12);
                    let m = (mx / l, my / l);
                    let k = 1.0 / (m.0 * v.0 + m.1 * v.1).max(0.3);
                    ((m.0 * k, m.1 * k), u.0 * v.0 + u.1 * v.1 > SMOOTH_DEG.to_radians().cos())
                }
                (Some(u), None) | (None, Some(u)) => (u, false),
                (None, None) => ((0.0, 0.0), false),
            };
            mitre.push((m, smooth));
        }
        let at = |i: usize, off: f64| (line[i].0 + mitre[i].0.0 * off, line[i].1 + mitre[i].0.1 * off);
        let pl = self.place;
        let w = |p: (f64, f64), z: f64| pl.at(p.0, p.1, z);
        let (up, down) = (pl.dir(0.0, 0.0, 1.0), pl.dir(0.0, 0.0, -1.0));
        for k in 0..segs {
            let (i, j) = (k, (k + 1) % n);
            let h = top(k);
            let (li, lj, ri, rj) = (at(i, a), at(j, a), at(i, b), at(j, b));
            // The faces' own normal, or the mitre's at a smooth bend.
            let face = right(k);
            let side = |v: usize| if mitre[v].1 { mitre[v].0 } else { face };
            let (si, sj) = (side(i), side(j));
            let out = |s: (f64, f64)| pl.dir(s.0, s.1, 0.0);
            let inn = |s: (f64, f64)| pl.dir(-s.0, -s.1, 0.0);
            self.quad_n([w(li, h), w(lj, h), w(rj, h), w(ri, h)], [up; 4], c);
            self.quad_n([w(li, z0), w(ri, z0), w(rj, z0), w(lj, z0)], [down; 4], c);
            self.quad_n([w(ri, z0), w(ri, h), w(rj, h), w(rj, z0)], [out(si), out(si), out(sj), out(sj)], c);
            self.quad_n([w(li, z0), w(lj, z0), w(lj, h), w(li, h)], [inn(si), inn(si), inn(sj), inn(sj)], c);
        }
        // Steps between segments of different heights, and open ends.
        for i in 0..n {
            let (prev, next) = joint(i);
            let (lo, hi, along) = match (prev, next) {
                (Some(p), Some(q)) if (top(p) - top(q)).abs() > 1e-9 => {
                    let lower = if top(p) < top(q) { p } else { q };
                    let r = right(lower);
                    (top(p).min(top(q)), top(p).max(top(q)), (-r.1, r.0))
                }
                (Some(p), None) => (z0, top(p), (-right(p).1, right(p).0)),
                (None, Some(q)) => (z0, top(q), (-right(q).1, right(q).0)),
                _ => continue,
            };
            let d = pl.dir(along.0, along.1, 0.0);
            let (l, r) = (at(i, a), at(i, b));
            self.quad_n([w(l, lo), w(r, lo), w(r, hi), w(l, hi)], [d; 4], c);
        }
    }

    fn quad(&mut self, glass: bool, p: [V3; 4], c: [f32; 4]) {
        self.tri(glass, [p[0], p[1], p[2]], [c; 3]);
        self.tri(glass, [p[0], p[2], p[3]], [c; 3]);
    }

    /// A box from corner `o` along the edges `u`, `v`, `w` (world), its
    /// faces turned outwards.
    fn cuboid(&mut self, o: V3, u: V3, v: V3, w: V3, c: Color32) {
        let c = rgba(c, 1.0);
        let centre = add(o, scale(add(add(u, v), w), 0.5));
        for (a, b, d) in [(u, v, w), (v, w, u), (w, u, v)] {
            for far in [false, true] {
                let base = if far { add(o, d) } else { o };
                let mut q = [base, add(base, a), add(add(base, a), b), add(base, b)];
                let mid = add(base, scale(add(a, b), 0.5));
                if dot(cross(a, b), sub(mid, centre)) < 0.0 {
                    q.reverse();
                }
                self.quad(false, q, c);
            }
        }
    }

    /// A box between two plate points (x, y, z) corners, axis-aligned in
    /// plate coordinates.
    fn plate_box(&mut self, a: (f64, f64, f64), b: (f64, f64, f64), c: Color32) {
        let p = self.place;
        let o = p.at(a.0, a.1, a.2);
        let u = sub(p.at(b.0, a.1, a.2), o);
        let v = sub(p.at(a.0, b.1, a.2), o);
        let w = sub(p.at(a.0, a.1, b.2), o);
        self.cuboid(o, u, v, w, c);
    }

    /// A prism standing on the plate point `c`, from plate z `z.0` to
    /// `z.1`: `sides` flat faces, its corners `r` from the axis, the first
    /// at the plate angle `turn`; capped at both ends.
    fn plate_prism(&mut self, c: (f64, f64), r: f64, sides: usize, turn: f64, z: (f64, f64), col: Color32) {
        let p = self.place;
        let corner = |k: usize, z: f64| {
            let a = turn + std::f64::consts::TAU * k as f64 / sides as f64;
            p.at(c.0 + r * a.cos(), c.1 + r * a.sin(), z)
        };
        let col = rgba(col, 1.0);
        for k in 0..sides {
            self.quad(false, [corner(k, z.0), corner(k + 1, z.0), corner(k + 1, z.1), corner(k, z.1)], col);
        }
        for h in [z.0, z.1] {
            for k in 1..sides - 1 {
                self.tri(false, [corner(0, h), corner(k, h), corner(k + 1, h)], [col; 3]);
            }
        }
    }

    /// A tube along the world path through the rings' centres, capped at
    /// both ends: each ring's section a rectangle with rounded corners,
    /// its half-widths along `side` and across, and its corner radius (as
    /// large as the smaller half-width throughout: an ellipse). Smooth
    /// around; along, smooth where the surface bends by less than
    /// [`SMOOTH_DEG`] (a ring repeated at the same centre makes a step).
    fn sweep(&mut self, rings: &[(V3, f32, f32, f32)], side: V3, c: Color32) {
        const ARC: usize = 4;
        let n = rings.len();
        if n < 2 {
            return;
        }
        let c = rgba(c, 1.0);
        let round = rings.iter().all(|&(_, u, v, r)| r >= u.min(v) - 1e-6);
        // The section: per point, its place and the tangent around (in the
        // ring's side and across axes) for half-widths (u, v), radius r.
        let count = if round { 4 * ARC } else { 4 * (ARC + 1) };
        let section = |u: f32, v: f32, r: f32, k: usize| -> ((f32, f32), (f32, f32)) {
            if round {
                let a = std::f32::consts::TAU * k as f32 / count as f32;
                ((u * a.cos(), v * a.sin()), (-u * a.sin(), v * a.cos()))
            } else {
                let r = r.min(0.98 * u.min(v));
                let (corner, j) = (k / (ARC + 1), k % (ARC + 1));
                let a = std::f32::consts::FRAC_PI_2 * (corner as f32 + j as f32 / ARC as f32);
                let (sx, sy) = ([1.0, -1.0, -1.0, 1.0][corner], [1.0, 1.0, -1.0, -1.0][corner]);
                ((sx * (u - r) + r * a.cos(), sy * (v - r) + r * a.sin()), (-a.sin(), a.cos()))
            }
        };
        // Each ring's frame: the path's tangent, `side` square to it, and
        // the third axis across.
        let frames: Vec<(V3, V3, V3)> = (0..n)
            .map(|i| {
                let mut t = sub(rings[(i + 1).min(n - 1)].0, rings[i.saturating_sub(1)].0);
                // At a step (a repeated centre), the next ring's own tangent.
                let mut j = i;
                while dot(t, t) < 1e-12 && j + 1 < n {
                    j += 1;
                    t = sub(rings[(j + 1).min(n - 1)].0, rings[j.saturating_sub(1)].0);
                }
                let t = norm(t);
                let u = norm(sub(side, scale(t, dot(side, t))));
                (t, u, cross(t, u))
            })
            .collect();
        let point = |i: usize, k: usize| {
            let (p, u, v, r) = rings[i];
            let ((x, y), _) = section(u, v, r, k);
            let (_, a, b) = frames[i];
            add(p, add(scale(a, x), scale(b, y)))
        };
        let around = |i: usize, k: usize| {
            let (_, u, v, r) = rings[i];
            let (_, (x, y)) = section(u, v, r, k);
            let (_, a, b) = frames[i];
            add(scale(a, x), scale(b, y))
        };
        // Along segment `s`, at its ring `i` and point `k`: the outward
        // normal, smoothed with the neighbouring segment's at a gentle bend.
        let along = |s: usize, k: usize| sub(point(s + 1, k), point(s, k));
        let normal = |s: usize, i: usize, k: usize| {
            let d = along(s, k);
            let other = if i == s { s.checked_sub(1) } else { (s + 1 < n - 1).then_some(s + 1) };
            let d = match other.map(|o| along(o, k)) {
                Some(e) if dot(norm(d), norm(e)) > SMOOTH_DEG.to_radians().cos() as f32 => add(norm(d), norm(e)),
                _ => d,
            };
            norm(cross(around(i, k), d))
        };
        for s in 0..n - 1 {
            for k in 0..count {
                let l = (k + 1) % count;
                let p = [point(s, k), point(s + 1, k), point(s + 1, l), point(s, l)];
                let nn = [normal(s, s, k), normal(s, s + 1, k), normal(s, s + 1, l), normal(s, s, l)];
                self.quad_n(p, nn, c);
            }
        }
        for (i, out) in [(0, -1.0), (n - 1, 1.0)] {
            let centre = rings[i].0;
            for k in 0..count {
                let mut t = [centre, point(i, k), point(i, (k + 1) % count)];
                if out < 0.0 {
                    t.swap(1, 2);
                }
                self.tri(false, t, [c; 3]);
            }
        }
    }

    /// A slab across the keyboard (world x) from `x0` to `x1`, its side
    /// the polygon `profile` in (y, z), capped by it; its edges shaded
    /// smooth where the profile bends by less than [`SMOOTH_DEG`].
    fn slab(&mut self, profile: &[(f32, f32)], x0: f32, x1: f32, c: Color32) {
        let mut p: Vec<(f64, f64)> = profile.iter().map(|&(y, z)| (y as f64, z as f64)).collect();
        if signed_area(&p) < 0.0 {
            p.reverse();
        }
        let n = p.len();
        let col = rgba(c, 1.0);
        for [i, j, k] in triangulate(&p) {
            for x in [x0, x1] {
                self.tri(false, [p[i], p[j], p[k]].map(|(y, z)| [x, y as f32, z as f32]), [col; 3]);
            }
        }
        // Counter-clockwise in (y, z): an edge's outward normal is its
        // direction turned clockwise.
        let out = |k: usize| {
            let (a, b) = (p[k], p[(k + 1) % n]);
            let l = (b.0 - a.0).hypot(b.1 - a.1).max(1e-12);
            ((b.1 - a.1) / l, -(b.0 - a.0) / l)
        };
        let at = |k: usize, e: usize| {
            let (o, m) = (out(e), out(if k == e { (e + n - 1) % n } else { (e + 1) % n }));
            let s = if o.0 * m.0 + o.1 * m.1 > SMOOTH_DEG.to_radians().cos() { (o.0 + m.0, o.1 + m.1) } else { o };
            norm([0.0, s.0 as f32, s.1 as f32])
        };
        for e in 0..n {
            let f = (e + 1) % n;
            let w = |k: usize, x: f32| [x, p[k].0 as f32, p[k].1 as f32];
            self.quad_n([w(e, x0), w(f, x0), w(f, x1), w(e, x1)], [at(e, e), at(f, e), at(f, e), at(e, e)], col);
        }
    }

    fn line(&mut self, a: V3, b: V3, c: [f32; 4]) {
        for pos in [a, b] {
            self.s.lines.push(Vertex { pos, normal: [0.0, 0.0, 1.0], color: c });
        }
    }

    fn label(&mut self, at: V3, text: impl Into<String>, page: &'static str) {
        self.s.labels.push(Label { at, text: text.into(), page });
    }

    /// The extent of everything drawn so far.
    fn bounds(&self) -> (V3, V3) {
        let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
        for v in self.s.tris.iter().chain(&self.s.glass).chain(&self.s.lines) {
            for i in 0..3 {
                lo[i] = lo[i].min(v.pos[i]);
                hi[i] = hi[i].max(v.pos[i]);
            }
        }
        if lo[0] <= hi[0] { (lo, hi) } else { ([-1.0; 3], [1.0; 3]) }
    }

    fn finish(mut self) -> Scene {
        (self.s.min, self.s.max) = self.bounds();
        self.s.vertical = self.place.vertical;
        self.s
    }
}

/// The board's outline in plate metres, counter-clockwise: the model's
/// own (`soundboard.outline`), or the full a × b rectangle.
pub fn board_outline(sb: &SoundboardParams) -> Vec<(f64, f64)> {
    let mut p = Outline::new(sb).map_or_else(|| vec![(0.0, 0.0), (sb.a, 0.0), (sb.a, sb.b), (0.0, sb.b)], |o| o.points);
    if signed_area(&p) < 0.0 {
        p.reverse();
    }
    p
}

/// The iron frame's outline in plate coordinates, as the engine has it: the
/// case's, carried forward over the pin block (`FrameParams::outline`).
pub fn frame_outline(model: &PianoModel, sb: &SoundboardParams) -> Vec<(f64, f64)> {
    let f = &model.frame;
    let plate = f.plate(sb);
    let mut p = Outline::new(&plate).map_or_else(|| vec![(0.0, 0.0), (f.a, 0.0), (f.a, f.b), (0.0, f.b)], |o| o.points);
    for q in &mut p {
        q.0 += f.offset_x;
    }
    if signed_area(&p) < 0.0 {
        p.reverse();
    }
    p
}

fn signed_area(p: &[(f64, f64)]) -> f64 {
    0.5 * p.iter().zip(p.iter().cycle().skip(1)).map(|(a, b)| a.0 * b.1 - b.0 * a.1).sum::<f64>()
}

fn cross2(a: (f64, f64), b: (f64, f64), c: (f64, f64)) -> f64 {
    (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
}

/// The part of the polygon `subject` inside the convex counter-clockwise
/// `window` (Sutherland–Hodgman).
pub fn clip(subject: &[(f64, f64)], window: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let mut out = subject.to_vec();
    for k in 0..window.len() {
        let (a, b) = (window[k], window[(k + 1) % window.len()]);
        let input = std::mem::take(&mut out);
        let cut = |p: (f64, f64), q: (f64, f64)| {
            let (dp, dq) = (cross2(a, b, p), cross2(a, b, q));
            let t = dp / (dp - dq);
            (p.0 + t * (q.0 - p.0), p.1 + t * (q.1 - p.1))
        };
        for (i, &p) in input.iter().enumerate() {
            let q = input[(i + 1) % input.len()];
            match (cross2(a, b, p) >= 0.0, cross2(a, b, q) >= 0.0) {
                (true, true) => out.push(q),
                (true, false) => out.push(cut(p, q)),
                (false, true) => out.extend([cut(p, q), q]),
                (false, false) => {}
            }
        }
        if out.is_empty() {
            break;
        }
    }
    out
}

/// Triangles covering the simple counter-clockwise polygon `p` (ear
/// clipping), as indices into it.
pub fn triangulate(p: &[(f64, f64)]) -> Vec<[usize; 3]> {
    let mut idx: Vec<usize> = (0..p.len()).collect();
    let mut tris = Vec::new();
    while idx.len() > 3 {
        let n = idx.len();
        let ear = (0..n).find(|&i| {
            let (a, b, c) = (p[idx[(i + n - 1) % n]], p[idx[i]], p[idx[(i + 1) % n]]);
            cross2(a, b, c) > 0.0
                && idx.iter().all(|&j| {
                    let q = p[j];
                    q == a || q == b || q == c || !(cross2(a, b, q) >= 0.0 && cross2(b, c, q) >= 0.0 && cross2(c, a, q) >= 0.0)
                })
        });
        let Some(i) = ear else { break };
        tris.push([idx[(i + n - 1) % n], idx[i], idx[(i + 1) % n]]);
        idx.remove(i);
    }
    if idx.len() == 3 {
        tris.push([idx[0], idx[1], idx[2]]);
    }
    tris
}

/// Where each key's strings start across the agraffes (m, plate y): the
/// keys' places along the keyboard, laid over the main bridge's span.
pub fn key_across(sb: &SoundboardParams, m: u8) -> f64 {
    let (lo, hi) = keyboard_span(sb);
    let (x, w) = sketches::key_span(m);
    hi - (x + w / 2.0) as f64 / 52.0 * (hi - lo)
}

/// The keyboard's extent across (plate y): the main bridge's ends.
fn keyboard_span(sb: &SoundboardParams) -> (f64, f64) {
    let y = |m: u8| sb.bridge_point_at(m as f64, true).1;
    let (a, b) = (y(108), y(sb.main_bridge_from));
    (a.min(b), a.max(b))
}

/// The agraffes' line (plate x), as the plan sketch draws it.
pub fn agraffe_x(model: &PianoModel) -> f64 {
    let f = &model.frame;
    let fx = if f.enabled { f.offset_x } else { -0.2 };
    fx + if f.enabled { f.agraffe_x } else { 0.25 }
}

/// The hammers' line (plate x): straight across the keyboard, as the
/// action's rail holds them. Each key's agraffe stands in front of it by
/// its strike point's share of the string, so that the agraffes run on a
/// diagonal, the bass's (the longest strings) frontmost, on the agraffe
/// line, the treble's just in front of the hammers. A grand's hammers
/// rise in front of its board, though: there the line stops short of the
/// board's edge, and the bass's agraffes come forward of the agraffe line.
pub fn hammer_x(model: &PianoModel, sb: &SoundboardParams) -> f64 {
    let ag = agraffe_x(model);
    let x = (21..=108u8).map(|m| ag + model.note(m).strike_pos * (sb.bridge_point(m).0 - ag)).fold(f64::MIN, f64::max);
    if sb.vertical { x } else { x.min(-HAMMER_CLEAR) }
}

/// A key's string in plate coordinates: from its agraffe to its bridge
/// point, at its height. Its hammer strikes it on the hammers' line `hx`
/// ([`hammer_x`]), in line with its key; the agraffe lies on the string's
/// line beyond, the more to the side the more the string slants (the
/// overstrung bass's, towards the bass end).
pub fn string_ends(model: &PianoModel, sb: &SoundboardParams, hx: f64, m: u8) -> ((f64, f64), (f64, f64), f64) {
    let z = STRING_Z + if m < sb.main_bridge_from { BASS_RISE } else { 0.0 };
    let (bx, by) = sb.bridge_point(m);
    let s = model.note(m).strike_pos;
    let back = |hit: f64, bridge: f64| (hit - s * bridge) / (1.0 - s);
    ((back(hx, bx), back(key_across(sb, m), by)), (bx, by), z)
}

/// A grand's leg under `top` (world, the case's underside), as most are
/// made: a moulded block under the case, a square shaft tapering down with
/// rounded corners and a bead under its neck, a collar and a brass ferrule
/// at its foot, on a caster of two brass wheels side by side.
fn leg(b: &mut Builder, top: V3, floor: f32) {
    let [x, y, zt] = top;
    let at = |z: f32| [x, y, z];
    // The wheels roll along y, on an axle across; the fork's cheeks
    // outside them, its housing above.
    let (wr, ww) = (0.028, 0.012);
    let hub = floor + wr;
    for s in [-1.0, 1.0] {
        let (x0, x1) = (x + s * 0.003, x + s * (0.003 + ww));
        let (x0, x1) = (x0.min(x1), x0.max(x1));
        let wheel = |x: f32, r: f32| ([x, y, hub], r, r, r);
        b.sweep(&[wheel(x0, wr - 0.004), wheel(x0 + 0.002, wr), wheel(x1 - 0.002, wr), wheel(x1, wr - 0.004)], [0.0, 1.0, 0.0], BRASS);
        b.cuboid([x + s * 0.019 - 0.0015, y - 0.012, hub - 0.006], [0.003, 0.0, 0.0], [0.0, 0.024, 0.0], [0.0, 0.0, wr + 0.02], BRASS);
    }
    let axle = |x: f32| ([x, y, hub], 0.005, 0.005, 0.005);
    b.sweep(&[axle(x - 0.022), axle(x + 0.022)], [0.0, 1.0, 0.0], STEEL);
    let zc = hub + wr + 0.014;
    let ring = |z: f32, w: f32, r: f32| (at(z), w, w, r);
    b.sweep(&[ring(hub + wr - 0.006, 0.02, 0.019), ring(zc - 0.004, 0.026, 0.025), ring(zc, 0.028, 0.027)], [1.0, 0.0, 0.0], BRASS);
    // The ferrule, then the shaft up to the block under the case.
    let f = zc + 0.022;
    b.sweep(&[ring(zc, 0.03, 0.008), ring(zc + 0.003, 0.033, 0.009), ring(f - 0.003, 0.033, 0.009), ring(f, 0.03, 0.008)], [1.0, 0.0, 0.0], BRASS);
    let neck = zt - 0.12;
    let wood = [
        ring(f, 0.031, 0.008),
        ring(f, 0.036, 0.01),
        ring(f + 0.004, 0.04, 0.012),
        ring(f + 0.018, 0.04, 0.012),
        ring(f + 0.03, 0.034, 0.01),
        ring(f + 0.045, 0.034, 0.01),
        ring(neck, 0.05, 0.014),
        ring(neck + 0.004, 0.056, 0.016),
        ring(neck + 0.014, 0.056, 0.016),
        ring(neck + 0.018, 0.05, 0.014),
        ring(zt - 0.075, 0.05, 0.014),
        ring(zt - 0.055, 0.054, 0.014),
        ring(zt - 0.042, 0.062, 0.014),
        ring(zt - 0.036, 0.066, 0.012),
        ring(zt - 0.036, 0.074, 0.01),
        ring(zt, 0.074, 0.01),
    ];
    b.sweep(&wood, [1.0, 0.0, 0.0], CASE);
}

/// A pedal from the face it comes out of at `y` (world), `x` across,
/// `z` high, towards the pianist: narrow where it leaves the box, widening
/// into a foot pad that rises a little to its rounded tip; turned about its
/// pivot inside the box by `press` (0 at rest … 1 down).
fn pedal(b: &mut Builder, x: f32, y: f32, z: f32, press: f32, c: Color32) {
    let (py, pz) = (y + 0.06, z);
    let (s, co) = (0.1 * press.clamp(0.0, 1.0)).sin_cos();
    let ring = |d: f32, w: f32, t: f32| {
        // Out of the face by d, then turned down (towards −z) about the pivot.
        let (dy, dz) = (y - d - py, 0.006 * (d / 0.13).max(0.0).powi(2));
        ([x, py + dy * co - dz * s, pz + dy * s + dz * co], w, t, 0.004)
    };
    let rings = [
        ring(-0.01, 0.009, 0.006),
        ring(0.0, 0.009, 0.006),
        ring(0.035, 0.011, 0.0056),
        ring(0.07, 0.017, 0.0052),
        ring(0.09, 0.02, 0.005),
        ring(0.112, 0.021, 0.005),
        ring(0.124, 0.019, 0.0048),
        ring(0.131, 0.013, 0.0043),
    ];
    b.sweep(&rings, [1.0, 0.0, 0.0], c);
}

/// A grand's pedal lyre centred at `cx` across and `y` along, from the
/// floor up to the keybed's underside at `top`, as on most grands: a block
/// screwed under the keybed, two posts bowed outwards like a lyre's arms
/// with slats between them, the box the pedals come out of at its foot,
/// and two braces from the box's back up to the belly rail at `brace`
/// (y, z).
fn lyre(b: &mut Builder, cx: f32, y: f32, top: f32, floor: f32, brace: (f32, f32), pedals: [(f32, Color32); 3]) {
    let across = |x0: f32, x1: f32, z: f32, half: (f32, f32), r: f32, ease: f32| {
        let ring = |x: f32, k: f32| ([x, y, z], half.0 - k, half.1 - k, r);
        [ring(x0, ease), ring(x0 + ease, 0.0), ring(x1 - ease, 0.0), ring(x1, ease)]
    };
    // The box, on a plinth, under a moulded top.
    let (bz0, bz1) = (floor + 0.04, floor + 0.115);
    b.sweep(&across(cx - 0.2, cx + 0.2, floor + 0.035, (0.062, 0.008), 0.004, 0.004), [0.0, 1.0, 0.0], CASE);
    b.sweep(&across(cx - 0.19, cx + 0.19, (bz0 + bz1) / 2.0, (0.055, (bz1 - bz0) / 2.0), 0.012, 0.004), [0.0, 1.0, 0.0], CASE);
    b.sweep(&across(cx - 0.2, cx + 0.2, bz1 + 0.007, (0.062, 0.008), 0.005, 0.004), [0.0, 1.0, 0.0], CASE);
    // The block under the keybed, a moulding under it.
    b.sweep(&across(cx - 0.21, cx + 0.21, top - 0.018, (0.048, 0.018), 0.008, 0.004), [0.0, 1.0, 0.0], CASE);
    b.sweep(&across(cx - 0.17, cx + 0.17, top - 0.042, (0.038, 0.006), 0.004, 0.003), [0.0, 1.0, 0.0], CASE);
    // The posts, between the box's top and the moulding.
    let (p0, p1) = (bz1 + 0.015, top - 0.048);
    for s in [-1.0f32, 1.0] {
        let rings: Vec<_> = (0..=16)
            .map(|i| {
                let t = i as f32 / 16.0;
                let bow = 0.045 * (std::f32::consts::PI * t).sin() * (1.0 - 0.35 * t);
                ([cx + s * (0.1 + bow), y, p0 + (p1 - p0) * t], 0.02, 0.013, 0.006)
            })
            .collect();
        b.sweep(&rings, [0.0, 1.0, 0.0], CASE);
    }
    for (x, w) in [(-0.045, 0.006), (0.0, 0.009), (0.045, 0.006)] {
        let ring = |z: f32| ([cx + x, y, z], 0.01, w, 0.004);
        b.sweep(&[ring(p0), ring(p1)], [0.0, 1.0, 0.0], CASE);
    }
    // The braces: steel rods with brass ends.
    for s in [-1.0f32, 1.0] {
        let (a, z) = ([cx + s * 0.15, y + 0.05, bz1 - 0.01], [cx + s * 0.2, brace.0, brace.1]);
        let d = norm(sub(z, a));
        let side = norm(cross(d, [1.0, 0.0, 0.0]));
        let rod = |p: V3, r: f32| (p, r, r, r);
        b.sweep(&[rod(a, 0.006), rod(z, 0.006)], side, Color32::from_rgb(64, 64, 70));
        for (p, q) in [(a, add(a, scale(d, 0.03))), (sub(z, scale(d, 0.03)), z)] {
            b.sweep(&[rod(p, 0.009), rod(q, 0.009)], side, BRASS);
        }
    }
    // The pedals through brass plates on the box's face.
    let face = y - 0.055;
    for (i, &(press, c)) in pedals.iter().enumerate() {
        let x = cx + (i as f32 - 1.0) * PEDAL_PITCH;
        b.cuboid([x - 0.017, face - 0.002, floor + 0.06], [0.034, 0.0, 0.0], [0.0, 0.002, 0.0], [0.0, 0.0, 0.026], BRASS);
        pedal(b, x, face, floor + 0.073, press, c);
    }
}

pub fn build(model: &PianoModel, live: Option<&Live>, o: &Options) -> Scene {
    let sb = model.soundboard.map(|s| s.resolved()).unwrap_or_default();
    let vertical = sb.vertical;
    let place = Place { vertical, top: UPRIGHT_BASE + sb.a };
    let mut b = Builder { s: Scene::default(), place };
    let lit = |p: Part| o.lit.contains(&p);
    let pick = |p: Part, c: Color32| if lit(p) { sketches::HIGHLIGHT } else { c };
    let (f, rim) = (&model.frame, &model.rim);
    let obs = live.map(|l| l.obs);
    let key_level = |m: u8| live.map_or(0.0, |l| l.keys[m as usize]);

    // The soundboard within its outline, on a grid so that its motion can
    // colour it: each grid cell clipped to the outline.
    let (gx, gy) = BOARD_GRID;
    let wood = pick(Part::Board, sketches::WOOD);
    let outline = board_outline(&sb);
    let field = live.and_then(|l| l.board);
    let colour = |x: f64, y: f64| -> [f32; 4] {
        let Some(f) = field else { return rgba(wood, 1.0) };
        let (u, v) = ((x / sb.a).clamp(0.0, 1.0) * (gx - 1) as f64, (y / sb.b).clamp(0.0, 1.0) * (gy - 1) as f64);
        let (i, j) = ((u as usize).min(gx - 2), (v as usize).min(gy - 2));
        let (fu, fv) = ((u - i as f64) as f32, (v - j as f64) as f32);
        let at = |i: usize, j: usize| f[j * gx + i];
        let level = (at(i, j) * (1.0 - fu) + at(i + 1, j) * fu) * (1.0 - fv) + (at(i, j + 1) * (1.0 - fu) + at(i + 1, j + 1) * fu) * fv;
        rgba(heat(level), 1.0)
    };
    for j in 0..gy - 1 {
        for i in 0..gx - 1 {
            let (x0, x1) = (sb.a * i as f64 / (gx - 1) as f64, sb.a * (i + 1) as f64 / (gx - 1) as f64);
            let (y0, y1) = (sb.b * j as f64 / (gy - 1) as f64, sb.b * (j + 1) as f64 / (gy - 1) as f64);
            let piece = clip(&outline, &[(x0, y0), (x1, y0), (x1, y1), (x0, y1)]);
            for k in 1..piece.len().saturating_sub(1) {
                let t = [piece[0], piece[k], piece[k + 1]];
                b.tri(false, t.map(|(x, y)| place.at(x, y, 0.0)), t.map(|(x, y)| colour(x, y)));
            }
        }
    }
    // Its underside (its edge is against the rim's wall).
    let under = rgba(wood.gamma_multiply(0.7), 1.0);
    for [i, j, k] in triangulate(&outline) {
        b.tri(false, [outline[i], outline[k], outline[j]].map(|(x, y)| place.at(x, y, -0.012)), [under; 3]);
    }
    b.label(place.at(sb.a * 0.45, sb.b * 0.7, 0.0), "Soundboard", "Soundboard");

    // The strings' ends: their agraffes on a diagonal in front of the
    // hammers ([`hammer_x`]), their tuning pins in front of those.
    let hx = hammer_x(model, &sb);
    let ends: Vec<_> = (21..=108u8).map(|m| string_ends(model, &sb, hx, m)).collect();
    let case_front = ends.iter().map(|e| e.0.0).fold(f64::MAX, f64::min) - PIN_GAP - PIN_ROW - PIN_R - 0.008;

    // The case: the rim's walls along the board's outline, carried forward
    // over the pin block to just in front of the tuning pins (as the frame
    // is), and the belly rail under the board's front edge.
    let t = if rim.enabled { rim.thickness } else { 0.02 };
    let (z0, z1) = if rim.enabled { (rim.top - rim.height, rim.top) } else { (-0.25, 0.05) };
    let case = pick(Part::Rim, CASE);
    let n = outline.len();
    let front = (0..n).find(|&k| outline[k].0.abs() < 1e-6 && outline[(k + 1) % n].0.abs() < 1e-6);
    let (mut across_lo, mut across_hi) = (0.0, sb.b);
    if let Some(k) = front {
        let (p, q) = (outline[k], outline[(k + 1) % n]);
        let forward = |p: (f64, f64)| if p.0.abs() < 1e-6 { (case_front, p.1) } else { p };
        let sides: Vec<_> = (1..=n).map(|i| forward(outline[(k + i) % n])).collect();
        b.wall(&sides, false, 0.0, t, z0, &|_| z1, case);
        (across_lo, across_hi) = (p.1.min(q.1), p.1.max(q.1));
        b.wall(&[p, q], false, -BELLY_RAIL, 0.0, z0, &|_| -0.012, case);
    } else {
        b.wall(&outline, true, 0.0, t, z0, &|_| z1, case);
    }

    // The pin block under the plate, from the case's front to just behind
    // the agraffes, short of the hammers.
    let block_back = |ax: f64| (ax + 0.008).min(hx - HAMMER_CLEAR);
    let mut back: Vec<(f64, f64)> = ends.iter().map(|&((ax, ay), _, _)| (block_back(ax), ay.clamp(across_lo, across_hi))).collect();
    back.sort_by(|p, q| q.1.total_cmp(&p.1));
    let mut block = vec![(case_front, across_lo), (case_front, across_hi), (back[0].0, across_hi)];
    block.extend(&back);
    block.push((back[back.len() - 1].0, across_lo));
    if signed_area(&block) < 0.0 {
        block.reverse();
    }
    let plate = rgba(pick(Part::Frame, IRON), 1.0);
    let (bz0, bz1) = (PIN_BLOCK_TOP - PIN_BLOCK_T, PIN_BLOCK_TOP);
    for [i, j, k] in triangulate(&block) {
        for z in [bz0, bz1] {
            b.tri(false, [block[i], block[j], block[k]].map(|(x, y)| place.at(x, y, z)), [plate; 3]);
        }
    }
    for i in 0..block.len() {
        let (p, q) = (block[i], block[(i + 1) % block.len()]);
        b.quad(false, [place.at(p.0, p.1, bz0), place.at(q.0, q.1, bz0), place.at(q.0, q.1, bz1), place.at(p.0, p.1, bz1)], plate);
    }
    let rim_label = outline.iter().cloned().fold((0.0, 0.0), |m, p| if p.0 > m.0 { p } else { m });
    b.label(place.at(rim_label.0 + t, rim_label.1, z1), if rim.enabled { "Rim" } else { "Case (not modelled)" }, "Frame & rim");

    // Bridges on the board.
    let main = pick(Part::Bridges, sketches::BRIDGE);
    let bass = if lit(Part::BassBridge) { sketches::HIGHLIGHT } else { main };
    let tb = sb.bridge_line(true);
    b.wall(&tb, false, -0.0125, 0.0125, 0.0, &|_| BRIDGE_H, main);
    let bb = sb.bridge_line(false);
    b.wall(&bb, false, -0.015, 0.015, 0.0, &|_| BRIDGE_H + BASS_RISE, bass);
    let (lx, ly) = tb[tb.len() / 2];
    b.label(place.at(lx, ly, BRIDGE_H), "Bridge", "Soundboard");
    b.label(place.at(bb[0].0, bb[0].1, BRIDGE_H + BASS_RISE), "Bass bridge", "Soundboard");

    // Strings, from their tuning pins across their agraffes, hammers and
    // dampers.
    let d = &model.damper;
    // Strings counted from the bass, for the pins' rows.
    let mut count = 0usize;
    for m in 21..=108u8 {
        let n = model.note(m);
        let ((ax, ay), (bx, by), z) = ends[(m - 21) as usize];
        let level = key_level(m);
        let base = if m < sb.main_bridge_from || n.strings == 1 { COPPER } else { STEEL };
        let base = if lit(Part::Length) || lit(Part::Unison) { sketches::HIGHLIGHT } else { base };
        let c = if m == o.key { o.ink } else { mix(base, sketches::HIGHLIGHT, level * 0.8) };
        // Across the unison, 2.5 mm apart.
        let (dx, dy) = (bx - ax, by - ay);
        let len = dx.hypot(dy).max(1e-9);
        let (nx, ny) = (-dy / len, dx / len);
        let segments = if level > 0.02 { 16 } else { 1 };
        for s in 0..n.strings {
            let off = (s as f64 - (n.strings as f64 - 1.0) / 2.0) * 0.0025;
            let point = |t: f64| {
                let amp = live.map_or(0.0, |l| {
                    let phase = l.time * std::f32::consts::TAU * (5.0 + 0.07 * m as f32) + s as f32;
                    0.012 * level * phase.sin() * (std::f32::consts::PI * t as f32).sin()
                });
                let z = z + amp as f64;
                place.at(ax + dx * t + nx * off, ay + dy * t + ny * off, z)
            };
            for k in 0..segments {
                let (t0, t1) = (k as f64 / segments as f64, (k + 1) as f64 / segments as f64);
                b.line(point(t0), point(t1), rgba(c, 1.0));
            }
            // Its tuning pin on the string's line beyond the agraffe, its
            // coil at the string's height, and the string's dead end from
            // it to the agraffe.
            let (back, across) = (PIN_GAP + PIN_ROW * (count % 2) as f64, off / 0.0025 * PIN_ACROSS);
            let (ux, uy) = (dx / len, dy / len);
            let (px, py) = (ax - ux * back + nx * across, ay - uy * back + ny * across);
            b.plate_prism((px, py), PIN_R, 6, 0.0, (PIN_BLOCK_TOP, z + 0.012), PIN);
            b.plate_prism((px, py), PIN_R + 0.001, 6, 0.0, (z - 0.003, z + 0.002), c);
            b.line(place.at(px + ux * PIN_R, py + uy * PIN_R, z), point(0.0), rgba(c, 1.0));
            count += 1;
        }
        // The agraffe, a brass stud the unison goes through, turned along
        // it, standing on the pin block (the highest treble's overhang it);
        // the hammer under (an upright's: in front of) the strings at
        // the strike point, all on one line.
        let turn = dy.atan2(dx);
        let r = 0.0038 + 0.0008 * (n.strings as f64 - 1.0);
        let foot = if ax + r <= block_back(ax) { PIN_BLOCK_TOP } else { z - 0.008 };
        b.plate_prism((ax, ay), r, 6, turn, (foot, z + 0.005), pick(Part::Agraffes, BRASS));
        let k = obs.map(|o| o.keys[m as usize]);
        let down = k.is_some_and(|k| k.down);
        let hammer_on = k.is_some_and(|k| k.hammer);
        let (sx, sy) = (ax + dx * n.strike_pos, ay + dy * n.strike_pos);
        // At rest, their felts just above the belly rail.
        let gap = if hammer_on { 0.0 } else if down { 0.012 } else { 0.025 };
        let felt = if hammer_on { sketches::HIGHLIGHT } else { pick(Part::Strike, pick(Part::Felt, sketches::FELT)) };
        let w = 0.0045;
        let (h0, h1) = if vertical { (z + 0.002 + gap, z + 0.032 + gap) } else { (z - 0.032 - gap, z - 0.002 - gap) };
        b.plate_box((sx - 0.012, sy - w, h0), (sx + 0.012, sy + w, h1), felt);
        // The damper on the strings, lifted by the key or the pedal.
        if m <= d.highest_note {
            let (px, py) = (ax + dx * d.position, ay + dy * d.position);
            let lift = 0.015 * (1.0 - k.map_or(1.0, |k| k.damper)) as f64;
            let felt = pick(Part::DamperPoint, pick(Part::Damper, Color32::from_rgb(90, 90, 96)));
            // Above a grand's strings, in front of an upright's.
            let (d0, d1) = (z + 0.002 + lift, z + 0.022 + lift);
            b.plate_box((px - 0.01, py - w, d0), (px + 0.01, py + w, d1), felt);
        }
    }
    let mid = string_ends(model, &sb, hx, 60);
    let n60 = model.note(60);
    let strike = (mid.0.0 + (mid.1.0 - mid.0.0) * n60.strike_pos, mid.0.1 + (mid.1.1 - mid.0.1) * n60.strike_pos);
    b.label(place.at(strike.0, strike.1, if vertical { mid.2 + 0.04 } else { mid.2 - 0.04 }), "Hammers", "Hammer");
    let damp = (mid.0.0 + (mid.1.0 - mid.0.0) * d.position, mid.0.1 + (mid.1.1 - mid.0.1) * d.position);
    b.label(place.at(damp.0, damp.1 + 0.12, mid.2 + 0.03), "Dampers", "Dampers");
    b.label(place.at((mid.0.0 + mid.1.0) / 2.0, mid.0.1 - 0.25, mid.2), "Strings", "Strings");
    let a0 = string_ends(model, &sb, hx, 21);
    b.label(place.at(a0.0.0, a0.0.1, a0.2 + 0.01), "Agraffes", "Frame & rim");
    let c8 = string_ends(model, &sb, hx, 108);
    b.label(place.at(c8.0.0 - PIN_GAP, c8.0.1, c8.2 + 0.015), "Tuning pins", "Frame & rim");

    // The iron frame over the strings' anchoring, see-through.
    if f.enabled && o.frame {
        let c = rgba(pick(Part::Frame, IRON), if lit(Part::Frame) || lit(Part::FrameHeight) { 0.45 } else { 0.18 });
        let z = f.height;
        let outline = frame_outline(model, &sb);
        let q: Vec<V3> = outline.iter().map(|&(x, y)| place.at(x, y, z)).collect();
        for [i, j, k] in triangulate(&outline) {
            b.tri(true, [q[i], q[j], q[k]], [c; 3]);
            b.tri(true, [q[i], q[k], q[j]], [c; 3]);
        }
        let edge = rgba(pick(Part::Frame, IRON), 0.9);
        for i in 0..q.len() {
            b.line(q[i], q[(i + 1) % q.len()], edge);
        }
        // Labelled at its tail, the farthest point from the keyboard.
        let tail = (0..outline.len()).max_by(|&i, &j| outline[i].0.total_cmp(&outline[j].0)).unwrap_or(0);
        b.label(q[tail], "Frame", "Frame & rim");
    }

    // The lid, hinged along the bass side.
    if !vertical && sb.lid_angle > 0.0 && o.lid {
        let th = sb.lid_angle.to_radians();
        let (h, len) = (sb.lid_hinge_height, 1.05 * sb.b);
        let p0 = |x: f64| place.at(x, sb.b, h);
        let p1 = |x: f64| place.at(x, sb.b - len * th.cos(), h + len * th.sin());
        let c = rgba(CASE, 0.35);
        b.quad(true, [p0(-t), p0(sb.a + t), p1(sb.a + t), p1(-t)], c);
        b.quad(true, [p1(-t), p1(sb.a + t), p0(sb.a + t), p0(-t)], c);
        for (a, z) in [(p0(-t), p1(-t)), (p0(sb.a + t), p1(sb.a + t)), (p1(-t), p1(sb.a + t))] {
            b.line(a, z, rgba(CASE, 0.9));
        }
        b.label(p1(sb.a * 0.6), format!("Lid, {:.0}°", sb.lid_angle), "Microphones");
    }

    // The keyboard, the keys down pressed in, its back against the case's
    // front: a grand's on its keybed, at the usual height above the floor.
    let keys_back = case_front - FRONT_T;
    let (klo, khi) = keyboard_span(&sb);
    let white = (khi - klo) / 52.0;
    let (y_back, kz) = if vertical { (UPRIGHT_KEYS_Y, UPRIGHT_KEYS_Z) } else { (keys_back, z0 + KEYBED_T + KEY_FRAME) };
    let key_top = kz + 0.022;
    let floor = if vertical { 0.0 } else { key_top - KEY_HEIGHT };
    let key_front = y_back - KEY_DEPTH;
    for m in 21..=108u8 {
        let k = obs.map(|o| o.keys[m as usize]);
        let down = k.is_some_and(|k| k.down);
        let (x, w) = sketches::key_span(m);
        let y0 = khi - (x + w) as f64 * white + 0.0006;
        let y1 = khi - x as f64 * white - 0.0006;
        let black = sketches::is_black(m);
        let depth = if black { KEY_DEPTH * 0.62 } else { KEY_DEPTH };
        let (zb, h) = if black { (0.022, 0.012) } else { (0.0, 0.022) };
        let drop = if down { -0.008 } else { 0.0 };
        let base = if black { EBONY } else { IVORY };
        let c = if down {
            sketches::HIGHLIGHT
        } else if m == o.key {
            mix(base, sketches::HIGHLIGHT, 0.5)
        } else {
            pick(Part::Key, base)
        };
        let c = mix(c, sketches::HIGHLIGHT, key_level(m) * 0.4);
        // Keys in world coordinates: x across, the back of the keys at y_back.
        let o3 = [-y1 as f32, (key_front + KEY_DEPTH - depth) as f32, (kz + zb + drop) as f32];
        b.cuboid(o3, [(y1 - y0) as f32, 0.0, 0.0], [0.0, depth as f32, 0.0], [0.0, 0.0, h as f32], c);
    }
    let cx = -((klo + khi) / 2.0) as f32;
    // Under a grand's front legs, their centres (world x, y).
    let mut front_legs = [[0.0f32; 2]; 2];
    if vertical {
        // The keybed and key slip, the cheeks at both ends of the keys.
        b.cuboid([-(khi + 0.06) as f32, (key_front - 0.02) as f32, (kz - 0.05) as f32], [(khi - klo + 0.12) as f32, 0.0, 0.0], [0.0, (KEY_DEPTH + 0.02) as f32, 0.0], [0.0, 0.0, 0.05], CASE);
        for x in [-(khi + 0.06), -(klo - 0.01)] {
            b.cuboid([x as f32, key_front as f32, (kz - 0.01) as f32], [0.05, 0.0, 0.0], [0.0, KEY_DEPTH as f32, 0.0], [0.0, 0.0, 0.06], CASE);
        }
    } else {
        // Its case, as most grands': the keybed under the case's front,
        // its sides carried forward as arms that come down in a curve from
        // the rim's top to a little above the keys, rounded at their
        // front; the cheek blocks between them and the keys' ends, the key
        // slip along the keys' front, and behind them the fallboard (open)
        // over a strip of felt. Across (plate y): the case's sides, wider
        // if the keys are.
        let (lo, hi) = (across_lo.min(klo - 0.04), across_hi.max(khi + 0.04));
        let (x_lo, x_hi) = (-(hi + t) as f32, -(lo - t) as f32);
        let (yf, yb) = ((key_front - KEY_SLIP - 0.002) as f32, (y_back + FRONT_T) as f32);
        let (z0f, top, bed) = (z0 as f32, key_top as f32, (z0 + KEYBED_T) as f32);
        b.cuboid([x_lo, yf + 0.004, z0f], [x_hi - x_lo, 0.0, 0.0], [0.0, yb - yf - 0.004, 0.0], [0.0, 0.0, KEYBED_T as f32], case);
        let arm = top + 0.035;
        let mut profile = vec![(yf, z0f), (yb, z0f), (yb, z1 as f32)];
        for i in 1..=10 {
            let s = i as f32 / 10.0;
            profile.push((yb - 0.09 * s, arm + (z1 as f32 - arm) * 0.5 * (1.0 + (std::f32::consts::PI * s).cos())));
        }
        let r = 0.022;
        for i in 0..=6 {
            let a = std::f32::consts::FRAC_PI_2 * i as f32 / 6.0;
            profile.push((yf + r - r * a.sin(), arm - r + r * a.cos()));
        }
        for (x0, x1) in [(x_lo, -hi as f32), (-lo as f32, x_hi)] {
            b.slab(&profile, x0, x1, case);
        }
        for (x0, x1) in [(-hi as f32, -(khi + 0.002) as f32), (-(klo - 0.002) as f32, -lo as f32)] {
            let block = |y: f32| ([(x0 + x1) / 2.0, y, (bed + top + 0.012) / 2.0], (x1 - x0) / 2.0, (top + 0.012 - bed) / 2.0, 0.008);
            b.sweep(&[block(yf + KEY_SLIP as f32), block(y_back as f32)], [1.0, 0.0, 0.0], case);
        }
        let along = |x0: f32, x1: f32, y: f32, z: (f32, f32), d: f32, r: f32| {
            let ring = |x: f32| ([x, y, (z.0 + z.1) / 2.0], d / 2.0, (z.1 - z.0) / 2.0, r);
            [ring(x0), ring(x1)]
        };
        b.sweep(&along(-hi as f32, -lo as f32, yf + KEY_SLIP as f32 / 2.0, (z0f, kz as f32 - 0.002), KEY_SLIP as f32, 0.005), [0.0, 1.0, 0.0], case);
        b.sweep(&along(-hi as f32, -lo as f32, (y_back + FRONT_T / 2.0) as f32, (bed, PIN_BLOCK_TOP as f32), FRONT_T as f32, 0.009), [0.0, 1.0, 0.0], case);
        b.cuboid([-(khi + 0.002) as f32, y_back as f32 - 0.006, top - 0.004], [(khi - klo + 0.004) as f32, 0.0, 0.0], [0.0, 0.006, 0.0], [0.0, 0.0, 0.012], FELT);
        // The front legs under the arms, flush with their outsides.
        let y = yf + 0.1;
        front_legs = [[x_lo + 0.074, y], [x_hi - 0.074, y]];
    }
    b.label([-((klo + khi) / 2.0) as f32, key_front as f32, (kz + 0.03) as f32], "Keyboard", "Player");

    // Pedals: soft, sostenuto, sustain from left to right.
    let values = obs.map_or([0.0; 3], |o| [o.soft, if o.sostenuto { 1.0 } else { 0.0 }, o.sustain]);
    let parts = [Part::UnaCorda, Part::UnaCorda, Part::SustainPedal];
    let pedals: [(f32, Color32); 3] = std::array::from_fn(|i| {
        let v = values[i];
        (v, if v > 0.05 || (i != 1 && lit(parts[i])) { sketches::HIGHLIGHT } else { BRASS })
    });
    if vertical {
        for (i, &(v, c)) in pedals.iter().enumerate() {
            pedal(&mut b, cx + (i as f32 - 1.0) * PEDAL_PITCH, key_front as f32, floor as f32 + 0.06, v, c);
        }
    } else {
        // The lyre under the keybed, in front of the fallboard, braced to
        // the belly rail; the third leg under the case's tail.
        lyre(&mut b, cx, y_back as f32 - 0.05, z0 as f32, floor as f32, (-(BELLY_RAIL / 2.0) as f32, z0 as f32), pedals);
        let tail = place.at(sb.a - 0.12, sb.b * 0.5, z0);
        for [x, y] in front_legs.into_iter().chain([[tail[0], tail[1]]]) {
            leg(&mut b, [x, y, z0 as f32], floor as f32);
        }
    }
    b.label([cx, key_front as f32 - 0.1, floor as f32 + 0.08], "Pedals", "Player");
    b.s.body = b.bounds();

    // The microphones and their stands.
    if o.mics {
        for (i, &(x, y, z)) in sb.mics.iter().enumerate() {
            let c = MIC[i];
            let p = place.at(x, y, z);
            let r = 0.025f32;
            let ax = [[r, 0.0, 0.0], [0.0, r, 0.0], [0.0, 0.0, r]];
            // An octahedron.
            for sx in [-1.0f32, 1.0] {
                for sy in [-1.0f32, 1.0] {
                    for sz in [-1.0f32, 1.0] {
                        let (a, bb, cc) = (scale(ax[0], sx), scale(ax[1], sy), scale(ax[2], sz));
                        let mut t = [add(p, a), add(p, bb), add(p, cc)];
                        if sx * sy * sz < 0.0 {
                            t.swap(1, 2);
                        }
                        b.tri(false, t, [rgba(c, 1.0); 3]);
                    }
                }
            }
            let foot = [p[0], p[1], floor as f32];
            b.line(p, foot, rgba(c, 0.7));
            b.label(p, if i == 0 { "Mic L" } else { "Mic R" }, "Microphones");
        }
    }

    b.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options() -> Options<'static> {
        Options { key: 60, lit: &[], lid: true, frame: true, mics: true, ink: Color32::WHITE }
    }

    #[test]
    fn the_default_grand_is_whole_and_to_scale() {
        let m = PianoModel::default();
        let s = build(&m, None, &options());
        assert_eq!(s.tris.len() % 3, 0);
        assert_eq!(s.lines.len() % 2, 0);
        assert!(s.tris.iter().chain(&s.lines).all(|v| v.pos.iter().chain(&v.normal).all(|x| x.is_finite())));
        // No flattened box: every face has an area.
        for t in s.tris.as_chunks::<3>().0 {
            let area = dot(cross(sub(t[1].pos, t[0].pos), sub(t[2].pos, t[0].pos)), cross(sub(t[1].pos, t[0].pos), sub(t[2].pos, t[0].pos)));
            assert!(area > 1e-14, "flat triangle {t:?}");
        }
        let sb = m.soundboard.unwrap();
        // The board spans its a × b, the rim around it.
        assert!((s.max[1] - s.min[1]) as f64 >= sb.a, "{:?} {:?}", s.min, s.max);
        assert!((s.max[0] - s.min[0]) as f64 >= sb.b);
        assert!(s.labels.iter().any(|l| l.text == "Soundboard"));
        // Strings cross: the bass ones start across the keyboard's bass end.
        let ((_, a0), (_, b0), _) = string_ends(&m, &sb, hammer_x(&m, &sb), 21);
        assert!(a0 > b0, "A0 from y = {a0} to {b0}");
    }

    #[test]
    fn the_strings_of_a_bridge_do_not_cross_each_other() {
        let m = PianoModel::default();
        let sb = m.soundboard.unwrap().resolved();
        // Two segments cross when each one's ends lie on either side of the other.
        let crosses = |(p, q): ((f64, f64), (f64, f64)), (r, s): ((f64, f64), (f64, f64))| {
            cross2(p, q, r) * cross2(p, q, s) < 0.0 && cross2(r, s, p) * cross2(r, s, q) < 0.0
        };
        let hx = hammer_x(&m, &sb);
        for (lo, hi) in [(21, sb.main_bridge_from), (sb.main_bridge_from, 109)] {
            for i in lo..hi {
                for j in i + 1..hi {
                    let (a, b) = (string_ends(&m, &sb, hx, i), string_ends(&m, &sb, hx, j));
                    assert!(!crosses((a.0, a.1), (b.0, b.1)), "keys {i} and {j} cross");
                }
            }
        }
    }

    #[test]
    fn the_hammers_are_in_line_the_agraffes_on_a_diagonal() {
        let m = PianoModel::default();
        let sb = m.soundboard.unwrap().resolved();
        let hx = hammer_x(&m, &sb);
        let ends: Vec<_> = (21..=108u8).map(|k| (k, string_ends(&m, &sb, hx, k))).collect();
        for &(k, ((ax, ay), (bx, by), _)) in &ends {
            let s = m.note(k).strike_pos;
            assert!((ax + s * (bx - ax) - hx).abs() < 1e-12, "key {k} struck off the line");
            assert!((ay + s * (by - ay) - key_across(&sb, k)).abs() < 1e-12, "key {k} struck off its key");
            assert!(ax < hx, "key {k}'s agraffe at {ax}");
        }
        // The hammers in front of the board; the bass's agraffe in front,
        // the treble's well behind it, just in front of the hammers.
        let (a0, c8) = (ends[0].1.0.0, ends[87].1.0.0);
        assert!(hx <= -HAMMER_CLEAR, "{hx}");
        assert!(ends[0].1.0.1 > key_across(&sb, 21) + 0.03, "A0's agraffe not to the bass side");
        assert!(c8 - a0 > 0.1 && hx - c8 < 0.01, "A0 at {a0}, C8 at {c8}, hammers at {hx}");
        // A tuning pin per string, in front of the agraffes and behind the keys.
        let strings: usize = (21..=108u8).map(|k| m.note(k).strings).sum();
        let pins = build(&m, None, &options()).tris.iter().filter(|v| v.color == rgba(PIN, 1.0)).count();
        assert_eq!(pins, strings * (6 * 2 + 2 * 4) * 3);
    }

    #[test]
    fn an_outlined_board_is_drawn_within_its_outline() {
        let mut m = PianoModel::default();
        let sb = m.soundboard.as_mut().unwrap();
        sb.outline = SoundboardParams::GRAND_OUTLINE;
        let sb = *sb;
        let outline = board_outline(&sb);
        let area = signed_area(&outline);
        assert!(area > 0.0 && area < 0.9 * sb.a * sb.b, "{area}");
        // The grid's pieces cover the outline, the underside too.
        let pieces: f64 = (0..10)
            .flat_map(|i| (0..10).map(move |j| (i, j)))
            .map(|(i, j)| {
                let (x0, y0, w, h) = (sb.a * i as f64 / 10.0, sb.b * j as f64 / 10.0, sb.a / 10.0, sb.b / 10.0);
                signed_area(&clip(&outline, &[(x0, y0), (x0 + w, y0), (x0 + w, y0 + h), (x0, y0 + h)]))
            })
            .sum();
        assert!((pieces - area).abs() < 1e-9 * area.max(1.0), "{pieces} vs {area}");
        let under: f64 = triangulate(&outline).iter().map(|t| signed_area(&t.map(|i| outline[i]))).sum();
        assert!((under - area).abs() < 1e-9, "{under} vs {area}");
        // No board drawn outside it: the board's top lies within the outline.
        let s = build(&m, None, &options());
        let corner = Place { vertical: false, top: 0.0 }.at(sb.a * 0.95, sb.b * 0.05, 0.0);
        assert!(!s.tris.iter().any(|v| v.pos[2] == 0.0 && (v.pos[0] - corner[0]).abs() < 0.02 && (v.pos[1] - corner[1]).abs() < 0.02));
    }

    #[test]
    fn the_rim_is_one_wall_without_overlaps() {
        let mut m = PianoModel::default();
        m.soundboard.as_mut().unwrap().outline = SoundboardParams::GRAND_OUTLINE;
        let sb = m.soundboard.unwrap();
        let outline = board_outline(&sb);
        let mut b = Builder { s: Scene::default(), place: Place { vertical: false, top: 0.0 } };
        let (t, z0, z1) = (0.075, -0.25, 0.05);
        b.wall(&outline, true, 0.0, t, z0, &|_| z1, CASE);
        // Its top is the ring between the outline and its offset: its area
        // is the offset polygon's minus the outline's, no more (no overlap).
        let top: f64 = b.s.tris.as_chunks::<3>().0.iter().filter(|t| t.iter().all(|v| (v.pos[2] - z1 as f32).abs() < 1e-6)).map(|t| {
            let p = t.map(|v| (v.pos[0] as f64, v.pos[1] as f64));
            signed_area(&p).abs()
        }).sum();
        let perimeter: f64 = outline.iter().zip(outline.iter().cycle().skip(1)).map(|(p, q)| (q.0 - p.0).hypot(q.1 - p.1)).sum();
        // A ring t wide around a convex-ish outline: perimeter·t plus the
        // corners' mitres, well under the overlapping blocks' sum.
        assert!(top > perimeter * t && top < perimeter * t + 8.0 * t * t, "{top} vs {}", perimeter * t);
        // Every normal is a unit vector.
        assert!(b.s.tris.iter().all(|v| (dot(v.normal, v.normal) - 1.0).abs() < 1e-4));
    }

    #[test]
    fn faces_point_outwards() {
        let mut b = Builder { s: Scene::default(), place: Place { vertical: false, top: 0.0 } };
        b.cuboid([0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0], Color32::WHITE);
        for t in b.s.tris.chunks(3) {
            let c = scale(add(add(t[0].pos, t[1].pos), t[2].pos), 1.0 / 3.0);
            assert!(dot(t[0].normal, sub(c, [0.5; 3])) > 0.0, "{t:?}");
        }
    }

    #[test]
    fn live_state_shows() {
        let m = PianoModel::default();
        let mut obs = Observation::default();
        obs.keys[60].down = true;
        obs.keys[60].damper = 0.0;
        let mut levels = [0.0; 128];
        levels[60] = 1.0;
        let field = vec![0.5; BOARD_GRID.0 * BOARD_GRID.1];
        let live = Live { obs: &obs, keys: &levels, board: Some(&field), time: 0.1 };
        let o = Options { key: 40, ..options() };
        let still = build(&m, None, &o);
        let playing = build(&m, Some(&live), &o);
        // The struck key's strings drawn in segments, its damper lifted.
        assert!(playing.lines.len() > still.lines.len());
        assert_ne!(playing.tris, still.tris);
    }
}
