//! Contacts on a string (fingertip, plectrum, fret, fingerboard), solved
//! sample by sample in modal space.
//!
//! While something touches the string, its modes are advanced here instead
//! of in the bank, and handed back as complex amplitudes afterwards. This is
//! the piano's hammer solver (`piano_core::hammer`) generalised to several
//! contact points in either polarisation. Each mode of each polarisation
//! uses the exact recursion of the bank,
//!
//! ```text
//! q[n+1] − 2c·q[n] + e·q[n−1] = γ·Σᵢ φ(xᵢ)·Fᵢ[n],   c = r·cosθ, e = r², γ = dt·r·sinθ/(m·ω)
//! ```
//!
//! For each contact, its compression u = s·(y(x) − target) is linear in the
//! unknown forces: u[n+1] = u* − Ĝ·λ, with Ĝᵢⱼ = sᵢsⱼ·Σ φ(xᵢ)γφ(xⱼ) over the
//! modes of their common polarisation. The laws are written so the force
//! magnitude λ is linear in Δu = u[n+1] − u[n−1]:
//!
//! - spring–dashpot (bilateral): λ = K·(u[n+1] + u[n−1])/2 + η·Δu, η = r/(2dt);
//! - power law K·u₊^α (unilateral), quadratised as in the piano (Ducceschi
//!   et al. 2021): with V = K/(α+1)·u₊^{α+1}, ψ = √(2V), g = V′/ψ at u[n],
//!   λ = g·ψ[n−½] + (g²/4 + η)·Δu and ψ[n+½] = ψ[n−½] + g·Δu/2.
//!
//! So each sample is one small linear system (I + D·Ĝ)·λ = a + D·(u* − u[n−1]),
//! with an active set for unilateral contacts that would pull. With fixed
//! targets the discrete energy (string + ψ² + spring terms) can only
//! decrease: the scheme is passive whatever the stiffness.
//!
//! The cross term of the bridge admittance is left out while contacts last
//! (each polarisation on its own admittance): it acts over tenths of a
//! second, contacts over milliseconds. The modes left out above `max_freq`
//! are not added back as a static correction yet (the piano does it for the
//! hammer); with a few hundred modes per string their compliance is small.

// Small fixed-size solver loops read better with explicit indices.
#![allow(clippy::needless_range_loop)]

use piano_core::complex::C64;

use crate::string::{StringTable, PERP};

