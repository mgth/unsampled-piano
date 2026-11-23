//! Precomputed description of every key: coupled modes of its unison and
//! the matching physical oscillators. Built off the audio thread (it
//! allocates and diagonalises up to 6×6 matrices per partial); voices only
//! read it.

use crate::board::{self, BoardMode};
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
    /// Share of the hammer force it receives: hit·(1 + spread·uneven).
    pub hit: f64,
    pub uneven: f64,
    /// Its shape when it is not the plain partial's (a string that runs on
    /// under a capo bar, [`crate::duplex`]); `k` is 0 for a mode of the
    /// length in front of the bar.
    pub shape: Option<crate::hammer::ModeShape>,
}

pub struct NoteModel {
    pub note: u8,
    pub params: NoteParams,
    /// Oscillators per partial: strings × polarisations.
    pub n_osc: usize,
    pub clusters: Vec<Cluster>,
    /// The first clusters are the speaking length's partials 1, 2, …; the
    /// ones after them the modes of the length in front of a capo bar
    /// (`frame.duplex`).
    pub n_partials: usize,
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
    /// The regions its low band is shared between ([`board::region_weights`]).
    pub region_mix: [(usize, f32); 2],
    /// Its weights on the low modes' drive profiles ([`board::LowDrive`]),
    /// when they run apart.
    pub basis: Option<[f32; board::KEYS]>,
    pub has_damper: bool,
    pub unison: crate::params::UnisonLaw,
    /// Output weight of the horizontal oscillators (polarization.radiation).
    pub h_radiation: f64,
    /// The low board bank's input this key's rocking moment drives, when the
    /// bridge rocks (`SoundboardParams::bridge_rocking`): the coupled modes'
    /// third output (`out3_*`) carries it, the sideways motion's force.
    pub rock_input: Option<usize>,
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
    /// The sympathetic table while the dampers are lifted: the same modes,
    /// braked by every free string instead of the undamped ones alone
    /// ([`params::SympatheticLaw::brake`]).
    pub sym_table_free: ModeTable,
    /// The board's forced response at the coupled modes' own frequencies
    /// (`soundboard.forced_radiation`): per pair of microphones, the same
    /// modes with no input and, as outputs 1 and 2, what each adds at those
    /// two microphones ([`crate::forced`]). Empty when off.
    pub forced: Vec<ModeTable>,
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
    /// The strike's stretching (`strike_stretch`), per longitudinal mode n:
    /// the rotation e^(i·nπc·dt/L) of the strain pulses leaving the strike
    /// point and the weight that turns Σ F_s² into the mode's bracket,
    /// (8/(L·c))·cos(nπx_h/L)·dt/(4Z²·strings). Empty when off.
    pub strike_rot: Vec<(f32, f32)>,
    pub strike_w: Vec<f32>,
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
        Self::build_with(model, board, board_modes, None, None, note, sample_rate, Resolution { max_modes, ..Resolution::DEFAULT })
    }

    /// [`Self::build`] at a given [`Resolution`], the frame two way when
    /// given.
    #[allow(clippy::too_many_arguments)]
    pub fn build_with(
        model: &PianoModel,
        board: Option<&Soundboard>,
        board_modes: Option<&[BoardMode]>,
        low: Option<&board::LowDrive>,
        frame: Option<&crate::structure::FrameAdmittance>,
        note: u8,
        sample_rate: f64,
        res: Resolution,
    ) -> Self {
        let max_modes = res.max_modes;
        let np = model.note(note);
        let geo = params::StringGeometry::of(&np, model.nonlinear.young);
        let region = board::region_of(note);
        // With the low modes driven at the key itself, the others take its
        // own region whole (no low band to share).
        let region_mix = if low.is_some() { [(region, 1.0), (region, 0.0)] } else { board::region_weights(note) };
        let basis = low.map(|ld| ld.key(note).map(|x| x as f32));
        let rock_input = low.and_then(|ld| ld.rock_input(note));
        // Weight of each oscillator on the rocking moment's force: the
        // sideways ones whole, the vertical ones not at all.
        let rocks = rock_input.is_some();
        // Each low mode's gain from this key (its drive shape · T, as the
        // profiles rebuild it).
        let low_gain: Option<(usize, Vec<f64>)> =
            low.map(|ld| (ld.n, ld.inputs.iter().map(|inp| inp.iter().zip(ld.key(note)).map(|(a, b)| a * b).sum()).collect()));
        let has_damper = note <= model.damper.highest_note;
        let damper_at = model.damper.place_of(note, np.strike_pos, np.length);
        let wedge = model.damper.wedge(note);
        let sympathy = board_modes.filter(|_| model.sympathetic.enabled);
        let dt = 1.0 / sample_rate;
        let mut n_listen = 0;
        let bridge_shapes = board.map(|b| b.bridge_shapes(note));
        let ns = np.strings;
        let pol = model.polarization.at(note);
        let n_osc = if pol.enabled { 2 * ns } else { ns };
        let nyq = 0.45 * sample_rate;
        let tau = std::f64::consts::TAU;
        let pattern = model.unison.pattern(note);
        // Each string's own inharmonicity (unison.b_spread), its first
        // partial kept where the tuning puts it.
        let bf = model.unison.b_factors(note);
        let b_of = |j: usize| np.b * bf[j];
        let f0v: Vec<f64> = (0..ns)
            .map(|j| {
                let f = np.f0 * 2f64.powf(np.detune[j] / 1200.0);
                if bf[j] == 1.0 { f } else { f * ((1.0 + np.b) / (1.0 + b_of(j))).sqrt() }
            })
            .collect();
        let f0h: Vec<f64> = f0v.iter().map(|f| f * 2f64.powf(pol.detune_cents / 1200.0)).collect();
        let (mut clusters, mut phys, mut modes) = (Vec::new(), Vec::new(), Vec::new());
        // Under a capo bar, the front duplex on: each string's modes on its
        // three supports, the bearing bar a step further for each string.
        let front: Option<Vec<crate::duplex::StringModes>> =
            board.map(|b| &b.params).or(model.soundboard.as_ref()).filter(|_| model.frame.duplex).and_then(|sb| {
                (0..ns)
                    .map(|j| {
                        let l = model.frame.duplex_string(sb, note, j, ns)?;
                        Some(crate::duplex::modes(np.length, b_of(j), np.strike_pos, l, nyq / params::partial_freq(f0v[j], b_of(j), 1)))
                    })
                    .collect()
            });
        // What the bearing bar and its felt take of a mode: its share of
        // motion in front of the capo.
        let front_loss = |d: &crate::duplex::DuplexMode| model.frame.duplex_loss.max(0.0) * (1.0 - d.speaking);
        for k in 1..=(max_modes / n_osc) as u32 {
            // String j's partial k where the string runs on under the capo.
            let at = |j: usize| front.as_ref().and_then(|f| f[j].partials.get(k as usize - 1).copied().flatten());
            let freq = |j: usize, f: f64| at(j).map_or_else(|| params::partial_freq(f, b_of(j), k), |d| params::partial_freq(f, b_of(j), 1) * d.ratio);
            let fv: Vec<f64> = f0v.iter().enumerate().map(|(j, &f)| freq(j, f)).collect();
            let fh: Vec<f64> = f0h.iter().enumerate().map(|(j, &f)| freq(j, f)).collect();
            if fv.iter().chain(&fh).any(|&f| f >= nyq) {
                break;
            }
            // The smooth admittance the sideways motion sees (polarization.smooth).
            let eps_ref = match board {
                Some(b) => C64::new(b.params.mean_mobility() * np.tension / np.length, 0.0),
                None => np.coupling(&model.bridge, fv[0]),
            };
            let eps = match (board, &bridge_shapes) {
                (Some(b), Some(sh)) => {
                    // The string's other end: the frame at its agraffe (the
                    // mode's slope is the same there, (−1)^(k+1)·kπ/L).
                    let at_frame = frame.map_or(C64::ZERO, |f| f.at(b.params.key_across(note), tau * fv[0]));
                    let y = b.bridge_admittance(sh, tau * fv[0]) + C64::new(b.edge_conductance(note, tau * fv[0]), 0.0) + at_frame;
                    let y = if b.params.bridge_yield { y + C64::new(b.yield_conductance(tau * fv[0]), 0.0) } else { y };
                    y.scale(np.tension / np.length)
                }
                _ => np.coupling(&model.bridge, fv[0]),
            };
            // A partial that shares its motion with the length in front of
            // the capo holds the bridge by its slope there.
            let eps = match &front {
                Some(_) => eps.scale((0..ns).map(|j| at(j).map_or(1.0, |d| (d.shape.bridge / k as f64).powi(2))).sum::<f64>() / ns as f64),
                None => eps,
            };
            let extra = |j: usize| at(j).map_or(0.0, |d| front_loss(&d));
            let dv: Vec<C64> = fv.iter().enumerate().map(|(j, &f)| C64::new(-(model.losses.sigma_on(note, f, &geo) + extra(j)), tau * f)).collect();
            let cl = if pol.enabled {
                let dh: Vec<C64> = fh.iter().enumerate().map(|(j, &f)| C64::new(-(model.losses.sigma_on(note, f, &geo) + extra(j)), tau * f)).collect();
                let (eps_h, eps_x) = pol.sideways(eps, eps_ref, fv[0]);
                unison::couple_polarised(&dv, &dh, eps, eps_h, eps_x)
            } else {
                unison::couple(&dv, eps)
            };
            let mut wout = [0.0; MAX_OSC];
            let mut wrock = [0.0; MAX_OSC];
            for j in 0..ns {
                phys.push(PhysOsc { k, freq: fv[j], sigma: -dv[j].re, hit: 1.0, uneven: pattern[j], shape: at(j).map(|d| d.shape) });
                wout[j] = at(j).map_or(1.0, |d| d.shape.bridge / k as f64);
            }
            if pol.enabled {
                for j in 0..ns {
                    phys.push(PhysOsc { k, freq: fh[j], sigma: -dv[j].re, hit: pol.hammer_tilt, uneven: pattern[j], shape: at(j).map(|d| d.shape) });
                    wout[ns + j] = pol.radiation * at(j).map_or(1.0, |d| d.shape.bridge / k as f64);
                    if rocks {
                        wrock[ns + j] = at(j).map_or(1.0, |d| d.shape.bridge / k as f64);
                    }
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
            let damp_w = if has_damper { model.damper.shape_at(model.damper.pad(note), k, damper_at, np.length, wedge, fv[0]) } else { 0.0 };
            let sideways = model.damper.horizontal_at(fv[0], wedge);
            for m in 0..n_osc {
                let u = cl.output_weight(m, &wout).scale(wk);
                let u3 = if rocks { cl.output_weight(m, &wrock).scale(wk) } else { C64::ZERO };
                // The felt rests on the strings: it brakes the vertical
                // polarisation fully, the sideways one by friction only.
                // First order in the damping: Σ_j w_j·V_jm² (Vᵀ·V = I).
                let mut pw = C64::ZERO;
                for j in 0..n_osc {
                    pw += (cl.v[j][m] * cl.v[j][m]).scale(if j < ns { 1.0 } else { sideways });
                }
                let lo = sideways.min(1.0);
                let damp_w = damp_w * pw.re.clamp(lo, lo.max(1.0));
                // Own contribution at this coupled mode's exact frequency (the
                // bridge stiffness shifts treble modes by several Hz).
                // Rendered path: velocity transfer, then (v[n] − v[n−1])·fs.
                let h_self = sympathy.map(|bm| {
                    let w = cl.lam[m].im;
                    let diff = (C64::ONE - C64::polar(1.0, -w * dt)).scale(sample_rate);
                    let h = match &low_gain {
                        // The low modes from the key itself, the others
                        // from its region.
                        Some((n, g)) => {
                            board::transfer_with(&bm[..*n], |i, _| g[i], board::BRIDGE + region, w, sample_rate)
                                + board::transfer(&bm[*n..], region, board::BRIDGE + region, w, sample_rate)
                        }
                        // Its low band reaches the board through two regions,
                        // the rest through its own (Voice::mix).
                        None => {
                            let lp = crate::voice::share_lowpass(w, sample_rate);
                            let low = region_mix.iter().fold(C64::ZERO, |h, &(r, wr)| h + board::transfer(bm, r, board::BRIDGE + region, w, sample_rate).scale(wr as f64));
                            let own = board::transfer(bm, region, board::BRIDGE + region, w, sample_rate);
                            lp * low + (C64::ONE - lp) * own
                        }
                    };
                    h * diff
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
                    out3_re: u3.re,
                    out3_im: u3.im,
                    damp_w,
                });
            }
            if fv[0] < model.sympathetic.max_freq {
                n_listen = modes.len();
            }
            clusters.push(cl);
        }
        let longitudinal = longitudinal(model, &np, n_osc, &clusters, &modes, note, sample_rate, res);
        // The modes of the lengths in front of the capo, after the
        // partials: one cluster for the strings' first, one for their
        // second, …. The hammer rings them and the bridge hears them by
        // their share of motion in the speaking length; they do not listen
        // to the bridge (their share is small) nor pull on the frame (far
        // too stiff there).
        let n_partials = clusters.len();
        let orders = front.as_ref().map_or(0, |f| f.iter().map(|s| s.duplex.len()).min().unwrap_or(0));
        for i in 0..orders {
            let d: Vec<crate::duplex::DuplexMode> = front.iter().flatten().map(|s| s.duplex[i]).collect();
            let fv: Vec<f64> = (0..ns).map(|j| params::partial_freq(f0v[j], b_of(j), 1) * d[j].ratio).collect();
            let fh: Vec<f64> = (0..ns).map(|j| params::partial_freq(f0h[j], b_of(j), 1) * d[j].ratio).collect();
            if modes.len() + n_osc > max_modes || fv.iter().chain(&fh).any(|&f| f >= nyq) {
                break;
            }
            // The bridge as a partial of the same wavelength would see it,
            // by the slope the mode has there.
            let hold = d.iter().map(|m| (m.shape.bridge / m.shape.nu).powi(2)).sum::<f64>() / ns as f64;
            let (eps_ref, eps) = match (board, &bridge_shapes) {
                (Some(b), Some(sh)) => (
                    C64::new(b.params.mean_mobility() * np.tension / np.length, 0.0),
                    {
                        let y = b.bridge_admittance(sh, tau * fv[0]) + C64::new(b.edge_conductance(note, tau * fv[0]), 0.0);
                        let y = if b.params.bridge_yield { y + C64::new(b.yield_conductance(tau * fv[0]), 0.0) } else { y };
                        y.scale(hold * np.tension / np.length)
                    },
                ),
                _ => (np.coupling(&model.bridge, fv[0]), np.coupling(&model.bridge, fv[0]).scale(hold)),
            };
            let dv: Vec<C64> = (0..ns).map(|j| C64::new(-(model.losses.sigma_on(note, fv[j], &geo) + front_loss(&d[j])), tau * fv[j])).collect();
            let cl = if pol.enabled {
                let dh: Vec<C64> = (0..ns).map(|j| C64::new(dv[j].re, tau * fh[j])).collect();
                let (eps_h, eps_x) = pol.sideways(eps, eps_ref, fv[0]);
                unison::couple_polarised(&dv, &dh, eps, eps_h, eps_x)
            } else {
                unison::couple(&dv, eps)
            };
            let mut wout = [0.0; MAX_OSC];
            let mut wrock = [0.0; MAX_OSC];
            for j in 0..ns {
                phys.push(PhysOsc { k: 0, freq: fv[j], sigma: -dv[j].re, hit: 1.0, uneven: pattern[j], shape: Some(d[j].shape) });
                wout[j] = d[j].shape.bridge;
            }
            if pol.enabled {
                for j in 0..ns {
                    phys.push(PhysOsc { k: 0, freq: fh[j], sigma: -dv[j].re, hit: pol.hammer_tilt, uneven: pattern[j], shape: Some(d[j].shape) });
                    wout[ns + j] = pol.radiation * d[j].shape.bridge;
                    if rocks {
                        wrock[ns + j] = d[j].shape.bridge;
                    }
                }
            }
            // The damper lies on the speaking length: it takes the mode by
            // its share of motion there.
            let near = (d[0].shape.nu.round() as u32).max(1);
            let damp_w = if has_damper { model.damper.shape_at(model.damper.pad(note), near, damper_at, np.length, wedge, fv[0]) * hold } else { 0.0 };
            for m in 0..n_osc {
                let u = cl.output_weight(m, &wout).scale(np.tension * std::f64::consts::PI / np.length);
                let u3 = cl.output_weight(m, &wrock).scale(np.tension * std::f64::consts::PI / np.length);
                modes.push(ModeSpec {
                    freq_hz: cl.lam[m].im / tau,
                    sigma: -cl.lam[m].re,
                    gain_re: 0.0,
                    gain_im: 0.0,
                    gain2_re: 0.0,
                    gain2_im: 0.0,
                    out_re: u.re,
                    out_im: u.im,
                    out2_re: 0.0,
                    out2_im: 0.0,
                    out3_re: u3.re,
                    out3_im: u3.im,
                    damp_w,
                });
            }
            clusters.push(cl);
        }
        let table = ModeTable::new(sample_rate, &modes);
        let sym_table = ModeTable::new(sample_rate, &sym_specs(&modes, None, sample_rate));
        let sym_table_free = ModeTable::new(sample_rate, &sym_specs(&modes, None, sample_rate));
        Self {
            note,
            params: np,
            n_osc,
            clusters,
            n_partials,
            phys,
            modes,
            region,
            region_mix,
            basis,
            has_damper,
            unison: model.unison,
            h_radiation: pol.radiation,
            rock_input,
            n_listen,
            longitudinal,
            table,
            sym_table,
            sym_table_free,
            forced: Vec::new(),
        }
    }

    /// The sympathetic table for the dampers' state.
    pub fn sym_table(&self, free: bool) -> &ModeTable {
        if free { &self.sym_table_free } else { &self.sym_table }
    }

    /// Damper decay-rate factor at contact `engagement` (0 lifted, 1 resting).
    pub fn damper_rate(&self, model: &PianoModel, engagement: f64) -> f64 {
        if !self.has_damper {
            return 0.0;
        }
        model.damper.rate_under(engagement, self.params.tension, self.params.mu, self.params.length, model.damper.wedge(self.note))
    }
}

/// The sympathetic bank's modes: input gains advanced by e^{iωD}, D =
/// [`BRIDGE_DELAY`] samples (the bridge input arrives that late, and the
/// coupling keeps its physical, passive phase at each mode's frequency),
/// times each mode's brake when given.
fn sym_specs(modes: &[ModeSpec], brake: Option<&[C64]>, sample_rate: f64) -> Vec<ModeSpec> {
    let delay = BRIDGE_DELAY / sample_rate;
    modes
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let adv = C64::polar(1.0, std::f64::consts::TAU * m.freq_hz * delay) * brake.and_then(|b| b.get(i)).copied().unwrap_or(C64::ONE);
            let g1 = C64::new(m.gain_re, m.gain_im) * adv;
            let g2 = C64::new(m.gain2_re, m.gain2_im) * adv;
            // (A listening bank is heard by its first output only: no rocking.)
            ModeSpec { gain_re: g1.re, gain_im: g1.im, gain2_re: g2.re, gain2_im: g2.im, out3_re: 0.0, out3_im: 0.0, ..*m }
        })
        .collect()
}

