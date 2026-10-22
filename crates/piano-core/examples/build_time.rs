//! Time to build the instrument, and its parts (soundboard modes and
//! radiation, note table, frame, rim).
use piano_core::params::PianoModel;
use piano_core::soundboard::Soundboard;
use std::time::Instant;

fn main() {
    let sr = 48_000.0;
    let model = PianoModel::default();
    let t = Instant::now();
    let inst = piano_core::instrument::Instrument::new(model, sr);
    println!("instrument built in {:?}; soundboard: {} modes", t.elapsed(), inst.board.as_ref().map_or(0, |b| b.len()));
    let t = Instant::now();
    let sb = Soundboard::new(model.soundboard.unwrap());
    let modes = piano_core::board::board_modes(&sb, sr);
    println!("  soundboard modes + radiation: {:?}", t.elapsed());
    let t = Instant::now();
    let _table = piano_core::note_model::PianoTable::new(&model, Some(&sb), Some(&modes), sr, piano_core::voice::MAX_MODES);
    println!("  note table: {:?}", t.elapsed());
    let t = Instant::now();
    let _frame = piano_core::structure::frame_modes(&model.frame, &sb.params, sr);
    println!("  frame: {:?}", t.elapsed());
    let t = Instant::now();
    let _rim = piano_core::structure::RimModel::new(&model.rim, &sb, modes.len(), sr);
    println!("  rim: {:?}", t.elapsed());
}
