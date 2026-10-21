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
/// Below this bridge-force energy (N²) a free-decaying sympathetic bank is
/// left alone.
const SYM_SILENCE: f32 = 1e-12;

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
    /// Response to the hammer.
    pub bank: ModalBank,
    /// Response to the bridge motion from the other keys.
    pub sym: ModalBank,
    pub contact: ContactSolver,
    sym_force: Vec<f32>,
    /// Own bridge force: this sub-block, and its history (ring indexed by
    /// absolute sample) to cancel its own contribution D samples later.
    force: Vec<f32>,
    force_ring: Vec<f32>,
    x2: Vec<f32>,
}

/// Ring length for the delayed signals (power of two ≥ 2·SUB_BLOCK).
pub const RING: usize = 1024;

impl Voice {
    pub fn new(sample_rate: f64, max_block: usize) -> Self {
        Self {
            note: 0,
            active: false,
            key_down: false,
            sostenuto: false,
            struck: false,
            engagement: 1.0,
            damper_speed: 1.0,
            bank: ModalBank::new(sample_rate, MAX_MODES, max_block),
            sym: ModalBank::new(sample_rate, MAX_MODES, max_block),
            contact: ContactSolver::new(MAX_MODES, sample_rate),
            sym_force: vec![0.0; max_block],
            force: vec![0.0; max_block],
            force_ring: vec![0.0; RING],
            x2: vec![0.0; max_block],
        }
    }

    /// Prepares the strings of a key at rest; `listen_only` runs only the
    /// partials needed for sympathetic resonance. The bridge input arrives
    /// `delay` seconds late: each mode's input gains are advanced by e^{iωD}
    /// so that the coupling keeps its physical (passive) phase at the mode's
    /// frequency. Real-time safe.
    pub fn setup(&mut self, nm: &NoteModel, listen_only: bool, delay: f64) {
        self.note = nm.note;
        self.bank.set_modes(nm.modes.iter().copied());
        if listen_only {
            self.bank.set_active_len(0);
        }
        self.bank.reset();
        self.sym.set_modes(nm.modes.iter().map(|m| {
            let adv = C64::polar(1.0, std::f64::consts::TAU * m.freq_hz * delay);
            let g1 = C64::new(m.gain_re, m.gain_im) * adv;
            let g2 = C64::new(m.gain2_re, m.gain2_im) * adv;
            crate::modal::ModeSpec { gain_re: g1.re, gain_im: g1.im, gain2_re: g2.re, gain2_im: g2.im, ..*m }
        }));
        self.sym.set_active_len(nm.n_listen);
        self.sym.reset();
        self.force.fill(0.0);
        self.force_ring.fill(0.0);
        self.struck = false;
        self.sostenuto = false;
    }

    /// Strikes the (possibly ringing) strings with the hammer at `v` m/s.
    /// `una_corda` ∈ [0, 1] shifts the hammer off the first string and onto
    /// softer felt (`softening` at full shift).
    pub fn strike(&mut self, nm: &NoteModel, v: f64, oversample: usize, una_corda: f64, softening: f64) {
        if self.contact.is_active() {
            self.hand_over(nm);
        }
        self.bank.set_active_len(nm.modes.len());
        self.struck = true;
        let n = nm.n_osc;
        let ns = nm.params.strings;
        let shift = if ns >= 2 { una_corda.clamp(0.0, 1.0) } else { 0.0 };
        let mut np = nm.params;
        np.hammer.q0 *= 1.0 - softening * una_corda.clamp(0.0, 1.0);
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
                let hit = if j % ns == 0 { p.hit * (1.0 - shift) } else { p.hit };
                PhysMode { k: p.k, freq: p.freq, sigma: p.sigma, state: zeta[j], hit }
            })
        });
        self.contact.strike(&np, v, oversample, modes);
    }

    fn hand_over(&mut self, nm: &NoteModel) {
        let n = nm.n_osc;
        for (ki, cl) in nm.clusters.iter().enumerate() {
            let zeta: [C64; MAX_OSC] =
                std::array::from_fn(|j| if j < n { self.contact.complex_state(ki * n + j) } else { C64::ZERO });
            let c = cl.to_modes(&zeta);
            for (m, cm) in c.iter().enumerate().take(n) {
                self.bank.set_state(ki * n + m, cm.re, cm.im);
            }
        }
    }

    /// Renders samples `from..to` of the sub-block starting at absolute
    /// sample `t0`: adds the primary bridge force to `out` and the
    /// sympathetic one to `out_sym`. `bridge` is the bridge displacement
    /// (from the other keys' primary forces) at the region point, `delay`
    /// samples earlier, when sympathy is on.
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        nm: &NoteModel,
        out: &mut [f32],
        out_sym: &mut [f32],
        t0: u64,
        from: usize,
        to: usize,
        bridge: Option<(&[f32], usize)>,
    ) {
        self.force[from..to].fill(0.0);
        let mut start = 0;
        if self.contact.is_active() {
            start = self.contact.run(&mut self.force[from..to]);
            if !self.contact.is_active() {
                self.hand_over(nm);
            }
        }
        let n = to - from;
        if start < n && !self.bank.is_empty() {
            self.bank.process(None, &mut self.force[from + start..to]);
        }
        for (i, (o, f)) in out.iter_mut().zip(&self.force[from..to]).enumerate() {
            *o += f;
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
                self.sym.process_two_inputs(u, &self.x2[..n], f);
            }
            // Damped: the sympathetic part still decays freely.
            None if self.sym.energy() > SYM_SILENCE => self.sym.process(None, f),
            None => {}
        }
        for (o, x) in out_sym.iter_mut().zip(f.iter()) {
            *o += x;
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
