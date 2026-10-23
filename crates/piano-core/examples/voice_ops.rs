//! Cost of the per-event voice operations that run on the audio thread
//! outside the sample loops: setting up a string (note-on on a free voice,
//! a listening string added when the dampers lift), striking a listening
//! string, and moving its damper (one decay-rate update per sub-block).
use piano_core::instrument::Instrument;
use piano_core::params::PianoModel;
use piano_core::voice::Voice;
use std::time::Instant;

fn main() {
    let sr = 48_000.0;
    let inst = Instrument::new(PianoModel::default(), sr);
    let mut v = Voice::new(sr, 128);
    let reps = 2000;
    println!("{:>5} {:>6} {:>10} {:>10} {:>12} {:>10}", "note", "modes", "setup µs", "listen µs", "→struck µs", "damper µs");
    for note in [21u8, 33, 45, 60, 72, 84, 96, 108] {
        let nm = inst.table.get(note);
        let time = |f: &mut dyn FnMut(&mut Voice)| {
            let mut v2 = Voice::new(sr, 128);
            let t = Instant::now();
            for _ in 0..reps {
                f(&mut v2);
            }
            1e6 * t.elapsed().as_secs_f64() / reps as f64
        };
        let setup = time(&mut |v| v.setup(nm, false));
        let listen = time(&mut |v| v.setup(nm, true));
        // A listening string struck: its primary bank comes alive.
        let struck = time(&mut |v| {
            v.setup(nm, true);
            v.bank.set_active_len(nm.modes.len());
        }) - listen;
        v.setup(nm, false);
        let mut k = 0;
        let damper = time(&mut |_| {
            k += 1;
            v.set_damper_rate(1.0 + (k % 7) as f64);
        });
        println!("{note:>5} {:>6} {setup:>10.1} {listen:>10.1} {struck:>12.1} {damper:>10.1}", nm.modes.len());
    }
}