/// The rear duplex (`sympathetic.rear`): the modes of each key's rear
/// length, per board region, as a listening bank takes them (input: the
/// bridge acceleration at the region, output: the force on the board
/// there). A length l at the key's tension and stiffness has its modes at
/// n·f0·(L/l)·√(1 + B·(L/l)²·n²); a moving end forces mode n with
/// −(μl/nπ)·ü and the mode pulls on the bridge with T·nπ/l, as a speaking
/// string's ([`NoteModel::build_with`]), the key's strings alike. Like a
/// speaking string it loses to the bridge what the bridge's conductance
/// takes, (T/l)·Re Y: fast, the length being short (some 100 dB/s for
/// 5 cm). None when off, or without a board to listen to.
pub fn rear_modes(model: &PianoModel, board: Option<&Soundboard>, sample_rate: f64) -> Option<Vec<Vec<ModeSpec>>> {
    let law = model.sympathetic;
    let sb = model.soundboard.filter(|_| law.enabled && law.rear)?;
    let mut regions = vec![Vec::new(); board::REGIONS];
    let (dt, tau) = (1.0 / sample_rate, std::f64::consts::TAU);
    for note in law.rear_from.max(21)..=108 {
        let np = model.note(note);
        let l = if sb.structure.enabled { sb.structure.back_length_at(note) } else { 0.04 + 0.1 * np.length };
        if !(l > 1e-3) {
            continue;
        }
        let (f0, b) = (np.f0 * np.length / l, np.b * (np.length / l).powi(2));
        let geo = params::StringGeometry::of(&np, model.nonlinear.young).with_length(np.length, l);
        let shapes = board.map(|bd| bd.bridge_shapes(note));
        for n in 1..=64u32 {
            let f = params::partial_freq(f0, b, n);
            if f >= law.max_freq.min(0.45 * sample_rate) {
                break;
            }
            let npi = n as f64 * std::f64::consts::PI;
            let g = C64::new(0.0, 2.0 * dt / (npi * tau * f)).scale(law.gain);
            // (The key's strings move together: one mode that loses to the
            // bridge what each would alone, times their number.)
            let to_bridge = np.strings as f64 * np.tension / l
                * match (board, &shapes) {
                    (Some(bd), Some(sh)) => (bd.bridge_admittance(sh, tau * f).re + bd.edge_conductance(note, tau * f)).max(0.0),
                    _ => model.bridge.admittance(f).re.max(0.0),
                };
            regions[board::region_of(note)].push(ModeSpec {
                freq_hz: f,
                sigma: model.losses.sigma_on(note, f, &geo) + law.rear_loss.max(0.0) + to_bridge,
                gain_re: g.re,
                gain_im: g.im,
                gain2_re: 0.0,
                gain2_im: 0.0,
                out_re: np.strings as f64 * np.tension * npi / l,
                out_im: 0.0,
                out2_re: 0.0,
                out2_im: 0.0,
                out3_re: 0.0,
                out3_im: 0.0,
                damp_w: 0.0,
            });
        }
    }
    Some(regions.iter().map(|m| sym_specs(m, None, sample_rate)).collect())
}

