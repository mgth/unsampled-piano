//! Hammer–string contact, explicit and provably passive.
//!
//! While the hammer touches the strings, their modes are advanced by this
//! solver instead of the modal bank; afterwards their complex amplitudes are
//! handed back (see [`crate::voice`]). Each mode uses the exact (impulse-invariant) recursion
//!
//! ```text
//! q[n+1] − 2c·q[n] + e·q[n−1] = φ_k·γ_k·F[n],   c = r·cosθ, e = r²
//! ```
//!
//! (the same discretisation as the bank), the hammer `m·ẍ = −F`, and the felt
//! force uses energy quadratisation (Ducceschi et al., JASA 2021; van Walstijn
//! et al., DAFx 2023): with V(u) = Q/(p+1)·u₊^{p+1}, ψ = √(2V), g = V′/ψ,
//!
//! ```text
//! ψ[n+½] = ψ[n−½] + g[n]·(u[n+1] − u[n−1])/2,   F[n] = g[n]·(ψ[n+½] + ψ[n−½])/2 + η[n]·(u[n+1] − u[n−1])
//! ```
//!
//! where η ≥ 0 is the linearised hysteresis of Stulov's
//! F = Q·[u^p + α·d(u^p)/dt]. Everything is linear in the unknown F, so each
//! step is one scalar division: no iteration, fixed cost O(modes).
//!
//! Energy: multiplying each equation by its centred difference gives
//! H[n+½] − H[n−½] = −Σ d_k(Δq_k)²/γ_k − η(Δu)² ≤ 0 with
//! H = Σ_k [s_k(q[n+1]² + q[n]²) − 2c_k·q[n+1]q[n]]/γ_k + (m/T²)(x[n+1] − x[n])² + ψ[n+½]²,
//! s = (1+e)/2, d = (1−e)/2. H is positive definite (|c| < s), hence stable.
//! The two guards below (no pulling force, ψ reset when out of contact) only
//! remove energy.

use crate::complex::C64;
use crate::params::NoteParams;

/// One string mode as seen by the contact solver.
#[derive(Clone, Copy, Debug)]
pub struct PhysMode {
    /// Partial number (mode shape sin(kπx/L)).
    pub k: u32,
    pub freq: f64,
    pub sigma: f64,
    /// Current complex amplitude ζ (displacement q = Re ζ, metres).
    pub state: C64,
    /// Fraction of the hammer seen by this string (1, or 0 when shifted away).
    pub hit: f64,
}

/// Separation time after which the hammer is considered gone (re-contacts
/// before that are simulated: they happen in the treble and the bass).
const RELEASE_AFTER_S: f64 = 0.004;
/// Hard limit of a contact episode.
const MAX_CONTACT_S: f64 = 0.03;

pub struct ContactSolver {
    // Per mode.
    c: Vec<f64>,
    ps: Vec<f64>,
    e: Vec<f64>,
    gphi: Vec<f64>,
    phi: Vec<f64>,
    /// Inverse of γ_k (energy weights), and output weights (bridge force).
    inv_gamma: Vec<f64>,
    out_w: Vec<f64>,
    /// Force on the agraffe end, −T·∂y/∂x(L) = T·Σ (kπ/L)·(−1)^{k+1}·q_k
    /// (k from 1: the fundamental pulls both ends the same way).
    out_a: Vec<f64>,
    q0: Vec<f64>,
    q1: Vec<f64>,
    n: usize,
    // Hammer and contact.
    /// Base sample period, and the solver's (base / oversample).
    base_dt: f64,
    oversample: usize,
    dt: f64,
    mass: f64,
    q_si: f64,
    /// Hard core: stiffness (SI), exponent, depth (m) of the top felt.
    core_q_si: f64,
    core_p: f64,
    core_depth: f64,
    /// Largest felt compression of the episode (m).
    max_u: f64,
    p: f64,
    alpha: f64,
    lambda: f64,
    x0: f64,
    x1: f64,
    w0: f64,
    w1: f64,
    psi: f64,
    // Episode bookkeeping.
    active: bool,
    samples: usize,
    separated: usize,
    max_force: f64,
    contact_samples: usize,
}

