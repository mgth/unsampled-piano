//! The bridge rocking (`soundboard.bridge_rocking`) carries the strings'
//! sideways motion to the built board by a path of its own: it changes
//! nothing without that motion, and is heard with it.

use assert_no_alloc::{assert_no_alloc, AllocDisabler};
use piano_core::{Engine, EngineConfig, Event, TimedEvent};

#[global_allocator]
static A: AllocDisabler = AllocDisabler;

/// A coarse built board (2 cm elements, modes below 600 Hz), each key
/// driving its low modes at its own bridge point; no room, no noise.
fn config(rocking: bool, sideways: bool) -> EngineConfig {
    let mut cfg = EngineConfig::default();
    cfg.model.room.enabled = false;
    cfg.model.limiter = 0.0;
    cfg.model.noise = piano_core::params::NoiseLaw::OFF;
    cfg.model.polarization.enabled = sideways;
    let sb = cfg.model.soundboard.as_mut().unwrap();
    sb.structure.enabled = true;
    sb.structure.step = 0.02;
    sb.structure.below = 600.0;
    sb.exact_below = 600.0;
    sb.bridge_rocking = rocking;
    cfg
}

/// A bass key and a tenor key, struck together, for 0.25 s.
fn render(cfg: EngineConfig) -> (Vec<f32>, Vec<f32>) {
    let mut engine = Engine::new(cfg);
    let bs = engine.config().max_block;
    let frames = (0.25 * engine.config().sample_rate) as usize / bs * bs;
    let (mut l, mut r) = (vec![0.0f32; frames], vec![0.0f32; frames]);
    let first = [
        TimedEvent { frame: 0, event: Event::NoteOn { note: 33, velocity: 100 } },
        TimedEvent { frame: 0, event: Event::NoteOn { note: 52, velocity: 90 } },
    ];
    for (i, (bl, br)) in l.chunks_mut(bs).zip(r.chunks_mut(bs)).enumerate() {
        let events: &[TimedEvent] = if i == 0 { &first } else { &[] };
        // The rocking's room is made when the engine is built: none here.
        assert_no_alloc(|| engine.process_stereo(events, bl, br));
    }
    (l, r)
}

fn same(a: &[f32], b: &[f32]) -> bool {
    a.iter().zip(b).all(|(x, y)| x.to_bits() == y.to_bits())
}

#[test]
fn without_sideways_motion_the_rocking_changes_nothing() {
    let (l0, r0) = render(config(false, false));
    let (l1, r1) = render(config(true, false));
    assert!(l0.iter().any(|x| *x != 0.0));
    assert!(same(&l0, &l1) && same(&r0, &r1), "the rocking moved a board that only the vertical force drives");
}

#[test]
fn with_sideways_motion_the_rocking_is_heard() {
    let (l0, r0) = render(config(false, true));
    let (l1, r1) = render(config(true, true));
    let rms = |x: &[f32]| (x.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / x.len() as f64).sqrt();
    let diff: Vec<f32> = l0.iter().zip(&l1).chain(r0.iter().zip(&r1)).map(|(a, b)| a - b).collect();
    let rel = rms(&diff) / rms(&[l0, r0].concat());
    // Heard, as a part of the sound, not a new sound on top of it.
    assert!(rel > 1e-3 && rel < 0.5, "rocking's share of the sound: {rel}");
}
