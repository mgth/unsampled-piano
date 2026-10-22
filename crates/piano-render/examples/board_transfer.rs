//! Mean |pressure / bridge force| (both mics, power sum) per octave band
//! from the bridge point of several keys, as rendered: how the board
//! colours each register's partials.
//!   cargo run --example board_transfer -- <preset.toml> [note …]
use piano_core::board::{board_modes, mic_transfer, region_of, to_acceleration_weights};
use piano_core::soundboard::Soundboard;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let model = piano_host::load_preset(std::path::Path::new(&args[1])).unwrap();
    let notes: Vec<u8> = if args.len() > 2 { args[2..].iter().map(|s| s.parse().unwrap()).collect() } else { vec![33, 48, 60, 72, 84] };
    let sr = 48_000.0;
    let mut modes = board_modes(&Soundboard::new(model.soundboard.unwrap()), sr);
    to_acceleration_weights(&mut modes, sr);
    let cen = [125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0];
    print!("note");
    for c in cen {
        print!("{c:>8.0}");
    }
    println!();
    for note in notes {
        let r = region_of(note);
        print!("{note:4}");
        for c in cen {
            let n = 400;
            let mut s = 0.0;
            for i in 0..n {
                let w = std::f64::consts::TAU * c * 2f64.powf(i as f64 / n as f64 - 0.5);
                s += mic_transfer(&modes, r, 0, w, sr).norm_sqr() + mic_transfer(&modes, r, 1, w, sr).norm_sqr();
            }
            print!("{:8.1}", 10.0 * (s / n as f64).log10());
        }
        println!();
    }
}
