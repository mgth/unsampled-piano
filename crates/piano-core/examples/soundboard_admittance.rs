//! Prints the bridge mobility of the default soundboard at a few keys
//! (CSV: note,freq,re,im) and its modal statistics.
use piano_core::soundboard::{Soundboard, SoundboardParams};

fn main() {
    let sb = Soundboard::new(SoundboardParams::GRAND_2M);
    eprintln!(
        "{} modes below {} Hz; 1 mode / {:.1} Hz (200–1000 Hz); first mode {:.1} Hz; mean mobility {:.2e} s/kg",
        sb.modes.len(),
        sb.params.f_max,
        1.0 / sb.modal_density(200.0, 1000.0),
        sb.modes[0].omega / std::f64::consts::TAU,
        sb.params.mean_mobility()
    );
    println!("note,freq,re,im");
    for note in [24u8, 45, 60, 84, 100] {
        let (x, y) = sb.params.bridge_point(note);
        let mut f = 20.0;
        while f < 12_000.0 {
            let yv = sb.admittance(x, y, std::f64::consts::TAU * f);
            println!("{note},{f:.2},{:.6e},{:.6e}", yv.re, yv.im);
            f *= 1.002;
        }
    }
}
