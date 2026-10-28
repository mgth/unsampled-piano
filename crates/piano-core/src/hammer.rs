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
//! The strings of a unison each press their own part of the felt: string s
//! sees the compression u_s = x − d_s − Σ_{k∈s} φ_k·q_k (d_s: how much lower
//! it lies than the highest) and takes F_s from a felt of σ_s of the
//! hammer's stiffness (σ_s its share, Σσ = 1). Each has its own ψ_s; the
//! hammer takes ΣF_s. The step is then a small linear system,
//! F_s(1 + a_s·Λ_s) + a_s·μ·ΣF = b_s (μ = Δt²/m), solved in closed form,
//! strings whose force would pull left out. Strings level with one another
//! give the single felt on their mean displacement exactly; strings not
//! quite level take pulses of different lengths and shapes, whose spectral
//! dips fall at different frequencies (docs/strike_shape.md).
//!
//! Energy: multiplying each equation by its centred difference gives
//! H[n+½] − H[n−½] = −Σ d_k(Δq_k)²/γ_k − η(Δu)² ≤ 0 with
//! H = Σ_k [s_k(q[n+1]² + q[n]²) − 2c_k·q[n+1]q[n]]/γ_k + (m/T²)(x[n+1] − x[n])² + Σ_s ψ_s[n+½]²,
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
    /// Time (s) after the start of the strike at which the hammer, at its
    /// speed, would reach this string (a unison's strings do not lie at one
    /// height): the string lies delay·v below the highest.
    pub delay: f64,
    /// The string of the unison it belongs to (0..3): its own contact.
    pub string: u8,
    /// Share of its bridge force that reaches the output, as in the modal
    /// bank: 1 for a vertical oscillator, `polarization.radiation` for a
    /// horizontal one.
    pub radiation: f64,
}

/// Separation time after which the hammer is considered gone (re-contacts
/// before that are simulated: they happen in the treble and the bass).
const RELEASE_AFTER_S: f64 = 0.004;
/// Hard limit of a contact episode.
const MAX_CONTACT_S: f64 = 0.03;

/// Modes per chunk of the update loop (f64 lanes of an AVX-512 register).
const CL: usize = 8;
/// Strings of a unison, each with its own contact.
pub const NS: usize = 3;

/// Per-mode data of [`CL`] modes, in lanes: the update loop vectorises
/// across modes. Padding lanes are all zero and stay zero.
#[derive(Clone, Copy, Default)]
#[repr(C, align(64))]
struct Lanes {
    c: [f64; CL],
    e: [f64; CL],
    /// Per string: γφ and φ of the modes on that string, 0 elsewhere.
    gphi: [[f64; CL]; NS],
    phi: [[f64; CL]; NS],
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
    /// Per string, Σ φ·pred: its displacement under the hammer if no force
    /// acted.
    w_star: [f64; NS],
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
    /// Per string: its share σ of the felt (0: not struck), how much lower
    /// it lies than the highest (m), and Σφ²γ of its modes.
    sigma: [f64; NS],
    offset: [f64; NS],
    lambda: [f64; NS],
    /// Residual (static) correction of the truncated modal string: per
    /// string, the compliance under the hammer of the modes left out (m/N);
    /// the share of the hammer force that reaches the bridge and the agraffe
    /// quasi-statically through them; and the compliance sum per unit
    /// weight, kept between strikes.
    c_res: [f64; NS],
    c_tail: f64,
    r_bridge: f64,
    r_agraffe: f64,
    /// What the residual was computed for (a voice keeps its key: the sums
    /// are redone only when the instrument changes).
    residual_key: [f64; 8],
    /// Contact force of each string at the last two steps (N).
    f0: [f64; NS],
    f1: [f64; NS],
    x0: f64,
    x1: f64,
    /// Flexible shank: its position now and one step earlier (m), effective
    /// mass, stiffness and damping to the head (0 mass: rigid hammer).
    y0: f64,
    y1: f64,
    shank_mass: f64,
    shank_k: f64,
    shank_c: f64,
    w0: [f64; NS],
    w1: [f64; NS],
    psi: [f64; NS],
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
            w_star: [0.0; NS],
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
            sigma: [0.0; NS],
            offset: [0.0; NS],
            lambda: [0.0; NS],
            c_res: [0.0; NS],
            c_tail: 0.0,
            r_bridge: 0.0,
            r_agraffe: 0.0,
            residual_key: [f64::NAN; 8],
            f0: [0.0; NS],
            f1: [0.0; NS],
            x0: 0.0,
            x1: 0.0,
            y0: 0.0,
            y1: 0.0,
            shank_mass: 0.0,
            shank_k: 0.0,
            shank_c: 0.0,
            w0: [0.0; NS],
            w1: [0.0; NS],
            psi: [0.0; NS],
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

