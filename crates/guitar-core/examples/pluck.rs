//! Renders a strummed open C chord, then the open strings one by one, to a
//! mono WAV: `cargo run --release -p guitar-core --example pluck [out.wav]`.

use std::sync::Arc;

use guitar_core::{Engine, Event, GuitarModel, Instrument};

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| "out/guitar_pluck.wav".into());
    let sr = 48_000u32;
    let mut e = Engine::new(Arc::new(Instrument::new(GuitarModel::default(), sr as f64)), 256);
    let mut out = Vec::new();
    let mut run = |e: &mut Engine, seconds: f64| {
        let mut buf = vec![0.0f32; (seconds * sr as f64) as usize];
        e.process(&mut buf);
        out.extend_from_slice(&buf);
    };
    // Strum, low to high, 25 ms apart.
    for n in [48, 52, 55, 60, 64] {
        e.handle(Event::NoteOn { note: n, velocity: 90 });
        run(&mut e, 0.025);
    }
    run(&mut e, 3.0);
    e.handle(Event::AllNotesOff);
    run(&mut e, 0.5);
    for n in [40, 45, 50, 55, 59, 64] {
        e.handle(Event::NoteOn { note: n, velocity: 100 });
        run(&mut e, 1.5);
        e.handle(Event::NoteOff { note: n });
    }
    run(&mut e, 0.5);

    if let Some(dir) = std::path::Path::new(&path).parent() {
        std::fs::create_dir_all(dir).unwrap();
    }
    let spec = hound::WavSpec { channels: 1, sample_rate: sr, bits_per_sample: 32, sample_format: hound::SampleFormat::Float };
    let mut w = hound::WavWriter::create(&path, spec).unwrap();
    for v in &out {
        w.write_sample(*v).unwrap();
    }
    w.finalize().unwrap();
    let peak = out.iter().fold(0.0f32, |m, v| m.max(v.abs()));
    println!("{path}: {:.1} s, peak {:.1} dBFS", out.len() as f64 / sr as f64, 20.0 * peak.log10());
}
