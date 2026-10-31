//! The board's conductance at each key's fundamental, against its smooth
//! trend (a ±1/3-octave mean) and the reference fit of
//! tools/bridge_conductance.py: keys whose fundamental sits on a board
//! resonance lose it fast (D4–E4, docs/felt.md).
use piano_core::params::{partial_freq, PianoModel};
use piano_core::soundboard::Soundboard;

fn main() {
    let model = PianoModel::default();
    let mut params = model.soundboard.unwrap();
    // Trial: LOSS=0.03 sets the board's loss factor.
    if let Some(l) = std::env::var("LOSS").ok().and_then(|v| v.parse().ok()) {
        params.loss_factor = l;
    }
    let mut sb = Soundboard::new(params);
    sb.load_strings(&model.damped_strings());
    println!("key   f0 Hz   Re Y at f0   ±1/3 oct mean   ratio dB   reference");
    for note in 48u8..=84 {
        let np = model.note(note);
        let shapes = sb.bridge_shapes(note);
        let f0 = partial_freq(np.f0, np.b, 1);
        let g = |f: f64| sb.bridge_admittance(&shapes, std::f64::consts::TAU * f).re;
        let n = 200;
        let mean = (0..n).map(|i| g(f0 * 2f64.powf((i as f64 / (n - 1) as f64 - 0.5) * 2.0 / 3.0))).sum::<f64>() / n as f64;
        let r = (-8.836 + 0.289 * f0.ln() - 0.0146 * note as f64).exp();
        let names = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
        println!("{:<4} {:7.1}   {:9.2e}   {:9.2e}   {:+6.1}   {:9.2e}", format!("{}{}", names[note as usize % 12], note / 12 - 1), f0, g(f0), mean, 10.0 * (g(f0) / mean).log10(), r);
    }
}
