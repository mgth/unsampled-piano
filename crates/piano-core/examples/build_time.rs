//! Time to build the instrument (note table + soundboard radiation).
fn main() {
    let t = std::time::Instant::now();
    let inst = piano_core::instrument::Instrument::new(piano_core::params::PianoModel::default(), 48_000.0);
    println!(
        "instrument built in {:?}; board banks: {} × {} modes",
        t.elapsed(),
        inst.board.as_ref().map_or(0, |b| b.banks.len()),
        inst.board.as_ref().map_or(0, |b| b.banks[0].len())
    );
}
