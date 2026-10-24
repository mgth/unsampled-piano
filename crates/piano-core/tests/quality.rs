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
