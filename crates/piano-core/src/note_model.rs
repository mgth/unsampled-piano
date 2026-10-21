//! Precomputed description of every key: coupled modes of its unison and
//! the matching physical oscillators. Built off the audio thread (it
//! allocates and diagonalises up to 6×6 matrices per partial); voices only
//! read it.

use crate::board::{self, BoardMode, MICS};
use crate::complex::C64;
use crate::modal::ModeSpec;
use crate::params::{self, NoteParams, PianoModel};
use crate::soundboard::Soundboard;
use crate::unison::{self, Cluster, MAX_OSC};

/// One physical oscillator (string j, polarisation) of partial k.
#[derive(Clone, Copy, Debug)]
pub struct PhysOsc {
    pub k: u32,
    pub freq: f64,
    pub sigma: f64,
    /// Share of the hammer force it receives.
    pub hit: f64,
}

pub struct NoteModel {
    pub note: u8,
    pub params: NoteParams,
    /// Oscillators per partial: strings × polarisations.
    pub n_osc: usize,
    pub clusters: Vec<Cluster>,
    /// Index `k_idx·n_osc + j`.
    pub phys: Vec<PhysOsc>,
    /// Coupled modes for the bank, index `k_idx·n_osc + m`. Input 1 is the
    /// bridge acceleration at the key's region point (sympathy; backward
    /// difference of the board's velocity output), input 2 the
    /// key's own bridge force (cancels its own contribution to input 1);
    /// `damp_w` is the damper's mode-shape weight.
    pub modes: Vec<ModeSpec>,
    /// Bridge region feeding / feeding back to this key.
    pub region: usize,
    pub has_damper: bool,
    /// Modes run by a string that only listens (partials < max_freq).
    pub n_listen: usize,
}

impl NoteModel {
    /// With a soundboard, the bridge coupling uses its modal admittance at
    /// the key's bridge point; otherwise the smooth [`params::BridgeLaw`].
    pub fn build(
        model: &PianoModel,
        board: Option<&Soundboard>,
        board_modes: Option<&[BoardMode]>,
        note: u8,
        sample_rate: f64,
        max_modes: usize,
    ) -> Self {
        let np = model.note(note);
        let region = board::region_of(note);
        let has_damper = note <= model.damper.highest_note;
        let sympathy = board_modes.filter(|_| model.sympathetic.enabled);
        let dt = 1.0 / sample_rate;
        let mut n_listen = 0;
        let bridge_xy = board.map(|b| b.params.bridge_point(note));
        let ns = np.strings;
        let pol = model.polarization;
        let n_osc = if pol.enabled { 2 * ns } else { ns };
        let nyq = 0.45 * sample_rate;
        let tau = std::f64::consts::TAU;
        let f0v: Vec<f64> = (0..ns).map(|j| np.f0 * 2f64.powf(np.detune[j] / 1200.0)).collect();
        let f0h: Vec<f64> = f0v.iter().map(|f| f * 2f64.powf(pol.detune_cents / 1200.0)).collect();
        let (mut clusters, mut phys, mut modes) = (Vec::new(), Vec::new(), Vec::new());
        for k in 1..=(max_modes / n_osc) as u32 {
            let fv: Vec<f64> = f0v.iter().map(|&f| params::partial_freq(f, np.b, k)).collect();
            let fh: Vec<f64> = f0h.iter().map(|&f| params::partial_freq(f, np.b, k)).collect();
            if fv.iter().chain(&fh).any(|&f| f >= nyq) {
                break;
            }
            let eps = match (board, bridge_xy) {
                (Some(b), Some((x, y))) => b.bridge_admittance(x, y, tau * fv[0]).scale(np.tension / np.length),
                _ => np.coupling(&model.bridge, fv[0]),
            };
            let dv: Vec<C64> = fv.iter().map(|&f| C64::new(-model.losses.sigma(note, f), tau * f)).collect();
            let cl = if pol.enabled {
                let dh: Vec<C64> = fh.iter().map(|&f| C64::new(-model.losses.sigma(note, f), tau * f)).collect();
                unison::couple_polarised(&dv, &dh, eps, eps.scale(pol.h_coupling), eps.scale(pol.passive_cross()))
            } else {
                unison::couple(&dv, eps)
            };
            let mut wout = [0.0; MAX_OSC];
            for j in 0..ns {
                phys.push(PhysOsc { k, freq: fv[j], sigma: -dv[j].re, hit: 1.0 });
                wout[j] = 1.0;
            }
            if pol.enabled {
                for j in 0..ns {
                    phys.push(PhysOsc { k, freq: fh[j], sigma: -dv[j].re, hit: pol.hammer_tilt });
                    wout[ns + j] = pol.radiation;
                }
            }
            // Bridge force of partial k: T·(kπ/L)·Σ_j w_j·q_j.
            let wk = np.tension * k as f64 * std::f64::consts::PI / np.length;
            let omega = tau * fv[0];
            // A moving bridge end forces string mode k with −(μL/kπ)·ü (wave
            // equation with y(0,t) = u(t)): gain −i·dt/(M·ω)·(−μL/kπ) =
            // i·2·dt/(kπ·ω) on the real acceleration input. (The displacement
            // form w_k·u is only valid at resonance: off resonance it makes
            // every mode follow the slow board motion statically.) Its own
            // contribution to ü, H·F_own, is cancelled by input 2.

            let mut vert = [0.0; MAX_OSC];
            vert[..ns].fill(1.0);
            let damp_w = if has_damper { model.damper.shape(k, np.length) } else { 0.0 };
            for m in 0..n_osc {
                let u = cl.output_weight(m, &wout).scale(wk);
                // Own contribution at this coupled mode's exact frequency (the
                // bridge stiffness shifts treble modes by several Hz).
                // Rendered path: velocity transfer, then (v[n] − v[n−1])·fs.
                let h_self = sympathy.map(|bm| {
                    let w = cl.lam[m].im;
                    let diff = (C64::ONE - C64::polar(1.0, -w * dt)).scale(sample_rate);
                    board::transfer(bm, region, MICS + region, w, sample_rate) * diff
                });
                let (g1, g2) = match h_self {
                    Some(h) => {
                        let kpi = k as f64 * std::f64::consts::PI;
                        let g1 = cl.output_weight(m, &vert) * C64::new(0.0, 2.0 * dt / (kpi * omega));
                        let g1 = g1.scale(model.sympathetic.gain);
                        (g1, if model.sympathetic.self_cancel { -(g1 * h) } else { C64::ZERO })
                    }
                    None => (C64::ZERO, C64::ZERO),
                };
                modes.push(ModeSpec {
                    freq_hz: cl.lam[m].im / tau,
                    sigma: -cl.lam[m].re,
                    gain_re: g1.re,
                    gain_im: g1.im,
                    gain2_re: g2.re,
                    gain2_im: g2.im,
                    out_re: u.re,
                    out_im: u.im,
                    damp_w,
                    ..Default::default()
                });
            }
            if fv[0] < model.sympathetic.max_freq {
                n_listen = modes.len();
            }
            clusters.push(cl);
        }
        Self { note, params: np, n_osc, clusters, phys, modes, region, has_damper, n_listen }
    }

