//! The keys and a grand's action, key by key, as most are made: a double
//! escapement on its rails and brackets. The model has none of it but the
//! hammers' line and blow, the dampers' place on the strings and the
//! engine's state; the rest situates them, to scale.
//!
//! Along a grand's keys (world y, away from the pianist), from the front:
//! the key on its key frame, pivoting on the balance rail; behind it, the
//! wippen on its rail, lifted by the key's capstan, carrying the jack and
//! the repetition lever; the jack under the knuckle of the hammer's shank,
//! its toe under the let-off button; the shank pivoting on its flange rail,
//! the hammer at its far end under the strings, resting on the hammer rest
//! rail; on the key's tail the backcheck, which catches the hammer as it
//! falls, and over its end the damper's underlever, hinged behind it on
//! the damper rail and lifted by it, its wire straight up under the damper
//! head on the strings.

use super::*;

/// The key: its front's travel, its sticks' width short of the action's
/// pitch (room for the brackets), the white keys' head and the black
/// keys' height and length, the crank from the keyboard's spacing to the
/// action's.
const KEY_DIP: f64 = 0.010;
const STICK_GAP: f64 = 0.0034;
const WHITE_HEAD: f64 = 0.05;
const BLACK_H: f64 = 0.012;
const BLACK_DEPTH: f64 = 0.093;
/// A white key's top, as replacement keytops are sold: a cover (ivory,
/// now plastic) some 2 mm thick (0.075–0.090 in) on the wood, its front
/// edge 1.5 mm past the wood's front (the fitters' gauge) and rounded; a
/// front plate (0.6–1.7 mm) over the whole of the wood's front, under the
/// cover's edge, which stands a little proud of it. The cover's
/// thickness, its overhang, the front plate's thickness, the cover's
/// front edge's radius.
const KEYTOP_T: f64 = 0.002;
const KEYTOP_OVER: f64 = 0.0015;
const KEY_FRONT_T: f64 = 0.001;
const KEYTOP_ROUND: f64 = 0.0007;
/// A black key's cap above the white keys: narrower at its top by this
/// much on each side, its long top edges rounded to this radius; its
/// front sloping back this far up to its top, the slope's top edge
/// rounded to this radius.
const BLACK_TAPER: f64 = 0.0012;
const BLACK_FILLET: f64 = 0.002;
const BLACK_SLOPE: f64 = 0.011;
const BLACK_NOSE: f64 = 0.004;
const CRANK: f64 = 0.035;
/// The balance rail behind the keys' front.
const BALANCE: f64 = 0.2;

/// The hammer: its shank's length to the head's centre and radius; the
/// treble's crown above the shank, its felt's thickness on the crown and
/// on the sides, its depth along the shank and width across (the bass's
/// heavier hammers larger, by their mass); its tail.
const SHANK_L: f64 = 0.135;
const SHANK_R: f32 = 0.0042;
const HEAD_H: f64 = 0.052;
/// The shank's tilt at the treble's blow (its head leaning towards the
/// pianist): the shank's lowest under the pin block's edge.
const BLOW_TILT: f64 = 0.3;
/// The crowns' height at rest (all on one line, below the board's
/// underside), and
/// below the strings once checked.
const REST_CROWN: f64 = -0.018;
const CHECKED: f64 = 0.016;
const TAIL: f64 = 0.006;
const KNUCKLE_AT: f64 = 0.017;
const KNUCKLE_R: f32 = 0.0075;

/// The wippen's pivot before the shank's, its height; the jack's length.
const WIPPEN_BACK: f64 = 0.08;
const JACK_L: f64 = 0.04;

/// The underlevers' hinge behind the hammers' line (on the damper rail, in
/// front of the belly rail), its height over the keys, and how far a
/// damper lifts.
const HINGE_BEHIND: f64 = 0.09;
const HINGE_UP: f64 = 0.008;
const DAMPER_LIFT: f64 = 0.015;
const WIRE_R: f32 = 0.0008;

/// The damper felts, cut to their strings: a single string sits in a
/// groove, a bichord's wedge goes down between its two strings, a
/// trichord's double wedge between its three (the middle one in the notch
/// between the points); the treble's thin strings, from
/// `FLAT_DAMPERS_FROM` up, take a flat felt. The wedges' and the grooves'
/// half-angles; the felt's top above the strings' centres, the cloth glued
/// over it under the head.
const FLAT_DAMPERS_FROM: u8 = 67;
const WEDGE_HALF: f64 = 0.52;
const GROOVE_HALF: f64 = 0.61;
const DAMPER_FELT_TOP: f64 = 0.008;
const DAMPER_CLOTH: f64 = 0.001;
/// The head in profile, a gendarme's hat: the felt in two, either side of
/// the slit under the hole the wire goes into; the wood low over the
/// felts, rising to a crown over the hole. Half the slit, the wood's
/// height at its ends and at its crown above the cloth, the crown's half
/// width and where the rise starts; the hole above the cloth, the
/// slit's half-width up to it.
const DAMPER_SLIT: f64 = 0.0015;
const DAMPER_END: f64 = 0.003;
const DAMPER_CROWN: f64 = 0.010;
const DAMPER_CROWN_W: f64 = 0.0025;
const DAMPER_RISE_AT: f64 = 0.0075;
const DAMPER_HOLE: f64 = 0.0065;
const DAMPER_SLOT: f64 = 0.0004;
/// Steel's density: a string's radius from its mass per length (a wound
/// string's a little under its winding's).
pub(super) const STEEL_DENSITY: f64 = 7850.0;

const KEY_WOOD: Color32 = Color32::from_rgb(222, 198, 156);
const ACTION_WOOD: Color32 = Color32::from_rgb(204, 172, 122);
const SHANK_WOOD: Color32 = Color32::from_rgb(214, 188, 140);
const MOLDING: Color32 = Color32::from_rgb(132, 80, 46);
const HAMMER_FELT: Color32 = Color32::from_rgb(232, 226, 210);
const UNDERFELT: Color32 = Color32::from_rgb(168, 44, 48);
const BUCKSKIN: Color32 = Color32::from_rgb(186, 156, 112);
const CLOTH: Color32 = Color32::from_rgb(150, 34, 44);
const LEATHER: Color32 = Color32::from_rgb(120, 78, 48);
const RAIL: Color32 = Color32::from_rgb(176, 178, 184);
const BRACKET: Color32 = Color32::from_rgb(92, 94, 102);
const LEAD: Color32 = Color32::from_rgb(112, 114, 122);

/// The material each of the action's colours stands for.
pub(super) const MATERIALS: [(Color32, Material); 13] = [
    (KEY_WOOD, Material::Wood),
    (ACTION_WOOD, Material::Wood),
    (SHANK_WOOD, Material::Wood),
    (MOLDING, Material::Wood),
    (HAMMER_FELT, Material::Felt),
    (UNDERFELT, Material::Felt),
    (CLOTH, Material::Felt),
    (sketches::FELT, Material::Felt),
    (BUCKSKIN, Material::Leather),
    (LEATHER, Material::Leather),
    (RAIL, Material::Steel),
    (BRACKET, Material::Steel),
    (LEAD, Material::Lead),
];

/// Where a run of drawn triangles and lines starts.
#[derive(Clone, Copy)]
pub(super) struct Mark(usize, usize);

impl Builder {
    pub(super) fn mark(&self) -> Mark {
        Mark(self.s.tris.len(), self.s.lines.len())
    }

    /// Turns everything drawn since `m` about the world line along x
    /// through (y, z) `at` by `angle` (from +y towards +z), then moves it
    /// by `dx` across.
    pub(super) fn turn(&mut self, m: Mark, at: (f64, f64), angle: f64, dx: f64) {
        let (s, c) = (angle.sin() as f32, angle.cos() as f32);
        let (y0, z0) = (at.0 as f32, at.1 as f32);
        let spin = |v: &mut Vertex, point: bool| {
            let [x, y, z] = v.pos;
            let (dy, dz) = if point { (y - y0, z - z0) } else { (y, z) };
            let (ry, rz) = (dy * c - dz * s, dy * s + dz * c);
            v.pos = if point { [x + dx as f32, y0 + ry, z0 + rz] } else { [x, ry, rz] };
        };
        for v in self.s.tris[m.0..].iter_mut().chain(&mut self.s.lines[m.1..]) {
            spin(v, true);
            let mut n = Vertex { pos: v.normal, ..*v };
            spin(&mut n, false);
            v.normal = n.pos;
        }
    }

