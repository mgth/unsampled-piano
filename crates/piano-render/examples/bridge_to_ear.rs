//! Ear pressure per bridge acceleration, sampled at the harmonics of every
//! key up to 8 kHz: the board and its radiation alone, as Plath's anechoic
//! measurements give them for a real concert grand (bridge accelerometer
//! and dummy head at the player, `tools/plath_tf.py`). One line per
//! sample: note, partial, frequency (Hz), 10·log10(mean over the two mics
//! of |p|² / |a|²) in dB re (Pa per m/s²)².
//!   cargo run --release -p piano-render --example bridge_to_ear -- <preset.toml> [listening.toml …]
use piano_core::board::{board_modes_at, mic_transfer, to_acceleration_weights};
use piano_core::instrument::REGIONS;
use piano_core::params::PianoModel;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut model = piano_host::load_preset(std::path::Path::new(&args[1])).unwrap();
    for l in &args[2..] {
        model = piano_host::load_over(std::path::Path::new(l), &model).unwrap();
    }
    let model = model.with_macros();
    let model = PianoModel { soundboard: model.soundboard.map(|s| s.resolved()), ..model };
    let sb = model.board().expect("a preset with a soundboard");
    let sr = 48_000.0;
    for note in 21u8..=108 {
        let (x, y) = sb.params.bridge_point(note);
        // One input per region, all at the key's point: the bank wants REGIONS of them.
        let mut modes = board_modes_at(&sb, &[(x, y); REGIONS], sr);
        to_acceleration_weights(&mut modes, sr);
        let f0 = 440.0 * 2f64.powf((note as f64 - 69.0) / 12.0);
        for k in 1..=60 {
            let f = k as f64 * f0;
            if f > 8000.0 {
                break;
            }
            let w = std::f64::consts::TAU * f;
            let p = 0.5 * (mic_transfer(&modes, 0, 0, w, sr).norm_sqr() + mic_transfer(&modes, 0, 1, w, sr).norm_sqr());
            let a = (w * sb.admittance(x, y, w).norm()).powi(2);
            println!("{note} {k} {f:.1} {:.2}", 10.0 * (p / a).log10());
        }
    }
}