    /// Contact force at the last step (N), all strings.
    pub fn force(&self) -> f64 {
        self.f0.iter().sum()
    }

    /// Contact force of each string at the last step (N).
    pub fn string_forces(&self) -> [f64; NS] {
        self.f0
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
        let mut lambda = [0.0; NS];
        let mut w0 = [0.0; NS];
        let mut w1 = [0.0; NS];
        let mut n = 0;
        // The strings of a unison lie side by side under one felt, each
        // pressing its own part: string s takes σ_s of the felt, its share
        // of the force `hit` (summed over its oscillators, the horizontal
        // one included), and its compression is the felt's travel past its
        // mean displacement, weighted alike. Strings at one height then make
        // the single felt on the unison's mean displacement, in parallel
        // (summing the displacements put them in series, the hammer seeing
        // them N² times too compliant). A string lower than the highest by
        // d meets the felt when it has travelled d.
        // (Modes come ordered by partial: the first ones are partial 1. No
        // allocation: the iterator is walked twice.)
        let modes = modes.into_iter();
        let share: f64 = modes.clone().take_while(|m| m.k == 1).map(|m| m.hit).sum::<f64>().max(1e-9);
        let (mut sigma, mut sq, mut offset) = ([0.0; NS], [0.0; NS], [f64::INFINITY; NS]);
        for m in modes.clone().take_while(|m| m.k == 1) {
            let s = (m.string as usize).min(NS - 1);
            sigma[s] += m.hit / share;
            sq[s] += (m.hit / share).powi(2);
            offset[s] = offset[s].min(m.delay.max(0.0) * v);
        }
        for s in 0..NS {
            if sigma[s] <= 1e-12 {
                (sigma[s], offset[s]) = (0.0, 0.0);
            }
        }
        let mut k_max = 0;
        for m in modes.take(self.ps.len()) {
            let i = n;
            n += 1;
            k_max = k_max.max(m.k);
            let ln = &mut self.lanes[i / CL];
            let l = i % CL;
            let k = m.k as f64;
            let s = (m.string as usize).min(NS - 1);
            let r = (-m.sigma * dt).exp();
            let th = std::f64::consts::TAU * m.freq * dt;
            let (sn, cs) = th.sin_cos();
            let gamma = dt * r * sn / (modal_mass * std::f64::consts::TAU * m.freq);
            // Mode shape at the strike point, averaged over the felt width,
            // weighted within its string. The bridge is at x = 0 and the
            // hammer strikes `strike_pos`·L from the *agraffe* (x = L):
            // striking near the bridge would push it quasi-statically with
            // most of the hammer force (a thump).
            let x = k * half_w;
            let phi = if sigma[s] > 0.0 {
                m.hit / (share * sigma[s])
                    * (k * std::f64::consts::PI * (1.0 - note.strike_pos)).sin()
                    * if x > 1e-9 { x.sin() / x } else { 1.0 }
            } else {
                0.0
            };
            ln.c[l] = r * cs;
            self.ps[i] = r * sn;
            ln.e[l] = r * r;
            for j in 0..NS {
                ln.phi[j][l] = if j == s { phi } else { 0.0 };
                ln.gphi[j][l] = if j == s { gamma * phi } else { 0.0 };
            }
            self.inv_gamma[i] = 1.0 / gamma;
            // Bridge force T·∂y/∂x(0) = T·Σ (kπ/L)·q_k, weighted as the bank
            // weights it (a horizontal oscillator radiated at full weight
            // here: a ringing string's sideways motion jumped at each
            // re-strike, a click).
            ln.out_w[l] = m.radiation * note.tension * k * std::f64::consts::PI / note.length;
            ln.out_a[l] = if m.k % 2 == 0 { -ln.out_w[l] } else { ln.out_w[l] };
            lambda[s] += phi * phi * gamma;
            // Displacements now and one sub-step earlier from the complex amplitude.
            let pole = C64::new(r * cs, r * sn);
            ln.q0[l] = m.state.re;
            ln.q1[l] = (m.state / pole).re;
            ln.pred[l] = 2.0 * ln.c[l] * ln.q0[l] - ln.e[l] * ln.q1[l];
            w0[s] += phi * ln.q0[l];
            w1[s] += phi * ln.q1[l];
        }
        self.n = n;
        if n % CL != 0 {
            // Padding lanes of the last chunk: inert.
            let ln = &mut self.lanes[n / CL];
            for a in [&mut ln.c, &mut ln.e, &mut ln.out_w, &mut ln.out_a, &mut ln.q0, &mut ln.q1, &mut ln.pred] {
                a[n % CL..].fill(0.0);
            }
            for j in 0..NS {
                ln.phi[j][n % CL..].fill(0.0);
                ln.gphi[j][n % CL..].fill(0.0);
            }
        }
        self.w_star = update::<false>(&mut self.lanes[..n.div_ceil(CL)], [0.0; NS]).3;
        // The modes kept stop at k_max (a few in the top treble: four at C8
        // below 20 kHz). For an ideal string pushed at x_h = (1 − β)·L from
        // the bridge, the static deflection gives the bridge β of the force,
        // the agraffe 1 − β, and the hammer a compliance β(1 − β)·L/T; the
        // modal sums converge slowly (as 1/k for the end forces), so a few
        // modes give the bridge a fraction of its quasi-static push: 15 % at
        // C8, the knock of the top treble lost. The modes above k_max are
        // added here as their static response, felt width included.
        // Each left-out mode k answers statically with q = φ·F/(m·ω_k²).
        let key = [note.f0, note.b, note.length, note.tension, note.mu, note.strike_pos, note.contact_width, k_max as f64];
        if key != self.residual_key {
            self.residual_key = key;
            self.residual(note, k_max, half_w, modal_mass);
        }
        // Each string's oscillators weigh (hit/share)/σ in its compression.
        self.c_res = std::array::from_fn(|s| if sigma[s] > 0.0 { self.c_tail * sq[s] / (sigma[s] * sigma[s]) } else { 0.0 });
        self.f0 = [0.0; NS];
        self.f1 = [0.0; NS];

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
        self.sigma = sigma;
        self.offset = offset;
        self.lambda = lambda;
        // Hammer arrives at the first string it meets now, at speed v, its
        // shank unbent.
        let first = (0..NS).filter(|&s| sigma[s] > 0.0).map(|s| offset[s] + w0[s]).fold(f64::INFINITY, f64::min);
        let first = if first.is_finite() { first } else { 0.0 };
        self.x0 = first;
        self.x1 = first - v * dt;
        self.shank_mass = h.shank_mass;
        self.shank_k = h.shank_k;
        self.shank_c = h.shank_c;
        self.y0 = self.x0;
        self.y1 = self.x1;
        self.w0 = w0;
        self.w1 = w1;
        self.psi = [0.0; NS];
        self.active = true;
        self.samples = 0;
        self.separated = 0;
        self.max_force = 0.0;
        self.contact_samples = 0;
    }

