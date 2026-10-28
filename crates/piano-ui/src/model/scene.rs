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
use piano_core::soundboard::SoundboardParams;
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
/// The keyboard: depth of the white keys, keybed under the keys (grand,
/// z), the floor.
const KEY_DEPTH: f64 = 0.15;
const KEY_Z: f64 = -0.02;
const GRAND_FLOOR: f64 = -0.72;
/// An upright's board bottom, keyboard height and distance in front.
const UPRIGHT_BASE: f64 = 0.2;
const UPRIGHT_KEYS_Z: f64 = 0.72;
const UPRIGHT_KEYS_Y: f64 = -0.27;

const STEEL: Color32 = Color32::from_rgb(170, 172, 178);
const COPPER: Color32 = Color32::from_rgb(184, 115, 70);
const IRON: Color32 = Color32::from_rgb(190, 160, 90);
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

    /// A box of `width` and `height` along the plate segment p → q at z0.
    fn beam(&mut self, p: (f64, f64), q: (f64, f64), z0: f64, width: f64, height: f64, c: Color32) {
        let pl = self.place;
        let (dx, dy) = (q.0 - p.0, q.1 - p.1);
        let l = dx.hypot(dy).max(1e-9);
        let (nx, ny) = (-dy / l * width / 2.0, dx / l * width / 2.0);
        let o = pl.at(p.0 - nx, p.1 - ny, z0);
        let u = sub(pl.at(q.0 - nx, q.1 - ny, z0), o);
        let v = sub(pl.at(p.0 + nx, p.1 + ny, z0), o);
        let w = sub(pl.at(p.0 - nx, p.1 - ny, z0 + height), o);
        self.cuboid(o, u, v, w, c);
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

/// A key's string in plate coordinates: from its agraffe to its bridge
/// point, at its height.
pub fn string_ends(model: &PianoModel, sb: &SoundboardParams, m: u8) -> ((f64, f64), (f64, f64), f64) {
    let z = STRING_Z + if m < sb.main_bridge_from { BASS_RISE } else { 0.0 };
    ((agraffe_x(model), key_across(sb, m)), sb.bridge_point(m), z)
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

    // The soundboard, a grid so that its motion can colour it.
    let (gx, gy) = BOARD_GRID;
    let wood = pick(Part::Board, sketches::WOOD);
    let colour_at = |i: usize, j: usize| -> [f32; 4] {
        match live.and_then(|l| l.board) {
            Some(field) => rgba(heat(field[j * gx + i]), 1.0),
            None => rgba(wood, 1.0),
        }
    };
    let pt = |i: usize, j: usize| place.at(sb.a * i as f64 / (gx - 1) as f64, sb.b * j as f64 / (gy - 1) as f64, 0.0);
    for j in 0..gy - 1 {
        for i in 0..gx - 1 {
            let p = [pt(i, j), pt(i + 1, j), pt(i + 1, j + 1), pt(i, j + 1)];
            let c = [colour_at(i, j), colour_at(i + 1, j), colour_at(i + 1, j + 1), colour_at(i, j + 1)];
            // Wound so that the normal points to the board's face (+z).
            b.tri(false, [p[0], p[2], p[1]], [c[0], c[2], c[1]]);
            b.tri(false, [p[0], p[3], p[2]], [c[0], c[3], c[2]]);
        }
    }
    b.plate_box((0.0, 0.0, -0.012), (sb.a, sb.b, -0.002), wood.gamma_multiply(0.7));
    b.label(place.at(sb.a * 0.75, sb.b * 0.2, 0.0), "Soundboard", "Soundboard");

    // The case: the rim's walls around the board.
    let t = if rim.enabled { rim.thickness } else { 0.02 };
    let (z0, z1) = if rim.enabled { (rim.top - rim.height, rim.top) } else { (-0.25, 0.05) };
    let case = pick(Part::Rim, CASE);
    b.plate_box((-t, -t, z0), (sb.a + t, 0.0, z1), case);
    b.plate_box((-t, sb.b, z0), (sb.a + t, sb.b + t, z1), case);
    b.plate_box((sb.a, 0.0, z0), (sb.a + t, sb.b, z1), case);
    b.plate_box((-t, 0.0, z0), (0.0, sb.b, z1.min(0.0)), case);
    b.label(place.at(sb.a + t, sb.b * 0.5, z1), if rim.enabled { "Rim" } else { "Case (not modelled)" }, "Frame & rim");

    // Bridges on the board.
    let main = pick(Part::Bridges, sketches::BRIDGE);
    let bass = if lit(Part::BassBridge) { sketches::HIGHLIGHT } else { main };
    let tb: Vec<(f64, f64)> = sb.treble_bridge.iter().map(|&(u, v)| (u * sb.a, v * sb.b)).collect();
    for w in tb.windows(2) {
        b.beam(w[0], w[1], 0.0, 0.025, BRIDGE_H, main);
    }
    let bb: Vec<(f64, f64)> = sb.bass_bridge.iter().map(|&(u, v)| (u * sb.a, v * sb.b)).collect();
    b.beam(bb[0], bb[1], 0.0, 0.03, BRIDGE_H + BASS_RISE, bass);
    let (lx, ly) = tb[2];
    b.label(place.at(lx, ly, BRIDGE_H), "Bridge", "Soundboard");
    b.label(place.at(bb[0].0, bb[0].1, BRIDGE_H + BASS_RISE), "Bass bridge", "Soundboard");

    // Strings, hammers and dampers.
    let ag = agraffe_x(model);
    let d = &model.damper;
    for m in 21..=108u8 {
        let n = model.note(m);
        let ((ax, ay), (bx, by), z) = string_ends(model, &sb, m);
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
        }
        // The agraffe, and the hammer under (an upright's: in front of)
        // the strings at the strike point.
        b.plate_box((ax - 0.004, ay - 0.005, z - 0.004), (ax + 0.004, ay + 0.005, z + 0.008), pick(Part::Agraffes, IRON));
        let k = obs.map(|o| o.keys[m as usize]);
        let down = k.is_some_and(|k| k.down);
        let hammer_on = k.is_some_and(|k| k.hammer);
        let (sx, sy) = (ax + dx * n.strike_pos, ay + dy * n.strike_pos);
        let gap = if hammer_on { 0.0 } else if down { 0.012 } else { 0.04 };
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
    let mid = string_ends(model, &sb, 60);
    let n60 = model.note(60);
    let strike = (mid.0.0 + (mid.1.0 - mid.0.0) * n60.strike_pos, mid.0.1 + (mid.1.1 - mid.0.1) * n60.strike_pos);
    b.label(place.at(strike.0, strike.1, if vertical { mid.2 + 0.04 } else { mid.2 - 0.04 }), "Hammers", "Hammer");
    let damp = (mid.0.0 + (mid.1.0 - mid.0.0) * d.position, mid.0.1 + (mid.1.1 - mid.0.1) * d.position);
    b.label(place.at(damp.0, damp.1 + 0.12, mid.2 + 0.03), "Dampers", "Dampers");
    b.label(place.at((mid.0.0 + mid.1.0) / 2.0, mid.0.1 - 0.25, mid.2), "Strings", "Strings");
    b.label(place.at(ag, sb.b * 0.92, STRING_Z + 0.01), "Agraffes", "Frame & rim");

    // The iron frame over the strings' anchoring, see-through.
    if f.enabled && o.frame {
        let c = rgba(pick(Part::Frame, IRON), if lit(Part::Frame) || lit(Part::FrameHeight) { 0.45 } else { 0.18 });
        let z = f.height;
        let q = [place.at(f.offset_x, 0.0, z), place.at(f.offset_x + f.a, 0.0, z), place.at(f.offset_x + f.a, f.b, z), place.at(f.offset_x, f.b, z)];
        b.quad(true, [q[0], q[1], q[2], q[3]], c);
        b.quad(true, [q[3], q[2], q[1], q[0]], c);
        let edge = rgba(pick(Part::Frame, IRON), 0.9);
        for i in 0..4 {
            b.line(q[i], q[(i + 1) % 4], edge);
        }
        b.label(q[1], "Frame", "Frame & rim");
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

    // The keyboard, the keys down pressed in.
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
        let (y_back, z) = if vertical { (UPRIGHT_KEYS_Y, UPRIGHT_KEYS_Z) } else { (ag - 0.03, KEY_Z) };
        let front = y_back - KEY_DEPTH;
        let o3 = [-y1 as f32, front as f32 + (KEY_DEPTH - depth) as f32, (z + zb + drop) as f32];
        b.cuboid(o3, [(y1 - y0) as f32, 0.0, 0.0], [0.0, depth as f32, 0.0], [0.0, 0.0, h as f32], c);
    }
    let (y_back, kz) = if vertical { (UPRIGHT_KEYS_Y, UPRIGHT_KEYS_Z) } else { (ag - 0.03, KEY_Z) };
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
        let ((_, a0), (_, b0), _) = string_ends(&m, &sb, 21);
        assert!(a0 > b0, "A0 from y = {a0} to {b0}");
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
