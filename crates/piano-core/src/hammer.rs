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
    /// Time (s) after the start of the strike at which the hammer reaches
    /// this string (a unison's strings do not lie at one height).
    pub delay: f64,
}

/// Separation time after which the hammer is considered gone (re-contacts
/// before that are simulated: they happen in the treble and the bass).
const RELEASE_AFTER_S: f64 = 0.004;
/// Hard limit of a contact episode.
const MAX_CONTACT_S: f64 = 0.03;

/// Modes per chunk of the update loop (f64 lanes of an AVX-512 register).
const CL: usize = 8;

/// Per-mode data of [`CL`] modes, in lanes: the update loop vectorises
/// across modes. Padding lanes are all zero and stay zero.
#[derive(Clone, Copy, Default)]
#[repr(C, align(64))]
struct Lanes {
    c: [f64; CL],
    e: [f64; CL],
    gphi: [f64; CL],
    phi: [f64; CL],
    /// Output weights (bridge force).
    out_w: [f64; CL],
    /// Force on the agraffe end, −T·∂y/∂x(L) = T·Σ (kπ/L)·(−1)^{k+1}·q_k
    /// (k from 1: the fundamental pulls both ends the same way).
    out_a: [f64; CL],
    q0: [f64; CL],
    q1: [f64; CL],
    /// Free prediction of the next displacement, 2c·q0 − e·q1.
    pred: [f64; CL],
}

/// Terms of the residual sums above the kept modes (the felt width's sinc
/// makes them converge as 1/k²).
const RESIDUAL_TERMS: u32 = 400;

