//! The board's forced response at the strings' own frequencies
//! (`soundboard.forced_radiation`, docs/forced_radiation.md).
//!
//! The board's bank radiates each mode through one complex weight per
//! microphone, its Rayleigh integral at the mode's own frequency ω_n: exact
//! for a mode ringing freely. A string's partial at ω drives every mode off
//! its resonance, and what the modes radiate together depends on the
//! wavelength at ω. For each key and each cluster of coupled modes (a
//! partial of its unison) at ω, [`corrections`] gives the difference
//! ΔH(ω) = Σ_n V_n(ω)·R_n(ω) − (the bank's transfer), the sum over the
//! built board's modes, V_n the modes' velocity response to the key's
//! bridge force (the bank's own one-poles), R_n(ω) every mode's direct
//! field at ω (front face through the case, its lid image, back face and
//! its wall image). The bank's transfer is what the engine renders: each
//! mode's weight at ω_n as acceleration weights, through its delay zones
//! (`radiation_delays`), driven as the engine drives it: at the key itself
//! for the modes below `exact_below`, at the centres of the key's regions
//! for the others ([`Forced::drive`]). The case's field (`case_field`) is statistical by
//! design and stays the bank's; so do the modes above the built board's.
//! The coupled modes then add Re(ΔH·w·z) at each microphone, w their bridge
//! force weight: the correction rides on the strings' own states.

use crate::board;
use crate::complex::C64;
use crate::soundboard::{Soundboard, RAD_ALL, RAD_CASE};
use std::f64::consts::TAU;

/// Clusters above this frequency (Hz) take no correction: the built board's
/// modes stop at 6 kHz on the concert grand and the bench's measures at
/// 8 kHz.
pub const FORCED_MAX_HZ: f64 = 8000.0;

/// Version of the cached tables' contents.
const VERSION: u32 = 2;

/// The bank's response at ω of one damped mode (frequency `freq_hz`, decay
/// `sigma`) to a real input of gain `g`, read through the complex weight
/// `w`, as [`board::transfer_with`] sums it.
pub fn mode_response(freq_hz: f64, sigma: f64, g: f64, w: C64, omega: f64, sample_rate: f64) -> C64 {
    let t = 1.0 / sample_rate;
    let p = C64::polar((-sigma * t).exp(), TAU * freq_hz * t);
    let b = C64::new(g, 0.0);
    let a = b / (C64::ONE - p * C64::polar(1.0, -omega * t)).scale(2.0);
    let bb = b / (C64::ONE - p * C64::polar(1.0, omega * t)).scale(2.0);
    w * a + (w * bb).conj() + C64::new(-0.5 * w.re * g, 0.0)
}

/// What the corrections of one board need, computed once: the paths from
/// the nodes, the modes' own weights as the bank holds them.
pub struct Forced<'a> {
    sb: &'a Soundboard,
    ff: crate::soundboard::ForcedField,
    /// Per built-board mode (`ff.modes`): frequency (Hz), decay (1/s), the
    /// impulse-shape factor, and its direct weights per zone `[mic·count + q]`.
    modes: Vec<(f64, f64, C64, Vec<C64>)>,
    /// Delay zones: count, and each zone's delay (samples).
    count: usize,
    step: usize,
    sample_rate: f64,
    /// The first board modes, which the engine drives at each key itself
    /// (those below `exact_below`, [`board::LowDrive`]); the others take
    /// the key's force at its regions' centres.
    keyed: usize,
    /// The bridge shapes of every region's centre, `[region][mode]`.
    regions: Vec<Vec<f64>>,
}

/// How many of the board's modes the engine drives at each key itself: as
/// [`board::LowDrive::new`] counts them among the modes the bank runs.
fn keyed_modes(sb: &Soundboard, sample_rate: f64) -> usize {
    let below = sb.params.exact_below;
    if below <= 0.0 {
        return 0;
    }
    let nyq = 0.45 * sample_rate;
    sb.modes.iter().map(|md| md.omega / TAU).filter(|f| *f < nyq).take_while(|f| *f < below).count()
}

