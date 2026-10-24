//! Deterministic offline renderer. Same input and options → bit-identical WAV.

use clap::{Parser, Subcommand};
use piano_core::{Engine, EngineConfig, Event, TimedEvent};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "unsampled-piano-render", about = "Render MIDI to WAV with the physical piano engine")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
    #[arg(long, global = true, default_value_t = 48_000)]
    sr: u32,
    #[arg(long, global = true, default_value_t = 128)]
    block: usize,
    /// Seconds rendered after the last event.
    #[arg(long, global = true, default_value_t = 3.0)]
    tail: f64,
    /// Instrument preset (TOML).
    #[arg(long, global = true)]
    preset: Option<PathBuf>,
    /// Listening presets (microphones, room) applied over the instrument,
    /// in order (presets/listening/…).
    #[arg(long, global = true)]
    listening: Vec<PathBuf>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Render a standard MIDI file (all tracks and channels merged).
    Midi { input: PathBuf, output: PathBuf },
    /// Render one note.
    Note {
        output: PathBuf,
        #[arg(long)]
        note: u8,
        #[arg(long, default_value_t = 100)]
        velocity: u8,
        /// Key held for this many seconds.
        #[arg(long, default_value_t = 4.0)]
        hold: f64,
    },
    /// Write the full default preset (every parameter) to a TOML file.
    DumpPreset { output: PathBuf },
    /// Every key from A0 to C8, one per `step` seconds.
    Sweep {
        output: PathBuf,
        #[arg(long, default_value_t = 90)]
        velocity: u8,
        #[arg(long, default_value_t = 0.5)]
        step: f64,
        /// Hold the sustain pedal down for the whole sweep (all strings ring).
        #[arg(long)]
        sustain: bool,
    },
}

/// (time in seconds, event)
type Timeline = Vec<(f64, Event)>;

fn midi_timeline(path: &std::path::Path) -> Result<Timeline, Box<dyn std::error::Error>> {
    Ok(piano_host::read_midi_file(path)?)
}

/// Stereo render (two microphones), interleaved.
fn render(tl: &Timeline, sr: u32, block: usize, tail: f64, model: piano_core::params::PianoModel) -> (Vec<f32>, usize) {
    let cfg = EngineConfig { sample_rate: sr as f64, max_block: block, model, ..Default::default() };
    let mut engine = Engine::new(cfg);
    let end = tl.last().map_or(0.0, |e| e.0) + tail;
    let total = (end * sr as f64).ceil() as usize;
    let frames = total.next_multiple_of(block);
    let (mut l, mut r) = (vec![0.0f32; frames], vec![0.0f32; frames]);
    let mut events: Vec<TimedEvent> = Vec::with_capacity(256);
    let mut next = 0;
    let mut peak_voices = 0;
    for (bi, (bl, br)) in l.chunks_mut(block).zip(r.chunks_mut(block)).enumerate() {
        let start = bi * block;
        events.clear();
        while next < tl.len() {
            let f = (tl[next].0 * sr as f64).round() as usize;
            if f >= start + block {
                break;
            }
            events.push(TimedEvent { frame: (f.max(start) - start) as u32, event: tl[next].1 });
            next += 1;
        }
        engine.process_stereo(&events, bl, br);
        peak_voices = peak_voices.max(engine.active_voices());
    }
    let out = l[..total].iter().zip(&r[..total]).flat_map(|(a, b)| [*a, *b]).collect();
    (out, peak_voices)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let mut model = match &cli.preset {
        Some(p) => piano_host::load_preset(p)?,
        None => Default::default(),
    };
    for l in &cli.listening {
        model = piano_host::load_listening(l, &model)?;
    }
    let (tl, output): (Timeline, PathBuf) = match cli.cmd {
        Cmd::DumpPreset { output } => {
            piano_host::save_preset(&model, &output)?;
            println!("{}", output.display());
            return Ok(());
        }
        Cmd::Midi { input, output } => (midi_timeline(&input)?, output),
        Cmd::Note { output, note, velocity, hold } => {
            (vec![(0.0, Event::NoteOn { note, velocity }), (hold, Event::NoteOff { note, velocity: 64 })], output)
        }
        Cmd::Sweep { output, velocity, step, sustain } => {
            let mut tl = Vec::new();
            if sustain {
                tl.push((0.0, Event::Control { cc: 64, value: 127 }));
            }
            for (i, note) in (21u8..=108).enumerate() {
                let t = i as f64 * step;
                tl.push((t, Event::NoteOn { note, velocity }));
                tl.push((t + step * 0.9, Event::NoteOff { note, velocity: 64 }));
            }
            (tl, output)
        }
    };
    let t = std::time::Instant::now();
    let (audio, peak_voices) = render(&tl, cli.sr, cli.block, cli.tail, model);
    let wall = t.elapsed().as_secs_f64();
    let dur = audio.len() as f64 / 2.0 / cli.sr as f64;
    let peak = audio.iter().fold(0.0f32, |m, v| m.max(v.abs()));

    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: cli.sr,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut w = hound::WavWriter::create(&output, spec)?;
    for s in &audio {
        w.write_sample(*s)?;
    }
    w.finalize()?;
    let hash = audio.iter().fold(0xcbf29ce484222325u64, |h, s| (h ^ s.to_bits() as u64).wrapping_mul(0x100000001b3));
    println!(
        "{}: {dur:.2} s, peak {:.1} dBFS, max {peak_voices} voices, rendered in {wall:.2} s ({:.1}× real time), hash {hash:016x}",
        output.display(),
        20.0 * peak.max(1e-12).log10(),
        dur / wall
    );
    Ok(())
}
