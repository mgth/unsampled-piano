//! Contact duration and peak force versus MIDI velocity (default model),
//! next to the hammer speed: how much the strike shortens when played
//! louder. Askenfelt & Jansson (1991) measure on C4 about 4 ms at pp down
//! to about 1.5–2 ms at ff.
use piano_core::engine::{hammer_velocity, strike_stats};
use piano_core::EngineConfig;

fn main() {
    let args: Vec<f64> = std::env::args().skip(1).map(|a| a.parse().unwrap()).collect();
    let mut cfg = EngineConfig::default();
    if let [h, p, y] = args[..] {
        cfg.model.hardness = h;
        cfg.model.p_offset = p;
        cfg.model.hysteresis = y;
    }
    println!("hardness {} p_offset {} hysteresis {}", cfg.model.hardness, cfg.model.p_offset, cfg.model.hysteresis);
    for note in [36u8, 60, 84] {
        let line: Vec<String> = [20u8, 40, 64, 90, 110, 127]
            .iter()
            .map(|&v| {
                let (t, f) = strike_stats(&cfg, note, v, None);
                format!("v{v} {:.1} m/s: {:.2} ms {:.0} N", hammer_velocity(&cfg, v), t * 1e3, f)
            })
            .collect();
        println!("note {note}: {}", line.join(" | "));
    }
}
