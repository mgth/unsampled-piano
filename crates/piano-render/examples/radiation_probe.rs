//! The board's radiation per mode at the first microphone: per third
//! octave of mode frequency, the mean |pressure per unit modal velocity|²
//! (dB) over the modes in the band, and the mean driving-point conductance
//! of the modes from a key's bridge point. Shows where the radiation jumps
//! (coincidence) for a listening set-up.
//!   cargo run --release -p piano-render --example radiation_probe -- <preset.toml> [listening.toml …] [--note N]

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut model = piano_host::load_preset(std::path::Path::new(&args[1])).unwrap();
    let mut note = 45u8;
    let mut i = 2;
    while i < args.len() {
        if args[i] == "--note" {
            note = args[i + 1].parse().unwrap();
            i += 2;
        } else {
            model = piano_host::load_listening(std::path::Path::new(&args[i]), &model).unwrap();
            i += 1;
        }
    }
    let model = model.with_macros();
    let model = piano_core::params::PianoModel { soundboard: model.soundboard.map(|s| s.resolved()), ..model };
    let sb = model.board().unwrap();
    let rad = sb.radiation();
    let (x, y) = sb.params.bridge_point(note);
    println!("mics {:?}; drive at note {note} ({x:.2}, {y:.2}) m", sb.params.mics);
    println!("third oct  modes   |w|² mean dB   Φ² mean dB   |w·Φ|² mean dB");
    let mut c = 62.5f64;
    while c < 10000.0 {
        let (lo, hi) = (c / 2f64.powf(1.0 / 6.0), c * 2f64.powf(1.0 / 6.0));
        let (mut sw, mut sp, mut swp, mut n) = (0.0, 0.0, 0.0, 0);
        for (md, w) in sb.modes.iter().zip(&rad) {
            let f = md.omega / std::f64::consts::TAU;
            if f >= lo && f < hi {
                let phi = sb.shape(md, x, y);
                let w2 = 0.5 * (w[0].norm_sqr() + w[1].norm_sqr());
                sw += w2;
                sp += phi * phi;
                swp += w2 * phi * phi;
                n += 1;
            }
        }
        if n > 0 {
            println!("{c:8.0} {n:6} {:12.1} {:12.1} {:14.1}", 10.0 * (sw / n as f64).log10(), 10.0 * (sp / n as f64).log10(), 10.0 * (swp / n as f64).log10());
        }
        c *= 2f64.powf(1.0 / 3.0);
    }
}
