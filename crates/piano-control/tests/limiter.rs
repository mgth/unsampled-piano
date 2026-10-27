//! The output stage: without the limiter a loud chord goes over full scale
//! and the status counts it; with it nothing leaves above the ceiling, the
//! status shows the gain taken off, and it switches live.

use piano_core::params::PianoModel;
use piano_core::{EngineConfig, Event, LiveParam, TimedEvent};
use piano_params::PianoControl;

const BS: usize = 256;

/// A fortissimo treble chord, 20 dB hot, for 0.5 s; the output's peak.
fn play(limiter: f64) -> (f32, piano_params::Status) {
    let model = PianoModel { output_gain_db: 20.0, limiter, ..PianoModel::default() };
    let cfg = EngineConfig { max_block: BS, ..Default::default() };
    let (control, mut slot) = piano_control::new(model, cfg);
    let chord: Vec<TimedEvent> =
        [72, 76, 79, 84, 88, 91].iter().map(|&note| TimedEvent { frame: 0, event: Event::NoteOn { note, velocity: 120 } }).collect();
    let mut peak = 0.0f32;
    for b in 0..(24_000 / BS) {
        let (mut l, mut r) = ([0.0f32; BS], [0.0f32; BS]);
        slot.process_stereo(if b == 0 { &chord } else { &[] }, &mut l, &mut r);
        peak = l.iter().chain(&r).fold(peak, |m, x| m.max(x.abs()));
    }
    (peak, control.status())
}

#[test]
fn the_limiter_keeps_the_output_under_its_ceiling() {
    let (peak, st) = play(0.0);
    assert!(peak > 1.0 && st.clipped > 0, "a hot chord should clip: peak {peak}, {} clipped", st.clipped);
    assert_eq!(st.limiting_db, 0.0);
    let (peak, st) = play(1.0);
    let ceiling = 10f32.powf(-1.0 / 20.0);
    assert!(peak <= ceiling * (1.0 + 1e-6), "peak {peak}");
    assert_eq!(st.clipped, 0);
    assert!(st.limiting_db > 3.0, "{}", st.limiting_db);
}

#[test]
fn the_limiter_switches_live() {
    let model = PianoModel { output_gain_db: 20.0, ..PianoModel::default() };
    let cfg = EngineConfig { max_block: BS, ..Default::default() };
    let (_control, mut slot) = piano_control::new(model, cfg);
    let mut events = vec![TimedEvent { frame: 0, event: Event::Param { id: LiveParam::Limiter, value: 1.0 } }];
    events.extend([72, 79, 84, 88].iter().map(|&note| TimedEvent { frame: 1, event: Event::NoteOn { note, velocity: 120 } }));
    let mut peak = 0.0f32;
    for b in 0..(24_000 / BS) {
        let (mut l, mut r) = ([0.0f32; BS], [0.0f32; BS]);
        slot.process_stereo(if b == 0 { &events } else { &[] }, &mut l, &mut r);
        peak = l.iter().chain(&r).fold(peak, |m, x| m.max(x.abs()));
    }
    assert!(peak <= 10f32.powf(-1.0 / 20.0) * (1.0 + 1e-6), "peak {peak}");
}
