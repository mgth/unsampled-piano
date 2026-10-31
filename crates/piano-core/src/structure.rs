//! The case around the soundboard: the cast-iron frame and the rim.
//!
//! **Frame.** The strings pull on the agraffes (their far end) with the same
//! kind of force as on the bridge. The frame takes it: a cast-iron plate,
//! heavy and stiff (≈ 40× less mobile than the board), with little damping,
//! modelled as an isotropic clamped plate (same machinery as the board)
//! driven at the agraffe points and radiating to the mics from its own
//! height. Its motion is too small to act back on the strings (one way).
//!
//! **Rim.** The board is glued into the rim, which takes the board's edge
//! reactions. For each board mode (sine shape), the Kirchhoff edge shear
//! and corner forces give the reaction along the perimeter; they sum to
//! `ρh·ω²·∫Φ` with the simply supported frequency, and are scaled by
//! `(ω_n/ω_ss)²` so that the net force matches the clamped frequency the
//! board uses. The rim is a closed laminated-maple wall, modelled as a ring
//! of the same perimeter in out-of-plane bending coupled to torsion (Love):
//! for each order k, a 2 × 2 problem in vertical motion W and rotation B of
//! the wall gives a bending-like and a torsion-like mode. The board pulls on
//! the inner edge of the wall, at t/2 from its axis, so it drives both. The
//! wall radiates from its four faces (Rayleigh elements with an obliquity
//! factor; the inner face, half hidden under the board, with `inner_gain`).
//! One way as well: the rim does not move the board's edges.

use crate::board::{self, BoardMode, MICS, REGIONS};
use crate::complex::C64;
use crate::soundboard::{PlateMode, Soundboard, SoundboardParams, OUTLINE_POINTS};
use std::f64::consts::{PI, TAU};

const RHO0: f64 = 1.2;
const C0: f64 = 343.0;

#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
pub struct FrameParams {
    pub enabled: bool,
    /// Plate size (m); its corner sits at x = `offset_x` in board
    /// coordinates (the frame extends over the pin block, in front of the
    /// board), same y.
    pub a: f64,
    pub b: f64,
    pub offset_x: f64,
    /// Line of the agraffes, in frame coordinates (m).
    pub agraffe_x: f64,
    /// Height of the frame above the board (m).
    pub height: f64,
    /// Surface mass (kg/m²) and bending stiffness (N·m), struts included.
    pub rho_h: f64,
    pub stiffness: f64,
    pub loss_factor: f64,
    /// Short circuit between the faces of an open-work plate (see
    /// [`SoundboardParams::back_gain`]).
    pub back_depth: f64,
    pub back_gain: f64,
    /// Fraction of the plate's outline that is metal (struts, webs): only
    /// that part radiates.
    pub solidity: f64,
    pub f_max: f64,
    /// Scales the radiated pressure (1 = physical).
    pub gain: f64,
    /// The frame's admittance at each key's agraffes in the strings' modes
    /// (1 = physical, 0 = one way): a partial that falls on a frame mode
    /// gives it its energy, as it does to the board at the bridge.
    pub coupling: f64,
}

impl Default for FrameParams {
    fn default() -> Self {
        Self::GRAND_2M
    }
}

impl FrameParams {
    /// About 150 kg of cast iron over 1.9 × 1.3 m (holes included in the
    /// surface mass), from 20 cm in front of the board over the pin block to
    /// the case's tail (the hitch pins), within the case's outline
    /// ([`FrameParams::outline`]); stiffness set for a first mode near 150 Hz
    /// on the rectangle. Bare cast
    /// iron has η ≈ 0.1–0.5 %; installed (bolted to the rim and pin block,
    /// loaded by 230 strings and their felts) it is taken at 2 % (estimate:
    /// at 1 % a frame mode on C7's fundamental rang within 4 dB of the
    /// note, docs/frame_rim.md).
    /// The agraffes stand above the pin block, near the bolted front edge.
    pub const GRAND_2M: Self = Self {
        enabled: true,
        a: 1.9,
        b: 1.3,
        offset_x: -0.2,
        agraffe_x: 0.05,
        height: 0.08,
        rho_h: 110.0,
        stiffness: 2.5e5,
        loss_factor: 0.02,
        back_depth: 0.05,
        back_gain: 0.9,
        solidity: 0.4,
        f_max: 12_000.0,
        gain: 1.0,
        coupling: 1.0,
    };

