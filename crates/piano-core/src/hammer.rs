//! Hammer–string contact, explicit and provably passive.
//!
//! While the hammer touches the string, the string modes are advanced by
//! this solver instead of the [`ModalBank`]; afterwards their state is handed
//! back to the bank. Each mode uses the exact (impulse-invariant) recursion
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

use crate::modal::ModalBank;
use crate::params::NoteParams;

/// Separation time after which the hammer is considered gone (re-contacts
/// before that are simulated: they happen in the treble and the bass).
const RELEASE_AFTER_S: f64 = 0.004;
/// Hard limit of a contact episode.
const MAX_CONTACT_S: f64 = 0.03;

pub struct ContactSolver {
    // Per mode.
    c: Vec<f64>,
    e: Vec<f64>,
    gphi: Vec<f64>,
    phi: Vec<f64>,
    /// Inverse of γ_k (energy weights), and output weights (bridge force).
    inv_gamma: Vec<f64>,
    out_w: Vec<f64>,
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
            e: v(),
            gphi: v(),
            phi: v(),
            inv_gamma: v(),
            out_w: v(),
            q0: v(),
            q1: v(),
            n: 0,
            base_dt: 1.0 / sample_rate,
            oversample: 1,
            dt: 1.0 / sample_rate,
            mass: 0.0,
            q_si: 0.0,
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

    /// Peak force of the current/last episode (N).
    pub fn max_force(&self) -> f64 {
        self.max_force
    }

    /// Duration of the current/last episode with nonzero force (s).
    pub fn contact_time(&self) -> f64 {
        self.contact_samples as f64 * self.dt
    }

