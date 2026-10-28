//! Exact discrete board transfer, with region or key drive and band decomposition.
//! ear_transfer PRESET NOTE FREQUENCIES_CSV [region|key]
//! Outputs pressure/force and bridge acceleration/force, before mic/output gains.
use piano_core::{
    board::{self, MICS, REGIONS},
    complex::C64,
    params::PianoModel,
};
use std::f64::consts::TAU;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<String> = std::env::args().collect();
    let model = piano_host::load_preset(std::path::Path::new(&a[1]))?.with_macros();
    let model = PianoModel {
        soundboard: model.soundboard.map(|s| s.resolved()),
        ..model
    };
    let note: u8 = a[2].parse()?;
    let sr = 48000.;
    let sb = model.board().ok_or("no board")?;
    let mut modes = board::board_modes(&sb, sr);
    let r = board::region_of(note);
    if a.get(4).is_some_and(|s| s == "key") {
        let shapes = sb.bridge_shapes(note);
        for (m, phi) in modes.iter_mut().zip(shapes) {
            for j in 0..REGIONS {
                m.inputs[j] = phi / sr;
            }
            let shape = C64::new(1., m.sigma / (TAU * m.freq_hz)).scale(phi);
            for o in &mut m.outputs[MICS..] {
                *o = (shape.re, shape.im);
            }
        }
    }
    board::to_acceleration_weights(&mut modes, sr);
    println!("frequency,band,pl_re,pl_im,pr_re,pr_im,a_re,a_im");
    for f in a[3].split(',').map(str::parse::<f64>) {
        let f = f?;
        let w = TAU * f;
        let acc = board::transfer(&modes, r, MICS + r, w, sr) * board::mic_differentiator(w, sr);
        for (label, lo, hi) in [
            ("all", 0., 1e9),
            ("low", 0., 1500.),
            ("mid", 1500., 4000.),
            ("upper", 4000., 8000.),
            ("near", 8000., 12000.),
            ("high", 12000., 1e9),
        ] {
            let mut selected = modes.clone();
            for m in &mut selected {
                if m.freq_hz < lo || m.freq_hz >= hi {
                    for o in &mut m.outputs[..MICS] {
                        *o = (0., 0.);
                    }
                }
            }
            let l = board::mic_transfer(&selected, r, 0, w, sr);
            let rr = board::mic_transfer(&selected, r, 1, w, sr);
            println!(
                "{f},{label},{},{},{},{},{},{}",
                l.re, l.im, rr.re, rr.im, acc.re, acc.im
            );
        }
    }
    Ok(())
}
