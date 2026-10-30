//! What shutting the lid changes at the listening presets' microphones:
//! mean |pressure per unit modal velocity|² of the board's modes per octave,
//! shut minus open (or removed), in dB (docs/lid.md).
use piano_core::soundboard::{Soundboard, SoundboardParams};

fn bands(sb: &Soundboard) -> Vec<f64> {
    let w = sb.radiation();
    (0..7)
        .map(|i| {
            let fc = 125.0 * 2f64.powi(i);
            let (lo, hi) = (fc / 2f64.sqrt(), fc * 2f64.sqrt());
            let e: f64 = sb.modes.iter().zip(&w).filter(|(m, _)| (lo..hi).contains(&(m.omega / std::f64::consts::TAU))).map(|(_, v)| v[0].norm_sqr() + v[1].norm_sqr()).sum();
            10.0 * e.log10()
        })
        .collect()
}

fn main() {
    let sets: [(&str, f64, [(f64, f64, f64); 2]); 3] = [
        ("audience, 1 m", 0.0, [(0.323529, -1.230769, 1.0), (0.735294, -1.230769, 1.0)]),
        ("concert, 2 m", 35.0, [(0.323529, -1.230769, 2.0), (0.735294, -1.230769, 2.0)]),
        ("close, 40 cm", 35.0, [(0.294118, 0.346154, 0.4), (0.705882, 0.653846, 0.4)]),
    ];
    println!("{:28} {}", "shut − open (dB)", (0..7).map(|i| format!("{:>6.0}", 125.0 * 2f64.powi(i))).collect::<String>());
    for (name, angle, mics) in sets {
        let open = SoundboardParams { lid_angle: angle, mics_relative: true, mics, ..SoundboardParams::GRAND_2M }.resolved();
        let shut = SoundboardParams { lid_shut: 1.0, ..open };
        let (o, s) = (bands(&Soundboard::new(open)), bands(&Soundboard::new(shut)));
        let label = format!("{name} (lid {})", if angle > 0.0 { format!("{angle:.0}°") } else { "removed".into() });
        println!("{label:28} {}", o.iter().zip(&s).map(|(o, s)| format!("{:>+6.1}", s - o)).collect::<String>());
    }
}
