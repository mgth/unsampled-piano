//! How fast a note dies after the key is released (dampers), per key: time
//! to fall 20 and 40 dB below the level just before the release, dry.
use piano_core::{Engine, EngineConfig, Event, TimedEvent};

fn main() {
    let mut cfg = EngineConfig::default();
    cfg.model.room.enabled = false;
    cfg.model.noise = piano_core::params::NoiseLaw::OFF;
    if let Some(s) = std::env::args().nth(1) {
        cfg.model.damper.strength = s.parse().unwrap();
    }
    println!("damper strength {}", cfg.model.damper.strength);
    let sr = cfg.sample_rate;
    for note in [28u8, 40, 52, 60, 72, 84] {
        let mut e = Engine::new(cfg);
        let bs = cfg.max_block;
        let mut out = vec![0.0f32; bs];
        let mut x = Vec::new();
        let off_at = (1.0 * sr) as usize / bs;
        for b in 0..(3.0 * sr) as usize / bs {
            let mut ev = Vec::new();
            if b == 0 {
                ev.push(TimedEvent { frame: 0, event: Event::NoteOn { note, velocity: 70 } });
            }
            if b == off_at {
                ev.push(TimedEvent { frame: 0, event: Event::NoteOff { note, velocity: 64 } });
            }
            e.process(&ev, &mut out);
            x.extend_from_slice(&out);
        }
        let h = 240; // 5 ms windows
        let env: Vec<f64> = x.chunks(h).map(|c| 10.0 * (c.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / h as f64 + 1e-30).log10()).collect();
        let k0 = off_at * bs / h;
        let before = env[k0 - 4..k0].iter().cloned().fold(f64::MIN, f64::max);
        let cross = |db: f64| env[k0..].iter().position(|&v| v < before - db).map_or(f64::NAN, |i| i as f64 * h as f64 / sr * 1000.0);
        println!("note {note:3}: −20 dB after {:6.0} ms, −40 dB after {:6.0} ms", cross(20.0), cross(40.0));
    }
}
