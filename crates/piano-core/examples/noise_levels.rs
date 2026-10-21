//! Level of each mechanical noise relative to a mezzo-forte C4, isolated as
//! the difference between renders with and without it.
use piano_core::params::NoiseLaw;
use piano_core::{Engine, EngineConfig, Event, TimedEvent};

fn render(cfg: EngineConfig, script: &[(f64, Event)], dur: f64) -> Vec<f32> {
    let mut e = Engine::new(cfg);
    let bs = cfg.max_block;
    let n = (dur * cfg.sample_rate) as usize / bs;
    let mut out = vec![0.0f32; n * bs];
    for (b, blk) in out.chunks_mut(bs).enumerate() {
        let ev: Vec<TimedEvent> = script
            .iter()
            .filter_map(|&(t, ev)| {
                let f = (t * cfg.sample_rate) as usize;
                (f / bs == b).then_some(TimedEvent { frame: (f % bs) as u32, event: ev })
            })
            .collect();
        e.process(&ev, blk);
    }
    out
}

fn rms_db(x: &[f32]) -> f64 {
    10.0 * (x.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / x.len() as f64).max(1e-30).log10()
}

fn main() {
    let sr = 48_000.0;
    let on = EngineConfig::default();
    let mut off = on;
    off.model.noise = NoiseLaw::OFF;
    let w = |x: &[f32], a: f64, b: f64| rms_db(&x[(a * sr) as usize..(b * sr) as usize]);
    let diff = |a: &[f32], b: &[f32]| a.iter().zip(b).map(|(x, y)| x - y).collect::<Vec<f32>>();

    let script = [(0.1, Event::NoteOn { note: 60, velocity: 64 }), (1.0, Event::NoteOff { note: 60, velocity: 64 })];
    let (a, b) = (render(on, &script, 1.5), render(off, &script, 1.5));
    let d = diff(&a, &b);
    let note = w(&b, 0.1, 0.4);
    println!("C4 mf, first 0.3 s:            {note:6.1} dB");
    println!("keybed knock (0.1–0.15 s):     {:6.1} dB  ({:+.1} dB re note)", w(&d, 0.1, 0.15), w(&d, 0.1, 0.15) - note);
    println!("damper landing (1.0–1.1 s):    {:6.1} dB  ({:+.1} dB re note)", w(&d, 1.0, 1.1), w(&d, 1.0, 1.1) - note);
    let pedal = [(0.1, Event::Control { cc: 64, value: 127 }), (0.8, Event::Control { cc: 64, value: 0 })];
    let p = render(on, &pedal, 1.2);
    println!("pedal down knock (0.1–0.15 s): {:6.1} dB  ({:+.1} dB re note)", w(&p, 0.1, 0.15), w(&p, 0.1, 0.15) - note);
    println!("pedal up knock (0.8–0.85 s):   {:6.1} dB  ({:+.1} dB re note)", w(&p, 0.8, 0.85), w(&p, 0.8, 0.85) - note);
}
