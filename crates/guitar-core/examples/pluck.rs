//! Renders each preset to a mono WAV: a strummed chord, then the open
//! strings one by one; and two technique phrases. `cargo run --release -p
//! guitar-core --example pluck [out_dir]` writes guitar_acoustic.wav,
//! guitar_electric_neck.wav, guitar_electric_bridge.wav, guitar_bass.wav,
//! guitar_bass_slap.wav, guitar_electric_legato.wav and
//! guitar_electric_bends.wav (default dir: out/).

use std::sync::Arc;

use guitar_core::params::Output;
use guitar_core::engine::Technique;
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

/// Events at times (s), rendered for `total` seconds.
fn phrase(model: GuitarModel, events: &[(f64, Event)], total: f64) -> Vec<f32> {
    let mut e = Engine::new(Arc::new(Instrument::new(model, SR as f64)), 256);
    let mut out = Vec::new();
    let mut now = 0.0;
    for &(t, ev) in events.iter().chain(std::iter::once(&(total, Event::AllNotesOff))) {
        let mut buf = vec![0.0f32; ((t - now) * SR as f64).round() as usize];
        e.process(&mut buf);
        out.extend_from_slice(&buf);
        now = t;
        e.handle(ev);
    }
    println!("  largest string–fret force {:.2} N", e.fret_force());
    out
}

fn note(note: u8, velocity: u8, technique: Technique) -> Event {
    Event::Note { note, velocity, technique }
}

fn off(note: u8) -> Event {
    Event::NoteOff { note }
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

    // Slap and pop: thumb on the low E, pop on the octave, twice, then a
    // hammer-on and a slapped G.
    use Technique::*;
    let b = 0.25;
    let slap = [
        (0.0, note(28, 110, Slap)),
        (b, off(28)),
        (b, note(40, 110, Pop)),
        (2.0 * b, off(40)),
        (2.0 * b, note(28, 110, Slap)),
        (3.0 * b, off(28)),
        (3.0 * b, note(40, 115, Pop)),
        (4.0 * b, off(40)),
        (4.0 * b, note(31, 110, Slap)),
        (4.5 * b, note(33, 100, Tap)),
        (6.0 * b, off(33)),
        (6.0 * b, note(35, 120, Slap)),
    ];
    write(&format!("{dir}/guitar_bass_slap.wav"), &phrase(GuitarModel::electric_bass(), &slap, 3.0));

    // Legato on the electric (neck pickup): A on the 5th fret of the low E,
    // hammer-on to the 7th, pull-off back, a tap at the 12th, then a hard
    // pluck toward the board on low action (buzz).
    let mut m = GuitarModel::electric_solid();
    m.fretboard.action_bass = 1.0e-3;
    let lick = [
        (0.0, note(45, 100, Pluck)),
        (0.4, note(47, 100, Tap)),
        (0.8, note(45, 100, PullOff)),
        (1.2, note(52, 110, Tap)),
        (2.0, off(52)),
        (2.2, note(40, 127, Pluck)),
    ];
    write(&format!("{dir}/guitar_electric_legato.wav"), &phrase(m, &lick, 4.0));

    // Bends: A on the G string bent a whole tone up over 150 ms (wheel
    // messages every 10 ms), vibrato, let down; then an E chord dived with
    // the arm and brought back.
    let mut bends = vec![(0.0, note(57, 100, Pluck))];
    let ramp = |t0: f64, from: f64, to: f64, out: &mut Vec<(f64, Event)>| {
        for i in 0..=15 {
            let x = i as f64 / 15.0;
            out.push((t0 + 0.01 * i as f64, Event::Bend { note: None, semitones: from + (to - from) * x }));
        }
    };
    ramp(0.4, 0.0, 2.0, &mut bends);
    bends.push((1.0, Event::Vibrato { depth: 0.25 }));
    bends.push((2.0, Event::Vibrato { depth: 0.0 }));
    ramp(2.1, 2.0, 0.0, &mut bends);
    bends.push((2.6, off(57)));
    for (i, n) in [40, 47, 52, 56, 59, 64].into_iter().enumerate() {
        bends.push((2.8 + 0.02 * i as f64, note(n, 95, Pluck)));
    }
    for i in 0..=30 {
        // Down to 0.8 and back, over 1.2 s.
        let x = (std::f64::consts::PI * i as f64 / 30.0).sin();
        bends.push((3.4 + 0.04 * i as f64, Event::Tremolo { position: 0.8 * x }));
    }
    write(&format!("{dir}/guitar_electric_bends.wav"), &phrase(GuitarModel::electric_solid(), &bends, 6.0));
}