    /// Damper decay-rate factor at contact `engagement` (0 lifted, 1 resting).
    pub fn damper_rate(&self, model: &PianoModel, engagement: f64) -> f64 {
        if !self.has_damper {
            return 0.0;
        }
        model.damper.rate(engagement, self.params.tension, self.params.mu, self.params.length)
    }
}

/// All 128 MIDI keys (out-of-range keys are clamped to the piano compass).
pub struct PianoTable {
    notes: Vec<NoteModel>,
}

impl PianoTable {
    pub fn new(
        model: &PianoModel,
        board: Option<&Soundboard>,
        board_modes: Option<&[BoardMode]>,
        sample_rate: f64,
        max_modes: usize,
    ) -> Self {
        Self {
            notes: (21u8..=108)
                .map(|n| NoteModel::build(model, board, board_modes, n, sample_rate, max_modes))
                .collect(),
        }
    }

    pub fn get(&self, note: u8) -> &NoteModel {
        &self.notes[(note.clamp(21, 108) - 21) as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every cluster of the default piano diagonalises cleanly.
    #[test]
    fn whole_table_is_well_conditioned() {
        let model = PianoModel::default();
        let t0 = std::time::Instant::now();
        let table = PianoTable::new(&model, None, None, 48_000.0, 768);
        eprintln!("table built in {:?}", t0.elapsed());
        let mut worst = 0.0f64;
        let mut worst_amp = 0.0f64;
        for note in 21u8..=108 {
            let nm = table.get(note);
            assert!(!nm.clusters.is_empty());
            for cl in &nm.clusters {
                let n = cl.n;
                for m in 0..n {
                    let mut s = C64::ZERO;
                    let mut amp = 0.0f64;
                    for j in 0..n {
                        s += cl.v[j][m] * cl.v[j][m];
                        amp = amp.max(cl.v[j][m].norm());
                    }
                    worst = worst.max((s - C64::ONE).norm());
                    worst_amp = worst_amp.max(amp);
                    assert!(cl.lam[m].re < 0.0, "growing mode at note {note}");
                }
            }
        }
        eprintln!("max |vᵀv − 1| = {worst:e}, max |V_jm| = {worst_amp:.2}");
        assert!(worst < 1e-7);
        // Near an exceptional point eigenvectors blow up and f32 cancels.
        assert!(worst_amp < 50.0, "ill-conditioned cluster: {worst_amp}");
    }

    /// With the modal soundboard (sharp admittance peaks), every coupled
    /// mode of every key still decays: the bridge is passive.
    #[test]
    fn full_instrument_is_passive() {
        let inst = crate::instrument::Instrument::new(PianoModel::default(), 48_000.0);
        for note in 21u8..=108 {
            for (i, m) in inst.table.get(note).modes.iter().enumerate() {
                assert!(m.sigma > 0.0, "note {note} mode {i}: σ = {}", m.sigma);
            }
        }
    }

    /// Every key, including single strings, has a fast and a slow component
    /// per partial (in-phase vertical vs out-of-phase/horizontal motion).
    #[test]
    fn every_key_has_two_decay_rates() {
        let model = PianoModel::default();
        for note in [21u8, 27, 30, 60, 100] {
            let nm = NoteModel::build(&model, None, None, note, 48_000.0, 768);
            let cl = &nm.clusters[2];
            let mut sig: Vec<f64> = cl.lam[..cl.n].iter().map(|l| -l.re).collect();
            sig.sort_by(f64::total_cmp);
            assert!(sig[cl.n - 1] > 1.5 * sig[0], "note {note}: {sig:?}");
        }
    }
}
