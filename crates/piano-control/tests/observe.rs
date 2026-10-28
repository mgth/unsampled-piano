//! The interface's live view: what the audio side publishes of the engine
//! reaches the controller (keys, dampers, strings' energy, board modes,
//! pedals), and the publishing allocates nothing on the audio thread.

use piano_core::params::PianoModel;
use piano_core::{EngineConfig, Event, TimedEvent};
use piano_params::PianoControl;

use assert_no_alloc::{assert_no_alloc, AllocDisabler};

#[global_allocator]
static A: AllocDisabler = AllocDisabler;

const BS: usize = 256;

#[test]
fn a_struck_key_shows_in_the_observation() {
    let cfg = EngineConfig { max_block: BS, ..Default::default() };
    let (control, mut slot) = piano_control::new(PianoModel::default(), cfg);
    assert!(control.observe().is_none(), "nothing published before the first blocks");
    let on = [
        TimedEvent { frame: 0, event: Event::NoteOn { note: 60, velocity: 90 } },
        TimedEvent { frame: 0, event: Event::Control { cc: 67, value: 127 } },
    ];
    let (mut l, mut r) = ([0.0f32; BS], [0.0f32; BS]);
    // 0.1 s with the key held, audio thread checked for allocations.
    for b in 0..(4_800 / BS) {
        assert_no_alloc(|| slot.process_stereo(if b == 0 { &on } else { &[] }, &mut l, &mut r));
    }
    let o = control.observe().expect("published");
    let k = o.keys[60];
    assert!(k.down && k.energy > 0.0 && k.damper < 0.5, "{k:?}");
    assert!(!o.keys[62].down && o.keys[62].energy == 0.0 && o.keys[62].damper == 1.0, "{:?}", o.keys[62]);
    assert_eq!(o.soft, 1.0);
    assert!(o.board_modes > 0 && o.board[..o.board_modes].iter().any(|&a| a > 0.0), "board {:?}", &o.board[..8]);

    // Released: the key up, its damper back on the strings.
    let off = [TimedEvent { frame: 0, event: Event::NoteOff { note: 60, velocity: 64 } }];
    for b in 0..(24_000 / BS) {
        slot.process_stereo(if b == 0 { &off } else { &[] }, &mut l, &mut r);
    }
    let k = control.observe().unwrap().keys[60];
    assert!(!k.down && k.damper > 0.9, "{k:?}");
}
