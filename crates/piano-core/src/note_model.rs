//! Precomputed description of every key: coupled modes of its unison and
//! the matching physical oscillators. Built off the audio thread (it
//! allocates and diagonalises up to 6×6 matrices per partial); voices only
//! read it.

use crate::complex::C64;
use crate::modal::ModeSpec;
use crate::params::{self, NoteParams, PianoModel};
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
    /// Coupled modes for the bank, index `k_idx·n_osc + m`.
    pub modes: Vec<ModeSpec>,
}

impl NoteModel {
    pub fn build(model: &PianoModel, note: u8, sample_rate: f64, max_modes: usize) -> Self {
        let np = model.note(note);
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
            let eps = np.coupling(&model.bridge, fv[0]);
            let dv: Vec<C64> = fv.iter().map(|&f| C64::new(-model.losses.sigma(note, f), tau * f)).collect();
            let cl = if pol.enabled {
                let dh: Vec<C64> = fh.iter().map(|&f| C64::new(-model.losses.sigma(note, f), tau * f)).collect();
                unison::couple_polarised(&dv, &dh, eps, eps.scale(pol.h_coupling), eps.scale(pol.cross))
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
            for m in 0..n_osc {
                let u = cl.output_weight(m, &wout).scale(wk);
                modes.push(ModeSpec {
                    freq_hz: cl.lam[m].im / tau,
                    sigma: -cl.lam[m].re,
                    out_re: u.re,
                    out_im: u.im,
                    ..Default::default()
                });
            }
            clusters.push(cl);
        }
        Self { note, params: np, n_osc, clusters, phys, modes }
    }
}

/// All 128 MIDI keys (out-of-range keys are clamped to the piano compass).
pub struct PianoTable {
    notes: Vec<NoteModel>,
}

impl PianoTable {
    pub fn new(model: &PianoModel, sample_rate: f64, max_modes: usize) -> Self {
        Self { notes: (21u8..=108).map(|n| NoteModel::build(model, n, sample_rate, max_modes)).collect() }
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
        let table = PianoTable::new(&model, 48_000.0, 768);
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

    /// Every key, including single strings, has a fast and a slow component
    /// per partial (in-phase vertical vs out-of-phase/horizontal motion).
    #[test]
    fn every_key_has_two_decay_rates() {
        let model = PianoModel::default();
        for note in [21u8, 27, 30, 60, 100] {
            let nm = NoteModel::build(&model, note, 48_000.0, 768);
            let cl = &nm.clusters[2];
            let mut sig: Vec<f64> = cl.lam[..cl.n].iter().map(|l| -l.re).collect();
            sig.sort_by(f64::total_cmp);
            assert!(sig[cl.n - 1] > 1.5 * sig[0], "note {note}: {sig:?}");
        }
    }
}
