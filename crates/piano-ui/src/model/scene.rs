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
//! What the model has no geometry for is drawn plainly: the board, the
//! case and the lid follow the model's outline (or its rectangle), its
//! polygon drawn as the curve it stands for ([`rounded`]); the strings run from their key's place across the agraffes to their bridge
//! point, which crosses the bass over the tenor as on overstrung pianos;
//! legs, keyboard and pedals only situate the instrument.

use egui::Color32;
use piano_core::params::PianoModel;
use piano_core::soundboard::{Outline, SoundboardParams, LID_OVERHANG};
use piano_core::observe::KeyState;
use piano_core::Observation;

use super::cam::{add, cross, dot, norm, scale, sub, V3};
use crate::sketches::{self, Part};

mod action;
mod lettering;
mod name;
mod plate;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct Vertex {
    pub pos: V3,
    pub normal: V3,
    /// sRGB, 0–1, straight alpha.
    pub color: [f32; 4],
    /// What it is made of ([`Material`] as a number), for the realistic
    /// view.
    pub mat: f32,
    /// A line's wire diameter (m), 0 for a stroke of the drawing.
    pub width: f32,
}

/// What a part is made of, for the 3D view's realistic look (the plain
/// views keep to its colour): the shader's numbering. Most parts' come
/// from the colour they are drawn in ([`material_of`]); a colour that
/// stands for no material (a part pointed at, a level) takes the one the
/// part being drawn sets ([`Builder::mat`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Material {
    /// A satin paint.
    #[default]
    Paint = 0,
    /// The case, lid, legs and lyre: polished black lacquer.
    Lacquer = 1,
    /// The soundboard: varnished spruce.
    Spruce = 2,
    /// Keys' sticks, the action, bridges: maple, hornbeam.
    Wood = 3,
    /// The plate: cast iron in gold bronze paint.
    Iron = 4,
    Steel = 5,
    /// The wound strings' copper.
    Copper = 6,
    Brass = 7,
    /// Felts and cloths.
    Felt = 8,
    Leather = 9,
    /// The white keys' tops, the black keys.
    KeyTop = 10,
    Ebony = 11,
    /// The keys' leads.
    Lead = 12,
    /// The name's gold leaf.
    Gilt = 13,
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
    /// The lid: see-through in the drawing (drawn as [`Self::glass`]),
    /// solid lacquer in the realistic look.
    pub lid: Vec<Vertex>,
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
    /// The floor's height (m).
    pub floor: f32,
    /// The soundboard's extent.
    pub board: (V3, V3),
    /// The engine's state laid on it: the board coloured by [`heat`].
    pub live: bool,
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
    /// The case (rim, arms, belly rail) drawn, or left out to show the
    /// action.
    pub case: bool,
    /// A grand's fallboard shut over the keys.
    pub fallboard: bool,
    /// How solid a grand's front is above its fallboard, up to the case's
    /// top, and the cover from there over the pin block (0 left out … 1
    /// solid).
    pub front: f32,
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
/// A unison's strings across, centre to centre.
const UNISON_GAP: f64 = 0.0025;
/// A grand's hammer line in front of its board: room for its hammers,
/// leaning back at rest and forward on the strings, to swing past the
/// board's edge, and for the dampers' wires behind them ([`action`]); the
/// pin block's back edge in front of it.
const HAMMER_CLEAR: f64 = 0.045;
const BLOCK_CLEAR: f64 = 0.015;
/// The tuning pins (7 mm), one per string in front of the agraffes (see
/// [`tuning_pins`]): the first row's distance, the rows' spacing.
const PIN_R: f64 = 0.0035;
const PIN_GAP: f64 = 0.025;
const PIN_ROW: f64 = 0.016;
/// A pin's lean away from its string's pull (rad, some 4°); its coil
/// (three turns of wire) below the strings' height, its height and the
/// wire's thickness; its square head's height, its top above the coil;
/// the bushing it stands in, radius and height.
const PIN_LEAN: f64 = 0.07;
const PIN_COIL_DROP: f64 = 0.007;
const PIN_COIL_H: f64 = 0.003;
const PIN_WIRE: f64 = 0.001;
const PIN_HEAD: f64 = 0.011;
const PIN_TOP: f64 = 0.02;
const PIN_BUSH_R: f64 = 0.0055;
const PIN_BUSH_H: f64 = 0.0015;
/// An agraffe, as they are made: a hexagonal flange screwed into the
/// plate, a neck, and a head the unison's strings go through, its flats
/// facing along them: the head's half-width, and its growth per string
/// beyond the first; the flange's radius beyond the head's and height;
/// the head's extent below and above the strings; the holes' radius; the
/// longest neck, beyond which the agraffe stands on a boss of the plate
/// (the bass's, its strings higher), and the boss's radius beyond the
/// flange's.
const AGRAFFE_HEAD: f64 = 0.0028;
const AGRAFFE_HEAD_PER: f64 = 0.0009;
const AGRAFFE_FLANGE: f64 = 0.0012;
const AGRAFFE_FLANGE_H: f64 = 0.003;
const AGRAFFE_BELOW: f64 = 0.0045;
const AGRAFFE_ABOVE: f64 = 0.0035;
const AGRAFFE_HOLE_R: f64 = 0.0006;
const AGRAFFE_NECK: f64 = 0.004;
const AGRAFFE_BOSS: f64 = 0.0025;
/// The felt strip on the plate under the strings' dead lengths, between
/// the agraffes and the pins: its middle in front of the agraffes, width
/// and thickness.
const UNDER_FELT_AT: f64 = 0.0125;
const UNDER_FELT_W: f64 = 0.012;
const UNDER_FELT_T: f64 = 0.004;
/// The hitch pins: their radius, how far they stand inside the case's wall
/// along the string, the rear lengths (a few cm in the top treble, growing
/// with the speaking length to some 20 cm in the bass, as on grands) and
/// how much lower they hold the strings than the bridge.
const HITCH_R: f64 = 0.0025;
const HITCH_INSET: f64 = 0.05;
const HITCH_MIN: f64 = 0.03;
const HITCH_BASE: f64 = 0.04;
const HITCH_SHARE: f64 = 0.1;
const HITCH_DROP: f64 = 0.006;
/// The bridge pins: radius, how far before and after the bridge's line
/// along the string, and the string's side of each.
const BRIDGE_PIN_R: f64 = 0.0009;
const BRIDGE_PIN_ALONG: f64 = 0.006;
const BRIDGE_PIN_SIDE: f64 = 0.0012;
/// The felt braid across the rear lengths: its width along the strings,
/// its thickness, and how far above and below the strings it weaves.
const BRAID_W: f32 = 0.012;
const BRAID_T: f32 = 0.0016;
const BRAID_WEAVE: f64 = 0.0035;
/// The pin block's top (under the plate) and thickness; the case's front
/// panel and the belly rail's thickness.
const PIN_BLOCK_TOP: f64 = 0.02;
const PIN_BLOCK_T: f64 = 0.032;
const FRONT_T: f64 = 0.02;
const BELLY_RAIL: f64 = 0.05;
const BELLY_AT: f64 = 0.06;
/// The keyboard: depth of the white keys. A grand's keybed, its underside
/// flush with the case's, its thickness and the keys' bottom above it; the
/// keys' top above the floor; the key slip's thickness.
const KEY_DEPTH: f64 = 0.15;
const KEYBED_T: f64 = 0.035;
const KEY_FRAME: f64 = 0.04;
const KEY_HEIGHT: f64 = 0.72;
const KEY_SLIP: f64 = 0.018;
/// A grand's cheek blocks' least width, between the keys and the case's
/// arms; the bass's agraffes' least distance from the board's bass side.
const KEY_CHEEK: f64 = 0.03;
const AGRAFFE_EDGE: f64 = 0.025;
/// An upright's board bottom, keyboard height and distance in front.
const UPRIGHT_BASE: f64 = 0.2;
const UPRIGHT_KEYS_Z: f64 = 0.72;
const UPRIGHT_KEYS_Y: f64 = -0.27;
/// The name on a grand's fallboard: its middle's height above the keys.
/// The fallboard shut: its top's height above the keys at the name board
/// and over their front, its thickness, its front edge's radius, the lip's
/// thickness in front of the keys.
const NAME_UP: f64 = 0.05;
const FALL_BACK: f64 = 0.085;
const FALL_FRONT: f64 = 0.018;
const FALL_T: f64 = 0.014;
const FALL_ROUND: f64 = 0.006;
const FALL_LIP: f64 = 0.006;
/// The cover over a grand's pin block: its thickness.
const COVER_T: f32 = 0.018;
/// The pedals' spacing across (m).
const PEDAL_PITCH: f32 = 0.07;

/// Bends up to this angle (degrees) are shaded as a curve, sharper ones as
/// corners.
const SMOOTH_DEG: f64 = 40.0;
/// An outline's bends sharper than this (degrees) stay corners in
/// [`rounded`].
const CORNER_DEG: f64 = 60.0;
/// [`rounded`]'s curves turn about this much (degrees) from one point to
/// the next.
const ROUND_STEP_DEG: f64 = 1.5;

const STEEL: Color32 = Color32::from_rgb(170, 172, 178);
const COPPER: Color32 = Color32::from_rgb(184, 115, 70);
const IRON: Color32 = Color32::from_rgb(190, 160, 90);
const BRASS: Color32 = Color32::from_rgb(222, 184, 98);
const PIN: Color32 = Color32::from_rgb(168, 170, 176);
const BUSHING: Color32 = Color32::from_rgb(150, 104, 72);
const HITCH: Color32 = Color32::from_rgb(150, 152, 160);
const BRIDGE_PIN: Color32 = Color32::from_rgb(196, 198, 206);
const CASE: Color32 = Color32::from_rgb(40, 34, 32);
const IVORY: Color32 = Color32::from_rgb(236, 232, 222);
const EBONY: Color32 = Color32::from_rgb(28, 26, 26);
/// The strip of felt behind the keys.
const FELT: Color32 = Color32::from_rgb(122, 28, 36);
/// The one under the strings' dead lengths.
const UNDER_FELT: Color32 = Color32::from_rgb(176, 48, 36);
const DAMPER_WOOD: Color32 = Color32::from_rgb(64, 46, 36);
const DAMPER_FELT: Color32 = Color32::from_rgb(226, 222, 212);
const MIC: [Color32; 2] = [Color32::from_rgb(70, 140, 220), Color32::from_rgb(220, 80, 90)];
const LYRE_ROD: Color32 = Color32::from_rgb(64, 64, 70);