    /// The polygon `p` (across, up) swept along from `a.0` to `a.1`, each
    /// point placed by `at(along, across, up)`: flat faces turned
    /// outwards, capped.
    pub(super) fn extrude(&mut self, p: &[(f64, f64)], a: (f64, f64), at: &dyn Fn(f64, f64, f64) -> V3, c: Color32) {
        let mut p = p.to_vec();
        if signed_area(&p) < 0.0 {
            p.reverse();
        }
        // Counter-clockwise across and up faces along, unless `at` mirrors
        // (an upright's plate).
        let o = at(0.0, 0.0, 0.0);
        let mirror = dot(cross(sub(at(0.0, 1.0, 0.0), o), sub(at(0.0, 0.0, 1.0), o)), sub(at(1.0, 0.0, 0.0), o)) < 0.0;
        let col = [rgba(c, 1.0); 3];
        let w = |k: usize, a: f64| at(a, p[k].0, p[k].1);
        let tri = |b: &mut Self, mut t: [V3; 3]| {
            if mirror {
                t.swap(1, 2);
            }
            b.tri(false, t, col);
        };
        for [i, j, k] in triangulate(&p) {
            tri(self, [w(i, a.1), w(j, a.1), w(k, a.1)]);
            tri(self, [w(i, a.0), w(k, a.0), w(j, a.0)]);
        }
        let n = p.len();
        for e in 0..n {
            let f = (e + 1) % n;
            tri(self, [w(e, a.0), w(f, a.0), w(f, a.1)]);
            tri(self, [w(e, a.0), w(f, a.1), w(e, a.1)]);
        }
    }

    /// A solid between the quads `lo` and `hi` (corresponding corners
    /// joined), its faces flat and turned outwards.
    pub(super) fn hexa(&mut self, lo: [V3; 4], hi: [V3; 4], c: Color32) {
        let c = rgba(c, 1.0);
        let all: Vec<V3> = lo.iter().chain(&hi).copied().collect();
        let centre = scale(all.iter().fold([0.0; 3], |a, &p| add(a, p)), 1.0 / 8.0);
        let mut face = |mut q: [V3; 4]| {
            let n = cross(sub(q[2], q[0]), sub(q[3], q[1]));
            let mid = scale(add(add(q[0], q[1]), add(q[2], q[3])), 0.25);
            if dot(n, sub(mid, centre)) < 0.0 {
                q.reverse();
            }
            self.quad(false, q, c);
        };
        face(lo);
        face(hi);
        for i in 0..4 {
            let j = (i + 1) % 4;
            face([lo[i], lo[j], hi[j], hi[i]]);
        }
    }

    /// A solid through the `sections` (world points, each a closed loop of
    /// as many points, the one joined to the next), capped at both ends,
    /// its faces turned outwards; shaded smooth where two faces meet at
    /// less than [`SMOOTH_DEG`], around a section or along.
    pub(super) fn loft(&mut self, sections: &[Vec<V3>], c: Color32) {
        let (n, m) = (sections.len(), sections[0].len());
        if n < 2 || m < 3 {
            return;
        }
        let col = rgba(c, 1.0);
        let all = sections.iter().flatten();
        let centre = scale(all.clone().fold([0.0; 3], |a, &p| add(a, p)), 1.0 / (n * m) as f32);
        // Each quad's corners (section i or i + 1, point k or k + 1) and
        // its outward normal; whether the loops' order turns them inwards.
        let corners = |i: usize, k: usize| [sections[i][k], sections[i + 1][k], sections[i + 1][(k + 1) % m], sections[i][(k + 1) % m]];
        let face = |i: usize, k: usize| {
            let q = corners(i, k);
            norm(cross(sub(q[2], q[0]), sub(q[3], q[1])))
        };
        let mid = |i: usize, k: usize| {
            let q = corners(i, k);
            scale(add(add(q[0], q[1]), add(q[2], q[3])), 0.25)
        };
        let votes: f32 = (0..n - 1).flat_map(|i| (0..m).map(move |k| (i, k))).map(|(i, k)| dot(face(i, k), sub(mid(i, k), centre)).signum()).sum();
        let flip = if votes < 0.0 { -1.0 } else { 1.0 };
        let normals: Vec<Vec<V3>> = (0..n - 1).map(|i| (0..m).map(|k| scale(face(i, k), flip)).collect()).collect();
        let smooth = SMOOTH_DEG.to_radians().cos() as f32;
        // At corner (i, k) of quad (qi, qk): the faces meeting there that
        // bend from it gently.
        let normal = |qi: usize, qk: usize, i: usize, k: usize| {
            let own = normals[qi][qk];
            let mut sum = [0.0; 3];
            for fi in [i.wrapping_sub(1), i] {
                for fk in [(k + m - 1) % m, k % m] {
                    if fi < n - 1 && dot(normals[fi][fk], own) > smooth {
                        sum = add(sum, normals[fi][fk]);
                    }
                }
            }
            norm(sum)
        };
        for i in 0..n - 1 {
            for k in 0..m {
                let mut q = corners(i, k);
                let mut nn = [normal(i, k, i, k), normal(i, k, i + 1, k), normal(i, k, i + 1, k + 1), normal(i, k, i, k + 1)];
                if flip < 0.0 {
                    q.reverse();
                    nn.reverse();
                }
                self.quad_n(q, nn, col);
            }
        }
        // The caps, facing away from the other end.
        let mean = |s: &[V3]| scale(s.iter().fold([0.0; 3], |a, &p| add(a, p)), 1.0 / m as f32);
        let (first, last) = (mean(&sections[0]), mean(&sections[n - 1]));
        for (i, out) in [(0, sub(first, last)), (n - 1, sub(last, first))] {
            let s = &sections[i];
            let c = mean(s);
            for k in 0..m {
                let mut t = [c, s[k], s[(k + 1) % m]];
                if dot(cross(sub(t[1], t[0]), sub(t[2], t[0])), out) < 0.0 {
                    t.swap(1, 2);
                }
                self.tri(false, t, [col; 3]);
            }
        }
    }

    /// A rod of radius `r` across the keys (world x) from `x0` to `x1`,
    /// its centre at (y, z): a prism of ten faces, shaded smooth.
    fn rod_x(&mut self, x0: f64, x1: f64, y: f64, z: f64, r: f64, c: Color32) {
        let p: Vec<(f64, f64)> = (0..10).map(|k| (std::f64::consts::TAU * k as f64 / 10.0).sin_cos()).map(|(s, co)| (y + r * co, z + r * s)).collect();
        self.piece(&p, (x0 + x1) / 2.0, x1 - x0, c);
    }

    /// A rod from `a` to `b` (world), its radius `r.0` at `a` and `r.1`
    /// at `b`: a prism of eight faces, shaded smooth, capped.
    fn rod(&mut self, a: V3, b: V3, r: (f32, f32), c: Color32) {
        const SIDES: usize = 8;
        let axis = norm(sub(b, a));
        let side = if axis[0].abs() < 0.9 { [1.0, 0.0, 0.0] } else { [0.0, 1.0, 0.0] };
        let u = norm(sub(side, scale(axis, dot(side, axis))));
        let v = cross(axis, u);
        let out: [V3; SIDES] = std::array::from_fn(|k| {
            let (s, co) = (std::f32::consts::TAU * k as f32 / SIDES as f32).sin_cos();
            add(scale(u, co), scale(v, s))
        });
        let col = rgba(c, 1.0);
        for k in 0..SIDES {
            let l = (k + 1) % SIDES;
            let p = [add(a, scale(out[k], r.0)), add(b, scale(out[k], r.1)), add(b, scale(out[l], r.1)), add(a, scale(out[l], r.0))];
            self.quad_n(p, [out[k], out[k], out[l], out[l]], col);
        }
        for (end, rr, n) in [(a, r.0, scale(axis, -1.0)), (b, r.1, axis)] {
            for k in 0..SIDES {
                let l = (k + 1) % SIDES;
                for pos in [end, add(end, scale(out[k], rr)), add(end, scale(out[l], rr))] {
                    let v = self.vertex(pos, n, col);
                    self.s.tris.push(v);
                }
            }
        }
    }

