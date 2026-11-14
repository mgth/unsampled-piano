//! Hammer force (N) per string at each output sample, one key and velocity,
//! as text (time ms, f1, f2, f3). The default instrument, or a preset's.
//!   cargo run --release -p piano-render --example strike_force_dump -- note velocity [preset.toml [sample rate]]
use piano_core::engine::strike_forces;
use piano_core::EngineConfig;

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let (note, velocity): (u8, u8) = (a[0].parse().unwrap(), a[1].parse().unwrap());
    let mut cfg = EngineConfig::default();
    if let Some(preset) = a.get(2) {
        cfg.model = piano_host::load_preset(std::path::Path::new(preset)).unwrap();
    }
    if let Some(sr) = a.get(3) {
        cfg.sample_rate = sr.parse().unwrap();
    }
    let dt = 1e3 / cfg.sample_rate;
    for (i, f) in strike_forces(&cfg, note, velocity).iter().enumerate() {
        println!("{:.4} {:.4} {:.4} {:.4}", i as f64 * dt, f[0], f[1], f[2]);
    }
}
