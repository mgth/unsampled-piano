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

/// A change of the halo's rise while a strike settles in leaves that strike
/// as it began (to zero or to another length): the next ones take it.
#[test]
fn a_live_rise_change_leaves_a_settling_strike_alone() {
    let mut cfg = config(0.25);
    cfg.model.body.rise = 0.1;
    let note = TimedEvent { frame: 0, event: Event::NoteOn { note: 48, velocity: 100 } };
    let run = |to: Option<f32>| {
        let mut e = Engine::new(cfg);
        let bs = e.config().max_block;
        let frames = 9600;
        let (mut l, mut r) = (vec![0.0f32; frames], vec![0.0f32; frames]);
        for (i, (bl, br)) in l.chunks_mut(bs).zip(r.chunks_mut(bs)).enumerate() {
            let mut ev = Vec::new();
            if i == 0 {
                ev.push(note);
            }
            // 64 frames into the 0.1 s ramp (blocks of 64 or more: the
            // second block starts at frame ≥ 64).
            if let (1, Some(v)) = (i, to) {
                ev.push(TimedEvent { frame: 0, event: Event::Param { id: LiveParam::BodyRise, value: v } });
            }
            e.process_stereo(&ev, bl, br);
        }
        (l, r)
    };
    let (lk, rk) = run(None);
    for to in [0.0, 0.03] {
        let (l, r) = run(Some(to));
        assert!(l.iter().chain(&r).all(|x| x.is_finite()));
        assert!(max_diff(&l, &lk) < 1e-6 && max_diff(&r, &rk) < 1e-6, "rise → {to}: {} {}", max_diff(&l, &lk), max_diff(&r, &rk));
    }
}

/// More strikes in one sub-block than the bank's queue holds, with free
/// slots all along: 16 note-ons at each of frames 0, 14, 28 and 42 (12-frame
/// pulses, at most 16 slots in use), then one more at frame 56. The last
/// note's halo is there.
#[test]
fn every_strike_of_a_crowded_sub_block_rings() {
    let notes = |last: bool| {
        let mut ev: Vec<TimedEvent> = (0..64u8).map(|k| TimedEvent { frame: 14 * (k as u32 / 16), event: Event::NoteOn { note: 21 + k, velocity: 100 } }).collect();
        if last {
            ev.push(TimedEvent { frame: 56, event: Event::NoteOn { note: 100, velocity: 100 } });
        }
        ev
    };
    let run = |gain: f64, last: bool| {
        let mut e = Engine::new(config(gain));
        let ev = notes(last);
        let (mut l, mut r) = (vec![0.0f32; 64], vec![0.0f32; 64]);
        e.process_stereo(&ev, &mut l, &mut r);
        // A few more blocks for the halo to speak.
        let mut tail = l;
        for _ in 0..20 {
            let (mut a, mut b) = (vec![0.0f32; 64], vec![0.0f32; 64]);
            e.process_stereo(&[], &mut a, &mut b);
            tail.extend_from_slice(&a);
        }
        tail
    };
    // The last note's halo alone: with and without it, minus the same
    // without the halo (its strings).
    let halo: Vec<f32> = run(0.25, true).iter().zip(run(0.25, false)).zip(run(0.0, true).iter().zip(run(0.0, false))).map(|((a, b), (c, d))| (a - b) - (c - d)).collect();
    let peak = halo.iter().fold(0.0f32, |m, x| m.max(x.abs()));
    assert!(peak > 1e-2, "the last note's halo peaks at {peak}");
}