/// The material each of the scene's own colours stands for.
const MATERIALS: [(Color32, Material); 19] = [
    (name::GILT, Material::Gilt),
    (STEEL, Material::Steel),
    (PIN, Material::Steel),
    (HITCH, Material::Steel),
    (BRIDGE_PIN, Material::Steel),
    (LYRE_ROD, Material::Steel),
    (COPPER, Material::Copper),
    (IRON, Material::Iron),
    (BRASS, Material::Brass),
    (BUSHING, Material::Wood),
    (DAMPER_WOOD, Material::Wood),
    (sketches::BRIDGE, Material::Wood),
    (sketches::WOOD, Material::Spruce),
    (CASE, Material::Lacquer),
    (IVORY, Material::KeyTop),
    (EBONY, Material::Ebony),
    (FELT, Material::Felt),
    (UNDER_FELT, Material::Felt),
    (DAMPER_FELT, Material::Felt),
];

/// The material the colour `c` stands for, if any.
fn material_of(c: [f32; 4]) -> Option<Material> {
    let k = |x: f32| (x * 255.0).round() as u8;
    let c = Color32::from_rgb(k(c[0]), k(c[1]), k(c[2]));
    MATERIALS.iter().chain(&action::MATERIALS).find(|(k, _)| *k == c).map(|&(_, m)| m)
}

/// A string's diameter, from its mass per length as if all steel (a
/// wound string's a little under its winding's), and its plain wire's
/// beyond the bridge and round its tuning pin (a wound string's core).
fn string_d(model: &PianoModel, m: u8) -> (f32, f32) {
    let d = 2.0 * (model.note(m).mu / (action::STEEL_DENSITY * std::f64::consts::PI)).sqrt();
    (d as f32, d.min(0.0016) as f32)
}

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
    /// What the part being drawn is made of where its colour says
    /// nothing ([`material_of`]).
    mat: Material,
    /// The last colour drawn, the material it had then and the one it
    /// gave (a colour is drawn many times over).
    last: ([f32; 4], Material, Material),
}

impl Builder {
    fn new(place: Place) -> Self {
        Self { s: Scene::default(), place, mat: Material::Paint, last: ([-1.0; 4], Material::Paint, Material::Paint) }
    }

    #[inline]
    fn vertex(&mut self, pos: V3, normal: V3, color: [f32; 4]) -> Vertex {
        if self.last.0 != color || self.last.1 != self.mat {
            self.miss(color);
        }
        Vertex { pos, normal, color, mat: self.last.2 as u8 as f32, width: 0.0 }
    }

    #[cold]
    fn miss(&mut self, color: [f32; 4]) {
        self.last = (color, self.mat, material_of(color).unwrap_or(self.mat));
    }

    fn tri(&mut self, glass: bool, p: [V3; 3], c: [[f32; 4]; 3]) {
        let n = norm(cross(sub(p[1], p[0]), sub(p[2], p[0])));
        for (pos, color) in p.into_iter().zip(c) {
            let v = self.vertex(pos, n, color);
            if glass { &mut self.s.glass } else { &mut self.s.tris }.push(v);
        }
    }

