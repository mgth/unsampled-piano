//! One key: its strings coupled at the bridge, the hammer, the damper.
//!
//! The [`ModalBank`] runs the coupled modes of the key's [`NoteModel`]
//! (index `k_idx·n + m`) with complex output weights = bridge force. While
//! the hammer touches the strings, the [`ContactSolver`] runs the physical
//! oscillators (index `k_idx·n + j`, uncoupled during these milliseconds)
//! and hands their complex amplitudes back through Vᵀ.
//!
//! Sympathy (one-way, after Bank, Zambon & Fontana 2010): the strings are
//! linear, so their motion is the response to the hammer (primary bank,
//! whose own bridge coupling is in its coupled modes) plus the response to
//! the bridge motion caused by the *other* keys (secondary bank, same modes,
//! driven by the bridge displacement minus the key's own contribution).
//! Only primary forces move the bridge for the strings: no feedback loop,
//! stable for any gain; both parts radiate.

use crate::complex::C64;
use crate::hammer::{ContactSolver, PhysMode};
use crate::modal::ModalBank;
use crate::note_model::NoteModel;
use crate::unison::MAX_OSC;

pub const MAX_MODES: usize = 768;

/// The practice mute's felt strip: a loose woven felt, far softer than the
/// hammer's pressed felt. In series with it, a strip of t mm leaves the
/// hammer MUTE_STIFFNESS_MM/t of its Q0 (3 mm: 0.03), and is lossier
/// (hysteresis factor).
const MUTE_STIFFNESS_MM: f64 = 0.09;
const MUTE_HYSTERESIS: f64 = 3.0;

pub struct Voice {
    pub note: u8,
    pub active: bool,
    pub key_down: bool,
    /// Held up by the sostenuto pedal.
    pub sostenuto: bool,
    /// Has been struck since it was set up (else it only listens).
    pub struck: bool,
    /// Damper contact, 0 lifted … 1 resting on the strings.
    pub engagement: f64,
    /// Speed factor of the next damper landing (from release velocity or
    /// pedal speed), 1 at mid release velocity.
    pub damper_speed: f64,
    /// Time the last blow took to cross the unison's height spread (s).
    strike_dt: f64,
    /// Response to the hammer.
    pub bank: ModalBank,
    /// Response to the bridge motion from the other keys.
    pub sym: ModalBank,
    pub contact: ContactSolver,
    sym_force: Vec<f32>,
    agraffe: Vec<f32>,
    /// Own bridge force: this sub-block, and its history (ring indexed by
    /// absolute sample) to cancel its own contribution D samples later.
    force: Vec<f32>,
    force_ring: Vec<f32>,
    x2: Vec<f32>,
    /// Geometric nonlinearity: complex displacement of each partial (mean
    /// over the strings), its slope this sample, longitudinal mode states.
    nl_re: Vec<f32>,
    nl_im: Vec<f32>,
    nl_a: Vec<f32>,
    nl_long: Vec<(f32, f32)>,
    /// Samples since the hammer left the strings, for the onset ramp.
    nl_age: u32,
    nl_ramp: u32,
    /// High-pass state (previous input, output).
    nl_hp: (f32, f32),
    /// Below this bridge-force energy (N²) a sympathetic bank at rest is
    /// left alone; below this tension modulation (N, times the coupling)
    /// the longitudinal modes are not computed ([`crate::engine::Shortcuts`]).
    sym_silence: f32,
    nl_silence: f32,
}


/// Ring length for the delayed signals (power of two ≥ 2·SUB_BLOCK).
pub const RING: usize = 1024;

impl Voice {
    pub fn new(sample_rate: f64, max_block: usize) -> Self {
        Self::with_capacity(sample_rate, max_block, (MAX_MODES, 48, 16))
    }

