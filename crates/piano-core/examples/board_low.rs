//! The board's lowest modes, and the bridge mobility at the bass keys'
//! partials 1–3: what the low strings can lose into the board.
use piano_core::params::{partial_freq, PianoModel};
use piano_core::soundboard::Soundboard;

fn main() {
    let model = PianoModel::default();
    let sb = Soundboard::new(model.soundboard.unwrap());
    let mut modes: Vec<_> = sb.modes.iter().map(|m| (m.omega / std::f64::consts::TAU, m.m, m.n, sb.mode_eta(m))).collect();
    modes.sort_by(|a, b| a.0.total_cmp(&b.0));
    println!("lowest modes (Hz, m, n, η):");
    for (f, m, n, eta) in modes.iter().take(14) {
        println!("  {f:7.1} ({m},{n}) η {eta:.3}");
    }
    let p = model.soundboard.unwrap();
    println!("bridge point's distance to the nearest edge (m):");
    for note in (21u8..=108).step_by(6) {
        let (x, y) = p.bridge_point_at(note as f64, note >= p.main_bridge_from);
        println!("  {note:3}: ({x:.2}, {y:.2}) m, {:.2} m", x.min(p.a - x).min(y).min(p.b - y));
    }
    println!("bridge mobility Y (s/kg) at the partials; σ added to one string = Re ε = (T/L)·Re Y");
    for note in [21u8, 24, 28, 33, 36, 40, 45, 48, 55, 60] {
        let np = model.note(note);
        let shapes = sb.bridge_shapes(note);
        let row: Vec<String> = (1..=3)
            .map(|k| {
                let f = partial_freq(np.f0, np.b, k);
                let y = sb.mobility(&shapes, std::f64::consts::TAU * f);
                format!("p{k} {f:6.1} Hz Re {:.1e} Im {:+.1e} σ {:.2}", y.re, y.im, np.tension / np.length * y.re)
            })
            .collect();
        println!("  {note:3} (T/L {:6.0} N/m): {}", np.tension / np.length, row.join(" | "));
    }
}