    /// The band between the profiles `outer` and `inner` in (world y, z),
    /// as many points each, the one over the other (a felt wrapped round a
    /// molding), from `x - w/2` to `x + w/2`: shaded smooth along them.
    fn band(&mut self, outer: &[(f64, f64)], inner: &[(f64, f64)], x: f64, w: f64, c: Color32) {
        let col = rgba(c, 1.0);
        let n = outer.len();
        let (x0, x1) = ((x - w / 2.0) as f32, (x + w / 2.0) as f32);
        let at = |p: (f64, f64), x: f32| [x, p.0 as f32, p.1 as f32];
        // A profile's normal at point i, turned `side` (1: outwards on the
        // outer, which runs left to right over the top).
        let normal = |p: &[(f64, f64)], i: usize, side: f64| {
            let (a, b) = (p[i.saturating_sub(1)], p[(i + 1).min(n - 1)]);
            let (dy, dz) = (b.0 - a.0, b.1 - a.1);
            norm([0.0, (-dz * side) as f32, (dy * side) as f32])
        };
        for i in 0..n - 1 {
            for x in [x0, x1] {
                self.quad(false, [at(outer[i], x), at(outer[i + 1], x), at(inner[i + 1], x), at(inner[i], x)], col);
            }
            let (a, b) = (normal(outer, i, 1.0), normal(outer, i + 1, 1.0));
            self.quad_n([at(outer[i], x0), at(outer[i + 1], x0), at(outer[i + 1], x1), at(outer[i], x1)], [a, b, b, a], col);
            let (a, b) = (normal(inner, i, -1.0), normal(inner, i + 1, -1.0));
            self.quad_n([at(inner[i], x0), at(inner[i + 1], x0), at(inner[i + 1], x1), at(inner[i], x1)], [a, b, b, a], col);
        }
        for i in [0, n - 1] {
            self.quad(false, [at(outer[i], x0), at(inner[i], x0), at(inner[i], x1), at(outer[i], x1)], col);
        }
    }

    /// The profile `p` in (world y, z), from `x - w/2` to `x + w/2`.
    fn piece(&mut self, p: &[(f64, f64)], x: f64, w: f64, c: Color32) {
        let p: Vec<(f32, f32)> = p.iter().map(|&(y, z)| (y as f32, z as f32)).collect();
        self.slab(&p, (x - w / 2.0) as f32, (x + w / 2.0) as f32, c);
    }

    /// A box in (world y, z) `y0..y1`, `z0..z1`, across `x ± w/2`.
    fn block(&mut self, y: (f64, f64), z: (f64, f64), x: f64, w: f64, c: Color32) {
        self.piece(&[(y.0, z.0), (y.1, z.0), (y.1, z.1), (y.0, z.1)], x, w, c);
    }
}

fn rot(p: (f64, f64), at: (f64, f64), a: f64) -> (f64, f64) {
    let (s, c) = a.sin_cos();
    let (dy, dz) = (p.0 - at.0, p.1 - at.1);
    (at.0 + dy * c - dz * s, at.1 + dy * s + dz * c)
}

/// An arch from (−w, base) over a crown at (0, top) down to (w, base),
/// left to right: straight sides up to `shoulder`, then a rounded top.
fn arch(w: f64, base: f64, shoulder: f64, top: f64) -> Vec<(f64, f64)> {
    let mut p = vec![(-w, base)];
    for k in 0..=8 {
        let t = std::f64::consts::PI * (1.0 - k as f64 / 8.0);
        p.push((w * t.cos(), shoulder + (top - shoulder) * t.sin().powf(0.75)));
    }
    p.push((w, base));
    p
}

/// What the keys and the action need from the instrument.
pub(super) struct Ctx<'a> {
    pub model: &'a PianoModel,
    pub sb: &'a SoundboardParams,
    /// The keys' strings from A0 ([`string_ends`]), and the hammers' line.
    pub ends: &'a [StringEnds],
    pub hx: f64,
    pub obs: Option<&'a Observation>,
    pub key: u8,
    pub lit: &'a dyn Fn(Part) -> bool,
    pub level: &'a dyn Fn(u8) -> f32,
    /// The keyboard across (plate y) and its white keys' width.
    pub span: (f64, f64),
    /// World: the keys' front and the back of their visible part, the
    /// bottom of the keys.
    pub key_front: f64,
    pub key_back: f64,
    pub kz: f64,
    pub vertical: bool,
    /// The parts the case hides drawn too (the case left out).
    pub inside: bool,
}

impl Ctx<'_> {
    fn pick(&self, p: Part, c: Color32) -> Color32 {
        if (self.lit)(p) { sketches::HIGHLIGHT } else { c }
    }

    fn state(&self, m: u8) -> KeyState {
        self.obs.map_or_else(KeyState::default, |o| o.keys[m as usize])
    }

    /// World x across of key `m`'s action (its strings' strike point).
    fn across(&self, m: u8) -> f64 {
        -key_across(self.model, self.sb, m)
    }

    /// The room across for key `m`'s action: the nearer of its
    /// neighbours' (the bass's hammers may be closer than the rest's).
    fn pitch(&self, m: u8) -> f64 {
        let gap = |n: u8| (self.across(n) - self.across(n + 1)).abs();
        match m {
            ..=21 => gap(21),
            108.. => gap(107),
            _ => gap(m - 1).min(gap(m)),
        }
    }
}

/// The action's fixed points (world y, z), the same for every key.
struct Geo {
    /// The hammers' line, the shanks' pivot, the wippens' pivot.
    yh: f64,
    pivot: (f64, f64),
    wippen: (f64, f64),
    balance: (f64, f64),
    /// The capstans, jacks and repetition levers' posts along; the top of
    /// the wippens and the bottom of their heels, the knuckle's bottom (the
    /// jack's top) at rest for the treble's hammers.
    cap: f64,
    jack: f64,
    post: f64,
    wippen_top: f64,
    heel: f64,
    knuckle: f64,
    /// The underlevers' hinge.
    hinge: (f64, f64),
}

/// A key's hammer: crown above the shank, depth, width, felt on the crown
/// and on the sides; its shank's angles at rest, on the strings and
/// checked.
struct Hammer {
    h: f64,
    d: f64,
    w: f64,
    felt: f64,
    side: f64,
    rest: f64,
    blow: f64,
    checked: f64,
}

impl Geo {
    fn new(c: &Ctx) -> Self {
        let yh = c.hx;
        let (s, co) = BLOW_TILT.sin_cos();
        let pivot = (yh - (SHANK_L * co - HEAD_H * s), STRING_Z - (SHANK_L * s + HEAD_H * co));
        let treble = Self::hammer_at(pivot, yh, STRING_Z, 0.0, 0.0);
        let knuckle = Self::knuckle_bottom(pivot, treble.rest);
        let jack = pivot.0 + KNUCKLE_AT * treble.rest.cos();
        let wippen_top = knuckle - JACK_L - 0.002;
        let wippen = (pivot.0 - WIPPEN_BACK, wippen_top - 0.005);
        let key_top = c.kz + 0.022;
        Self {
            yh,
            pivot,
            wippen,
            balance: (c.key_front + BALANCE, c.kz),
            cap: wippen.0 + 0.6 * (jack - wippen.0),
            jack,
            post: jack - 0.042,
            wippen_top,
            heel: wippen_top - 0.019,
            knuckle,
            hinge: (yh + HINGE_BEHIND, key_top + HINGE_UP),
        }
    }

    /// The hammer that reaches the strings at height `z` from `pivot` on
    /// the line `yh`, its crown on the rest line; `size` 0 (treble) … 1
    /// (the heaviest bass) sets its felt.
    fn hammer_at(pivot: (f64, f64), yh: f64, z: f64, size: f64, pitch: f64) -> Hammer {
        let (dy, dz) = (yh - pivot.0, z - pivot.1);
        let reach = dy.hypot(dz);
        let h = (reach * reach - SHANK_L * SHANK_L).max(0.0).sqrt();
        let phi = h.atan2(SHANK_L);
        let at = |crown: f64| ((crown - pivot.1) / reach).clamp(-1.0, 1.0).asin() - phi;
        let w = 0.0085 + 0.003 * size;
        Hammer {
            h,
            d: 0.017 + 0.008 * size,
            w: if pitch > 0.0 { w.min(pitch - 0.0035) } else { w },
            felt: 0.011 + 0.007 * size,
            side: 0.0032 + 0.002 * size,
            rest: at(REST_CROWN),
            blow: dz.atan2(dy) - phi,
            checked: at(z - CHECKED),
        }
    }

