//! Hammer force (N) per string at each output sample, one key and velocity,
//! as text (time ms, f1, f2, f3).
//!   cargo run --release -p piano-render --example strike_force_dump -- note velocity
use piano_core::engine::strike_forces;
use piano_core::EngineConfig;

fn main() {
    let cfg = EngineConfig::default();
    let a: Vec<u8> = std::env::args().skip(1).map(|s| s.parse().unwrap()).collect();
    let dt = 1e3 / cfg.sample_rate;
    for (i, f) in strike_forces(&cfg, a[0], a[1]).iter().enumerate() {
        println!("{:.4} {:.4} {:.4} {:.4}", i as f64 * dt, f[0], f[1], f[2]);
    }
}
