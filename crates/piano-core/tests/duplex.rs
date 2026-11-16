//! The front duplex in the engine (`frame.duplex`, docs/capo_duplex.md).
use piano_core::{Engine, EngineConfig, Event, TimedEvent};

fn render(cfg: EngineConfig, note: u8, secs: f64) -> Vec<f32> {
    let mut e = Engine::new(cfg);
    let bs = cfg.max_block;
    let mut out = vec![0.0f32; (secs * cfg.sample_rate) as usize / bs * bs];
    for (b, blk) in out.chunks_mut(bs).enumerate() {
        let ev = [TimedEvent { frame: 0, event: Event::NoteOn { note, velocity: 100 } }];
        e.process(if b == 0 { &ev } else { &[] }, blk);
    }
    out
}

fn rms(x: &[f32]) -> f64 {
    (x.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / x.len() as f64).sqrt()
}

/// The short lengths ring a little with a note under the capo bar: the note
/// changes, by little, and dies as before; a note below the bar keeps its
/// own strings (its halo of free top strings moves a little), and with the
/// bar alone every note is the same to the sample.
#[test]
fn the_duplex_rings_under_the_capo_and_nowhere_else() {
    let mut base = EngineConfig::default();
    base.model.room.enabled = false;
    base.model.noise = piano_core::params::NoiseLaw::OFF;
    let mut capo = base;
    capo.model.frame.capo_bar = true;
    let mut duplex = capo;
    duplex.model.frame.duplex = true;
    let same = |a: &[f32], b: &[f32], what: &str| assert!(a == b, "{what}: {:e} apart", rms(&a.iter().zip(b).map(|(x, y)| x - y).collect::<Vec<f32>>()));
    same(&render(base, 96, 0.3), &render(base, 96, 0.3), "one render twice");
    same(&render(base, 96, 0.3), &render(capo, 96, 0.3), "the bar alone");
    // A key below the bar: its own strings are the same; the free strings
    // of the top, which answer it, have moved by a few cents.
    let (a, b) = (render(capo, 60, 0.3), render(duplex, 60, 0.3));
    let diff: Vec<f32> = a.iter().zip(&b).map(|(x, y)| x - y).collect();
    assert!(rms(&diff) < 0.01 * rms(&a), "a key below the bar moves by {:.1} dB re the note", 20.0 * (rms(&diff) / rms(&a)).log10());
    let (a, b) = (render(capo, 96, 1.5), render(duplex, 96, 1.5));
    assert!(b.iter().all(|v| v.is_finite()));
    let diff: Vec<f32> = a.iter().zip(&b).map(|(x, y)| x - y).collect();
    assert!(rms(&diff) > 1e-4 * rms(&a), "the duplex changes nothing");
    let level = 20.0 * (rms(&b) / rms(&a)).log10();
    assert!(level.abs() < 1.5, "the note's level moves by {level} dB");
    let n = a.len();
    let late = 20.0 * (rms(&b[n - n / 5..]) / rms(&b[..n / 5])).log10();
    assert!(late < -6.0, "the note does not die: {late} dB");
}

fn played(cfg: EngineConfig, note: u8, hold: f64, secs: f64) -> Vec<f32> {
    let mut e = Engine::new(cfg);
    let bs = cfg.max_block;
    let off = (hold * cfg.sample_rate) as usize / bs;
    let mut out = vec![0.0f32; (secs * cfg.sample_rate) as usize / bs * bs];
    for (b, blk) in out.chunks_mut(bs).enumerate() {
        let on = [TimedEvent { frame: 0, event: Event::NoteOn { note, velocity: 100 } }];
        let up = [TimedEvent { frame: 0, event: Event::NoteOff { note, velocity: 64 } }];
        e.process(if b == 0 { &on[..] } else if b == off { &up[..] } else { &[] }, blk);
    }
    out
}

/// The rear lengths (`sympathetic.rear`) ring a little under a note, well
/// under it, and die after the key is up.
#[test]
fn the_rear_lengths_ring_faintly_and_die() {
    let mut base = EngineConfig::default();
    base.model.room.enabled = false;
    base.model.noise = piano_core::params::NoiseLaw::OFF;
    let mut rear = base;
    rear.model.sympathetic.rear = true;
    let (a, b) = (played(base, 72, 0.5, 3.0), played(rear, 72, 0.5, 3.0));
    assert!(b.iter().all(|v| v.is_finite()));
    let n = (0.5 * base.sample_rate) as usize;
    let diff: Vec<f32> = a.iter().zip(&b).map(|(x, y)| x - y).collect();
    let held = 20.0 * (rms(&diff[..n]) / rms(&a[..n])).log10();
    assert!(held > -70.0 && held < -8.0, "the rear lengths under the held note: {held} dB re the note");
    let late = 20.0 * (rms(&b[5 * n..]) / rms(&b[..n])).log10();
    assert!(late < -50.0, "two seconds after the key-up: {late} dB re the note");
}