    fn knuckle_bottom(pivot: (f64, f64), angle: f64) -> f64 {
        rot((pivot.0 + KNUCKLE_AT, pivot.1 - SHANK_R as f64 - 0.85 * KNUCKLE_R as f64), pivot, angle).1 - KNUCKLE_R as f64
    }
}

/// The keyboard (and a grand's keys' sticks and action behind it).
pub(super) fn keys(b: &mut Builder, c: &Ctx) {
    let g = Geo::new(c);
    let (klo, khi) = c.span;
    let white = (khi - klo) / 52.0;
    let wx = |u: f32| u as f64 * white - khi;
    let shift = if c.vertical { 0.0 } else { 0.0025 * c.obs.map_or(0.0, |o| o.soft as f64) };
    let key_top = c.kz + 0.022;
    // An upright's keys pivot out of sight behind the keyboard.
    let balance = if c.vertical { (c.key_back + 0.15, c.kz) } else { g.balance };
    let dip = (KEY_DIP / (balance.0 - c.key_front)).asin();
    b.mat = Material::Wood;
    for m in 21..=108u8 {
        let k = c.state(m);
        let black = sketches::is_black(m);
        let (x, w) = sketches::key_span(m);
        let (x0, x1) = (wx(x) + 0.0006, wx(x + w) - 0.0006);
        let base = if black { EBONY } else { IVORY };
        let top = if k.down {
            sketches::HIGHLIGHT
        } else if m == c.key {
            mix(base, sketches::HIGHLIGHT, 0.5)
        } else {
            c.pick(Part::Key, base)
        };
        let top = mix(top, sketches::HIGHLIGHT, (c.level)(m) * 0.4);
        let mark = b.mark();
        b.mat = if black { Material::Ebony } else { Material::KeyTop };
        let (y0, y1) = (c.key_front, c.key_back);
        // The part the pianist sees, and where it leaves the keyboard:
        // a white key's narrow part between its black neighbours.
        let tail = if black {
            let yf = y1 - BLACK_DEPTH;
            b.block((yf, y1), (c.kz, key_top), (x0 + x1) / 2.0, x1 - x0, top);
            b.loft(&black_cap((x0, x1), (yf, y1), key_top), top);
            (x0, x1)
        } else {
            let edge = |n: u8, left: bool| {
                if (21..=108).contains(&n) && sketches::is_black(n) {
                    let (x, w) = sketches::key_span(n);
                    Some(if left { wx(x + w) + 0.0006 } else { wx(x) - 0.0006 })
                } else {
                    None
                }
            };
            let (n0, n1) = (edge(m.wrapping_sub(1), true).unwrap_or(x0), edge(m + 1, false).unwrap_or(x1));
            // The wood under its cover and behind its lip; the cover.
            let (yh, zc) = (y0 + WHITE_HEAD, key_top - KEYTOP_T);
            b.block((y0, yh), (c.kz, zc), (x0 + x1) / 2.0, x1 - x0, KEY_WOOD);
            b.block((yh, y1), (c.kz, zc), (n0 + n1) / 2.0, n1 - n0, KEY_WOOD);
            b.piece(&keytop_head(y0, yh, (c.kz, key_top)), (x0 + x1) / 2.0, x1 - x0, top);
            b.block((yh, y1), (zc, key_top), (n0 + n1) / 2.0, n1 - n0, top);
            (n0, n1)
        };
        b.mat = Material::Wood;
        if c.vertical {
            let angle = if k.down { dip } else { 0.0 };
            b.turn(mark, balance, angle, shift);
            continue;
        }
        let cx = c.across(m);
        let hammer = hammer(c, &g, m);
        let checked = tail_at(&g, &hammer, hammer.checked);
        let backcheck = checked.0 + 0.008;
        // On to under its damper's wire.
        let end = if m <= c.model.damper.highest_note { (backcheck + 0.012).max(damper_wire(c, m) + 0.008) } else { backcheck + 0.012 };
        if !c.inside {
            b.turn(mark, g.balance, if k.down { dip } else { 0.0 }, shift);
            action(b, c, &g, m, &hammer, shift, if k.down { dip } else { 0.0 });
            continue;
        }
        // Cranked from the keyboard's spacing to the action's, then
        // straight to the tail.
        let stick = c.pitch(m) - STICK_GAP;
        let (s0, s1) = (cx - stick / 2.0, cx + stick / 2.0);
        let (yc, z0, z1) = (y1 + CRANK, c.kz, key_top);
        let lo = [[tail.0, y1, z0], [tail.1, y1, z0], [s1, yc, z0], [s0, yc, z0]];
        let hi = lo.map(|[x, y, _]| [x, y, z1]);
        b.hexa(lo.map(f32s), hi.map(f32s), KEY_WOOD);
        b.block((yc, end), (z0, z1), cx, stick, KEY_WOOD);
        // Its leads, more in the bass, in front of the balance rail.
        let leads = 1 + (108 - m as usize) / 30;
        for i in 0..leads {
            b.rod_x(s0 - 0.0003, s1 + 0.0003, g.balance.0 - 0.022 - i as f64 * 0.016, c.kz + 0.011, 0.0065, LEAD);
        }
        // The capstan under the wippen's heel.
        let k_hammer = hammer.rest;
        let wippen_rest = wippen_angle(&g, k_hammer);
        let heel = rot((g.cap, g.heel - 0.003), g.wippen, wippen_rest).1;
        let hc = (heel - key_top) as f32;
        #[rustfmt::skip]
        let rings = [(0.0, 0.0028, false), (hc - 0.007, 0.0028, false), (hc - 0.007, 0.0046, false), (hc - 0.0018, 0.0046, false), (hc - 0.0004, 0.0032, false), (hc, 0.0012, false)];
        b.lathe(f32s([cx, g.cap, key_top]), [0.0, 0.0, 1.0], [1.0, 0.0, 0.0], &rings, &[BRASS]);
        // The backcheck on the tail, facing the hammer's tail as it falls.
        let bz = checked.1 + 0.005;
        b.rod(f32s([cx, backcheck + 0.002, key_top - 0.002]), f32s([cx, backcheck + 0.003, bz - 0.012]), (0.0011, 0.0011), STEEL);
        let lean = 0.06;
        let head = [(backcheck - 0.0015, bz - 0.013), (backcheck + 0.006, bz - 0.013), (backcheck + 0.006 + lean * 0.026, bz + 0.013), (backcheck - 0.0015 + lean * 0.026, bz + 0.013)];
        b.piece(&head, cx, 0.0085, ACTION_WOOD);
        let face = [(backcheck - 0.0035, bz - 0.012), (backcheck - 0.0015, bz - 0.012), (backcheck - 0.0015 + lean * 0.024, bz + 0.012), (backcheck - 0.0035 + lean * 0.024, bz + 0.012)];
        b.piece(&face, cx, 0.0085, LEATHER);
        // The pressed key turns about its balance rail, the soft pedal
        // shifts it with the action.
        b.turn(mark, g.balance, if k.down { dip } else { 0.0 }, shift);
        action(b, c, &g, m, &hammer, shift, if k.down { dip } else { 0.0 });
    }
    if c.inside && !c.vertical {
        rails(b, c, &g, shift);
    }
    b.mat = Material::Paint;
}

/// A quarter of a circle (y, z) about `c`, radius `r`, from angle `a0`
/// to `a1` (degrees), both ends included.
fn quarter(c: (f64, f64), r: f64, a0: f64, a1: f64) -> impl Iterator<Item = (f64, f64)> {
    (0..=4).map(move |k| {
        let a = (a0 + (a1 - a0) * k as f64 / 4.0).to_radians();
        (c.0 + r * a.cos(), c.1 + r * a.sin())
    })
}

