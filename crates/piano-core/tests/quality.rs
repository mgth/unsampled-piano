//! CPU quality levels: what each leaves out of the instrument, the preset
//! untouched.

use piano_core::params::PianoModel;
use piano_core::{Engine, EngineConfig, Quality};

#[test]
fn levels_reduce_the_model_only() {
    let m = PianoModel::default();
    assert_eq!(format!("{:?}", Quality::Full.apply(&m)), format!("{m:?}"));
    let b = Quality::Balanced.apply(&m);
    assert!(b.sympathetic.enabled && b.sympathetic.max_freq <= 2000.0 && b.nonlinear.max_note <= 48);
    let l = Quality::Light.apply(&m);
    assert!(!l.sympathetic.enabled && !l.nonlinear.enabled && !l.frame.enabled && !l.rim.enabled && !l.polarization.enabled);
    // Everything else as the preset says.
    assert_eq!((l.a4, l.hardness, l.room, l.damper), (m.a4, m.hardness, m.room, m.damper));
    assert_eq!(Quality::Balanced.polyphony(128), 96);
    assert_eq!(Quality::Light.polyphony(128), 128);
    for q in Quality::ALL {
        assert_eq!(Quality::from_name(q.name()), Some(q));
    }
}

#[test]
fn a_light_engine_plays() {
    let cfg = EngineConfig { quality: Quality::Light, ..Default::default() };
    let mut e = Engine::new(cfg);
    let (mut l, mut r) = (vec![0.0f32; 128], vec![0.0f32; 128]);
    let mut peak = 0.0f32;
    for i in 0..40 {
        let on = [piano_core::TimedEvent { frame: 0, event: piano_core::Event::NoteOn { note: 60, velocity: 90 } }];
        e.process_stereo(if i == 0 { &on } else { &[] }, &mut l, &mut r);
        peak = l.iter().fold(peak, |p, x| p.max(x.abs()));
    }
    assert!(peak > 1e-4 && peak.is_finite(), "{peak}");
}

/// Overkill keeps a listening string on every key, damped or not, with the
/// pedal up: every string hears the bridge. Full only on the strings whose
/// damper is lifted (the undamped treble and the keys played).
#[test]
fn overkill_listens_on_every_string() {
    let count = |quality| {
        let mut e = Engine::new(EngineConfig { quality, ..Default::default() });
        let (mut l, mut r) = (vec![0.0f32; 128], vec![0.0f32; 128]);
        let on = [piano_core::TimedEvent { frame: 0, event: piano_core::Event::NoteOn { note: 60, velocity: 90 } }];
        for i in 0..2 {
            e.process_stereo(if i == 0 { &on } else { &[] }, &mut l, &mut r);
        }
        e.active_voices()
    };
    assert_eq!(count(Quality::Overkill), 88);
    assert!(count(Quality::Full) < 88);
}

/// `sympathetic.bridge_cut`: the board's modes above `bridge_below` only
/// radiate. Off, the cut's frequency changes nothing; on, a pedalled chord
/// stays close to the whole model's, and is the same on one thread or two.
#[test]
fn the_bridge_cut_costs_little_and_nothing_when_off() {
    use piano_core::{Event, TimedEvent};
    let render = |model: PianoModel, threads: usize| {
        let mut e = Engine::new(EngineConfig { model, threads, ..Default::default() });
        let (mut l, mut r) = (vec![0.0f32; 128], vec![0.0f32; 128]);
        let mut on = vec![TimedEvent { frame: 0, event: Event::Control { cc: 64, value: 127 } }];
        on.extend([48u8, 60, 64, 67, 96].map(|note| TimedEvent { frame: 3, event: Event::NoteOn { note, velocity: 100 } }));
        let mut out = Vec::new();
        for i in 0..300 {
            e.process_stereo(if i == 0 { &on } else { &[] }, &mut l, &mut r);
            out.extend_from_slice(&l);
            out.extend_from_slice(&r);
        }
        out
    };
    let same = |a: &[f32], b: &[f32]| a.iter().zip(b).all(|(x, y)| x.to_bits() == y.to_bits());
    let whole = PianoModel::default();
    let mut off = whole;
    off.sympathetic.bridge_below = 6000.0;
    let mut cut = off;
    cut.sympathetic.bridge_cut = true;
    let a = render(whole, 1);
    assert!(same(&a, &render(off, 1)), "the cut's frequency alone changes the sound");
    let c = render(cut, 1);
    assert!(same(&c, &render(cut, 2)), "the cut depends on the threads");
    let power = |x: &mut dyn Iterator<Item = f64>| x.map(|v| v * v).sum::<f64>();
    let p = power(&mut a.iter().map(|&v| v as f64));
    let d = power(&mut a.iter().zip(&c).map(|(&x, &y)| (y - x) as f64));
    let db = 10.0 * (d / p).log10();
    assert!(d > 0.0 && db < -30.0, "difference {db:.1} dB re the signal");
}
