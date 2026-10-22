//! Loss budget of the bass partials: intrinsic string T60 versus the T60 of
//! the strongest radiating coupled mode, and the bridge conductance seen.
use piano_core::params::PianoModel;
use piano_core::soundboard::Soundboard;

fn main() {
    let model = PianoModel::default();
    let sr = 48_000.0;
    let sb = Soundboard::new(model.soundboard.unwrap());
    let modes = piano_core::board::board_modes(&sb, sr);
    for note in [24u8, 30, 36, 42, 47] {
        let nm = piano_core::note_model::NoteModel::build(&model, Some(&sb), Some(&modes), note, sr, 1200);
        let shapes = sb.bridge_shapes(note);
        println!("note {note} ({} strings)", model.note(note).strings);
        for k in [1u32, 2, 4, 6, 8, 10, 12] {
            let ms: Vec<_> = nm.modes.iter().zip(&nm.phys).filter(|(_, p)| p.k == k).map(|(m, _)| m).collect();
            if ms.is_empty() {
                continue;
            }
            let f = ms[0].freq_hz;
            let main = ms.iter().max_by(|a, b| a.out_re.hypot(a.out_im).total_cmp(&b.out_re.hypot(b.out_im))).unwrap();
            let s_int = model.losses.sigma(note, f);
            let g = sb.mobility(&shapes, std::f64::consts::TAU * f).re;
            println!(
                "  k {k:2} {f:7.1} Hz: T60 intrinsic {:6.1} s, strongest mode {:5.1} s, Re Y {:.1e} s/kg",
                6.91 / s_int,
                6.91 / main.sigma,
                g
            );
        }
    }
}