/// A white key's cover over its head and its front plate, in (y, z):
/// from the wood's front `y0` back to `yh`, the key from `z.0` up to its
/// top `z.1`; the cover's front edge past the plate, rounded.
fn keytop_head(y0: f64, yh: f64, z: (f64, f64)) -> Vec<(f64, f64)> {
    let (top, zc) = (z.1, z.1 - KEYTOP_T);
    let (yp, yf) = (y0 - KEY_FRONT_T, y0 - KEYTOP_OVER);
    let mut p = vec![(yh, zc), (y0, zc), (y0, z.0), (yp, z.0), (yp, zc), (yf, zc)];
    p.extend(quarter((yf + KEYTOP_ROUND, top - KEYTOP_ROUND), KEYTOP_ROUND, 180.0, 90.0));
    p.push((yh, top));
    p
}

/// A black key's cap from `x.0` to `x.1` across, `y.0` (its front) to
/// `y.1`, standing on `base`: its sections along it, each narrower at its
/// top, the top's long edges rounded, the front sloping back to the top
/// over a rounded edge.
fn black_cap(x: (f64, f64), y: (f64, f64), base: f64) -> Vec<Vec<V3>> {
    let (h, f, r) = (BLACK_H, BLACK_SLOPE, BLACK_NOSE);
    // The profile along (from the front, height): up the slope, round its
    // top edge (a fillet between the slope and the top), on to the back.
    let slope = h.atan2(f);
    let t = r * (slope / 2.0).tan();
    let (dy, dz) = (f / f.hypot(h), h / f.hypot(h));
    let mut along = vec![(0.0, 0.0008), (f - dy * t, h - dz * t)];
    along.extend(quarter((f + t, h - r), r, 90.0 + slope.to_degrees(), 90.0).skip(1));
    along.push((y.1 - y.0, h));
    let cx = (x.0 + x.1) / 2.0;
    let w = x.1 - x.0;
    along
        .into_iter()
        .map(|(a, hh)| {
            let top = w / 2.0 - BLACK_TAPER * hh / h;
            let rr = BLACK_FILLET.min(0.45 * hh).min(0.45 * top);
            let mut s = vec![(cx - w / 2.0, base), (cx + w / 2.0, base)];
            s.extend(quarter((cx + top - rr, base + hh - rr), rr, 0.0, 90.0));
            s.extend(quarter((cx - top + rr, base + hh - rr), rr, 90.0, 180.0));
            s.into_iter().map(|(sx, z)| f32s([sx, y.0 + a, z])).collect()
        })
        .collect()
}

fn f32s(p: [f64; 3]) -> V3 {
    p.map(|v| v as f32)
}

fn hammer(c: &Ctx, g: &Geo, m: u8) -> Hammer {
    let z = c.ends[(m - 21) as usize].2;
    let mass = c.model.note(m).hammer.mass_kg;
    let size = ((mass - 0.0054) / (0.011 - 0.0054)).clamp(0.0, 1.0);
    Geo::hammer_at(g.pivot, g.yh, z, size, c.pitch(m))
}

/// The bottom of the molding's tail (world y, z) with the shank at `angle`.
fn tail_at(g: &Geo, h: &Hammer, angle: f64) -> (f64, f64) {
    rot((g.pivot.0 + SHANK_L + h.d / 2.0 + TAIL, g.pivot.1 - 0.0075), g.pivot, angle)
}

/// The wippen's angle about its pivot with its jack under the knuckle of
/// a shank at rest at `angle` (the capstans set to it).
fn wippen_angle(g: &Geo, angle: f64) -> f64 {
    let drop = g.knuckle - Geo::knuckle_bottom(g.pivot, angle);
    -(drop / (g.jack - g.wippen.0)).atan()
}

/// One key's action over its key turned by `dip`, shifted across by
/// `shift`: only what shows through the case unless it is left out.
#[allow(clippy::too_many_arguments)]
fn action(b: &mut Builder, c: &Ctx, g: &Geo, m: u8, h: &Hammer, shift: f64, dip: f64) {
    let k = c.state(m);
    let cx = c.across(m);
    let level = (c.level)(m);
    // The wippen, lifted by the capstan as the key goes down.
    if c.inside {
        let rest = wippen_angle(g, h.rest);
        let cap_rise = rot((g.cap, g.balance.1 + 0.022), g.balance, dip).1 - (g.balance.1 + 0.022);
        let lift = (cap_rise / (g.cap - g.wippen.0)).atan();
        wippen(b, c, g, cx, k.down, rest + lift, shift);
    }

    // The hammer, its shank at the angle the engine's state gives it.
    let angle = if k.hammer {
        h.blow
    } else if k.down {
        h.checked
    } else {
        h.rest
    };
    let mark = b.mark();
    let (py, pz) = g.pivot;
    let hy = py + SHANK_L;
    b.rod(f32s([cx, py - 0.006, pz]), f32s([cx, hy + 0.002, pz]), (SHANK_R, SHANK_R * 0.88), c.pick(Part::Shank, SHANK_WOOD));
    if c.inside {
        let kz = pz - SHANK_R as f64 - 0.85 * KNUCKLE_R as f64;
        b.rod_x(cx - 0.006, cx + 0.006, py + KNUCKLE_AT, kz, KNUCKLE_R as f64, BUCKSKIN);
    }
    // The head: the molding, its tail at the back, under the underfelt
    // and the felt, wrapped round its top.
    let (d, hh) = (h.d / 2.0, h.h);
    let (mw, mt) = (d - h.side, hh - h.felt);
    let under = (h.side * 0.35, h.felt * 0.35);
    let base = mt - 0.6 * h.d;
    let at = |p: Vec<(f64, f64)>| -> Vec<(f64, f64)> { p.into_iter().map(|(a, z)| (hy + a, pz + z)).collect() };
    let mut molding = vec![(-mw, -0.0075), (mw + TAIL, -0.0075), (mw + TAIL, -0.002), (mw, 0.006)];
    let top = arch(mw, base + 0.004, mt - 0.006, mt);
    molding.extend(top.iter().rev().skip(1).take(top.len() - 2).copied());
    b.piece(&at(molding), cx, h.w, MOLDING);
    let band = |inner: (f64, f64, f64), outer: (f64, f64, f64), col: Color32, b: &mut Builder| {
        b.band(&at(arch(outer.0, base, outer.1, outer.2)), &at(arch(inner.0, base, inner.1, inner.2)), cx, h.w, col);
    };
    let felt = if k.hammer { sketches::HIGHLIGHT } else { mix(c.pick(Part::Strike, c.pick(Part::Felt, HAMMER_FELT)), sketches::HIGHLIGHT, level * 0.3) };
    let inner = (mw, mt - 0.006, mt);
    let mid = (mw + under.0, mt - 0.006 + under.1 * 0.5, mt + under.1);
    band(inner, mid, c.pick(Part::Core, UNDERFELT), b);
    b.mat = Material::Felt;
    band(mid, (d, mt - 0.006 + h.felt * 0.5, hh), felt, b);
    b.mat = Material::Wood;
    b.turn(mark, g.pivot, angle, shift);
    // Its flange on the rail in front of the pivot.
    if c.inside {
        b.block((py - 0.01, py + 0.004), (pz - 0.006, pz + 0.006), cx + shift, 0.009, ACTION_WOOD);
    }

    // The damper: its underlever, hinged behind on the damper rail, over
    // the key's end, which lifts it (or the pedal does) by the cloth under
    // its wire block; the wire straight up under the head on the strings.
    // None in the high treble.
    let d = &c.model.damper;
    if m > d.highest_note {
        return;
    }
    let lift = DAMPER_LIFT * (1.0 - k.damper as f64);
    let (hy, hz) = g.hinge;
    let wire = damper_wire(c, m);
    if c.inside {
        let key_top = c.kz + 0.022;
        let mark = b.mark();
        b.block((wire - 0.004, hy + 0.004), (hz - 0.004, hz + 0.004), cx, 0.008, c.pick(Part::Damper, ACTION_WOOD));
        b.block((wire - 0.004, wire + 0.006), (key_top, hz - 0.004), cx, 0.008, CLOTH);
        b.block((wire - 0.004, wire + 0.004), (hz + 0.004, hz + 0.01), cx, 0.01, ACTION_WOOD);
        b.turn(mark, g.hinge, -(lift / (hy - wire)).atan(), 0.0);
        b.block((hy - 0.004, hy + 0.006), (hz - 0.006, hz + 0.006), cx, 0.008, ACTION_WOOD);
    }
    // The head on the strings, along them.
    let ((ax, ay), (bx, by), z) = c.ends[(m - 21) as usize];
    let (ux, uy) = {
        let l = (bx - ax).hypot(by - ay).max(1e-9);
        ((bx - ax) / l, (by - ay) / l)
    };
    let (px, py) = damper_at(c.model, c.ends[(m - 21) as usize], m);
    let place = b.place;
    let corner = |a: f64, s: f64, h: f64| place.at(px + ux * a - uy * s, py + uy * a + ux * s, z + lift + h);
    let (half, hole) = damper_head(b, &corner, c.model, m, c.pick(Part::DamperPoint, DAMPER_FELT), c.pick(Part::Damper, DAMPER_WOOD));
    // The wire: up from the underlever, through the gap to the next
    // unison to the hole's height, and across into it.
    let across = |n: u8| {
        let ((ax, ay), (bx, by), _) = c.ends[(n - 21) as usize];
        ay + (wire - ax) * (by - ay) / (bx - ax)
    };
    let bass = |n: u8| n < c.sb.main_bridge_from;
    let next = if m < 108 && (bass(m + 1) == bass(m) || m == 21) { m + 1 } else { m - 1 };
    let gap = -(across(m) + across(next)) / 2.0;
    let side = [corner(0.0, -half, hole), corner(0.0, half, hole)];
    let into = if (side[0][0] - gap as f32).abs() < (side[1][0] - gap as f32).abs() { side[0] } else { side[1] };
    let lever = hz + 0.01 + lift;
    let up = f32s([gap, wire, into[2] as f64]);
    let path = [f32s([cx, wire, lever]), f32s([cx, wire, lever + 0.012]), f32s([gap, wire, lever + 0.024]), up, into];
    for p in path[if c.inside { 0 } else { 2 }..].windows(2) {
        b.rod(p[0], p[1], (WIRE_R, WIRE_R), STEEL);
    }
}


