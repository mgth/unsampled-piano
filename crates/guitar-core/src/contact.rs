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

use crate::string::{Bent, StringTable, PERP};

/// Contacts on one string at the same time.
pub const MAX_CONTACTS: usize = 24;
/// Contact groups (ids below this).
pub const GROUPS: usize = 8;

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
    /// A strike: from `start` to `to` in a raised cosine over `samples`
    /// (it arrives at speed), back to `start` over `back` samples (held at
    /// `to` when `back` is 0).
    Stroke { start: f64, to: f64, samples: usize, back: usize },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Contact {
    /// Distance from the bridge (m).
    pub x: f64,
    /// Polarisation it acts in ([`PERP`] or [`crate::string::PAR`]).
    pub pol: usize,
    /// u = sign·(y − target) is the compression: +1 when the contact lies
    /// on the positive side of the string (a fret, y counted toward the
    /// board), −1 on the negative side (a thumb or a finger coming down).
    pub sign: f64,
    pub law: Law,
    pub motion: Motion,
    pub release: Release,
    /// Contacts released together (the two directions of one fingertip):
    /// a force limit applies to the magnitude of their summed forces.
    /// Below [`GROUPS`].
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
    /// Force magnitude λ at the last step (N).
    lam: f64,
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
    /// Scratch of the per-sample system.
    mat: [[f64; MAX_CONTACTS]; MAX_CONTACTS],
    rhs: [f64; MAX_CONTACTS],
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
            mat: [[0.0; MAX_CONTACTS]; MAX_CONTACTS],
            rhs: [0.0; MAX_CONTACTS],
            samples: 0,
            active: false,
            max_force: 0.0,
        }
    }

    /// Starts an episode on `table` from the per-polarisation amplitudes
    /// `zeta` (one per partial), the mute at scale `damping`. Contacts are
    /// added next. Real-time safe.
    pub fn begin(&mut self, table: &StringTable, zeta: &[[C64; 2]], damping: f64) {
        self.begin_bent(table, zeta, damping, None, 1.0);
    }

    /// [`Self::begin`] with the partials at another tension (`bent`, tension
    /// ratio `rho`).
    pub fn begin_bent(&mut self, table: &StringTable, zeta: &[[C64; 2]], damping: f64, bent: Option<&[Bent]>, rho: f64) {
        self.np = table.len().min(self.max_p);
        for n in 0..self.np {
            self.k[n] = table.partials[n].k;
            self.force[n] = rho * table.force[n];
            for pol in 0..2 {
                let (omega, sigma) = bent.map_or(table.partials[n].polar[pol], |b| b[n].polar[pol]);
                let pole = self.set_mode(table, n, pol, omega, sigma, damping, rho);
                let (i, z) = (pol * self.max_p + n, zeta[n][pol]);
                self.q0[i] = z.re;
                self.q1[i] = (z / pole).re;
            }
        }
        self.slots = [None; MAX_CONTACTS];
        self.samples = 0;
        self.active = false;
        self.max_force = 0.0;
    }

    /// Coefficients and output weights of mode (`pol`, `n`) at tension ratio
    /// `rho`; its pole.
    #[allow(clippy::too_many_arguments)]
    fn set_mode(&mut self, table: &StringTable, n: usize, pol: usize, omega: f64, sigma: f64, damping: f64, rho: f64) -> C64 {
        let dt = self.dt;
        let p = &table.partials[n];
        // Pushed past the cap by a bend: kept below Nyquist, and silent.
        let audible = omega < std::f64::consts::TAU * table.f_cap;
        let omega = omega.min(std::f64::consts::TAU * table.f_cap);
        let i = pol * self.max_p + n;
        let sg = sigma + damping * p.damp_w;
        let r = (-sg * dt).exp();
        let (sn, cs) = (omega * dt).sin_cos();
        self.c[i] = r * cs;
        self.ps[i] = r * sn;
        self.e[i] = r * r;
        self.gamma[i] = dt * r * sn / (table.modal_mass * omega);
        // ẏ = Re((−σ + iω)·ζ), ζ = q[n] + i·(e·q[n−1] − c·q[n])/ps.
        let w = if audible { table.win[n][pol] } else { 0.0 };
        (self.wa[i], self.wb[i]) = if table.velocity_out {
            (w * (-sg + omega * self.c[i] / self.ps[i]), -w * omega * self.e[i] / self.ps[i])
        } else {
            // A force output is T·∂y/∂x: it follows the tension.
            (rho * w, 0.0)
        };
        if !audible && pol == PERP {
            self.force[n] = 0.0;
        }
        C64::new(r * cs, r * sn)
    }

    /// Retunes the running episode to the partials `bent` (tension ratio
    /// `rho`), the string's motion kept: each mode's complex amplitude
    /// carries over, and its previous sample is recomputed for the new pole;
    /// so is each contact's previous compression, which the next step
    /// differences against. Real-time safe.
    pub fn retune(&mut self, table: &StringTable, bent: &[Bent], rho: f64, damping: f64) {
        for n in 0..self.np {
            self.force[n] = rho * table.force[n];
            for pol in 0..2 {
                let z = self.complex_state(pol, n);
                let (omega, sigma) = bent[n].polar[pol];
                let pole = self.set_mode(table, n, pol, omega, sigma, damping, rho);
                let i = pol * self.max_p + n;
                self.q0[i] = z.re;
                self.q1[i] = (z / pole).re;
            }
        }
        let t_prev = self.samples.saturating_sub(1);
        for slot in 0..MAX_CONTACTS {
            if let Some(mut s) = self.slots[slot] {
                s.u1 = s.c.sign * (self.y(&self.q1, slot, s.c.pol) - Self::target(&s, t_prev));
                self.slots[slot] = Some(s);
            }
        }
        self.rebuild_g();
    }

    fn y(&self, q: &[f64], slot: usize, pol: usize) -> f64 {
        let sh = &self.shapes[slot * self.max_p..][..self.np];
        sh.iter().zip(&q[pol * self.max_p..][..self.np]).map(|(a, b)| a * b).sum()
    }

    fn target(s: &Slot, t: usize) -> f64 {
        let rise = |from: f64, to: f64, f: f64| from + (to - from) * (1.0 - (std::f64::consts::PI * f.clamp(0.0, 1.0)).cos()) / 2.0;
        let dt = t.saturating_sub(s.start);
        match s.c.motion {
            Motion::Fixed(y) => y,
            Motion::Ramp { to, samples } => rise(s.from, to, dt as f64 / samples.max(1) as f64),
            Motion::Stroke { start, to, samples, back } => {
                if dt <= samples || back == 0 {
                    rise(start, to, dt as f64 / samples.max(1) as f64)
                } else {
                    rise(to, start, (dt - samples) as f64 / back as f64)
                }
            }
        }
    }

    /// Adds a contact now: its slot, None when all are taken. Real-time safe.
    pub fn add(&mut self, c: Contact) -> Option<usize> {
        assert!((c.group as usize) < GROUPS, "contact group {} ≥ {GROUPS}", c.group);
        let slot = self.slots.iter().position(|s| s.is_none())?;
        // sin(k_n·x) with k_n = n·k_1: by its recurrence.
        let th = self.k[0] * c.x;
        let c2 = 2.0 * th.cos();
        let (mut s0, mut s1) = (0.0, th.sin());
        for n in 0..self.np {
            self.shapes[slot * self.max_p + n] = s1;
            (s0, s1) = (s1, c2 * s1 - s0);
        }
        let (y0, y1) = (self.y(&self.q0, slot, c.pol), self.y(&self.q1, slot, c.pol));
        let from = match c.motion {
            Motion::Fixed(y) | Motion::Stroke { start: y, .. } => y,
            Motion::Ramp { .. } => y0,
        };
        let u0 = c.sign * (y0 - from);
        // Starting in contact, a unilateral law starts with ψ = 0: setting
        // ψ = √(2V(u)) would add the energy of a penetration that nothing
        // did. Passive either way; the force builds up from the motion.
        let psi = 0.0;
        self.slots[slot] = Some(Slot { c, start: self.samples, from, u0, u1: c.sign * (y1 - from), psi, fade: None, lam: 0.0 });
        self.rebuild_g_for(slot);
        self.active = true;
        Some(slot)
    }

    /// Takes a contact away now. Real-time safe.
    pub fn remove(&mut self, slot: usize) {
        if self.slots[slot].take().is_some() {
            self.rebuild_g_for(slot);
            self.active = self.slots.iter().any(|s| s.is_some());
        }
    }

    /// Force magnitude of a contact at the last step (N), 0 when gone.
    pub fn force(&self, slot: usize) -> f64 {
        self.slots[slot].map_or(0.0, |s| s.lam)
    }

    /// Fastest a contact's target moves (m/s): how fast a hand can drive
    /// the string.
    pub fn target_speed(&self) -> f64 {
        let per = |d: f64, n: usize| std::f64::consts::PI * d.abs() / (2.0 * n.max(1) as f64 * self.dt);
        self.slots
            .iter()
            .flatten()
            .map(|s| match s.c.motion {
                Motion::Fixed(_) => 0.0,
                Motion::Ramp { to, samples } => per(to - s.from, samples),
                Motion::Stroke { start, to, samples, back } => per(to - start, samples).max(if back > 0 { per(to - start, back) } else { 0.0 }),
            })
            .fold(0.0, f64::max)
    }

    /// Whether any contact of `group` is present.
    pub fn has_group(&self, group: u8) -> bool {
        self.slots.iter().flatten().any(|s| s.c.group == group)
    }

    fn rebuild_g(&mut self) {
        for i in 0..MAX_CONTACTS {
            self.rebuild_g_for(i);
        }
    }

    /// Row and column `i` of Ĝ.
    fn rebuild_g_for(&mut self, i: usize) {
        for j in 0..MAX_CONTACTS {
            let g = self.g_entry(i, j);
            self.g[i][j] = g;
            self.g[j][i] = g;
        }
    }

    fn g_entry(&self, i: usize, j: usize) -> f64 {
        match (self.slots[i], self.slots[j]) {
            (Some(a), Some(b)) if a.c.pol == b.c.pol => {
                let (si, sj) = (&self.shapes[i * self.max_p..][..self.np], &self.shapes[j * self.max_p..][..self.np]);
                let gm = &self.gamma[a.c.pol * self.max_p..][..self.np];
                a.c.sign * b.c.sign * si.iter().zip(sj).zip(gm).map(|((x, y), g)| x * y * g).sum::<f64>()
            }
            _ => 0.0,
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
            // Contacts that exert nothing whatever happens (a = d = 0: a
            // fret not touched) stay out of the system.
            let nf = (0..m).filter(|&i| !out_set[i] && (a[i] != 0.0 || d[i] != 0.0)).fold(0, |n, i| {
                free[n] = i;
                n + 1
            });
            lam.fill(0.0);
            if nf > 0 {
                // Only the nf × nf block is written and read: no clearing.
                let (mat, rhs) = (&mut self.mat, &mut self.rhs);
                for (r, &i) in free[..nf].iter().enumerate() {
                    let u1 = self.slots[idx[i]].unwrap().u1;
                    for (c, &j) in free[..nf].iter().enumerate() {
                        mat[r][c] = (r == c) as u8 as f64 + d[i] * self.g[idx[i]][idx[j]];
                    }
                    rhs[r] = a[i] + d[i] * (u_star[i] - u1);
                }
                solve(mat, rhs, nf);
                for (r, &i) in free[..nf].iter().enumerate() {
                    lam[i] = rhs[r];
                }
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
        for pol in 0..2 {
            let r = pol * mp..pol * mp + np;
            self.q1[r.clone()].copy_from_slice(&self.q0[r.clone()]);
            self.q0[r.clone()].copy_from_slice(&self.pred[r]);
        }
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
            s.lam = lam[i];
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
        let mut group_f2 = [0.0f64; GROUPS];
        for i in 0..m {
            group_f2[self.slots[idx[i]].unwrap().c.group as usize] += lam[i] * lam[i];
        }
        let t = self.samples;
        let mut slip = [false; GROUPS];
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
        // Only each polarisation's active partials: past them lies what a
        // longer string of an earlier episode left.
        let mut out = 0.0;
        for pol in 0..2 {
            for i in pol * mp..pol * mp + np {
                out += self.wa[i] * self.q0[i] + self.wb[i] * self.q1[i];
            }
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
        let th = self.k[0] * x;
        let c2 = 2.0 * th.cos();
        let (mut s0, mut s1) = (0.0, th.sin());
        let mut y = 0.0;
        for q in &self.q0[pol * self.max_p..][..self.np] {
            y += s1 * q;
            (s0, s1) = (s1, c2 * s1 - s0);
        }
        y
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

    #[test]
    fn a_shorter_string_does_not_hear_the_longer_one() {
        // An episode on the open low E, then one on its 12th fret (half the
        // partials) from rest with a contact that pushes nothing: silence.
        let m = GuitarModel::default();
        let (open, oct) = (open(&m, 0), StringTable::new(&m, 0, fret_length(m.scale_length, 12), 0.3, SR));
        assert!(oct.len() < open.len());
        let mut sol = ContactSolver::new(open.len(), SR);
        let still = Contact { x: 0.1, pol: PERP, sign: 1.0, law: Law::Spring { stiffness: 0.0, damping: 0.0 }, motion: Motion::Fixed(0.0), release: Release::Never, group: 0 };
        let plucked: Vec<[C64; 2]> = open.shape.iter().map(|a| [C64::new(a * 2e-3, 0.0), C64::new(a * 1e-3, 0.0)]).collect();
        sol.begin(&open, &plucked, 0.0);
        sol.add(still);
        let (mut out, mut force) = (vec![0.0f32; 20], vec![0.0f32; 20]);
        sol.run(&mut out, &mut force);
        sol.begin(&oct, &vec![[C64::ZERO; 2]; oct.len()], 0.0);
        sol.add(still);
        let (mut out, mut force) = (vec![0.0f32; 20], vec![0.0f32; 20]);
        sol.run(&mut out, &mut force);
        assert!(out.iter().chain(&force).all(|&v| v == 0.0), "{out:?}");
    }

    #[test]
    fn retune_keeps_the_motion_and_moves_the_pitch() {
        // A ringing string retuned a whole tone up: same displacement now,
        // and its fundamental then turns at the new frequency.
        let m = GuitarModel::electric_solid();
        let t = open(&m, 3);
        let zeta: Vec<[C64; 2]> = t.shape.iter().map(|a| [C64::new(a * 1e-3, 0.0), C64::new(a * 5e-4, 0.0)]).collect();
        let mut sol = ContactSolver::new(t.len(), SR);
        sol.begin(&t, &zeta, 0.0);
        let still = Contact { x: 0.1, pol: PERP, sign: 1.0, law: Law::Spring { stiffness: 0.0, damping: 0.0 }, motion: Motion::Fixed(0.0), release: Release::Never, group: 0 };
        sol.add(still);
        let (mut o, mut f) = (vec![0.0f32; 100], vec![0.0f32; 100]);
        sol.run(&mut o, &mut f);
        let before: Vec<C64> = (0..t.len()).map(|n| sol.complex_state(PERP, n)).collect();
        let rho = t.ratio_for_pitch(2f64.powf(2.0 / 12.0));
        let bent: Vec<Bent> = (0..t.len()).map(|n| t.bent(n, rho)).collect();
        sol.retune(&t, &bent, rho, 0.0);
        for (n, b) in before.iter().enumerate().take(10) {
            assert!((sol.complex_state(PERP, n) - *b).norm() < 1e-12 + 1e-9 * b.norm(), "partial {}", n + 1);
        }
        // One sample later the fundamental has turned by the new ω·dt.
        sol.run(&mut o[..1], &mut f[..1]);
        let turned = (sol.complex_state(PERP, 0) / before[0]).arg();
        let expect = bent[0].polar[PERP].0 / SR;
        assert!((turned - expect).abs() < 1e-6 * expect, "turned {turned}, expected {expect}");
    }

    #[test]
    fn a_force_output_follows_the_tension() {
        // Bridge-force output (gain 0.1): after a retune to ρ = 2, and when
        // started bent, the output is 0.1 × the force the solver computes.
        let m = GuitarModel::default();
        let t = open(&m, 3);
        let mut zeta = vec![[C64::ZERO; 2]; t.len()];
        zeta[0][PERP] = C64::new(1e-3, 0.0);
        let rho = 2.0;
        let bent: Vec<Bent> = (0..t.len()).map(|n| t.bent(n, rho)).collect();
        let still = Contact { x: 0.1, pol: PERP, sign: 1.0, law: Law::Spring { stiffness: 0.0, damping: 0.0 }, motion: Motion::Fixed(0.0), release: Release::Never, group: 0 };
        for started_bent in [false, true] {
            let mut sol = ContactSolver::new(t.len(), SR);
            if started_bent {
                sol.begin_bent(&t, &zeta, 0.0, Some(&bent), rho);
            } else {
                sol.begin(&t, &zeta, 0.0);
                sol.retune(&t, &bent, rho, 0.0);
            }
            sol.add(still);
            let (mut o, mut f) = (vec![0.0f32; 4], vec![0.0f32; 4]);
            sol.run(&mut o, &mut f);
            for (a, b) in o.iter().zip(&f) {
                assert!((a - 0.1 * b).abs() < 1e-6 * b.abs(), "started bent {started_bent}: output {a} vs force {b}");
            }
        }
    }

    #[test]
    fn retuning_under_a_contact_is_starting_there() {
        // A spring holds a ringing string; a retune mid-contact must give the
        // same next force as a solver started at the new tension from the
        // same state and the same spring.
        let m = GuitarModel::electric_solid();
        let t = open(&m, 3);
        let zeta: Vec<[C64; 2]> = t.shape.iter().map(|a| [C64::new(a * 1e-3, 0.0), C64::ZERO]).collect();
        let spring = Contact { x: 0.1, pol: PERP, sign: 1.0, law: Law::Spring { stiffness: 5e3, damping: 5.0 }, motion: Motion::Fixed(0.0), release: Release::Never, group: 0 };
        let mut a = ContactSolver::new(t.len(), SR);
        a.begin(&t, &zeta, 0.0);
        let sa = a.add(spring).unwrap();
        let (mut o, mut f) = (vec![0.0f32; 100], vec![0.0f32; 100]);
        a.run(&mut o, &mut f);
        let rho = t.ratio_for_pitch(2f64.powf(2.0 / 12.0));
        let bent: Vec<Bent> = (0..t.len()).map(|n| t.bent(n, rho)).collect();
        a.retune(&t, &bent, rho, 0.0);
        let now: Vec<[C64; 2]> = (0..t.len()).map(|n| [a.complex_state(PERP, n), a.complex_state(PAR, n)]).collect();
        let mut b = ContactSolver::new(t.len(), SR);
        b.begin_bent(&t, &now, 0.0, Some(&bent), rho);
        let sb = b.add(spring).unwrap();
        a.step();
        b.step();
        let (fa, fb) = (a.force(sa), b.force(sb));
        assert!((fa - fb).abs() < 1e-9 * fb.abs(), "retuned {fa} N vs started there {fb} N");
    }
}