impl<'a> Forced<'a> {
    /// None when the board has no built modes radiating from their nodes
    /// ([`Soundboard::forced_field`]) or does not radiate.
    pub fn new(sb: &'a Soundboard, sample_rate: f64) -> Option<Self> {
        if !sb.params.radiate {
            return None;
        }
        let ff = sb.forced_field()?;
        let zones = board::zones_for(sb, sample_rate);
        let mics = ff.mics;
        // The direct field only, by zone (the case's field apart).
        let (count, step, own): (usize, usize, Vec<Vec<C64>>) = match zones {
            Some((z, from, width)) => {
                let per = z.count + 1;
                let zr = sb.radiation_zoned(from, width, z.count, true);
                (z.count, z.step, zr.into_iter().map(|r| (0..mics).flat_map(|m| (0..z.count).map(move |q| (m, q))).map(|(m, q)| r[m * per + q]).collect()).collect())
            }
            None => (1, 0, sb.radiation_parts(None, RAD_ALL & !RAD_CASE)),
        };
        let nyq = 0.45 * sample_rate;
        let modes = ff
            .modes
            .iter()
            .map(|&(n, _)| {
                let md = &sb.modes[n];
                let f = md.omega / TAU;
                // (A mode the bank does not run: no part in either sum.)
                let w = if f < nyq { own[n].clone() } else { vec![C64::ZERO; mics * count] };
                (f, sb.sigma(md), C64::new(1.0, sb.sigma(md) / md.omega), w)
            })
            .collect();
        let regions = (0..board::REGIONS).map(|r| sb.bridge_shapes(board::region_center_note(r))).collect();
        Some(Self { sb, ff, modes, count, step, sample_rate, keyed: keyed_modes(sb, sample_rate), regions })
    }

    pub fn mics(&self) -> usize {
        self.ff.mics
    }

    /// The engine's gain from `note`'s bridge force at ω (rad/s) to board
    /// mode `n` (per sample, as the bank's inputs), `shapes` the key's own
    /// bridge shapes. The modes below `exact_below` are driven at the key
    /// itself. The others take the force at the centres of its regions
    /// (`Voice::mix`): whole at its own region when the low modes have
    /// their own drive, else its low band shared between the two regions
    /// around the key ([`board::region_weights`]).
    pub fn drive(&self, note: u8, shapes: &[f64], n: usize, omega: f64) -> C64 {
        let t = 1.0 / self.sample_rate;
        if n < self.keyed {
            return C64::new(shapes[n] * t, 0.0);
        }
        let own = self.regions[board::region_of(note)][n];
        if self.keyed > 0 {
            return C64::new(own * t, 0.0);
        }
        let lp = crate::voice::share_lowpass(omega, self.sample_rate);
        let shared = board::region_weights(note).iter().fold(0.0, |s, &(r, w)| s + w as f64 * self.regions[r][n]);
        (lp.scale(shared) + (C64::ONE - lp).scale(own)).scale(t)
    }

    /// ΔH at each microphone (Pa per N of bridge force) for `note`'s bridge
    /// force at frequency `f` (Hz), `shapes` its bridge shapes
    /// ([`Soundboard::bridge_shapes`]): the exact field of the modes driven
    /// at the key's own point, minus what the bank renders of them.
    pub fn delta(&self, note: u8, shapes: &[f64], f: f64) -> Vec<C64> {
        let w = TAU * f;
        self.delta_with(shapes, |n| self.drive(note, shapes, n, w), f)
    }