    /// Starts a strike at velocity `v` (m/s) on the modes held by `bank`,
    /// whose current state (possibly ringing) is taken over. The contact is
    /// integrated `oversample` times faster than the bank (short treble
    /// contacts); the modes themselves stay below the bank's Nyquist, so the
    /// output is simply taken every `oversample` steps. Real-time safe.
    pub fn strike(&mut self, bank: &ModalBank, note: &NoteParams, v: f64, oversample: usize) {
        let n = bank.len().min(self.c.len());
        self.n = n;
        self.oversample = oversample.max(1);
        self.dt = self.base_dt / self.oversample as f64;
        let dt = self.dt;
        let modal_mass = note.mu * note.length / 2.0;
        let half_w = std::f64::consts::PI * note.contact_width / (2.0 * note.length);
        let mut lambda = 0.0;
        let mut w0 = 0.0;
        let mut w1 = 0.0;
        for i in 0..n {
            let spec = bank.spec(i);
            let k = (i + 1) as f64;
            let r = (-spec.sigma * dt).exp();
            let th = std::f64::consts::TAU * spec.freq_hz * dt;
            let (sn, cs) = th.sin_cos();
            let gamma = dt * r * sn / (modal_mass * std::f64::consts::TAU * spec.freq_hz);
            // Mode shape at the strike point, averaged over the felt width.
            let x = k * half_w;
            let phi = (k * std::f64::consts::PI * note.strike_pos).sin() * if x > 1e-9 { x.sin() / x } else { 1.0 };
            self.c[i] = r * cs;
            self.e[i] = r * r;
            self.phi[i] = phi;
            self.gphi[i] = gamma * phi;
            self.inv_gamma[i] = 1.0 / gamma;
            // Bridge force T·∂y/∂x(0) = T·Σ (kπ/L)·q_k.
            self.out_w[i] = note.tension * k * std::f64::consts::PI / note.length;
            lambda += phi * phi * gamma;
            // Current string state from the bank.
            let (qn, qn1) = bank.displacement_state(i, self.out_w[i], r, th);
            self.q0[i] = qn;
            self.q1[i] = qn1;
            w0 += phi * qn;
            w1 += phi * qn1;
        }
        let h = note.hammer;
        self.mass = h.mass_kg;
        self.q_si = h.q_si();
        self.p = h.p;
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

    /// Advances up to `out.len()` samples, *adding* the bridge force to
    /// `out`. Returns how many samples were rendered: fewer than requested
    /// when the episode ends, after which [`Self::hand_over`] must be called.
    pub fn run(&mut self, out: &mut [f32]) -> usize {
        for (i, o) in out.iter_mut().enumerate() {
            if !self.active {
                return i;
            }
            let mut y = 0.0;
            for _ in 0..self.oversample {
                y = self.step();
            }
            *o += y as f32;
        }
        out.len()
    }

    fn step(&mut self) -> f64 {
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
        let (g, eta) = if u_now > 0.0 {
            let up = u_now.powf(self.p - 1.0);
            let g = (self.q_si * (self.p + 1.0) / 2.0).sqrt() * u_now.powf((self.p - 1.0) / 2.0);
            let eta = self.q_si * self.alpha * self.p * up / (2.0 * dt);
            (g, eta)
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
        if !clamped {
            self.psi += g * (u_next - u_prev) / 2.0;
        }
        if u_next <= 0.0 {
            self.psi = 0.0;
        }

        // Updates.
        let x_next = x_star - dt * dt / self.mass * f;
        let mut w_next = 0.0;
        let mut out = 0.0;
        for i in 0..n {
            let q = 2.0 * self.c[i] * self.q0[i] - self.e[i] * self.q1[i] + self.gphi[i] * f;
            self.q1[i] = self.q0[i];
            self.q0[i] = q;
            w_next += self.phi[i] * q;
            out += self.out_w[i] * q;
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
        out
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

    /// Writes the string state back into the bank (same modes, same order).
    /// The complex amplitude of a mode does not depend on the sampling rate,
    /// so converting with the solver's pole is exact.
    pub fn hand_over(&self, bank: &mut ModalBank) {
        for i in 0..self.n {
            let spec = bank.spec(i);
            let r = (-spec.sigma * self.dt).exp();
            let th = std::f64::consts::TAU * spec.freq_hz * self.dt;
            bank.set_displacement_state(i, self.out_w[i], r, th, self.q0[i], self.q1[i]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modal::ModeSpec;
    use crate::params::PianoModel;

    fn setup(note: u8, sigma_scale: f64) -> (ModalBank, NoteParams) {
        let sr = 48_000.0;
        let np = PianoModel::default().note(note);
        let mut bank = ModalBank::new(sr, 512, 128);
        bank.set_modes((1..=512u32).map_while(|k| {
            let f = crate::params::partial_freq(np.f0, np.b, k);
            (f < 0.45 * sr).then(|| ModeSpec { freq_hz: f, sigma: sigma_scale * (1.0 + 1e-6 * f * f), ..Default::default() })
        }));
        (bank, np)
    }

    #[test]
    fn lossless_contact_conserves_energy() {
        let (bank, mut np) = setup(60, 0.0);
        np.hammer.alpha = 0.0;
        let mut s = ContactSolver::new(512, 48_000.0);
        s.strike(&bank, &np, 3.0, 1);
        let mut out = [0.0f32; 1];
        s.run(&mut out);
        let h0 = s.energy();
        // Exact while in contact (only the separation guard may then remove
        // energy, and it never adds any).
        let mut prev = h0;
        while s.is_active() {
            s.run(&mut out);
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
                let (bank, np) = setup(note, 1.0);
                let mut s = ContactSolver::new(512, 48_000.0);
                s.strike(&bank, &np, v, 1);
                let mut out = [0.0f32; 1];
                s.run(&mut out);
                let mut prev = s.energy();
                while s.is_active() {
                    s.run(&mut out);
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
        // A very heavy string behaves like a rigid wall: Stulov's closed form.
        let (bank, mut np) = setup(60, 0.0);
        np.mu *= 1e6;
        np.hammer.alpha = 0.0;
        for v in [1.0, 4.0] {
            let mut s = ContactSolver::new(512, 48_000.0);
            s.strike(&bank, &np, v, 1);
            let mut out = [0.0f32; 64];
            while s.is_active() {
                s.run(&mut out);
            }
            let want = np.hammer.contact_time_rigid(v);
            let got = s.contact_time();
            assert!((got - want).abs() < 2.0 / 48_000.0 + 0.02 * want, "v {v}: {got} vs {want}");
        }
    }

    #[test]
    fn contact_shortens_and_force_grows_with_velocity() {
        let (bank, np) = setup(60, 1.0);
        let mut last: Option<(f64, f64)> = None;
        for v in [0.5, 1.0, 2.0, 4.0] {
            let mut s = ContactSolver::new(512, 48_000.0);
            s.strike(&bank, &np, v, 1);
            let mut out = [0.0f32; 64];
            while s.is_active() {
                s.run(&mut out);
            }
            if let Some((t, f)) = last {
                assert!(s.contact_time() <= t, "contact time grows with v");
                assert!(s.max_force() > f);
            }
            last = Some((s.contact_time(), s.max_force()));
        }
    }
}

