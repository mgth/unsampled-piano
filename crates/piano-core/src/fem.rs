//! The soundboard as it is built, by finite elements: a spruce plate whose
//! thickness varies, ribs under it across the grain, bridges over it, held
//! along the outline (docs/real_board.md).
//!
//! Every point of the board is a stack of orthotropic layers, reduced to its
//! bending stiffness about its own neutral plane (classical laminate theory:
//! D* = D − B·A⁻¹·B), its transverse shear stiffness, its surface mass and
//! rotary inertia. A Reissner–Mindlin plate: the shear across the grain
//! matters, spruce being 300 times softer there than along it; without it
//! the board has half its modes at 6 kHz. Elements: MITC4 rectangles (w, θx,
//! θy at each corner, assumed shear, no locking) on a regular grid, each
//! integrated on 4 × 4 points, so that a rib lies where it does.
//!
//! Solver: the modes below a frequency, by spectrum slicing. Each slice's
//! number of modes comes from the inertia of K − λ·M (Sylvester), and a
//! shift-invert Lanczos with full reorthogonalisation runs in its middle until
//! it has them all. A slice per thread; the result is kept on disk, by a hash
//! of what it depends on.

use crate::soundboard::{Outline, SoundboardParams};
use std::collections::HashMap;
use std::f64::consts::TAU;
use std::sync::{Arc, Mutex, OnceLock};

/// Rib entries of [`BoardStructure::ribs`].
pub const RIB_POINTS: usize = 20;
/// Points of the main bridge's thickness curve.
pub const MAIN_BRIDGE_POINTS: usize = 8;
/// Points of the bass bridge's thickness curve.
pub const BASS_BRIDGE_POINTS: usize = 3;

/// What the board is made of and where, for [`BoardFem`]. The default is the
/// Steinway D of Chabassier's thesis and RT-425, switched off.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
pub struct BoardStructure {
    /// The board's modes below `below` come from this structure.
    pub enabled: bool,
    /// How the rim holds it: clamped (no rotation), or simply supported, the
    /// transverse movement alone held at the nodes along the outline.
    pub clamped: bool,
    /// Grid step (m) and highest own mode (Hz).
    pub step: f64,
    pub below: f64,
    /// The plate's grain, degrees from the board's length towards +y.
    pub grain_angle: f64,
    /// Plate and ribs: density (kg/m³), moduli along and across the grain,
    /// in-plane shear modulus (Pa), Poisson's ratio, transverse shear moduli
    /// along the grain and across it (Pa).
    pub spruce_density: f64,
    pub spruce_el: f64,
    pub spruce_er: f64,
    pub spruce_g: f64,
    pub spruce_nu: f64,
    pub spruce_gl: f64,
    pub spruce_gr: f64,
    /// The bridges, the same.
    pub bridge_density: f64,
    pub bridge_el: f64,
    pub bridge_er: f64,
    pub bridge_g: f64,
    pub bridge_nu: f64,
    pub bridge_gl: f64,
    pub bridge_gr: f64,
    /// Plate thickness (m): `plate_center` away from the rim, falling towards
    /// `plate_edge` at it over a distance `plate_taper` (exponentially).
    pub plate_center: f64,
    pub plate_edge: f64,
    pub plate_taper: f64,
    /// Each rib: its two ends in normalised plate coordinates (as the
    /// outline), its thickness plate included at the middle and at the ends
    /// (m), parabolic between them. From the treble to the bass; unused
    /// entries have no length.
    pub ribs: [(f64, f64, f64, f64, f64, f64); RIB_POINTS],
    /// Width of the first rib and of the last (m), linear between them.
    pub rib_width_treble: f64,
    pub rib_width_bass: f64,
    /// Bridge thickness, plate included: pairs of MIDI note and metres,
    /// ascending, on the main and on the bass bridge; its width (m). The
    /// bridge's fibres follow its curve.
    pub main_bridge_thickness: [(f64, f64); MAIN_BRIDGE_POINTS],
    pub bass_bridge_thickness: [(f64, f64); BASS_BRIDGE_POINTS],
    pub bridge_width: f64,
}

impl Default for BoardStructure {
    fn default() -> Self {
        Self::STEINWAY_D
    }
}