    /// A voice holding `modes` coupled modes and a phantom-partial model of
    /// `nl.0` partials and `nl.1` longitudinal modes
    /// ([`crate::note_model::PianoTable::capacity`]).
    pub fn with_capacity(sample_rate: f64, max_block: usize, (modes, nl_partials, nl_long): (usize, usize, usize)) -> Self {
        Self {
            note: 0,
            active: false,
            key_down: false,
            sostenuto: false,
            struck: false,
            engagement: 1.0,
            damper_speed: 1.0,
            strike_dt: 0.0,
            bank: ModalBank::new(sample_rate, modes, max_block),
            sym: ModalBank::new(sample_rate, modes, max_block),
            contact: ContactSolver::new(modes, sample_rate),
            sym_force: vec![0.0; max_block],
            agraffe: vec![0.0; max_block],
            force: vec![0.0; max_block],
            force_ring: vec![0.0; RING],
            x2: vec![0.0; max_block],
            nl_re: vec![0.0; nl_partials],
            nl_im: vec![0.0; nl_partials],
            nl_a: vec![0.0; nl_partials],
            nl_long: vec![(0.0, 0.0); nl_long],
            nl_age: 0,
            // The stretching builds up while the hammer pushes: 2 ms ramp
            // from the hand-over (a step would ring the high longitudinal
            // modes like a click).
            nl_ramp: (0.002 * sample_rate) as u32,
            nl_hp: (0.0, 0.0),
            sym_silence: crate::engine::Shortcuts::DEFAULT.sym_silence,
            nl_silence: crate::engine::Shortcuts::DEFAULT.nl_silence,
        }
    }

    pub fn set_shortcuts(&mut self, s: &crate::engine::Shortcuts) {
        self.sym_silence = s.sym_silence;
        self.nl_silence = s.nl_silence;
    }

    /// Prepares the strings of a key at rest; `listen_only` runs only the
    /// partials needed for sympathetic resonance. Copies the key's
    /// precomputed coefficients ([`NoteModel::table`]). Real-time safe.
    pub fn setup(&mut self, nm: &NoteModel, listen_only: bool) {
        self.note = nm.note;
        self.bank.load(&nm.table);
        if listen_only {
            self.bank.set_active_len(0);
        }
        self.bank.reset();
        self.sym.load(&nm.sym_table);
        self.sym.set_active_len(nm.n_listen);
        self.sym.reset();
        self.force.fill(0.0);
        self.force_ring.fill(0.0);
        self.nl_long.fill((0.0, 0.0));
        self.nl_hp = (0.0, 0.0);
        self.struck = false;
        self.sostenuto = false;
    }

    /// Strikes the (possibly ringing) strings with the hammer at `v` m/s.
    /// `una_corda` ∈ [0, 1] shifts the hammer off the first string and onto
    /// felt the strings have not grooved: softer (`softening` at full
    /// shift) and less compacted, so thicker over the hard core (by half the
    /// softening: +30 % at 0.6). `mute` (mm) is the felt strip of an
    /// upright's practice pedal between hammer and strings: the hammer
    /// strikes through a soft, lossy layer before its own felt.
    #[allow(clippy::too_many_arguments)]
    pub fn strike(&mut self, nm: &NoteModel, v: f64, oversample: usize, una_corda: f64, softening: f64, mute: f64) {
        if self.contact.is_active() {
            self.hand_over(nm);
        }
        self.bank.set_active_len(nm.modes.len());
        self.struck = true;
        let n = nm.n_osc;
        let ns = nm.params.strings;
        let shift = if ns >= 2 { una_corda.clamp(0.0, 1.0) } else { 0.0 };
        let mut np = nm.params;
        let u = softening * una_corda.clamp(0.0, 1.0);
        np.hammer.q0 *= 1.0 - u;
        // The fresh felt beside the grooves is softer throughout: the inner
        // felt too (the graded felt has no depth to push back).
        np.hammer.core_q *= 1.0 - u;
        np.hammer.core_depth *= 1.0 + 0.5 * u;
        if mute > 0.0 {
            np.hammer.q0 *= (MUTE_STIFFNESS_MM / mute).min(1.0);
            np.hammer.alpha *= MUTE_HYSTERESIS;
            np.hammer.core_depth += 1e-3 * mute;
        }
        let spread = nm.unison.spread_at(v);
        self.strike_dt = nm.unison.height_spread / v.max(0.05);
        let bank = &self.bank;
        let modes = nm.clusters.iter().enumerate().flat_map(move |(ki, cl)| {
            let mut c = [C64::ZERO; MAX_OSC];
            for (m, cm) in c.iter_mut().enumerate().take(n) {
                let (re, im) = bank.state(ki * n + m);
                *cm = C64::new(re, im);
            }
            let zeta = cl.to_strings(&c);
            (0..n).map(move |j| {
                let p = nm.phys[ki * n + j];
                let hit = p.hit * (1.0 + spread * p.uneven);
                let hit = if j % ns == 0 { hit * (1.0 - shift) } else { hit };
                PhysMode { k: p.k, freq: p.freq, sigma: p.sigma, state: zeta[j], hit }
            })
        });
        self.contact.strike(&np, v, oversample, modes);
    }