    /// [`Self::delta`] with the bank's drive of each mode given (`bank(n)`,
    /// per sample).
    fn delta_with(&self, shapes: &[f64], drive: impl Fn(usize) -> C64, f: f64) -> Vec<C64> {
        let sr = self.sample_rate;
        let w = TAU * f;
        let t = 1.0 / sr;
        let nyq = 0.45 * sr;
        let d_w = board::mic_differentiator(w, sr);
        let mics = self.ff.mics;
        let mut vel = Vec::with_capacity(self.modes.len());
        let mut bank = vec![C64::ZERO; mics];
        for (&(n, _), (fm, sigma, shape, own)) in self.ff.modes.iter().zip(&self.modes) {
            if *fm >= nyq {
                vel.push(C64::ZERO);
                continue;
            }
            vel.push(mode_response(*fm, *sigma, shapes[n] * t, *shape, w, sr));
            // A real drive is the bank's input gain; one with a phase (the
            // shared low band's filter) a filter ahead of the mode.
            let g = drive(n);
            let d_n = board::mic_differentiator(TAU * fm, sr);
            for (m, b) in bank.iter_mut().enumerate() {
                for q in 0..self.count {
                    let wq = own[m * self.count + q] * *shape / d_n;
                    let h = if g.im == 0.0 { mode_response(*fm, *sigma, g.re, wq, w, sr) } else { mode_response(*fm, *sigma, 1.0, wq, w, sr) * g };
                    *b += h * d_w * C64::polar(1.0, -w * (q * self.step) as f64 / sr);
                }
            }
        }
        let exact = self.sb.forced_direct(&self.ff, w, &vel);
        exact.iter().zip(&bank).map(|(e, b)| *e - *b).collect()
    }
}

/// ΔH for every key's clusters: `freqs[key][cluster]` (Hz; 0 or above
/// [`FORCED_MAX_HZ`]: none), `[key][cluster][mic]`. From the disk when the
/// same board, microphones and frequencies were computed before.
pub fn corrections(sb: &Soundboard, freqs: &[Vec<f64>], sample_rate: f64) -> Option<Vec<Vec<Vec<C64>>>> {
    let text = format!("{VERSION} {:?} {sample_rate} {freqs:?}", sb.params);
    let key = text.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ b as u64).wrapping_mul(0x0000_0100_0000_01b3));
    let path = cache_dir().map(|d| d.join(format!("{key:016x}.bin")));
    if let Some(c) = path.as_deref().and_then(|p| read(p, key, freqs)) {
        return Some(c);
    }
    let forced = Forced::new(sb, sample_rate)?;
    let out = crate::par::map(freqs.len(), |i| {
        let shapes = sb.bridge_shapes(21 + i as u8);
        freqs[i].iter().map(|&f| if f > 0.0 && f < FORCED_MAX_HZ { forced.delta(21 + i as u8, &shapes, f) } else { vec![C64::ZERO; forced.mics()] }).collect::<Vec<_>>()
    });
    if let Some(p) = &path {
        let _ = write(p, key, &out);
    }
    Some(out)
}

fn cache_dir() -> Option<std::path::PathBuf> {
    let var = |name: &str| std::env::var_os(name).filter(|v| !v.is_empty()).map(std::path::PathBuf::from);
    var("XDG_CACHE_HOME").or_else(|| var("HOME").map(|h| h.join(".cache"))).or_else(|| var("LOCALAPPDATA")).map(|d| d.join("unsampled-piano").join("forced"))
}

fn write(path: &std::path::Path, key: u64, c: &[Vec<Vec<C64>>]) -> std::io::Result<()> {
    let mut b = Vec::new();
    b.extend(key.to_le_bytes());
    for x in c.iter().flatten().flatten() {
        b.extend(x.re.to_le_bytes());
        b.extend(x.im.to_le_bytes());
    }
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d)?;
    }
    // Written aside, then renamed: a reader never sees half a file.
    let tmp = path.with_extension(format!("tmp{}", std::process::id()));
    std::fs::write(&tmp, b)?;
    std::fs::rename(tmp, path)
}

