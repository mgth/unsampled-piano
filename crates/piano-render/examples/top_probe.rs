//! Controlled top-register diagnostics. No production settings are changed.
//! top_probe PRESET NOTE VELOCITY MODE OUTPUT.wav [HOLD] [TAIL]
//! MODE: base, listen_all, thresholds, contact2, uncapped, nonlinear, force.
//! `contact` writes per-string force samples and contact statistics to stdout.
use piano_core::{
    Engine, EngineConfig, Event, Quality, TimedEvent,
    engine::{Shortcuts, strike_forces, strike_stats_full},
    instrument::Instrument,
    note_model::Resolution,
};
use std::{path::Path, sync::Arc};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<String> = std::env::args().collect();
    if a.len() < 6 {
        return Err("top_probe PRESET NOTE VELOCITY MODE OUTPUT.wav [HOLD] [TAIL]".into());
    }
    let mut model = piano_host::load_preset(Path::new(&a[1]))?;
    let note: u8 = a[2].parse()?;
    let velocity: u8 = a[3].parse()?;
    let mode = a[4].as_str();
    if mode == "force" {
        model.soundboard.as_mut().unwrap().radiate = false;
    }
    if mode == "nonlinear" {
        model.nonlinear.max_note = 108;
    }
    let mut cfg = EngineConfig { model, ..Default::default() };
    if mode == "contact" {
        let p = model.note(note);
        let (t, f, u) = strike_stats_full(&cfg, note, velocity, None);
        eprintln!(
            "note={note},velocity={velocity},f0={},B={},q0={},duration_ms={},peak_n={f},compression_mm={}",
            p.f0,
            p.b,
            p.hammer.q0,
            t * 1000.,
            u * 1000.
        );
        println!("sample,s0,s1,s2");
        for (i, fs) in strike_forces(&cfg, note, velocity).iter().enumerate() {
            println!("{i},{},{},{}", fs[0], fs[1], fs[2]);
        }
        return Ok(());
    }
    // Keep the same built instrument while changing only contact stepping.
    let resolution = if mode == "uncapped" { Resolution::UNCAPPED } else { Resolution::DEFAULT };
    let inst = Arc::new(Instrument::with_resolution(model, cfg.sample_rate, resolution));
    if mode == "contact2" {
        cfg.quality = Quality::Overkill;
    }
    let mut engine = Engine::with_instrument(cfg, inst);
    let mut shortcuts = Shortcuts::DEFAULT;
    match mode {
        "listen_all" => {
            shortcuts.listen_all = true;
            shortcuts.deaf_above = f64::INFINITY;
        }
        "thresholds" => {
            shortcuts = Shortcuts { listen_all: false, deaf_above: 0.99, ..Shortcuts::NONE };
        }
        "base" | "contact2" | "uncapped" | "nonlinear" | "force" => (),
        _ => return Err(format!("unknown mode {mode}").into()),
    }
    engine.set_shortcuts(shortcuts);
    let hold: f64 = a.get(6).map(|s| s.parse()).transpose()?.unwrap_or(1.3);
    let tail: f64 = a.get(7).map(|s| s.parse()).transpose()?.unwrap_or(0.1);
    let seconds = hold + tail;
    let off = (hold * cfg.sample_rate).round() as usize;
    let n = (seconds * cfg.sample_rate).round() as usize;
    let spec =
        hound::WavSpec { channels: 2, sample_rate: cfg.sample_rate as u32, bits_per_sample: 32, sample_format: hound::SampleFormat::Float };
    let mut wav = hound::WavWriter::create(&a[5], spec)?;
    for start in (0..n).step_by(128) {
        let len = (n - start).min(128);
        let (mut l, mut r) = ([0.0f32; 128], [0.0f32; 128]);
        let mut events = Vec::new();
        if start == 0 {
            events.push(TimedEvent { frame: 0, event: Event::NoteOn { note, velocity } });
        }
        if off >= start && off < start + len {
            events.push(TimedEvent { frame: (off - start) as u32, event: Event::NoteOff { note, velocity: 64 } });
        }
        engine.process_stereo(&events, &mut l[..len], &mut r[..len]);
        for i in 0..len {
            wav.write_sample(l[i])?;
            wav.write_sample(r[i])?;
        }
    }
    wav.finalize()?;
    Ok(())
}
