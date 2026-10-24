//! What each engine shortcut changes in the signal: one scenario rendered
//! with the default shortcuts, then with one of them turned off.
//!   cargo run --release -p piano-core --example shortcut_diff
use piano_core::engine::Shortcuts;
use piano_core::{Engine, EngineConfig, Event, TimedEvent};
use std::sync::Arc;

fn render(inst: &Arc<piano_core::instrument::Instrument>, s: Shortcuts, events: &[(f64, Event)], secs: f64) -> Vec<f32> {
    let cfg = EngineConfig::default();
    let mut e = Engine::with_instrument(cfg, inst.clone());
    e.set_shortcuts(s);
    let bs = cfg.max_block;
    let n = (secs * cfg.sample_rate) as usize / bs;
    let mut out = vec![0.0f32; n * bs];
    let mut cur = 0;
    for (b, blk) in out.chunks_mut(bs).enumerate() {
        let mut ev = Vec::new();
        while cur < events.len() && ((events[cur].0 * cfg.sample_rate) as usize) < (b + 1) * bs {
            ev.push(TimedEvent { frame: ((events[cur].0 * cfg.sample_rate) as usize - b * bs) as u32, event: events[cur].1 });
            cur += 1;
        }
        e.process(&ev, blk);
    }
    out
}

/// (name, (time s, event), duration s)
type Scenario = (&'static str, Vec<(f64, Event)>, f64);

fn db(x: f64) -> f64 {
    10.0 * x.max(1e-30).log10()
}

fn main() {
    let inst = Arc::new(piano_core::Quality::Full.instrument(&Default::default(), 48_000.0));
    let on = |t, n, v| (t, Event::NoteOn { note: n, velocity: v });
    let ped = |t, d: bool| (t, Event::Control { cc: 64, value: if d { 127 } else { 0 } });
    let scenarios: Vec<Scenario> = vec![
        ("C2 ff held, pedal", vec![ped(0.0, true), on(0.05, 36, 120)], 6.0),
        ("C4 mf, released at 1 s", vec![on(0.05, 60, 80), (1.0, Event::NoteOff { note: 60, velocity: 64 })], 3.0),
        ("chord + pedal release", vec![ped(0.0, true), on(0.05, 36, 100), on(0.05, 48, 100), on(0.05, 55, 100), on(0.05, 64, 100), ped(2.0, false)], 4.0),
    ];
    let d = Shortcuts::DEFAULT;
    let variants: Vec<(&str, Shortcuts)> = vec![
        ("voice silence", Shortcuts { voice_silence: Shortcuts::NONE.voice_silence, ..d }),
        ("board silence", Shortcuts { board_silence: Shortcuts::NONE.board_silence, ..d }),
        ("sym silence", Shortcuts { sym_silence: Shortcuts::NONE.sym_silence, ..d }),
        ("nl silence", Shortcuts { nl_silence: Shortcuts::NONE.nl_silence, ..d }),
        ("deaf strings", Shortcuts { deaf_above: f64::INFINITY, listen_all: true, ..d }),
        ("drop partials", Shortcuts { drop_partials: 0.0, ..d }),
        ("all (Overkill's)", Shortcuts::NONE),
    ];
    for (name, ev, secs) in &scenarios {
        let base = render(&inst, d, ev, *secs);
        let p: f64 = base.iter().map(|x| (*x as f64).powi(2)).sum();
        println!("{name}");
        for (vn, s) in &variants {
            let alt = render(&inst, *s, ev, *secs);
            let e: f64 = base.iter().zip(&alt).map(|(a, b)| ((a - b) as f64).powi(2)).sum();
            // Worst 100 ms window, relative to the whole render's level.
            let w = 4800;
            let worst = base.chunks(w).zip(alt.chunks(w)).map(|(a, b)| a.iter().zip(b).map(|(x, y)| ((x - y) as f64).powi(2)).sum::<f64>() / w as f64).fold(0.0, f64::max);
            println!("  {vn:18} {:7.1} dB overall, worst 100 ms {:7.1} dB", db(e / p), db(worst / (p / base.len() as f64)));
        }
    }
}
