//! What the case's heights change in the sound: the lid's hinge
//! (`soundboard.lid_hinge_height`) and the rim's top and height
//! (`rim.top`, `rim.height`), one at a time then together, at the
//! listening presets' microphones. A few notes rendered with the full
//! engine (room on), per octave band, variant minus the old heights
//! ([`OLD`]), dB (docs/lid.md). The variant's heights are the model's
//! defaults, or those given.
//!   cargo run --release -p piano-core --example case_heights [hinge top height]
use piano_core::params::PianoModel;
use piano_core::soundboard::SoundboardParams;
use piano_core::structure::RimParams;
use piano_core::{Engine, EngineConfig, Event, Quality, TimedEvent};
use std::f64::consts::PI;
use std::sync::Arc;

const SR: f64 = 48_000.0;

/// The reference: the lid's hinge, the rim's top and height (m) before the
/// case took a real grand's (docs/lid.md, "The case's heights").
const OLD: (f64, f64, f64) = (0.25, 0.05, 0.30);

fn render(model: &PianoModel, events: &[(f64, Event)], secs: f64) -> Vec<f32> {
    let cfg = EngineConfig { model: *model, ..EngineConfig::default() };
    let inst = Arc::new(Quality::Full.instrument(model, SR));
    let mut e = Engine::with_instrument(cfg, inst);
    let bs = cfg.max_block;
    let n = (secs * SR) as usize / bs;
    let mut out = vec![0.0f32; n * bs];
    let mut cur = 0;
    for (b, blk) in out.chunks_mut(bs).enumerate() {
        let mut ev = Vec::new();
        while cur < events.len() && ((events[cur].0 * SR) as usize) < (b + 1) * bs {
            ev.push(TimedEvent { frame: ((events[cur].0 * SR) as usize - b * bs) as u32, event: events[cur].1 });
            cur += 1;
        }
        e.process(&ev, blk);
    }
    out
}

/// Power of the (mono) signal in the octave band around `fc`: two RBJ
/// band-pass sections in cascade.
fn band(x: &[f32], fc: f64) -> f64 {
    let w = 2.0 * PI * fc / SR;
    let q = 2f64.sqrt() / (2.0 - 1.0);
    let al = w.sin() / (2.0 * q);
    let a0 = 1.0 + al;
    let (b0, b2, a1, a2) = (al / a0, -al / a0, -2.0 * w.cos() / a0, (1.0 - al) / a0);
    let mut e = 0.0;
    let mut s = [[0.0f64; 4]; 2];
    for &v in x {
        let mut y = v as f64;
        for st in &mut s {
            let o = b0 * y + b2 * st[1] - a1 * st[2] - a2 * st[3];
            (st[1], st[0], st[3], st[2]) = (st[0], y, st[2], o);
            y = o;
        }
        e += y * y;
    }
    e
}

fn main() {
    let args: Vec<f64> = std::env::args().skip(1).filter_map(|a| a.parse().ok()).collect();
    let new = PianoModel::default();
    let (hinge, top, height) = (
        args.first().copied().unwrap_or(new.soundboard.unwrap().lid_hinge_height),
        args.get(1).copied().unwrap_or(new.rim.top),
        args.get(2).copied().unwrap_or(new.rim.height),
    );
    // The default model at the old heights.
    let base = PianoModel { rim: RimParams { top: OLD.1, height: OLD.2, ..new.rim }, ..new };
    let sb0 = SoundboardParams { lid_hinge_height: OLD.0, ..new.soundboard.unwrap() };
    // The listening presets' microphones (presets/listening/Microphones).
    let rel = |mics, angle, shut, gain| SoundboardParams { mics, mics_relative: true, lid_angle: angle, lid_shut: shut, mic_gain_db: gain, ..sb0 };
    let presets: [(&str, SoundboardParams); 5] = [
        ("default (lid off, 1.5 m)", sb0),
        ("close, lid 35°", SoundboardParams { mic_pattern: 0.5, ..rel([(0.294118, 0.346154, 0.4), (0.705882, 0.653846, 0.4)], 35.0, 0.0, -9.6) }),
        ("concert, lid 35°", rel([(0.323529, -1.230769, 2.0), (0.735294, -1.230769, 2.0)], 35.0, 0.0, 3.4)),
        ("player, lid 35°", rel([(-0.558824, 0.569231, 0.45), (-0.558824, 0.430769, 0.45)], 35.0, 0.0, -6.1)),
        ("audience, lid shut", rel([(0.323529, -1.230769, 1.0), (0.735294, -1.230769, 1.0)], 0.0, 1.0, 0.0)),
    ];
    let on = |t, n, v| (t, Event::NoteOn { note: n, velocity: v });
    let off = |t, n| (t, Event::NoteOff { note: n, velocity: 64 });
    // C2, C4, C6 mf held 1.5 s, then a ff chord.
    let events = vec![on(0.05, 36, 80), off(1.6, 36), on(2.0, 60, 80), off(3.5, 60), on(4.0, 84, 80), off(5.5, 84), on(6.0, 36, 115), on(6.0, 48, 115), on(6.0, 55, 115), on(6.0, 64, 115), on(6.0, 72, 115)];
    let secs = 9.0;
    let bands: Vec<f64> = (0..8).map(|i| 62.5 * 2f64.powi(i)).collect();
    println!("hinge {} → {hinge} m, rim top {} → {top} m, rim height {} → {height} m; variant − old (dB)", OLD.0, OLD.1, OLD.2);
    println!("{:26} {:14} {:>6} {}", "microphones", "variant", "all", bands.iter().map(|f| format!("{:>6.0}", f)).collect::<String>());
    for (name, sb) in presets {
        let old = PianoModel { soundboard: Some(sb), ..base };
        let x0 = render(&old, &events, secs);
        let p0: Vec<f64> = bands.iter().map(|&f| band(&x0, f)).collect();
        let all0: f64 = x0.iter().map(|v| (*v as f64).powi(2)).sum();
        let variants = [
            ("hinge", PianoModel { soundboard: Some(SoundboardParams { lid_hinge_height: hinge, ..sb }), ..base }),
            ("rim", PianoModel { rim: RimParams { top, height, ..base.rim }, ..old }),
            ("both", PianoModel { soundboard: Some(SoundboardParams { lid_hinge_height: hinge, ..sb }), rim: RimParams { top, height, ..base.rim }, ..base }),
        ];
        for (vn, m) in variants {
            let x = render(&m, &events, secs);
            let all: f64 = x.iter().map(|v| (*v as f64).powi(2)).sum();
            let row: String = bands.iter().zip(&p0).map(|(&f, &p)| format!("{:>+6.1}", 10.0 * (band(&x, f) / p).log10())).collect();
            println!("{name:26} {vn:14} {:>+6.1} {row}", 10.0 * (all / all0).log10());
        }
    }
}
