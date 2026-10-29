//! Does the board's sum of modes radiate its direct field? For a point
//! force at a key's bridge point and a set of driving frequencies, the
//! pressure at the first microphone per unit force, computed two ways:
//! as the engine does (each mode's Rayleigh integral at its own frequency,
//! scaled to the driving one), and exactly (every mode's integral at the
//! driving frequency, so that the modes' sum interferes as the plate's
//! actual velocity field does). The difference is what the engine loses
//! of the direct field, a compact source around the drive point.
//!   cargo run --release -p piano-render --example direct_field -- <preset.toml> [listening.toml …] [--note N]
use piano_core::complex::C64;
use piano_core::params::PianoModel;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut model = piano_host::load_preset(std::path::Path::new(&args[1])).unwrap();
    let mut note = 45u8;
    let mut i = 2;
    while i < args.len() {
        if args[i] == "--note" {
            note = args[i + 1].parse().unwrap();
            i += 2;
        } else {
            model = piano_host::load_listening(std::path::Path::new(&args[i]), &model).unwrap();
            i += 1;
        }
    }
    let model = model.with_macros();
    let model = PianoModel { soundboard: model.soundboard.map(|s| s.resolved()), ..model };
    let sb = model.board().unwrap();
    let (x, y) = sb.params.bridge_point(note);
    let phi: Vec<f64> = sb.modes.iter().map(|md| sb.shape(md, x, y)).collect();
    let own = sb.radiation();
    println!("mics {:?}; force at note {note} ({x:.2}, {y:.2}) m; {} modes", sb.params.mics, sb.modes.len());
    println!("  f (Hz)   engine dB   exact dB   exact − engine");
    let mut f = 250.0f64;
    while f < 9000.0 {
        let w = std::f64::consts::TAU * f;
        let at = sb.radiation_at(w);
        let (mut p_eng, mut p_ex) = (C64::ZERO, C64::ZERO);
        for (n, md) in sb.modes.iter().enumerate() {
            let den = C64::new(md.omega * md.omega - w * w, sb.mode_eta(md) * md.omega * w);
            let vel = C64::new(0.0, w * phi[n]) / den;
            p_eng += vel * own[n][0].scale(w / md.omega);
            p_ex += vel * at[n][0];
        }
        let (e, x) = (20.0 * p_eng.norm().log10(), 20.0 * p_ex.norm().log10());
        println!("{f:8.0} {e:11.1} {x:10.1} {:+10.1}", x - e);
        f *= 2f64.powf(1.0 / 3.0);
    }
}