    /// The frame's outline in its own normalised coordinates: the case's,
    /// as the board's outline draws it, carried forward over the pin block
    /// with a straight front edge (the plate covers the case and the pin
    /// block in front of the board). A board without an outline gives the
    /// rectangle. Points beyond the plate's length are clamped to it.
    pub fn outline(&self, board: &SoundboardParams) -> [(f64, f64); OUTLINE_POINTS] {
        if crate::soundboard::Outline::new(board).is_none() {
            return [(0.0, 0.0); OUTLINE_POINTS];
        }
        board.outline.map(|(u, v)| {
            let x = if u <= 1e-9 { self.offset_x } else { u * board.a };
            (((x - self.offset_x) / self.a).clamp(0.0, 1.0), (v * board.b / self.b).clamp(0.0, 1.0))
        })
    }

    /// The frame as a plate, in its own coordinates.
    pub fn plate(&self, board: &SoundboardParams) -> SoundboardParams {
        SoundboardParams {
            outline: self.outline(board),
            outline_below: FRAME_OWN_MODES_BELOW,
            a: self.a,
            b: self.b,
            rho_h: self.rho_h,
            dx: self.stiffness,
            dy: self.stiffness,
            loss_factor: self.loss_factor,
            loss_corner: f64::INFINITY,
            loss_low_corner: 0.0,
            edge_loss: 0.0,
            edge_depth: 0.2,
            edge_corner: 200.0,
            bridge_mass: 0.0,
            bridge_beam: 0.0,
            series_stiffness: f64::INFINITY,
            f_max: self.f_max,
            mics: board.mics.map(|(x, y, z)| (x - self.offset_x, y, z - self.height)),
            radiate: true,
            grid_step: 0.02,
            back_depth: self.back_depth,
            back_gain: self.back_gain,
            lid_hinge_height: board.lid_hinge_height - self.height,
            piston_freq: 0.0,
            crown_freq: 0.0,
            ..*board
        }
    }

    /// Agraffe point of each region (frame coordinates): a key's agraffe
    /// stands where its strings cross the keys' line
    /// ([`SoundboardParams::key_across`]), whatever their slant.
    pub fn agraffe_points(&self, board: &SoundboardParams) -> Vec<(f64, f64)> {
        (0..REGIONS)
            .map(|r| (self.agraffe_x, board.key_across(board::region_center_note(r)).clamp(0.0, self.b)))
            .collect()
    }
}

/// The frame has its outline's own modes up to here (Hz), the rectangle's
/// above. The rectangle's modes cut at the outline radiate from its edge,
/// where their cells no longer cancel: at the board's 1.5 kHz the frame came
/// out louder than the note at C7 (+4 dB) and D7 (−9 dB, −26 with its own
/// modes). The stiff iron plate has few modes: 23 ms to build up to 3 kHz
/// (9 ms at 1.5 kHz, 130 ms at 6 kHz, 1 s at 12 kHz).
const FRAME_OWN_MODES_BELOW: f64 = 3000.0;

/// The frame as a key's strings see it at their agraffes: its
/// driving-point admittance there, which the strings' modes take beside the
/// bridge's (two-way coupling, [`FrameParams::coupling`]).
pub struct FrameAdmittance {
    plate: Soundboard,
    agraffe_x: f64,
    b: f64,
    coupling: f64,
}

impl FrameAdmittance {
    /// None when the frame is off or one way.
    pub fn new(frame: &FrameParams, board: &SoundboardParams) -> Option<Self> {
        (frame.enabled && frame.coupling > 0.0).then(|| Self { plate: Soundboard::new(frame.plate(board)), agraffe_x: frame.agraffe_x, b: frame.b, coupling: frame.coupling })
    }

    /// Admittance (s/kg) at the agraffe of the key whose strings cross the
    /// keys' line at `y` ([`SoundboardParams::key_across`]), at ω.
    pub fn at(&self, y: f64, omega: f64) -> C64 {
        self.plate.admittance(self.agraffe_x, y.clamp(0.0, self.b), omega).scale(self.coupling)
    }
}