/// Where key `m`'s damper rests on its strings with `ends` (plate x, y):
/// the share of the speaking length the engine damps at, from its own
/// length (the drawn string's is the bridge's and the hammers' line's),
/// laid on the string as drawn.
pub(super) fn damper_at(model: &PianoModel, ends: StringEnds, m: u8) -> (f64, f64) {
    let n = model.note(m);
    damper_point(ends, model.damper.place(n.strike_pos, n.length))
}

/// A grand's key `m`'s damper wire along (world y): under its head.
fn damper_wire(c: &Ctx, m: u8) -> f64 {
    damper_at(c.model, c.ends[(m - 21) as usize], m).0
}

/// Where the strings of a key with `ends` pass at `t` of their speaking
/// length (plate x, y): as they are drawn, towards their bridge pins'
/// gap ([`build`]).
pub(super) fn damper_point(ends: StringEnds, t: f64) -> (f64, f64) {
    let ((ax, ay), (bx, by), _) = ends;
    let (dx, dy) = (bx - ax, by - ay);
    let l = dx.hypot(dy).max(1e-9);
    let (ux, uy, nx, ny) = (dx / l, dy / l, -dy / l, dx / l);
    let (cx, cy) = (-ux * BRIDGE_PIN_ALONG + nx * BRIDGE_PIN_SIDE, -uy * BRIDGE_PIN_ALONG + ny * BRIDGE_PIN_SIDE);
    (ax + (dx + cx) * t, ay + (dy + cy) * t)
}

/// A damper's felt in section across its `strings` of radius `r`
/// (across, up from their centres), cut to them unless `flat`, and its
/// half-width.
fn felt_section(strings: usize, r: f64, flat: bool) -> (Vec<(f64, f64)>, f64) {
    let (g, top) = (UNISON_GAP, DAMPER_FELT_TOP);
    let half = (strings as f64 - 1.0) * g / 2.0 + 0.0025;
    if flat || !(1..=3).contains(&strings) {
        return (vec![(-half, r), (half, r), (half, top), (-half, top)], half);
    }
    if strings == 1 {
        // The groove's sides on the string, the felt's lips down to its
        // lower half.
        let r = r.min(0.003);
        let (s, co) = GROOVE_HALF.sin_cos();
        let (apex, lip) = (r / s, -0.5 * r);
        let w = (apex - lip) * s / co;
        let half = half.max(w + 0.0015);
        return (vec![(-half, top), (-half, lip), (-w, lip), (0.0, apex), (w, lip), (half, lip), (half, top)], half);
    }
    // A point between each two strings, its flanks on both (the strings
    // drawn as lines: a wound string's radius kept within the gap).
    let r = r.min(0.4 * g);
    let (s, co) = WEDGE_HALF.sin_cos();
    let depth = (g / 2.0 * co - r) / s;
    let rise = |across: f64| across * co / s - depth;
    let tips: Vec<f64> = (0..strings - 1).map(|k| (k as f64 - (strings as f64 - 2.0) / 2.0) * g).collect();
    let mut p = vec![(-half, top), (-half, rise(half + tips[0]))];
    for (i, &t) in tips.iter().enumerate() {
        p.push((t, -depth));
        if let Some(&next) = tips.get(i + 1) {
            p.push(((t + next) / 2.0, rise((next - t) / 2.0)));
        }
    }
    let last = tips[tips.len() - 1];
    p.extend([(half, rise(half - last)), (half, top)]);
    (p, half)
}

/// Key `m`'s damper head, placed by `at(along, across, up)` from its
/// strings' centres under it: as long as the engine's pad, its felt cut
/// to them in two pads either side of the slit, the cloth over them, the
/// wooden head in a gendarme's hat. Its half-width across and its hole's
/// height, for the wire.
pub(super) fn damper_head(b: &mut Builder, at: &dyn Fn(f64, f64, f64) -> V3, model: &PianoModel, m: u8, felt: Color32, wood: Color32) -> (f64, f64) {
    let n = model.note(m);
    let r = (n.mu / (STEEL_DENSITY * std::f64::consts::PI)).sqrt();
    let (section, half) = felt_section(n.strings, r, m >= FLAT_DAMPERS_FROM);
    let l = model.damper.width / 2.0;
    let base = DAMPER_FELT_TOP + DAMPER_CLOTH;
    for pad in [(-l, -DAMPER_SLIT), (DAMPER_SLIT, l)] {
        b.extrude(&section, pad, at, felt);
    }
    let rect = |s: f64, h: (f64, f64)| [(-s, h.0), (s, h.0), (s, h.1), (-s, h.1)];
    b.extrude(&rect(half, (DAMPER_FELT_TOP, base)), (-l, l), at, CLOTH);
    // The wood in profile (along, up), swept across: its top from end to
    // end, its ends' top corners rounded, then its bottom back with the
    // slit up to the hole.
    let top = |a: f64| {
        let t = ((a.abs() - DAMPER_CROWN_W) / (DAMPER_RISE_AT - DAMPER_CROWN_W)).clamp(0.0, 1.0);
        let crown = DAMPER_CROWN - 0.0015 * (a / DAMPER_CROWN_W).min(1.0).powi(2);
        let h = DAMPER_END + (crown - DAMPER_END) * (1.0 + (std::f64::consts::PI * t).cos()) / 2.0;
        let round = 0.0015;
        let e = (a.abs() - (l - round)).max(0.0);
        base + h - round + (round * round - e * e).max(0.0).sqrt()
    };
    let steps = 32;
    let mut profile: Vec<(f64, f64)> = (0..=steps).map(|k| -l + 2.0 * l * k as f64 / steps as f64).map(|a| (a, top(a))).collect();
    profile.extend([(l, base), (DAMPER_SLOT, base), (DAMPER_SLOT, base + DAMPER_HOLE), (-DAMPER_SLOT, base + DAMPER_HOLE), (-DAMPER_SLOT, base), (-l, base)]);
    let w = half - 0.0005;
    b.extrude(&profile, (-w, w), &|s, a, h| at(a, s, h), wood);
    (half, base + DAMPER_HOLE)
}


