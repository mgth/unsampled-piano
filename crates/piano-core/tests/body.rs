//! The body's halo (crate::body) follows the events: a live change of its
//! law before a note in the same sub-block applies to that note, and the
//! halo never starts before the note's own frame.

use piano_core::{Engine, EngineConfig, Event, LiveParam, TimedEvent};

/// Only the strings and the body: no room, no limiter, no other noise.
fn config(gain: f64) -> EngineConfig {
    let mut cfg = EngineConfig::default();
    cfg.model.room.enabled = false;
    cfg.model.limiter = 0.0;
    cfg.model.noise = piano_core::params::NoiseLaw::OFF;
    cfg.model.body.enabled = true;
    cfg.model.body.gain = gain;
    cfg.model.body.delay = 0.0;
    cfg.model.body.levels = [0.0; 8];
    cfg
}

fn render(engine: &mut Engine, first: &[TimedEvent], frames: usize) -> (Vec<f32>, Vec<f32>) {
    let bs = engine.config().max_block;
    let (mut l, mut r) = (vec![0.0f32; frames], vec![0.0f32; frames]);
    for (i, (bl, br)) in l.chunks_mut(bs).zip(r.chunks_mut(bs)).enumerate() {
        let events: &[TimedEvent] = if i == 0 { first } else { &[] };
        engine.process_stereo(events, bl, br);
    }
    (l, r)
}

fn max_diff(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| (x - y).abs()).fold(0.0, f32::max)
}

#[test]
fn a_live_gain_before_the_note_applies_to_it() {
    let mut a = Engine::new(config(0.0));
    let mut b = Engine::new(config(0.25));
    let note = TimedEvent { frame: 0, event: Event::NoteOn { note: 48, velocity: 100 } };
    let gain = TimedEvent { frame: 0, event: Event::Param { id: LiveParam::BodyGain, value: 0.25 } };
    let (la, ra) = render(&mut a, &[gain, note], 9600);
    let (lb, rb) = render(&mut b, &[note], 9600);
    assert!(max_diff(&la, &lb) < 1e-6 && max_diff(&ra, &rb) < 1e-6, "{} {}", max_diff(&la, &lb), max_diff(&ra, &rb));
}

#[test]
fn the_halo_waits_for_the_note_frame() {
    let mut with = Engine::new(config(0.25));
    let mut without = Engine::new(config(0.0));
    let note = [TimedEvent { frame: 48, event: Event::NoteOn { note: 48, velocity: 100 } }];
    let (lw, rw) = render(&mut with, &note, 4800);
    let (lo, ro) = render(&mut without, &note, 4800);
    // Before the note: the body adds nothing.
    assert_eq!(max_diff(&lw[..48], &lo[..48]), 0.0);
    assert_eq!(max_diff(&rw[..48], &ro[..48]), 0.0);
    // After it, it does.
    assert!(max_diff(&lw[48..], &lo[48..]) > 1e-4);
}