    /// A quad with a normal at each corner (smooth shading).
    fn quad_n(&mut self, p: [V3; 4], n: [V3; 4], c: [f32; 4]) {
        for k in [0, 1, 2, 0, 2, 3] {
            let v = self.vertex(p[k], n[k], c);
            self.s.tris.push(v);
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
        if n < 2 {
            return;
        }
        let segs = if closed { n } else { n - 1 };
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

    /// A flat solid from `points` (u, v), its `loops` (first point,
    /// count; outlines counter-clockwise, holes clockwise) and its face's
    /// counter-clockwise `tris`, between the heights `h.0` and `h.1`;
    /// `at(u, v, h)` places it. Its bottom face is left out on the
    /// surface it stands on (h 0).
    #[allow(clippy::too_many_arguments)]
    fn prism(&mut self, points: &[(f64, f64)], loops: &[(usize, usize)], tris: &[[usize; 3]], h: (f64, f64), at: &dyn Fn(f64, f64, f64) -> V3, c: Color32) {
        let c = rgba(c, 1.0);
        let up = sub(at(0.0, 0.0, 1.0), at(0.0, 0.0, 0.0));
        let faces = if h.0 > 0.0 { vec![(h.1, 1.0f32), (h.0, -1.0)] } else { vec![(h.1, 1.0)] };
        for t in tris {
            let [p, q, r] = t.map(|i| points[i]);
            // Its slivers' rounding aside.
            if ((q.0 - p.0) * (r.1 - p.1) - (r.0 - p.0) * (q.1 - p.1)).abs() < 1e-14 {
                continue;
            }
            for &(z, side) in &faces {
                let mut w = [p, q, r].map(|p| at(p.0, p.1, z));
                if dot(cross(sub(w[1], w[0]), sub(w[2], w[0])), up) * side < 0.0 {
                    w.swap(1, 2);
                }
                self.tri(false, w, [c; 3]);
            }
        }
        for &(first, count) in loops {
            for k in 0..count {
                let (p, q) = (points[first + k], points[first + (k + 1) % count]);
                let (du, dv) = (q.0 - p.0, q.1 - p.1);
                let l = du.hypot(dv).max(1e-12);
                // Outwards: to the right of the way round.
                let n = norm(sub(at(p.0 + dv / l, p.1 - du / l, h.0), at(p.0, p.1, h.0)));
                self.quad_n([at(p.0, p.1, h.0), at(q.0, q.1, h.0), at(q.0, q.1, h.1), at(p.0, p.1, h.1)], [n; 4], c);
            }
        }
    }

    /// Makes what was drawn solid since the `from`th vertex see-through,
    /// as solid as `alpha` (left out at 0).
    fn see_through(&mut self, from: usize, alpha: f32) {
        if alpha >= 1.0 {
            return;
        }
        let drawn = self.s.tris.split_off(from);
        if alpha > 0.0 {
            self.s.glass.extend(drawn.into_iter().map(|v| Vertex { color: [v.color[0], v.color[1], v.color[2], v.color[3] * alpha], ..v }));
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

    /// A turned part on the world axis from `base` along the unit `axis`,
    /// its square sections' flats facing `side`: each ring its height
    /// along the axis, its radius (a square's half-width) and whether it is
    /// square (its corners rounded) rather than round; each band between
    /// two rings its own colour, shaded smooth around a round one and flat
    /// on a square one or a step; the top capped.
    fn lathe(&mut self, base: V3, axis: V3, side: V3, rings: &[(f32, f32, bool)], colours: &[Color32]) {
        const SIDES: usize = 12;
        let u = norm(sub(side, scale(axis, dot(side, axis))));
        let v = cross(axis, u);
        // Around, at unit radius: a round section's points (its normals),
        // and a square's (a superellipse).
        let round: [V3; SIDES] = std::array::from_fn(|k| {
            let (s, c) = (std::f32::consts::TAU * (k as f32 + 0.5) / SIDES as f32).sin_cos();
            add(scale(u, c), scale(v, s))
        });
        let square: [V3; SIDES] = std::array::from_fn(|k| {
            let (s, c) = (std::f32::consts::TAU * (k as f32 + 0.5) / SIDES as f32).sin_cos();
            scale(round[k], 1.0 / (c.powi(6) + s.powi(6)).powf(1.0 / 6.0))
        });
        let points: Vec<[V3; SIDES]> = rings
            .iter()
            .map(|&(h, r, sq)| {
                let centre = add(base, scale(axis, h));
                std::array::from_fn(|k| add(centre, scale(if sq { square[k] } else { round[k] }, r)))
            })
            .collect();
        for i in 0..rings.len() - 1 {
            let c = rgba(colours[i.min(colours.len() - 1)], 1.0);
            let ((h0, r0, sq0), (h1, r1, sq1)) = (rings[i], rings[i + 1]);
            let step = (h1 - h0).abs() < 0.2 * (r1 - r0).abs();
            // The profile's slope tilts a round band's normals off the radius.
            let slope = |k: usize| norm(sub(scale(round[k % SIDES], h1 - h0), scale(axis, r1 - r0)));
            for k in 0..SIDES {
                let l = (k + 1) % SIDES;
                let p = [points[i][k], points[i][l], points[i + 1][l], points[i + 1][k]];
                if step || sq0 || sq1 {
                    let n = norm(cross(sub(p[1], p[0]), sub(p[3], p[0])));
                    let n = if dot(n, n) > 0.5 { n } else { norm(cross(sub(p[2], p[3]), sub(p[0], p[3]))) };
                    self.quad_n(p, [n; 4], c);
                } else {
                    self.quad_n(p, [slope(k), slope(l), slope(l), slope(k)], c);
                }
            }
        }
        let last = rings.len() - 1;
        let top = add(base, scale(axis, rings[last].0));
        let c = rgba(colours[last.min(colours.len()) - 1], 1.0);
        for k in 0..SIDES {
            self.tri(false, [top, points[last][k], points[last][(k + 1) % SIDES]], [c; 3]);
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
        self.wire(a, b, c, 0.0);
    }

    /// A line that is a wire `d` metres thick.
    fn wire(&mut self, a: V3, b: V3, c: [f32; 4], d: f32) {
        for pos in [a, b] {
            let v = Vertex { width: d, ..self.vertex(pos, [0.0, 0.0, 1.0], c) };
            self.s.lines.push(v);
        }
    }

    fn label(&mut self, at: V3, text: impl Into<String>, page: &'static str) {
        self.s.labels.push(Label { at, text: text.into(), page });
    }

    /// The extent of everything drawn so far.
    fn bounds(&self) -> (V3, V3) {
        let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
        for v in self.s.tris.iter().chain(&self.s.glass).chain(&self.s.lid).chain(&self.s.lines) {
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

/// The corner `c` of a profile from `a` on to `z`, rounded to radius `r`:
/// the arc's points from the one side to the other.
fn fillet(a: (f64, f64), c: (f64, f64), z: (f64, f64), r: f64) -> Vec<(f64, f64)> {
    let unit = |p: (f64, f64)| {
        let (x, y) = (p.0 - c.0, p.1 - c.1);
        let l = x.hypot(y).max(1e-12);
        (x / l, y / l)
    };
    let (u1, u2) = (unit(a), unit(z));
    let half = (u1.0 * u2.0 + u1.1 * u2.1).clamp(-1.0, 1.0).acos() / 2.0;
    let (t, d) = (r / half.tan(), r / half.sin());
    let bis = unit((c.0 + u1.0 + u2.0, c.1 + u1.1 + u2.1));
    let centre = (c.0 + bis.0 * d, c.1 + bis.1 * d);
    let angle = |p: (f64, f64)| (p.1 - centre.1).atan2(p.0 - centre.0);
    let (a0, mut a1) = (angle((c.0 + u1.0 * t, c.1 + u1.1 * t)), angle((c.0 + u2.0 * t, c.1 + u2.1 * t)));
    if a1 - a0 > std::f64::consts::PI {
        a1 -= std::f64::consts::TAU;
    } else if a0 - a1 > std::f64::consts::PI {
        a1 += std::f64::consts::TAU;
    }
    (0..=6).map(|k| a0 + (a1 - a0) * k as f64 / 6.0).map(|a| (centre.0 + r * a.cos(), centre.1 + r * a.sin())).collect()
}

/// The board's outline in plate metres, counter-clockwise: the model's
/// own (`soundboard.outline`), [`rounded`], or the full a × b rectangle.
pub fn board_outline(sb: &SoundboardParams) -> Vec<(f64, f64)> {
    let mut p = Outline::new(sb).map_or_else(|| vec![(0.0, 0.0), (sb.a, 0.0), (sb.a, sb.b), (0.0, sb.b)], |o| o.points);
    if signed_area(&p) < 0.0 {
        p.reverse();
    }
    rounded(&p)
}

/// The closed polygon `p` drawn as the curve it stands for (a grand's
/// bentside and tail are a few vertices in the model): through every
/// vertex, its corners (bends over [`CORNER_DEG`]) kept and its edges along
/// its bounding box straight; every other edge a cubic, tangent at each end
/// to the straight edge beside it, else to its two edges' bisector (along
/// the box's side at a vertex on it), split every [`ROUND_STEP_DEG`] or so
/// of turn. It stays within the box: a board's outline within its a × b.
pub fn rounded(p: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let n = p.len();
    if n < 3 {
        return p.to_vec();
    }
    let (lo, hi) = p.iter().fold(((f64::MAX, f64::MAX), (f64::MIN, f64::MIN)), |(l, h), q| ((l.0.min(q.0), l.1.min(q.1)), (h.0.max(q.0), h.1.max(q.1))));
    let edge = |k: usize| {
        let (a, b) = (p[k], p[(k + 1) % n]);
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let l = dx.hypot(dy);
        ((dx / l.max(1e-12), dy / l.max(1e-12)), l)
    };
    let prev = |i: usize| (i + n - 1) % n;
    let corner: Vec<bool> = (0..n)
        .map(|i| {
            let (u, v) = (edge(prev(i)).0, edge(i).0);
            u.0 * v.0 + u.1 * v.1 < CORNER_DEG.to_radians().cos()
        })
        .collect();
    let side = |a: f64, b: f64, lo: f64, hi: f64| (a - b).abs() < 1e-9 && ((a - lo).abs() < 1e-9 || (a - hi).abs() < 1e-9);
    let straight: Vec<bool> = (0..n)
        .map(|k| {
            let (a, b) = (p[k], p[(k + 1) % n]);
            (corner[k] && corner[(k + 1) % n]) || side(a.0, b.0, lo.0, hi.0) || side(a.1, b.1, lo.1, hi.1) || edge(k).1 < 1e-9
        })
        .collect();
    // The tangent at a vertex that is not a corner.
    let tangent = |i: usize| -> Option<(f64, f64)> {
        let (u, v) = (edge(prev(i)).0, edge(i).0);
        let on = |a: f64, lo: f64, hi: f64| (a - lo).abs() < 1e-9 || (a - hi).abs() < 1e-9;
        match (corner[i], straight[prev(i)], straight[i]) {
            (true, _, _) => None,
            (_, true, false) => Some(u),
            (_, false, true) => Some(v),
            _ => {
                let (mx, my) = (u.0 + v.0, u.1 + v.1);
                Some(match (on(p[i].0, lo.0, hi.0), on(p[i].1, lo.1, hi.1)) {
                    (true, false) => (0.0, my.signum()),
                    (false, true) => (mx.signum(), 0.0),
                    _ => {
                        let l = mx.hypot(my).max(1e-12);
                        (mx / l, my / l)
                    }
                })
            }
        }
    };
    let mut out = Vec::with_capacity(4 * n);
    for k in 0..n {
        out.push(p[k]);
        if straight[k] {
            continue;
        }
        let (j, (c, l)) = ((k + 1) % n, edge(k));
        let (t0, t1) = (tangent(k).unwrap_or(c), tangent(j).unwrap_or(c));
        let off = |t: (f64, f64)| (t.0 * c.0 + t.1 * c.1).clamp(-1.0, 1.0).acos();
        let pieces = ((off(t0) + off(t1)).to_degrees() / ROUND_STEP_DEG).ceil().clamp(1.0, 48.0) as usize;
        for s in 1..pieces {
            let t = s as f64 / pieces as f64;
            let (t2, t3) = (t * t, t * t * t);
            let (h0, h1, h2, h3) = (2.0 * t3 - 3.0 * t2 + 1.0, t3 - 2.0 * t2 + t, 3.0 * t2 - 2.0 * t3, t3 - t2);
            let at = |a: f64, ta: f64, b: f64, tb: f64| h0 * a + h1 * l * ta + h2 * b + h3 * l * tb;
            out.push((at(p[k].0, t0.0, p[j].0, t1.0).clamp(lo.0, hi.0), at(p[k].1, t0.1, p[j].1, t1.1).clamp(lo.1, hi.1)));
        }
    }
    out
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
    rounded(&p)
}

pub fn signed_area(p: &[(f64, f64)]) -> f64 {
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

/// Where each key's hammer strikes its strings across (m, plate y): evenly
/// from one semitone to the next, as on a grand (whose keys are not: the
/// action's shanks fan out to the strings), C8 and the main bridge's
/// lowest key in line with where their strings cross the keys' line
/// ([`SoundboardParams::key_across`]), the bass beyond at the same pitch,
/// but closer together if that would bring A0's agraffe within
/// [`AGRAFFE_EDGE`] of the board's bass side (the case's wall): its
/// string runs on from its strike, away from its bridge, to its agraffe.
/// The engine has no place across for the bass's hammers (it drives the
/// frame in line with their bridge points).
pub fn key_across(model: &PianoModel, sb: &SoundboardParams, m: u8) -> f64 {
    let y = |m: u8| sb.key_across(m);
    let lo_key = sb.main_bridge_from.min(107);
    let pitch = (y(lo_key) - y(108)) / (108 - lo_key) as f64;
    if m >= lo_key || pitch <= 0.0 {
        return y(108) + pitch * (108 - m) as f64;
    }
    let s = model.note(21).strike_pos;
    let a0 = (1.0 - s) * (sb.b - AGRAFFE_EDGE) + s * sb.bridge_point(21).1;
    let bass = ((a0 - y(lo_key)) / (lo_key - 21) as f64).clamp(0.5 * pitch, pitch);
    y(lo_key) + bass * (lo_key - m) as f64
}

/// The keyboard's extent across (plate y): C8 and the main bridge's lowest
/// key in line with their strings, the bass keys beyond, but moved
/// towards the treble (or the bass) as far as leaves room for cheek
/// blocks [`KEY_CHEEK`] wide inside the case's walls (centred if there is
/// not that much room); the keys crank to their hammers behind it.
fn keyboard_span(sb: &SoundboardParams) -> (f64, f64) {
    let y = |m: u8| sb.key_across(m);
    let mid = |m: u8| {
        let (x, w) = sketches::key_span(m);
        (x + w / 2.0) as f64
    };
    let lo_key = sb.main_bridge_from.min(107);
    let white = (y(lo_key) - y(108)) / (mid(108) - mid(lo_key));
    let hi = y(108) + mid(108) * white;
    let lo = hi - 52.0 * white;
    let (lo, hi) = (lo.min(hi), lo.max(hi));
    let room = (KEY_CHEEK, sb.b - KEY_CHEEK);
    let shift = if hi - lo > room.1 - room.0 {
        (sb.b - lo - hi) / 2.0
    } else {
        (room.0 - lo).max(0.0) + (room.1 - hi).min(0.0)
    };
    (lo + shift, hi + shift)
}

/// A string's rear length (m), from its bridge point `p` (plate) on along
/// its direction `u` to its hitch pin: [`HITCH_BASE`] and [`HITCH_SHARE`]
/// of its speaking length, but [`HITCH_INSET`] short of the board's
/// outline (the case's wall), and at least [`HITCH_MIN`].
pub fn hitch_length(outline: &[(f64, f64)], p: (f64, f64), u: (f64, f64), speaking: f64) -> f64 {
    let n = outline.len();
    let mut best = HITCH_BASE + HITCH_SHARE * speaking;
    for i in 0..n {
        let (a, b) = (outline[i], outline[(i + 1) % n]);
        let e = (b.0 - a.0, b.1 - a.1);
        let denom = u.0 * e.1 - u.1 * e.0;
        let len = e.0.hypot(e.1);
        if denom.abs() < 1e-12 || len < 1e-12 {
            continue;
        }
        let w = (a.0 - p.0, a.1 - p.1);
        let (t, s) = ((w.0 * e.1 - w.1 * e.0) / denom, (w.0 * u.1 - w.1 * u.0) / denom);
        if t > 0.0 && (0.0..=1.0).contains(&s) {
            // The inset measured square to the wall, along the string.
            let cos = (denom / len).abs().max(0.25);
            best = best.min(t - HITCH_INSET / cos);
        }
    }
    best.max(HITCH_MIN)
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

/// A key's string's ends in plate coordinates (its agraffe, its bridge
/// point) and its height.
pub type StringEnds = ((f64, f64), (f64, f64), f64);

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
    ((back(hx, bx), back(key_across(model, sb, m), by)), (bx, by), z)
}

/// A string's tuning pin: where it stands on the plate, the agraffe hole
/// its string runs to from it, and the key's string's direction there.
#[derive(Clone, Copy, Debug)]
pub struct TuningPin {
    pub at: (f64, f64),
    pub hole: (f64, f64),
    pub dir: (f64, f64),
}

/// The tuning pins, one per string from the bass up (each key's strings
/// across in turn), as grands have them: in the pin block in front of the
/// agraffes, spread evenly across (each key's share of an even pitch,
/// whatever the keys' own spacing), a unison's on a diagonal one row
/// further from the agraffes per string (single strings alternating
/// between two rows, the one next to the first unison in front, clear of
/// that unison's nearer pin a row back), so that the farther pins'
/// strings pass between the nearer pins. `ends` are the keys' [`string_ends`] from A0.
pub fn tuning_pins(model: &PianoModel, sb: &SoundboardParams, ends: &[StringEnds]) -> Vec<TuningPin> {
    let ay = |m: u8| ends[(m - 21) as usize].0.1;
    let bass = |m: u8| m < sb.main_bridge_from;
    let unison = (21..=108u8).find(|&m| model.note(m).strings > 1).unwrap_or(109);
    let mut pins = Vec::new();
    for m in 21..=108u8 {
        let ((ax, _), (bx, by), _) = ends[(m - 21) as usize];
        // The keys' places across over an octave around it on its bridge,
        // fitted with a line: its even place and the pitch there.
        let keys: Vec<f64> = (m.saturating_sub(6).max(21)..=(m + 6).min(108)).filter(|&k| bass(k) == bass(m)).map(f64::from).collect();
        let n = keys.len() as f64;
        let (mk, my) = (keys.iter().sum::<f64>() / n, keys.iter().map(|&k| ay(k as u8)).sum::<f64>() / n);
        let (sxy, sxx) = keys.iter().fold((0.0, 0.0), |(a, b), &k| (a + (k - mk) * (ay(k as u8) - my), b + (k - mk) * (k - mk)));
        let slope = if sxx > 0.0 { sxy / sxx } else { 0.0 };
        let (even, pitch) = (my + slope * (m as f64 - mk), slope.abs());
        // Along the key's strings and across them (the unison's 2.5 mm
        // apart at the agraffe, as drawn).
        let (dx, dy) = (bx - ax, by - ay(m));
        let len = dx.hypot(dy).max(1e-9);
        let (ux, uy) = (dx / len, dy / len);
        let (nx, ny) = (-uy, ux);
        let strings = model.note(m).strings;
        for s in 0..strings {
            let mid = s as f64 - (strings as f64 - 1.0) / 2.0;
            let row = if strings == 1 { (unison as i32 - 1 - m as i32).rem_euclid(2) as f64 } else { s as f64 };
            let (back, across) = (PIN_GAP + PIN_ROW * row, mid * pitch / strings as f64);
            pins.push(TuningPin { at: (ax - ux * back + nx * across, even - uy * back + ny * across), hole: (ax + nx * mid * UNISON_GAP, ay(m) + ny * mid * UNISON_GAP), dir: (ux, uy) });
        }
    }
    pins
}

/// The `i`th string's tuning pin `p`, for a string at height `z` drawn in
/// `c`, as they are made: a steel pin in a bushing, leaning a little away
/// from its string's pull, three turns of the string coiled round it a
/// little below the strings' height, a square head for the tuning hammer
/// under a rounded top, its flats turned any way; and the string's dead
/// length from the coil to its agraffe.
fn tuning_pin(b: &mut Builder, p: &TuningPin, i: usize, z: f64, c: Color32, wire: f32) {
    let place = b.place;
    let hole = place.at(p.hole.0, p.hole.1, z);
    let (ux, uy) = p.dir;
    let lean = PIN_LEAN.tan();
    let base = place.at(p.at.0, p.at.1, PIN_BLOCK_TOP);
    let axis = place.dir(-ux * lean, -uy * lean, 1.0);
    // Turned by the golden angle from one pin to the next: no two alike.
    let turn = i as f64 * 2.399_963;
    let side = place.dir(turn.cos(), turn.sin(), 0.0);
    let (r, rb, w) = (PIN_R as f32, PIN_BUSH_R as f32, PIN_WIRE as f32);
    let (hb, ch) = (PIN_BUSH_H as f32, (PIN_COIL_H / 2.0) as f32);
    let coil = (z - PIN_COIL_DROP - PIN_BLOCK_TOP) as f32;
    let top = coil + PIN_TOP as f32;
    let head = top - PIN_HEAD as f32;
    #[rustfmt::skip]
    let rings = [
        (0.0, rb, false), (hb, rb, false), (hb, r, false),
        (coil - ch, r, false), (coil - ch, r + w, false), (coil + ch, r + w, false), (coil + ch, r, false),
        (head, r, false), (head + 0.0008, 0.8 * r, true), (top - 0.0012, 0.72 * r, true),
        (top - 0.0003, 0.55 * r, false), (top, 0.3 * r, false),
    ];
    b.lathe(base, axis, side, &rings, &[BUSHING, BUSHING, PIN, c, c, c, PIN, PIN, PIN, PIN, PIN]);
    // The coil's turns, up to where the string leaves it towards its
    // agraffe.
    let at = |h: f32| add(base, scale(axis, h));
    let to = sub(hole, at(coil));
    let e1 = norm(sub(to, scale(axis, dot(to, axis))));
    let e2 = cross(axis, e1);
    let (turns, steps) = (3.0, 24);
    let helix = |k: usize| {
        let t = k as f32 / steps as f32;
        let a = -std::f32::consts::TAU * turns * (1.0 - t);
        let (h, rr) = (coil - ch + w / 2.0 + (2.0 * ch - w) * t, r + w + 0.0001);
        add(at(h), add(scale(e1, rr * a.cos()), scale(e2, rr * a.sin())))
    };
    let groove = rgba(mix(c, Color32::BLACK, 0.45), 1.0);
    for k in 0..steps {
        b.wire(helix(k), helix(k + 1), groove, wire);
    }
    b.wire(helix(steps), hole, rgba(c, 1.0), wire);
}

/// The agraffe at `at` (plate) for a unison of `strings` along `u` at
/// height `z`, standing on `foot` (see the [`AGRAFFE_HEAD`] constants):
/// brass `c`, its boss the plate's `plate`; the strings' holes through
/// its head 2.5 mm apart, as they are drawn.
#[allow(clippy::too_many_arguments)]
fn agraffe(b: &mut Builder, at: (f64, f64), u: (f64, f64), z: f64, foot: f64, strings: usize, c: Color32, plate: Color32) {
    let place = b.place;
    let hw = AGRAFFE_HEAD + AGRAFFE_HEAD_PER * (strings as f64 - 1.0);
    let flange = hw + AGRAFFE_FLANGE;
    let head = z - AGRAFFE_BELOW;
    let foot = if head - AGRAFFE_NECK - AGRAFFE_FLANGE_H > foot {
        let top = head - AGRAFFE_NECK - AGRAFFE_FLANGE_H;
        b.plate_prism(at, flange + AGRAFFE_BOSS, 8, 0.0, (foot, top), plate);
        top
    } else {
        foot
    };
    b.plate_prism(at, flange, 6, u.1.atan2(u.0), (foot, foot + AGRAFFE_FLANGE_H), c);
    let base = place.at(at.0, at.1, foot + AGRAFFE_FLANGE_H);
    let (h, top, w) = ((head - foot - AGRAFFE_FLANGE_H) as f32, (z + AGRAFFE_ABOVE - foot - AGRAFFE_FLANGE_H) as f32, hw as f32);
    #[rustfmt::skip]
    let rings = [
        (0.0, 0.6 * w, false), (h, 0.6 * w, false), (h, w, true),
        (top - 0.0012, w, true), (top - 0.0003, 0.8 * w, false), (top, 0.45 * w, false),
    ];
    b.lathe(base, place.dir(0.0, 0.0, 1.0), place.dir(u.0, u.1, 0.0), &rings, &[c]);
    // The holes, on the faces the strings go in and out by.
    let (n, hole) = ((-u.1, u.0), rgba(Color32::from_rgb(40, 32, 20), 1.0));
    for s in 0..strings {
        let off = (s as f64 - (strings as f64 - 1.0) / 2.0) * UNISON_GAP;
        for side in [-1.0, 1.0] {
            let d = side * (hw + 0.0001);
            let (cx, cy) = (at.0 + u.0 * d + n.0 * off, at.1 + u.1 * d + n.1 * off);
            let rim = |k: usize| {
                let (sn, cs) = (std::f64::consts::TAU * k as f64 / 6.0).sin_cos();
                let r = AGRAFFE_HOLE_R;
                place.at(cx + n.0 * r * cs, cy + n.1 * r * cs, z + r * sn)
            };
            let centre = place.at(cx, cy, z);
            for k in 0..6 {
                let t = if side > 0.0 { [centre, rim(k), rim(k + 1)] } else { [centre, rim(k + 1), rim(k)] };
                b.tri(false, t, [hole; 3]);
            }
        }
    }
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
        b.sweep(&[rod(a, 0.006), rod(z, 0.006)], side, LYRE_ROD);
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
    let mut b = Builder::new(place);
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
    b.mat = Material::Spruce;
    for j in 0..gy - 1 {
        for i in 0..gx - 1 {
            let (x0, x1) = (sb.a * i as f64 / (gx - 1) as f64, sb.a * (i + 1) as f64 / (gx - 1) as f64);
            let (y0, y1) = (sb.b * j as f64 / (gy - 1) as f64, sb.b * (j + 1) as f64 / (gy - 1) as f64);
            let piece = clip(&outline, &[(x0, y0), (x1, y0), (x1, y1), (x0, y1)]);
            for k in 1..piece.len().saturating_sub(1) {
                let t = [piece[0], piece[k], piece[k + 1]];
                // The slivers where a cell's edge passes by a rounded
                // outline's point aside.
                if cross2(t[0], t[1], t[2]).abs() < 1e-7 {
                    continue;
                }
                b.tri(false, t.map(|(x, y)| place.at(x, y, 0.0)), t.map(|(x, y)| colour(x, y)));
            }
        }
    }
    // Its underside (its edge is against the rim's wall).
    let under = rgba(wood.gamma_multiply(0.7), 1.0);
    for [i, j, k] in triangulate(&outline) {
        b.tri(false, [outline[i], outline[k], outline[j]].map(|(x, y)| place.at(x, y, -0.012)), [under; 3]);
    }
    b.mat = Material::Paint;
    b.s.board = b.bounds();
    b.label(place.at(sb.a * 0.45, sb.b * 0.7, 0.0), "Soundboard", "Soundboard");

    // The strings' ends: their agraffes on a diagonal in front of the
    // hammers ([`hammer_x`]), their tuning pins in front of those.
    let hx = hammer_x(model, &sb);
    let ends: Vec<_> = (21..=108u8).map(|m| string_ends(model, &sb, hx, m)).collect();
    let pins = tuning_pins(model, &sb, &ends);
    let case_front = pins.iter().map(|p| p.at.0).fold(f64::MAX, f64::min) - PIN_BUSH_R - 0.008;

    // The case: the rim's walls along the board's outline, carried forward
    // over the pin block to just in front of the tuning pins (as the frame
    // is), and the belly rail under the board's front edge.
    let t = if rim.enabled { rim.thickness } else { 0.02 };
    let (z0, z1) = if rim.enabled { (rim.top - rim.height, rim.top) } else { (-0.24, 0.16) };
    let case = pick(Part::Rim, CASE);
    // A grand's case goes on above the rim up to the lid's hinge: the box
    // the shut lid closes ([`SoundboardParams::lid_shut`]), the lid resting
    // on it. Only the rim below is in the structure's model.
    let lid_top = if vertical { z1 } else { sb.lid_hinge_height.max(z1) };
    let n = outline.len();
    let front = (0..n).find(|&k| outline[k].0.abs() < 1e-6 && outline[(k + 1) % n].0.abs() < 1e-6);
    let (mut across_lo, mut across_hi) = (0.0, sb.b);
    if let Some(k) = front {
        let (p, q) = (outline[k], outline[(k + 1) % n]);
        let forward = |p: (f64, f64)| if p.0.abs() < 1e-6 { (case_front, p.1) } else { p };
        let sides: Vec<_> = (1..=n).map(|i| forward(outline[(k + i) % n])).collect();
        (across_lo, across_hi) = (p.1.min(q.1), p.1.max(q.1));
        if o.case {
            b.wall(&sides, false, 0.0, t, z0, &|_| z1, case);
            if lid_top > z1 {
                b.wall(&sides, false, 0.0, t, z1, &|_| lid_top, CASE);
            }
            // A grand's under the board, behind the hammers resting under
            // its edge and the keys' tails.
            let at = if vertical { -BELLY_RAIL } else { BELLY_AT };
            b.wall(&[p, q], false, at, at + BELLY_RAIL, z0, &|_| -0.012, case);
        }
    } else if o.case {
        b.wall(&outline, true, 0.0, t, z0, &|_| z1, case);
        if lid_top > z1 {
            b.wall(&outline, true, 0.0, t, z1, &|_| lid_top, CASE);
        }
    }

    // The pin block under the plate, from the case's front to just behind
    // the agraffes, short of the hammers.
    let block_back = |ax: f64| (ax + 0.008).min(hx - BLOCK_CLEAR);
    let mut back: Vec<(f64, f64)> = ends.iter().map(|&((ax, ay), _, _)| (block_back(ax), ay.clamp(across_lo, across_hi))).collect();
    back.sort_by(|p, q| q.1.total_cmp(&p.1));
    let mut block = vec![(case_front, across_lo), (case_front, across_hi), (back[0].0, across_hi)];
    block.extend(&back);
    block.push((back[back.len() - 1].0, across_lo));
    // A key past the board's edge (A0, the keyboard being wider than the
    // board) lands on a corner once clamped: one point, not two.
    block.dedup_by(|p, q| (p.0 - q.0).hypot(p.1 - q.1) < 1e-9);
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
    // The felt strips on it under the strings' dead lengths, along the
    // agraffes just in front of them: one per bridge's strings.
    for bass in [true, false] {
        let mut strip: Vec<(f64, f64)> = (21..=108u8)
            .zip(&ends)
            .filter(|&(m, _)| (m < sb.main_bridge_from) == bass)
            .map(|(_, &((ax, ay), (bx, by), _))| {
                let l = (bx - ax).hypot(by - ay).max(1e-9);
                (ax - (bx - ax) / l * UNDER_FELT_AT, ay - (by - ay) / l * UNDER_FELT_AT)
            })
            .collect();
        // No bass strip when the main bridge carries every string.
        if strip.len() < 2 {
            continue;
        }
        strip.sort_by(|p, q| q.1.total_cmp(&p.1));
        b.wall(&strip, false, -UNDER_FELT_W / 2.0, UNDER_FELT_W / 2.0, PIN_BLOCK_TOP, &|_| PIN_BLOCK_TOP + UNDER_FELT_T, UNDER_FELT);
    }
    let rim_label = outline.iter().cloned().fold((0.0, 0.0), |m, p| if p.0 > m.0 { p } else { m });
    if o.case {
        b.label(place.at(rim_label.0 + t, rim_label.1, z1), if rim.enabled { "Rim" } else { "Case (not modelled)" }, "Frame & rim");
    }

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
    // Strings counted from the bass, as their tuning pins are.
    let mut count = 0usize;
    // Where the felt braid crosses each unison's rear length.
    struct Crossing {
        bass: bool,
        /// Plate place, the strings' height there and their direction.
        at: (f64, f64),
        z: f64,
        dir: (f64, f64),
    }
    let mut braid: Vec<Crossing> = Vec::new();
    for m in 21..=108u8 {
        let n = model.note(m);
        let ((ax, ay), (bx, by), z) = ends[(m - 21) as usize];
        let level = key_level(m);
        let wound = m < sb.main_bridge_from || n.strings == 1;
        let base = if wound { COPPER } else { STEEL };
        b.mat = if wound { Material::Copper } else { Material::Steel };
        let (wire, plain) = string_d(model, m);
        let base = if lit(Part::Length) || lit(Part::Unison) { sketches::HIGHLIGHT } else { base };
        let c = if m == o.key { o.ink } else { mix(base, sketches::HIGHLIGHT, level * 0.8) };
        // Across the unison, 2.5 mm apart.
        let (dx, dy) = (bx - ax, by - ay);
        let len = dx.hypot(dy).max(1e-9);
        let (nx, ny) = (-dy / len, dx / len);
        let segments = if level > 0.02 { 16 } else { 1 };
        // Beyond the bridge, the strings' rear lengths to their hitch pins,
        // a little lower on the plate.
        let (ux, uy) = (dx / len, dy / len);
        let rear = hitch_length(&outline, (bx, by), (ux, uy), len);
        let tail = if m == o.key { o.ink } else { pick(Part::Frame, HITCH) };
        // Over the bridge, between its two pins: the speaking length ends
        // against the front pin, to one side, and the string crosses to
        // the other side of the back pin (side bearing).
        let (pa, pd) = (BRIDGE_PIN_ALONG, BRIDGE_PIN_SIDE);
        for s in 0..n.strings {
            let off = (s as f64 - (n.strings as f64 - 1.0) / 2.0) * UNISON_GAP;
            let point = |t: f64| {
                let amp = live.map_or(0.0, |l| {
                    let phase = l.time * std::f32::consts::TAU * (5.0 + 0.07 * m as f32) + s as f32;
                    0.012 * level * phase.sin() * (std::f32::consts::PI * t as f32).sin()
                });
                let z = z + amp as f64;
                let (cx, cy) = (-ux * pa + nx * pd, -uy * pa + ny * pd);
                place.at(ax + (dx + cx) * t + nx * off, ay + (dy + cy) * t + ny * off, z)
            };
            for k in 0..segments {
                let (t0, t1) = (k as f64 / segments as f64, (k + 1) as f64 / segments as f64);
                b.wire(point(t0), point(t1), rgba(c, 1.0), wire);
            }
            // Its tuning pin, and the string's dead length from the pin's
            // coil to the agraffe.
            tuning_pin(&mut b, &pins[count], count, z, c, plain);
            // The bridge pins, and the string between them.
            let (gx, gy) = (bx + ux * pa + nx * (off - pd), by + uy * pa + ny * (off - pd));
            let foot = z - (STRING_Z - BRIDGE_H);
            let front = (bx - ux * pa + nx * (off + pd + BRIDGE_PIN_R), by - uy * pa + ny * (off + pd + BRIDGE_PIN_R));
            b.plate_prism(front, BRIDGE_PIN_R, 6, 0.0, (foot, z + 0.004), BRIDGE_PIN);
            b.plate_prism((gx - nx * BRIDGE_PIN_R, gy - ny * BRIDGE_PIN_R), BRIDGE_PIN_R, 6, 0.0, (foot, z + 0.004), BRIDGE_PIN);
            b.wire(point(1.0), place.at(gx, gy, z), rgba(c, 1.0), wire);
            // Its rear length, plain wire, round its hitch pin.
            let (hx, hy) = (bx + ux * rear + nx * off, by + uy * rear + ny * off);
            b.wire(place.at(gx, gy, z), place.at(hx - ux * HITCH_R, hy - uy * HITCH_R, z - HITCH_DROP), rgba(tail, 1.0), plain);
            // Standing on the plate's rail.
            b.plate_prism((hx, hy), HITCH_R, 6, 0.0, (plate::RAIL_TOP - 0.001, z - HITCH_DROP + 0.004), HITCH);
            count += 1;
        }
        b.mat = Material::Paint;
        // The felt woven through the rear lengths of the strings that have
        // dampers (the treble's ring free, a duplex).
        if m <= d.highest_note {
            braid.push(Crossing { bass: m < sb.main_bridge_from, at: (bx + ux * rear * 0.45, by + uy * rear * 0.45), z: z - HITCH_DROP * 0.45, dir: (ux, uy) });
        }
        // The agraffe the unison goes through, standing on the pin block
        // (the highest treble's overhang it).
        let r = AGRAFFE_HEAD + AGRAFFE_HEAD_PER * (n.strings as f64 - 1.0) + AGRAFFE_FLANGE;
        let foot = if ax + r <= block_back(ax) { PIN_BLOCK_TOP } else { z - 0.008 };
        agraffe(&mut b, (ax, ay), (ux, uy), z, foot, n.strings, pick(Part::Agraffes, BRASS), pick(Part::Frame, IRON));
        // An upright's hammer in front of the strings at the strike
        // point, all on one line, and its damper; a grand's are its
        // action's ([`action`]).
        if !vertical {
            continue;
        }
        let k = obs.map(|o| o.keys[m as usize]);
        let down = k.is_some_and(|k| k.down);
        let hammer_on = k.is_some_and(|k| k.hammer);
        let (sx, sy) = (ax + dx * n.strike_pos, ay + dy * n.strike_pos);
        // At rest, their felts just above the belly rail.
        let gap = if hammer_on { 0.0 } else if down { 0.012 } else { 0.025 };
        let felt = if hammer_on { sketches::HIGHLIGHT } else { pick(Part::Strike, pick(Part::Felt, sketches::FELT)) };
        let w = 0.0045;
        let (h0, h1) = (z + 0.002 + gap, z + 0.032 + gap);
        b.plate_box((sx - 0.012, sy - w, h0), (sx + 0.012, sy + w, h1), felt);
        // The damper on the strings, in front of them, lifted by the key
        // or the pedal.
        if m <= d.highest_note {
            let (px, py) = action::damper_at(model, ends[(m - 21) as usize], m);
            let lift = 0.015 * (1.0 - k.map_or(1.0, |k| k.damper)) as f64;
            let at = |a: f64, s: f64, h: f64| place.at(px + ux * a - uy * s, py + uy * a + ux * s, z + lift + h);
            let (felt, wood) = (pick(Part::DamperPoint, DAMPER_FELT), pick(Part::Damper, DAMPER_WOOD));
            action::damper_head(&mut b, &at, model, m, felt, wood);
        }
    }
    // The braid: a band of felt across each bridge's rear lengths, over
    // one unison and under the next.
    for bass in [true, false] {
        let set: Vec<_> = braid.iter().filter(|e| e.bass == bass).collect();
        if set.len() < 2 {
            continue;
        }
        let steps = 4;
        let mut rings = Vec::new();
        for i in 0..set.len() - 1 {
            let (p, q) = (set[i], set[i + 1]);
            for k in 0..steps {
                let t = k as f64 / steps as f64;
                let weave = BRAID_WEAVE * (std::f64::consts::PI * (i as f64 + t)).cos();
                let (x, y) = (p.at.0 + (q.at.0 - p.at.0) * t, p.at.1 + (q.at.1 - p.at.1) * t);
                rings.push((place.at(x, y, p.z + (q.z - p.z) * t + weave), BRAID_W / 2.0, BRAID_T / 2.0, BRAID_T / 2.0 - 0.0001));
            }
        }
        let last = set[set.len() - 1];
        let weave = BRAID_WEAVE * (std::f64::consts::PI * (set.len() - 1) as f64).cos();
        rings.push((place.at(last.at.0, last.at.1, last.z + weave), BRAID_W / 2.0, BRAID_T / 2.0, BRAID_T / 2.0 - 0.0001));
        let u = set.iter().fold((0.0, 0.0), |a, e| (a.0 + e.dir.0, a.1 + e.dir.1));
        b.sweep(&rings, place.dir(u.0, u.1, 0.0), FELT);
    }
    let mid = string_ends(model, &sb, hx, 60);
    let n60 = model.note(60);
    let strike = (mid.0.0 + (mid.1.0 - mid.0.0) * n60.strike_pos, mid.0.1 + (mid.1.1 - mid.0.1) * n60.strike_pos);
    b.label(place.at(strike.0, strike.1 - 0.12, if vertical { mid.2 + 0.04 } else { mid.2 - 0.04 }), "Hammers", "Hammer");
    let damp = action::damper_at(model, mid, 60);
    b.label(place.at(damp.0, damp.1 + 0.12, mid.2 + 0.03), "Dampers", "Dampers");
    b.label(place.at((mid.0.0 + mid.1.0) / 2.0, mid.0.1 - 0.25, mid.2), "Strings", "Strings");
    let a0 = string_ends(model, &sb, hx, 21);
    b.label(place.at(a0.0.0, a0.0.1, a0.2 + 0.01), "Agraffes", "Frame & rim");
    let c8 = string_ends(model, &sb, hx, 108);
    b.label(place.at(c8.0.0 - PIN_GAP, c8.0.1, c8.2 + 0.015), "Tuning pins", "Frame & rim");

    // The iron frame over the strings, as a plate is cast ([`plate`]).
    if f.enabled && o.frame {
        let lit_frame = lit(Part::Frame) || lit(Part::FrameHeight);
        b.mat = Material::Iron;
        plate::plate(&mut b, model, &sb, &ends, if lit_frame { sketches::HIGHLIGHT } else { IRON });
        b.mat = Material::Paint;
        // Labelled at its tail, the farthest point from the keyboard.
        let outline = frame_outline(model, &sb);
        let tail = outline.iter().copied().max_by(|p, q| p.0.total_cmp(&q.0)).unwrap_or((0.0, 0.0));
        b.label(place.at(tail.0, tail.1, f.height), "Frame", "Frame & rim");
    }

    // The lid, hinged along the bass side, of the case's shape as the
    // radiation has it ([`SoundboardParams::lid_outline`]): over the rim,
    // its bentside the board's, rounded as the board is.
    let lid = Outline { points: outline.clone() }.grown(LID_OVERHANG).points;
    let lid_from = b.s.glass.len();
    if !vertical && sb.lid_shut >= 1.0 && o.lid {
        // Shut, with its front flap carried forward to the case's front.
        let h = sb.lid_hinge_height;
        let mut top: Vec<(f64, f64)> = lid.iter().map(|&p| if p.0 <= -LID_OVERHANG + 1e-9 { (case_front, p.1) } else { p }).collect();
        if signed_area(&top) < 0.0 {
            top.reverse();
        }
        let c = rgba(CASE, 0.25);
        for [i, j, k] in triangulate(&top) {
            let p = [top[i], top[j], top[k]].map(|(x, y)| place.at(x, y, h));
            b.tri(true, p, [c; 3]);
            b.tri(true, [p[2], p[1], p[0]], [c; 3]);
        }
        for i in 0..top.len() {
            let (p, q) = (top[i], top[(i + 1) % top.len()]);
            b.line(place.at(p.0, p.1, h), place.at(q.0, q.1, h), rgba(CASE, 0.9));
        }
        b.label(place.at(sb.a * 0.6, sb.b * 0.5, h), "Lid, shut", "Microphones");
    } else if !vertical && sb.lid_angle > 0.0 && o.lid {
        // Open, its front flap folded back on it, turned about the hinge
        // y = b: what lies past it is the rim's top.
        let (th, h) = (sb.lid_angle.to_radians(), sb.lid_hinge_height);
        let far = 10.0 * (sb.a + sb.b);
        let mut top = clip(&lid, &[(-far, -far), (far, -far), (far, sb.b), (-far, sb.b)]);
        if signed_area(&top) < 0.0 {
            top.reverse();
        }
        let up = |(x, y): (f64, f64)| place.at(x, sb.b - (sb.b - y) * th.cos(), h + (sb.b - y) * th.sin());
        let c = rgba(CASE, 0.35);
        for [i, j, k] in triangulate(&top) {
            let p = [top[i], top[j], top[k]].map(up);
            b.tri(true, p, [c; 3]);
            b.tri(true, [p[2], p[1], p[0]], [c; 3]);
        }
        for i in 0..top.len() {
            let (p, q) = (top[i], top[(i + 1) % top.len()]);
            if p.1.min(q.1) < sb.b - 1e-9 {
                b.line(up(p), up(q), rgba(CASE, 0.9));
            }
        }
        // Labelled at its edge farthest from the hinge.
        let edge = top.iter().copied().min_by(|p, q| p.1.total_cmp(&q.1)).unwrap_or((sb.a * 0.6, 0.0));
        b.label(up(edge), format!("Lid, {:.0}°", sb.lid_angle), "Microphones");
    }
    b.s.lid = b.s.glass.split_off(lid_from);

    // The keyboard, the keys down pressed in, its back against the case's
    // front: a grand's on its keybed, at the usual height above the floor.
    let keys_back = case_front - FRONT_T;
    let (klo, khi) = keyboard_span(&sb);
    let (y_back, kz) = if vertical { (UPRIGHT_KEYS_Y, UPRIGHT_KEYS_Z) } else { (keys_back, z0 + KEYBED_T + KEY_FRAME) };
    let key_top = kz + 0.022;
    let floor = if vertical { 0.0 } else { key_top - KEY_HEIGHT };
    let key_front = y_back - KEY_DEPTH;
    let ctx = action::Ctx {
        model,
        sb: &sb,
        ends: &ends,
        hx,
        obs,
        key: o.key,
        lit: &lit,
        level: &key_level,
        span: (klo, khi),
        key_front,
        key_back: y_back,
        kz,
        vertical,
        inside: !o.case,
    };
    action::keys(&mut b, &ctx);
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
        // Its case, as most grands': the keybed under the case's front
        // and the action, its sides carried forward as arms that come down in a curve from
        // the case's top to a little above the keys, rounded at their
        // front; the cheek blocks between them and the keys' ends, the key
        // slip along the keys' front, and behind them the fallboard (open)
        // over a strip of felt. Across (plate y): the case's sides, wider
        // if the keys are.
        let (lo, hi) = (across_lo.min(klo - KEY_CHEEK), across_hi.max(khi + KEY_CHEEK));
        let (x_lo, x_hi) = (-(hi + t) as f32, -(lo - t) as f32);
        let (yf, yb) = ((key_front - KEY_SLIP - 0.002) as f32, (y_back + FRONT_T) as f32);
        let (z0f, top, bed) = (z0 as f32, key_top as f32, (z0 + KEYBED_T) as f32);
        b.cuboid([x_lo, yf + 0.004, z0f], [x_hi - x_lo, 0.0, 0.0], [0.0, BELLY_AT as f32 - yf - 0.004, 0.0], [0.0, 0.0, KEYBED_T as f32], case);
        // The front legs under the arms, flush with their outsides.
        let y = yf + 0.1;
        front_legs = [[x_lo + 0.074, y], [x_hi - 0.074, y]];
        let arm = top + 0.035;
        let mut profile = vec![(yf, z0f), (yb, z0f), (yb, lid_top as f32)];
        for i in 1..=10 {
            let s = i as f32 / 10.0;
            profile.push((yb - 0.09 * s, arm + (lid_top as f32 - arm) * 0.5 * (1.0 + (std::f32::consts::PI * s).cos())));
        }
        let r = 0.022;
        for i in 0..=6 {
            let a = std::f32::consts::FRAC_PI_2 * i as f32 / 6.0;
            profile.push((yf + r - r * a.sin(), arm - r + r * a.cos()));
        }
        for (x0, x1) in [(x_lo, -hi as f32), (-lo as f32, x_hi)].into_iter().filter(|_| o.case) {
            b.slab(&profile, x0, x1, case);
        }
        for (x0, x1) in [(-hi as f32, -(khi + 0.002) as f32), (-(klo - 0.002) as f32, -lo as f32)].into_iter().filter(|_| o.case) {
            let block = |y: f32| ([(x0 + x1) / 2.0, y, (bed + top + 0.012) / 2.0], (x1 - x0) / 2.0, (top + 0.012 - bed) / 2.0, 0.008);
            b.sweep(&[block(yf + KEY_SLIP as f32), block(y_back as f32)], [1.0, 0.0, 0.0], case);
        }
        let along = |x0: f32, x1: f32, y: f32, z: (f32, f32), d: f32, r: f32| {
            let ring = |x: f32| ([x, y, (z.0 + z.1) / 2.0], d / 2.0, (z.1 - z.0) / 2.0, r);
            [ring(x0), ring(x1)]
        };
        // The fallboard over the keys, which run on under it, the name
        // on it; shut, a board over the keys from it to their front, the
        // name on that.
        if o.case {
            b.sweep(&along(-hi as f32, -lo as f32, yf + KEY_SLIP as f32 / 2.0, (z0f, kz as f32 - 0.002), KEY_SLIP as f32, 0.005), [0.0, 1.0, 0.0], case);
            b.sweep(&along(-hi as f32, -lo as f32, (y_back + FRONT_T / 2.0) as f32, (top + 0.014, PIN_BLOCK_TOP as f32), FRONT_T as f32, 0.009), [0.0, 1.0, 0.0], case);
            b.cuboid([-(khi + 0.002) as f32, y_back as f32 - 0.006, top - 0.004], [(khi - klo + 0.004) as f32, 0.0, 0.0], [0.0, 0.006, 0.0], [0.0, 0.0, 0.012], FELT);
            // Above it, the front closed up to the case's top, under the
            // lid, and a cover from there back over the pin block to the
            // hammers' line: as solid as asked.
            let from = b.s.tris.len();
            let shut = lid_top as f32 - 0.002;
            b.sweep(&along(-hi as f32, -lo as f32, (y_back + FRONT_T / 2.0) as f32, (PIN_BLOCK_TOP as f32, shut), FRONT_T as f32, 0.009), [0.0, 1.0, 0.0], case);
            let (c0, c1) = (case_front as f32, hx as f32);
            b.cuboid([-across_hi as f32, c0, shut - COVER_T], [(across_hi - across_lo) as f32, 0.0, 0.0], [0.0, c1 - c0, 0.0], [0.0, 0.0, COVER_T], case);
            b.see_through(from, o.front);
            let (cx, kt) = (cx as f64, key_top);
            if o.fallboard {
                // Its top from high on the name board down to just in front
                // of the keys, its front edge rounded, a lip down in front
                // of them onto the key slip; under it, clear of the black
                // keys.
                let (back, front) = ((y_back, kt + FALL_BACK), (key_front - FALL_LIP - 0.003, kt + FALL_FRONT));
                let mut profile = vec![back];
                profile.extend(fillet(back, front, (front.0, kz), FALL_ROUND));
                profile.extend([(front.0, kz), (front.0 + FALL_LIP, kz), (front.0 + FALL_LIP, kt + 0.004), (back.0, back.1 - FALL_T)]);
                let profile: Vec<(f32, f32)> = profile.into_iter().map(|(y, z)| (y as f32, z as f32)).collect();
                b.slab(&profile, -hi as f32, -lo as f32, case);
                // The name on its top, a little behind the middle.
                let (dy, dz) = (back.0 - front.0, back.1 - front.1);
                let l = dy.hypot(dz);
                let (dy, dz) = (dy / l, dz / l);
                let (py, pz) = (front.0 + dy * 0.55 * l, front.1 + dz * 0.55 * l);
                name::name(&mut b, &|u, v, h| [(cx + u) as f32, (py + dy * v - dz * h) as f32, (pz + dz * v + dy * h) as f32]);
            } else {
                name::name(&mut b, &|u, v, h| [(cx + u) as f32, (y_back - h) as f32, (kt + NAME_UP + v) as f32]);
            }
        }
    }
    b.label([-((klo + khi) / 2.0) as f32, key_front as f32, (kz + 0.03) as f32], "Keyboard", "Player");

    // Pedals: soft, sostenuto, sustain from left to right.
    let values = obs.map_or([0.0; 3], |o| [o.soft, if o.sostenuto { 1.0 } else { 0.0 }, o.sustain]);
    let parts = [Part::UnaCorda, Part::UnaCorda, Part::SustainPedal];
    let pedals: [(f32, Color32); 3] = std::array::from_fn(|i| {
        let v = values[i];
        (v, if v > 0.05 || (i != 1 && lit(parts[i])) { sketches::HIGHLIGHT } else { BRASS })
    });
    b.mat = Material::Brass;
    if vertical {
        for (i, &(v, c)) in pedals.iter().enumerate() {
            pedal(&mut b, cx + (i as f32 - 1.0) * PEDAL_PITCH, key_front as f32, floor as f32 + 0.06, v, c);
        }
    } else {
        // The lyre under the keybed, in front of the fallboard, braced to
        // the belly rail; the third leg under the case's tail.
        lyre(&mut b, cx, y_back as f32 - 0.05, z0 as f32, floor as f32, ((BELLY_AT + BELLY_RAIL / 2.0) as f32, z0 as f32), pedals);
        let tail = place.at(sb.a - 0.12, sb.b * 0.5, z0);
        for [x, y] in front_legs.into_iter().chain([[tail[0], tail[1]]]) {
            leg(&mut b, [x, y, z0 as f32], floor as f32);
        }
    }
    b.mat = Material::Paint;
    b.label([cx, key_front as f32 - 0.1, floor as f32 + 0.08], "Pedals", "Player");
    b.s.body = b.bounds();
    b.s.floor = floor as f32;
    b.s.live = live.is_some();

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
            b.wire(p, foot, rgba(c, 0.7), 0.012);
            b.label(p, if i == 0 { "Mic L" } else { "Mic R" }, "Microphones");
        }
    }

    b.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options() -> Options<'static> {
        Options { key: 60, lit: &[], lid: true, frame: true, mics: true, case: true, fallboard: false, front: 1.0, ink: Color32::WHITE }
    }

    #[test]
    fn the_default_grand_is_whole_and_to_scale() {
        let m = PianoModel::default();
        let s = build(&m, None, &options());
        assert_eq!(s.tris.len() % 3, 0);
        assert_eq!(s.lines.len() % 2, 0);
        assert!(s.tris.iter().chain(&s.lines).all(|v| v.pos.iter().chain(&v.normal).all(|x| x.is_finite())));
        // No flattened box: every face has an area (but the name's
        // letters, whose curves leave thin slivers).
        for t in s.tris.as_chunks::<3>().0.iter().filter(|t| t[0].mat != Material::Gilt as u8 as f32) {
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
            assert!((ay + s * (by - ay) - key_across(&m, &sb, k)).abs() < 1e-12, "key {k} struck off its key");
            assert!(ax < hx, "key {k}'s agraffe at {ax}");
        }
        // The hammers in front of the board; the bass's agraffe in front,
        // the treble's well behind it, just in front of the hammers.
        let (a0, c8) = (ends[0].1.0.0, ends[87].1.0.0);
        assert!(hx <= -HAMMER_CLEAR, "{hx}");
        assert!(ends[0].1.0.1 > key_across(&m, &sb, 21) + 0.03, "A0's agraffe not to the bass side");
        assert!(c8 - a0 > 0.1 && hx - c8 < 0.01, "A0 at {a0}, C8 at {c8}, hammers at {hx}");
        // A tuning pin per string, in front of its agraffe.
        let strings: usize = (21..=108u8).map(|k| m.note(k).strings).sum();
        let pins = build(&m, None, &options()).tris.iter().filter(|v| v.color == rgba(PIN, 1.0)).count();
        assert_eq!(pins, strings * (6 * 12 * 2 + 12) * 3);
        let tuning = tuning_pins(&m, &sb, &ends.iter().map(|e| e.1).collect::<Vec<_>>());
        assert!(tuning.iter().all(|p| p.at.0 < p.hole.0 - PIN_GAP + 0.001), "a pin behind its agraffe");
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
    fn a_rounded_board_is_covered_by_its_top() {
        // The grand's, and one edited: its spine's back end 5 cm forward,
        // which would bend the tail out past the board's length.
        let mut edited = SoundboardParams::GRAND_2M;
        edited.outline[8].0 = 1.0 - 0.05 / edited.a;
        for sb in [SoundboardParams::GRAND_2M, edited] {
            let m = PianoModel { soundboard: Some(sb), ..PianoModel::default() };
            let sb = m.soundboard.unwrap().resolved();
            let outline = board_outline(&sb);
            assert!(outline.iter().all(|&(x, y)| (0.0..=sb.a).contains(&x) && (0.0..=sb.b).contains(&y)));
            let s = build(&m, None, &options());
            let top: f64 = s
                .tris
                .as_chunks::<3>()
                .0
                .iter()
                .filter(|t| t.iter().all(|v| v.pos[2] == 0.0 && v.mat == Material::Spruce as u8 as f32))
                .map(|t| 0.5 * (dot(cross(sub(t[1].pos, t[0].pos), sub(t[2].pos, t[0].pos)), cross(sub(t[1].pos, t[0].pos), sub(t[2].pos, t[0].pos))) as f64).sqrt())
                .sum();
            let area = signed_area(&outline);
            assert!((top / area - 1.0).abs() < 1e-4, "{top} vs {area}");
        }
    }

    #[test]
    fn a_rounded_outline_is_a_curve_through_the_models() {
        let sb = SoundboardParams { outline: SoundboardParams::GRAND_OUTLINE, ..SoundboardParams::GRAND_2M };
        let mut model = Outline::new(&sb).unwrap().points;
        if signed_area(&model) < 0.0 {
            model.reverse();
        }
        let round = rounded(&model);
        assert!(round.len() > 3 * model.len(), "{}", round.len());
        // Every vertex of the model's on it, in order.
        let mut at = 0;
        for p in &model {
            at += round[at..].iter().position(|q| q == p).expect("a vertex left out");
        }
        // Close to the polygon: within 2.5 cm of it, its area within 1 %.
        let near = Outline { points: model.clone() };
        assert!(round.iter().all(|&(x, y)| near.edge_distance(x, y) < 0.025));
        assert!((signed_area(&round) / signed_area(&model) - 1.0).abs() < 0.01);
        // Smooth but at the three corners (front, spine): no bend over a
        // few degrees elsewhere.
        let n = round.len();
        let bends = (0..n).filter(|&i| {
            let (a, b, c) = (round[(i + n - 1) % n], round[i], round[(i + 1) % n]);
            let (u, v) = ((b.0 - a.0, b.1 - a.1), (c.0 - b.0, c.1 - b.1));
            (u.0 * v.0 + u.1 * v.1) / (u.0.hypot(u.1) * v.0.hypot(v.1)) < 8f64.to_radians().cos()
        });
        assert_eq!(bends.count(), 3);
        // A rectangle stays one.
        let rect = [(0.0, 0.0), (1.7, 0.0), (1.7, 1.3), (0.0, 1.3)];
        assert_eq!(rounded(&rect), rect);
    }

    #[test]
    fn the_rim_is_one_wall_without_overlaps() {
        let mut m = PianoModel::default();
        m.soundboard.as_mut().unwrap().outline = SoundboardParams::GRAND_OUTLINE;
        let sb = m.soundboard.unwrap();
        let outline = board_outline(&sb);
        let mut b = Builder::new(Place { vertical: false, top: 0.0 });
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
    fn any_bass_bridge_builds_grand_or_upright() {
        // Down to none: the main bridge carrying every string from A0.
        for vertical in [false, true] {
            for from in [21, 22, 28, 45, 60, 107] {
                for case in [true, false] {
                    let mut m = PianoModel::default();
                    let sb = m.soundboard.as_mut().unwrap();
                    (sb.vertical, sb.main_bridge_from) = (vertical, from);
                    let s = build(&m, None, &Options { case, ..options() });
                    assert!(!s.tris.is_empty(), "vertical {vertical}, main bridge from {from}");
                    assert!(s.tris.iter().chain(&s.lines).all(|v| v.pos.iter().all(|x| x.is_finite())), "vertical {vertical}, main bridge from {from}, case {case}");
                }
            }
        }
    }

    #[test]
    fn the_keys_and_the_strings_fit_inside_the_case() {
        let m = PianoModel::default();
        let sb = m.soundboard.unwrap().resolved();
        // The keyboard and its cheek blocks between the rim's walls, its
        // arms in line with them.
        let (lo, hi) = keyboard_span(&sb);
        assert!(lo >= KEY_CHEEK - 1e-9 && hi <= sb.b - KEY_CHEEK + 1e-9, "keys from {lo} to {hi}, board {}", sb.b);
        // Every string's front end and tuning pin inside the bass side.
        let hx = hammer_x(&m, &sb);
        let ends: Vec<_> = (21..=108u8).map(|k| string_ends(&m, &sb, hx, k)).collect();
        assert!(ends.iter().all(|e| e.0.1 <= sb.b - AGRAFFE_EDGE + 1e-9), "an agraffe in the wall");
        let pins = tuning_pins(&m, &sb, &ends);
        let far = pins.iter().map(|p| p.at.1).fold(f64::MIN, f64::max);
        assert!(far + PIN_BUSH_R < sb.b, "a tuning pin at {far}");
        // The bass's hammers closer together, but no closer than half the
        // others' pitch, in order.
        let main = key_across(&m, &sb, 107) - key_across(&m, &sb, 108);
        for k in 21..sb.main_bridge_from {
            let gap = key_across(&m, &sb, k) - key_across(&m, &sb, k + 1);
            assert!(gap >= 0.5 * main - 1e-12 && gap <= main + 1e-12, "key {k}: {gap} vs {main}");
        }
    }

    #[test]
    fn the_strings_pass_between_the_tuning_pins() {
        let m = PianoModel::default();
        let sb = m.soundboard.unwrap().resolved();
        let hx = hammer_x(&m, &sb);
        let ends: Vec<_> = (21..=108u8).map(|k| string_ends(&m, &sb, hx, k)).collect();
        let pins = tuning_pins(&m, &sb, &ends);
        assert_eq!(pins.len(), (21..=108u8).map(|k| m.note(k).strings).sum::<usize>());
        let coil = PIN_R + PIN_WIRE;
        let (mut wire_gap, mut pin_gap) = (f64::MAX, f64::MAX);
        for (i, p) in pins.iter().enumerate() {
            // From where it leaves its coil to its agraffe.
            let (dx, dy) = (p.hole.0 - p.at.0, p.hole.1 - p.at.1);
            let l = dx.hypot(dy);
            let a = (p.at.0 + dx / l * coil, p.at.1 + dy / l * coil);
            for (j, q) in pins.iter().enumerate() {
                if i == j {
                    continue;
                }
                pin_gap = pin_gap.min((q.at.0 - p.at.0).hypot(q.at.1 - p.at.1) - 2.0 * coil);
                let (ex, ey) = (p.hole.0 - a.0, p.hole.1 - a.1);
                let t = (((q.at.0 - a.0) * ex + (q.at.1 - a.1) * ey) / (ex * ex + ey * ey)).clamp(0.0, 1.0);
                wire_gap = wire_gap.min((a.0 + ex * t - q.at.0).hypot(a.1 + ey * t - q.at.1) - PIN_R);
            }
        }
        // 0.27 mm in A2's unison since the main strings fan (their agraffes,
        // and so their pins, closer together in the tenor).
        assert!(wire_gap > 0.00025, "a string touches a pin: {wire_gap}");
        assert!(pin_gap > 0.001, "two coils touch: {pin_gap}");
    }

    #[test]
    fn the_parts_are_made_of_something() {
        // Only the microphones and the agraffes' holes are plain paint;
        // the strings are wires as thick as they are.
        let mut model = PianoModel::default();
        for (case, vertical) in [(true, false), (false, false), (true, true)] {
            model.soundboard.as_mut().unwrap().vertical = vertical;
            let s = build(&model, None, &Options { key: 0, case, ..options() });
            let paint = Material::Paint as u8 as f32;
            let mut colours: Vec<_> = s.tris.iter().filter(|v| v.mat == paint).map(|v| v.color.map(|c| (c * 255.0).round() as u8)).collect();
            colours.sort();
            colours.dedup();
            let allowed = [rgba(MIC[0], 1.0), rgba(MIC[1], 1.0), rgba(Color32::from_rgb(40, 32, 20), 1.0)].map(|c| c.map(|c| (c * 255.0).round() as u8));
            assert!(colours.iter().all(|c| allowed.contains(c)), "unmade colours {colours:?} (case {case}, upright {vertical})");
            let strings = s.lines.iter().filter(|v| [Material::Steel, Material::Copper].map(|m| m as u8 as f32).contains(&v.mat));
            assert!(strings.clone().count() > 400 && strings.clone().all(|v| v.width > 0.0005 && v.width < 0.007));
        }
    }

    #[test]
    fn the_plate_stands_over_the_dampers_lifted() {
        // Every damper up (the pedal down): their heads under the plate's
        // bar and struts, and behind the bar.
        let m = PianoModel::default();
        let mut obs = Observation::default();
        for k in obs.keys.iter_mut() {
            k.damper = 0.0;
        }
        let levels = [0.0f32; 128];
        let live = Live { obs: &obs, keys: &levels, board: None, time: 0.0 };
        let s = build(&m, Some(&live), &options());
        let heads = [rgba(DAMPER_FELT, 1.0), rgba(DAMPER_WOOD, 1.0)];
        let (top, front) = s.tris.iter().filter(|v| heads.contains(&v.color)).fold((f32::MIN, f32::MAX), |(z, y), v| (z.max(v.pos[2]), y.min(v.pos[1])));
        assert!(top > 0.05 && (top as f64) < plate::CLEAR - 0.002, "heads up to {top}");
        // World y is plate x on a grand.
        assert!((front as f64) > plate::BAR_BACK, "heads from {front}");
    }

    #[test]
    fn faces_point_outwards() {
        let mut b = Builder::new(Place { vertical: false, top: 0.0 });
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

