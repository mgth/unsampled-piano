//! Live parameters reach the sound without rebuilding the instrument.

use piano_core::{Engine, EngineConfig, Event, LiveParam, TimedEvent};

fn rms(x: &[f32]) -> f64 {
    (x.iter().map(|&v| (v as f64) * (v as f64)).sum::<f64>() / x.len() as f64).sqrt()
}

/// Renders a held note; `gain_db` (if any) is sent as a live change at 0.5 s.
fn render(engine: &mut Engine, gain_db: Option<f32>) -> Vec<f32> {
    let bs = engine.config().max_block;
    let sr = engine.config().sample_rate as usize;
    let mut out = vec![0.0f32; sr];
    for (i, blk) in out.chunks_mut(bs).enumerate() {
        let mut events = Vec::new();
        if i == 0 {
            events.push(TimedEvent { frame: 0, event: Event::NoteOn { note: 60, velocity: 90 } });
        }
        if let Some(value) = gain_db.filter(|_| i == sr / 2 / bs) {
            events.push(TimedEvent { frame: 0, event: Event::Param { id: LiveParam::OutputGainDb, value } });
        }
        engine.process(&events, blk);
    }
    out
}

#[test]
fn output_gain_is_live_and_smooth() {
    let cfg = EngineConfig::default();
    let mut a = Engine::new(cfg);
    let mut b = Engine::with_instrument(cfg, a.instrument().clone());
    let dry = render(&mut a, None);
    let wet = render(&mut b, Some(-20.0));
    let sr = cfg.sample_rate as usize;
    // Untouched before the change.
    assert!(dry[..sr / 2].iter().zip(&wet[..sr / 2]).all(|(x, y)| x == y));
    // −20 dB once the 20 ms smoothing has settled (5 time constants)…
    let tail = sr / 2 + sr / 10;
    let ratio = rms(&wet[tail..]) / rms(&dry[tail..]);
    assert!((ratio - 0.1).abs() < 0.002, "ratio {ratio}");
    // …and no step: the first samples after the change are still near 0 dB.
    let head = sr / 2..sr / 2 + 64;
    let r0 = rms(&wet[head.clone()]) / rms(&dry[head]);
    assert!(r0 > 0.9, "gain jumped: {r0}");
}
