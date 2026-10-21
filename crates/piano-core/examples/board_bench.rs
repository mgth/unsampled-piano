//! Cost of the MIMO soundboard bank per mode and sample.
use piano_core::board::{board_modes, BoardBank, REGIONS};
use piano_core::soundboard::{Soundboard, SoundboardParams};
use std::time::Instant;

fn main() {
    let sr = 48_000.0;
    let sb = Soundboard::new(SoundboardParams::GRAND_2M);
    let modes = board_modes(&sb, sr);
    let len = 64;
    for isa in piano_core::modal::Isa::available() {
    for (label, active, disp) in [("16 inputs, 18 outputs", 16, true), ("16 inputs, 2 outputs", 16, false), ("0 inputs, 18 outputs", 0, true)] {
        let label = format!("{isa:?} {label}");
        let mut bank = BoardBank::new(sr, &modes, len);
        bank.set_isa(isa);
        let mut inputs = vec![vec![0.0f32; len]; REGIONS];
        for (r, x) in inputs.iter_mut().enumerate().take(active) {
            for (t, v) in x.iter_mut().enumerate() {
                *v = ((t * 31 + r * 7) % 13) as f32 - 6.0;
            }
        }
        let mut d = vec![vec![0.0f32; len]; REGIONS];
        let (mut a, mut b) = (vec![0.0f32; len], vec![0.0f32; len]);
        let _ftz = piano_core::denormal::DenormalGuard::new();
        let n = 2000;
        let t = Instant::now();
        for _ in 0..n {
            bank.process(&inputs, len, [&mut a, &mut b], if disp { Some(&mut d) } else { None });
        }
        let el = t.elapsed().as_secs_f64();
        let per = el * 1e9 / (n * len * modes.len()) as f64;
        println!("{label:32} {per:.3} ns/mode·sample → {:.1} % of a core at 48 kHz", 100.0 * per * 1e-9 * modes.len() as f64 * sr);
    }
    }
}
