//! Worst case (all keys at 127 every 30 ms, pedal down) under variants.
use piano_core::instrument::Instrument;
use piano_core::{Engine, EngineConfig, Event, TimedEvent};
use std::sync::Arc;
use std::time::Instant;

fn run(label: &str, cfg: EngineConfig) {
    let inst = Arc::new(Instrument::new(cfg.model, cfg.sample_rate));
    let mut e = Engine::with_instrument(cfg, inst);
    let bs = cfg.max_block;
    let (mut l, mut r) = (vec![0.0f32; bs], vec![0.0f32; bs]);
    let total = (4.64 * cfg.sample_rate) as usize / bs;
    let step = (0.03 * cfg.sample_rate) as usize;
    let t = Instant::now();
    for b in 0..total {
        let mut ev = Vec::new();
        if b == 0 {
            ev.push(TimedEvent { frame: 0, event: Event::Control { cc: 64, value: 127 } });
        }
        for s in 0..bs {
            let n = b * bs + s;
            if n.is_multiple_of(step) && n / step < 88 {
                ev.push(TimedEvent { frame: s as u32, event: Event::NoteOn { note: 21 + (n / step) as u8, velocity: 127 } });
            }
        }
        e.process_stereo(&ev, &mut l, &mut r);
    }
    let el = t.elapsed().as_secs_f64();
    println!("{label:32} {:5.2}× real time ({:.0} % of a core), {} modes at the end", 4.64 / el, 100.0 * el / 4.64, e.active_modes());
}

fn main() {
    let base = EngineConfig::default();
    run("default", base);
    let mut c = base;
    c.model.sympathetic.enabled = false;
    run("no sympathy", c);
    let mut c = base;
    c.model.sympathetic.max_freq = 2000.0;
    run("sympathy below 2 kHz", c);
    let mut c = base;
    c.model.frame.enabled = false;
    run("no frame", c);
    let mut c = base;
    c.model.rim.enabled = false;
    run("no rim", c);
    let mut c = base;
    c.model.rim.couple_f_max = 1.0;
    run("rim uncoupled (bank only)", c);
    c.model.frame.enabled = false;
    run("no frame, no rim", c);
    let mut c = base;
    c.model.soundboard = None;
    run("no soundboard", c);
}
