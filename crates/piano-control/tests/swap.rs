//! Background rebuilds: the audio side swaps engines without allocating,
//! held notes go on unchanged, replaced engines are freed by the worker,
//! and a burst of edits makes a single build.

use std::sync::Arc;
use std::time::{Duration, Instant};

use assert_no_alloc::{assert_no_alloc, AllocDisabler};
use piano_control::EngineSlot;
use piano_core::params::PianoModel;
use piano_core::{Engine, EngineConfig, Event, TimedEvent};
use piano_params::{PianoControl, Value};

#[global_allocator]
static A: AllocDisabler = AllocDisabler;

const BS: usize = 256;

fn cfg() -> EngineConfig {
    EngineConfig { max_block: BS, ..Default::default() }
}

fn block(slot: &mut EngineSlot, events: &[TimedEvent]) -> ([f32; BS], [f32; BS]) {
    let (mut l, mut r) = ([0.0f32; BS], [0.0f32; BS]);
    assert_no_alloc(|| slot.process_stereo(events, &mut l, &mut r));
    (l, r)
}

fn wait_until(mut f: impl FnMut() -> bool) {
    let t = Instant::now();
    while !f() {
        assert!(t.elapsed() < Duration::from_secs(20), "timed out");
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn swap_keeps_held_notes_and_never_allocates() {
    swap_keeps_held_notes(cfg());
}

/// With a pool, the replaced engine and the new one run side by side on two
/// threads: the same output, sample for sample.
#[test]
fn swap_on_two_threads_keeps_held_notes() {
    swap_keeps_held_notes(EngineConfig { threads: 2, ..cfg() });
}

fn swap_keeps_held_notes(cfg: EngineConfig) {
    let (mut ctl, mut slot) = piano_control::new(PianoModel::default(), cfg);
    let old = Arc::downgrade(slot.current().instrument());
    let mut reference = Engine::with_instrument(EngineConfig { threads: 1, ..cfg }, slot.current().instrument().clone());
    let on = [TimedEvent { frame: 0, event: Event::NoteOn { note: 60, velocity: 100 } }];
    let (mut rl, mut rr) = ([0.0f32; BS], [0.0f32; BS]);
    let mut n = 0;
    let mut swapped_at = None;
    // Until 1 s after the swap the old engine plays alone: the sum must be
    // the reference, sample for sample.
    while swapped_at.is_none_or(|s| n < s + 48_000 / BS) {
        let events: &[TimedEvent] = if n == 0 { &on } else { &[] };
        let (l, r) = block(&mut slot, events);
        reference.process_stereo(events, &mut rl, &mut rr);
        let err = l.iter().zip(&rl).chain(r.iter().zip(&rr)).fold(0.0f32, |m, (a, b)| m.max((a - b).abs()));
        assert!(err < 1e-7, "block {n}: discontinuity {err}");
        if n == 20 {
            ctl.set("hardness", Value::Float(0.9));
        }
        if swapped_at.is_none() && slot.engines() == 2 {
            swapped_at = Some(n);
        }
        if swapped_at.is_none() {
            std::thread::sleep(Duration::from_millis(2));
        }
        n += 1;
    }
    assert!(rl.iter().any(|&x| x != 0.0), "the held note went silent");
    assert_eq!(slot.current().instrument().model.hardness, 0.9);
    assert!(!ctl.status().rebuilding);
    // New notes go to the new engine only.
    assert_eq!(slot.current().sounding_voices(), 0);
    block(&mut slot, &[TimedEvent { frame: 0, event: Event::NoteOn { note: 72, velocity: 90 } }]);
    // (with the undamped treble strings it sets ringing)
    assert!(slot.current().sounding_voices() >= 1);
    // The old engine fades out after RETIRE_AFTER_S and goes back to the
    // worker, which frees it (and its instrument, once the reference goes).
    let t = Instant::now();
    while slot.engines() > 1 {
        assert!(t.elapsed() < Duration::from_secs(30));
        block(&mut slot, &[]);
    }
    drop(reference);
    wait_until(|| old.upgrade().is_none());
    assert_eq!(ctl.shared().builds.load(std::sync::atomic::Ordering::Relaxed), 1);
}

#[test]
fn a_burst_of_edits_builds_once() {
    let (mut ctl, mut slot) = piano_control::new(PianoModel::default(), cfg());
    for i in 0..10 {
        ctl.set("hardness", Value::Float(0.5 + 0.05 * i as f64));
        block(&mut slot, &[]);
        std::thread::sleep(Duration::from_millis(20));
    }
    wait_until(|| !ctl.status().rebuilding);
    block(&mut slot, &[]);
    assert_eq!(ctl.shared().builds.load(std::sync::atomic::Ordering::Relaxed), 1);
    assert!((slot.current().instrument().model.hardness - 0.95).abs() < 1e-12);
    assert!(ctl.status().last_build.is_some());
}

#[test]
fn live_edits_do_not_rebuild() {
    let (mut ctl, mut slot) = piano_control::new(PianoModel::default(), cfg());
    ctl.set("output_gain_db", Value::Float(-12.0));
    assert!(!ctl.status().rebuilding);
    block(&mut slot, &[]);
    std::thread::sleep(DEBOUNCE_AND_MORE);
    assert_eq!(ctl.shared().builds.load(std::sync::atomic::Ordering::Relaxed), 0);
    assert_eq!(ctl.model().output_gain_db, -12.0);
}

const DEBOUNCE_AND_MORE: Duration = Duration::from_millis(400);

#[test]
fn midi_learn_routes_a_controller() {
    let (mut ctl, mut slot) = piano_control::new(PianoModel::default(), cfg());
    ctl.midi().unwrap().learn("output_gain_db");
    let cc = |value| [TimedEvent { frame: 0, event: Event::Control { cc: 20, value } }];
    // Learnt once moved over part of its travel (MidiLearn::LEARN_TRAVEL).
    block(&mut slot, &cc(0));
    block(&mut slot, &cc(40));
    ctl.poll();
    assert_eq!(ctl.midi().unwrap().learning(), None);
    let before = ctl.model().output_gain_db;
    // The route table reaches the audio side; CC 20 now drives the gain.
    block(&mut slot, &cc(127));
    ctl.poll();
    assert_ne!(ctl.model().output_gain_db, before);
    std::thread::sleep(DEBOUNCE_AND_MORE);
    assert_eq!(ctl.shared().builds.load(std::sync::atomic::Ordering::Relaxed), 0);
}
