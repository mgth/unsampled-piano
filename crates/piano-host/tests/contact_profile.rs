//! An explicit per-key preset must play exactly like the isolated bench presets.
use piano_core::{Engine, EngineConfig, Event, TimedEvent, params::{PianoModel, TrebleContact}};

fn render(model: PianoModel, note: u8, velocity: u8) -> Vec<[u32; 2]> {
    let mut engine = Engine::new(EngineConfig { model, ..Default::default() });
    let mut audio = Vec::new();
    for block in 0..25 {
        let (mut l, mut r) = ([0.0; 128], [0.0; 128]);
        let events = [TimedEvent { frame: 0, event: Event::NoteOn { note, velocity } }];
        engine.process_stereo(if block == 0 { &events } else { &[] }, &mut l, &mut r);
        audio.extend(l.into_iter().zip(r).map(|(l, r)| [l.to_bits(), r.to_bits()]));
    }
    audio
}

#[test]
fn contact_profile_is_absent_by_default_and_roundtrips() {
    let model = PianoModel::default();
    let text = piano_host::preset_toml(&model);
    assert!(!text.contains("treble_contact"));
    let decoded: PianoModel = toml::from_str(&text).unwrap();
    assert!(decoded.treble_contact.is_none());
    let mut profile = model;
    profile.treble_contact = Some([TrebleContact::from_model(&model); 16]);
    profile.treble_contact.as_mut().unwrap()[9].strike_ratio = 0.105;
    let decoded: PianoModel = toml::from_str(&piano_host::preset_toml(&profile)).unwrap();
    assert_eq!(decoded.contact_at(102).unwrap().strike_ratio, 0.105);
    assert!(decoded.contact_at(92).is_none());
    assert!(decoded.contact_at(109).is_none());
}

#[test]
fn per_key_contact_matches_isolated_preset_including_string_heights() {
    let base = PianoModel::default();
    let mut profile = base;
    let mut contacts = [TrebleContact::from_model(&base); 16];
    for note in [97, 99, 102, 108] {
        contacts[note - 93] = TrebleContact {
            hardness: 0.17, p_offset: -0.7, hysteresis: 0.2,
            contact_width_scale: 1.1, height_spread: 0.00004, strike_ratio: 0.105,
        };
    }
    profile.treble_contact = Some(contacts);
    for note in [92, 93, 97, 99, 102, 108] {
        let contact = profile.contact_at(note).copied().unwrap_or_else(|| TrebleContact::from_model(&base));
        let mut isolated = base;
        isolated.hardness = contact.hardness;
        isolated.p_offset = contact.p_offset;
        isolated.hysteresis = contact.hysteresis;
        isolated.contact_width_scale = contact.contact_width_scale;
        isolated.unison.height_spread = contact.height_spread;
        isolated.strike_ratio = contact.strike_ratio;
        for velocity in [45, 100] {
            assert_eq!(render(profile, note, velocity), render(isolated, note, velocity), "note {note}, velocity {velocity}");
        }
    }
}

#[test]
fn tone_also_moves_the_explicit_treble_contact() {
    let base = PianoModel::default();
    let mut profile = base;
    profile.treble_contact = Some([TrebleContact::from_model(&base); 16]);
    profile.macros.tone = 0.5;
    let voiced = profile.with_macros();
    let mut global = base;
    global.macros.tone = 0.5;
    let want = global.with_macros();
    assert_eq!(voiced.note(102).hammer.q0, want.note(102).hammer.q0);
    assert_eq!(voiced.note(102).hammer.core_q, want.note(102).hammer.core_q);
    assert_eq!(voiced.with_macros().note(102).hammer.q0, voiced.note(102).hammer.q0);
}
