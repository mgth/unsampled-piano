//! The board's forced radiation at a string partial's frequency, two ways.
//! For keys `--from`..=108 by `--step` and each of their partials up to
//! `--fmax` Hz, the transfer from the key's bridge force (at its own bridge
//! point, every board mode) to each microphone at the partial's frequency ω:
//! as the engine renders it (each mode's radiation weight at its own
//! frequency ω_n, acceleration weights, the bank's real one-poles) and
//! exactly (every mode's weight at ω, applied to the same modes' velocity
//! response). The case's diffuse field (`case_field`) is statistical by
//! design (a pseudo-random phase per mode and microphone): it is the
//! engine's in both. One line per partial and microphone:
//!   note k f mic  eng_re eng_im  ex_re ex_im  case_re case_im
//!   [front_eng front_ex back_eng back_ex image_eng image_ex: re im each, with --parts]
//!   [|bridge acceleration per unit force| (m/s² per N), with --accel]
//!   [the correction `soundboard.forced_radiation` adds (`forced::Forced::delta`:
//!    the built board's modes only, exact minus the bank's): re im, with --delta]
//! (pressure per unit force, Pa/N). The partials are the strings' own
//! (their mean frequency over the unison's oscillators).
//!   cargo run --release -p piano-render --example forced_radiation -- <preset.toml> [listening.toml …] [--step N] [--from N] [--fmax F] [--parts] [--accel] [--delta]
use piano_core::board::{board_modes_heard_at, mic_transfer, mic_transfer_folded, to_acceleration_weights, zones_for, BoardMode};
use piano_core::complex::C64;
use piano_core::instrument::REGIONS;
use piano_core::params::PianoModel;
use piano_core::soundboard::{RAD_ALL, RAD_BACK, RAD_CASE, RAD_FRONT, RAD_IMAGE};
use std::f64::consts::TAU;

