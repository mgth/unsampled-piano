//! The key's return (`noise.key_return`): a released key falls back on its
//! rail and the hammer on its rest, a thud after the key-up whose level
//! does not follow the note's (docs/action_noise.md).

use piano_core::{Engine, EngineConfig, Event, TimedEvent};

const SR: usize = 48_000;
/// A top key, without a damper: its release changes nothing else.
const KEY: u8 = 108;
const UP: usize = SR / 2;

/// Only the strings and the action's knock: no room, no limiter, no body.
fn config(key_return: f64) -> EngineConfig {
    let mut cfg = EngineConfig::default();
    cfg.model.room.enabled = false;
    cfg.model.limiter = 0.0;
    cfg.model.body.enabled = false;
    cfg.model.noise = piano_core::params::NoiseLaw::OFF;
    cfg.model.noise.action = 0.02;
    cfg.model.noise.key_return = key_return;
    cfg
}

fn at(frame: usize, event: Event) -> TimedEvent {
    TimedEvent { frame: frame as u32, event }
}

fn render(cfg: EngineConfig, events: &[TimedEvent], frames: usize) -> Vec<f32> {
    let bs = cfg.max_block;
    let mut engine = Engine::new(cfg);
    let (mut l, mut r) = (vec![0.0f32; frames], vec![0.0f32; frames]);
    for (i, (bl, br)) in l.chunks_mut(bs).zip(r.chunks_mut(bs)).enumerate() {
        let (a, b) = (i * bs, i * bs + bl.len());
        let ev: Vec<TimedEvent> =
            events.iter().filter(|e| (a..b).contains(&(e.frame as usize))).map(|e| TimedEvent { frame: e.frame - a as u32, event: e.event }).collect();
        engine.process_stereo(&ev, bl, br);
    }
    l
}

/// What the return adds: the take with it minus the take without, same
/// events. First sample that differs, and the peak of the difference.
fn added(events: &[TimedEvent], key_return: f64) -> (Option<usize>, f32) {
    let frames = SR;
    let (with, without) = (render(config(key_return), events, frames), render(config(0.0), events, frames));
    let d: Vec<f32> = with.iter().zip(&without).map(|(a, b)| a - b).collect();
    (d.iter().position(|x| *x != 0.0), d.iter().fold(0.0f32, |m, x| m.max(x.abs())))
}

fn note(velocity: u8, release: u8) -> [TimedEvent; 2] {
    [at(0, Event::NoteOn { note: KEY, velocity }), at(UP, Event::NoteOff { note: KEY, velocity: release })]
}

#[test]
fn the_thud_comes_after_the_key_up_by_its_delay() {
    let delay = (config(0.2).model.noise.key_return_delay * SR as f64) as usize;
    let (first, peak) = added(&note(40, 64), 0.2);
    let first = first.expect("no thud");
    assert!(first >= UP + delay && first < UP + delay + 64, "thud at {first}, key-up at {UP}, delay {delay}");
    assert!(peak > 0.0);
}

#[test]
fn its_level_does_not_follow_the_notes() {
    let (_, soft) = added(&note(15, 64), 0.2);
    let (_, loud) = added(&note(110, 64), 0.2);
    assert!((soft - loud).abs() < 1e-3 * soft, "soft {soft}, loud {loud}");
}

#[test]
fn a_faster_release_lands_sooner_and_harder() {
    let (slow_at, slow) = added(&note(40, 64), 0.2);
    let (fast_at, fast) = added(&note(40, 127), 0.2);
    assert!(fast_at.unwrap() < slow_at.unwrap());
    assert!(fast > 1.5 * slow, "fast {fast}, slow {slow}");
}

#[test]
fn a_note_on_at_velocity_zero_is_a_release_too() {
    let events = [at(0, Event::NoteOn { note: KEY, velocity: 40 }), at(UP, Event::NoteOn { note: KEY, velocity: 0 })];
    assert_eq!(added(&events, 0.2), added(&note(40, 64), 0.2));
}

#[test]
fn a_key_struck_again_before_it_is_back_makes_no_thud() {
    // Released, then struck again 20 ms later and held: the key never came
    // back to rest.
    let again = UP + SR / 50;
    let events = [note(40, 64)[0], note(40, 64)[1], at(again, Event::NoteOn { note: KEY, velocity: 40 })];
    assert_eq!(added(&events, 0.2), (None, 0.0));
}
