//! The musician's controls that move live parameters only (touch, release,
//! mechanism): moving one while playing sounds exactly as an instrument
//! rebuilt with the same value, and moving it twice does not compound.

use piano_core::instrument::Instrument;
use piano_core::params::PianoModel;
use piano_core::{Engine, EngineConfig, Event, LiveParam, TimedEvent};
use std::sync::Arc;

fn render(model: PianoModel, script: &[(f64, Event)], dur: f64) -> Vec<f32> {
    let cfg = EngineConfig::default();
    let mut e = Engine::with_instrument(cfg, Arc::new(Instrument::new(model, cfg.sample_rate)));
    let bs = cfg.max_block;
    let n = (dur * cfg.sample_rate) as usize / bs;
    let mut out = vec![0.0f32; n * bs];
    for (b, blk) in out.chunks_mut(bs).enumerate() {
        let ev: Vec<TimedEvent> = script
            .iter()
            .filter_map(|&(t, ev)| {
                let f = (t * cfg.sample_rate) as usize;
                (f / bs == b).then_some(TimedEvent { frame: (f % bs) as u32, event: ev })
            })
            .collect();
        e.process(&ev, blk);
    }
    out
}

/// A key played loud then soft, released, and the pedal pressed and lifted:
/// it touches the velocity law, the dampers and every mechanical noise.
fn notes() -> Vec<(f64, Event)> {
    vec![
        (0.02, Event::NoteOn { note: 60, velocity: 110 }),
        (0.30, Event::NoteOff { note: 60, velocity: 64 }),
        (0.40, Event::NoteOn { note: 67, velocity: 30 }),
        (0.50, Event::Control { cc: 64, value: 127 }),
        (0.70, Event::NoteOff { note: 67, velocity: 64 }),
        (0.90, Event::Control { cc: 64, value: 0 }),
    ]
}

#[test]
fn live_controls_match_a_rebuild() {
    for (id, set) in [
        (LiveParam::MacroTouch, (|m: &mut PianoModel, v| m.macros.touch = v) as fn(&mut PianoModel, f64)),
        (LiveParam::MacroRelease, |m, v| m.macros.release = v),
        (LiveParam::MacroMechanism, |m, v| m.macros.mechanism = v),
    ] {
        let v = 0.6;
        let mut rebuilt = PianoModel::default();
        set(&mut rebuilt, v);
        let a = render(rebuilt, &notes(), 1.4);
        // Live, moved once to another value first: no compounding.
        let mut script = vec![(0.0, Event::Param { id, value: -0.8 }), (0.0, Event::Param { id, value: v as f32 })];
        script.extend(notes());
        let b = render(PianoModel::default(), &script, 1.4);
        let peak = a.iter().fold(0.0f32, |p, x| p.max(x.abs()));
        let diff = a.iter().zip(&b).fold(0.0f32, |d, (x, y)| d.max((x - y).abs()));
        assert!(diff <= 1e-5 * peak, "{id:?}: live and rebuilt differ by {diff:e} (peak {peak:e})");
        // And the control does change the sound.
        let base = render(PianoModel::default(), &notes(), 1.4);
        let moved = base.iter().zip(&b).fold(0.0f32, |d, (x, y)| d.max((x - y).abs()));
        assert!(moved > 1e-3 * peak, "{id:?} changes nothing");
    }
}