impl BoardStructure {
    /// RT-425's woods and its figure 2's thicknesses, read in metres off the
    /// figure (docs/real_board.md): its 16 ribs, 2–2.6 cm wide (the thesis
    /// measured them on the piano and counted 17).
    pub const STEINWAY_D: Self = Self {
        enabled: false,
        clamped: true,
        step: 0.01,
        below: 3000.0,
        grain_angle: 40.0,
        spruce_density: 390.0,
        spruce_el: 11.6e9,
        spruce_er: 0.9e9,
        spruce_g: 0.75e9,
        spruce_nu: 0.37,
        spruce_gl: 0.72e9,
        spruce_gr: 0.039e9,
        bridge_density: 750.0,
        bridge_el: 13.7e9,
        bridge_er: 2.24e9,
        bridge_g: 1.61e9,
        bridge_nu: 0.3,
        bridge_gl: 1.06e9,
        bridge_gr: 0.46e9,
        plate_center: 0.009,
        plate_edge: 0.0068,
        plate_taper: 0.1,
        ribs: [
            (0.0765, 0.2213, 0.1312, 0.1098, 0.0154, 0.0125),
            (0.0812, 0.346, 0.166, 0.1732, 0.0183, 0.013),
            (0.0821, 0.4835, 0.1942, 0.2549, 0.0201, 0.0154),
            (0.0961, 0.5893, 0.2246, 0.3272, 0.0233, 0.0182),
            (0.1185, 0.6782, 0.2634, 0.3826, 0.0271, 0.021),
            (0.1595, 0.7338, 0.3126, 0.4216, 0.0299, 0.0234),
            (0.2083, 0.7782, 0.3669, 0.4548, 0.0327, 0.0253),
            (0.2606, 0.8252, 0.4356, 0.4683, 0.0342, 0.0249),
            (0.3202, 0.8622, 0.5143, 0.4662, 0.0335, 0.0234),
            (0.3789, 0.896, 0.5813, 0.4833, 0.0339, 0.0216),
            (0.4504, 0.9086, 0.6582, 0.4849, 0.033, 0.0203),
            (0.5344, 0.9101, 0.7367, 0.4975, 0.0309, 0.0198),
            (0.6148, 0.9095, 0.8143, 0.5024, 0.0293, 0.0184),
            (0.6957, 0.9125, 0.8871, 0.5222, 0.0263, 0.0168),
            (0.777, 0.9099, 0.9301, 0.5976, 0.0226, 0.0161),
            (0.8579, 0.9129, 0.9454, 0.7344, 0.0174, 0.0154),
            (0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            (0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            (0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            (0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
        ],
        rib_width_treble: 0.02,
        rib_width_bass: 0.026,
        main_bridge_thickness: [(41.0, 0.055), (49.0, 0.067), (53.0, 0.07), (61.0, 0.064), (69.0, 0.055), (81.0, 0.042), (97.0, 0.032), (108.0, 0.029)],
        bass_bridge_thickness: [(21.0, 0.052), (32.0, 0.058), (40.0, 0.06)],
        bridge_width: 0.03,
    };
}

/// One layer of a stack: density, in-plane stiffness Q̄ (11, 12, 16, 22, 26,
/// 66), transverse shear (xx, xy, yy), bottom, top.
#[derive(Clone, Copy)]
struct Layer {
    density: f64,
    q: [f64; 6],
    shear: [f64; 3],
    z0: f64,
    z1: f64,
}

/// A stack reduced to the plate's terms.
#[derive(Clone, Copy)]
struct Section {
    /// D11, D12, D16, D22, D26, D66.
    d: [f64; 6],
    /// κ²·G·h: xx, xy, yy.
    shear: [f64; 3],
    rho: f64,
    /// Rotary inertia about the stack's centre of mass.
    inertia: f64,
}

/// Mindlin's shear correction.
const KAPPA2: f64 = 5.0 / 6.0;

fn reduce(layers: &[Layer]) -> Section {
    let (mut a, mut b, mut d, mut rho, mut moment, mut shear) = ([0.0; 6], [0.0; 6], [0.0; 6], 0.0, 0.0, [0.0; 3]);
    for l in layers {
        let (z0, z1) = (l.z0, l.z1);
        rho += l.density * (z1 - z0);
        moment += l.density * (z1 * z1 - z0 * z0) / 2.0;
        for i in 0..6 {
            a[i] += l.q[i] * (z1 - z0);
            b[i] += l.q[i] * (z1 * z1 - z0 * z0) / 2.0;
            d[i] += l.q[i] * (z1 * z1 * z1 - z0 * z0 * z0) / 3.0;
        }
        for (sum, g) in shear.iter_mut().zip(l.shear) {
            *sum += KAPPA2 * g * (z1 - z0);
        }
    }
    let centre = moment / rho;
    let inertia = layers.iter().map(|l| l.density * ((l.z1 - centre).powi(3) - (l.z0 - centre).powi(3)) / 3.0).sum();
    let m = |v: &[f64; 6]| [[v[0], v[1], v[2]], [v[1], v[3], v[4]], [v[2], v[4], v[5]]];
    let (ma, mb) = (m(&a), m(&b));
    let (c00, c01, c02) = (ma[1][1] * ma[2][2] - ma[1][2] * ma[2][1], ma[1][2] * ma[2][0] - ma[1][0] * ma[2][2], ma[1][0] * ma[2][1] - ma[1][1] * ma[2][0]);
    let det = ma[0][0] * c00 + ma[0][1] * c01 + ma[0][2] * c02;
    let inv = [
        [c00 / det, (ma[0][2] * ma[2][1] - ma[0][1] * ma[2][2]) / det, (ma[0][1] * ma[1][2] - ma[0][2] * ma[1][1]) / det],
        [c01 / det, (ma[0][0] * ma[2][2] - ma[0][2] * ma[2][0]) / det, (ma[0][2] * ma[1][0] - ma[0][0] * ma[1][2]) / det],
        [c02 / det, (ma[0][1] * ma[2][0] - ma[0][0] * ma[2][1]) / det, (ma[0][0] * ma[1][1] - ma[0][1] * ma[1][0]) / det],
    ];
    let mut bab = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            for k in 0..3 {
                for l in 0..3 {
                    bab[i][j] += mb[i][k] * inv[k][l] * mb[l][j];
                }
            }
        }
    }
    Section { d: [d[0] - bab[0][0], d[1] - bab[0][1], d[2] - bab[0][2], d[3] - bab[1][1], d[4] - bab[1][2], d[5] - bab[2][2]], shear, rho, inertia }
}

/// Transverse shear of a wood whose fibres point along (c, s).
fn rotated_shear(gl: f64, gr: f64, c: f64, s: f64) -> [f64; 3] {
    [c * c * gl + s * s * gr, c * s * (gl - gr), s * s * gl + c * c * gr]
}

/// Plane-stress stiffness of an orthotropic wood whose fibres point along
/// (c, s), in the board's axes (Q̄11, Q̄12, Q̄16, Q̄22, Q̄26, Q̄66).
fn rotated(el: f64, er: f64, g: f64, nu: f64, c: f64, s: f64) -> [f64; 6] {
    let k = 1.0 / (1.0 - nu * nu * er / el);
    let (q11, q22, q12, q66) = (k * el, k * er, k * nu * er, g);
    let (c2, s2) = (c * c, s * s);
    let (c4, s4, sc2) = (c2 * c2, s2 * s2, s2 * c2);
    [
        q11 * c4 + 2.0 * (q12 + 2.0 * q66) * sc2 + q22 * s4,
        (q11 + q22 - 4.0 * q66) * sc2 + q12 * (s4 + c4),
        (q11 - q12 - 2.0 * q66) * s * c * c2 + (q12 - q22 + 2.0 * q66) * s * s2 * c,
        q11 * s4 + 2.0 * (q12 + 2.0 * q66) * sc2 + q22 * c4,
        (q11 - q12 - 2.0 * q66) * s * s2 * c + (q12 - q22 + 2.0 * q66) * s * c * c2,
        (q11 + q22 - 2.0 * q12 - 2.0 * q66) * sc2 + q66 * (s4 + c4),
    ]
}

/// Linear interpolation on (key, value) points, flat outside them.
fn curve(points: &[(f64, f64)], key: f64) -> f64 {
    let mut last = points[0];
    if key <= last.0 {
        return last.1;
    }
    for &q in &points[1..] {
        if key <= q.0 {
            return last.1 + (q.1 - last.1) * (key - last.0) / (q.0 - last.0).max(1e-9);
        }
        last = q;
    }
    last.1
}

struct Rib {
    p: (f64, f64),
    dir: (f64, f64),
    len: f64,
    mid: f64,
    end: f64,
    half_width: f64,
}

struct BridgePiece {
    p: (f64, f64),
    dir: (f64, f64),
    len: f64,
    thickness: (f64, f64),
}

/// The board's materials, point by point.
struct Structure {
    s: BoardStructure,
    outline: Option<Outline>,
    a: f64,
    b: f64,
    ribs: Vec<Rib>,
    bridges: Vec<BridgePiece>,
    grain: [f64; 6],
    grain_shear: [f64; 3],
    rib_q: Vec<([f64; 6], [f64; 3])>,
}

impl Structure {
    fn new(p: &SoundboardParams) -> Self {
        let s = p.structure;
        let (ca, sa) = (s.grain_angle.to_radians().cos(), s.grain_angle.to_radians().sin());
        let used: Vec<_> = s.ribs.iter().filter(|r| r.0 != r.2 || r.1 != r.3).collect();
        let width = |i: usize| s.rib_width_treble + (s.rib_width_bass - s.rib_width_treble) * i as f64 / (used.len().max(2) - 1) as f64;
        let ribs: Vec<Rib> = used
            .iter()
            .enumerate()
            .map(|(i, r)| {
                let (x0, y0, x1, y1) = (r.0 * p.a, r.1 * p.b, r.2 * p.a, r.3 * p.b);
                let len = (x1 - x0).hypot(y1 - y0);
                Rib { p: (x0, y0), dir: ((x1 - x0) / len, (y1 - y0) / len), len, mid: r.4, end: r.5, half_width: 0.5 * width(i) }
            })
            .collect();
        let rib_q = ribs.iter().map(|r| (rotated(s.spruce_el, s.spruce_er, s.spruce_g, s.spruce_nu, r.dir.0, r.dir.1), rotated_shear(s.spruce_gl, s.spruce_gr, r.dir.0, r.dir.1))).collect();
        let main = p.main_bridge_from as f64;
        let mut bridges = Vec::new();
        for (lo, hi, on_main) in [(main, 108.0, true), (21.0, main - 1.0, false)] {
            let n = (((hi - lo) * 2.0) as usize).max(1);
            let at = |i: usize| lo + (hi - lo) * i as f64 / n as f64;
            for i in 0..n {
                let (q0, q1) = (p.bridge_point_at(at(i), on_main), p.bridge_point_at(at(i + 1), on_main));
                let len = (q1.0 - q0.0).hypot(q1.1 - q0.1);
                if len <= 0.0 {
                    continue;
                }
                let th = |k: f64| if on_main { curve(&s.main_bridge_thickness, k) } else { curve(&s.bass_bridge_thickness, k) };
                bridges.push(BridgePiece { p: q0, dir: ((q1.0 - q0.0) / len, (q1.1 - q0.1) / len), len, thickness: (th(at(i)), th(at(i + 1))) });
            }
        }
        Self { s, outline: Outline::new(p), a: p.a, b: p.b, ribs, bridges, grain: rotated(s.spruce_el, s.spruce_er, s.spruce_g, s.spruce_nu, ca, sa), grain_shear: rotated_shear(s.spruce_gl, s.spruce_gr, ca, sa), rib_q }
    }

    fn plate_thickness(&self, x: f64, y: f64) -> f64 {
        let d = self.outline.as_ref().map_or_else(|| x.min(self.a - x).min(y).min(self.b - y).max(0.0), |o| o.edge_distance(x, y));
        self.s.plate_center - (self.s.plate_center - self.s.plate_edge) * (-d / self.s.plate_taper.max(1e-6)).exp()
    }

    /// The stack at (x, y). `near`: the bridge pieces worth looking at for
    /// this element.
    fn section(&self, x: f64, y: f64, near: &[usize]) -> Section {
        let s = &self.s;
        let t = self.plate_thickness(x, y);
        let empty = Layer { density: 0.0, q: [0.0; 6], shear: [0.0; 3], z0: 0.0, z1: 0.0 };
        let mut layers = [Layer { density: s.spruce_density, q: self.grain, shear: self.grain_shear, z0: 0.0, z1: t }, empty, empty];
        let mut n = 1;
        for (r, (q, sh)) in self.ribs.iter().zip(&self.rib_q) {
            let (dx, dy) = (x - r.p.0, y - r.p.1);
            let along = dx * r.dir.0 + dy * r.dir.1;
            if along < 0.0 || along > r.len || (dy * r.dir.0 - dx * r.dir.1).abs() > r.half_width {
                continue;
            }
            let u = 2.0 * along / r.len - 1.0;
            let total = r.mid - (r.mid - r.end) * u * u;
            layers[n] = Layer { density: s.spruce_density, q: *q, shear: *sh, z0: t - total.max(t), z1: 0.0 };
            n += 1;
            break;
        }
        let half = 0.5 * s.bridge_width;
        let mut bridge: Option<(f64, usize, f64)> = None;
        for &i in near {
            let b = &self.bridges[i];
            let (dx, dy) = (x - b.p.0, y - b.p.1);
            let along = (dx * b.dir.0 + dy * b.dir.1).clamp(0.0, b.len);
            let dist = (dx - along * b.dir.0).hypot(dy - along * b.dir.1);
            if dist <= half && bridge.is_none_or(|(d, _, _)| dist < d) {
                bridge = Some((dist, i, along / b.len));
            }
        }
        if let Some((_, i, u)) = bridge {
            let b = &self.bridges[i];
            let total = b.thickness.0 + (b.thickness.1 - b.thickness.0) * u;
            let q = rotated(s.bridge_el, s.bridge_er, s.bridge_g, s.bridge_nu, b.dir.0, b.dir.1);
            layers[n] = Layer { density: s.bridge_density, q, shear: rotated_shear(s.bridge_gl, s.bridge_gr, b.dir.0, b.dir.1), z0: t, z1: total.max(t) };
            n += 1;
        }
        reduce(&layers[..n])
    }
}

/// Symmetric band matrix: `at[i·(band + 1) + d]` is the entry (i, i + d).
#[derive(Clone)]
struct Band {
    n: usize,
    band: usize,
    at: Vec<f64>,
}

impl Band {
    fn new(n: usize, band: usize) -> Self {
        Self { n, band, at: vec![0.0; n * (band + 1)] }
    }

    fn add(&mut self, i: usize, j: usize, v: f64) {
        let (i, j) = if i <= j { (i, j) } else { (j, i) };
        self.at[i * (self.band + 1) + j - i] += v;
    }

    fn mul(&self, x: &[f64], y: &mut [f64]) {
        y.iter_mut().for_each(|v| *v = 0.0);
        let w = self.band + 1;
        for i in 0..self.n {
            let row = &self.at[i * w..(i + 1) * w];
            y[i] += row[0] * x[i];
            let top = (self.n - i).min(w);
            let (mut acc, xi) = (0.0, x[i]);
            for d in 1..top {
                acc += row[d] * x[i + d];
                y[i + d] += row[d] * xi;
            }
            y[i] += acc;
        }
    }
}

/// LDLᵀ of K − σ·M, without pivoting: what a modal slice needs. Band solvers
/// of this kind stay stable as long as σ is no eigenvalue; the Lanczos
/// residuals check it.
struct Ldl {
    n: usize,
    band: usize,
    at: Vec<f64>,
}

impl Ldl {
    fn new(k: &Band, m: &Band, sigma: f64) -> Self {
        let (n, band) = (k.n, k.band);
        let w = band + 1;
        let mut at: Vec<f64> = k.at.iter().zip(&m.at).map(|(a, b)| a - sigma * b).collect();
        let mut row = vec![0.0; w];
        for i in 0..n {
            let top = (n - i).min(w);
            row[..top].copy_from_slice(&at[i * w..i * w + top]);
            let d = if row[0].abs() < 1e-300 { 1e-300 } else { row[0] };
            for j in 1..top {
                let l = row[j] / d;
                if l != 0.0 {
                    let target = &mut at[(i + j) * w..(i + j) * w + top - j];
                    for (t, r) in target.iter_mut().zip(&row[j..top]) {
                        *t -= l * r;
                    }
                }
                at[i * w + j] = l;
            }
            at[i * w] = d;
        }
        Self { n, band, at }
    }

    /// Eigenvalues of the pencil below σ.
    fn below(&self) -> usize {
        (0..self.n).filter(|i| self.at[i * (self.band + 1)] < 0.0).count()
    }

    fn solve(&self, x: &mut [f64]) {
        let w = self.band + 1;
        for i in 0..self.n {
            let top = (self.n - i).min(w);
            let (xi, row) = (x[i], &self.at[i * w..i * w + top]);
            for (t, l) in x[i + 1..i + top].iter_mut().zip(&row[1..]) {
                *t -= l * xi;
            }
        }
        for i in (0..self.n).rev() {
            let top = (self.n - i).min(w);
            let row = &self.at[i * w..i * w + top];
            let sum: f64 = x[i + 1..i + top].iter().zip(&row[1..]).map(|(a, b)| a * b).sum();
            x[i] = x[i] / row[0] - sum;
        }
    }
}

fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

/// The `count` eigenpairs of K·x = λ·M·x in [lo, hi) (M-normalised), by
/// shift-invert Lanczos at their middle.
fn slice(k: &Band, m: &Band, lo: f64, hi: f64, count: usize, seed: u64) -> Vec<(f64, Vec<f64>)> {
    if count == 0 {
        return Vec::new();
    }
    let n = k.n;
    let sigma = 0.5 * (lo.sqrt() + hi.sqrt());
    let ldl = Ldl::new(k, m, sigma * sigma);
    let sigma = sigma * sigma;
    let mut state = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
    let mut random = |v: &mut [f64]| {
        for x in v.iter_mut() {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            *x = (state >> 11) as f64 / (1u64 << 53) as f64 - 0.5;
        }
    };
    let mut v = vec![0.0; n];
    random(&mut v);
    let mut mv = vec![0.0; n];
    m.mul(&v, &mut mv);
    let norm = dot(&v, &mv).sqrt();
    v.iter_mut().chain(mv.iter_mut()).for_each(|x| *x /= norm);
    let (mut basis, mut mbasis) = (vec![v], vec![mv]);
    let (mut alpha, mut beta): (Vec<f64>, Vec<f64>) = (Vec::new(), Vec::new());
    let mut w = vec![0.0; n];
    let mut mw = vec![0.0; n];
    let limit = (count * 12 + 400).min(n);
    let mut next_check = 2 * count + 20;
    loop {
        let j = basis.len() - 1;
        w.copy_from_slice(&mbasis[j]);
        ldl.solve(&mut w);
        let a = dot(&w, &mbasis[j]);
        alpha.push(a);
        for _ in 0..2 {
            for (q, mq) in basis.iter().zip(&mbasis) {
                let c = dot(&w, mq);
                for (t, x) in w.iter_mut().zip(q) {
                    *t -= c * x;
                }
            }
        }
        m.mul(&w, &mut mw);
        let b = dot(&w, &mw).max(0.0).sqrt();
        let steps = alpha.len();
        if steps >= next_check || steps >= limit {
            next_check = steps + 20;
            let mut t = vec![0.0; steps * steps];
            for i in 0..steps {
                t[i * steps + i] = alpha[i];
                if i + 1 < steps {
                    t[i * steps + i + 1] = beta[i];
                    t[(i + 1) * steps + i] = beta[i];
                }
            }
            let (theta, s) = crate::eigen::symmetric_ql(&t, steps);
            let good: Vec<usize> = (0..steps)
                .filter(|&i| {
                    let lambda = sigma + 1.0 / theta[i];
                    theta[i] != 0.0 && lambda >= lo && lambda < hi && (b * s[i][steps - 1]).abs() <= 1e-7 * theta[i].abs()
                })
                .collect();
            if good.len() >= count || steps >= limit {
                return good
                    .into_iter()
                    .map(|i| {
                        let mut x = vec![0.0; n];
                        for (q, c) in basis.iter().zip(&s[i]) {
                            for (t, y) in x.iter_mut().zip(q) {
                                *t += c * y;
                            }
                        }
                        (sigma + 1.0 / theta[i], x)
                    })
                    .collect();
            }
        }
        beta.push(b);
        if b > 1e-12 * a.abs() {
            basis.push(w.iter().map(|x| x / b).collect());
            mbasis.push(mw.iter().map(|x| x / b).collect());
        } else {
            // An invariant subspace: on with a vector orthogonal to it.
            random(&mut w);
            for _ in 0..2 {
                for (q, mq) in basis.iter().zip(&mbasis) {
                    let c = dot(&w, mq);
                    for (t, x) in w.iter_mut().zip(q) {
                        *t -= c * x;
                    }
                }
            }
            m.mul(&w, &mut mw);
            let r = dot(&w, &mw).sqrt();
            *beta.last_mut().unwrap() = 0.0;
            basis.push(w.iter().map(|x| x / r).collect());
            mbasis.push(mw.iter().map(|x| x / r).collect());
        }
    }
}

/// 4-point Gauss–Legendre on [0, 1]: (ξ, weight).
const GAUSS: [(f64, f64); 4] = [
    (0.069_431_844_202_973_71, 0.173_927_422_568_726_93),
    (0.330_009_478_207_571_87, 0.326_072_577_431_273_07),
    (0.669_990_521_792_428_1, 0.326_072_577_431_273_07),
    (0.930_568_155_797_026_3, 0.173_927_422_568_726_93),
];

/// The board's modes on a regular grid: the displacement of each at every
/// node that moves, and its reactions on what holds it.
pub struct BoardFem {
    pub nx: usize,
    pub ny: usize,
    pub hx: f64,
    pub hy: f64,
    /// Index of each grid node among those whose w moves, (nx + 1)·(ny + 1),
    /// or −1.
    node: Vec<i32>,
    pub omega: Vec<f64>,
    n_free: usize,
    /// omega.len() × n_free.
    shapes: Vec<f32>,
    /// Clamped nodes that carry a force, in metres, and for each mode the
    /// force on them per unit modal displacement, omega.len() × supports.
    pub supports: Vec<(f64, f64)>,
    reactions: Vec<f32>,
    /// Mass of the board (kg) and its area (m²).
    pub mass: f64,
    pub area: f64,
}

impl BoardFem {
    /// Mass-normalised mode shape at (x, y).
    pub fn shape(&self, mode: usize, x: f64, y: f64) -> f64 {
        let (u, v) = (x / self.hx, y / self.hy);
        if u < 0.0 || v < 0.0 || u >= self.nx as f64 || v >= self.ny as f64 {
            return 0.0;
        }
        let (i, j) = (u as usize, v as usize);
        let (fx, fy) = (u - i as f64, v - j as f64);
        let table = &self.shapes[mode * self.n_free..(mode + 1) * self.n_free];
        let at = |a: usize, b: usize| -> f64 {
            let n = self.node[a * (self.ny + 1) + b];
            if n < 0 { 0.0 } else { table[n as usize] as f64 }
        };
        (1.0 - fx) * ((1.0 - fy) * at(i, j) + fy * at(i, j + 1)) + fx * ((1.0 - fy) * at(i + 1, j) + fy * at(i + 1, j + 1))
    }

    /// The forces with which a mode pushes on the rim, per unit modal
    /// displacement: (x, y, N).
    pub fn forces(&self, mode: usize) -> Vec<(f64, f64, f64)> {
        let n = self.supports.len();
        self.supports.iter().zip(&self.reactions[mode * n..(mode + 1) * n]).map(|(&(x, y), &f)| (x, y, f as f64)).collect()
    }

    fn build(p: &SoundboardParams) -> Self {
        let st = Structure::new(p);
        let s = p.structure;
        let (nx, ny) = ((p.a / s.step).ceil() as usize, (p.b / s.step).ceil() as usize);
        let (hx, hy) = (p.a / nx as f64, p.b / ny as f64);
        let active: Vec<bool> = (0..nx * ny)
            .map(|c| {
                let (x, y) = (((c / ny) as f64 + 0.5) * hx, ((c % ny) as f64 + 0.5) * hy);
                st.outline.as_ref().is_none_or(|o| o.contains(x, y))
            })
            .collect();
        let is_active = |i: i64, j: i64| i >= 0 && j >= 0 && (i as usize) < nx && (j as usize) < ny && active[i as usize * ny + j as usize];
        let node_id = |i: usize, j: usize| i * (ny + 1) + j;
        // Each node's index among those whose w moves; its dofs (w, θx, θy),
        // −1 when held.
        let mut node = vec![-1i32; (nx + 1) * (ny + 1)];
        let mut held = vec![false; (nx + 1) * (ny + 1)];
        let mut dof = vec![-1i32; 3 * (nx + 1) * (ny + 1)];
        let (mut n_free, mut n) = (0, 0usize);
        for i in 0..=nx {
            for j in 0..=ny {
                let (a, b) = (i as i64, j as i64);
                let around = [is_active(a - 1, b - 1), is_active(a, b - 1), is_active(a - 1, b), is_active(a, b)];
                let (any, all) = (around.iter().any(|&x| x), around.iter().all(|&x| x));
                let id = node_id(i, j);
                held[id] = any && !all;
                if !any || (!all && s.clamped) {
                    continue;
                }
                if all {
                    node[id] = n_free;
                    n_free += 1;
                }
                for d in usize::from(!all)..3 {
                    dof[3 * id + d] = n as i32;
                    n += 1;
                }
            }
        }
        // Corners in the order (0,0), (1,0), (0,1), (1,1).
        let corners = |c: usize| -> [usize; 4] {
            let (i, j) = (c / ny, c % ny);
            [node_id(i, j), node_id(i + 1, j), node_id(i, j + 1), node_id(i + 1, j + 1)]
        };
        let elements: Vec<usize> = (0..nx * ny).filter(|&c| active[c]).collect();
        let global = |c: usize, l: usize| -> Option<usize> {
            let v = dof[3 * corners(c)[l / 3] + l % 3];
            (v >= 0).then_some(v as usize)
        };
        let mut band = 0;
        for &c in &elements {
            let used: Vec<usize> = (0..12).filter_map(|l| global(c, l)).collect();
            if let (Some(lo), Some(hi)) = (used.iter().min(), used.iter().max()) {
                band = band.max(hi - lo);
            }
        }
        // Tables at the integration points: curvatures (κxx, κyy, κxy) and
        // assumed shears (γx, γy) per dof, shape values per corner. MITC4: γx
        // tied at the middles of the edges along x, γy along y.
        let mut curv = Vec::with_capacity(16 * 12);
        let mut gamma = Vec::with_capacity(16 * 12);
        let mut value = Vec::with_capacity(16 * 4);
        for &(eta, _) in &GAUSS {
            for &(xi, _) in &GAUSS {
                let nx_ = [1.0 - xi, xi, 1.0 - xi, xi];
                let ny_ = [1.0 - eta, 1.0 - eta, eta, eta];
                let dx_ = [-1.0 / hx, 1.0 / hx, -1.0 / hx, 1.0 / hx];
                let dy_ = [-1.0 / hy, -1.0 / hy, 1.0 / hy, 1.0 / hy];
                for k in 0..4 {
                    let (nd_x, nd_y) = (dx_[k] * ny_[k], nx_[k] * dy_[k]);
                    // Shear tied: γx depends on η alone through its edge, γy on ξ.
                    let (gx_w, gx_t) = (dx_[k] * ny_[k], 0.5 * ny_[k]);
                    let (gy_w, gy_t) = (nx_[k] * dy_[k], 0.5 * nx_[k]);
                    value.push(nx_[k] * ny_[k]);
                    curv.push([0.0, 0.0, 0.0]);
                    gamma.push([gx_w, gy_w]);
                    curv.push([nd_x, 0.0, nd_y]);
                    gamma.push([gx_t, 0.0]);
                    curv.push([0.0, nd_y, nd_x]);
                    gamma.push([0.0, gy_t]);
                }
            }
        }
        // Element matrices, 12 × 12 each.
        let element = |c: usize| -> (Vec<f64>, Vec<f64>, f64) {
            let (x0, y0) = ((c / ny) as f64 * hx, (c % ny) as f64 * hy);
            let reach = hx.hypot(hy) + s.bridge_width;
            let (cx, cy) = (x0 + 0.5 * hx, y0 + 0.5 * hy);
            let near: Vec<usize> = st
                .bridges
                .iter()
                .enumerate()
                .filter(|(_, b)| {
                    let along = ((cx - b.p.0) * b.dir.0 + (cy - b.p.1) * b.dir.1).clamp(0.0, b.len);
                    (cx - b.p.0 - along * b.dir.0).hypot(cy - b.p.1 - along * b.dir.1) < reach
                })
                .map(|(i, _)| i)
                .collect();
            let (mut ke, mut me, mut mass) = (vec![0.0; 144], vec![0.0; 144], 0.0);
            for (g2, &(eta, w2)) in GAUSS.iter().enumerate() {
                for (g1, &(xi, w1)) in GAUSS.iter().enumerate() {
                    let g = g2 * 4 + g1;
                    let sec = st.section(x0 + xi * hx, y0 + eta * hy, &near);
                    let (d, sh) = (sec.d, sec.shear);
                    let w = w1 * w2 * hx * hy;
                    mass += w * sec.rho;
                    let (cq, gq, vq) = (&curv[g * 12..(g + 1) * 12], &gamma[g * 12..(g + 1) * 12], &value[g * 4..(g + 1) * 4]);
                    for i in 0..12 {
                        let (b, e) = (cq[i], gq[i]);
                        let db = [d[0] * b[0] + d[1] * b[1] + d[2] * b[2], d[1] * b[0] + d[3] * b[1] + d[4] * b[2], d[2] * b[0] + d[4] * b[1] + d[5] * b[2]];
                        let se = [sh[0] * e[0] + sh[1] * e[1], sh[1] * e[0] + sh[2] * e[1]];
                        for j in 0..12 {
                            let (bj, ej) = (cq[j], gq[j]);
                            ke[i * 12 + j] += w * (db[0] * bj[0] + db[1] * bj[1] + db[2] * bj[2] + se[0] * ej[0] + se[1] * ej[1]);
                            if i % 3 == j % 3 {
                                let m = if i % 3 == 0 { sec.rho } else { sec.inertia };
                                me[i * 12 + j] += w * m * vq[i / 3] * vq[j / 3];
                            }
                        }
                    }
                }
            }
            (ke, me, mass)
        };
        let matrices = crate::par::map(elements.len(), |e| element(elements[e]));
        let (mut k, mut m) = (Band::new(n, band), Band::new(n, band));
        let total_mass: f64 = matrices.iter().map(|x| x.2).sum();
        for (&c, (ke, me, _)) in elements.iter().zip(&matrices) {
            for a in 0..12 {
                let Some(ga) = global(c, a) else { continue };
                for b in 0..12 {
                    let Some(gb) = global(c, b) else { continue };
                    if ga <= gb {
                        k.add(ga, gb, ke[a * 12 + b]);
                        m.add(ga, gb, me[a * 12 + b]);
                    }
                }
            }
        }
        // Slices of 400 Hz up to the corner.
        let top = TAU * s.below;
        let slices = ((s.below / 400.0).ceil() as usize).max(1);
        let edges: Vec<f64> = (0..=slices).map(|i| (top * i as f64 / slices as f64).powi(2)).collect();
        let below: Vec<usize> = crate::par::map(edges.len(), |i| if i == 0 { 0 } else { Ldl::new(&k, &m, edges[i]).below() });
        let pairs = crate::par::map(slices, |i| slice(&k, &m, edges[i], edges[i + 1], below[i + 1] - below[i], i as u64 + 1));
        let mut found: Vec<(f64, Vec<f64>)> = pairs.into_iter().flatten().collect();
        found.sort_by(|x, y| x.0.total_cmp(&y.0));
        let count = found.len();
        // Reactions: F = −K_cf·φ at the held w dofs (the plate's equation
        // there, inertia of the held dofs left out).
        let mut supports = Vec::new();
        let mut support_of = HashMap::new();
        for i in 0..=nx {
            for j in 0..=ny {
                if held[node_id(i, j)] {
                    support_of.insert(node_id(i, j), supports.len());
                    supports.push((i as f64 * hx, j as f64 * hy));
                }
            }
        }
        let mut reactions = vec![0.0f32; count * supports.len()];
        let per_mode: Vec<Vec<f32>> = crate::par::map(count, |mode| {
            let phi = &found[mode].1;
            let mut out = vec![0.0f64; supports.len()];
            for (&c, (ke, _, _)) in elements.iter().zip(&matrices) {
                for (corner, id) in corners(c).iter().enumerate() {
                    let Some(&sup) = support_of.get(id) else { continue };
                    let row = corner * 3;
                    for l in 0..12 {
                        if let Some(g) = global(c, l) {
                            out[sup] -= ke[row * 12 + l] * phi[g];
                        }
                    }
                }
            }
            out.into_iter().map(|x| x as f32).collect()
        });
        for (mode, r) in per_mode.iter().enumerate() {
            reactions[mode * supports.len()..(mode + 1) * supports.len()].copy_from_slice(r);
        }
        let omega = found.iter().map(|x| x.0.max(0.0).sqrt()).collect();
        let mut shapes = vec![0.0f32; count * n_free as usize];
        for (mode, (_, v)) in found.iter().enumerate() {
            let table = &mut shapes[mode * n_free as usize..(mode + 1) * n_free as usize];
            for id in 0..node.len() {
                if node[id] >= 0 {
                    table[node[id] as usize] = v[dof[3 * id] as usize] as f32;
                }
            }
        }
        let area = elements.len() as f64 * hx * hy;
        Self { nx, ny, hx, hy, node, omega, n_free: n_free as usize, shapes, supports, reactions, mass: total_mass, area }
    }

    fn write(&self, path: &std::path::Path) -> std::io::Result<()> {
        let mut out: Vec<u8> = Vec::with_capacity(self.shapes.len() * 4 + self.reactions.len() * 4 + self.node.len() * 4 + 1024);
        for v in [self.nx, self.ny, self.n_free, self.omega.len(), self.supports.len()] {
            out.extend((v as u64).to_le_bytes());
        }
        for v in [self.hx, self.hy, self.mass, self.area] {
            out.extend(v.to_le_bytes());
        }
        self.omega.iter().for_each(|v| out.extend(v.to_le_bytes()));
        self.supports.iter().for_each(|v| {
            out.extend(v.0.to_le_bytes());
            out.extend(v.1.to_le_bytes());
        });
        self.node.iter().for_each(|v| out.extend(v.to_le_bytes()));
        self.shapes.iter().for_each(|v| out.extend(v.to_le_bytes()));
        self.reactions.iter().for_each(|v| out.extend(v.to_le_bytes()));
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        // Whole or nothing: another process may be reading the same file.
        let tmp = path.with_extension(format!("tmp{}", std::process::id()));
        std::fs::write(&tmp, out)?;
        std::fs::rename(&tmp, path)
    }

    fn read(path: &std::path::Path) -> Option<Self> {
        let data = std::fs::read(path).ok()?;
        let mut at = 0;
        let mut take = |n: usize| -> Option<&[u8]> {
            let s = data.get(at..at + n)?;
            at += n;
            Some(s)
        };
        let mut head = [0usize; 5];
        for h in &mut head {
            *h = u64::from_le_bytes(take(8)?.try_into().ok()?) as usize;
        }
        let [nx, ny, n_free, count, nsup] = head;
        let float = |s: &[u8]| s.as_chunks::<8>().0.iter().map(|c| f64::from_le_bytes(*c)).collect::<Vec<f64>>();
        let single = |s: &[u8]| s.as_chunks::<4>().0.iter().map(|c| f32::from_le_bytes(*c)).collect::<Vec<f32>>();
        let scalars = float(take(32)?);
        let omega = float(take(count * 8)?);
        let sup = float(take(nsup * 16)?);
        let node: Vec<i32> = take((nx + 1) * (ny + 1) * 4)?.as_chunks::<4>().0.iter().map(|c| i32::from_le_bytes(*c)).collect();
        let shapes = single(take(count * n_free * 4)?);
        let reactions = single(take(count * nsup * 4)?);
        Some(Self {
            nx,
            ny,
            hx: scalars[0],
            hy: scalars[1],
            node,
            omega,
            n_free,
            shapes,
            supports: sup.chunks_exact(2).map(|c| (c[0], c[1])).collect(),
            reactions,
            mass: scalars[2],
            area: scalars[3],
        })
    }
}

/// What the modes depend on, as a hash (the crown only moves their
/// frequencies afterwards, in [`crate::soundboard::Soundboard::new`]).
fn key(p: &SoundboardParams) -> u64 {
    const VERSION: u32 = 2;
    let text = format!(
        "{VERSION} {:?} {} {} {:?} {:?} {:?} {} {} {}",
        BoardStructure { enabled: true, ..p.structure },
        p.a,
        p.b,
        p.outline,
        p.treble_bridge,
        p.bass_bridge,
        p.bass_bridge_bow,
        p.main_string_slant,
        p.main_bridge_from
    );
    text.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ b as u64).wrapping_mul(0x0000_0100_0000_01b3))
}

