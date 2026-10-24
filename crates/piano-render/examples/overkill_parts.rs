//! Which of Overkill's differences from Full changes the sound, on a MIDI
//! file: Full with one of them at a time, compared with Full and with
//! Overkill (overall and worst 100 ms difference, dB below the signal).
//!   cargo run --release -p piano-render --example overkill_parts -- file.mid [seconds]
use piano_core::engine::Shortcuts;
use piano_core::instrument::Instrument;
use piano_core::note_model::Resolution;
use piano_core::params::PianoModel;
use piano_core::{Engine, EngineConfig, Quality, TimedEvent};
use std::sync::Arc;

fn render(inst: Instrument, quality: Quality, shortcuts: Shortcuts, tl: &[(f64, piano_core::Event)], secs: f64) -> Vec<f32> {
    let cfg = EngineConfig { quality, ..Default::default() };
    let mut e = Engine::with_instrument(cfg, Arc::new(inst));
    e.set_shortcuts(shortcuts);
    let (sr, bs) = (cfg.sample_rate, cfg.max_block);
    let n = (secs * sr) as usize / bs;
    let mut out = vec![0.0f32; n * bs];
    let mut cur = 0;
    for (b, blk) in out.chunks_mut(bs).enumerate() {
        let mut ev = Vec::new();
        while cur < tl.len() && ((tl[cur].0 * sr) as usize) < (b + 1) * bs {
            ev.push(TimedEvent { frame: ((tl[cur].0 * sr) as usize).saturating_sub(b * bs) as u32, event: tl[cur].1 });
            cur += 1;
        }
        e.process(&ev, blk);
    }
    out
}

fn diff(a: &[f32], b: &[f32]) -> (f64, f64) {
    let p: f64 = a.iter().map(|x| (*x as f64).powi(2)).sum::<f64>() / a.len() as f64;
    let e: f64 = a.iter().zip(b).map(|(x, y)| ((x - y) as f64).powi(2)).sum::<f64>() / a.len() as f64;
    let w = 4800;
    let worst = a.chunks(w).zip(b.chunks(w)).map(|(x, y)| x.iter().zip(y).map(|(u, v)| ((u - v) as f64).powi(2)).sum::<f64>() / w as f64).fold(0.0, f64::max);
    (10.0 * (e / p).max(1e-30).log10(), 10.0 * (worst / p).max(1e-30).log10())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let secs: f64 = args.get(2).map_or(60.0, |s| s.parse().unwrap());
    let tl: Vec<_> = piano_host::read_midi_file(std::path::Path::new(&args[1])).unwrap().into_iter().filter(|e| e.0 < secs - 2.0).collect();
    let sr = 48_000.0;
    let m = PianoModel::default();
    let full = || Instrument::with_resolution(m, sr, Resolution::DEFAULT);
    let reference = render(Quality::Overkill.instrument(&m, sr), Quality::Overkill, Shortcuts::NONE, &tl, secs);
    let base = render(full(), Quality::Full, Shortcuts::DEFAULT, &tl, secs);
    let (o, w) = diff(&reference, &base);
    println!("{:34} vs Overkill {o:6.1} / {w:6.1} dB", "Full");
    let with = |f: fn(&mut PianoModel)| {
        let mut x = m;
        f(&mut x);
        x
    };
    let only = std::env::var("ONLY").ok();
    let variants: Vec<(&str, Instrument, Quality, Shortcuts)> = vec![
        ("+ no shortcuts", full(), Quality::Full, Shortcuts::NONE),
        ("+ voice silence 200 dB", full(), Quality::Full, Shortcuts { voice_silence: Shortcuts::NONE.voice_silence, ..Shortcuts::DEFAULT }),
        ("+ board silence 200 dB", full(), Quality::Full, Shortcuts { board_silence: Shortcuts::NONE.board_silence, ..Shortcuts::DEFAULT }),
        ("+ sym silence 200 dB", full(), Quality::Full, Shortcuts { sym_silence: Shortcuts::NONE.sym_silence, ..Shortcuts::DEFAULT }),
        ("+ nl silence 200 dB", full(), Quality::Full, Shortcuts { nl_silence: Shortcuts::NONE.nl_silence, ..Shortcuts::DEFAULT }),
        ("+ damped strings hear the bridge", full(), Quality::Full, Shortcuts { deaf_above: f64::INFINITY, ..Shortcuts::DEFAULT }),
        ("+ no partial dropping", full(), Quality::Full, Shortcuts { drop_partials: 0.0, ..Shortcuts::DEFAULT }),
        ("+ modes up to 0.45 fs", Instrument::with_resolution(m, sr, Resolution { max_modes: 4096, ..Resolution::DEFAULT }), Quality::Full, Shortcuts::DEFAULT),
        ("+ twice the contact steps", full(), Quality::Overkill, Shortcuts::DEFAULT),
        ("+ sympathy on every partial", Instrument::new(with(|x| x.sympathetic.max_freq = f64::INFINITY), sr), Quality::Full, Shortcuts::DEFAULT),
        ("+ phantom partials on every key", Instrument::new(with(|x| x.nonlinear.max_note = 108), sr), Quality::Full, Shortcuts::DEFAULT),
        (
            "+ phantom partials, every partial",
            Instrument::with_resolution(with(|x| {
                x.nonlinear.max_partial_hz = f64::INFINITY;
                x.nonlinear.max_long_hz = f64::INFINITY;
            }), sr, Resolution { nl_partials: 256, nl_long: 64, ..Resolution::DEFAULT }),
            Quality::Full,
            Shortcuts::DEFAULT,
        ),
    ];
    for (name, inst, q, s) in variants.into_iter().filter(|v| only.as_ref().is_none_or(|o| v.0.contains(o.as_str()))) {
        let v = render(inst, q, s, &tl, secs);
        let (fo, fw) = diff(&base, &v);
        let (ro, rw) = diff(&reference, &v);
        println!("{name:34} vs Full {fo:6.1} / {fw:6.1} dB, vs Overkill {ro:6.1} / {rw:6.1} dB");
    }
}