/// A key's wippen at `cx` across and what it carries, turned about its
/// pivot by `angle`, its jack escaped if the key is `down`; the let-off
/// button and drop screw over it.
fn wippen(b: &mut Builder, c: &Ctx, g: &Geo, cx: f64, down: bool, angle: f64, shift: f64) {
    let mark = b.mark();
    let (wt, wb) = (g.wippen_top, g.wippen_top - 0.011);
    let (wy, jy) = (g.wippen.0, g.jack);
    // The jack, its toe under the let-off button; escaped once the key is
    // down.
    let jb = wt + 0.002;
    let jack_c = c.pick(Part::Repetition, ACTION_WOOD);
    let jm = b.mark();
    b.piece(&[(jy - 0.018, jb), (jy + 0.003, jb), (jy + 0.003, g.knuckle), (jy - 0.003, g.knuckle - 0.0015), (jy - 0.003, jb + 0.008), (jy - 0.016, jb + 0.0065)], cx, 0.007, jack_c);
    b.turn(jm, (jy, jb + 0.003), if down { -0.12 } else { 0.0 }, 0.0);
    // The wippen: its pivot in front, its heel on the capstan, the
    // repetition lever on its post, under the knuckle beside the jack.
    let (cy, py) = (g.cap, g.post);
    let (wz, heel) = (g.wippen.1, g.heel);
    let body = [
        (wy - 0.005, wz + 0.004),
        (wy + 0.006, wz + 0.005),
        (py - 0.008, wt),
        (jy + 0.013, wt),
        (jy + 0.016, wt - 0.003),
        (jy + 0.016, wb + 0.003),
        (jy + 0.012, wb),
        (cy + 0.013, wb),
        (cy + 0.008, heel),
        (cy - 0.008, heel),
        (cy - 0.015, wb),
        (wy + 0.006, wz - 0.005),
        (wy - 0.005, wz - 0.004),
    ];
    b.piece(&body, cx, 0.0105, ACTION_WOOD);
    b.block((cy - 0.008, cy + 0.008), (heel - 0.003, heel), cx, 0.0105, CLOTH);
    // The repetition lever on its post, its front end over the drop
    // screw, its back under the knuckle beside the jack.
    let rl = g.knuckle - 0.0045;
    b.piece(&[(py - 0.007, wt), (py + 0.004, wt), (py + 0.0025, rl - 0.002), (py - 0.0035, rl - 0.002)], cx, 0.008, ACTION_WOOD);
    let rep = c.pick(Part::Repetition, SHANK_WOOD);
    b.piece(&[(py - 0.013, rl - 0.0035), (jy + 0.009, rl - 0.004), (jy + 0.012, rl - 0.001), (jy + 0.012, rl + 0.004), (py - 0.009, rl + 0.004), (py - 0.014, rl + 0.001)], cx, 0.006, rep);
    // Its spring, a wire from the post to under the lever.
    b.wire(f32s([cx + 0.0035, py + 0.004, wt + 0.006]), f32s([cx + 0.0035, jy - 0.01, rl - 0.004]), rgba(STEEL, 1.0), 0.0008);
    b.turn(mark, g.wippen, angle, shift);
    // The let-off button over the jack's toe, the drop screw over the
    // repetition lever's front.
    b.block((jy - 0.021, jy - 0.012), (jb + 0.009, jb + 0.0125), cx + shift, 0.007, CLOTH);
    let at = |z: f64| f32s([cx + shift, py - 0.011, z]);
    b.rod(at(rl + 0.006), at(g.pivot.1 - 0.012), (0.0016, 0.0016), BRASS);
    b.rod(at(rl + 0.0055), at(rl + 0.0085), (0.003, 0.003), CLOTH);
}