    fn hand_over(&mut self, nm: &NoteModel) {
        self.nl_age = 0;
        let n = nm.n_osc;
        let ns = nm.params.strings;
        // The strings of a unison do not lie at one height: the hammer meets
        // each a little later than the highest, and each then rings that
        // much later (a phase lag growing with frequency). The contact solve
        // strikes them together; the lag is applied as it hands over.
        let h = nm.unison.heights(nm.note);
        let lag: [f64; 3] = std::array::from_fn(|j| h[j] * self.strike_dt);
        for (ki, cl) in nm.clusters.iter().enumerate() {
            let zeta: [C64; MAX_OSC] = std::array::from_fn(|j| {
                if j < n {
                    let w = std::f64::consts::TAU * nm.phys[ki * n + j].freq;
                    self.contact.complex_state(ki * n + j) * C64::polar(1.0, -w * lag[j % ns])
                } else {
                    C64::ZERO
                }
            });
            let c = cl.to_modes(&zeta);
            for (m, cm) in c.iter().enumerate().take(n) {
                self.bank.set_state(ki * n + m, cm.re, cm.im);
            }
        }
    }

    /// Renders samples `from..to` of the sub-block starting at absolute
    /// sample `t0`: adds the primary bridge force to `out`, the sympathetic
    /// one to `out_sym` and the force on the agraffes to `out_agraffe`. `bridge` is the bridge displacement
    /// (from the other keys' primary forces) at the region point, `delay`
    /// samples earlier, when sympathy is on.
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        nm: &NoteModel,
        out: &mut [f32],
        out_sym: &mut [f32],
        out_agraffe: Option<&mut [f32]>,
        t0: u64,
        from: usize,
        to: usize,
        bridge: Option<(&[f32], usize)>,
    ) {
        self.compute(nm, out_agraffe.is_some(), t0, from, to, bridge);
        self.mix(out, out_sym, out_agraffe, from, to);
    }

    /// The voice's part of [`Self::render`]: its forces for `from..to`,
    /// kept until [`Self::mix`]. Touches nothing outside the voice, so
    /// voices can compute in parallel.
    pub fn compute(&mut self, nm: &NoteModel, with_agraffe: bool, t0: u64, from: usize, to: usize, bridge: Option<(&[f32], usize)>) {
        self.force[from..to].fill(0.0);
        let n = to - from;
        self.agraffe[..n].fill(0.0);
        let mut start = 0;
        if self.contact.is_active() {
            let agraffe = with_agraffe.then_some(&mut self.agraffe[..n]);
            start = self.contact.run(&mut self.force[from..to], agraffe);
            if !self.contact.is_active() {
                self.hand_over(nm);
            }
        }
        // Partial displacements at the start of the free segment (the bank
        // states), for the longitudinal modes.
        let nl = nm.longitudinal.as_ref().filter(|_| self.struck && start < n);
        let nl_on = nl.is_some_and(|lg| self.nl_start(lg, nm.n_osc));
        if start < n && !self.bank.is_empty() {
            if with_agraffe {
                self.bank.process2(None, &mut self.force[from + start..to], &mut self.agraffe[start..n]);
            } else {
                self.bank.process(None, &mut self.force[from + start..to]);
            }
        }
        if let (true, Some(lg)) = (nl_on, nl) {
            self.nl_run(lg, from + start, to);
        }
        for (i, f) in self.force[from..to].iter().enumerate() {
            self.force_ring[((t0 + (from + i) as u64) as usize) & (RING - 1)] = *f;
        }
        let f = &mut self.sym_force[..n];
        f.fill(0.0);
        match bridge {
            Some((u, delay)) => {
                // Own primary force `delay` samples earlier, to cancel it in u.
                for i in 0..n {
                    let t = t0 + (from + i) as u64;
                    self.x2[i] = self.force_ring[(t.wrapping_sub(delay as u64) as usize) & (RING - 1)];
                }
                // A string at rest under a bridge at rest (the board asleep):
                // nothing to compute.
                let still = u.iter().chain(&self.x2[..n]).all(|&x| x == 0.0);
                if !(still && self.sym.energy() < self.sym_silence) {
                    self.sym.process_two_inputs(u, &self.x2[..n], f);
                }
            }
            // Damped: the sympathetic part still decays freely.
            None if self.sym.energy() > self.sym_silence => self.sym.process(None, f),
            None => {}
        }
    }

    /// Adds the forces of the last [`Self::compute`] to the region sums.
    pub fn mix(&self, out: &mut [f32], out_sym: &mut [f32], out_agraffe: Option<&mut [f32]>, from: usize, to: usize) {
        let n = to - from;
        if let Some(out_agraffe) = out_agraffe {
            for (o, a) in out_agraffe.iter_mut().zip(&self.agraffe[..n]) {
                *o += a;
            }
        }
        for (o, f) in out.iter_mut().zip(&self.force[from..to]) {
            *o += f;
        }
        for (o, x) in out_sym.iter_mut().zip(&self.sym_force[..n]) {
            *o += x;
        }
    }

    /// Reads each partial's mean vertical displacement from the bank; false
    /// (and the longitudinal modes cleared) when the stretching is too small
    /// to matter.
    fn nl_start(&mut self, lg: &crate::note_model::Longitudinal, n_osc: usize) -> bool {
        let np = lg.slope.len();
        let mut s2 = 0.0f32;
        for k in 0..np {
            let (mut re, mut im) = (0.0f32, 0.0f32);
            for m in 0..n_osc {
                let i = k * n_osc + m;
                let (zr, zi) = self.bank.state(i);
                let (wr, wi) = lg.weights[i];
                re += wr * zr as f32 - wi * zi as f32;
                im += wr * zi as f32 + wi * zr as f32;
            }
            self.nl_re[k] = re;
            self.nl_im[k] = im;
            let a = lg.slope[k];
            s2 += a * a * (re * re + im * im);
        }
        let long_e: f32 = self.nl_long[..lg.long_pole.len()].iter().map(|z| z.0 * z.0 + z.1 * z.1).sum();
        // Estimated board force: coupling · tension modulation (N).
        if lg.coupling * lg.ea4 * s2 < self.nl_silence && long_e < 1e-20 * self.nl_silence {
            self.nl_long.fill((0.0, 0.0));
            return false;
        }
        true
    }

    /// Adds the longitudinal force on the bridge to `force[a..b]`, advancing
    /// the partial displacements freely from the segment's start.
    fn nl_run(&mut self, lg: &crate::note_model::Longitudinal, a: usize, b: usize) {
        let np = lg.slope.len();
        let nl = lg.long_pole.len();
        for t in a..b {
            // Structure of arrays: vectorises.
            let (re, im, a) = (&mut self.nl_re[..np], &mut self.nl_im[..np], &mut self.nl_a[..np]);
            let (pr, pi, sl) = (&lg.pole_re[..np], &lg.pole_im[..np], &lg.slope[..np]);
            for k in 0..np {
                let (x, y) = (re[k], im[k]);
                a[k] = sl[k] * x;
                re[k] = pr[k] * x - pi[k] * y;
                im[k] = pr[k] * y + pi[k] * x;
            }
            let a_k = &self.nl_a[..np];
            let ramp = if self.nl_age < self.nl_ramp {
                let x = self.nl_age as f32 / self.nl_ramp as f32;
                0.5 - 0.5 * (std::f32::consts::PI * x).cos()
            } else {
                1.0
            };
            self.nl_age = self.nl_age.saturating_add(1);
            let mut out = 0.0f32;
            for n in 1..=nl {
                // Σ_{j+k=n} a_j·a_k (j, k ≥ 1) + 2·Σ_k a_k·a_{k+n}.
                let mut sum = 0.0f32;
                for j in 1..n.min(np + 1) {
                    if n - j <= np {
                        sum += a_k[j - 1] * a_k[n - j - 1];
                    }
                }
                // 8 independent accumulators: vectorisable (a single float
                // sum cannot be reordered by the compiler).
                let mut acc = [0.0f32; 8];
                if n < np {
                    let (x, y) = (&a_k[..np - n], &a_k[n..np]);
                    let ((cx, rx), (cy, ry)) = (x.as_chunks::<8>(), y.as_chunks::<8>());
                    for (u, v) in cx.iter().zip(cy) {
                        for l in 0..8 {
                            acc[l] += u[l] * v[l];
                        }
                    }
                    for (u, v) in rx.iter().zip(ry) {
                        acc[0] += u * v;
                    }
                }
                let diff = ((acc[0] + acc[4]) + (acc[1] + acc[5])) + ((acc[2] + acc[6]) + (acc[3] + acc[7]));
                let f = ramp * lg.long_gain[n - 1] * (sum + 2.0 * diff);
                let (zr, zi) = self.nl_long[n - 1];
                let (pr, pi) = lg.long_pole[n - 1];
                let z = (pr * zr - pi * zi + f, pr * zi + pi * zr);
                self.nl_long[n - 1] = z;
                out += lg.long_out[n - 1] * z.1;
            }
            // y = r·(y + x − x₋₁): first-order high-pass (bridge moment).
            let (x1, y1) = self.nl_hp;
            let y = lg.hp_pole * (y1 + out - x1);
            self.nl_hp = (out, y);
            self.force[t] += y;
        }
    }

    /// A damper landing on the strings with momentum `impulse` (N·s) at
    /// `position`·L (pad `width` m), spread over `landing` s: each vertical
    /// string's mode k receives Δq̇ = φ_k·S(ω)·I/M (S: half-sine spectrum),
    /// i.e. Δζ = −i·φ_k·S·I/(M·ω), projected on the coupled modes.
    /// Real-time safe.
    pub fn damper_kick(&mut self, nm: &NoteModel, impulse: f64, position: f64, width: f64, landing: f64) {
        if impulse == 0.0 {
            return;
        }
        let np = &nm.params;
        let n = nm.n_osc;
        let ns = np.strings;
        let modal_mass = np.mu * np.length / 2.0;
        // Partials dropped once silent ring again under the felt.
        if !self.bank.is_empty() && self.bank.len() < nm.modes.len() {
            self.bank.set_active_len(nm.modes.len());
        }
        // A listening string only runs its sympathetic bank.
        let bank = if self.bank.is_empty() { &mut self.sym } else { &mut self.bank };
        for (ki, cl) in nm.clusters.iter().enumerate() {
            if (ki + 1) * n > bank.len() {
                break;
            }
            let k = (ki + 1) as f64;
            let a = k * std::f64::consts::PI * width / (2.0 * np.length);
            // Dampers sit near the strike line, `position`·L from the agraffe.
            let phi = (k * std::f64::consts::PI * (1.0 - position)).sin() * if a > 1e-9 { a.sin() / a } else { 1.0 };
            let omega = std::f64::consts::TAU * nm.phys[ki * n].freq;
            // The felt lands over `landing` seconds: soft, low-passed thud.
            let soft = crate::params::NoiseLaw::half_sine_spectrum(omega, landing);
            let dz = C64::new(0.0, -phi * soft * impulse / (modal_mass * omega));
            let mut zeta = [C64::ZERO; MAX_OSC];
            zeta[..ns].fill(dz);
            let dc = cl.to_modes(&zeta);
            for (m, d) in dc.iter().enumerate().take(n) {
                let (re, im) = bank.state(ki * n + m);
                bank.set_state(ki * n + m, re + d.re, im + d.im);
            }
        }
    }

    /// Stops running the struck strings' partials once they have died away
    /// (below `threshold`, output-weighted energy, see
    /// [`ModalBank::drop_tail`]); not during the hammer contact.
    pub fn drop_silent_partials(&mut self, threshold: f32) {
        if threshold > 0.0 && !self.contact.is_active() && !self.bank.is_empty() {
            self.bank.drop_tail(threshold);
        }
    }

    /// Sets the damper decay-rate factor (see [`NoteModel::damper_rate`]).
    pub fn set_damper_rate(&mut self, rate: f64) {
        self.bank.set_damping(rate as f32);
        self.sym.set_damping(rate as f32);
    }

    /// Output-weighted energy (bridge force²), for voice management.
    pub fn energy(&self) -> f32 {
        if self.contact.is_active() { f32::INFINITY } else { self.bank.energy() + self.sym.energy() }
    }
}