pub struct ContactSolver {
    lanes: Vec<Lanes>,
    // Per mode, outside the update loop.
    ps: Vec<f64>,
    /// Inverse of γ_k (energy weights).
    inv_gamma: Vec<f64>,
    n: usize,
    /// Σ φ·pred: string displacement under the hammer if no force acted.
    w_star: f64,
    isa: crate::modal::Isa,
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
    /// Residual (static) correction of the truncated modal string: the
    /// compliance under the hammer of the modes left out (m/N), and the
    /// share of the hammer force that reaches the bridge and the agraffe
    /// quasi-statically through them.
    c_res: f64,
    r_bridge: f64,
    r_agraffe: f64,
    /// What the residual was computed for (a voice keeps its key: the sums
    /// are redone only when the instrument changes).
    residual_key: [f64; 8],
    /// Strings the hammer has not reached yet: per mode, the step at which
    /// it does (0: from the start), its weight φ and γ; and the next such
    /// step (usize::MAX: none left).
    gate: Vec<usize>,
    gate_phi: Vec<f64>,
    gate_gamma: Vec<f64>,
    next_gate: usize,
    /// Contact force at the last two steps (N).
    f0: f64,
    f1: f64,
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
        Self {
            lanes: vec![Lanes::default(); max_modes.div_ceil(CL)],
            ps: vec![0.0; max_modes],
            inv_gamma: vec![0.0; max_modes],
            n: 0,
            w_star: 0.0,
            isa: crate::modal::Isa::detect(),
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
            c_res: 0.0,
            r_bridge: 0.0,
            r_agraffe: 0.0,
            residual_key: [f64::NAN; 8],
            gate: vec![0; max_modes],
            gate_phi: vec![0.0; max_modes],
            gate_gamma: vec![0.0; max_modes],
            next_gate: usize::MAX,
            f0: 0.0,
            f1: 0.0,
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
        let shares2: f64 = modes.clone().take_while(|m| m.k == 1).map(|m| (m.hit / share).powi(2)).sum();
        let mut k_max = 0;
        self.next_gate = usize::MAX;
        for m in modes.take(self.ps.len()) {
            let i = n;
            n += 1;
            k_max = k_max.max(m.k);
            let ln = &mut self.lanes[i / CL];
            let l = i % CL;
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
            ln.c[l] = r * cs;
            self.ps[i] = r * sn;
            ln.e[l] = r * r;
            // A string the hammer reaches later takes no force until then;
            // at rest, it joins without moving the felt (w unchanged).
            let gate = (m.delay.max(0.0) / dt).round() as usize;
            self.gate[i] = gate;
            self.gate_phi[i] = phi;
            self.gate_gamma[i] = gamma;
            let phi = if gate > 0 {
                self.next_gate = self.next_gate.min(gate);
                0.0
            } else {
                phi
            };
            ln.phi[l] = phi;
            ln.gphi[l] = gamma * phi;
            self.inv_gamma[i] = 1.0 / gamma;
            // Bridge force T·∂y/∂x(0) = T·Σ (kπ/L)·q_k.
            ln.out_w[l] = note.tension * k * std::f64::consts::PI / note.length;
            ln.out_a[l] = if m.k % 2 == 0 { -ln.out_w[l] } else { ln.out_w[l] };
            lambda += phi * phi * gamma;
            // Displacements now and one sub-step earlier from the complex amplitude.
            let pole = C64::new(r * cs, r * sn);
            ln.q0[l] = m.state.re;
            ln.q1[l] = (m.state / pole).re;
            ln.pred[l] = 2.0 * ln.c[l] * ln.q0[l] - ln.e[l] * ln.q1[l];
            w0 += phi * ln.q0[l];
            w1 += phi * ln.q1[l];
        }
        self.n = n;
        if n % CL != 0 {
            // Padding lanes of the last chunk: inert.
            let ln = &mut self.lanes[n / CL];
            for a in [&mut ln.c, &mut ln.e, &mut ln.gphi, &mut ln.phi, &mut ln.out_w, &mut ln.out_a, &mut ln.q0, &mut ln.q1, &mut ln.pred] {
                a[n % CL..].fill(0.0);
            }
        }
        self.w_star = update::<false>(&mut self.lanes[..n.div_ceil(CL)], 0.0).3;
        // The modes kept stop at k_max (a few in the top treble: four at C8
        // below 20 kHz). For an ideal string pushed at x_h = (1 − β)·L from
        // the bridge, the static deflection gives the bridge β of the force,
        // the agraffe 1 − β, and the hammer a compliance β(1 − β)·L/T; the
        // modal sums converge slowly (as 1/k for the end forces), so a few
        // modes give the bridge a fraction of its quasi-static push: 15 % at
        // C8, the knock of the top treble lost. The modes above k_max are
        // added here as their static response, felt width included.
        // Each left-out mode k answers statically with q = φ·F/(m·ω_k²).
        let key = [note.f0, note.b, note.length, note.tension, note.mu, note.strike_pos, note.contact_width, k_max as f64 + shares2 * 1e3];
        if key != self.residual_key {
            self.residual_key = key;
            self.residual(note, k_max, shares2, half_w, modal_mass);
        }
        self.f0 = 0.0;
        self.f1 = 0.0;

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
        self.lambda = lambda + dt * dt / self.mass + self.c_res;
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

    /// The residual sums of `strike` (see there), for modes above `k_max`.
    fn residual(&mut self, note: &NoteParams, k_max: u32, shares2: f64, half_w: f64, modal_mass: f64) {
        let (mut c_tail, mut b_tail, mut a_tail) = (0.0, 0.0, 0.0);
        for k in k_max + 1..=k_max + RESIDUAL_TERMS {
            let kf = k as f64;
            let x = kf * half_w;
            let sinc = if x > 1e-9 { x.sin() / x } else { 1.0 };
            let phi = (kf * std::f64::consts::PI * (1.0 - note.strike_pos)).sin() * sinc;
            let w = std::f64::consts::TAU * crate::params::partial_freq(note.f0, note.b, k);
            let q = phi / (modal_mass * w * w);
            let out_w = note.tension * kf * std::f64::consts::PI / note.length;
            c_tail += phi * q;
            b_tail += out_w * q;
            a_tail += if k % 2 == 0 { -out_w * q } else { out_w * q };
        }
        self.c_res = shares2 * c_tail;
        self.r_bridge = b_tail;
        self.r_agraffe = a_tail;
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
        let (ln, l) = (&self.lanes[i / CL], i % CL);
        let (pr, pi, r2) = (ln.c[l], self.ps[i], ln.e[l]);
        let a = ln.q0[l];
        C64::new(a, (r2 * ln.q1[l] - a * pr) / pi)
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

    /// The hammer reaches the strings due at this step: their modes start
    /// taking the force (the felt sees their displacement from now on).
    fn open_gates(&mut self) {
        let step = self.samples;
        let mut next = usize::MAX;
        for i in 0..self.n {
            let g = self.gate[i];
            if g == step {
                let (ln, l) = (&mut self.lanes[i / CL], i % CL);
                let (phi, gamma) = (self.gate_phi[i], self.gate_gamma[i]);
                ln.phi[l] = phi;
                ln.gphi[l] = gamma * phi;
                self.lambda += phi * phi * gamma;
                self.w0 += phi * ln.q0[l];
                self.w1 += phi * ln.q1[l];
                self.w_star += phi * ln.pred[l];
            } else if g > step {
                next = next.min(g);
            }
        }
        self.next_gate = next;
    }

    fn step(&mut self) -> (f64, f64) {
        if self.samples == self.next_gate {
            self.open_gates();
        }
        let dt = self.dt;
        // String displacement under the hammer from the free predictions.
        let w_star = self.w_star;
        let x_star = 2.0 * self.x0 - self.x1;
        let u_star = x_star - w_star;
        // The residual modes deflect with the force at once (no inertia).
        let u_now = self.x0 - self.w0 - self.c_res * self.f0;
        let u_prev = self.x1 - self.w1 - self.c_res * self.f1;

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
        let lanes = &mut self.lanes[..self.n.div_ceil(CL)];
        let (w_next, out, out_a, w_star) = match self.isa {
            crate::modal::Isa::Generic => update::<true>(lanes, f),
            // SAFETY: only selected when detected.
            #[cfg(target_arch = "x86_64")]
            crate::modal::Isa::Avx2 => unsafe { update_avx2(lanes, f) },
            #[cfg(target_arch = "x86_64")]
            crate::modal::Isa::Avx512 => unsafe { update_avx512(lanes, f) },
        };
        self.w_star = w_star;
        self.x1 = self.x0;
        self.x0 = x_next;
        self.w1 = self.w0;
        self.w0 = w_next;
        self.f1 = self.f0;
        self.f0 = f;

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
        (out + self.r_bridge * f, out_a + self.r_agraffe * f)
    }

    /// Discrete energy H[n+½] (twice the physical energy), for checks.
    pub fn energy(&self) -> f64 {
        let mut h = 0.0;
        for i in 0..self.n {
            let (ln, l) = (&self.lanes[i / CL], i % CL);
            let s = (1.0 + ln.e[l]) / 2.0;
            h += (s * (ln.q0[l] * ln.q0[l] + ln.q1[l] * ln.q1[l]) - 2.0 * ln.c[l] * ln.q0[l] * ln.q1[l]) * self.inv_gamma[i];
        }
        let v = self.x0 - self.x1;
        // The residual modes deflect by ρ[n] = c·F[n−1] (the solve lags them
        // one step): the work F[n]·(ρ[n+1] − ρ[n−1])/2 is the change of
        // c·(F[n]² + F[n−1]²)/4 plus c·(F[n] − F[n−2])²/4 ≥ 0, dissipated.
        // Twice that stored energy:
        h + self.mass / (self.dt * self.dt) * v * v + self.psi * self.psi + self.c_res * (self.f0 * self.f0 + self.f1 * self.f1) / 2.0
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
fn update_avx2(lanes: &mut [Lanes], f: f64) -> (f64, f64, f64, f64) {
    update::<true>(lanes, f)
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx512f")]
fn update_avx512(lanes: &mut [Lanes], f: f64) -> (f64, f64, f64, f64) {
    update::<true>(lanes, f)
}

/// One pass over the modes: with `STEP`, q ← pred + γφ·F and the next
/// prediction; returns Σφq, the bridge and agraffe forces, and Σφ·pred
/// (the next step's free displacement under the hammer). Lane sums are
/// reduced in a fixed order: the same result on every ISA.
#[inline(always)]
fn update<const STEP: bool>(lanes: &mut [Lanes], f: f64) -> (f64, f64, f64, f64) {
    let (mut w, mut o, mut oa, mut ws) = ([0.0f64; CL], [0.0f64; CL], [0.0f64; CL], [0.0f64; CL]);
    for ln in lanes {
        for l in 0..CL {
            let q = if STEP { ln.pred[l] + ln.gphi[l] * f } else { ln.q0[l] };
            if STEP {
                let p = 2.0 * ln.c[l] * q - ln.e[l] * ln.q0[l];
                ln.q1[l] = ln.q0[l];
                ln.q0[l] = q;
                ln.pred[l] = p;
            }
            w[l] += ln.phi[l] * q;
            o[l] += ln.out_w[l] * q;
            oa[l] += ln.out_a[l] * q;
            ws[l] += ln.phi[l] * ln.pred[l];
        }
    }
    let sum = |a: [f64; CL]| ((a[0] + a[4]) + (a[1] + a[5])) + ((a[2] + a[6]) + (a[3] + a[7]));
    (sum(w), sum(o), sum(oa), sum(ws))
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
                    delay: 0.0,
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
        // Exact while in contact but for the residual modes' one-step lag,
        // which only dissipates (a few 10⁻⁵ over this contact); the separation guard may then
        // remove energy, and never adds any.
        let mut prev = h0;
        while s.is_active() {
            s.run(&mut out, None);
            let h = s.energy();
            if s.psi > 0.0 {
                assert!((h - h0).abs() / h0 < 1e-3, "drift in contact {:e}", (h - h0) / h0);
            }
            assert!(h <= prev * (1.0 + 1e-12));
            prev = h;
        }
        assert!(s.contact_time() > 1e-4, "no contact");
        assert!((prev - h0).abs() / h0 < 1e-3, "guards removed too much: {:e}", (prev - h0) / h0);
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

