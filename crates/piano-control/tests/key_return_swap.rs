//! The key's return across a rebuild: a key released on an engine that is
//! then replaced, and struck again on the new one before its return has
//! landed, makes no thud. The strike goes to the new engine only; the old
//! one, which holds the return, learns that the key went down.

use piano_core::params::{NoiseLaw, PianoModel};
use piano_core::{Engine, EngineConfig, Event, TimedEvent};

const BS: usize = 256;
const SR: usize = 48_000;
/// A top key, without a damper.
const KEY: u8 = 108;

fn cfg(key_return: f64) -> EngineConfig {
    let mut cfg = EngineConfig { max_block: BS, ..Default::default() };
    cfg.model.room.enabled = false;
    cfg.model.limiter = 0.0;
    cfg.model.body.enabled = false;
    cfg.model.noise = NoiseLaw::OFF;
    cfg.model.noise.action = 0.02;
    cfg.model.noise.key_return = key_return;
    cfg
}

/// The key struck, released at 0.1 s, the engine replaced 8 ms later, and
/// the key struck again 20 ms after its release if `again`: 0.4 s, left.
fn render(key_return: f64, again: bool) -> Vec<f32> {
    let cfg = cfg(key_return);
    let model: PianoModel = cfg.model;
    let (_ctl, mut slot) = piano_control::new(model, cfg);
    let up = SR / 10;
    let mut events = vec![(0, Event::NoteOn { note: KEY, velocity: 40 }), (up, Event::NoteOff { note: KEY, velocity: 64 })];
    if again {
        events.push((up + SR / 50, Event::NoteOn { note: KEY, velocity: 40 }));
    }
    let swap = (up + SR * 8 / 1000) / BS;
    let (mut left, mut right) = (vec![0.0f32; SR * 4 / 10 / BS * BS], vec![0.0f32; SR * 4 / 10 / BS * BS]);
    for (i, (l, r)) in left.chunks_mut(BS).zip(right.chunks_mut(BS)).enumerate() {
        if i == swap {
            slot.inject(Box::new(Engine::new(cfg)));
        }
        let ev: Vec<TimedEvent> =
            events.iter().filter(|e| e.0 / BS == i).map(|&(f, event)| TimedEvent { frame: (f % BS) as u32, event }).collect();
        slot.process_stereo(&ev, l, r);
    }
    left
}

fn differ(a: &[f32], b: &[f32]) -> bool {
    a.iter().zip(b).any(|(x, y)| x != y)
}

#[test]
fn a_key_struck_again_on_the_new_engine_stops_the_old_ones_return() {
    // Left alone, the replaced engine lands the return of the key it saw
    // released.
    assert!(differ(&render(0.2, false), &render(0.0, false)), "no thud from the replaced engine");
    assert!(!differ(&render(0.2, true), &render(0.0, true)), "the replaced engine's thud under the new note");
}
