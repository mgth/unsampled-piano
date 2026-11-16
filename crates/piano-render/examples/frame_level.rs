//! What the frame radiates under a key's note, in a preset: the difference
//! between the note rendered with the frame and without it (dry, no room,
//! no noises, velocity 100 unless VEL is set), re the note without it, in
//! three windows, 0-0.2 s, 0.5-1.5 s and 2-4 s (analysis,
//! docs/capo_duplex.md). The difference holds what the frame radiates and
//! what its admittance changes in the strings; `frame.coupling` 0 leaves
//! the radiation alone (COUPLING=0).
//!   cargo run --release -p piano-render --example frame_level -- <preset.toml> [key …]
use piano_core::params::NoiseLaw;
use piano_core::{Engine, EngineConfig, Event, TimedEvent};

fn render(cfg: EngineConfig, note: u8, velocity: u8, dur: f64) -> Vec<f32> {
    let mut e = Engine::new(cfg);
    let bs = cfg.max_block;
    let n = (dur * cfg.sample_rate) as usize / bs;
    let mut out = vec![0.0f32; n * bs];
    for (b, blk) in out.chunks_mut(bs).enumerate() {
        let ev = [TimedEvent { frame: 0, event: Event::NoteOn { note, velocity } }];
        e.process(if b == 0 { &ev } else { &[] }, blk);
    }
    out
}

fn db(x: &[f32]) -> f64 {
    10.0 * (x.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / x.len() as f64).max(1e-30).log10()
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let mut base = EngineConfig::default();
    base.model = piano_host::load_preset(std::path::Path::new(&a[0])).unwrap();
    base.model.room.enabled = false;
    base.model.noise = NoiseLaw::OFF;
    if let Some(c) = std::env::var("COUPLING").ok().and_then(|v| v.parse().ok()) {
        base.model.frame.coupling = c;
    }
    let vel = std::env::var("VEL").ok().and_then(|v| v.parse().ok()).unwrap_or(100u8);
    let sr = base.sample_rate;
    let mut none = base;
    none.model.frame.enabled = false;
    let keys: Vec<u8> = a[1..].iter().filter_map(|k| k.parse().ok()).collect();
    let keys = if keys.is_empty() { (72..=108).step_by(2).collect() } else { keys };
    println!("key,early,mid,late");
    for note in keys {
        let x = render(none, note, vel, 4.0);
        let d: Vec<f32> = render(base, note, vel, 4.0).iter().zip(&x).map(|(p, q)| p - q).collect();
        let w = |s: f64, e: f64| {
            let r = (s * sr) as usize..((e * sr) as usize).min(x.len());
            db(&d[r.clone()]) - db(&x[r])
        };
        println!("{note},{:.1},{:.1},{:.1}", w(0.0, 0.2), w(0.5, 1.5), w(2.0, 4.0));
    }
}
