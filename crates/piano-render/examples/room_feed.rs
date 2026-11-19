//! What drives the room (analysis): for a key's bridge point, the source the
//! room takes from the two microphones (their pressure scaled to 1 m) against
//! the power radiated towards the engine's probes around the piano
//! (docs/room_feed.md), per frequency.
//!   cargo run --release -p piano-render --example room_feed -- <preset.toml> <note> <f0> <f1> <steps> [listening.toml …]
use piano_core::board::{board_modes_at, transfer_with};
use piano_core::complex::C64;
use piano_core::params::PianoModel;

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let mut model = piano_host::load_preset(std::path::Path::new(&a[0])).unwrap();
    for l in &a[5..] {
        model = piano_host::load_listening(std::path::Path::new(l), &model).unwrap();
    }
    let model = model.with_macros();
    let note: u8 = a[1].parse().unwrap();
    let (f0, f1, steps): (f64, f64, usize) = (a[2].parse().unwrap(), a[3].parse().unwrap(), a[4].parse().unwrap());
    let sr = 48_000.0;
    let model = PianoModel { soundboard: model.soundboard.map(|s| s.resolved()), ..model };
    let sb = model.board().expect("a soundboard");
    let p = sb.params.bridge_point(note);
    let modes = board_modes_at(&sb, &[p; piano_core::board::REGIONS], sr);
    // The engine's probes, every mode (the engine stops at PROBES_BELOW).
    let dist = piano_core::board::PROBE_DISTANCE;
    let pts = piano_core::board::probe_points(&sb.params);
    let rad = sb.radiation_points(&pts, usize::MAX);
    let nyq = 0.45 * sr;
    let kept: Vec<usize> = (0..sb.modes.len()).filter(|&i| sb.modes[i].omega / std::f64::consts::TAU < nyq).collect();
    assert_eq!(kept.len(), modes.len());
    // Mic distances to the room's source point.
    let r = piano_core::room::mic_distances(&sb.params);
    // Probe k as output 0 of a copy of the modes.
    let probe_modes: Vec<Vec<_>> = (0..pts.len())
        .map(|k| {
            modes
                .iter()
                .zip(&kept)
                .map(|(m, &i)| {
                    let mut m = m.clone();
                    let shape = C64::new(1.0, m.sigma / (std::f64::consts::TAU * m.freq_hz));
                    let w = rad[i][k] * shape;
                    m.outputs[0] = (w.re, w.im);
                    m
                })
                .collect()
        })
        .collect();
    println!("Hz      mics→1m dB  probes rms dB  per probe dB (lid, spine, tail, below)");
    for s in 0..steps {
        let f = f0 * (f1 / f0).powf(s as f64 / (steps - 1).max(1) as f64);
        let w = std::f64::consts::TAU * f;
        let h = |ms: &[piano_core::board::BoardMode], o: usize| transfer_with(ms, |_, m| m.inputs[0], o, w, sr);
        // Velocity weights: the pressure is their sum (no differentiator here).
        let mic = (h(&modes, 0).scale(r[0]) + h(&modes, 1).scale(r[1])).scale(0.5);
        let per: Vec<f64> = probe_modes.iter().map(|ms| h(ms, 0).norm() * dist).collect();
        let rms = (per.iter().map(|x| x * x).sum::<f64>() / per.len() as f64).sqrt();
        let db = |x: f64| 20.0 * x.max(1e-30).log10();
        print!("{f:7.1} {:10.1} {:12.1}  ", db(mic.norm()), db(rms));
        for x in &per {
            print!(" {:6.1}", db(*x));
        }
        println!();
    }
}
