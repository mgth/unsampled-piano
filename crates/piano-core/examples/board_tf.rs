//! From a key's string force to the two microphones, at each of its partials,
//! built board against fitted plate (docs/real_board.md). A preset's own
//! board, from a JSON dump: `unsampled-piano-render` is not needed.
//!   board_tf <note> [crown]
use piano_core::board;
use piano_core::fem::BoardStructure;
use piano_core::soundboard::{Soundboard, SoundboardParams};

fn main() {
    let note: u8 = std::env::args().nth(1).and_then(|v| v.parse().ok()).unwrap_or(28);
    let f0 = 440.0 * 2f64.powf((note as f64 - 69.0) / 12.0);
    let base = SoundboardParams {
        a: 2.124,
        b: 1.46,
        outline: [(0.0, 0.0096), (0.1149, 0.0068), (0.2373, 0.1781), (0.3362, 0.3014), (0.5621, 0.363), (0.8682, 0.411), (0.9976, 0.5753), (0.9976, 0.9315), (0.9341, 1.0), (0.0584, 1.0)],
        main_bridge_from: 41,
        treble_bridge: [(0.0443, 0.048), (0.0809, 0.202), (0.1596, 0.38), (0.3441, 0.579), (0.8023, 0.842)],
        bass_bridge: [(0.5765, 0.562), (0.8707, 0.753)],
        main_string_slant: 0.0,
        crown_freq: 55.0,
        rho_h: 14.7,
        dx: 5737.5,
        dy: 2550.0,
        rib_spacing: 0.11,
        rib_share: 0.5,
        rib_jitter: 0.3,
        impedance_slope_bass: -0.02,
        mics: [(0.294118, 0.346154, 0.4), (0.705882, 0.653846, 0.4)],
        mics_relative: true,
        mic_pattern: 0.5,
        lid_angle: 35.0,
        lid_removed: false,
        ..SoundboardParams::GRAND_2M
    }
    .resolved();
    let sr = 48_000.0;
    for (name, p) in [("fitted", base), ("built", SoundboardParams { structure: BoardStructure { enabled: true, ..base.structure }, ..base })] {
        let sb = Soundboard::new(p);
        let modes = board::board_modes(&sb, sr);
        let sh = sb.bridge_shapes(note);
        let t = 1.0 / sr;
        let tf = |w: f64, o: usize| { let h = board::transfer_with(&modes, |i, _| sh[i] * t, o, w, sr); h.re.hypot(h.im) };
        print!("{name:7} p:");
        for k in 1..=16 {
            let f = k as f64 * f0;
            let w = std::f64::consts::TAU * f;
            let (a, b) = (tf(w, 0), tf(w, 1));
            let y = sb.mobility(&sh, w).re;
            print!(" {k}:{:+.0}/{:+.0}/{:.0}", 20.0 * (a.max(1e-12)).log10() + 20.0 * w.log10(), 20.0 * (b.max(1e-12)).log10() + 20.0 * w.log10(), 10.0 * y.max(1e-12).log10());
        }
        println!();
        // Mean level per third of an octave, across the corner at 3 kHz.
        print!("{name:7} bands:");
        for fc in [500.0, 1000.0, 1600.0, 2000.0, 2500.0, 3150.0, 4000.0, 5000.0, 6300.0, 8000.0] {
            let n = 48;
            let e: f64 = (0..n)
                .map(|i| {
                    let w = std::f64::consts::TAU * fc * 2f64.powf((i as f64 / (n - 1) as f64 - 0.5) / 3.0);
                    (w * tf(w, 0)).powi(2) + (w * tf(w, 1)).powi(2)
                })
                .sum::<f64>()
                / n as f64;
            print!(" {fc:.0}:{:.1}", 10.0 * e.log10());
        }
        println!();
    }
}
