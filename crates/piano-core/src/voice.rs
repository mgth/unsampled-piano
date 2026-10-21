//! One key: its strings coupled at the bridge, the hammer, the damper.
//!
//! The [`ModalBank`] runs the coupled modes of the key's [`NoteModel`]
//! (index `k_idx·n + m`) with complex output weights = bridge force. While
//! the hammer touches the strings, the [`ContactSolver`] runs the physical
//! oscillators (index `k_idx·n + j`, uncoupled during these milliseconds)
//! and hands their complex amplitudes back through Vᵀ.

use crate::complex::C64;
use crate::hammer::{ContactSolver, PhysMode};
use crate::modal::ModalBank;
use crate::note_model::NoteModel;
use crate::unison::MAX_OSC;

pub const MAX_MODES: usize = 768;

pub struct Voice {
    pub note: u8,
    pub active: bool,
    pub key_down: bool,
    pub damped: bool,
    pub bank: ModalBank,
    pub contact: ContactSolver,
}

impl Voice {
    pub fn new(sample_rate: f64, max_block: usize) -> Self {
        Self {
            note: 0,
            active: false,
            key_down: false,
            damped: false,
            bank: ModalBank::new(sample_rate, MAX_MODES, max_block),
            contact: ContactSolver::new(MAX_MODES, sample_rate),
        }
    }

    /// Prepares the strings of a key at rest. Real-time safe (no allocation).
    pub fn setup(&mut self, nm: &NoteModel) {
        self.note = nm.note;
        self.bank.set_modes(nm.modes.iter().copied());
        self.bank.reset();
        self.damped = false;
    }

    /// Strikes the (possibly ringing) strings with the hammer at `v` m/s.
    pub fn strike(&mut self, nm: &NoteModel, v: f64, oversample: usize) {
        if self.contact.is_active() {
            self.hand_over(nm);
        }
        let n = nm.n_osc;
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
                PhysMode { k: p.k, freq: p.freq, sigma: p.sigma, state: zeta[j], hit: p.hit }
            })
        });
        self.contact.strike(&nm.params, v, oversample, modes);
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

    /// Adds `out.len()` samples of bridge force to `out`.
    pub fn render(&mut self, nm: &NoteModel, out: &mut [f32]) {
        let mut start = 0;
        if self.contact.is_active() {
            start = self.contact.run(out);
            if !self.contact.is_active() {
                self.hand_over(nm);
            }
        }
        if start < out.len() {
            self.bank.process(None, &mut out[start..]);
        }
    }

    /// Extra decay rate on every mode (placeholder damper), 0 to release.
    pub fn set_damping(&mut self, extra_sigma: f64) {
        self.bank.update_poles(extra_sigma);
        self.damped = extra_sigma > 0.0;
    }

    /// Output-weighted energy (bridge force²), for voice management.
    pub fn energy(&self) -> f32 {
        if self.contact.is_active() { f32::INFINITY } else { self.bank.energy() }
    }
}