/// The key frame under the keys, the action's rails across and the
/// brackets holding them.
fn rails(b: &mut Builder, c: &Ctx, g: &Geo, shift: f64) {
    let (lo, hi) = (c.across(21).min(-c.span.1) - 0.01, c.across(108).max(-c.span.0) + 0.01);
    let (x, w) = ((lo + hi) / 2.0 + shift, hi - lo);
    let bed = c.kz - KEY_FRAME;
    let frame = c.pick(Part::KeybedKnock, KEY_WOOD.gamma_multiply(0.85));
    // The key frame: front rail, balance rail, back rail.
    b.block((c.key_front + 0.008, c.key_front + 0.03), (bed, c.kz - 0.002), x, w, frame);
    b.block((c.key_front + 0.008, c.key_front + 0.03), (c.kz - 0.002, c.kz), x, w, CLOTH);
    b.block((g.balance.0 - 0.014, g.balance.0 + 0.014), (bed, c.kz), x, w, frame);
    b.block((g.yh - 0.02, g.yh + 0.012), (bed, c.kz - 0.002), x, w, frame);
    b.block((g.yh - 0.02, g.yh + 0.012), (c.kz - 0.002, c.kz), x, w, CLOTH);
    // The wippen rail, the let-off rail and its buttons, the hammers'
    // flange rail, the hammer rest rail (one per section: the bass's
    // shanks rest lower), the damper rail.
    let rail = c.pick(Part::ActionKnock, RAIL);
    let (wy, wz) = g.wippen;
    b.block((wy - 0.026, wy - 0.006), (wz - 0.013, wz + 0.011), x, w, rail);
    let jb = g.wippen_top + 0.002;
    b.block((g.jack - 0.032, g.jack - 0.011), (jb + 0.0125, jb + 0.0235), x, w, rail);
    let (py, pz) = g.pivot;
    b.block((py - 0.032, py - 0.01), (pz - 0.012, pz + 0.008), x, w, rail);
    let (hy, hz) = g.hinge;
    b.block((hy + 0.004, hy + 0.014), (hz - 0.012, hz + 0.012), x, w, rail);
    let rest_at = py + 0.085;
    let mut m = 21;
    while m <= 108 {
        let bass = m < c.sb.main_bridge_from;
        let last = (m..=108).take_while(|&n| (n < c.sb.main_bridge_from) == bass).last().unwrap_or(m);
        let (x0, x1) = (c.across(m) - c.pitch(m) / 2.0 + 0.001, c.across(last) + c.pitch(last) / 2.0 - 0.001);
        let (x0, x1) = (x0.min(x1), x0.max(x1));
        let lowest = (m..=last).map(|n| hammer(c, g, n).rest).fold(f64::MAX, f64::min);
        let top = pz + (rest_at - py) * lowest.tan() - SHANK_R as f64;
        b.block((rest_at - 0.007, rest_at + 0.007), (top - 0.003, top), (x0 + x1) / 2.0 + shift, x1 - x0, CLOTH);
        b.block((rest_at - 0.007, rest_at + 0.007), (top - 0.016, top - 0.003), (x0 + x1) / 2.0 + shift, x1 - x0, rail);
        m = last + 1;
    }
    // The brackets, between keys: at both ends, at the bass's break and
    // through the treble.
    let breaks = [20.5, c.sb.main_bridge_from as f64 - 0.5, 68.5, 88.5, 108.5];
    // A truss: a foot on the balance rail, struts up to the wippen rail,
    // the let-off rail, the flange rail and the rest rail.
    let rest_top = pz + (rest_at - py) * hammer(c, g, 108).rest.tan() - SHANK_R as f64 - 0.019;
    let nodes = [
        (g.balance.0, c.kz + 0.004),
        (wy - 0.016, wz - 0.004),
        (g.jack - 0.021, jb + 0.012),
        (py - 0.021, pz - 0.006),
        (rest_at, rest_top),
    ];
    let struts = [(0, 1, 0.011), (1, 2, 0.008), (2, 3, 0.008), (1, 3, 0.006), (3, 4, 0.006)];
    for at in breaks {
        let across = |m: f64| {
            let k = m.clamp(21.0, 108.0);
            c.across(k as u8) + (m - k) * c.pitch(k as u8)
        };
        let x = (across(at - 0.5) + across(at + 0.5)) / 2.0 + shift;
        let col = c.pick(Part::ActionKnock, BRACKET);
        b.block((g.balance.0 - 0.02, g.balance.0 + 0.02), (c.kz, c.kz + 0.006), x, 0.012, col);
        for &(i, j, t) in &struts {
            let ((y0, z0), (y1, z1)) = (nodes[i], nodes[j]);
            let l = (y1 - y0).hypot(z1 - z0);
            let (ny, nz) = (-(z1 - z0) / l * t / 2.0, (y1 - y0) / l * t / 2.0);
            b.piece(&[(y0 - ny, z0 - nz), (y1 - ny, z1 - nz), (y1 + ny, z1 + nz), (y0 + ny, z0 + nz)], x, 0.003, col);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_swept_profile_faces_outwards_grand_or_upright() {
        for vertical in [false, true] {
            let place = Place { vertical, top: 1.0 };
            let mut b = Builder::new(place);
            let at = |a: f64, s: f64, h: f64| place.at(0.3 * a - 0.8 * s, 0.8 * a + 0.3 * s, h);
            let (section, _) = felt_section(1, 0.001, false);
            b.extrude(&section, (-0.01, 0.01), &at, Color32::WHITE);
            for t in b.s.tris.chunks(3) {
                // Every face seen from outside: its normal away from a point
                // well inside the felt, above the groove.
                let c = scale(add(add(t[0].pos, t[1].pos), t[2].pos), 1.0 / 3.0);
                assert!(dot(t[0].normal, sub(c, at(0.0, 0.0, 0.0065))) > 0.0, "vertical {vertical}: {t:?}");
            }
        }
    }

    #[test]
    fn a_black_key_faces_outwards_and_rounds_its_edges() {
        let mut b = Builder::new(Place { vertical: false, top: 0.0 });
        b.loft(&black_cap((0.0, 0.011), (0.0, 0.093), 0.0), EBONY);
        // A point well inside the cap, low and behind its sloping front.
        let inside = [0.0055, 0.05, 0.004];
        let mut slanted = 0;
        for t in b.s.tris.chunks(3) {
            let c = scale(add(add(t[0].pos, t[1].pos), t[2].pos), 1.0 / 3.0);
            for v in t {
                assert!(dot(v.normal, sub(c, inside)) > 0.0, "{t:?}");
                // Rounded: normals between up and the sides.
                let n = v.normal;
                if n[2] > 0.2 && n[0].abs() > 0.2 {
                    slanted += 1;
                }
            }
        }
        assert!(slanted > 50, "{slanted}");
        // As wide as the key at its foot, narrower at its top.
        let (lo, hi) = b.bounds();
        assert!((hi[0] - lo[0] - 0.011).abs() < 1e-6 && (hi[2] - BLACK_H as f32).abs() < 1e-6);
        let top = b.s.tris.iter().filter(|v| v.pos[2] > BLACK_H as f32 - 1e-6).map(|v| v.pos[0]);
        let (a, z) = top.fold((f32::MAX, f32::MIN), |(a, z), x| (a.min(x), z.max(x)));
        assert!(z - a < 0.011 - 2.0 * (BLACK_TAPER + 0.9 * BLACK_FILLET) as f32, "{a} {z}");
    }

    #[test]
    fn the_damper_felts_rest_on_their_strings_without_cutting_them() {
        // Distance from q to the segment a–b.
        let seg = |q: (f64, f64), a: (f64, f64), b: (f64, f64)| {
            let (ex, ey) = (b.0 - a.0, b.1 - a.1);
            let t = (((q.0 - a.0) * ex + (q.1 - a.1) * ey) / (ex * ex + ey * ey)).clamp(0.0, 1.0);
            (q.0 - a.0 - t * ex).hypot(q.1 - a.1 - t * ey)
        };
        for strings in 1..=3 {
            for r in [0.0004, 0.0008, 0.0015, 0.0025] {
                for flat in [false, true] {
                    let (p, half) = felt_section(strings, r, flat);
                    let r = if strings == 1 { r.min(0.003) } else if flat { r } else { r.min(0.4 * UNISON_GAP) };
                    assert!(p.iter().all(|q| q.0.abs() <= half + 1e-12 && q.1 <= DAMPER_FELT_TOP), "{p:?}");
                    for s in 0..strings {
                        let o = ((s as f64 - (strings as f64 - 1.0) / 2.0) * UNISON_GAP, 0.0);
                        let inside = (0..p.len()).filter(|&k| {
                            let (a, b) = (p[k], p[(k + 1) % p.len()]);
                            (a.1 > o.1) != (b.1 > o.1) && o.0 < a.0 + (o.1 - a.1) / (b.1 - a.1) * (b.0 - a.0)
                        });
                        assert!(inside.count() % 2 == 0, "{strings} strings, r {r}, flat {flat}: string {s} in the felt");
                        let gap = (0..p.len()).map(|k| seg(o, p[k], p[(k + 1) % p.len()])).fold(f64::MAX, f64::min);
                        assert!((gap - r).abs() < 1e-9, "{strings} strings, r {r}, flat {flat}: string {s} {gap} from the felt");
                    }
                }
            }
        }
    }

    #[test]
    fn the_hammers_reach_their_strings_and_clear_the_board_and_pin_block() {
        let model = PianoModel::default();
        let sb = model.soundboard.map(|s| s.resolved()).unwrap_or_default();
        let hx = hammer_x(&model, &sb);
        let ends: Vec<_> = (21..=108u8).map(|m| string_ends(&model, &sb, hx, m)).collect();
        let (lit, level) = (|_: Part| false, |_: u8| 0.0);
        let c = Ctx {
            model: &model,
            sb: &sb,
            ends: &ends,
            hx,
            obs: None,
            key: 60,
            lit: &lit,
            level: &level,
            span: keyboard_span(&sb),
            key_front: -0.44,
            key_back: -0.29,
            kz: -0.175,
            vertical: false,
            inside: true,
        };
        let g = Geo::new(&c);
        let pin_block = (hx - BLOCK_CLEAR, PIN_BLOCK_TOP - PIN_BLOCK_T);
        for m in 21..=108u8 {
            let h = hammer(&c, &g, m);
            let z = ends[(m - 21) as usize].2;
            let crown = |a: f64| rot((g.pivot.0 + SHANK_L, g.pivot.1 + h.h), g.pivot, a);
            let (y, cz) = crown(h.blow);
            assert!((y - hx).abs() < 1e-9 && (cz - z).abs() < 1e-9, "key {m}'s crown at ({y}, {cz}) on the blow");
            assert!((crown(h.rest).1 - REST_CROWN).abs() < 1e-9 && (crown(h.checked).1 - (z - CHECKED)).abs() < 1e-9, "key {m}");
            assert!(h.rest < h.checked && h.checked < h.blow, "key {m}: {} {} {}", h.rest, h.checked, h.blow);
            // The head's outline as it swings from rest to the strings:
            // never through the board (its edge at y = 0, 12 mm thick) nor
            // into the belly rail; the shank's top never in the pin block.
            let (d, base) = (h.d / 2.0, h.h - h.felt - 0.6 * h.d);
            let mut outline = vec![(-d, -0.0075), (d + TAIL, -0.0075), (d + TAIL, -0.002)];
            outline.extend(arch(d, base, h.h - h.felt * 0.5, h.h));
            for k in 0..=40 {
                let a = h.rest + (h.blow - h.rest) * k as f64 / 40.0;
                for &(u, v) in &outline {
                    let (y, z) = rot((g.pivot.0 + SHANK_L + u, g.pivot.1 + v), g.pivot, a);
                    assert!(y < 0.0 || !(-0.012..0.0).contains(&z), "key {m}'s head in the board at ({y}, {z}), angle {a}");
                    assert!(y < BELLY_AT, "key {m}'s head in the belly rail at ({y}, {z})");
                }
                for j in 0..=20 {
                    let (y, z) = rot((g.pivot.0 + SHANK_L * j as f64 / 20.0, g.pivot.1 + SHANK_R as f64), g.pivot, a);
                    assert!(y > pin_block.0 || z < pin_block.1, "key {m}'s shank in the pin block at ({y}, {z})");
                }
            }
            // The key's tail and backcheck in front of the belly rail.
            let checked = tail_at(&g, &h, h.checked);
            assert!(checked.0 + 0.02 + 0.012 < BELLY_AT, "key {m}'s tail at {}", checked.0);
            // The damper's underlever and wire behind the backcheck (its
            // rod and its head, leaning back), the key's tail under them,
            // the hinge in front of the belly rail.
            if m <= model.damper.highest_note {
                let backcheck = checked.0 + 0.008;
                let wire = damper_wire(&c, m);
                assert!(wire - 0.004 > backcheck + 0.003 + 0.0011, "key {m}'s underlever on its backcheck's rod: wire at {wire}, backcheck at {backcheck}");
                assert!(wire - WIRE_R as f64 > backcheck + 0.006 + 0.06 * 0.026, "key {m}'s wire through its backcheck: {wire}, {backcheck}");
                assert!(g.hinge.0 + 0.014 < BELLY_AT && g.hinge.0 > wire + 0.015, "key {m}'s hinge at {}", g.hinge.0);
            }
        }
    }
}
