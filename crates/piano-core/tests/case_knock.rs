//! The case's knock (crate::case) follows the events: a live change of its
//! resonances applies from its own frame, whatever the host's blocks, and
//! the knock never starts before the note's own frame.

use piano_core::{Engine, EngineConfig, Event, LiveParam, TimedEvent};

/// Only the strings and the case's knock: no room, no limiter, no body, no
/// other noise.
fn config(case: f64) -> EngineConfig {
    let mut cfg = EngineConfig::default();
    cfg.model.room.enabled = false;
    cfg.model.limiter = 0.0;
    cfg.model.body.enabled = false;
    cfg.model.noise = piano_core::params::NoiseLaw::OFF;
    cfg.model.noise.case = case;
    cfg
}

/// Renders `frames` samples in blocks of `bs`, each event at its frame.
fn render(engine: &mut Engine, events: &[TimedEvent], frames: usize, bs: usize) -> (Vec<f32>, Vec<f32>) {
    let (mut l, mut r) = (vec![0.0f32; frames], vec![0.0f32; frames]);
    for (i, (bl, br)) in l.chunks_mut(bs).zip(r.chunks_mut(bs)).enumerate() {
        let (a, b) = (i * bs, i * bs + bl.len());
        let ev: Vec<TimedEvent> = events
            .iter()
            .filter(|e| (a..b).contains(&(e.frame as usize)))
            .map(|e| TimedEvent { frame: e.frame - a as u32, event: e.event })
            .collect();
        engine.process_stereo(&ev, bl, br);
    }
    (l, r)
}

fn max_diff(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| (x - y).abs()).fold(0.0, f32::max)
}

const NOTE: TimedEvent = TimedEvent { frame: 0, event: Event::NoteOn { note: 96, velocity: 100 } };
const LOW: TimedEvent = TimedEvent { frame: 48, event: Event::Param { id: LiveParam::NoiseCaseLow, value: 140.0 } };

#[test]
fn a_live_change_of_the_resonances_applies_from_its_own_frame() {
    let bs = EngineConfig::default().max_block;
    let mut changed = Engine::new(config(0.05));
    let mut kept = Engine::new(config(0.05));
    let (lc, rc) = render(&mut changed, &[NOTE, LOW], 4800, bs);
    let (lk, rk) = render(&mut kept, &[NOTE], 4800, bs);
    // Up to the change, the bank rang with its old resonances.
    assert_eq!(max_diff(&lc[..48], &lk[..48]), 0.0);
    assert_eq!(max_diff(&rc[..48], &rk[..48]), 0.0);
    // From it on, with the new ones.
    assert!(max_diff(&lc[48..], &lk[48..]) > 1e-5);
}

#[test]
fn the_host_blocks_do_not_move_the_knock() {
    let bs = EngineConfig::default().max_block;
    // The knock alone: the same engine with and without it, each rendered
    // in the host's blocks and in blocks split at the change's frame.
    let knock = |split: usize| {
        let mut with = Engine::new(config(0.05));
        let mut without = Engine::new(config(0.0));
        let (lw, _) = render(&mut with, &[NOTE, LOW], 4800, split);
        let (lo, _) = render(&mut without, &[NOTE, LOW], 4800, split);
        lw.iter().zip(&lo).map(|(a, b)| a - b).collect::<Vec<f32>>()
    };
    let (whole, split) = (knock(bs), knock(48));
    let peak = whole.iter().fold(0.0f32, |m, x| m.max(x.abs()));
    assert!(peak > 1e-4, "{peak}");
    assert!(max_diff(&whole, &split) < 1e-4 * peak, "{} of {peak}", max_diff(&whole, &split));
}

#[test]
fn the_knock_waits_for_the_note_frame() {
    let note = [TimedEvent { frame: 48, ..NOTE }];
    let bs = EngineConfig::default().max_block;
    let (lw, rw) = render(&mut Engine::new(config(0.05)), &note, 4800, bs);
    let (lo, ro) = render(&mut Engine::new(config(0.0)), &note, 4800, bs);
    assert_eq!(max_diff(&lw[..48], &lo[..48]), 0.0);
    assert_eq!(max_diff(&rw[..48], &ro[..48]), 0.0);
    assert!(max_diff(&lw[48..], &lo[48..]) > 1e-5);
}