fn read(path: &std::path::Path, key: u64, freqs: &[Vec<f64>]) -> Option<Vec<Vec<Vec<C64>>>> {
    let b = std::fs::read(path).ok()?;
    if b.len() < 8 || u64::from_le_bytes(b[..8].try_into().ok()?) != key {
        return None;
    }
    let n: usize = freqs.iter().map(|f| f.len()).sum();
    let body = &b[8..];
    if n == 0 || body.len() % (16 * n) != 0 {
        return None;
    }
    let mics = body.len() / (16 * n);
    let mut vals = body.chunks_exact(16).map(|c| C64::new(f64::from_le_bytes(c[..8].try_into().unwrap()), f64::from_le_bytes(c[8..].try_into().unwrap())));
    Some(freqs.iter().map(|f| f.iter().map(|_| (0..mics).map(|_| vals.next().unwrap()).collect()).collect()).collect())
}

/// The mean frequency (Hz) of each cluster of a key's coupled modes.
pub fn cluster_freqs(nm: &crate::note_model::NoteModel) -> Vec<f64> {
    let n = nm.n_osc.max(1);
    (0..nm.clusters.len())
        .map(|k| {
            let m = &nm.modes[(k * n).min(nm.modes.len())..((k + 1) * n).min(nm.modes.len())];
            if m.is_empty() { 0.0 } else { m.iter().map(|s| s.freq_hz).sum::<f64>() / m.len() as f64 }
        })
        .collect()
}

/// The voice's tables for a key whose clusters take `delta[cluster][mic]`:
/// per pair of microphones, the key's coupled modes with no input and as
/// outputs 1 and 2 the pressure weights ΔH·w·`gain`, w the mode's
/// bridge-force weight (added after the engine's differentiator).
pub fn tables(nm: &crate::note_model::NoteModel, delta: &[Vec<C64>], gain: f64, sample_rate: f64) -> Vec<crate::modal::ModeTable> {
    tables_for(&nm.modes, nm.n_osc, delta, gain, sample_rate)
}

