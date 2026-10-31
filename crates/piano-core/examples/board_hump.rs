//! The board's conductance at each key's fundamental against the
//! reference fit of tools/bridge_conductance.py, and which modes make it.
//! usage: board_hump              every key from C3 to C6, with a summary
//!        board_hump <note> …     the modes that make Re Y(f0) at these keys
//! SB="bridge_beam=1,loss_factor=0.03" overrides board parameters.
use piano_core::complex::C64;
use piano_core::params::{partial_freq, PianoModel};
use piano_core::soundboard::SoundboardParams;
use std::f64::consts::TAU;

fn set(p: &mut SoundboardParams, name: &str, v: f64) {
    match name {
        "rho_h" => p.rho_h = v,
        "dx" => p.dx = v,
        "dy" => p.dy = v,
        "loss_factor" => p.loss_factor = v,
        "loss_low_corner" => p.loss_low_corner = v,
        "bridge_beam" => p.bridge_beam = v,
        "bridge_stiffness" => p.bridge_stiffness = v,
        "bridge_line_mass" => p.bridge_line_mass = v,
        "bridge_spread" => p.bridge_spread = v,
        "crown_freq" => p.crown_freq = v,
        "outline_below" => p.outline_below = v,
        "rib_spacing" => p.rib_spacing = v,
        "rib_share" => p.rib_share = v,
        "rib_jitter" => p.rib_jitter = v,
        "string_load" => p.string_load = v,
        "mobility" => *p = p.with_mobility_scale(v),
        // Modal density × v at the same infinite-plate mobility.
        "tb1" => p.treble_bridge[1].0 = v,
        "tb2" => p.treble_bridge[2].0 = v,
        "tb3" => p.treble_bridge[3].0 = v,
        "tb4" => p.treble_bridge[4].0 = v,
        "density" => {
            p.rho_h *= v;
            p.dx /= v;
            p.dy /= v;
        }
        _ => panic!("unknown parameter {name}"),
    }
}

fn main() {
    let mut model = PianoModel::default();
    let mut params = model.soundboard.unwrap();
    for kv in std::env::var("SB").unwrap_or_default().split(',').filter(|s| !s.is_empty()) {
        let (k, v) = kv.split_once('=').expect("name=value");
        set(&mut params, k, v.parse().expect("a number"));
    }
    model.soundboard = Some(params);
    let sb = model.board().unwrap();
    let names = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
    let name = |n: u8| format!("{}{}", names[n as usize % 12], n as i32 / 12 - 1);
    let notes: Vec<u8> = std::env::args().skip(1).map(|x| x.parse().unwrap()).collect();
    if notes.is_empty() {
        println!("key   f0 Hz   Re Y at f0   reference   ratio");
        let mut logs = Vec::new();
        for note in 48u8..=84 {
            let np = model.note(note);
            let f0 = partial_freq(np.f0, np.b, 1);
            let g = sb.bridge_admittance(&sb.bridge_shapes(note), TAU * f0).re;
            let r = (-8.836 + 0.289 * f0.ln() - 0.0146 * note as f64).exp();
            logs.push((g / r).ln());
            println!("{:<4} {:7.1}   {:9.2e}   {:9.2e}   {:5.2}{}", name(note), f0, g, r, g / r, if g / r > 2.5 { "  >2.5" } else { "" });
        }
        let n = logs.len() as f64;
        let mean = logs.iter().sum::<f64>() / n;
        let rms = (logs.iter().map(|l| (l - mean).powi(2)).sum::<f64>() / n).sqrt();
        let own = |f: f64| sb.modes.iter().filter(|m| m.m > 0 && m.omega < TAU * f).count();
        println!("modes below 200/300/500 Hz: {}/{}/{}, lowest {:.1} Hz, all {}", own(200.0), own(300.0), own(500.0), sb.modes.iter().find(|m| m.m > 0).unwrap().omega / TAU, sb.modes.len());
        println!("geometric mean ratio {:.2}, rms {:.2} (×{:.2}), over 2.5×: {}", mean.exp(), rms, rms.exp(), logs.iter().filter(|l| **l > 2.5f64.ln()).count());
        return;
    }
    for note in notes {
        let np = model.note(note);
        let f0 = partial_freq(np.f0, np.b, 1);
        let w = TAU * f0;
        let shapes = sb.bridge_shapes(note);
        let total = sb.bridge_admittance(&shapes, w).re;
        println!("{}  f0 {f0:.1} Hz  Re Y {total:.3e}  drive length {:.3} m", name(note), sb.drive_length(w));
        let mut rows: Vec<(f64, usize)> = sb
            .modes
            .iter()
            .zip(&shapes)
            .enumerate()
            .map(|(i, (md, phi))| {
                let den = C64::new(md.omega * md.omega - w * w, sb.mode_eta(md) * md.omega * w);
                ((C64::new(0.0, w * phi * phi) / den).re, i)
            })
            .collect();
        rows.sort_by(|a, b| b.0.total_cmp(&a.0));
        for &(g, i) in rows.iter().take(6) {
            let md = &sb.modes[i];
            let eta = sb.mode_eta(md);
            println!(
                "  {:7.1} Hz ({:2},{:2}) ritz {:>4}  η {:.4} (own {:.4})  Φ² {:.3e}  peak {:.2e}  share {:5.1} %",
                md.omega / TAU,
                md.m,
                md.n,
                md.ritz.map_or("-".into(), |r| r.to_string()),
                eta,
                sb.eta(md.omega),
                shapes[i] * shapes[i],
                shapes[i] * shapes[i] / (eta * md.omega),
                100.0 * g / total
            );
        }
    }
}
