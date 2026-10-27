//! The outlined board (soundboard.rs, docs/outline.md): its modes against
//! a shorter rectangle's, the grand outline's modes and bridges, the rim's
//! coupling, the bridge conductance by register, the lowest modes and the
//! build times. `cargo run --release -p piano-core --example outline_modes`.
use piano_core::soundboard::{Soundboard, SoundboardParams, OUTLINE_POINTS};
use std::f64::consts::TAU;
use std::time::Instant;

fn main() {
    let base = SoundboardParams::GRAND_2M;
    // A shorter rectangle as an outline: the analytic modes of a 0.75·a board.
    let r = 0.75;
    let mut outline = [(0.0, 1.0); OUTLINE_POINTS];
    outline[..4].copy_from_slice(&[(0.0, 0.0), (r, 0.0), (r, 1.0), (0.0, 1.0)]);
    let p = SoundboardParams { outline, ..base };
    let t = Instant::now();
    let sb = Soundboard::new(p);
    let dt = t.elapsed();
    let exact = Soundboard::new(SoundboardParams { a: r * base.a, ..base });
    let own: Vec<f64> = sb.modes.iter().filter(|m| m.ritz.is_some()).map(|m| m.omega / TAU).collect();
    let want: Vec<f64> = exact.modes.iter().filter(|m| m.m > 0).map(|m| m.omega / TAU).take(own.len()).collect();
    println!("shorter rectangle: basis {} modes, {} cells held, {} own modes, build {:.3} s", sb.basis.len(), sb.cells.len(), own.len(), dt.as_secs_f64());
    for (i, (a, b)) in own.iter().zip(&want).enumerate().filter(|(i, _)| i % 5 == 0 || *i < 6) {
        println!("  {i:3}: {a:7.1} Hz  vs {b:7.1}  ({:+.1} %)", 100.0 * (a / b - 1.0));
    }
    let err: f64 = (own.iter().zip(&want).map(|(a, b)| (a / b - 1.0).powi(2)).sum::<f64>() / own.len() as f64).sqrt();
    println!("  rms {:.2} %, count {} vs {}", 100.0 * err, own.len(), exact.modes.iter().filter(|m| m.m > 0 && m.omega < TAU * base.outline_below).count());
    // Shape outside: mass share outside the outline.
    let leak: f64 = sb
        .modes
        .iter()
        .filter(|m| m.ritz.is_some())
        .map(|m| sb.held_forces(m).unwrap().iter().map(|&(_, _, f)| f * f).sum::<f64>().sqrt())
        .fold(0.0, f64::max);
    println!("  largest held force norm {leak:.3e}");

    let p = SoundboardParams { outline: SoundboardParams::GRAND_OUTLINE, ..base };
    let t = Instant::now();
    let sb = Soundboard::new(p);
    let dt = t.elapsed();
    let o = sb.outline.as_ref().unwrap();
    println!(
        "grand outline: area {:.2} m² of {:.2} ({:.0} %), basis {}, {} own modes below {} Hz (rectangle {}), {} modes in all (rectangle {}), build {:.3} s",
        o.area(),
        base.a * base.b,
        100.0 * o.area() / (base.a * base.b),
        sb.basis.len(),
        sb.ritz.len(),
        base.outline_below,
        Soundboard::new(base).modes.iter().filter(|m| m.m > 0 && m.omega < TAU * base.outline_below).count(),
        sb.modes.len(),
        Soundboard::new(base).modes.len(),
        dt.as_secs_f64()
    );
    let lows: Vec<String> = sb.modes.iter().take(12).map(|m| format!("{:.0}", m.omega / TAU)).collect();
    let lows_r: Vec<String> = Soundboard::new(base).modes.iter().take(12).map(|m| format!("{:.0}", m.omega / TAU)).collect();
    println!("  lowest: {}\n  rect:   {}", lows.join(" "), lows_r.join(" "));
    let bridge = |note: u8| {
        let (x, y) = p.bridge_point(note);
        o.edge_distance(x, y)
    };
    let worst = (21..=108).map(|n| (bridge(n), n)).fold((f64::INFINITY, 0), |a, b| if b.0 < a.0 { b } else { a });
    println!("  bridges: nearest the edge {:.3} m at key {}", worst.0, worst.1);
    for n in [21u8, 28, 29, 40, 60, 108] {
        let (x, y) = p.bridge_point(n);
        println!("    key {n}: edge {:.3} m (rectangle {:.3}), inside {}", bridge(n), x.min(p.a - x).min(y).min(p.b - y), o.contains(x, y));
    }
    // Rim: the edge forces of the own modes balance their inertia,
    // Σ F = ω²·∫ρh·Φ, as the rectangle's edge reactions do
    // (structure.rs, edge_reactions_balance_the_plate).
    let rim = piano_core::structure::RimModel::new(&piano_core::structure::RimParams::GRAND_2M, &sb, sb.modes.len(), 48_000.0);
    println!("  rim couples {} board modes", rim.couple.len());
    // Bridge conductance (smoothed over ±1/6 octave, with the damped
    // strings' load) at partials 1–4 by register: outline against rectangle.
    {
        use piano_core::params::{partial_freq, PianoModel};
        let model = PianoModel::default();
        let boards = [base, p].map(|sbp| PianoModel { soundboard: Some(sbp), ..model }.board().unwrap());
        println!("conductance, outline / rectangle (dB), partials 1–4:");
        for (name, lo, hi) in [("bass A0–B2", 21, 47), ("middle C3–B4", 48, 71), ("treble C5–B6", 72, 95), ("top C7–C8", 96, 108)] {
            let mut d = [0.0f64; 4];
            let mut n = 0;
            for note in (lo..=hi).step_by(2) {
                let np = model.note(note);
                let g: Vec<Vec<f64>> = boards
                    .iter()
                    .map(|sb| {
                        let sh = sb.bridge_shapes(note);
                        (1..=4)
                            .map(|k| {
                                let f = partial_freq(np.f0, np.b, k);
                                (0..41).map(|i| sb.mobility(&sh, TAU * f * 2f64.powf((i as f64 - 20.0) / 120.0)).re).sum::<f64>()
                            })
                            .collect()
                    })
                    .collect();
                for k in 0..4 {
                    d[k] += 10.0 * (g[1][k] / g[0][k]).log10();
                }
                n += 1;
            }
            println!("  {name:13}: {}", d.map(|x| format!("{:+5.1}", x / n as f64)).join(" "));
        }
    }

    // The lowest modes: drive at A2 / B2's bridge, radiation (close mics).
    {
        use piano_core::params::PianoModel;
        let close = |sbp: SoundboardParams| SoundboardParams { mics_relative: true, mics: [(0.294, 0.346, 0.4), (0.706, 0.654, 0.4)], lid_angle: 35.0, ..sbp }.resolved();
        for (name, sbp) in [("rectangle", base), ("outline", p)] {
            let sb = PianoModel { soundboard: Some(close(sbp)), ..PianoModel::default() }.board().unwrap();
            let rad = sb.radiation();
            let (a2, b2) = (sb.bridge_shapes(45), sb.bridge_shapes(47));
            println!("{name}: lowest modes (Hz, η loaded, Φ at A2 / B2, |p| mic 1 per m/s)");
            for (i, md) in sb.modes.iter().enumerate().filter(|(_, m)| m.m > 0).take(4) {
                println!("  {:6.1}  {:.3}  {:+.3} {:+.3}  {:.3}", md.omega / TAU, sb.mode_eta(md), a2[i], b2[i], rad[i][0].norm());
            }
        }
    }

    // Instrument build time, default model and with the outline.
    use piano_core::params::PianoModel;
    for (name, sbp) in [("rectangle", base), ("outline", p)] {
        let model = PianoModel { soundboard: Some(sbp), ..PianoModel::default() };
        let mut best = f64::INFINITY;
        for _ in 0..3 {
            let t = Instant::now();
            let _ = piano_core::instrument::Instrument::new(model, 48_000.0);
            best = best.min(t.elapsed().as_secs_f64());
        }
        println!("instrument build, {name}: {best:.3} s");
    }
}
