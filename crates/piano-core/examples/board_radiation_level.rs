//! Pressure at the first microphone per newton at a key's bridge, mean
//! power per third octave: Σ_n w_n(ω)·v_n(ω), v_n the modal velocity.
//! usage: board_radiation_level <note>; SB="density=2": modal density × 2 at the same mean mobility.
use piano_core::complex::C64;
use piano_core::params::PianoModel;
use std::f64::consts::TAU;

fn main() {
    let note: u8 = std::env::args().nth(1).unwrap().parse().unwrap();
    let mut model = PianoModel::default();
    let mut p = model.soundboard.unwrap();
    for kv in std::env::var("SB").unwrap_or_default().split(',').filter(|s| !s.is_empty()) {
        let (k, v) = kv.split_once('=').unwrap();
        let v: f64 = v.parse().unwrap();
        match k {
            "density" => {
                p.rho_h *= v;
                p.dx /= v;
                p.dy /= v;
            }
            _ => panic!("{k}"),
        }
    }
    model.soundboard = Some(p);
    let sb = model.board().unwrap();
    let shapes = sb.bridge_shapes(note);
    for f in [125.0, 160.0, 200.0, 250.0, 315.0, 400.0, 500.0, 630.0, 800.0, 1000.0, 1250.0, 1600.0, 2000.0] {
        let n = 24;
        let (mut pw, mut g) = (0.0, 0.0);
        for i in 0..n {
            let fr: f64 = f * 2f64.powf((i as f64 / n as f64 - 0.5) / 3.0);
            let w = TAU * fr;
            let rad = sb.radiation_at(w);
            let mut pr = C64::ZERO;
            for ((md, phi), r) in sb.modes.iter().zip(&shapes).zip(&rad) {
                let v = C64::new(0.0, w * phi) / C64::new(md.omega * md.omega - w * w, sb.mode_eta(md) * md.omega * w);
                pr += r[0] * v;
            }
            pw += pr.norm_sqr();
            g += sb.mobility(&shapes, w).re;
        }
        println!("{f:6.0} Hz  p/F {:6.1} dB  Re Y {:.2e}", 10.0 * (pw / n as f64).log10(), g / n as f64);
    }
}
