//! Level of the frame and rim contributions, isolated as the difference
//! between renders with and without them (dry, no room, no noises).
use piano_core::params::NoiseLaw;
use piano_core::soundboard::Soundboard;
use piano_core::structure::RimModel;
use piano_core::{Engine, EngineConfig, Event, TimedEvent};

fn render(cfg: EngineConfig, script: &[(f64, Event)], dur: f64) -> Vec<f32> {
    let mut e = Engine::new(cfg);
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

fn rms_db(x: &[f32]) -> f64 {
    10.0 * (x.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / x.len() as f64).max(1e-30).log10()
}

fn main() {
    let sr = 48_000.0;
    let mut base = EngineConfig::default();
    base.model.room.enabled = false;
    base.model.noise = NoiseLaw::OFF;
    // FRAME_COUPLING=c: the frame's two-way coupling (1 by default).
    if let Some(c) = std::env::var("FRAME_COUPLING").ok().and_then(|v| v.parse().ok()) {
        base.model.frame.coupling = c;
    }
    // FRAME_LOSS=η: the frame's loss factor.
    if let Some(v) = std::env::var("FRAME_LOSS").ok().and_then(|v| v.parse().ok()) {
        base.model.frame.loss_factor = v;
    }

    let sb = Soundboard::new(base.model.soundboard.unwrap());
    let rim = RimModel::new(&base.model.rim, &sb, sb.modes.len(), sr);
    println!("rim: {} modes, {} projections, {} board modes coupled", rim.modes.len(), rim.projections.len(), rim.couple.len());
    for m in rim.modes.iter().filter(|m| m.orient == 0) {
        println!("  k {:2}  {:7.1} Hz  W {:8.4}  B {:8.4}", m.k, m.freq_hz, m.w, m.b);
    }
    let frame = piano_core::structure::frame_modes(&base.model.frame, &base.model.soundboard.unwrap(), sr);
    println!("frame: {} modes, first {:.1} Hz", frame.len(), frame[0].freq_hz);

    let mut none = base;
    none.model.frame.enabled = false;
    none.model.rim.enabled = false;
    let mut fr = none;
    fr.model.frame.enabled = true;
    let mut rm = none;
    rm.model.rim.enabled = true;

    let w = |x: &[f32], a: f64, b: f64| rms_db(&x[(a * sr) as usize..((b * sr) as usize).min(x.len())]);
    let diff = |a: &[f32], b: &[f32]| a.iter().zip(b).map(|(x, y)| x - y).collect::<Vec<f32>>();
    println!("\nlevels re the note without frame/rim, windows 0–0.2 s | 0.5–1.5 s | 2–4 s");
    // Keys given on the command line, else six across the keyboard.
    let keys: Vec<u8> = std::env::args().skip(1).filter_map(|a| a.parse().ok()).collect();
    let keys = if keys.is_empty() { vec![28, 45, 60, 72, 84, 96] } else { keys };
    for (note, vel) in keys.into_iter().map(|k| (k, 100u8)) {
        let script = [(0.0, Event::NoteOn { note, velocity: vel })];
        let a = render(none, &script, 4.0);
        let f = diff(&render(fr, &script, 4.0), &a);
        let r = diff(&render(rm, &script, 4.0), &a);
        let wins = [(0.0, 0.2), (0.5, 1.5), (2.0, 4.0)];
        let fmt = |d: &[f32]| wins.iter().map(|&(s, e)| format!("{:+6.1}", w(d, s, e) - w(&a, s, e))).collect::<Vec<_>>().join(" ");
        println!("note {note:3}: frame {}   rim {}", fmt(&f), fmt(&r));
    }
}