/// [`tables`] for coupled modes `modes`, `n_osc` to a cluster.
pub fn tables_for(modes: &[crate::modal::ModeSpec], n_osc: usize, delta: &[Vec<C64>], gain: f64, sample_rate: f64) -> Vec<crate::modal::ModeTable> {
    let mics = delta.iter().map(|d| d.len()).max().unwrap_or(0);
    if mics == 0 || delta.iter().flatten().all(|d| *d == C64::ZERO) {
        return Vec::new();
    }
    let n = n_osc.max(1);
    (0..mics.div_ceil(2))
        .map(|pair| {
            let specs: Vec<crate::modal::ModeSpec> = modes
                .iter()
                .enumerate()
                .map(|(i, m)| {
                    let w = C64::new(m.out_re, m.out_im).scale(gain);
                    let at = |mic: usize| delta.get(i / n).and_then(|c| c.get(mic)).map_or(C64::ZERO, |h| *h * w);
                    let (a, b) = (at(2 * pair), at(2 * pair + 1));
                    crate::modal::ModeSpec {
                        freq_hz: m.freq_hz,
                        sigma: m.sigma,
                        gain_re: 0.0,
                        gain_im: 0.0,
                        out_re: a.re,
                        out_im: a.im,
                        out2_re: b.re,
                        out2_im: b.im,
                        out3_re: 0.0,
                        out3_im: 0.0,
                        gain2_re: 0.0,
                        gain2_im: 0.0,
                        damp_w: m.damp_w,
                    }
                })
                .collect();
            crate::modal::ModeTable::new(sample_rate, &specs)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::soundboard::SoundboardParams;

    /// A small built board radiating from its nodes (as soundboard's
    /// `the_built_board_radiates_from_its_nodes`), the case's field off.
    fn board() -> Soundboard {
        board_with(0.0)
    }

    /// The same, its modes below `exact_below` driven at each key.
    fn board_with(exact_below: f64) -> Soundboard {
        let structure = crate::fem::BoardStructure { enabled: true, step: 0.04, below: 400.0, ..crate::fem::BoardStructure::STEINWAY_D };
        let p = SoundboardParams { a: 2.124, b: 1.46, rho_h: 14.7, crown_freq: 0.0, f_max: 400.0, structure, grid_step: 0.08, case_field: false, exact_below, ..SoundboardParams::GRAND_2M }.resolved();
        Soundboard::new(p)
    }

    /// The transfer at ω from `note`'s bridge force to a mic of the modes
    /// `keep` selects, as the engine renders them (`modes` its bank, with
    /// acceleration weights): the first `keyed` driven at the key itself
    /// (`LowDrive`), the others at the centres of its regions, its low band
    /// shared between two of them when no mode is keyed (`Voice::mix`).
    #[allow(clippy::too_many_arguments)]
    fn rendered(sb: &Soundboard, modes: &[board::BoardMode], keyed: usize, keep: impl Fn(usize) -> bool, note: u8, mic: usize, w: f64, sr: f64) -> C64 {
        let shapes = sb.bridge_shapes(note);
        let low = board::transfer_with(&modes[..keyed], |i, _| if keep(i) { shapes[i] / sr } else { 0.0 }, mic, w, sr);
        let region = |q: usize| board::transfer_with(&modes[keyed..], |i, m| if keep(keyed + i) { m.inputs[q] } else { 0.0 }, mic, w, sr);
        let own = region(board::region_of(note));
        let high = if keyed > 0 {
            own
        } else {
            let lp = crate::voice::share_lowpass(w, sr);
            let shared = board::region_weights(note).iter().fold(C64::ZERO, |s, &(q, wq)| s + region(q).scale(wq as f64));
            lp * shared + (C64::ONE - lp) * own
        };
        (low + high) * board::mic_differentiator(w, sr)
    }

    /// The bank's transfer at ω from a key's bridge force to its mics, as
    /// the engine renders it, and the exact one (every built mode's
    /// integral at ω on its response to the force at the key's own point;
    /// the other modes as the bank has them, the correction leaves them).
    fn transfers(sb: &Soundboard, note: u8, f: f64, sr: f64) -> (Vec<C64>, Vec<C64>) {
        let mut modes = board::board_modes(sb, sr);
        let keyed = board::LowDrive::new(sb, &modes, sb.params.exact_below, sr).filter(|_| sb.params.exact_below > 0.0).map_or(0, |l| l.n);
        let w = TAU * f;
        let at = sb.radiation_parts_of(Some(w), RAD_ALL & !RAD_CASE, modes.len());
        let shapes = sb.bridge_shapes(note);
        let exact_fem: Vec<C64> = (0..2)
            .map(|mic| {
                modes.iter().zip(&sb.modes).zip(&at).enumerate().filter(|(_, ((_, md), _))| md.fem.is_some()).fold(C64::ZERO, |s, (i, ((m, md), r))| {
                    s + mode_response(m.freq_hz, m.sigma, shapes[i] / sr, C64::new(1.0, sb.sigma(md) / md.omega), w, sr) * r[mic]
                })
            })
            .collect();
        board::to_acceleration_weights(&mut modes, sr);
        let bank: Vec<C64> = (0..2).map(|mic| rendered(sb, &modes, keyed, |_| true, note, mic, w, sr)).collect();
        let exact = (0..2).map(|mic| exact_fem[mic] + rendered(sb, &modes, keyed, |i| sb.modes[i].fem.is_none(), note, mic, w, sr)).collect();
        (bank, exact)
    }

    #[test]
    fn the_node_sum_is_each_modes_integral() {
        let sb = board();
        let ff = sb.forced_field().expect("radiates from its nodes");
        let w = TAU * 333.0;
        let at = sb.radiation_parts(Some(w), RAD_ALL & !RAD_CASE);
        for i in [0, 3, 7] {
            let mut vel = vec![C64::ZERO; ff.modes.len()];
            vel[i] = C64::new(0.3, -1.2);
            let got = sb.forced_direct(&ff, w, &vel);
            for (mic, g) in got.iter().enumerate() {
                let want = vel[i] * at[ff.modes[i].0][mic];
                assert!((*g - want).norm() < 1e-9 * want.norm().max(1e-30), "mode {i} mic {mic}: {g:?} against {want:?}");
            }
        }
    }

    /// The bank plus the correction is the exact transfer, where the bank
    /// alone is off; a steady sine force rendered through the correction's
    /// bank comes out at the correction's level and phase.
    #[test]
    fn a_steady_sine_reaches_the_mics_at_the_exact_transfer() {
        let sr = 48_000.0;
        let sb = board();
        let forced = Forced::new(&sb, sr).unwrap();
        let note = 52;
        let f = 287.0;
        let (bank, exact) = transfers(&sb, note, f, sr);
        let delta = forced.delta(note, &sb.bridge_shapes(note), f);
        let mut off = 0.0f64;
        for mic in 0..2 {
            let fixed = bank[mic] + delta[mic];
            assert!((fixed - exact[mic]).norm() < 1e-6 * exact[mic].norm(), "mic {mic}: {fixed:?} against {exact:?}");
            off = off.max((20.0 * (bank[mic].norm() / exact[mic].norm()).log10()).abs() + (bank[mic] / exact[mic]).arg().abs());
        }
        assert!(off > 0.5, "the bank alone is already exact here: {off}");
        // One string mode at f, force Re(z): its correction rendered.
        let spec = crate::modal::ModeSpec { freq_hz: f, sigma: 0.2, gain_re: 0.0, gain_im: 0.0, out_re: 1.0, out_im: 0.0, ..Default::default() };
        let mut string = crate::modal::ModalBank::new(sr, 16, 256);
        string.set_modes([spec]);
        string.set_state(0, 1.0, 0.0);
        let t = &tables_for(&[spec], 1, &[delta.clone()], 1.0, sr)[0];
        let mut follow = crate::modal::ModalBank::new(sr, 16, 256);
        follow.load(t);
        let (mut a, mut b, mut force) = (vec![0.0f32; 256], vec![0.0f32; 256], vec![0.0f32; 256]);
        let mut z = Vec::new();
        let mut p = Vec::new();
        for _ in 0..8 {
            a.fill(0.0);
            b.fill(0.0);
            force.fill(0.0);
            follow.follow(&string);
            z.push(string.state(0));
            string.process(None, &mut force);
            follow.process2(None, &mut a, &mut b);
            for (x, y) in a.iter().zip(&b) {
                p.push([*x, *y]);
            }
        }
        // The bank's state z_t: a sample in the middle of block 4.
        let (zr, zi) = z[4];
        let pole = C64::polar((-0.2f64 / sr).exp(), TAU * f / sr);
        for k in [1usize, 100, 200] {
            let zt = C64::new(zr, zi) * C64::polar(pole.norm().powi(k as i32 + 1), pole.arg() * (k + 1) as f64);
            for mic in 0..2 {
                let want = (delta[mic] * zt).re;
                let got = p[4 * 256 + k][mic] as f64;
                assert!((got - want).abs() < 0.01 * delta[mic].norm(), "mic {mic} sample {k}: {got} against {want}");
            }
        }
    }

    /// The correction subtracts what the engine renders, whatever drives
    /// the modes: all at the key (`exact_below` above them), all at the
    /// regions' centres (0, the low band shared between two regions), or
    /// the low ones at the key and the others at its region.
    #[test]
    fn the_bank_is_subtracted_as_the_engine_drives_it() {
        let sr = 48_000.0;
        for exact_below in [0.0, 200.0, 1000.0] {
            let sb = board_with(exact_below);
            let forced = Forced::new(&sb, sr).unwrap();
            let modes = board::board_modes(&sb, sr);
            let keyed = board::LowDrive::new(&sb, &modes, exact_below, sr).filter(|_| exact_below > 0.0).map_or(0, |l| l.n);
            assert_eq!(forced.keyed, keyed, "exact_below {exact_below}");
            assert!(match exact_below as u32 {
                0 => keyed == 0,
                200 => keyed > 0 && keyed < modes.len(),
                _ => keyed == modes.len(),
            });
            for (note, f) in [(21u8, 385.0), (52, 287.0), (84, 131.0), (108, 350.0)] {
                let (bank, exact) = transfers(&sb, note, f, sr);
                let shapes = sb.bridge_shapes(note);
                let delta = forced.delta(note, &shapes, f);
                for mic in 0..2 {
                    let fixed = bank[mic] + delta[mic];
                    assert!((fixed - exact[mic]).norm() < 1e-6 * exact[mic].norm(), "exact_below {exact_below} note {note} mic {mic}: {fixed:?} against {:?}", exact[mic]);
                }
            }
        }
        // With every mode driven at the regions, the bank is not the one a
        // force at the key's own point would give: A0's whole force goes to
        // the first region, centred two keys above.
        let sb = board();
        let forced = Forced::new(&sb, sr).unwrap();
        let (bank, exact) = transfers(&sb, 21, 385.0, sr);
        let shapes = sb.bridge_shapes(21);
        let at_key = forced.delta_with(&shapes, |i| C64::new(shapes[i] / sr, 0.0), 385.0);
        assert!((bank[0] + at_key[0] - exact[0]).norm() > 0.05 * exact[0].norm(), "the regions' drive is the key's here");
    }

    /// A mode driven at its own frequency radiates as the bank has it.
    #[test]
    fn a_mode_at_its_own_frequency_is_unchanged() {
        let sr = 48_000.0;
        let sb = board();
        let forced = Forced::new(&sb, sr).unwrap();
        let (x, y) = sb.params.bridge_point(60);
        let mut modes = board::board_modes_heard_at(&sb, &[(x, y); board::REGIONS], sr);
        board::to_acceleration_weights(&mut modes, sr);
        for &(n, _) in forced.ff.modes.iter().take(10) {
            // That mode alone, at its frequency.
            let mut shapes = vec![0.0; sb.modes.len()];
            shapes[n] = sb.bridge_shapes(60)[n];
            let f = sb.modes[n].omega / TAU;
            // The bank driven as the exact sum is: at the key's point.
            let d = forced.delta_with(&shapes, |i| C64::new(shapes[i] / sr, 0.0), f);
            let one: Vec<board::BoardMode> = modes.iter().enumerate().map(|(i, m)| if i == n { *m } else { board::BoardMode { inputs: [0.0; board::REGIONS], ..*m } }).collect();
            // Within the mode's damping: the bank radiates its response's
            // negative-frequency part with the conjugate weight, of relative
            // size σ/ω at resonance.
            let loss = sb.sigma(&sb.modes[n]) / sb.modes[n].omega;
            for (mic, dm) in d.iter().enumerate() {
                let h = board::mic_transfer(&one, 0, mic, TAU * f, sr);
                assert!(dm.norm() < (0.02 + 2.0 * loss) * h.norm(), "{f} Hz (sigma/omega {loss:.3}) mic {mic}: {dm:?} against {h:?}");
            }
        }
    }

    /// Off (the default), no key carries a correction.
    #[test]
    fn off_adds_nothing() {
        let model = crate::params::PianoModel::default();
        assert!(!model.soundboard.unwrap().forced_radiation);
        let inst = crate::instrument::Instrument::new(model, 48_000.0);
        assert!(inst.table.notes().iter().all(|n| n.forced.is_empty()));
    }
}
