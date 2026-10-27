//! A keyboard reaching the engine through two MIDI paths sends every event
//! twice, sometimes a few samples apart: the second note-on must not strike
//! the string again (2 ms apart it made a soft note harsh and metallic).
//! Nor can a key strike again while its hammer still presses the string.

use piano_core::{Engine, EngineConfig, Event, TimedEvent};

fn render(script: &[(usize, Event)], samples: usize) -> (Vec<f32>, u32) {
    let cfg = EngineConfig::default();
    let mut e = Engine::new(cfg);
    let bs = cfg.max_block;
    let mut out = vec![0.0f32; samples.next_multiple_of(bs)];
    for (b, blk) in out.chunks_mut(bs).enumerate() {
        let ev: Vec<TimedEvent> =
            script.iter().filter(|(t, _)| t / bs == b).map(|&(t, event)| TimedEvent { frame: (t % bs) as u32, event }).collect();
        e.process(&ev, blk);
    }
    (out, e.doubled_notes())
}

/// Largest difference over the peak.
fn rel_diff(a: &[f32], b: &[f32]) -> f32 {
    let peak = a.iter().fold(0.0f32, |m, x| m.max(x.abs()));
    a.iter().zip(b).fold(0.0f32, |m, (x, y)| m.max((x - y).abs())) / peak
}

const ON: Event = Event::NoteOn { note: 52, velocity: 56 };
const OFF: Event = Event::NoteOff { note: 52, velocity: 64 };
const N: usize = 48_000 / 2;

#[test]
fn a_doubled_note_on_strikes_once() {
    let (single, none) = render(&[(1000, ON), (15_000, OFF)], N);
    assert_eq!(none, 0);
    for gap in [1, 96, 500] {
        let (double, doubled) = render(&[(1000, ON), (1000 + gap, ON), (15_000, OFF), (15_000 + gap, OFF)], N);
        // The ignored event still splits the block it falls in: the sums
        // differ in their last bits, not more.
        let d = rel_diff(&single, &double);
        assert!(d < 1e-4, "a second note-on {gap} samples later changed the sound by {d:e} of the peak");
        assert_eq!(doubled, 1, "the double is counted");
    }
    // A real repetition (released, struck again 0.1 s later) still strikes.
    let (repeated, doubled) = render(&[(1000, ON), (3000, OFF), (5800, ON)], N);
    assert!(rel_diff(&single, &repeated) > 1e-2);
    assert_eq!(doubled, 0);
}

#[test]
fn no_strike_while_the_hammer_presses_the_string() {
    let (single, _) = render(&[(1000, ON), (15_000, OFF)], N);
    // Released 1 ms after, struck again 2 ms after: the hammer still lies on
    // the strings. Not a double (a note-off came between), not played.
    let (again, doubled) = render(&[(1000, ON), (1048, OFF), (1096, ON), (15_000, OFF)], N);
    assert_eq!(doubled, 0);
    let d = rel_diff(&single, &again);
    assert!(d < 1e-2, "a strike during the hammer's contact changed the sound by {d:e} of the peak");
}
