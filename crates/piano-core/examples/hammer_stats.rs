//! Contact duration and peak force of the hammer, per key, velocity and
//! felt hardness (analysis).
use piano_core::engine::strike_stats;
use piano_core::EngineConfig;

fn main() {
    for (h, hy) in [(0.35, 1.0), (1.0, 1.0)] {
        let mut cfg = EngineConfig::default();
        cfg.model.hardness = h;
        cfg.model.hysteresis = hy;
        let line: Vec<String> = [(33u8, 100u8), (60, 40), (60, 100), (84, 40), (84, 100), (96, 40), (96, 100), (105, 40), (105, 100)]
            .iter()
            .map(|&(n, v)| {
                let (t, f) = strike_stats(&cfg, n, v, None);
                format!("{n}/v{v}: {:.2} ms {:.0} N", t * 1e3, f)
            })
            .collect();
        println!("hardness {h} hysteresis {hy}: {}", line.join(" | "));
    }
}