fn cache_dir() -> Option<std::path::PathBuf> {
    let var = |name: &str| std::env::var_os(name).filter(|v| !v.is_empty()).map(std::path::PathBuf::from);
    var("XDG_CACHE_HOME").or_else(|| var("HOME").map(|h| h.join(".cache"))).or_else(|| var("LOCALAPPDATA")).map(|d| d.join("unsampled-piano").join("boards"))
}

/// The modes of this structure: from memory, from the disk, or computed.
pub fn board(p: &SoundboardParams) -> Arc<BoardFem> {
    static MEMORY: OnceLock<Mutex<HashMap<u64, Arc<BoardFem>>>> = OnceLock::new();
    let k = key(p);
    let memory = MEMORY.get_or_init(Default::default);
    // Held while computing: two builds of the same board wait for one.
    let mut held = memory.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(b) = held.get(&k) {
        return b.clone();
    }
    let path = cache_dir().map(|d| d.join(format!("{k:016x}.bin")));
    let board = path.as_deref().and_then(BoardFem::read).unwrap_or_else(|| {
        let b = BoardFem::build(p);
        if let Some(path) = &path {
            let _ = b.write(path);
        }
        b
    });
    let board = Arc::new(board);
    held.insert(k, board.clone());
    board
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A square isotropic plate: no ribs, no bridges, no outline.
    fn plain(side: f64, step: f64, below: f64) -> SoundboardParams {
        let mut s = BoardStructure::STEINWAY_D;
        s.enabled = true;
        s.step = step;
        s.below = below;
        (s.spruce_el, s.spruce_er, s.spruce_g, s.spruce_nu) = (1e10, 1e10, 1e10 / 2.6, 0.3);
        (s.spruce_gl, s.spruce_gr) = (1e10 / 2.6, 1e10 / 2.6);
        s.spruce_density = 400.0;
        (s.plate_center, s.plate_edge) = (0.01, 0.01);
        s.ribs = [(0.0, 0.0, 0.0, 0.0, 0.0, 0.0); RIB_POINTS];
        s.bridge_width = 0.0;
        SoundboardParams { a: side, b: side, outline: [(0.0, 0.0); crate::soundboard::OUTLINE_POINTS], structure: s, ..SoundboardParams::GRAND_2M }
    }

    #[test]
    fn a_clamped_square_has_its_known_modes() {
        let p = plain(1.0, 0.0125, 700.0);
        let fem = BoardFem::build(&p);
        let d: f64 = 1e10 * 1e-6 / 12.0 / (1.0 - 0.09);
        let unit = (d / (400.0 * 0.01)).sqrt();
        // Leissa: λ = ωa²√(ρh/D) for the clamped square.
        for (w, known) in fem.omega.iter().zip([35.985, 73.394, 73.394, 108.22, 131.58, 132.20]) {
            assert!((w / unit / known - 1.0).abs() < 0.005, "{} against {known}", w / unit);
        }
    }

    #[test]
    fn the_slices_miss_no_mode() {
        let p = plain(1.0, 0.025, 1200.0);
        let fem = BoardFem::build(&p);
        // The dense pencil, on the same matrices through a smaller board: the
        // count of modes below the top against a separate inertia is what
        // each slice checks; here their frequencies must also be sorted and
        // each slice full.
        let d: f64 = 1e10 * 1e-6 / 12.0 / (1.0 - 0.09);
        let density = 1.0 / (4.0 * std::f64::consts::PI) * (4.0 / d).sqrt();
        let expected = density * fem.omega.last().unwrap();
        assert!(fem.omega.len() as f64 > 0.6 * expected && (fem.omega.len() as f64) < 1.2 * expected, "{} against {expected}", fem.omega.len());
        assert!(fem.omega.windows(2).all(|w| w[0] <= w[1]));
    }

    #[test]
    fn the_reactions_carry_the_plates_inertia() {
        let p = plain(1.0, 0.025, 400.0);
        let fem = BoardFem::build(&p);
        // Σ F = ω²·∫ρh·Φ, on the plate's own grid.
        for mode in 0..fem.omega.len().min(6) {
            let total: f64 = fem.forces(mode).iter().map(|f| f.2).sum();
            let (mut integral, n) = (0.0, 200);
            for i in 0..n {
                for j in 0..n {
                    integral += fem.shape(mode, (i as f64 + 0.5) / n as f64, (j as f64 + 0.5) / n as f64) / (n * n) as f64;
                }
            }
            let inertia = fem.omega[mode].powi(2) * 4.0 * integral;
            let scale = fem.omega[mode].powi(2) * 4.0f64.sqrt();
            assert!((total - inertia).abs() < 0.02 * scale, "mode {mode}: {total} against {inertia}");
        }
    }

    #[test]
    fn a_rib_stiffens_along_itself() {
        // A strip of 2 cm spruce on 9 mm, along x: stiff along x, as the rib's
        // own bending about the stack's neutral plane says.
        let s = BoardStructure::STEINWAY_D;
        let q = rotated(s.spruce_el, s.spruce_er, s.spruce_g, s.spruce_nu, 1.0, 0.0);
        let plate = rotated(s.spruce_el, s.spruce_er, s.spruce_g, s.spruce_nu, 0.0, 1.0);
        let layer = |q, z0, z1| Layer { density: 390.0, q, shear: [1.0; 3], z0, z1 };
        let sec = reduce(&[layer(plate, 0.0, 0.009), layer(q, -0.02, 0.0)]);
        let alone = s.spruce_el * 0.02f64.powi(3) / 12.0;
        assert!(sec.d[0] > alone && sec.d[0] < 2.0 * alone, "{} against {alone}", sec.d[0]);
        assert!((sec.rho - 390.0 * 0.029).abs() < 1e-9);
        // One layer: the plate's own inertia about its middle.
        let one = reduce(&[layer(q, 0.0, 0.01)]);
        assert!((one.inertia - 390.0 * 0.01f64.powi(3) / 12.0).abs() < 1e-12);
    }

    #[test]
    fn the_shape_is_mass_normalised() {
        let p = plain(1.0, 0.025, 400.0);
        let fem = BoardFem::build(&p);
        let n = 200;
        for mode in 0..fem.omega.len().min(4) {
            let mut sum = 0.0;
            for i in 0..n {
                for j in 0..n {
                    sum += fem.shape(mode, (i as f64 + 0.5) / n as f64, (j as f64 + 0.5) / n as f64).powi(2) / (n * n) as f64;
                }
            }
            assert!((sum * 4.0 - 1.0).abs() < 0.01, "{sum}");
        }
    }

    #[test]
    fn the_cache_gives_back_what_was_written() {
        let p = plain(1.0, 0.1, 300.0);
        let fem = BoardFem::build(&p);
        let dir = std::env::temp_dir().join(format!("fem-cache-{}", std::process::id()));
        let path = dir.join("b.bin");
        fem.write(&path).unwrap();
        let back = BoardFem::read(&path).unwrap();
        std::fs::remove_dir_all(&dir).ok();
        assert_eq!(fem.omega, back.omega);
        assert_eq!(fem.shape(0, 0.31, 0.47), back.shape(0, 0.31, 0.47));
        assert_eq!(fem.forces(0), back.forces(0));
    }
}
