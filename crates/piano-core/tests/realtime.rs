//! The audio path must not allocate, must be deterministic and must stay
//! bounded under abuse (all keys, all velocities, fast repetitions, pedal).

use assert_no_alloc::{assert_no_alloc, AllocDisabler};
use piano_core::{Engine, EngineConfig, Event, LiveParam, TimedEvent};

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
        if i % 40 == 20 {
            let id = LiveParam::ALL[i / 40 % LiveParam::ALL.len()];
            let value = id.get(&engine.instrument().model) as f32 * if i % 80 == 20 { 0.5 } else { 1.0 };
            events.push(TimedEvent { frame: 3, event: Event::Param { id, value } });
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

/// Every key at several velocities, with re-strikes during contact and while
/// ringing: output stays finite and bounded.
#[test]
fn whole_keyboard_restrikes_stay_bounded() {
    let mut engine = Engine::new(EngineConfig { polyphony: 96, ..Default::default() });
    let bs = engine.config().max_block;
    let mut out = vec![0.0f32; bs];
    let mut peak = 0.0f32;
    for note in 21u8..=108 {
        for (j, &velocity) in [1u8, 40, 80, 127].iter().enumerate() {
            // Two strikes 1 ms apart (second one lands during contact), then
            // one 20 ms later on the ringing string.
            let ev = [
                TimedEvent { frame: 0, event: Event::NoteOn { note, velocity } },
                TimedEvent { frame: 48, event: Event::NoteOn { note, velocity: 127 - velocity / 2 } },
            ];
            engine.process(&ev, &mut out);
            for _ in 0..8 {
                engine.process(&[], &mut out);
                peak = out.iter().fold(peak, |m, v| m.max(v.abs()));
            }
            engine.process(&[TimedEvent { frame: 5, event: Event::NoteOn { note, velocity } }], &mut out);
            if j == 3 {
                engine.process(&[TimedEvent { frame: 0, event: Event::NoteOff { note, velocity: 64 } }], &mut out);
            }
            assert!(out.iter().all(|v| v.is_finite()), "note {note} vel {velocity}");
        }
    }
    assert!(peak < 50.0, "peak {peak}");
}
