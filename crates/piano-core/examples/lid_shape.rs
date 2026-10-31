//! The open lid's radiation at the listening presets' microphones (and two
//! off them, where the bentside's missing corner could matter): mean
//! |pressure per unit modal velocity|² of the board's modes per third
//! octave, dB, and the time the radiation weights take (docs/lid.md).
//! Run it before and after a change to the lid's geometry and compare.
use piano_core::soundboard::{Soundboard, SoundboardParams};
use std::time::Instant;

fn main() {
    let sets: [(&str, [(f64, f64, f64); 2]); 5] = [
        ("concert, 2 m", [(0.323529, -1.230769, 2.0), (0.735294, -1.230769, 2.0)]),
        ("close, 40 cm", [(0.294118, 0.346154, 0.4), (0.705882, 0.653846, 0.4)]),
        ("player", [(-0.558824, 0.569231, 0.45), (-0.558824, 0.430769, 0.45)]),
        ("treble side, 1 m", [(0.6, -0.8, 0.6), (0.9, -0.8, 0.6)]),
        ("over the tail, 1.2 m", [(1.2, -0.2, 1.0), (1.4, 0.1, 1.2)]),
    ];
    let bands: Vec<f64> = (0..19).map(|i| 125.0 * 2f64.powf(i as f64 / 3.0)).collect();
    println!("{:22} {}", "dB", bands.iter().map(|f| format!("{:>6.0}", f)).collect::<String>());
    for (name, mics) in sets {
        let sb = Soundboard::new(SoundboardParams { lid_angle: 35.0, lid_removed: false, mics_relative: true, mics, ..SoundboardParams::GRAND_2M }.resolved());
        let t = Instant::now();
        let w = sb.radiation();
        let ms = t.elapsed().as_secs_f64() * 1e3;
        let row: String = bands
            .iter()
            .map(|&fc| {
                let (lo, hi) = (fc / 2f64.powf(1.0 / 6.0), fc * 2f64.powf(1.0 / 6.0));
                let e: f64 = sb.modes.iter().zip(&w).filter(|(m, _)| (lo..hi).contains(&(m.omega / std::f64::consts::TAU))).map(|(_, v)| v[0].norm_sqr() + v[1].norm_sqr()).sum();
                format!("{:>6.1}", 10.0 * e.log10())
            })
            .collect();
        println!("{name:22} {row}  ({ms:.0} ms)");
    }
}
