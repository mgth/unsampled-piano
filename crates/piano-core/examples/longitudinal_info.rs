//! Longitudinal model per key: core diameter, EA, first longitudinal
//! frequency, partials and modes used.
use piano_core::params::PianoModel;
use piano_core::soundboard::Soundboard;

fn main() {
    let model = PianoModel::default();
    let sb = Soundboard::new(model.soundboard.unwrap());
    for note in [21u8, 28, 33, 40, 48, 60, 72] {
        let nm = piano_core::note_model::NoteModel::build(&model, Some(&sb), None, note, 48_000.0, piano_core::engine::MAX_MODES);
        let np = &nm.params;
        let d4 = 64.0 * np.b * np.tension * np.length * np.length / (std::f64::consts::PI.powi(3) * model.nonlinear.young);
        match &nm.longitudinal {
            Some(lg) => {
                let ea = 4.0 * lg.ea4 as f64;
                let f1 = (ea / np.mu).sqrt() / (2.0 * np.length);
                println!(
                    "key {note}: core {:.2} mm, EA {:.2e} N, f_L1 {:.0} Hz ({:.1}× f0), {} partials, {} longitudinal modes",
                    d4.powf(0.25) * 1e3,
                    ea,
                    f1,
                    f1 / np.f0,
                    lg.slope.len(),
                    lg.long_pole.len()
                );
            }
            None => println!("key {note}: none"),
        }
    }
}
