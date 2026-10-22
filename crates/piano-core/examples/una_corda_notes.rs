//! Una corda across the keyboard: late/early level change (dB) with the
//! pedal, the measure of tests/pedals.rs.
use piano_core::{Engine, EngineConfig, Event, TimedEvent};

fn render(soft: u8, note: u8) -> Vec<f32> {
    let mut cfg = EngineConfig::default();
    cfg.model.room.enabled = false;
    let mut e = Engine::new(cfg);
    let mut out = vec![0.0f32; 128];
    let mut all = Vec::new();
    for b in 0..(48_000 / 128) {
        let mut ev = vec![];
        if b == 0 {
            ev.push(TimedEvent { frame: 0, event: Event::Control { cc: 67, value: soft } });
            ev.push(TimedEvent { frame: 1, event: Event::NoteOn { note, velocity: 100 } });
        }
        e.process(&ev, &mut out);
        all.extend_from_slice(&out);
    }
    all
}

fn main() {
    let rms = |x: &[f32], a: f64, b: f64| {
        let s = &x[(a * 48_000.0) as usize..(b * 48_000.0) as usize];
        10.0 * (s.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / s.len() as f64).log10()
    };
    for note in [48u8, 55, 60, 64, 67, 72, 79] {
        let (a, b) = (render(0, note), render(127, note));
        let s0 = rms(&a, 0.6, 1.0) - rms(&a, 0.0, 0.1);
        let s1 = rms(&b, 0.6, 1.0) - rms(&b, 0.0, 0.1);
        println!("note {note}: late/early {s0:6.1} → {s1:6.1} dB ({:+.1})", s1 - s0);
    }
}
