//! What a key's partials give the frame at their agraffes against what they
//! give the board at the bridge: the two conductances, Re Y (s/kg), and
//! their ratio, the share of the string's power the frame takes.
//!   cargo run --release -p piano-core --example frame_drain -- [key …]
//! (RECT=1 without the board's outline, BELOW=Hz its own modes' corner,
//! DIST=1 the bridge points' distance to the outline.)
use piano_core::params::PianoModel;
use piano_core::structure::FrameAdmittance;
use std::f64::consts::TAU;

fn main() {
    let mut model = PianoModel::default();
    // RECT=1: the same board without its outline.
    if let (true, Some(s)) = (std::env::var_os("RECT").is_some(), model.soundboard.as_mut()) {
        s.outline = [(0.0, 0.0); piano_core::soundboard::OUTLINE_POINTS];
    }
    // BELOW=Hz: the outlined board's own modes up to there.
    if let (Some(v), Some(s)) = (std::env::var("BELOW").ok().and_then(|v| v.parse().ok()), model.soundboard.as_mut()) {
        s.outline_below = v;
    }
    let t = std::time::Instant::now();
    let board = model.board().expect("a soundboard");
    eprintln!("board built in {:.2} s", t.elapsed().as_secs_f64());
    let p = board.params;
    let frame = FrameAdmittance::new(&model.frame, &p).expect("a frame");
    // DIST=1: each key's bridge point and its distance to the outline.
    if std::env::var_os("DIST").is_some() {
        let o = piano_core::soundboard::Outline::new(&p);
        for note in (48..=108).step_by(3) {
            let (x, y) = p.bridge_point(note);
            let d = o.as_ref().map_or(f64::NAN, |o| o.edge_distance(x, y));
            println!("{note:3}: bridge ({x:.2}, {y:.2}) m, {:.0} cm from the outline", d * 100.0);
        }
        return;
    }
    let keys: Vec<u8> = std::env::args().skip(1).filter_map(|a| a.parse().ok()).collect();
    let keys = if keys.is_empty() { vec![84, 90, 94, 96, 98, 102] } else { keys };
    println!("key partial   f (Hz)   board Re Y    frame Re Y   frame/board");
    for note in keys {
        let np = model.note(note);
        let shapes = board.bridge_shapes(note);
        for k in 1..=4u32 {
            let f = np.f0 * k as f64 * (1.0 + np.b * (k * k) as f64).sqrt();
            let w = TAU * f;
            let yb = board.bridge_admittance(&shapes, w).re;
            let yf = frame.at(p.key_across(note), w).re;
            println!("{note:3} {k:7} {f:9.1} {yb:12.3e} {yf:13.3e} {:+9.1} dB", 10.0 * (yf / yb).log10());
        }
    }
}
