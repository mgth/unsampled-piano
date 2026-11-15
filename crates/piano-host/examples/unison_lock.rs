//! Do the strings of a unison lock or beat? Per key and partial, the
//! mistuning of its extreme strings against the bridge's coupling
//! ε = (T/L)·Y (Weinreich 1977): two strings ε apart or less in half their
//! mistuning move as one pair of modes (no beat, two decay rates); further
//! apart, each keeps its frequency and the partial beats at their distance.
//!   cargo run --release -p piano-host --example unison_lock -- preset.toml [first last [step]]
//! Columns: the partial, its frequency, the mistuning Δf (Hz), |ε|/π, Re ε/π
//! and Im ε/π (Hz: the mistuning a coupling of that size holds), the string's
//! own loss and one string's bridge loss (dB/s), then the vertical coupled
//! modes the engine runs: distance of their frequencies (Hz) and their decays
//! (dB/s).
use piano_core::params::partial_freq;
use std::f64::consts::{PI, TAU};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let model = piano_host::load_preset(std::path::Path::new(&args[1])).unwrap();
    let lo: u8 = args.get(2).and_then(|v| v.parse().ok()).unwrap_or(29);
    let hi: u8 = args.get(3).and_then(|v| v.parse().ok()).unwrap_or(47);
    let step: usize = args.get(4).and_then(|v| v.parse().ok()).unwrap_or(1);
    let csv = std::env::var("CSV").is_ok();
    let sr = 48_000.0;
    let sb = model.board().unwrap();
    let modes = piano_core::board::board_modes(&sb, sr);
    let db = |s: f64| 8.686 * s;
    for note in (lo..=hi).step_by(step) {
        let np = model.note(note);
        if np.strings < 2 {
            continue;
        }
        let nm = piano_core::note_model::NoteModel::build(&model, Some(&sb), Some(&modes), note, sr, 1200);
        let shapes = sb.bridge_shapes(note);
        let d = &np.detune[..np.strings];
        let width = d.iter().cloned().fold(f64::MIN, f64::max) - d.iter().cloned().fold(f64::MAX, f64::min);
        if !csv {
            println!("key {note}: {} strings, {:.2} cent(s) between the extreme ones, T/L {:.0} N/m", np.strings, width, np.tension / np.length);
            println!("  k    f Hz   Δf Hz  |ε|/π   Re ε/π   Im ε/π   own dB/s  bridge dB/s | modes: Δf Hz, decays dB/s");
        }
        for k in 1..=64u32 {
            let f = partial_freq(np.f0, np.b, k);
            if f > 4000.0 {
                break;
            }
            let w = TAU * f;
            let y = sb.bridge_admittance(&shapes, w) + piano_core::complex::C64::new(sb.edge_conductance(note, w), 0.0);
            let eps = y.scale(np.tension / np.length);
            let df = f * (2f64.powf(width / 1200.0) - 1.0);
            // The vertical modes: those the hammer drives and the bridge hears most.
            let mut ms: Vec<_> = nm.modes.iter().zip(&nm.phys).filter(|(_, p)| p.k == k).map(|(m, _)| m).collect();
            ms.sort_by(|a, b| b.out_re.hypot(b.out_im).total_cmp(&a.out_re.hypot(a.out_im)));
            ms.truncate(np.strings);
            let fmin = ms.iter().map(|m| m.freq_hz).fold(f64::MAX, f64::min);
            let fmax = ms.iter().map(|m| m.freq_hz).fold(f64::MIN, f64::max);
            let dec: Vec<String> = ms.iter().map(|m| format!("{:.1}", db(m.sigma))).collect();
            if csv {
                println!("{note},{k},{f:.2},{df:.4},{:.4},{:.4},{:.4},{:.3},{:.3},{:.4}", eps.norm() / PI, eps.re / PI, eps.im / PI, db(model.losses.sigma(note, f)), db(eps.re), fmax - fmin);
            } else {
                println!(
                    " {k:2} {f:7.1} {df:7.3} {:6.3} {:8.3} {:8.3} {:9.2} {:10.2}  | {:.3}, {}",
                    eps.norm() / PI,
                    eps.re / PI,
                    eps.im / PI,
                    db(model.losses.sigma(note, f)),
                    db(eps.re),
                    fmax - fmin,
                    dec.join(" ")
                );
            }
        }
    }
}
