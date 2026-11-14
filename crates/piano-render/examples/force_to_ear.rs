//! From a key's string force to the microphones, on a frequency grid: the
//! board and its radiation as one stage, which is what the ear gets whatever
//! the bridge's own movement (docs/bass_gaps.md). One line per frequency:
//! Hz, 10·log10 of the mean over the two microphones of |p/F|² (dB re
//! (Pa/N)²), 20·log10 |Y| and 10·log10 Re Y at the key's bridge point
//! (dB re 1 s/kg).
//!   cargo run --release -p piano-render --example force_to_ear -- <preset.toml> <note> <lo Hz> <hi Hz> <step Hz> [listening.toml …]
use piano_core::board::{board_modes_at, mic_transfer, to_acceleration_weights};
use piano_core::instrument::REGIONS;
use piano_core::params::PianoModel;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut model = piano_host::load_preset(std::path::Path::new(&args[1])).unwrap();
    let note: u8 = args[2].parse().unwrap();
    let (lo, hi, step): (f64, f64, f64) = (args[3].parse().unwrap(), args[4].parse().unwrap(), args[5].parse().unwrap());
    for l in &args[6..] {
        model = piano_host::load_listening(std::path::Path::new(l), &model).unwrap();
    }
    let model = model.with_macros();
    let model = PianoModel { soundboard: model.soundboard.map(|s| s.resolved()), ..model };
    let sb = model.board().expect("a preset with a soundboard");
    let sr = 48_000.0;
    let (x, y) = sb.params.bridge_point(note);
    let mut modes = board_modes_at(&sb, &[(x, y); REGIONS], sr);
    to_acceleration_weights(&mut modes, sr);
    let mut f = lo;
    while f <= hi {
        let w = std::f64::consts::TAU * f;
        let p = 0.5 * (mic_transfer(&modes, 0, 0, w, sr).norm_sqr() + mic_transfer(&modes, 0, 1, w, sr).norm_sqr());
        let adm = sb.admittance(x, y, w);
        println!("{f:.2} {:.2} {:.2} {:.2}", 10.0 * p.max(1e-30).log10(), 20.0 * adm.norm().max(1e-15).log10(), 10.0 * adm.re.max(1e-15).log10());
        f += step;
    }
}
