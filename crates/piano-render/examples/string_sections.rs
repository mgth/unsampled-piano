//! Where a preset's sections of strings (`soundboard.string_sections`) put
//! each key on its bridge, against the same preset without them (analysis):
//! the bridge point's move (mm) along the bridge and across the strings,
//! and the step from the key below.
//!   cargo run --release -p piano-render --example string_sections -- <preset.toml>

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let model = piano_host::load_preset(std::path::Path::new(&a[0])).unwrap().with_macros();
    let with = model.soundboard.expect("a soundboard");
    let without = piano_core::soundboard::SoundboardParams { string_sections: Default::default(), ..with };
    println!("| key | MIDI | x (m) | y (m) | moved (mm) | across (mm) | step from the key below (mm) | without |");
    println!("|---|---|---|---|---|---|---|---|");
    let d = |p: (f64, f64), q: (f64, f64)| 1e3 * (p.0 - q.0).hypot(p.1 - q.1);
    for n in 21..=108u8 {
        let (p, q) = (with.bridge_point(n), without.bridge_point(n));
        let step = |s: &piano_core::soundboard::SoundboardParams| if n > 21 && (n == s.main_bridge_from) == false { d(s.bridge_point(n), s.bridge_point(n - 1)) } else { 0.0 };
        let (sin, cos) = with.string_slant(n as f64).sin_cos();
        let across = 1e3 * (-(p.0 - q.0) * sin + (p.1 - q.1) * cos);
        println!("| {} | {n} | {:.3} | {:.3} | {:.1} | {:+.1} | {:.1} | {:.1} |", n - 20, p.0, p.1, d(p, q), across, step(&with), step(&without));
    }
}
