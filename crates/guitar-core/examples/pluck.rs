//! Renders each preset to a mono WAV: a strummed chord, then the open
//! strings one by one. `cargo run --release -p guitar-core --example pluck
//! [out_dir]` writes guitar_acoustic.wav, guitar_electric_neck.wav,
//! guitar_electric_bridge.wav and guitar_bass.wav (default dir: out/).

use std::sync::Arc;

use guitar_core::params::Output;
use guitar_core::{Engine, Event, GuitarModel, Instrument};

const SR: u32 = 48_000;

fn render(model: GuitarModel, chord: &[u8]) -> Vec<f32> {
    let opens: Vec<u8> = model.strings.iter().map(|s| s.open_note).collect();
    let mut e = Engine::new(Arc::new(Instrument::new(model, SR as f64)), 256);
    let mut out = Vec::new();
    let mut run = |e: &mut Engine, seconds: f64| {
        let mut buf = vec![0.0f32; (seconds * SR as f64) as usize];
        e.process(&mut buf);
        out.extend_from_slice(&buf);
    };
    // Strum, low to high, 25 ms apart.
    for &n in chord {
        e.handle(Event::NoteOn { note: n, velocity: 90 });
        run(&mut e, 0.025);
    }
    run(&mut e, 3.0);
    e.handle(Event::AllNotesOff);
    run(&mut e, 0.5);
    for n in opens {
        e.handle(Event::NoteOn { note: n, velocity: 100 });
        run(&mut e, 1.5);
        e.handle(Event::NoteOff { note: n });
    }
    run(&mut e, 0.5);
    out
}

fn write(path: &str, x: &[f32]) {
    let spec = hound::WavSpec { channels: 1, sample_rate: SR, bits_per_sample: 32, sample_format: hound::SampleFormat::Float };
    let mut w = hound::WavWriter::create(path, spec).unwrap();
    for v in x {
        w.write_sample(*v).unwrap();
    }
    w.finalize().unwrap();
    let peak = x.iter().fold(0.0f32, |m, v| m.max(v.abs()));
    let rms = (x.iter().map(|&v| (v as f64).powi(2)).sum::<f64>() / x.len() as f64).sqrt();
    println!("{path}: {:.1} s, peak {:.1} dBFS, rms {:.1} dBFS", x.len() as f64 / SR as f64, 20.0 * peak.log10(), 20.0 * rms.log10());
}

fn main() {
    let dir = std::env::args().nth(1).unwrap_or_else(|| "out".into());
    std::fs::create_dir_all(&dir).unwrap();
    let open_c = [48, 52, 55, 60, 64];
    write(&format!("{dir}/guitar_acoustic.wav"), &render(GuitarModel::steel_acoustic(), &open_c));
    // E power chord on the electric, neck then bridge pickup.
    let power = [40, 47, 52, 56, 59, 64];
    for (name, pickup) in [("neck", 2), ("bridge", 0)] {
        let mut m = GuitarModel::electric_solid();
        if let Output::Magnetic(o) = &mut m.output {
            o.selected = vec![pickup];
        }
        write(&format!("{dir}/guitar_electric_{name}.wav"), &render(m, &power));
    }
    write(&format!("{dir}/guitar_bass.wav"), &render(GuitarModel::electric_bass(), &[28, 35, 40]));
}
