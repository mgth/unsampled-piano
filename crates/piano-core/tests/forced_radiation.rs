//! `soundboard.forced_radiation` in the engine (docs/forced_radiation.md):
//! the correction goes to the microphones alone, not to the room's source.

use piano_core::{Engine, EngineConfig, Event, TimedEvent};

/// A small built board (as crate::forced's tests), the strings, the board
/// and the room: no limiter, no noise, no body.
fn config(forced: bool, room: bool) -> EngineConfig {
    let mut cfg = EngineConfig::default();
    let structure = piano_core::fem::BoardStructure { enabled: true, step: 0.04, below: 400.0, ..piano_core::fem::BoardStructure::STEINWAY_D };
    cfg.model.soundboard = Some(piano_core::soundboard::SoundboardParams {
        a: 2.124,
        b: 1.46,
        rho_h: 14.7,
        crown_freq: 0.0,
        f_max: 400.0,
        structure,
        grid_step: 0.08,
        case_field: false,
        forced_radiation: forced,
        ..piano_core::soundboard::SoundboardParams::GRAND_2M
    });
    cfg.model.room.enabled = room;
    cfg.model.limiter = 0.0;
    cfg.model.noise = piano_core::params::NoiseLaw::OFF;
    cfg.model.body.enabled = false;
    cfg
}

fn render(cfg: EngineConfig, frames: usize) -> Vec<f32> {
    let mut engine = Engine::new(cfg);
    let bs = engine.config().max_block;
    let (mut l, mut r) = (vec![0.0f32; frames], vec![0.0f32; frames]);
    for (i, (bl, br)) in l.chunks_mut(bs).zip(r.chunks_mut(bs)).enumerate() {
        let note = [TimedEvent { frame: 0, event: Event::NoteOn { note: 60, velocity: 85 } }];
        engine.process_stereo(if i == 0 { &note } else { &[] }, bl, br);
    }
    l.extend(r);
    l
}

fn peak(a: &[f32]) -> f32 {
    a.iter().fold(0.0, |m, x| m.max(x.abs()))
}

fn minus(a: &[f32], b: &[f32]) -> Vec<f32> {
    a.iter().zip(b).map(|(x, y)| x - y).collect()
}

/// What the room adds (the render with the room less the render without)
/// is the same with the option on and off.
#[test]
fn the_room_does_not_hear_the_correction() {
    let frames = 24_000;
    let [on_room, on_dry, off_room, off_dry] = [(true, true), (true, false), (false, true), (false, false)].map(|(f, r)| render(config(f, r), frames));
    // The option does something, and the room too.
    let correction = peak(&minus(&on_dry, &off_dry));
    let room = peak(&minus(&off_room, &off_dry));
    assert!(correction > 1e-4 && room > 1e-4, "correction {correction}, room {room}");
    let gap = peak(&minus(&minus(&on_room, &on_dry), &minus(&off_room, &off_dry)));
    assert!(gap < 1e-5 * peak(&off_room).max(1e-3), "the room's part moves by {gap} with the option (correction {correction}, room {room})");
}