impl ContactSolver {
    pub fn new(max_modes: usize, sample_rate: f64) -> Self {
        let v = || vec![0.0; max_modes];
        Self {
            c: v(),
            ps: v(),
            e: v(),
            gphi: v(),
            phi: v(),
            inv_gamma: v(),
            out_w: v(),
            out_a: v(),
            q0: v(),
            q1: v(),
            n: 0,
            base_dt: 1.0 / sample_rate,
            oversample: 1,
            dt: 1.0 / sample_rate,
            mass: 0.0,
            q_si: 0.0,
            core_q_si: 0.0,
            core_p: 2.5,
            core_depth: 1e-3,
            max_u: 0.0,
            p: 0.0,
            alpha: 0.0,
            lambda: 0.0,
            x0: 0.0,
            x1: 0.0,
            w0: 0.0,
            w1: 0.0,
            psi: 0.0,
            active: false,
            samples: 0,
            separated: 0,
            max_force: 0.0,
            contact_samples: 0,
        }
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Largest felt compression of the current/last episode (m).
    pub fn max_compression(&self) -> f64 {
        self.max_u
    }

    /// Felt law at compression u > 0: potential energy, force, stiffness.
    /// Top felt Q·u^p, plus the core Q_c·(u − d)^p_c beyond the depth d.
    #[inline]
    fn felt(&self, u: f64) -> (f64, f64, f64) {
        let up = u.powf(self.p);
        let (mut v, mut f, mut k) = (self.q_si * up * u / (self.p + 1.0), self.q_si * up, self.q_si * self.p * up / u);
        let d = u - self.core_depth;
        if self.core_q_si > 0.0 && d > 0.0 {
            let dp = d.powf(self.core_p);
            v += self.core_q_si * dp * d / (self.core_p + 1.0);
            f += self.core_q_si * dp;
            k += self.core_q_si * self.core_p * dp / d;
        }
        (v, f, k)
    }

    /// Peak force of the current/last episode (N).
    pub fn max_force(&self) -> f64 {
        self.max_force
    }

    /// Duration of the current/last episode with nonzero force (s).
    pub fn contact_time(&self) -> f64 {
        self.contact_samples as f64 * self.dt
    }

    /// Starts a strike at velocity `v` (m/s) on the given string modes
    /// (all strings of the unison, possibly already ringing). The contact is
    /// integrated `oversample` times faster than the bank (short treble
    /// contacts); the modes stay below the bank's Nyquist, so the output is
    /// simply taken every `oversample` steps. During the few milliseconds of
    /// contact the strings are treated as uncoupled (bridge coupling acts on
    /// a time scale 1/|ε| of 0.1–1 s). Real-time safe.
    pub fn strike<I>(&mut self, note: &NoteParams, v: f64, oversample: usize, modes: I)
    where
        I: IntoIterator<Item = PhysMode>,
        I::IntoIter: Clone,
    {
        self.oversample = oversample.max(1);
        self.dt = self.base_dt / self.oversample as f64;
        let dt = self.dt;
        let modal_mass = note.mu * note.length / 2.0;
        let half_w = std::f64::consts::PI * note.contact_width / (2.0 * note.length);
        let mut lambda = 0.0;
        let mut w0 = 0.0;
        let mut w1 = 0.0;
        let mut n = 0;
        // The strings of a unison lie side by side under one felt: in
        // parallel. The felt presses on their mean displacement (weighted by
        // the share each one takes, `hit`) and each takes that share of the
        // force; summing the displacements instead put them in series, the
        // hammer seeing them N² times too compliant.
        // (Modes come ordered by partial: the first ones are partial 1. No
        // allocation: the iterator is walked twice.)
        let modes = modes.into_iter();
        let share: f64 = modes.clone().take_while(|m| m.k == 1).map(|m| m.hit).sum::<f64>().max(1e-9);
        for m in modes.take(self.c.len()) {
            let i = n;
            n += 1;
            let k = m.k as f64;
            let r = (-m.sigma * dt).exp();
            let th = std::f64::consts::TAU * m.freq * dt;
            let (sn, cs) = th.sin_cos();
            let gamma = dt * r * sn / (modal_mass * std::f64::consts::TAU * m.freq);
            // Mode shape at the strike point, averaged over the felt width.
            // The bridge is at x = 0 and the hammer strikes `strike_pos`·L
            // from the *agraffe* (x = L): striking near the bridge would push
            // it quasi-statically with most of the hammer force (a thump).
            let x = k * half_w;
            let phi = m.hit / share
                * (k * std::f64::consts::PI * (1.0 - note.strike_pos)).sin()
                * if x > 1e-9 { x.sin() / x } else { 1.0 };
            self.c[i] = r * cs;
            self.ps[i] = r * sn;
            self.e[i] = r * r;
            self.phi[i] = phi;
            self.gphi[i] = gamma * phi;
            self.inv_gamma[i] = 1.0 / gamma;
            // Bridge force T·∂y/∂x(0) = T·Σ (kπ/L)·q_k.
            self.out_w[i] = note.tension * k * std::f64::consts::PI / note.length;
            self.out_a[i] = if m.k % 2 == 0 { -self.out_w[i] } else { self.out_w[i] };
            lambda += phi * phi * gamma;
            // Displacements now and one sub-step earlier from the complex amplitude.
            let pole = C64::new(r * cs, r * sn);
            self.q0[i] = m.state.re;
            self.q1[i] = (m.state / pole).re;
            w0 += phi * self.q0[i];
            w1 += phi * self.q1[i];
        }
        self.n = n;
        let h = note.hammer;
        self.mass = h.mass_kg;
        self.q_si = h.q_si();
        self.p = h.p;
        self.core_q_si = h.core_q * 1000f64.powf(h.core_p);
        self.core_p = h.core_p;
        self.core_depth = h.core_depth;
        self.max_u = 0.0;
        // Passivity must not depend on parameters: hysteresis only dissipates.
        self.alpha = h.alpha.max(0.0);
        self.lambda = lambda + dt * dt / self.mass;
        // Hammer arrives at the string now, at speed v.
        self.x0 = w0;
        self.x1 = w0 - v * dt;
        self.w0 = w0;
        self.w1 = w1;
        self.psi = 0.0;
        self.active = true;
        self.samples = 0;
        self.separated = 0;
        self.max_force = 0.0;
        self.contact_samples = 0;
    }

    /// Number of modes handled by the current episode.
    pub fn len(&self) -> usize {
        self.n
    }

    pub fn is_empty(&self) -> bool {
        self.n == 0
    }

    /// Complex amplitude ζ of mode `i` now (q = Re ζ), independent of the
    /// sampling rate: this is what is handed back to the coupled modes.
    pub fn complex_state(&self, i: usize) -> C64 {
        let (pr, pi, r2) = (self.c[i], self.ps[i], self.e[i]);
        let a = self.q0[i];
        C64::new(a, (r2 * self.q1[i] - a * pr) / pi)
    }

    /// Advances up to `out.len()` samples, *adding* the bridge force to
    /// `out` (and the agraffe force to `agraffe` when given). Returns how
    /// many samples were rendered: fewer than requested when the episode
    /// ends, after which the state must be handed back.
    pub fn run(&mut self, out: &mut [f32], mut agraffe: Option<&mut [f32]>) -> usize {
        for (i, o) in out.iter_mut().enumerate() {
            if !self.active {
                return i;
            }
            let mut y = (0.0, 0.0);
            for _ in 0..self.oversample {
                y = self.step();
            }
            *o += y.0 as f32;
            if let Some(a) = agraffe.as_deref_mut() {
                a[i] += y.1 as f32;
            }
        }
        out.len()
    }

    fn step(&mut self) -> (f64, f64) {
        let n = self.n;
        let dt = self.dt;
        // Free predictions and string displacement under the hammer.
        let mut w_star = 0.0;
        for i in 0..n {
            w_star += self.phi[i] * (2.0 * self.c[i] * self.q0[i] - self.e[i] * self.q1[i]);
        }
        let x_star = 2.0 * self.x0 - self.x1;
        let u_star = x_star - w_star;
        let u_now = self.x0 - self.w0;
        let u_prev = self.x1 - self.w1;

        // Quadratised felt law and linearised hysteresis at u[n].
        // g = F/√(2V) (= √(Q(p+1)/2)·u^((p−1)/2) for a single power law),
        // hysteresis α·dF/dt linearised.
        let (g, eta) = if u_now > 0.0 {
            let (v, f, k) = self.felt(u_now);
            let g = if v > 0.0 { f / (2.0 * v).sqrt() } else { 0.0 };
            (g, self.alpha * k / (2.0 * dt))
        } else {
            (0.0, 0.0)
        };
        let a = g * g / 4.0 + eta;
        let mut f = (g * self.psi + a * (u_star - u_prev)) / (1.0 + a * self.lambda);
        let clamped = f < 0.0;
        if clamped {
            f = 0.0;
        }
        let u_next = u_star - f * self.lambda;
        self.max_u = self.max_u.max(u_next);
        if !clamped {
            self.psi += g * (u_next - u_prev) / 2.0;
        }
        if u_next <= 0.0 {
            self.psi = 0.0;
        }

        // Updates.
        let x_next = x_star - dt * dt / self.mass * f;
        let mut w_next = 0.0;
        let (mut out, mut out_a) = (0.0, 0.0);
        for i in 0..n {
            let q = 2.0 * self.c[i] * self.q0[i] - self.e[i] * self.q1[i] + self.gphi[i] * f;
            self.q1[i] = self.q0[i];
            self.q0[i] = q;
            w_next += self.phi[i] * q;
            out += self.out_w[i] * q;
            out_a += self.out_a[i] * q;
        }
        self.x1 = self.x0;
        self.x0 = x_next;
        self.w1 = self.w0;
        self.w0 = w_next;

        // Episode end: separated long enough, or too long.
        self.samples += 1;
        self.max_force = self.max_force.max(f);
        if f > 0.0 {
            self.contact_samples += 1;
            self.separated = 0;
        } else if self.contact_samples > 0 || self.samples > 1 {
            self.separated += 1;
        }
        if self.separated as f64 * dt > RELEASE_AFTER_S || self.samples as f64 * dt > MAX_CONTACT_S {
            self.active = false;
        }
        (out, out_a)
    }

    /// Discrete energy H[n+½] (twice the physical energy), for checks.
    pub fn energy(&self) -> f64 {
        let mut h = 0.0;
        for i in 0..self.n {
            let s = (1.0 + self.e[i]) / 2.0;
            h += (s * (self.q0[i] * self.q0[i] + self.q1[i] * self.q1[i]) - 2.0 * self.c[i] * self.q0[i] * self.q1[i])
                * self.inv_gamma[i];
        }
        let v = self.x0 - self.x1;
        h + self.mass / (self.dt * self.dt) * v * v + self.psi * self.psi
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::PianoModel;

    fn setup(note: u8, sigma_scale: f64) -> (Vec<PhysMode>, NoteParams) {
        let sr = 48_000.0;
        let np = PianoModel::default().note(note);
        let modes = (1..=512u32)
            .map_while(|k| {
                let f = crate::params::partial_freq(np.f0, np.b, k);
                (f < 0.45 * sr).then_some(PhysMode {
                    k,
                    freq: f,
                    sigma: sigma_scale * (1.0 + 1e-6 * f * f),
                    state: C64::ZERO,
                    hit: 1.0,
                })
            })
            .collect();
        (modes, np)
    }

    #[test]
    fn complex_state_round_trips() {
        let (mut modes, np) = setup(48, 1.0);
        for (i, m) in modes.iter_mut().enumerate() {
            m.state = C64::new(1e-4 / (i + 1) as f64, -3e-5 * (i % 3) as f64);
        }
        let mut s = ContactSolver::new(512, 48_000.0);
        s.strike(&np, 1.0, 3, modes.iter().copied());
        for (i, m) in modes.iter().enumerate() {
            assert!((s.complex_state(i) - m.state).norm() < 1e-12 * (1.0 + m.state.norm()) + 1e-18);
        }
    }

    #[test]
    fn lossless_contact_conserves_energy() {
        let (modes, mut np) = setup(60, 0.0);
        np.hammer.alpha = 0.0;
        let mut s = ContactSolver::new(512, 48_000.0);
        s.strike(&np, 3.0, 1, modes.iter().copied());
        let mut out = [0.0f32; 1];
        s.run(&mut out, None);
        let h0 = s.energy();
        // Exact while in contact (only the separation guard may then remove
        // energy, and it never adds any).
        let mut prev = h0;
        while s.is_active() {
            s.run(&mut out, None);
            let h = s.energy();
            if s.psi > 0.0 {
                assert!((h - h0).abs() / h0 < 1e-10, "drift in contact {:e}", (h - h0) / h0);
            }
            assert!(h <= prev * (1.0 + 1e-12));
            prev = h;
        }
        assert!(s.contact_time() > 1e-4, "no contact");
        assert!((prev - h0).abs() / h0 < 1e-4, "guards removed too much: {:e}", (prev - h0) / h0);
    }

    #[test]
    fn lossy_contact_energy_never_grows() {
        for note in [21u8, 45, 60, 84, 100, 108] {
            for v in [0.3, 1.0, 3.0, 7.0] {
                let (modes, np) = setup(note, 1.0);
                let mut s = ContactSolver::new(512, 48_000.0);
                s.strike(&np, v, 1, modes.iter().copied());
                let mut out = [0.0f32; 1];
                s.run(&mut out, None);
                let mut prev = s.energy();
                while s.is_active() {
                    s.run(&mut out, None);
                    let h = s.energy();
                    assert!(h <= prev * (1.0 + 1e-12), "note {note} v {v}: {h} > {prev}");
                    assert!(out[0].is_finite());
                    prev = h;
                }
            }
        }
    }

    #[test]
    fn heavy_string_matches_rigid_surface_contact() {
        // A very heavy string behaves like a rigid wall: Stulov's closed form
        // (single-layer felt: no core).
        let (modes, mut np) = setup(60, 0.0);
        np.mu *= 1e6;
        np.hammer.alpha = 0.0;
        np.hammer.core_q = 0.0;
        for v in [1.0, 4.0] {
            let mut s = ContactSolver::new(512, 48_000.0);
            s.strike(&np, v, 1, modes.iter().copied());
            let mut out = [0.0f32; 64];
            while s.is_active() {
                s.run(&mut out, None);
            }
            let want = np.hammer.contact_time_rigid(v);
            let got = s.contact_time();
            assert!((got - want).abs() < 2.0 / 48_000.0 + 0.02 * want, "v {v}: {got} vs {want}");
        }
    }

    #[test]
    fn contact_shortens_and_force_grows_with_velocity() {
        let (modes, np) = setup(60, 1.0);
        let mut last: Option<(f64, f64)> = None;
        for v in [0.5, 1.0, 2.0, 4.0] {
            let mut s = ContactSolver::new(512, 48_000.0);
            s.strike(&np, v, 1, modes.iter().copied());
            let mut out = [0.0f32; 64];
            while s.is_active() {
                s.run(&mut out, None);
            }
            if let Some((t, f)) = last {
                assert!(s.contact_time() <= t, "contact time grows with v");
                assert!(s.max_force() > f);
            }
            last = Some((s.contact_time(), s.max_force()));
        }
    }
}