    /// The residual sums of `strike` (see there), for modes above `k_max`.
    fn residual(&mut self, note: &NoteParams, k_max: u32, half_w: f64, modal_mass: f64) {
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
        self.c_tail = c_tail;
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

    fn step(&mut self) -> (f64, f64) {
        let dt = self.dt;
        let mu = dt * dt / self.mass;
        // The shank pulls the head with a force known at this step (explicit
        // central difference: stable while ω·dt < 2, ω some 10³ rad/s).
        let fs = if self.shank_mass > 0.0 {
            self.shank_k * (self.y0 - self.x0) + self.shank_c * ((self.y0 - self.y1) - (self.x0 - self.x1)) / dt
        } else {
            0.0
        };
        let x_star = 2.0 * self.x0 - self.x1 + dt * dt / self.mass * fs;

        // Per string: the compression if no force acted, now and one step
        // earlier (the residual modes deflect with the force at once), and
        // the quadratised felt law (a felt of σ of the stiffness: V_s = σ·V,
        // g_s = √σ·g) with its linearised hysteresis at u[n].
        let (mut u_star, mut u_prev, mut g, mut a, mut b, mut big) = ([0.0; NS], [0.0; NS], [0.0; NS], [0.0; NS], [0.0; NS], [0.0; NS]);
        let mut on = [false; NS];
        for s in 0..NS {
            let sg = self.sigma[s];
            if sg <= 0.0 {
                continue;
            }
            on[s] = true;
            let d = self.offset[s];
            u_star[s] = x_star - d - self.w_star[s];
            let u_now = self.x0 - d - self.w0[s] - self.c_res[s] * self.f0[s];
            u_prev[s] = self.x1 - d - self.w1[s] - self.c_res[s] * self.f1[s];
            // g = F/√(2V) (= √(Q(p+1)/2)·u^((p−1)/2) for a single power law),
            // hysteresis α·dF/dt linearised.
            let (gs, eta) = if u_now > 0.0 {
                let (v, f, k) = self.felt(u_now);
                let g = if v > 0.0 { f / (2.0 * v).sqrt() } else { 0.0 };
                (sg.sqrt() * g, sg * self.alpha * k / (2.0 * dt))
            } else {
                (0.0, 0.0)
            };
            g[s] = gs;
            a[s] = gs * gs / 4.0 + eta;
            b[s] = gs * self.psi[s] + a[s] * (u_star[s] - u_prev[s]);
            big[s] = self.lambda[s] + self.c_res[s];
        }
        // F_s·(1 + a_s·Λ_s) + a_s·μ·S = b_s, S = ΣF: S = Σ(b/(1+aΛ)) / (1 + μ·Σ(a/(1+aΛ))).
        // A string whose force would pull is left out (no pulling force) and
        // the rest solved again.
        let mut f = [0.0; NS];
        let mut total = 0.0;
        for _ in 0..NS {
            let (mut sb, mut sa) = (0.0, 0.0);
            for s in 0..NS {
                if on[s] {
                    let den = 1.0 + a[s] * big[s];
                    sb += b[s] / den;
                    sa += a[s] / den;
                }
            }
            total = sb / (1.0 + mu * sa);
            let mut again = false;
            for s in 0..NS {
                f[s] = if on[s] { (b[s] - a[s] * mu * total) / (1.0 + a[s] * big[s]) } else { 0.0 };
                if on[s] && f[s] < 0.0 {
                    on[s] = false;
                    again = true;
                }
            }
            if !again {
                break;
            }
        }
        if on.iter().all(|o| !o) {
            (f, total) = ([0.0; NS], 0.0);
        }
        for s in 0..NS {
            if self.sigma[s] <= 0.0 {
                continue;
            }
            let u_next = u_star[s] - big[s] * f[s] - mu * total;
            self.max_u = self.max_u.max(u_next);
            if on[s] {
                self.psi[s] += g[s] * (u_next - u_prev[s]) / 2.0;
            }
            if u_next <= 0.0 {
                self.psi[s] = 0.0;
            }
        }

        // Updates.
        let x_next = x_star - mu * total;
        if self.shank_mass > 0.0 {
            let y_next = 2.0 * self.y0 - self.y1 - dt * dt / self.shank_mass * fs;
            self.y1 = self.y0;
            self.y0 = y_next;
        }
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
        self.max_force = self.max_force.max(total);
        if total > 0.0 {
            self.contact_samples += 1;
            self.separated = 0;
        } else if self.contact_samples > 0 || self.samples > 1 {
            self.separated += 1;
        }
        if self.separated as f64 * dt > RELEASE_AFTER_S || self.samples as f64 * dt > MAX_CONTACT_S {
            self.active = false;
        }
        (out + self.r_bridge * total, out_a + self.r_agraffe * total)
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
        // The shank: its motion and the spring to the head, in the form the
        // explicit central difference conserves (the spring's energy taken
        // across two steps).
        let shank = if self.shank_mass > 0.0 {
            let vy = self.y0 - self.y1;
            self.shank_mass / (self.dt * self.dt) * vy * vy + self.shank_k * (self.y0 - self.x0) * (self.y1 - self.x1)
        } else {
            0.0
        };
        let felt: f64 = (0..NS).map(|s| self.psi[s] * self.psi[s] + self.c_res[s] * (self.f0[s] * self.f0[s] + self.f1[s] * self.f1[s]) / 2.0).sum();
        h + self.mass / (self.dt * self.dt) * v * v + felt + shank
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
fn update_avx2(lanes: &mut [Lanes], f: [f64; NS]) -> ([f64; NS], f64, f64, [f64; NS]) {
    update::<true>(lanes, f)
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx512f")]
fn update_avx512(lanes: &mut [Lanes], f: [f64; NS]) -> ([f64; NS], f64, f64, [f64; NS]) {
    update::<true>(lanes, f)
}

/// One pass over the modes: with `STEP`, q ← pred + γφ·F (each string's
/// force on its modes) and the next prediction; returns per string Σφq, the
/// bridge and agraffe forces, and per string Σφ·pred (the next step's free
/// displacement under the hammer). Lane sums are reduced in a fixed order:
/// the same result on every ISA.
#[inline(always)]
fn update<const STEP: bool>(lanes: &mut [Lanes], f: [f64; NS]) -> ([f64; NS], f64, f64, [f64; NS]) {
    let (mut w, mut o, mut oa, mut ws) = ([[0.0f64; CL]; NS], [0.0f64; CL], [0.0f64; CL], [[0.0f64; CL]; NS]);
    for ln in lanes {
        for l in 0..CL {
            let q = if STEP { ln.pred[l] + ln.gphi[0][l] * f[0] + ln.gphi[1][l] * f[1] + ln.gphi[2][l] * f[2] } else { ln.q0[l] };
            if STEP {
                let p = 2.0 * ln.c[l] * q - ln.e[l] * ln.q0[l];
                ln.q1[l] = ln.q0[l];
                ln.q0[l] = q;
                ln.pred[l] = p;
            }
            for s in 0..NS {
                w[s][l] += ln.phi[s][l] * q;
                ws[s][l] += ln.phi[s][l] * ln.pred[l];
            }
            o[l] += ln.out_w[l] * q;
            oa[l] += ln.out_a[l] * q;
        }
    }
    let sum = |a: [f64; CL]| ((a[0] + a[4]) + (a[1] + a[5])) + ((a[2] + a[6]) + (a[3] + a[7]));
    (w.map(sum), sum(o), sum(oa), ws.map(sum))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::PianoModel;

    fn setup(note: u8, sigma_scale: f64) -> (Vec<PhysMode>, NoteParams) {
        let sr = 48_000.0;
        // A rigid hammer of the whole mass (the flexible shank has its own test).
        let mut model = PianoModel::default();
        model.shank_share = 0.0;
        let np = model.note(note);
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
                    string: 0,
                    radiation: 1.0,
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
            if s.psi.iter().any(|&p| p > 0.0) {
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
    fn flexible_shank_energy_never_grows() {
        // With the shank (default model), lossless strings and felt: only
        // the shank's damping and the guards remove energy; the explicit
        // spring adds none beyond rounding.
        let mut model = PianoModel::default();
        model.shank_share = 0.3;
        model.shank_loss = 0.0;
        for note in [33u8, 60, 84, 100, 108] {
            for v in [0.5, 3.0, 7.0] {
                let (modes, _) = setup(note, 0.0);
                let mut np = model.note(note);
                np.hammer.alpha = 0.0;
                let mut s = ContactSolver::new(512, 48_000.0);
                s.strike(&np, v, 2, modes.iter().copied());
                let mut out = [0.0f32; 1];
                s.run(&mut out, None);
                let h0 = s.energy();
                while s.is_active() {
                    s.run(&mut out, None);
                    let h = s.energy();
                    assert!(h <= h0 * (1.0 + 1e-3), "note {note} v {v}: {h} > {h0}");
                    assert!(out[0].is_finite());
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
    /// A unison at heights `heights` (m) below the highest string, struck at
    /// `v`: each partial's modes, one per string.
    fn unison(note: u8, heights: &[f64], v: f64) -> (Vec<PhysMode>, NoteParams) {
        let (single, _) = setup(note, 1.0);
        let np = PianoModel::default().note(note);
        let modes = single
            .iter()
            .flat_map(|m| heights.iter().enumerate().map(move |(s, h)| PhysMode { delay: h / v, string: s as u8, ..*m }))
            .collect();
        (modes, np)
    }

    fn strike_out(np: &NoteParams, v: f64, modes: &[PhysMode]) -> (ContactSolver, Vec<f64>) {
        let mut s = ContactSolver::new(modes.len(), 48_000.0);
        s.strike(np, v, 4, modes.iter().copied());
        let mut out = [0.0f32; 1];
        let mut force = Vec::new();
        while s.is_active() && s.run(&mut out, None) == 1 {
            force.push(s.force());
        }
        (s, force)
    }

    #[test]
    fn level_strings_make_the_single_felt() {
        // Three strings at one height move as one string three times as
        // heavy and as tense (same modes) under the whole felt: each takes a
        // third of the force.
        for v in [0.5, 2.0] {
            let (three, np) = unison(84, &[0.0; 3], v);
            let (one, _) = unison(84, &[0.0], v);
            let mut np3 = np;
            np3.mu *= 3.0;
            np3.tension *= 3.0;
            let (s1, f1) = strike_out(&np3, v, &one);
            let (s3, f3) = strike_out(&np, v, &three);
            let peak = f1.iter().cloned().fold(0.0, f64::max);
            let n = f1.len().min(f3.len());
            for (a, b) in f1[..n].iter().zip(&f3[..n]) {
                assert!((a - b).abs() < 1e-9 * peak, "{a} vs {b}");
            }
            for i in 0..one.len() {
                let z = s1.complex_state(i);
                for j in 0..3 {
                    assert!((s3.complex_state(3 * i + j) - z).norm() < 1e-6 * (z.norm() + 1e-15), "mode {i}");
                }
            }
        }
    }

    #[test]
    fn uneven_strings_energy_never_grows() {
        for note in [45u8, 84, 100] {
            for v in [0.4, 1.5, 5.0] {
                let (modes, np) = unison(note, &[0.0, 0.36e-4, 1e-4], v);
                let mut s = ContactSolver::new(modes.len(), 48_000.0);
                s.strike(&np, v, 2, modes.iter().copied());
                let mut out = [0.0f32; 1];
                s.run(&mut out, None);
                let mut prev = s.energy();
                while s.is_active() {
                    s.run(&mut out, None);
                    let h = s.energy();
                    assert!(h <= prev * (1.0 + 1e-12), "note {note} v {v}: {h} > {prev}");
                    prev = h;
                }
            }
        }
    }

    /// Per partial 2–4 of `note`, the largest fall of its level (re partial
    /// 1, the strings summed) below its level at softer blows (dB), over
    /// 0.4–2.5 m/s; averaged.
    fn dips(note: u8, heights: &[f64]) -> f64 {
        let mut levels: Vec<[f64; 3]> = Vec::new();
        for j in 0..22 {
            let v = 0.4 * (2.5f64 / 0.4).powf(j as f64 / 21.0);
            let (modes, np) = unison(note, heights, v);
            let (s, _) = strike_out(&np, v, &modes);
            let n = heights.len();
            let partial = |k: usize| (0..n).map(|j| s.complex_state(k * n + j)).fold(C64::ZERO, |a, b| a + b).norm();
            levels.push(std::array::from_fn(|k| 20.0 * (partial(k + 1) / partial(0)).log10()));
        }
        let mut total = 0.0;
        for k in 0..3 {
            let (mut best, mut dip) = (f64::NEG_INFINITY, 0.0f64);
            for l in &levels {
                best = best.max(l[k]);
                dip = dip.max(best - l[k]);
            }
            total += dip;
        }
        total / 3.0
    }

    #[test]
    fn uneven_strings_fill_the_spectral_dips() {
        // A smooth, symmetric force pulse has spectral zeros that sweep
        // through the partials as the blow quickens: one partial fades at
        // some velocity (a hollow "plop" at C6 around velocity 50). The
        // single felt on the unison's mean displacement, with the lower
        // strings let in when the hammer reached them, kinked the pulse and
        // dug 15–25 dB dips at C6–C7 (real grands: 5–10). Each string on its
        // own contact keeps the pulse smooth and the dips shallower. C7
        // keeps 13 dB (Salamander: 8; 20 before).
        for (note, heights, most) in [(84u8, [0.0, 0.36e-4, 1e-4], 9.0), (90, [0.0, 1e-4, 0.86e-4], 9.0), (96, [0.0, 1e-4, 0.13e-4], 15.0)] {
            let d = dips(note, &heights);
            eprintln!("key {note}: partials 2-4, mean dip {d:.1} dB");
            assert!(d < most, "key {note}: {d:.1} dB");
        }
    }
}
