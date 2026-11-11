//! Time to build a preset's instrument, part by part.
//!   build_parts <preset.toml>
use piano_core::soundboard::Soundboard;
use std::time::Instant;

fn main() {
    let path = std::env::args().nth(1).expect("a preset");
    let model = piano_host::load_preset(std::path::Path::new(&path)).unwrap().with_macros();
    let sr = 48_000.0;
    let t = Instant::now();
    let p = model.board().unwrap().params;
    println!("board (modes, air, corner): {:.2} s", t.elapsed().as_secs_f64());
    let t = Instant::now();
    let fem = piano_core::fem::board(&p);
    println!("  modes again: {:.2} s, {} of them", t.elapsed().as_secs_f64(), fem.omega.len());
    let t = Instant::now();
    let sb = Soundboard::new(p.resolved());
    println!("  soundboard again: {:.2} s, {} modes", t.elapsed().as_secs_f64(), sb.modes.len());
    let t = Instant::now();
    let modes = piano_core::board::board_modes(&sb, sr);
    println!("radiation and regions: {:.2} s", t.elapsed().as_secs_f64());
    let t = Instant::now();
    let low = piano_core::board::LowDrive::new(&sb, &modes, p.exact_below, sr);
    println!("drive per key: {:.2} s, {} modes", t.elapsed().as_secs_f64(), low.as_ref().map_or(0, |l| l.n));
    let t = Instant::now();
    let inst = piano_core::instrument::Instrument::new(model, sr);
    println!("whole instrument: {:.2} s, {} board modes", t.elapsed().as_secs_f64(), inst.board.as_ref().map_or(0, |b| b.len()));
}
