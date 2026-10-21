//! Peak level per 0.5 s of notes under sympathy variants: a stability check
//! of the sympathetic coupling (nothing may grow).
use piano_core::{Engine, EngineConfig, Event, TimedEvent};

fn run(label: &str, cfg: EngineConfig, note: u8, pedal: bool) {
    let mut e = Engine::new(cfg);
    let bs = cfg.max_block;
    let mut out = vec![0.0f32; bs];
    let mut line = format!("{label:28}");
    let blocks_per = (0.5 * cfg.sample_rate) as usize / bs;
    for seg in 0..10 {
        let mut peak = 0.0f32;
        for b in 0..blocks_per {

            let mut ev = Vec::new();
            if seg == 0 && b == 0 && pedal {
                ev.push(TimedEvent { frame: 0, event: Event::Control { cc: 64, value: 127 } });
            }
            if seg == 0 && b == 20 && note > 0 {
                ev.push(TimedEvent { frame: 0, event: Event::NoteOn { note, velocity: 100 } });
            }
            e.process(&ev, &mut out);
            peak = out.iter().fold(peak, |m, v| m.max(v.abs()));
        }
        line += &format!(" {:7.1}", 20.0 * peak.max(1e-30).log10());
    }
    println!("{line}  (voices {})", e.active_voices());
    if label.contains("pedal") && label.contains("on") {
        let mut v = e.voice_energies();
        v.sort_by(|a, b| b.1.total_cmp(&a.1));
        println!("   top energies: {:?}", &v[..5]);
    }
}

fn main() {
    let base = EngineConfig::default();
    run("sympathy on, C4", base, 60, false);
    run("sympathy on, C4, pedal", base, 60, true);
    let mut off = base;
    off.model.sympathetic.enabled = false;
    run("sympathy off, C4, pedal", off, 60, true);
    run("sympathy on, C7 (no damper)", base, 96, false);
    let mut solo = base;
    solo.model.damper.highest_note = 108;
    run("only the struck string", solo, 60, false);
    run("only the struck A0", solo, 21, false);
    run("only the struck C7", solo, 96, false);
    run("all strings free (pedal), C4", solo, 60, true);
    let mut few = solo;
    few.model.damper.highest_note = 107;
    run("C4 + one free string (C8)", few, 60, false);
    few.model.damper.highest_note = 102;
    run("C4 + free F#7..C8", few, 60, false);
    run("no strike, free F#7..C8", few, 0, false);
    run("only the struck C8", solo, 108, false);
    run("only the struck F#7", solo, 102, false);
    run("only the struck C6", solo, 84, false);
}
