//! Loss budget of a preset's bass partials: the string's own loss, the
//! bridge's conductance at the partial and the decay it gives one string,
//! 8.69·(T/L)·Re Y dB/s, the coupled modes the engine runs, and the board
//! modes that carry that conductance.
//!   cargo run --release -p piano-host --example bass_loss -- preset.toml [first last [partials]]
use piano_core::params::partial_freq;
use std::f64::consts::TAU;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let model = piano_host::load_preset(std::path::Path::new(&args[1])).unwrap();
    let lo: u8 = args.get(2).and_then(|v| v.parse().ok()).unwrap_or(21);
    let hi: u8 = args.get(3).and_then(|v| v.parse().ok()).unwrap_or(28);
    let kmax: u32 = args.get(4).and_then(|v| v.parse().ok()).unwrap_or(8);
    let sr = 48_000.0;
    let sb = model.board().unwrap();
    let modes = piano_core::board::board_modes(&sb, sr);
    let frame = piano_core::structure::FrameAdmittance::new(&model.frame, &sb.params);
    let db = |s: f64| 8.686 * s;
    if sb.params.structure.strings {
        let h = sb.params.structure.hold.0;
        let (bass, main): (f32, f32) = (h[..(sb.params.main_bridge_from as usize - 21)].iter().sum(), h[(sb.params.main_bridge_from as usize - 21)..].iter().sum());
        println!("the strings' hold: A0 {:.1}, C4 {:.1}, C7 {:.1} kN/m; bass bridge {:.2}, main bridge {:.2} MN/m", h[0] / 1e3, h[39] / 1e3, h[75] / 1e3, bass / 1e6, main / 1e6);
    }
    println!("first board modes: Hz, η own (wood, air, low corner), η with the strings' load");
    for md in sb.modes.iter().take(14) {
        println!("  {:6.1} Hz  {:.3}  {:.3}{}", md.omega / TAU, sb.mode_eta(md) - 2.0 * md.load / md.omega, sb.mode_eta(md), if md.m == 0 { "  (piston)" } else { "" });
    }
    println!("key  k    f Hz  own dB/s  Re Y dB   bridge dB/s  frame dB/s | engine's modes dB/s [weight dB] | board modes carrying Re Y: Hz (share %)");
    for note in lo..=hi {
        let np = model.note(note);
        let nm = piano_core::note_model::NoteModel::build(&model, Some(&sb), Some(&modes), note, sr, 1200);
        let shapes = sb.bridge_shapes(note);
        for k in 1..=kmax {
            let f = partial_freq(np.f0, np.b, k);
            let w = TAU * f;
            let y = sb.bridge_admittance(&shapes, w);
            let g = y.re + sb.edge_conductance(note, w);
            let own = model.losses.sigma(note, f);
            let fr = frame.as_ref().map_or(0.0, |fa| fa.at(sb.params.key_across(note), w).re);
            let ms: Vec<_> = nm.modes.iter().zip(&nm.phys).filter(|(_, p)| p.k == k).map(|(m, _)| m).collect();
            let wmax = ms.iter().map(|m| m.out_re.hypot(m.out_im)).fold(1e-300, f64::max);
            let list: Vec<String> = ms.iter().map(|m| format!("{:5.1}[{:+4.0}]", db(m.sigma), 20.0 * (m.out_re.hypot(m.out_im) / wmax).log10())).collect();
            let mut parts: Vec<(f64, f64)> = sb
                .modes
                .iter()
                .zip(&shapes)
                .map(|(md, phi)| {
                    let e = sb.mode_eta(md) * md.omega * w;
                    let d = md.omega * md.omega - w * w;
                    (md.omega / TAU, w * phi * phi * e / (d * d + e * e))
                })
                .collect();
            let total: f64 = parts.iter().map(|p| p.1).sum();
            parts.sort_by(|a, b| b.1.total_cmp(&a.1));
            let top: Vec<String> = parts.iter().take(4).map(|p| format!("{:.0} ({:.0})", p.0, 100.0 * p.1 / total)).collect();
            if std::env::var("CSV").is_ok() {
                let piston: f64 = sb.modes.iter().zip(&shapes).filter(|(md, _)| md.m == 0).map(|(md, phi)| {
                    let e = sb.mode_eta(md) * md.omega * w;
                    let d = md.omega * md.omega - w * w;
                    w * phi * phi * e / (d * d + e * e)
                }).sum();
                let sig: Vec<String> = ms.iter().map(|m| format!("{:.3}", db(m.sigma))).collect();
                println!("csv,{note},{k},{f:.2},{:.3},{:.3},{:.3},{}", db(own), db(np.tension / np.length * g), db(np.tension / np.length * piston), sig.join(","));
                continue;
            }
            println!(
                "{note:3} {k:2} {f:7.1} {:8.2} {:8.1} {:10.1} {:10.1}   | {} | {}",
                db(own),
                10.0 * g.max(1e-12).log10(),
                db(np.tension / np.length * g),
                db(np.tension / np.length * fr),
                list.join(" "),
                top.join(", ")
            );
        }
        println!("    {} string(s), T {:.0} N, L {:.3} m, T/L {:.0} N/m, Z0 {:.1} kg/s", np.strings, np.tension, np.length, np.tension / np.length, (np.tension * np.mu).sqrt());
    }
}
