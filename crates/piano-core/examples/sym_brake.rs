//! The free strings' brake on each key's sympathetic drive
//! ([`piano_core::params::SympatheticLaw::brake`]): per key, the brake in dB
//! on its partials 1–8 (the strongest of its coupled modes, 20·log10|b|),
//! the dampers down (undamped keys free) and lifted (every key free).
//!   cargo run --release -p piano-core --example sym_brake -- [key …]
use piano_core::instrument::Instrument;
use piano_core::note_model::sym_brakes;
use piano_core::params::PianoModel;

fn main() {
    let keys: Vec<u8> = std::env::args().skip(1).filter_map(|a| a.parse().ok()).collect();
    let keys = if keys.is_empty() { (21..=108).step_by(3).collect() } else { keys };
    let sr = 48_000.0;
    let inst = Instrument::new(PianoModel::default(), sr);
    let bm = inst.board.as_deref().expect("a soundboard");
    let notes = inst.table.notes();
    let t = std::time::Instant::now();
    let up = sym_brakes(notes, bm, |nm| !nm.has_damper, sr);
    let down = sym_brakes(notes, bm, |_| true, sr);
    eprintln!("brakes in {:.2} s", t.elapsed().as_secs_f64());
    for (name, b) in [("pedal up", &up), ("pedal down", &down)] {
        println!("{name}: dB on partials 1–8 (weakest coupled mode of each)");
        for &key in &keys {
            let i = (key - 21) as usize;
            let nm = &notes[i];
            let per: Vec<String> = (0..8)
                .map(|k| {
                    let idx = k * nm.n_osc;
                    if idx + nm.n_osc > nm.n_listen {
                        return "    —".into();
                    }
                    let worst = b[i][idx..idx + nm.n_osc].iter().map(|c| c.norm()).fold(1.0, f64::min);
                    format!("{:5.1}", 20.0 * worst.log10())
                })
                .collect();
            println!("  {key:3}: {}", per.join(" "));
        }
    }
}