/// A listening mode's response to its bridge input 1 (sympathetic force per
/// bridge acceleration, the bank's own discrete transfer) at ω (rad/s).
fn mode_response(m: &ModeSpec, omega: f64, sample_rate: f64) -> C64 {
    let t = 1.0 / sample_rate;
    let z = C64::polar(1.0, -omega * t);
    let p = C64::polar((-m.sigma * t).exp(), std::f64::consts::TAU * m.freq_hz * t);
    let wb = C64::new(m.out_re, m.out_im) * C64::new(m.gain_re, m.gain_im);
    (wb / (C64::ONE - p * z) + wb.conj() / (C64::ONE - p.conj() * z)).scale(0.5)
}

/// Other free strings' modes within this share of a mode's frequency brake
/// it; farther ones answer off resonance, their mean load already in the
/// board (`SoundboardParams::string_load`).
const BRAKE_WINDOW: f64 = 0.02;

/// Per key and listening mode, the brake 1/(1 − Σ_k A·H_k) on its
/// sympathetic drive: H_k the bridge force of another free key's listening
/// modes per bridge acceleration (near the mode's frequency), A the board's
/// acceleration at the key's region per force at theirs, the free strings
/// taken to move with the bridge at the key's region. Passive strings give
/// |brake| ≤ 1; capped there.
pub fn sym_brakes(notes: &[NoteModel], board_modes: &[BoardMode], free: impl Fn(&NoteModel) -> bool, sample_rate: f64) -> Vec<Vec<C64>> {
    // Every free key's listening modes, by frequency.
    let mut pool: Vec<(f64, usize, usize)> = notes
        .iter()
        .enumerate()
        .filter(|(_, nm)| free(nm))
        .flat_map(|(i, nm)| nm.modes[..nm.n_listen].iter().enumerate().map(move |(j, m)| (m.freq_hz, i, j)))
        .collect();
    pool.sort_by(|a, b| a.0.total_cmp(&b.0));
    crate::par::map(notes.len(), |i| {
        let nm = &notes[i];
        let mut acc = [C64::ZERO; board::REGIONS];
        nm.modes[..nm.n_listen]
            .iter()
            .map(|m| {
                let (lo, hi) = (m.freq_hz * (1.0 - BRAKE_WINDOW), m.freq_hz * (1.0 + BRAKE_WINDOW));
                let start = pool.partition_point(|e| e.0 < lo);
                let omega = std::f64::consts::TAU * m.freq_hz;
                acc.fill(C64::ZERO);
                let mut any = false;
                for &(_, k, j) in pool[start..].iter().take_while(|e| e.0 <= hi).filter(|e| e.1 != i) {
                    acc[notes[k].region] += mode_response(&notes[k].modes[j], omega, sample_rate);
                    any = true;
                }
                if !any {
                    return C64::ONE;
                }
                let diff = (C64::ONE - C64::polar(1.0, -omega / sample_rate)).scale(sample_rate);
                let s = acc.iter().enumerate().filter(|(_, h)| h.norm_sqr() > 0.0).fold(C64::ZERO, |s, (r, &h)| {
                    s + board::transfer(board_modes, r, board::BRIDGE + nm.region, omega, sample_rate) * diff * h
                });
                let b = C64::ONE / (C64::ONE - s);
                if b.norm() > 1.0 { b.scale(1.0 / b.norm()) } else { b }
            })
            .collect()
    })
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
    let coupling = nl.coupling_at(note, model.scale.break_note);
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
        long_out.push((coupling * ea * n as f64 * PI / np.length) as f32);
    }
    // The strike's stretching, d'Alembert while the pulses run free: the
    // string's speed under the hammer is v = F/(2Z) and the strain v²/c²
    // leaves at ±c, so (4/L)·∫y'²·cos(nπx/L)dx = (8/(L·c))·cos(nπx_h/L)·
    // ∫v²(s)·cos(nπc(t − s)/L)ds, x_h from the bridge.
    let (mut strike_rot, mut strike_w) = (Vec::new(), Vec::new());
    if nl.strike_stretch {
        let c = (np.tension / np.mu).sqrt();
        let z2 = np.tension * np.mu;
        let xh = (1.0 - np.strike_pos) * np.length;
        for n in 1..=n_long {
            let r = C64::polar(1.0, n as f64 * PI * c * dt / np.length);
            strike_rot.push((r.re as f32, r.im as f32));
            strike_w.push((nl.strike_scale * 8.0 / (np.length * c) * (n as f64 * PI * xh / np.length).cos() * dt / (4.0 * z2 * ns as f64)) as f32);
        }
    }
    Some(Longitudinal {
        strike_rot,
        strike_w,
        slope,
        weights,
        pole_re,
        pole_im,
        long_pole,
        long_gain,
        long_out,
        ea4: (ea / 4.0) as f32,
        coupling: coupling as f32,
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
        Self::with_resolution(model, board, board_modes, None, None, sample_rate, Resolution { max_modes, ..Resolution::DEFAULT })
    }

    pub fn with_resolution(
        model: &PianoModel,
        board: Option<&Soundboard>,
        board_modes: Option<&[BoardMode]>,
        low: Option<&board::LowDrive>,
        frame: Option<&crate::structure::FrameAdmittance>,
        sample_rate: f64,
        res: Resolution,
    ) -> Self {
        // Independent keys: built in parallel (≈ 0.5 s on one core).
        let mut notes = crate::par::map(88, |i| NoteModel::build_with(model, board, board_modes, low, frame, 21 + i as u8, sample_rate, res));
        if let (true, Some(bm)) = (model.sympathetic.enabled && model.sympathetic.brake, board_modes) {
            let up = sym_brakes(&notes, bm, |nm| !nm.has_damper, sample_rate);
            let down = sym_brakes(&notes, bm, |_| true, sample_rate);
            for ((nm, up), down) in notes.iter_mut().zip(&up).zip(&down) {
                nm.sym_table = ModeTable::new(sample_rate, &sym_specs(&nm.modes, Some(up), sample_rate));
                nm.sym_table_free = ModeTable::new(sample_rate, &sym_specs(&nm.modes, Some(down), sample_rate));
            }
        }
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

    /// The 88 keys, A0 first.
    pub fn notes(&self) -> &[NoteModel] {
        &self.notes
    }

    /// The 88 keys, A0 first, to be completed after the build.
    pub fn notes_mut(&mut self) -> &mut [NoteModel] {
        &mut self.notes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The frame two way: a key's partials lose to the frame at their
    /// agraffes as well as to the board (in all; one coupled mode of a
    /// unison may lose a little less, the frame's admittance mixing them
    /// differently).
    #[test]
    fn the_frame_takes_energy_from_the_strings() {
        let model = PianoModel::default();
        let board = model.board().unwrap();
        let frame = crate::structure::FrameAdmittance::new(&model.frame, &board.params).unwrap();
        let res = Resolution::DEFAULT;
        for note in [84u8, 96] {
            let one = NoteModel::build_with(&model, Some(&board), None, None, None, note, 48_000.0, res);
            let two = NoteModel::build_with(&model, Some(&board), None, None, Some(&frame), note, 48_000.0, res);
            let rate = |nm: &NoteModel| nm.modes.iter().map(|m| m.sigma).sum::<f64>();
            assert!(rate(&two) > rate(&one), "key {note}: {} vs {}", rate(&two), rate(&one));
        }
    }

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

    /// The free strings' brake: never a gain, nothing on the fundamentals
    /// of the undamped top with the dampers down (no other free string
    /// shares them),
    /// and several dB somewhere once every string is free (octaves).
    #[test]
    fn free_strings_brake_where_they_share_a_frequency() {
        let inst = crate::instrument::Instrument::new(PianoModel::default(), 48_000.0);
        let (bm, notes) = (inst.board.as_deref().unwrap(), inst.table.notes());
        let up = sym_brakes(notes, bm, |nm| !nm.has_damper, 48_000.0);
        let down = sym_brakes(notes, bm, |_| true, 48_000.0);
        let db = |b: &C64| 20.0 * b.norm().log10();
        for (i, (u, d)) in up.iter().zip(&down).enumerate() {
            assert!(u.iter().chain(d).all(|b| b.norm() <= 1.0 + 1e-12), "key {}", 21 + i);
            // The halo under the top's notes: D#6–B6's fundamentals.
            if (87..=95).contains(&(21 + i)) {
                let p1 = &u[..notes[i].n_osc];
                assert!(p1.iter().all(|b| db(b) > -0.5), "key {}: {:?}", 21 + i, p1.iter().map(db).fold(0.0, f64::min));
            }
        }
        let strongest = down.iter().flatten().map(db).fold(0.0, f64::min);
        assert!(strongest < -3.0, "strongest brake {strongest} dB");
    }

    /// The rear duplex (`sympathetic.rear`): none by default; on, each key
    /// from `rear_from` up has its rear length's modes in its region, at the
    /// speaking string's frequency times L/l, losing to the bridge on top
    /// of the string's own losses, the more the shorter the length.
    #[test]
    fn the_rear_lengths_listen_from_their_lowest_key_up() {
        let mut model = PianoModel::default();
        assert!(rear_modes(&model, None, 48_000.0).is_none());
        model.sympathetic.rear = true;
        let board = model.board().unwrap();
        let regions = rear_modes(&model, Some(&board), 48_000.0).unwrap();
        assert_eq!(regions.len(), board::REGIONS);
        let from = model.sympathetic.rear_from;
        assert!(regions[..board::region_of(from)].iter().all(|r| r.is_empty()));
        assert!(regions[board::region_of(from)..].iter().all(|r| !r.is_empty()));
        // C7's rear length: its first mode among its region's.
        let np = model.note(96);
        let st = model.soundboard.unwrap().structure;
        let l = if st.enabled { st.back_length_at(96) } else { 0.04 + 0.1 * np.length };
        let f = params::partial_freq(np.f0 * np.length / l, np.b * (np.length / l).powi(2), 1);
        let m = regions[board::region_of(96)].iter().find(|m| (m.freq_hz - f).abs() < 1e-6).expect("C7's rear length");
        assert!(m.sigma > 2.0 * model.losses.sigma(96, f), "{} against {}", m.sigma, model.losses.sigma(96, f));
        assert!(m.out_re > 0.0 && (m.gain_re != 0.0 || m.gain_im != 0.0));
        let all = regions.iter().flatten();
        assert!(all.clone().all(|m| m.freq_hz < model.sympathetic.max_freq && m.sigma > 0.0));
        // Without the key's switch, a board alone changes nothing.
        model.sympathetic.enabled = false;
        assert!(rear_modes(&model, Some(&board), 48_000.0).is_none());
    }

    /// The front duplex (`frame.duplex`): nothing below the capo bar, and
    /// nothing with the bar alone; under it the partials move by cents and
    /// the lengths in front of the bar add their modes after them.
    #[test]
    fn the_front_duplex_adds_its_modes_under_the_capo() {
        let build = |m: &PianoModel, note| NoteModel::build(m, None, None, note, 48_000.0, 768);
        let plain = PianoModel::default();
        let mut capo = plain;
        capo.frame.capo_bar = true;
        let mut duplex = capo;
        duplex.frame.duplex = true;
        for note in [60u8, 96] {
            assert_eq!(build(&plain, note).modes, build(&capo, note).modes);
        }
        let (a, b) = (build(&capo, 60), build(&duplex, 60));
        assert_eq!(a.modes, b.modes);
        assert_eq!(b.n_partials, b.clusters.len());
        let (a, b) = (build(&capo, 96), build(&duplex, 96));
        let n = b.n_osc;
        assert!(b.n_partials + 1 >= a.clusters.len() && b.n_partials <= a.clusters.len());
        assert!(b.clusters.len() >= b.n_partials + 2, "{} clusters for {} partials", b.clusters.len(), b.n_partials);
        assert_eq!(b.modes.len(), b.clusters.len() * n);
        assert_eq!(b.phys.len(), b.modes.len());
        // The partials: within a third of a semitone of the plain string's
        // (the first within 2 cents: it is the one tuned), all decaying.
        for (i, (p, q)) in a.phys.iter().zip(&b.phys[..b.n_partials * n]).enumerate() {
            let cents = 1200.0 * (q.freq / p.freq).log2();
            assert!(cents.abs() < if i < n { 2.0 } else { 40.0 }, "partial {}: {cents} cents", p.k);
            assert!(q.k == p.k && q.shape.is_some());
        }
        // The short lengths' modes: 4 to 8 kHz for the first, a slope at
        // the bridge well under a partial's there, a decay of their own.
        for d in &b.phys[b.n_partials * n..] {
            assert!(d.k == 0 && d.freq > 4000.0 && d.freq < 0.45 * 48_000.0);
            let sh = d.shape.unwrap();
            assert!(sh.bridge.abs() < 0.9 * sh.nu, "{sh:?}");
        }
        let first = &b.phys[b.n_partials * n];
        assert!(first.freq < 8000.0 && first.sigma > capo.losses.sigma(96, first.freq));
        assert!(b.modes.iter().all(|m| m.sigma > 0.0 && m.freq_hz > 0.0));
        // They take nothing from the bridge and do not drive the frame.
        assert!(b.modes[b.n_partials * n..].iter().all(|m| m.gain_re == 0.0 && m.gain_im == 0.0 && m.out2_re == 0.0 && (m.out_re != 0.0 || m.out_im != 0.0)));
    }

    /// Lengths in front of the capo longer than the speaking ones (the
    /// controls go to 120 mm, C8 speaks 52 mm): the first partial keeps its
    /// tuning and the others their places, the short lengths' first modes
    /// falling below the note.
    #[test]
    fn a_front_length_longer_than_the_speaking_one_keeps_the_top_in_tune() {
        let build = |m: &PianoModel, note| NoteModel::build(m, None, None, note, 48_000.0, 768);
        let mut capo = PianoModel::default();
        capo.frame.capo_bar = true;
        for front in [0.06, 0.12] {
            let mut duplex = capo;
            duplex.frame.duplex = true;
            (duplex.frame.duplex_far, duplex.frame.duplex_near) = (front, front);
            for note in [96u8, 104, 108] {
                let (a, b) = (build(&capo, note), build(&duplex, note));
                let n = b.n_osc;
                assert!(b.n_partials > 0);
                for (i, (p, q)) in a.phys.iter().zip(&b.phys[..b.n_partials * n]).enumerate() {
                    let cents = 1200.0 * (q.freq / p.freq).log2();
                    assert!(q.k == p.k && cents.abs() < if i < n { 2.0 } else { 40.0 }, "key {note}, {front} m in front, partial {}: {cents} cents", p.k);
                }
                assert!(b.modes.iter().all(|m| m.sigma > 0.0 && m.freq_hz > 0.0));
            }
        }
    }

    /// Every key, including single strings, has a fast and a slow component
    /// per partial (in-phase vertical vs out-of-phase/horizontal motion).
    #[test]
    fn each_string_s_own_inharmonicity_spreads_the_upper_partials_alone() {
        let base = PianoModel::default();
        let model = PianoModel { unison: crate::params::UnisonLaw { b_spread: 0.05, ..base.unison }, ..base };
        let note = 84u8;
        let (a, b) = (NoteModel::build(&base, None, None, note, 48_000.0, 768), NoteModel::build(&model, None, None, note, 48_000.0, 768));
        let ns = a.params.strings;
        let split = |nm: &NoteModel, ki: usize| {
            let f: Vec<f64> = (0..ns).map(|j| nm.phys[ki * nm.n_osc + j].freq).collect();
            f.iter().cloned().fold(f64::MIN, f64::max) - f.iter().cloned().fold(f64::MAX, f64::min)
        };
        // Partial 1 of every string stays where the tuning puts it.
        for j in 0..ns {
            assert!((a.phys[j].freq - b.phys[j].freq).abs() < 1e-9 * a.phys[j].freq, "string {j}");
        }
        // A high partial splits by its own amount, no longer k times partial 1's.
        let k = 6;
        assert!((split(&a, k - 1) / split(&a, 0) - (a.phys[(k - 1) * a.n_osc].freq / a.phys[0].freq)).abs() < 0.05);
        assert!((split(&b, k - 1) - split(&a, k - 1)).abs() > 0.2 * split(&a, k - 1), "{} vs {}", split(&b, k - 1), split(&a, k - 1));
    }

    #[test]
    fn the_bridge_s_yield_damps_the_high_partials_alone() {
        let base = PianoModel::default();
        let mut sb = base.soundboard.unwrap();
        sb.bridge_yield = true;
        sb.yield_conductance = 2e-3;
        sb.yield_corner = 3000.0;
        let model = PianoModel { soundboard: Some(sb), ..base };
        let (b0, b1) = (base.board().unwrap(), model.board().unwrap());
        let note = 72u8;
        let a = NoteModel::build(&base, Some(&b0), None, note, 48_000.0, 768);
        let b = NoteModel::build(&model, Some(&b1), None, note, 48_000.0, 768);
        let fastest = |nm: &NoteModel, ki: usize| nm.modes[ki * nm.n_osc..(ki + 1) * nm.n_osc].iter().map(|m| m.sigma).fold(0.0, f64::max);
        // Partial 1 (523 Hz) within 3 %; partial 9 (4.9 kHz) far faster.
        assert!((fastest(&b, 0) / fastest(&a, 0) - 1.0).abs() < 0.03, "{} vs {}", fastest(&b, 0), fastest(&a, 0));
        assert!(fastest(&b, 8) > 2.0 * fastest(&a, 8), "{} vs {}", fastest(&b, 8), fastest(&a, 8));
        // Off, the same law at any value is nothing.
        let mut off = sb;
        off.bridge_yield = false;
        let c = NoteModel::build(&PianoModel { soundboard: Some(off), ..base }, Some(&PianoModel { soundboard: Some(off), ..base }.board().unwrap()), None, note, 48_000.0, 768);
        assert!(a.modes.iter().zip(&c.modes).all(|(x, y)| x.sigma == y.sigma && x.freq_hz == y.freq_hz));
    }

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