/// Contacts on one string at the same time.
pub const MAX_CONTACTS: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Law {
    /// Bilateral spring–dashpot: a fingertip holding the string.
    Spring { stiffness: f64, damping: f64 },
    /// Unilateral power law K·u₊^α, dashpot r while touching: a fret, the
    /// fingerboard, a nail.
    Unilateral { stiffness: f64, exponent: f64, damping: f64 },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Motion {
    Fixed(f64),
    /// From where the string is when the contact starts to `to` (m), a
    /// raised cosine over `samples`, then held.
    Ramp { to: f64, samples: usize },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Contact {
    /// Distance from the bridge (m).
    pub x: f64,
    /// Polarisation it acts in ([`PERP`] or [`crate::string::PAR`]).
    pub pol: usize,
    /// u = sign·(y − target) is the compression: +1 when the contact is
    /// below the string (it pushes up), −1 above.
    pub sign: f64,
    pub law: Law,
    pub motion: Motion,
    pub release: Release,
    /// Contacts released together (the two directions of one fingertip):
    /// a force limit applies to the magnitude of their summed forces.
    pub group: u8,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Release {
    /// Stays until the episode is ended from outside.
    Never,
    /// Removed at once after this many samples from its start.
    After(usize),
    /// Slips off once the group's force reaches `limit` (N), or `timeout`
    /// samples after its start: its stiffness and damping fade out over
    /// `slip` samples (raised cosine), then it is removed.
    Force { limit: f64, slip: usize, timeout: usize },
}

#[derive(Clone, Copy)]
struct Slot {
    c: Contact,
    start: usize,
    /// Target when it started (where the string was, for a ramp).
    from: f64,
    /// u[n], u[n−1].
    u0: f64,
    u1: f64,
    psi: f64,
    /// Sample the slip began at.
    fade: Option<usize>,
}

impl Slot {
    /// Remaining share of the contact's stiffness at sample `t`.
    fn grip(&self, t: usize) -> f64 {
        match (self.fade, self.c.release) {
            (Some(f), Release::Force { slip, .. }) => {
                let x = (t.saturating_sub(f) as f64 / slip.max(1) as f64).min(1.0);
                (1.0 + (std::f64::consts::PI * x).cos()) / 2.0
            }
            _ => 1.0,
        }
    }

    fn done(&self, t: usize) -> bool {
        match self.c.release {
            Release::Never => false,
            Release::After(n) => t >= self.start + n,
            Release::Force { slip, .. } => self.fade.is_some_and(|f| t >= f + slip.max(1)),
        }
    }
}

pub struct ContactSolver {
    dt: f64,
    max_p: usize,
    /// Partials of the current episode.
    np: usize,
    // Per mode, index p·max_p + n.
    c: Vec<f64>,
    ps: Vec<f64>,
    e: Vec<f64>,
    gamma: Vec<f64>,
    q0: Vec<f64>,
    q1: Vec<f64>,
    pred: Vec<f64>,
    // Output weights: primary on q[n] and q[n−1] (a velocity needs both),
    // perpendicular bridge force on q[n].
    wa: Vec<f64>,
    wb: Vec<f64>,
    // Per partial.
    k: Vec<f64>,
    force: Vec<f64>,
    shapes: Vec<f64>,
    slots: [Option<Slot>; MAX_CONTACTS],
    g: [[f64; MAX_CONTACTS]; MAX_CONTACTS],
    samples: usize,
    active: bool,
    max_force: f64,
}

/// Solves `a·x = b` for the first `n` unknowns (partial pivoting); `b`
/// receives x. Singular pivots leave their unknown at 0.
fn solve(a: &mut [[f64; MAX_CONTACTS]; MAX_CONTACTS], b: &mut [f64; MAX_CONTACTS], n: usize) {
    for col in 0..n {
        let piv = (col..n).max_by(|&i, &j| a[i][col].abs().total_cmp(&a[j][col].abs())).unwrap();
        a.swap(col, piv);
        b.swap(col, piv);
        let d = a[col][col];
        if d.abs() < 1e-300 {
            continue;
        }
        for row in col + 1..n {
            let f = a[row][col] / d;
            for k in col..n {
                a[row][k] -= f * a[col][k];
            }
            b[row] -= f * b[col];
        }
    }
    for col in (0..n).rev() {
        let mut s = b[col];
        for k in col + 1..n {
            s -= a[col][k] * b[k];
        }
        b[col] = if a[col][col].abs() < 1e-300 { 0.0 } else { s / a[col][col] };
    }
}

impl ContactSolver {
    /// Allocates everything up front; nothing allocates afterwards.
    pub fn new(max_partials: usize, sample_rate: f64) -> Self {
        let m = 2 * max_partials;
        Self {
            dt: 1.0 / sample_rate,
            max_p: max_partials,
            np: 0,
            c: vec![0.0; m],
            ps: vec![0.0; m],
            e: vec![0.0; m],
            gamma: vec![0.0; m],
            q0: vec![0.0; m],
            q1: vec![0.0; m],
            pred: vec![0.0; m],
            wa: vec![0.0; m],
            wb: vec![0.0; m],
            k: vec![0.0; max_partials],
            force: vec![0.0; max_partials],
            shapes: vec![0.0; MAX_CONTACTS * max_partials],
            slots: [None; MAX_CONTACTS],
            g: [[0.0; MAX_CONTACTS]; MAX_CONTACTS],
            samples: 0,
            active: false,
            max_force: 0.0,
        }
    }

    /// Starts an episode on `table` from the per-polarisation amplitudes
    /// `zeta` (one per partial), the mute at scale `damping`. Contacts are
    /// added next. Real-time safe.
    pub fn begin(&mut self, table: &StringTable, zeta: &[[C64; 2]], damping: f64) {
        let dt = self.dt;
        self.np = table.len().min(self.max_p);
        for (n, p) in table.partials.iter().enumerate().take(self.np) {
            self.k[n] = p.k;
            self.force[n] = table.force[n];
            for pol in 0..2 {
                let i = pol * self.max_p + n;
                let (omega, sigma) = p.polar[pol];
                let r = (-(sigma + damping * p.damp_w) * dt).exp();
                let (sn, cs) = (omega * dt).sin_cos();
                self.c[i] = r * cs;
                self.ps[i] = r * sn;
                self.e[i] = r * r;
                self.gamma[i] = dt * r * sn / (table.modal_mass * omega);
                // ẏ = Re((−σ + iω)·ζ), ζ = q[n] + i·(e·q[n−1] − c·q[n])/ps.
                let w = table.win[n][pol];
                (self.wa[i], self.wb[i]) = if table.velocity_out {
                    let sg = sigma + damping * p.damp_w;
                    (w * (-sg + omega * self.c[i] / self.ps[i]), -w * omega * self.e[i] / self.ps[i])
                } else {
                    (w, 0.0)
                };
                let z = zeta[n][pol];
                self.q0[i] = z.re;
                self.q1[i] = (z / C64::new(r * cs, r * sn)).re;
            }
        }
        self.slots = [None; MAX_CONTACTS];
        self.samples = 0;
        self.active = false;
        self.max_force = 0.0;
    }

    fn y(&self, q: &[f64], slot: usize, pol: usize) -> f64 {
        let sh = &self.shapes[slot * self.max_p..][..self.np];
        sh.iter().zip(&q[pol * self.max_p..][..self.np]).map(|(a, b)| a * b).sum()
    }

    fn target(s: &Slot, t: usize) -> f64 {
        match s.c.motion {
            Motion::Fixed(y) => y,
            Motion::Ramp { to, samples } => {
                let f = ((t - s.start.min(t)) as f64 / samples.max(1) as f64).min(1.0);
                s.from + (to - s.from) * (1.0 - (std::f64::consts::PI * f).cos()) / 2.0
            }
        }
    }

    /// Adds a contact now. False when all slots are taken. Real-time safe.
    pub fn add(&mut self, c: Contact) -> bool {
        let Some(slot) = self.slots.iter().position(|s| s.is_none()) else { return false };
        for n in 0..self.np {
            self.shapes[slot * self.max_p + n] = (self.k[n] * c.x).sin();
        }
        let (y0, y1) = (self.y(&self.q0, slot, c.pol), self.y(&self.q1, slot, c.pol));
        let from = match c.motion {
            Motion::Fixed(y) => y,
            Motion::Ramp { .. } => y0,
        };
        let u0 = c.sign * (y0 - from);
        let psi = match c.law {
            Law::Unilateral { stiffness, exponent, .. } if u0 > 0.0 => (2.0 * stiffness / (exponent + 1.0) * u0.powf(exponent + 1.0)).sqrt(),
            _ => 0.0,
        };
        self.slots[slot] = Some(Slot { c, start: self.samples, from, u0, u1: c.sign * (y1 - from), psi, fade: None });
        self.rebuild_g();
        self.active = true;
        true
    }

    fn rebuild_g(&mut self) {
        for i in 0..MAX_CONTACTS {
            for j in 0..MAX_CONTACTS {
                self.g[i][j] = match (self.slots[i], self.slots[j]) {
                    (Some(a), Some(b)) if a.c.pol == b.c.pol => {
                        let (si, sj) = (&self.shapes[i * self.max_p..][..self.np], &self.shapes[j * self.max_p..][..self.np]);
                        let gm = &self.gamma[a.c.pol * self.max_p..][..self.np];
                        a.c.sign * b.c.sign * si.iter().zip(sj).zip(gm).map(|((x, y), g)| x * y * g).sum::<f64>()
                    }
                    _ => 0.0,
                };
            }
        }
    }

    /// Contacts present (the episode lasts while there are some).
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Largest contact force of the episode (N).
    pub fn max_force(&self) -> f64 {
        self.max_force
    }

    /// Complex amplitude ζ of partial `n` in polarisation `pol` now.
    pub fn complex_state(&self, pol: usize, n: usize) -> C64 {
        let i = pol * self.max_p + n;
        let a = self.q0[i];
        C64::new(a, (self.e[i] * self.q1[i] - a * self.c[i]) / self.ps[i])
    }

    /// Advances up to `out.len()` samples, *adding* the instrument's output
    /// to `out` and the perpendicular bridge force to `force`. Returns the
    /// samples rendered: fewer than requested when the last contact goes,
    /// after which the state is handed back.
    pub fn run(&mut self, out: &mut [f32], force: &mut [f32]) -> usize {
        for (i, (o, f)) in out.iter_mut().zip(force.iter_mut()).enumerate() {
            if !self.active {
                return i;
            }
            let (a, b) = self.step();
            *o += a as f32;
            *f += b as f32;
        }
        out.len()
    }

    fn step(&mut self) -> (f64, f64) {
        let (mp, np) = (self.max_p, self.np);
        for pol in 0..2 {
            for i in pol * mp..pol * mp + np {
                self.pred[i] = 2.0 * self.c[i] * self.q0[i] - self.e[i] * self.q1[i];
            }
        }
        // Per contact: u*, and λ = a + d·Δu.
        let mut idx = [0usize; MAX_CONTACTS];
        let mut m = 0;
        let (mut u_star, mut a, mut d, mut gq) = ([0.0; MAX_CONTACTS], [0.0; MAX_CONTACTS], [0.0; MAX_CONTACTS], [0.0; MAX_CONTACTS]);
        let t1 = self.samples + 1;
        for slot in 0..MAX_CONTACTS {
            let Some(s) = self.slots[slot] else { continue };
            idx[m] = slot;
            u_star[m] = s.c.sign * (self.y(&self.pred, slot, s.c.pol) - Self::target(&s, t1));
            let grip = s.grip(t1);
            (a[m], d[m], gq[m]) = match s.c.law {
                Law::Spring { stiffness, damping } => {
                    let (k, r) = (grip * stiffness, grip * damping);
                    (k * s.u1, k / 2.0 + r / (2.0 * self.dt), 0.0)
                }
                Law::Unilateral { stiffness, exponent, damping } => {
                    let (stiffness, damping) = (grip * stiffness, grip * damping);
                    if s.u0 > 0.0 && stiffness > 0.0 {
                        let v = stiffness / (exponent + 1.0) * s.u0.powf(exponent + 1.0);
                        let g = stiffness * s.u0.powf(exponent) / (2.0 * v).sqrt();
                        (g * s.psi, g * g / 4.0 + damping / (2.0 * self.dt), g)
                    } else {
                        (0.0, 0.0, 0.0)
                    }
                }
            };
            m += 1;
        }
        // (I + D·Ĝ)·λ = a + D·(u* − u[n−1]), unilateral contacts that would
        // pull taken out (λ = 0) until none does.
        let mut lam = [0.0; MAX_CONTACTS];
        let mut out_set = [false; MAX_CONTACTS];
        loop {
            let mut free = [0usize; MAX_CONTACTS];
            let nf = (0..m).filter(|&i| !out_set[i]).fold(0, |n, i| {
                free[n] = i;
                n + 1
            });
            let mut mat = [[0.0; MAX_CONTACTS]; MAX_CONTACTS];
            let mut rhs = [0.0; MAX_CONTACTS];
            for (r, &i) in free[..nf].iter().enumerate() {
                let u1 = self.slots[idx[i]].unwrap().u1;
                for (c, &j) in free[..nf].iter().enumerate() {
                    mat[r][c] = (r == c) as u8 as f64 + d[i] * self.g[idx[i]][idx[j]];
                }
                rhs[r] = a[i] + d[i] * (u_star[i] - u1);
            }
            solve(&mut mat, &mut rhs, nf);
            lam.fill(0.0);
            for (r, &i) in free[..nf].iter().enumerate() {
                lam[i] = rhs[r];
            }
            let mut changed = false;
            for i in 0..m {
                let s = self.slots[idx[i]].unwrap();
                if !out_set[i] && matches!(s.c.law, Law::Unilateral { .. }) && lam[i] < 0.0 {
                    out_set[i] = true;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        // States: compressions, ψ, modes.
        self.q1[..mp + np].copy_from_slice(&self.q0[..mp + np]);
        self.q0[..mp + np].copy_from_slice(&self.pred[..mp + np]);
        for i in 0..m {
            let slot = idx[i];
            let mut s = self.slots[slot].unwrap();
            let u_next = u_star[i] - (0..m).map(|j| self.g[slot][idx[j]] * lam[j]).sum::<f64>();
            if let Law::Unilateral { .. } = s.c.law {
                if !out_set[i] {
                    s.psi += gq[i] * (u_next - s.u1) / 2.0;
                }
                if u_next <= 0.0 {
                    s.psi = 0.0;
                }
            }
            s.u1 = s.u0;
            s.u0 = u_next;
            self.slots[slot] = Some(s);
            self.max_force = self.max_force.max(lam[i].abs());
            // Force on the string: −s·λ at x, in its polarisation.
            let f = -s.c.sign * lam[i];
            let base = s.c.pol * mp;
            let sh = &self.shapes[slot * mp..][..np];
            for n in 0..np {
                self.q0[base + n] += self.gamma[base + n] * sh[n] * f;
            }
        }
        self.samples += 1;
        // Slips: a group's force reaching its limit (or its timeout).
        let mut group_f2 = [0.0f64; 256];
        for i in 0..m {
            group_f2[self.slots[idx[i]].unwrap().c.group as usize] += lam[i] * lam[i];
        }
        let t = self.samples;
        let mut slip = [false; 256];
        for s in self.slots.iter().flatten() {
            if let Release::Force { limit, timeout, .. } = s.c.release
                && s.fade.is_none()
                && (group_f2[s.c.group as usize] >= limit * limit || t >= s.start + timeout)
            {
                slip[s.c.group as usize] = true;
            }
        }
        let mut gone = false;
        for s in &mut self.slots {
            if let Some(sl) = s {
                if sl.fade.is_none() && slip[sl.c.group as usize] && matches!(sl.c.release, Release::Force { .. }) {
                    sl.fade = Some(t);
                }
                if sl.done(t) {
                    *s = None;
                    gone = true;
                }
            }
        }
        if gone {
            self.rebuild_g();
            self.active = self.slots.iter().any(|s| s.is_some());
        }
        let mut out = 0.0;
        for i in 0..mp + np {
            out += self.wa[i] * self.q0[i] + self.wb[i] * self.q1[i];
        }
        let base = PERP * mp;
        let force = self.force[..np].iter().zip(&self.q0[base..base + np]).map(|(w, q)| w * q).sum();
        (out, force)
    }

    /// Whether the contacts of `group` have begun to slip off.
    pub fn slipping(&self, group: u8) -> bool {
        self.slots.iter().flatten().any(|s| s.c.group == group && s.fade.is_some())
    }

    /// Displacement of the string at `x` in polarisation `pol` now (m).
    pub fn displacement(&self, x: f64, pol: usize) -> f64 {
        (0..self.np).map(|n| (self.k[n] * x).sin() * self.q0[pol * self.max_p + n]).sum()
    }

    /// Discrete energy H[n+½] of the strings and contacts (twice the
    /// physical energy, as in the piano's solver), for checks.
    pub fn energy(&self) -> f64 {
        let mut h = 0.0;
        for pol in 0..2 {
            for i in pol * self.max_p..pol * self.max_p + self.np {
                let s = (1.0 + self.e[i]) / 2.0;
                h += (s * (self.q0[i] * self.q0[i] + self.q1[i] * self.q1[i]) - 2.0 * self.c[i] * self.q0[i] * self.q1[i]) / self.gamma[i];
            }
        }
        for s in self.slots.iter().flatten() {
            h += match s.c.law {
                Law::Unilateral { .. } => s.psi * s.psi,
                Law::Spring { stiffness, .. } => stiffness / 2.0 * (s.u0 * s.u0 + s.u1 * s.u1),
            };
        }
        h
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::GuitarModel;
    use crate::string::{fret_length, PAR};

    const SR: f64 = 48_000.0;

    fn open(m: &GuitarModel, s: usize) -> StringTable {
        let l = fret_length(m.scale_length, 0);
        StringTable::new(m, s, l, l - m.mute.distance, SR)
    }

    #[test]
    fn slow_finger_drag_and_release_is_the_ideal_pluck() {
        let mut m = GuitarModel::default();
        m.pluck.width = 0.0;
        let t = open(&m, 5);
        let mut sol = ContactSolver::new(t.len(), SR);
        let rest = vec![[C64::ZERO; 2]; t.len()];
        sol.begin(&t, &rest, 0.0);
        let drag = (0.3 * SR) as usize;
        let x0 = m.pluck.position;
        let law = Law::Spring { stiffness: 1e5, damping: 2.0 };
        sol.add(Contact { x: x0, pol: PERP, sign: 1.0, law, motion: Motion::Ramp { to: 2e-3, samples: drag }, release: Release::After(drag), group: 0 });
        let mut out = vec![0.0f32; drag + 10];
        let done = sol.run(&mut out, &mut vec![0.0f32; drag + 10]);
        assert_eq!(done, drag);
        // The string where the finger let go: a triangle, its height read
        // on the fundamental (the truncated series under-reads the corner
        // by ≈ 2 %: the residual left out, see the module notes).
        let h = sol.complex_state(PERP, 0).re / t.shape[0];
        assert!((1.5e-3..2.05e-3).contains(&h), "height at release {h}");
        for n in 1..8 {
            let got = sol.complex_state(PERP, n).re;
            let ideal = t.shape[n] * h;
            assert!((got - ideal).abs() < 0.01 * t.shape[0].abs() * h, "partial {}: {got} vs {ideal}", n + 1);
            assert!(sol.complex_state(PAR, n).norm() < 1e-12);
        }
    }

    #[test]
    fn fret_like_obstacle_is_passive_and_holds_the_string() {
        let m = GuitarModel::default();
        let t = open(&m, 0);
        let mut sol = ContactSolver::new(t.len(), SR);
        // Ringing low E, 1 mm at the pluck point, against an obstacle 0.3 mm
        // under its rest position halfway along.
        let zeta: Vec<[C64; 2]> = t.shape.iter().map(|a| [C64::new(a * 1e-3, 0.0), C64::ZERO]).collect();
        sol.begin(&t, &zeta, 0.0);
        let law = Law::Unilateral { stiffness: 1e9, exponent: 1.5, damping: 0.0 };
        let below = -0.3e-3;
        sol.add(Contact { x: 0.325, pol: PERP, sign: -1.0, law, motion: Motion::Fixed(below), release: Release::Never, group: 0 });
        let mut h_prev = sol.energy();
        let mut deepest: f64 = 0.0;
        for _ in 0..(0.2 * SR) as usize {
            sol.step();
            let h = sol.energy();
            assert!(h <= h_prev * (1.0 + 1e-9), "energy grew: {h_prev} → {h}");
            h_prev = h;
            deepest = deepest.max(sol.slots[0].unwrap().u0);
        }
        assert!(sol.max_force() > 0.0, "never touched");
        // A stiff obstacle: penetration stays a few µm.
        assert!(deepest < 2e-5, "penetration {deepest}");
    }

    #[test]
    fn several_contacts_in_both_polarisations() {
        let m = GuitarModel::default();
        let t = open(&m, 2);
        let mut sol = ContactSolver::new(t.len(), SR);
        sol.begin(&t, &vec![[C64::ZERO; 2]; t.len()], 0.0);
        let law = Law::Spring { stiffness: 5e3, damping: 5.0 };
        for (pol, to) in [(PERP, 1e-3), (PAR, -2e-3)] {
            sol.add(Contact { x: 0.12, pol, sign: 1.0, law, motion: Motion::Ramp { to, samples: 960 }, release: Release::After(960), group: 0 });
        }
        // A third one, held: a fingertip resting lightly near the 12th fret.
        sol.add(Contact { x: 0.33, pol: PERP, sign: 1.0, law: Law::Spring { stiffness: 50.0, damping: 1.0 }, motion: Motion::Fixed(0.0), release: Release::Never, group: 1 });
        let mut out = vec![0.0f32; 960];
        assert_eq!(sol.run(&mut out, &mut vec![0.0f32; 960]), 960);
        assert!(out.iter().all(|v| v.is_finite()));
        let y = |pol| -> f64 { t.partials.iter().enumerate().map(|(n, p)| (p.k * 0.12).sin() * sol.complex_state(pol, n).re).sum() };
        assert!(y(PERP) > 0.3e-3 && y(PAR) < -0.6e-3, "{} {}", y(PERP), y(PAR));
        assert!(sol.is_active());
    }

    #[test]
    fn finger_slips_off_at_its_force_and_fades_out() {
        let m = GuitarModel::default();
        let t = open(&m, 5);
        let mut sol = ContactSolver::new(t.len(), SR);
        sol.begin(&t, &vec![[C64::ZERO; 2]; t.len()], 0.0);
        let (x0, limit) = (m.pluck.position, 2.0);
        let k_string = t.tension * t.length / (x0 * (t.length - x0));
        let law = Law::Spring { stiffness: 5e3, damping: 5.0 };
        let release = Release::Force { limit, slip: 24, timeout: 4800 };
        sol.add(Contact { x: x0, pol: PERP, sign: 1.0, law, motion: Motion::Ramp { to: 0.01, samples: 2400 }, release, group: 3 });
        let mut at_slip = None;
        let mut n = 0;
        while sol.is_active() && n < 4800 {
            sol.step();
            n += 1;
            if at_slip.is_none() && sol.slipping(3) {
                at_slip = Some((n, sol.displacement(x0, PERP)));
            }
        }
        let (slip_at, y) = at_slip.expect("never slipped");
        assert!(!sol.is_active() && n == slip_at + 24, "removed {} samples after the slip began", n - slip_at);
        // Released where the string pushes back with the limit force.
        let expect = limit / k_string;
        assert!((y / expect - 1.0).abs() < 0.05, "released at {y} m, expected {expect}");
    }
}
