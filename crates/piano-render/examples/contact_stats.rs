//! Contact duration, peak force and felt compression per key and velocity
//! for a preset (hammer voicing analysis).
//!   cargo run --release -p piano-render --example contact_stats -- [preset.toml]
use piano_core::engine::strike_stats_full;
use piano_core::EngineConfig;

fn main() {
    let mut cfg = EngineConfig::default();
    if let Some(p) = std::env::args().nth(1) {
        cfg.model = piano_host::load_preset(std::path::Path::new(&p)).unwrap();
    }
    for note in [21u8, 36, 48, 60, 72, 84, 96] {
        let line: Vec<String> = [20u8, 40, 70, 100, 127]
            .iter()
            .map(|&v| {
                let (t, f, u) = strike_stats_full(&cfg, note, v, None);
                format!("v{v}: {:.2} ms {:.0} N {:.2} mm", t * 1e3, f, u * 1e3)
            })
            .collect();
        println!("key {note}: {}", line.join(" | "));
    }
}
