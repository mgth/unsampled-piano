//! Contact duration and peak force versus MIDI velocity (default model),
//! next to the hammer speed: how much the strike shortens when played
//! louder. Askenfelt & Jansson (1991) measure on C4 about 4 ms at pp down
//! to about 1.5–2 ms at ff.
use piano_core::engine::{hammer_velocity, strike_stats_full};
use piano_core::EngineConfig;

fn main() {
    let args: Vec<f64> = std::env::args().skip(1).map(|a| a.parse().unwrap()).collect();
    let mut cfg = EngineConfig::default();
    if let [h, p, y] = args[..] {
        cfg.model.hardness = h;
        cfg.model.p_offset = p;
        cfg.model.hysteresis = y;
    }
    if let [_, _, _, sb] = args[..] {
        cfg.model.hardness = args[0];
        cfg.model.p_offset = args[1];
        cfg.model.hysteresis = args[2];
        cfg.model.hardness_slope_bass = sb;
    }
    println!("hardness {} p_offset {} hysteresis {}", cfg.model.hardness, cfg.model.p_offset, cfg.model.hysteresis);
    println!("bass slope {}", cfg.model.hardness_slope_bass);
    for note in [21u8, 36, 48, 60, 72, 84, 96] {
        let line: Vec<String> = [40u8, 100]
            .iter()
            .map(|&v| {
                let (t, f, u) = strike_stats_full(&cfg, note, v, None);
                format!("v{v} {:.1} m/s: {:.2} ms {:.0} N {:.2} mm", hammer_velocity(&cfg, v), t * 1e3, f, u * 1e3)
            })
            .collect();
        println!("note {note}: {}", line.join(" | "));
    }
}
