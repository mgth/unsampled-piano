//! All coupled modes of one partial of one key: frequency, T60, output
//! weight (bridge force) and input weight (hammer, through the strings).
use piano_core::params::PianoModel;
use piano_core::soundboard::Soundboard;

fn main() {
    let a: Vec<u32> = std::env::args().skip(1).map(|x| x.parse().unwrap()).collect();
    let (note, k) = (a[0] as u8, a[1]);
    let model = PianoModel::default();
    let sb = Soundboard::new(model.soundboard.unwrap());
    let modes = piano_core::board::board_modes(&sb, 48_000.0);
    let nm = piano_core::note_model::NoteModel::build(&model, Some(&sb), Some(&modes), note, 48_000.0, piano_core::engine::MAX_MODES);
    for (m, p) in nm.modes.iter().zip(&nm.phys).filter(|(_, p)| p.k == k) {
        println!(
            "{:9.3} Hz  T60 {:6.2} s  |w| {:9.3e}  (phys osc {:8.3} Hz, σ0 {:.3})",
            m.freq_hz,
            6.91 / m.sigma,
            m.out_re.hypot(m.out_im),
            p.freq,
            p.sigma
        );
    }
}
