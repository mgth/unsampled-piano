//! Deterministic offline renderer. Same input and options → bit-identical WAV.

use clap::{Parser, Subcommand};
use piano_core::{Engine, EngineConfig, Event, TimedEvent};
use std::path::PathBuf;

#[derive(Parser)]
#[command(about = "Render MIDI to WAV with the physical piano engine")]
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
    /// Every key from A0 to C8, one per `step` seconds.
    Sweep {
        output: PathBuf,
        #[arg(long, default_value_t = 90)]
        velocity: u8,
        #[arg(long, default_value_t = 0.5)]
        step: f64,
    },
}

/// (time in seconds, event)
type Timeline = Vec<(f64, Event)>;

fn midi_timeline(path: &PathBuf) -> Result<Timeline, Box<dyn std::error::Error>> {
    use midly::{MetaMessage, MidiMessage, Smf, Timing, TrackEventKind};
    let data = std::fs::read(path)?;
    let smf = Smf::parse(&data)?;
    let mut evs: Vec<(u64, usize, TrackEventKind)> = Vec::new();
    for (ti, track) in smf.tracks.iter().enumerate() {
        let mut tick = 0u64;
        for e in track {
            tick += e.delta.as_int() as u64;
            evs.push((tick, ti, e.kind));
        }
    }
    evs.sort_by_key(|e| (e.0, e.1));
    let (tpb, fixed_tick_s) = match smf.header.timing {
        Timing::Metrical(t) => (t.as_int() as f64, None),
        Timing::Timecode(fps, sub) => (1.0, Some(1.0 / (fps.as_f32() as f64 * sub as f64))),
    };
    let mut us_per_beat = 500_000.0;
    let (mut last_tick, mut time) = (0u64, 0.0f64);
    let mut out = Vec::new();
    for (tick, _, kind) in evs {
        let dt = (tick - last_tick) as f64;
        time += fixed_tick_s.unwrap_or(us_per_beat * 1e-6 / tpb) * dt;
        last_tick = tick;
        match kind {
            TrackEventKind::Meta(MetaMessage::Tempo(t)) => us_per_beat = t.as_int() as f64,
            TrackEventKind::Midi { message, .. } => {
                let ev = match message {
                    MidiMessage::NoteOn { key, vel } => Event::NoteOn { note: key.as_int(), velocity: vel.as_int() },
                    MidiMessage::NoteOff { key, vel } => Event::NoteOff { note: key.as_int(), velocity: vel.as_int() },
                    MidiMessage::Controller { controller, value } => {
                        Event::Control { cc: controller.as_int(), value: value.as_int() }
                    }
                    _ => continue,
                };
                out.push((time, ev));
            }
            _ => {}
        }
    }
    Ok(out)
}

fn render(tl: &Timeline, sr: u32, block: usize, tail: f64) -> (Vec<f32>, usize) {
    let cfg = EngineConfig { sample_rate: sr as f64, max_block: block, ..Default::default() };
    let mut engine = Engine::new(cfg);
    let end = tl.last().map_or(0.0, |e| e.0) + tail;
    let total = (end * sr as f64).ceil() as usize;
    let mut out = vec![0.0f32; total.next_multiple_of(block)];
    let mut events: Vec<TimedEvent> = Vec::with_capacity(256);
    let mut next = 0;
    let mut peak_voices = 0;
    for (bi, blk) in out.chunks_mut(block).enumerate() {
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
        engine.process(&events, blk);
        peak_voices = peak_voices.max(engine.active_voices());
    }
    out.truncate(total);
    (out, peak_voices)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let (tl, output): (Timeline, PathBuf) = match cli.cmd {
        Cmd::Midi { input, output } => (midi_timeline(&input)?, output),
        Cmd::Note { output, note, velocity, hold } => {
            (vec![(0.0, Event::NoteOn { note, velocity }), (hold, Event::NoteOff { note, velocity: 64 })], output)
        }
        Cmd::Sweep { output, velocity, step } => {
            let mut tl = Vec::new();
            for (i, note) in (21u8..=108).enumerate() {
                let t = i as f64 * step;
                tl.push((t, Event::NoteOn { note, velocity }));
                tl.push((t + step * 0.9, Event::NoteOff { note, velocity: 64 }));
            }
            (tl, output)
        }
    };
    let t = std::time::Instant::now();
    let (audio, peak_voices) = render(&tl, cli.sr, cli.block, cli.tail);
    let wall = t.elapsed().as_secs_f64();
    let dur = audio.len() as f64 / cli.sr as f64;
    let peak = audio.iter().fold(0.0f32, |m, v| m.max(v.abs()));

    let spec = hound::WavSpec {
        channels: 1,
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
