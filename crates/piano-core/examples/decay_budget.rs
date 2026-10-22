//! Where the energy of the first partials goes: intrinsic string loss vs
//! bridge (board) loss, per key, for the coupled unison modes.
use piano_core::params::PianoModel;
use piano_core::soundboard::Soundboard;

fn main() {
    let model = PianoModel::default();
    let sr = 48_000.0;
    let sb = Soundboard::new(model.soundboard.unwrap());
    let modes = piano_core::board::board_modes(&sb, sr);
    let t60 = |s: f64| 6.91 / s;
    println!("note  k   f (Hz)  T60 string  Re Y (s/kg)  |Y|·T/L    T60 of the coupled modes (fast … slow), [out weight dB]");
    for note in (45..=99).step_by(3) {
        let nm = piano_core::note_model::NoteModel::build(&model, Some(&sb), Some(&modes), note, sr, 1200);
        let np = model.note(note);
        let shapes = sb.bridge_shapes(note);
        for k in 1..=2u32 {
            let ms: Vec<_> = nm.modes.iter().zip(&nm.phys).filter(|(_, p)| p.k == k).map(|(m, _)| m).collect();
            if ms.is_empty() {
                continue;
            }
            let f = ms[0].freq_hz;
            let yb = sb.bridge_admittance(&shapes, std::f64::consts::TAU * f);
            let sig_s = model.losses.sigma(note, f);
            let wmax = ms.iter().map(|m| m.out_re.hypot(m.out_im)).fold(0.0, f64::max);
            let list: Vec<String> = ms
                .iter()
                .map(|m| format!("{:5.2}[{:+4.0}]", t60(m.sigma), 20.0 * (m.out_re.hypot(m.out_im) / wmax).log10()))
                .collect();
            println!(
                "{note:3} {k:3} {f:8.1} {:8.2} s {:10.2e} {:10.2e}   {}",
                t60(sig_s),
                yb.re,
                yb.norm() * np.tension / np.length,
                list.join(" ")
            );
        }
    }
}
