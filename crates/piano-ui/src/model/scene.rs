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
/// The keyboard: depth of the white keys, keybed under the keys (grand,
/// z), the floor.
const KEY_DEPTH: f64 = 0.15;
const KEY_Z: f64 = -0.02;
const GRAND_FLOOR: f64 = -0.72;
/// An upright's board bottom, keyboard height and distance in front.
const UPRIGHT_BASE: f64 = 0.2;
const UPRIGHT_KEYS_Z: f64 = 0.72;
const UPRIGHT_KEYS_Y: f64 = -0.27;

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
    // is), a panel across its front behind the keys, and the belly rail
    // under the board's front edge.
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
        b.plate_box((case_front - FRONT_T, across_lo - t, z0), (case_front, across_hi + t, PIN_BLOCK_TOP), case);
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
    // front.
    let keys_back = case_front - FRONT_T;
    let (klo, khi) = keyboard_span(&sb);
    let white = (khi - klo) / 52.0;
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
        let (y_back, z) = if vertical { (UPRIGHT_KEYS_Y, UPRIGHT_KEYS_Z) } else { (keys_back, KEY_Z) };
        let front = y_back - KEY_DEPTH;
        let o3 = [-y1 as f32, front as f32 + (KEY_DEPTH - depth) as f32, (z + zb + drop) as f32];
        b.cuboid(o3, [(y1 - y0) as f32, 0.0, 0.0], [0.0, depth as f32, 0.0], [0.0, 0.0, h as f32], c);
    }
    let (y_back, kz) = if vertical { (UPRIGHT_KEYS_Y, UPRIGHT_KEYS_Z) } else { (keys_back, KEY_Z) };
    // The keybed and key slip.
    b.cuboid([-(khi + 0.06) as f32, (y_back - KEY_DEPTH - 0.02) as f32, (kz - 0.05) as f32], [(khi - klo + 0.12) as f32, 0.0, 0.0], [0.0, (KEY_DEPTH + 0.02) as f32, 0.0], [0.0, 0.0, 0.05], CASE);
    // The cheeks at both ends of the keys.
    for x in [-(khi + 0.06), -(klo - 0.01)] {
        b.cuboid([x as f32, (y_back - KEY_DEPTH) as f32, (kz - 0.01) as f32], [0.05, 0.0, 0.0], [0.0, KEY_DEPTH as f32, 0.0], [0.0, 0.0, 0.06], CASE);
    }
    b.label([-((klo + khi) / 2.0) as f32, (y_back - KEY_DEPTH) as f32, (kz + 0.03) as f32], "Keyboard", "Player");

    // Pedals: soft, sostenuto, sustain from left to right.
    let floor = if vertical { 0.0 } else { GRAND_FLOOR };
    let cx = -((klo + khi) / 2.0) as f32;
    let values = obs.map_or([0.0; 3], |o| [o.soft, if o.sostenuto { 1.0 } else { 0.0 }, o.sustain]);
    let parts = [Part::UnaCorda, Part::UnaCorda, Part::SustainPedal];
    for (i, v) in values.iter().enumerate() {
        let x = cx + (i as f32 - 1.0) * 0.07 - 0.02;
        let y = (y_back - KEY_DEPTH) as f32 - 0.02;
        let z = floor as f32 + 0.05 - 0.02 * v;
        let c = if *v > 0.05 || (i != 1 && lit(parts[i])) { sketches::HIGHLIGHT } else { IRON };
        b.cuboid([x, y - 0.12, z], [0.04, 0.0, 0.0], [0.0, 0.14, 0.0], [0.0, 0.0, 0.012], c);
    }
    if !vertical {
        // The pedals' lyre, and three legs.
        b.cuboid([cx - 0.12, (y_back - KEY_DEPTH) as f32 + 0.0, floor as f32 + 0.06], [0.24, 0.0, 0.0], [0.0, 0.05, 0.0], [0.0, 0.0, (kz - 0.05 - floor - 0.06) as f32], CASE);
        for (x, y) in [(0.12, 0.05), (0.12, sb.b - 0.05), (sb.a - 0.12, sb.b * 0.5)] {
            let p = place.at(x, y, z0);
            b.cuboid([p[0] - 0.04, p[1] - 0.04, floor as f32], [0.08, 0.0, 0.0], [0.0, 0.08, 0.0], [0.0, 0.0, p[2] - floor as f32], CASE);
        }
    }
    b.label([cx, (y_back - KEY_DEPTH) as f32 - 0.1, floor as f32 + 0.08], "Pedals", "Player");
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
            let foot = if vertical { [p[0], p[1], 0.0] } else { [p[0], p[1], GRAND_FLOOR as f32] };
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
