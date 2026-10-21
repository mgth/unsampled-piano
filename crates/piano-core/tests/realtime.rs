//! The audio path must not allocate, must be deterministic and must stay
//! bounded under abuse (all keys, all velocities, fast repetitions, pedal).

use assert_no_alloc::{assert_no_alloc, AllocDisabler};
use piano_core::{Engine, EngineConfig, Event, TimedEvent};

#[global_allocator]
static A: AllocDisabler = AllocDisabler;

fn torture(engine: &mut Engine, blocks: usize) -> Vec<f32> {
    let bs = engine.config().max_block;
    let mut out = vec![0.0f32; bs * blocks];
    let mut events = Vec::with_capacity(16);
    for (i, blk) in out.chunks_mut(bs).enumerate() {
        events.clear();
        let note = 21 + (i * 7 % 88) as u8;
        let velocity = 1 + (i * 37 % 127) as u8;
        events.push(TimedEvent { frame: (i % bs) as u32, event: Event::NoteOn { note, velocity } });
        if i % 3 == 0 {
            events.push(TimedEvent { frame: (bs - 1) as u32, event: Event::NoteOff { note: note.saturating_sub(14).max(21), velocity: 40 } });
        }
        if i % 50 == 0 {
            events.push(TimedEvent { frame: 0, event: Event::Control { cc: 64, value: if i % 100 == 0 { 127 } else { 0 } } });
            events.sort_by_key(|e| e.frame);
        }
        assert_no_alloc(|| engine.process(&events, blk));
    }
    out
}

#[test]
fn no_allocation_bounded_and_deterministic() {
    let cfg = EngineConfig::default();
    let a = torture(&mut Engine::new(cfg), 1500);
    let b = torture(&mut Engine::new(cfg), 1500);
    assert!(a.iter().zip(&b).all(|(x, y)| x.to_bits() == y.to_bits()), "non-deterministic render");
    let peak = a.iter().fold(0.0f32, |m, v| m.max(v.abs()));
    assert!(peak.is_finite() && peak < 50.0, "peak {peak}");
}
