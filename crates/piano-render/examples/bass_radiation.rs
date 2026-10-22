//! Mean |pressure / bridge force| (both mics, power sum) per third octave
//! from the bridge region of a bass key, for a preset and variants of the
//! board's low-frequency radiation. Shows the shape the board gives to the
//! bass partials.  cargo run --example bass_radiation -- <preset.toml> [note]
use piano_core::board::{board_modes, mic_transfer, region_of, to_acceleration_weights};
use piano_core::soundboard::{Soundboard, SoundboardParams};

fn bands(p: SoundboardParams, note: u8) -> Vec<f64> {
    let sr = 48_000.0;
    let mut modes = board_modes(&Soundboard::new(p), sr);
    to_acceleration_weights(&mut modes, sr);
    let r = region_of(note);
    let mut out = Vec::new();
    let mut f0 = 25.0;
    while f0 < 2000.0 {
        let n = 200;
        let mut s = 0.0;
        for i in 0..n {
            let w = std::f64::consts::TAU * f0 * 2f64.powf((i as f64 / n as f64 - 0.5) / 3.0);
            s += mic_transfer(&modes, r, 0, w, sr).norm_sqr() + mic_transfer(&modes, r, 1, w, sr).norm_sqr();
        }
        out.push(10.0 * (s / n as f64).log10());
        f0 *= 2f64.powf(1.0 / 3.0);
    }
    out
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let model = piano_host::load_preset(std::path::Path::new(&args[1])).unwrap();
    let note: u8 = args.get(2).map(|s| s.parse().unwrap()).unwrap_or(33);
    let p = model.soundboard.unwrap();
    let mut f = 25.0;
    print!("{:24}", "Hz");
    while f < 2000.0 {
        print!("{:6.0}", f);
        f *= 2f64.powf(1.0 / 3.0);
    }
    println!();
    let base = bands(p, note);
    let show = |label: &str, q: SoundboardParams| {
        let b = bands(q, note);
        print!("{label:24}");
        for (x, y) in b.iter().zip(&base) {
            print!("{:6.1}", if label == "preset" { *x } else { x - y });
        }
        println!();
    };
    show("preset", p);
    show("no piston", SoundboardParams { piston_freq: 0.0, ..p });
    show("baffled (back 0)", SoundboardParams { back_gain: 0.0, ..p });
    show("back 0.9", SoundboardParams { back_gain: 0.9, ..p });
    show("back 1.0", SoundboardParams { back_gain: 1.0, ..p });
    show("back depth 0.3", SoundboardParams { back_depth: 0.3, ..p });
    show("piston radiation 0", SoundboardParams { piston_radiation: 0.0, ..p });
}
