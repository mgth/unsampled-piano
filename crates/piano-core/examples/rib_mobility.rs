//! The board's driving-point conductance at every key's bridge point,
//! sampled at its partials up to 8 kHz: per third octave, the median and
//! the spread (p75 − p25) of 10·log10 Re(Y) (dB re 1 s/kg). Ribs confine
//! the vibration above ≈ 1 kHz on a real board (Ege et al. 2013), which
//! lowers and roughens the mobility there.
//!   cargo run --release -p piano-core --example rib_mobility
//! (RIB_SPACING, RIB_SHARE, RIB_JITTER, BEAM, SPREAD and BELOW override the board;
//! docs/ribs.md, docs/bridge_beam.md.)
use piano_core::params::PianoModel;
use piano_core::soundboard::Soundboard;
use std::f64::consts::TAU;

fn main() {
    let model = PianoModel::default();
    let mut p = model.soundboard.expect("a soundboard").resolved();
    let env = |k: &str| std::env::var(k).ok().and_then(|v| v.parse::<f64>().ok());
    if let Some(v) = env("RIB_SPACING") {
        p.rib_spacing = v;
    }
    if let Some(v) = env("RIB_SHARE") {
        p.rib_share = v;
    }
    if let Some(v) = env("RIB_JITTER") {
        p.rib_jitter = v;
    }
    if let Some(v) = env("BEAM") {
        p.bridge_beam = v;
    }
    if let Some(v) = env("SPREAD") {
        p.bridge_spread = v;
    }
    if let Some(v) = env("BELOW") {
        p.outline_below = v;
    }
    let t = std::time::Instant::now();
    let mut sb = Soundboard::new(p);
    let built = t.elapsed();
    sb.load_strings(&model.damped_strings());
    let mut samples: Vec<(f64, f64)> = Vec::new();
    for note in 21u8..=108 {
        let shapes = sb.bridge_shapes(note);
        let np = model.note(note);
        for k in 1..=60 {
            let f = np.f0 * k as f64 * (1.0 + np.b * (k * k) as f64).sqrt();
            if f > 8000.0 {
                break;
            }
            let y = sb.bridge_admittance(&shapes, TAU * f);
            samples.push((f, 10.0 * y.re.max(1e-12).log10()));
        }
    }
    println!("{} modes, built in {:.2} s", sb.modes.len(), built.as_secs_f64());
    println!("third oct |  median  spread  (10·log10 Re Y, dB re 1 s/kg)");
    let mut c = 125.0;
    while c < 9000.0 {
        let mut v: Vec<f64> = samples.iter().filter(|s| s.0 >= c / 2f64.powf(1.0 / 6.0) && s.0 < c * 2f64.powf(1.0 / 6.0)).map(|s| s.1).collect();
        if v.len() >= 3 {
            v.sort_by(f64::total_cmp);
            let q = |x: f64| v[((v.len() - 1) as f64 * x).round() as usize];
            println!("{:9.0} | {:7.1} {:7.1}", c, q(0.5), q(0.75) - q(0.25));
        }
        c *= 2f64.powf(1.0 / 3.0);
    }
}
