//! Precomputed description of every key: coupled modes of its unison and
//! the matching physical oscillators. Built off the audio thread (it
//! allocates and diagonalises up to 6×6 matrices per partial); voices only
//! read it.

use crate::board::{self, BoardMode, MICS};
use crate::complex::C64;
use crate::modal::{ModeSpec, ModeTable};
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
    /// `damp_w` is the damper's mode-shape weight; output 2 is the force on
    /// the agraffe (driving the frame).
    pub modes: Vec<ModeSpec>,
    /// Bridge region feeding / feeding back to this key.
    pub region: usize,
    pub has_damper: bool,
    /// Modes run by a string that only listens (partials < max_freq).
    pub n_listen: usize,
    /// Geometric nonlinearity (phantom partials), when enabled for the key.
    pub longitudinal: Option<Longitudinal>,
    /// `modes` as bank coefficients, for the primary bank and for the
    /// sympathetic one (input gains advanced by e^{iωD}, D = [`BRIDGE_DELAY`]
    /// samples: the bridge input arrives that late, and the coupling keeps its
    /// physical, passive phase at each mode's frequency).
    pub table: ModeTable,
    pub sym_table: ModeTable,
}

/// Delay (samples) of the bridge acceleration seen by the strings: the
/// engine's sub-block (the ring delay) plus the half sample of the backward
/// difference.
pub const BRIDGE_DELAY: f64 = crate::engine::SUB_BLOCK as f64 + 0.5;

/// How finely the strings are resolved: caps that keep the cost bounded
/// where the parts left out were judged inaudible.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Resolution {
    /// Coupled modes per key at most (the partials stop at 0.45·fs anyway).
    pub max_modes: usize,
    /// Partials and longitudinal modes of the phantom-partial model at most.
    pub nl_partials: usize,
    pub nl_long: usize,
}

impl Resolution {
    pub const DEFAULT: Self = Self { max_modes: crate::voice::MAX_MODES, nl_partials: 48, nl_long: 16 };
    /// No cap below the sample rate's own limit.
    pub const UNCAPPED: Self = Self { max_modes: 4096, nl_partials: 256, nl_long: 64 };
}

impl Default for Resolution {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Precomputed longitudinal model of a key (see [`params::StringNonlinearity`]).
///
/// The slope of string j is y' = Σ_k a_k·cos(kπx/L), a_k = (kπ/L)·q_k. The
/// stretching drives longitudinal mode n (shape sin(nπx/L), modal mass μL/2)
/// with F_n = −(EA·nπ/8)·(Σ_{j+k=n} a_j·a_k + 2·Σ_k a_k·a_{k+n}), and the
/// mode pushes on the bridge with EA·(nπ/L)·ξ_n. The voice tracks each
/// partial's mean vertical displacement from the coupled modes' states.
pub struct Longitudinal {
    /// Partials used (k = 1..n_part), their slope factor kπ/L.
    pub slope: Vec<f32>,
    /// Weight of each coupled mode (index k_idx·n_osc + m) in the mean
    /// vertical displacement of its partial, complex.
    pub weights: Vec<(f32, f32)>,
    /// Pole of each partial (its strongest mode) per sample.
    pub pole_re: Vec<f32>,
    pub pole_im: Vec<f32>,
    /// Longitudinal modes: pole, input gain (generalised force → state, all
    /// the strings), output weight (state → board force, with the coupling).
    pub long_pole: Vec<(f32, f32)>,
    pub long_gain: Vec<f32>,
    pub long_out: Vec<f32>,
    /// Gating: E·A/4 (tension change per squared slope, N) and coupling.
    pub ea4: f32,
    pub coupling: f32,
    /// One-pole high-pass pole on the board force (bridge moment).
    pub hp_pole: f32,
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
        Self::build_with(model, board, board_modes, note, sample_rate, Resolution { max_modes, ..Resolution::DEFAULT })
    }

