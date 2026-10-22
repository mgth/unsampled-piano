//! How much the sympathetic resonance adds, pedal down: one note, a chord,
//! and the energy of each listening string after 1 s (primary bank vs
//! sympathetic bank), dry.
use piano_core::{Engine, EngineConfig, Event, TimedEvent};

fn render(cfg: EngineConfig, notes: &[u8], dur: f64) -> (Vec<f32>, Engine) {
    let mut e = Engine::new(cfg);
    let bs = cfg.max_block;
    let mut out = vec![0.0f32; bs];
    let mut all = Vec::new();
    for b in 0..(dur * cfg.sample_rate) as usize / bs {
        let mut ev = Vec::new();
        if b == 0 {
            ev.push(TimedEvent { frame: 0, event: Event::Control { cc: 64, value: 127 } });
        }
        if b == 10 {
            for &n in notes {
                ev.push(TimedEvent { frame: 0, event: Event::NoteOn { note: n, velocity: 80 } });
            }
        }
        e.process(&ev, &mut out);
        all.extend_from_slice(&out);
    }
    (all, e)
}

fn main() {
    let gain: f64 = std::env::args().nth(1).map_or(1.0, |g| g.parse().unwrap());
    let mut cfg = EngineConfig::default();
    cfg.model.sympathetic.gain = gain;
    println!("sympathetic gain {gain}");
    cfg.model.room.enabled = false;
    cfg.model.noise = piano_core::params::NoiseLaw::OFF;
    let mut off = cfg;
    off.model.sympathetic.enabled = false;
    let db = |x: &[f32]| 10.0 * (x.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / x.len() as f64).log10();
    for (label, notes) in [("C4", &[60u8][..]), ("C2", &[36][..]), ("C major chord C3-E3-G3-C4", &[48, 52, 55, 60][..])] {
        let (a, e) = render(cfg, notes, 3.0);
        let (b, _) = render(off, notes, 3.0);
        let sr = cfg.sample_rate;
        let w = |x: &[f32], s: f64, t: f64| db(&x[(s * sr) as usize..(t * sr) as usize]);
        println!(
            "{label:28} with − without sympathy: 0–0.5 s {:+.1} dB, 0.5–1.5 s {:+.1} dB, 1.5–3 s {:+.1} dB",
            w(&a, 0.0, 0.5) - w(&b, 0.0, 0.5),
            w(&a, 0.5, 1.5) - w(&b, 0.5, 1.5),
            w(&a, 1.5, 3.0) - w(&b, 1.5, 3.0)
        );
        let mut v = e.voice_energies_split();
        v.sort_by(|x, y| y.2.total_cmp(&x.2));
        println!("   strongest sympathetic banks at 3 s (note, primary, sympathetic): {:?}", &v[..4]);
    }
}