/// The digital velocity response of one bank mode at ω, per unit input,
/// with a complex output weight `w` (as `board::transfer_with` sums it).
fn mode_response(m: &BoardMode, g: f64, w: C64, omega: f64, sr: f64) -> C64 {
    let t = 1.0 / sr;
    let p = C64::polar((-m.sigma * t).exp(), TAU * m.freq_hz * t);
    let b = C64::new(g, 0.0);
    let a = b / (C64::ONE - p * C64::polar(1.0, -omega * t)).scale(2.0);
    let bb = b / (C64::ONE - p * C64::polar(1.0, omega * t)).scale(2.0);
    w * a + (w * bb).conj() + C64::new(-0.5 * w.re * g, 0.0)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut model = piano_host::load_preset(std::path::Path::new(&args[1])).unwrap();
    let (mut step, mut from, mut fmax, mut parts, mut accel, mut with_delta) = (3u8, 21u8, 8000.0f64, false, false, false);
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--step" => {
                step = args[i + 1].parse().unwrap();
                i += 2;
            }
            "--from" => {
                from = args[i + 1].parse().unwrap();
                i += 2;
            }
            "--fmax" => {
                fmax = args[i + 1].parse().unwrap();
                i += 2;
            }
            "--parts" => {
                parts = true;
                i += 1;
            }
            "--accel" => {
                accel = true;
                i += 1;
            }
            "--delta" => {
                with_delta = true;
                i += 1;
            }
            l => {
                model = piano_host::load_listening(std::path::Path::new(l), &model).unwrap();
                i += 1;
            }
        }
    }
    let model = model.with_macros();
    let model = PianoModel { soundboard: model.soundboard.map(|s| s.resolved()), ..model };
    let sb = model.board().expect("a preset with a soundboard");
    let sr = 48_000.0;
    let zones = zones_for(&sb, sr).map(|z| z.0);
    let n_mics = piano_core::board::heard_mics(&sb).len();
    eprintln!("mics {:?}; zones {:?}; {} modes", sb.params.mics, zones, sb.modes.len());
    let direct = RAD_ALL & !RAD_CASE;
    let t0 = std::time::Instant::now();
    let own = sb.radiation_parts(None, direct);
    let own_parts: Vec<Vec<Vec<C64>>> = if parts { [RAD_FRONT, RAD_BACK, RAD_IMAGE].iter().map(|&b| sb.radiation_parts(None, b)).collect() } else { Vec::new() };
    eprintln!("own weights {:.2} s", t0.elapsed().as_secs_f64());
    let forced = if with_delta { piano_core::forced::Forced::new(&sb, sr) } else { None };
    let mut note = from;
    let mut spent = 0.0;
    let mut count = 0;
    while note <= 108 {
        let (x, y) = sb.params.bridge_point(note);
        // The engine: every region input at the key's point.
        let mut heard = board_modes_heard_at(&sb, &[(x, y); REGIONS], sr);
        let n = heard.len();
        let shape: Vec<C64> = heard.iter().zip(&sb.modes).map(|(_, md)| C64::new(1.0, sb.sigma(md) / md.omega)).collect();
        // Only the direct field, unzoned, as acceleration weights.
        let with = |w: &[Vec<C64>]| -> Vec<BoardMode> {
            let mut m = heard.clone();
            for (j, bm) in m.iter_mut().enumerate() {
                for o in bm.outputs.iter_mut() {
                    *o = (0.0, 0.0);
                }
                for mic in 0..n_mics {
                    let v = w[j][mic] * shape[j];
                    bm.outputs[mic] = (v.re, v.im);
                }
            }
            to_acceleration_weights(&mut m, sr);
            m
        };
        let eng_direct = with(&own);
        let eng_parts: Vec<Vec<BoardMode>> = own_parts.iter().map(|w| with(w)).collect();
        to_acceleration_weights(&mut heard, sr);
        let nm = piano_core::note_model::NoteModel::build(&model, None, None, note, sr, piano_core::engine::MAX_MODES);
        for k in 1..=nm.n_partials as u32 {
            let fs: Vec<f64> = nm.phys.iter().filter(|p| p.k == k).map(|p| p.freq).collect();
            if fs.is_empty() {
                continue;
            }
            let f = fs.iter().sum::<f64>() / fs.len() as f64;
            if f > fmax {
                break;
            }
            let w = TAU * f;
            let t1 = std::time::Instant::now();
            let at = sb.radiation_parts_of(Some(w), direct, n);
            let at_parts: Vec<Vec<Vec<C64>>> = if parts { [RAD_FRONT, RAD_BACK, RAD_IMAGE].iter().map(|&b| sb.radiation_parts_of(Some(w), b, n)).collect() } else { Vec::new() };
            spent += t1.elapsed().as_secs_f64();
            count += 1;
            // The modes' velocity response to the key's force.
            let delta = forced.as_ref().map(|fd| fd.delta(note, &sb.bridge_shapes(note), f));
            let vel: Vec<C64> = heard.iter().zip(&shape).map(|(m, s)| mode_response(m, m.inputs[0], *s, w, sr)).collect();
            for mic in 0..n_mics {
                let total = mic_transfer_folded(&heard, zones.as_ref(), 0, mic, w, sr);
                let eng = mic_transfer(&eng_direct, 0, mic, w, sr);
                let ex = vel.iter().zip(&at).fold(C64::ZERO, |s, (v, r)| s + *v * r[mic]);
                let case = total - eng;
                print!("{note} {k} {f:.2} {mic} {:.6e} {:.6e} {:.6e} {:.6e} {:.6e} {:.6e}", eng.re, eng.im, ex.re, ex.im, case.re, case.im);
                for (ep, ap) in eng_parts.iter().zip(&at_parts) {
                    let e = mic_transfer(ep, 0, mic, w, sr);
                    let xp = vel.iter().zip(ap).fold(C64::ZERO, |s, (v, r)| s + *v * r[mic]);
                    print!(" {:.6e} {:.6e} {:.6e} {:.6e}", e.re, e.im, xp.re, xp.im);
                }
                if let Some(d) = &delta {
                    print!(" {:.6e} {:.6e}", d[mic].re, d[mic].im);
                }
                if accel {
                    print!(" {:.6e}", w * sb.admittance(x, y, w).norm());
                }
                println!();
            }
        }
        eprintln!("note {note}: {count} partials so far, {:.3} s per partial", spent / count.max(1) as f64);
        note += step;
    }
}