    /// [`Self::build`] at a given [`Resolution`].
    pub fn build_with(
        model: &PianoModel,
        board: Option<&Soundboard>,
        board_modes: Option<&[BoardMode]>,
        note: u8,
        sample_rate: f64,
        res: Resolution,
    ) -> Self {
        let max_modes = res.max_modes;
        let np = model.note(note);
        let region = board::region_of(note);
        let has_damper = note <= model.damper.highest_note;
        let sympathy = board_modes.filter(|_| model.sympathetic.enabled);
        let dt = 1.0 / sample_rate;
        let mut n_listen = 0;
        let bridge_shapes = board.map(|b| b.bridge_shapes(note));
        let ns = np.strings;
        let pol = model.polarization;
        let n_osc = if pol.enabled { 2 * ns } else { ns };
        let nyq = 0.45 * sample_rate;
        let tau = std::f64::consts::TAU;
        let hits = model.unison.hits(note);
        let f0v: Vec<f64> = (0..ns).map(|j| np.f0 * 2f64.powf(np.detune[j] / 1200.0)).collect();
        let f0h: Vec<f64> = f0v.iter().map(|f| f * 2f64.powf(pol.detune_cents / 1200.0)).collect();
        let (mut clusters, mut phys, mut modes) = (Vec::new(), Vec::new(), Vec::new());
        for k in 1..=(max_modes / n_osc) as u32 {
            let fv: Vec<f64> = f0v.iter().map(|&f| params::partial_freq(f, np.b, k)).collect();
            let fh: Vec<f64> = f0h.iter().map(|&f| params::partial_freq(f, np.b, k)).collect();
            if fv.iter().chain(&fh).any(|&f| f >= nyq) {
                break;
            }
            let eps = match (board, &bridge_shapes) {
                (Some(b), Some(sh)) => (b.bridge_admittance(sh, tau * fv[0]) + C64::new(b.edge_conductance(note, tau * fv[0]), 0.0)).scale(np.tension / np.length),
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
                phys.push(PhysOsc { k, freq: fv[j], sigma: -dv[j].re, hit: hits[j] });
                wout[j] = 1.0;
            }
            if pol.enabled {
                for j in 0..ns {
                    phys.push(PhysOsc { k, freq: fh[j], sigma: -dv[j].re, hit: pol.hammer_tilt * hits[j] });
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
                    // Agraffe end: same slope weights with (−1)^{k+1}.
                    out2_re: if k % 2 == 0 { -u.re } else { u.re },
                    out2_im: if k % 2 == 0 { -u.im } else { u.im },
                    damp_w,
                });
            }
            if fv[0] < model.sympathetic.max_freq {
                n_listen = modes.len();
            }
            clusters.push(cl);
        }
        let longitudinal = longitudinal(model, &np, n_osc, &clusters, &modes, note, sample_rate, res);
        let delay = BRIDGE_DELAY / sample_rate;
        let sym_modes: Vec<ModeSpec> = modes
            .iter()
            .map(|m| {
                let adv = C64::polar(1.0, tau * m.freq_hz * delay);
                let g1 = C64::new(m.gain_re, m.gain_im) * adv;
                let g2 = C64::new(m.gain2_re, m.gain2_im) * adv;
                ModeSpec { gain_re: g1.re, gain_im: g1.im, gain2_re: g2.re, gain2_im: g2.im, ..*m }
            })
            .collect();
        let table = ModeTable::new(sample_rate, &modes);
        let sym_table = ModeTable::new(sample_rate, &sym_modes);
        Self {
            note,
            params: np,
            n_osc,
            clusters,
            phys,
            modes,
            region,
            has_damper,
            n_listen,
            longitudinal,
            table,
            sym_table,
        }
    }

    /// Damper decay-rate factor at contact `engagement` (0 lifted, 1 resting).
    pub fn damper_rate(&self, model: &PianoModel, engagement: f64) -> f64 {
        if !self.has_damper {
            return 0.0;
        }
        model.damper.rate(engagement, self.params.tension, self.params.mu, self.params.length)
    }
}

