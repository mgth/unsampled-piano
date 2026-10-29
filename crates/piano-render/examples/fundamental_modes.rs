//! Isolated H1 modal envelope after the actual hammer contact. Research only.
//! fundamental_modes PRESET NOTE VELOCITY
//! Compare coherent and phase-averaged modal power; excludes later sympathetic input.
use piano_core::{
    EngineConfig,
    complex::C64,
    engine::{contact_oversampling, hammer_velocity},
    instrument::Instrument,
    voice::Voice,
};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<String> = std::env::args().collect();
    if a.len() != 4 {
        return Err("fundamental_modes PRESET NOTE VELOCITY".into());
    }
    let model = piano_host::load_preset(Path::new(&a[1]))?;
    let note: u8 = a[2].parse()?;
    let velocity: u8 = a[3].parse()?;
    if !(21..=108).contains(&note) || !(1..=127).contains(&velocity) {
        return Err("invalid note/velocity".into());
    }
    let cfg = EngineConfig {
        model,
        ..Default::default()
    };
    let inst = Instrument::new(model, cfg.sample_rate);
    let nm = inst.table.get(note);
    let mut voice = Voice::new(cfg.sample_rate, 128);
    voice.setup(nm, false);
    voice.strike(
        nm,
        hammer_velocity(&cfg, velocity),
        contact_oversampling(&cfg, note),
        0.,
        0.,
        0.,
    );
    while voice.contact.is_active() {
        if voice.contact.run(&mut [0f32], None) == 0 {
            break;
        }
    }
    let z: Vec<_> = (0..nm.n_osc)
        .map(|j| voice.contact.complex_state(j))
        .collect();
    let c = nm.clusters[0].to_modes(&z);
    let amps: Vec<_> = (0..nm.n_osc)
        .map(|j| c[j] * C64::new(nm.modes[j].out_re, nm.modes[j].out_im))
        .collect();
    let fast = nm.modes[..nm.n_osc]
        .iter()
        .map(|m| m.sigma)
        .fold(0f64, f64::max);
    let p0: f64 = amps.iter().map(|a| a.norm_sqr()).sum();
    println!("time,coherent_db,phase_averaged_db,interference_db,fast_db,slow_db");
    for t in [0., 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 1.] {
        let mut sum = C64::ZERO;
        let (mut power, mut fast_power, mut slow_power) = (0., 0., 0.);
        for (j, a) in amps.iter().enumerate() {
            let m = &nm.modes[j];
            let v = *a * C64::polar((-m.sigma * t).exp(), std::f64::consts::TAU * m.freq_hz * t);
            sum += v;
            power += v.norm_sqr();
            if m.sigma > fast / 1.5 {
                fast_power += v.norm_sqr();
            } else {
                slow_power += v.norm_sqr();
            }
        }
        let db = |p: f64| 10. * (p.max(1e-30) / p0).log10();
        println!(
            "{t},{},{},{},{},{}",
            db(sum.norm_sqr()),
            db(power),
            10. * (sum.norm_sqr() / power).log10(),
            db(fast_power),
            db(slow_power)
        );
    }
    Ok(())
}