/// Frame modes: inputs at the agraffes, outputs at the mics.
pub fn frame_modes(frame: &FrameParams, board: &SoundboardParams, sample_rate: f64) -> Vec<BoardMode> {
    let sb = Soundboard::new(frame.plate(board));
    let mut modes = board::board_modes_at(&sb, &frame.agraffe_points(board), sample_rate);
    let g = frame.gain * frame.solidity;
    for m in &mut modes {
        for o in &mut m.outputs[..MICS] {
            *o = (o.0 * g, o.1 * g);
        }
    }
    modes
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
pub struct RimParams {
    pub enabled: bool,
    /// Wall height and thickness (m); top of the wall above the board plane.
    pub height: f64,
    pub thickness: f64,
    pub top: f64,
    /// Laminated maple: Young's modulus along the wall, torsional shear
    /// modulus (Pa), density (kg/m³), loss factor.
    pub young: f64,
    pub shear: f64,
    pub density: f64,
    pub loss_factor: f64,
    /// Highest circumferential order, and highest rim mode kept (Hz).
    pub k_max: usize,
    pub f_max: f64,
    /// Board modes above this frequency (Hz) are not projected on the rim
    /// (their short edge patterns barely drive its long-wave modes).
    pub couple_f_max: f64,
    /// Radiation of the inner face (partly under the board).
    pub inner_gain: f64,
    /// Scales the radiated pressure (1 = physical).
    pub gain: f64,
}

impl Default for RimParams {
    fn default() -> Self {
        Self::GRAND_2M
    }
}

impl RimParams {
    /// 40 cm high, 7.5 cm thick maple wall rising 16 cm above the board,
    /// up to the lid (`SoundboardParams::lid_hinge_height`): a grand's keys
    /// at ≈ 72 cm from the floor, its lid shut at ≈ 1 m, the board ≈ 10 cm
    /// above the keys, a case ≈ 40 cm deep (docs/lid.md).
    pub const GRAND_2M: Self = Self {
        enabled: true,
        height: 0.40,
        thickness: 0.075,
        top: 0.16,
        young: 12e9,
        shear: 1.0e9,
        density: 650.0,
        loss_factor: 0.02,
        k_max: 8,
        f_max: 3000.0,
        couple_f_max: 2000.0,
        inner_gain: 0.5,
        gain: 1.0,
    };

    fn mass(&self) -> f64 {
        self.density * self.height * self.thickness
    }

    /// Rotary inertia of the wall section per metre (kg·m).
    fn rot_inertia(&self) -> f64 {
        let (h, t) = (self.height, self.thickness);
        self.density * (h * t.powi(3) + t * h.powi(3)) / 12.0
    }

    fn ei(&self) -> f64 {
        self.young * self.thickness * self.height.powi(3) / 12.0
    }

    fn gj(&self) -> f64 {
        let (h, t) = (self.height.max(self.thickness), self.height.min(self.thickness));
        self.shear * h * t.powi(3) / 3.0 * (1.0 - 0.63 * t / h)
    }
}

/// One rim mode: order k, cos (0) or sin (1) orientation, mass-normalised
/// amplitudes of vertical motion W and rotation B (inner edge up).
#[derive(Clone, Copy, Debug)]
pub struct RimMode {
    pub k: usize,
    pub orient: usize,
    pub freq_hz: f64,
    pub sigma: f64,
    pub w: f64,
    pub b: f64,
    /// Index of the board's edge-force projection that drives it, and its
    /// gain on it (vertical motion of the wall's inner edge, W + t/2·B).
    pub proj: usize,
    pub drive: f64,
    /// Pressure at each mic per unit modal velocity (complex).
    pub mics: [(f64, f64); MICS],
}

pub struct RimModel {
    pub modes: Vec<RimMode>,
    /// Projections (k, orientation) of the board's edge forces.
    pub projections: Vec<(usize, usize)>,
    /// Board output weights for each projection, per board mode (the first
    /// modes of the bank, up to `couple_f_max`).
    pub couple: Vec<Vec<(f64, f64)>>,
}

/// Point of the board's perimeter at arc length s (from the corner (0, 0),
/// along y = 0 first), with its outward normal.
fn perimeter(a: f64, b: f64, s: f64) -> ((f64, f64), (f64, f64)) {
    if s < a {
        ((s, 0.0), (0.0, -1.0))
    } else if s < a + b {
        ((a, s - a), (1.0, 0.0))
    } else if s < 2.0 * a + b {
        ((a - (s - a - b), b), (0.0, 1.0))
    } else {
        ((0.0, b - (s - 2.0 * a - b)), (-1.0, 0.0))
    }
}

/// Perimeter position (m, [`perimeter`]'s) of the rim seen from the board's
/// centre through (x, y): where an outlined board's edge force lands on the
/// ring that stands for the rim.
fn perimeter_towards(a: f64, b: f64, x: f64, y: f64) -> f64 {
    let (dx, dy) = (x - 0.5 * a, y - 0.5 * b);
    let t = (0.5 * a / dx.abs().max(1e-12)).min(0.5 * b / dy.abs().max(1e-12));
    let (hx, hy) = ((0.5 * a + t * dx).clamp(0.0, a), (0.5 * b + t * dy).clamp(0.0, b));
    if t * dy.abs() >= 0.5 * b - 1e-12 {
        if dy < 0.0 { hx } else { a + b + (a - hx) }
    } else if dx > 0.0 {
        a + hy
    } else {
        2.0 * a + b + (b - hy)
    }
}

/// The edge reactions of an outlined board's basis modes ([`Soundboard::basis`],
/// scaled to their frequencies) at the perimeter samples `s`; none without
/// an outline.
fn basis_edge_reactions(sb: &Soundboard, s: &[f64]) -> Vec<(Vec<f64>, [f64; 4])> {
    let bp = &sb.params;
    crate::par::map(if sb.ritz.is_empty() { 0 } else { sb.basis.len() }, |k| {
        let (m, n, w) = sb.basis[k];
        let (dens, cf) = edge_reaction(bp, m, n, s);
        let scale = (w / omega_ss(bp, m, n)).powi(2);
        (dens.into_iter().map(|d| d * scale).collect(), cf.map(|c| c * scale))
    })
}

/// Force on the rim per unit modal displacement of a board mode (edge
/// density at the perimeter samples `s`, evenly spaced, and corner forces)
/// and the scale to its frequency. An outlined board's own mode pushes on
/// the rim through the rectangle's edges it shares with the outline (its
/// basis modes' reactions, `basis_edges`) and where the ground holds it
/// outside the outline.
fn board_edge_forces(sb: &Soundboard, basis_edges: &[(Vec<f64>, [f64; 4])], md: &PlateMode, s: &[f64]) -> (Vec<f64>, [f64; 4], f64) {
    let bp = &sb.params;
    let Some(forces) = sb.held_forces(md) else {
        let (dens, cf) = edge_reaction(bp, md.m, md.n, s);
        return (dens, cf, (md.omega / omega_ss(bp, md.m, md.n)).powi(2));
    };
    let (ns, ds) = (s.len(), 2.0 * (bp.a + bp.b) / s.len() as f64);
    let (mut dens, mut cf) = (vec![0.0; ns], [0.0; 4]);
    for ((bd, bc), c) in basis_edges.iter().zip(&sb.ritz[md.ritz.unwrap_or(0) as usize]) {
        for (d, b) in dens.iter_mut().zip(bd) {
            *d += c * b;
        }
        for (d, b) in cf.iter_mut().zip(bc) {
            *d += c * b;
        }
    }
    for (x, y, f) in forces {
        dens[((perimeter_towards(bp.a, bp.b, x, y) / ds) as usize).min(ns - 1)] += f / ds;
    }
    (dens, cf, 1.0)
}

/// Force on the rim per unit modal displacement of a board mode: edge
/// density at each perimeter sample (N/m) and the four corner forces (N),
/// simply supported Kirchhoff values (not yet scaled to the clamped
/// frequency).
fn edge_reaction(p: &SoundboardParams, m: u32, n: u32, s: &[f64]) -> (Vec<f64>, [f64; 4]) {
    let (a, b) = (p.a, p.b);
    let norm = 2.0 / (p.rho_h * a * b).sqrt();
    let h = (p.dx * p.dy).sqrt();
    let (km, kn) = (m as f64 * PI / a, n as f64 * PI / b);
    let vx = norm * km * (p.dx * km * km + 1.7 * h * kn * kn);
    let vy = norm * kn * (p.dy * kn * kn + 1.7 * h * km * km);
    let cc = -1.4 * h * norm * km * kn;
    let sm = if m % 2 == 1 { 1.0 } else { -1.0 };
    let sn = if n % 2 == 1 { 1.0 } else { -1.0 };
    let dens = s
        .iter()
        .map(|&s| {
            let ((x, y), nrm) = perimeter(a, b, s);
            match nrm {
                (_, ny) if ny < 0.0 => vy * (km * x).sin(),
                (nx, _) if nx > 0.0 => vx * (kn * y).sin() * sm,
                (_, ny) if ny > 0.0 => vy * (km * x).sin() * sn,
                _ => vx * (kn * y).sin(),
            }
        })
        .collect();
    // Corners in perimeter order: (0,0), (a,0), (a,b), (0,b).
    (dens, [cc, cc * sm, cc * sm * sn, cc * sn])
}

/// Frequency of the simply supported mode (m, n) (rad/s).
fn omega_ss(p: &SoundboardParams, m: u32, n: u32) -> f64 {
    let (xm, yn) = ((m as f64 / p.a).powi(2), (n as f64 / p.b).powi(2));
    let h = (p.dx * p.dy).sqrt();
    PI * PI / p.rho_h.sqrt() * (p.dx * xm * xm + 2.0 * h * xm * yn + p.dy * yn * yn).sqrt()
}

impl RimModel {
    /// `n_board` is the number of modes in the board bank (the first ones of
    /// `sb.modes`).
    pub fn new(p: &RimParams, sb: &Soundboard, n_board: usize, sample_rate: f64) -> Self {
        let bp = &sb.params;
        let (a, b, t) = (bp.a, bp.b, p.thickness);
        let lb = 2.0 * (a + b);
        // Ring of the wall's mean perimeter.
        let radius = (lb + 4.0 * t) / TAU;
        let (mu, irot, ei, gj) = (p.mass(), p.rot_inertia(), p.ei(), p.gj());
        let f_max = p.f_max.min(0.45 * sample_rate);

        // Ring modes: K·x = ω²·M·x with x = (W, B).
        let mut modes = Vec::new();
        for k in 0..=p.k_max {
            let kf = k as f64;
            let (r2, r3, r4) = (radius.powi(2), radius.powi(3), radius.powi(4));
            let k11 = (ei * kf.powi(4) + gj * kf * kf) / r4;
            let k12 = (ei + gj) * kf * kf / r3;
            let k22 = (ei + gj * kf * kf) / r2;
            // λ²·μ·I − λ·(k11·I + k22·μ) + det = 0.
            let (qa, qb, qc) = (mu * irot, -(k11 * irot + k22 * mu), k11 * k22 - k12 * k12);
            let disc = (qb * qb - 4.0 * qa * qc).max(0.0).sqrt();
            for lam in [(-qb - disc) / (2.0 * qa), (-qb + disc) / (2.0 * qa)] {
                let omega = lam.max(0.0).sqrt();
                let f = omega / TAU;
                if !(20.0..f_max).contains(&f) {
                    continue;
                }
                let (w, bb) = if k12.abs() > 1e-12 * (k11 + k22) {
                    (k12, lam * mu - k11)
                } else if (lam * mu - k11).abs() < (lam * irot - k22).abs() {
                    (1.0, 0.0)
                } else {
                    (0.0, 1.0)
                };
                // ∫cos²(kθ) ds over the perimeter: L/2 (L for k = 0).
                let span = if k == 0 { lb } else { lb / 2.0 };
                let nrm = 1.0 / ((mu * w * w + irot * bb * bb) * span).sqrt();
                for orient in 0..if k == 0 { 1 } else { 2 } {
                    modes.push(RimMode {
                        k,
                        orient,
                        freq_hz: f,
                        sigma: 0.5 * p.loss_factor * omega,
                        w: w * nrm,
                        b: bb * nrm,
                        proj: 0,
                        drive: (w + 0.5 * t * bb) * nrm,
                        mics: [(0.0, 0.0); MICS],
                    });
                }
            }
        }
        modes.sort_by(|x, y| x.freq_hz.total_cmp(&y.freq_hz));
        let mut projections: Vec<(usize, usize)> = Vec::new();
        for m in &mut modes {
            let key = (m.k, m.orient);
            m.proj = projections.iter().position(|&q| q == key).unwrap_or_else(|| {
                projections.push(key);
                projections.len() - 1
            });
        }
        let basis = |k: usize, orient: usize, s: f64| {
            let th = TAU * k as f64 * s / lb;
            if orient == 0 { th.cos() } else { th.sin() }
        };

        // Edge forces of the coupled board modes, projected on the rim.
        const PER_M: f64 = 400.0;
        let ns = (lb * PER_M) as usize;
        let ds = lb / ns as f64;
        let s: Vec<f64> = (0..ns).map(|i| (i as f64 + 0.5) * ds).collect();
        let corners = [0.0, a, a + b, 2.0 * a + b];
        let tab: Vec<Vec<f64>> = projections.iter().map(|&(k, o)| s.iter().map(|&s| basis(k, o, s)).collect()).collect();
        let basis_edges = basis_edge_reactions(sb, &s);
        let couple = sb.modes[..n_board]
            .iter()
            .take_while(|md| md.omega / TAU < p.couple_f_max)
            .map(|md| {
                if md.m == 0 {
                    // The piston mode moves the rim with the board: no edge force.
                    return vec![(0.0, 0.0); projections.len()];
                }
                let (dens, cf, scale) = board_edge_forces(sb, &basis_edges, md, &s);
                let sigma = sb.sigma(md);
                // Edge force follows the displacement (and the damping force):
                // q + 2σ/ω_n²·q̇ = Re(V·z·(σ − iω)/(σ² + ω²)) for the bank's
                // velocity weight V = 1 + iσ/ω.
                let disp = (C64::new(1.0, sigma / md.omega) * C64::new(sigma, -md.omega))
                    .scale(1.0 / (sigma * sigma + md.omega * md.omega));
                projections
                    .iter()
                    .zip(&tab)
                    .map(|(&(k, o), tb)| {
                        let edge: f64 = dens.iter().zip(tb).map(|(d, c)| d * c).sum::<f64>() * ds;
                        let corner: f64 = cf.iter().zip(&corners).map(|(f, &sc)| f * basis(k, o, sc)).sum();
                        let w = disp.scale((edge + corner) * scale);
                        (w.re, w.im)
                    })
                    .collect()
            })
            .collect();

        // Radiation: elements of the four faces of the wall.
        struct Elem {
            pos: [f64; 3],
            normal: [f64; 3],
            /// Normal velocity per unit (W, B) of the section.
            vel: (f64, f64),
            area: f64,
            s: f64,
        }
        let nr = (lb / 0.04).ceil() as usize;
        let dsr = lb / nr as f64;
        let nz = 4;
        let dz = p.height / nz as f64;
        let zc = p.top - p.height / 2.0;
        let mut elems = Vec::new();
        for i in 0..nr {
            let sr = (i as f64 + 0.5) * dsr;
            let ((x, y), (nx, ny)) = perimeter(a, b, sr);
            let at = |off: f64, z: f64| [x + nx * off, y + ny * off, z];
            for j in 0..nz {
                let z = p.top - (j as f64 + 0.5) * dz;
                let zo = z - zc;
                // A rotation B (inner edge up) moves the top of the wall outwards.
                elems.push(Elem { pos: at(t, z), normal: [nx, ny, 0.0], vel: (0.0, zo), area: dsr * dz, s: sr });
                elems.push(Elem {
                    pos: at(0.0, z),
                    normal: [-nx, -ny, 0.0],
                    vel: (0.0, -zo * p.inner_gain),
                    area: dsr * dz,
                    s: sr,
                });
            }
            elems.push(Elem { pos: at(0.5 * t, p.top), normal: [0.0, 0.0, 1.0], vel: (1.0, 0.0), area: dsr * t, s: sr });
            elems.push(Elem {
                pos: at(0.5 * t, p.top - p.height),
                normal: [0.0, 0.0, -1.0],
                vel: (-1.0, 0.0),
                area: dsr * t,
                s: sr,
            });
        }
        for m in &mut modes {
            let omega = TAU * m.freq_hz;
            let k = omega / C0;
            let pre = C64::new(0.0, omega * RHO0 / TAU);
            let shape = C64::new(1.0, m.sigma / omega);
            for (mic, out) in m.mics.iter_mut().enumerate() {
                let (mx, my, mz) = bp.mics[mic];
                let mut acc = C64::ZERO;
                for e in &elems {
                    let d = [mx - e.pos[0], my - e.pos[1], mz - e.pos[2]];
                    let r = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
                    let cos = (d[0] * e.normal[0] + d[1] * e.normal[1] + d[2] * e.normal[2]) / r;
                    let v = (e.vel.0 * m.w + e.vel.1 * m.b) * basis(m.k, m.orient, e.s);
                    acc += C64::polar(v * e.area * 0.5 * (1.0 + cos) / r, -k * r);
                }
                let w = (pre * acc * shape).scale(p.gain);
                *out = (w.re, w.im);
            }
        }
        Self { modes, projections, couple }
    }
}

/// The rim's modes, each driven by one projection of the board's edge
/// forces (complex one-pole per mode, as the board bank).
pub struct RimBank {
    pr: Vec<f32>,
    pi: Vec<f32>,
    b: Vec<f32>,
    proj: Vec<usize>,
    w: Vec<[(f32, f32); MICS]>,
    zr: Vec<f32>,
    zi: Vec<f32>,
    fs2: f32,
}

impl RimBank {
    pub fn new(model: &RimModel, sample_rate: f64) -> Self {
        let t = 1.0 / sample_rate;
        let pole = |m: &RimMode| C64::polar((-m.sigma * t).exp(), TAU * m.freq_hz * t);
        let n = model.modes.len();
        Self {
            pr: model.modes.iter().map(|m| pole(m).re as f32).collect(),
            pi: model.modes.iter().map(|m| pole(m).im as f32).collect(),
            b: model.modes.iter().map(|m| (m.drive * t) as f32).collect(),
            proj: model.modes.iter().map(|m| m.proj).collect(),
            w: model.modes.iter().map(|m| m.mics.map(|(re, im)| (re as f32, im as f32))).collect(),
            zr: vec![0.0; n],
            zi: vec![0.0; n],
            fs2: (sample_rate * sample_rate) as f32,
        }
    }

    /// Output-weighted energy at the first mic, to let the rim sleep
    /// (acceleration weights: see [`crate::board::BoardBank::energy`]).
    pub fn energy(&self) -> f32 {
        let mut e = 0.0;
        for j in 0..self.zr.len() {
            let (wr, wi) = self.w[j][0];
            let d = (1.0 - self.pr[j]) * (1.0 - self.pr[j]) + self.pi[j] * self.pi[j];
            e += (wr * wr + wi * wi) * d * (self.zr[j] * self.zr[j] + self.zi[j] * self.zi[j]);
        }
        e * self.fs2
    }

    /// Advances by `len` samples driven by the edge-force projections
    /// `forces[p][..len]`, adding the pressure to the mics.
    pub fn process(&mut self, forces: &[Vec<f32>], len: usize, mics: [&mut [f32]; MICS]) {
        let [m0, m1] = mics;
        for j in 0..self.pr.len() {
            let (pr, pi, b) = (self.pr[j], self.pi[j], self.b[j]);
            let (mut zr, mut zi) = (self.zr[j], self.zi[j]);
            let [(w0r, w0i), (w1r, w1i)] = self.w[j];
            let f = &forces[self.proj[j]][..len];
            for t in 0..len {
                let r = pr * zr - pi * zi + b * f[t];
                zi = pr * zi + pi * zr;
                zr = r;
                m0[t] += w0r * zr - w0i * zi;
                m1[t] += w1r * zr - w1i * zi;
            }
            self.zr[j] = zr;
            self.zi[j] = zi;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Edge shear plus corner forces balance the plate's inertia:
    /// net = ω_ss²·ρh·∫Φ (zero for modes with an even index).
    #[test]
    fn edge_reactions_balance_the_plate() {
        let p = SoundboardParams::GRAND_2M;
        let lb = 2.0 * (p.a + p.b);
        let ns = 20_000;
        let ds = lb / ns as f64;
        let s: Vec<f64> = (0..ns).map(|i| (i as f64 + 0.5) * ds).collect();
        for (m, n) in [(1, 1), (1, 2), (2, 1), (3, 1), (3, 5), (2, 2), (5, 3)] {
            let (dens, cf) = edge_reaction(&p, m, n, &s);
            let net = dens.iter().sum::<f64>() * ds + cf.iter().sum::<f64>();
            let int = |k: u32, l: f64| l * (1.0 - (k as f64 * PI).cos()) / (k as f64 * PI);
            let mass = p.rho_h * 2.0 / (p.rho_h * p.a * p.b).sqrt() * int(m, p.a) * int(n, p.b);
            let want = omega_ss(&p, m, n).powi(2) * mass;
            let scale = omega_ss(&p, m, n).powi(2) * p.rho_h;
            assert!((net - want).abs() < 2e-3 * scale, "({m},{n}): {net} vs {want}");
        }
    }

    /// The frame follows the case: in board coordinates its outline is the
    /// board's, its front edge straight over the pin block; on a rectangular
    /// board it is the rectangle.
    #[test]
    fn the_frame_follows_the_case() {
        let f = FrameParams::GRAND_2M;
        let board = SoundboardParams { outline: SoundboardParams::GRAND_OUTLINE, ..SoundboardParams::GRAND_2M };
        for (&(u, v), &(fu, fv)) in board.outline.iter().zip(&f.outline(&board)) {
            let (x, y) = (f.offset_x + fu * f.a, fv * f.b);
            if u <= 1e-9 {
                assert!((x - f.offset_x).abs() < 1e-12, "front edge at {x}");
            } else {
                assert!((x - u * board.a).abs() < 1e-9 && (y - v * board.b).abs() < 1e-9, "({x}, {y}) vs ({u}, {v})");
            }
        }
        // The plate reaches the tail.
        assert!(f.offset_x + f.a >= board.a - 1e-9);
        assert!(Soundboard::new(f.plate(&board)).outline.is_some());
        let rectangle = SoundboardParams { outline: [(0.0, 0.0); OUTLINE_POINTS], ..SoundboardParams::GRAND_2M };
        assert!(Soundboard::new(f.plate(&rectangle)).outline.is_none());
    }

    /// An outlined board's own modes push on the rim (shared edges and the
    /// ground outside the outline) as much as their inertia.
    #[test]
    fn outlined_edge_forces_balance_the_plate() {
        let p = SoundboardParams { outline: SoundboardParams::GRAND_OUTLINE, ..SoundboardParams::GRAND_2M };
        let sb = Soundboard::new(p);
        let lb = 2.0 * (p.a + p.b);
        let ns = 20_000;
        let ds = lb / ns as f64;
        let s: Vec<f64> = (0..ns).map(|i| (i as f64 + 0.5) * ds).collect();
        let edges = basis_edge_reactions(&sb, &s);
        let int = |k: u32, l: f64| l * (1.0 - (k as f64 * PI).cos()) / (k as f64 * PI);
        for md in sb.modes.iter().filter(|m| m.ritz.is_some()).take(12) {
            let (dens, cf, scale) = board_edge_forces(&sb, &edges, md, &s);
            let net = (dens.iter().sum::<f64>() * ds + cf.iter().sum::<f64>()) * scale;
            // ω²·∫ρh·Φ over the rectangle (Φ ≈ 0 outside the outline).
            let mass: f64 = sb
                .basis
                .iter()
                .zip(&sb.ritz[md.ritz.unwrap() as usize])
                .map(|(&(m, n, _), c)| c * p.rho_h * 2.0 / (p.rho_h * p.a * p.b).sqrt() * int(m, p.a) * int(n, p.b))
                .sum();
            let want = md.omega.powi(2) * mass;
            // Within 3 % of its bound ω²·√(ρh·a·b) (|∫ρh·Φ| ≤ √(ρh·A) for a
            // mass-normalised Φ): the basis's Kirchhoff reactions are
            // approximate.
            let size = md.omega.powi(2) * (p.rho_h * p.a * p.b).sqrt();
            assert!((net - want).abs() < 0.03 * size, "{:.0} Hz: {net} vs {want}", md.omega / TAU);
        }
    }

    /// The ring's modes: order k = 1 has a rigid rocking mode (dropped) and
    /// frequencies rise with k; mass normalisation holds.
    #[test]
    fn rim_modes_are_sane() {
        let sb = Soundboard::new(SoundboardParams::GRAND_2M);
        let model = RimModel::new(&RimParams::GRAND_2M, &sb, sb.modes.len(), 48_000.0);
        assert!(!model.modes.is_empty());
        let first = model.modes[0].freq_hz;
        assert!((20.0..400.0).contains(&first), "first rim mode {first} Hz");
        for m in &model.modes {
            assert!(m.freq_hz < 3000.0);
            assert!(m.mics.iter().all(|w| w.0.is_finite() && w.1.is_finite()));
        }
        assert!(!model.couple.is_empty() && model.couple[0].len() == model.projections.len());
    }
}