/// Longitudinal model of a key, or None when disabled or above `max_note`.
#[allow(clippy::too_many_arguments)]
fn longitudinal(
    model: &PianoModel,
    np: &NoteParams,
    n_osc: usize,
    clusters: &[Cluster],
    modes: &[ModeSpec],
    note: u8,
    sample_rate: f64,
    res: Resolution,
) -> Option<Longitudinal> {
    let nl = &model.nonlinear;
    if !nl.enabled || note > nl.max_note || clusters.is_empty() {
        return None;
    }
    use std::f64::consts::{PI, TAU};
    let dt = 1.0 / sample_rate;
    let ns = np.strings;
    // Steel core from the inharmonicity: B = π³·E·d⁴/(64·T·L²).
    let d4 = 64.0 * np.b * np.tension * np.length * np.length / (PI.powi(3) * nl.young);
    let ea = nl.young * PI * d4.sqrt() / 4.0;
    let c_long = (ea / np.mu).sqrt();
    let n_part = clusters
        .iter()
        .enumerate()
        .take_while(|(i, _)| modes[i * n_osc].freq_hz < nl.max_partial_hz)
        .count()
        .min(res.nl_partials);
    let n_long = (1..=res.nl_long).take_while(|&n| n as f64 * c_long / (2.0 * np.length) < nl.max_long_hz.min(0.45 * sample_rate)).count();
    if n_part < 2 || n_long == 0 {
        return None;
    }
    let mut vert = [0.0; MAX_OSC];
    vert[..ns].fill(1.0 / ns as f64);
    let mut weights = Vec::with_capacity(n_part * n_osc);
    let (mut pole_re, mut pole_im) = (Vec::with_capacity(n_part), Vec::with_capacity(n_part));
    for (ki, cl) in clusters.iter().take(n_part).enumerate() {
        let mut best = (0.0, (1.0f32, 0.0f32));
        for m in 0..n_osc {
            let w = cl.output_weight(m, &vert);
            weights.push((w.re as f32, w.im as f32));
            let md = &modes[ki * n_osc + m];
            if w.norm() > best.0 {
                let p = C64::polar((-md.sigma * dt).exp(), TAU * md.freq_hz * dt);
                best = (w.norm(), (p.re as f32, p.im as f32));
            }
        }
        pole_re.push(best.1 .0);
        pole_im.push(best.1 .1);
    }
    let slope = (1..=n_part).map(|k| (k as f64 * PI / np.length) as f32).collect();
    let m_long = np.mu * np.length / 2.0;
    let (mut long_pole, mut long_gain, mut long_out) = (Vec::new(), Vec::new(), Vec::new());
    for n in 1..=n_long {
        let w = TAU * n as f64 * c_long / (2.0 * np.length);
        let sigma = 0.5 * nl.long_loss * w;
        let p = C64::polar((-sigma * dt).exp(), w * dt);
        long_pole.push((p.re as f32, p.im as f32));
        // ζ ← p·ζ + F·dt/(m·ω), ξ = Im ζ; F_n per unit of the bracket.
        long_gain.push((-(ea * n as f64 * PI / 8.0) * ns as f64 * dt / (m_long * w)) as f32);
        long_out.push((nl.coupling * ea * n as f64 * PI / np.length) as f32);
    }
    Some(Longitudinal {
        slope,
        weights,
        pole_re,
        pole_im,
        long_pole,
        long_gain,
        long_out,
        ea4: (ea / 4.0) as f32,
        coupling: nl.coupling as f32,
        hp_pole: (-std::f64::consts::TAU * nl.coupling_corner * dt).exp() as f32,
    })
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
        Self::with_resolution(model, board, board_modes, sample_rate, Resolution { max_modes, ..Resolution::DEFAULT })
    }

    pub fn with_resolution(
        model: &PianoModel,
        board: Option<&Soundboard>,
        board_modes: Option<&[BoardMode]>,
        sample_rate: f64,
        res: Resolution,
    ) -> Self {
        // Independent keys: built in parallel (≈ 0.5 s on one core).
        let notes = crate::par::map(88, |i| NoteModel::build_with(model, board, board_modes, 21 + i as u8, sample_rate, res));
        Self { notes }
    }

    /// Most modes of a key, and most partials and longitudinal modes of
    /// the phantom-partial model: what a voice must hold.
    pub fn capacity(&self) -> (usize, usize, usize) {
        let nl = |f: fn(&Longitudinal) -> usize| self.notes.iter().filter_map(|n| n.longitudinal.as_ref().map(f)).max().unwrap_or(0);
        (self.notes.iter().map(|n| n.modes.len()).max().unwrap_or(0), nl(|l| l.slope.len()), nl(|l| l.long_pole.len()))
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
