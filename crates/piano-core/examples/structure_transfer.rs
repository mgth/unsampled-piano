//! Mean |pressure / force| at mic 0 for the board (bridge point) and the
//! frame (agraffe point) of a region, per octave band.
use piano_core::board::{board_modes, region_of, transfer};
use piano_core::soundboard::Soundboard;
use piano_core::structure::frame_modes;

fn main() {
    let sr = 48_000.0;
    let p = piano_core::soundboard::SoundboardParams::GRAND_2M;
    let board = board_modes(&Soundboard::new(p), sr);
    let frame = frame_modes(&piano_core::structure::FrameParams::GRAND_2M, &p, sr);
    for note in [45u8, 60, 84, 96] {
        let r = region_of(note);
        print!("note {note}:");
        let mut f0 = 62.5;
        while f0 < 8000.0 {
            let (mut b, mut f) = (0.0, 0.0);
            let n = 400;
            for i in 0..n {
                let fr = f0 * 2f64.powf(i as f64 / n as f64);
                let w = std::f64::consts::TAU * fr;
                b += transfer(&board, r, 0, w, sr).norm_sqr();
                f += transfer(&frame, r, 0, w, sr).norm_sqr();
            }
            print!("  {:5.0}: {:+5.1}", f0, 10.0 * (f / b).log10());
            f0 *= 2.0;
        }
        println!();
    }
}
