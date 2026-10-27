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
    /// CPU quality: overkill (no shortcut, the reference), full (default),
    /// balanced or light (docs/performance.md).
    #[arg(long, global = true, default_value = "full")]
    quality: String,
    /// Threads the engine renders on, as the player's setting (the output
    /// is the same, bit for bit, whatever the count).
    #[arg(long, global = true, default_value_t = 1)]
    threads: usize,
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
    /// Render a player's flight recording again (the folder it wrote when
    /// it stopped), block for block, and compare with the audio it kept.
    Replay { dir: PathBuf, output: PathBuf },
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

/// Renders a flight recording (crate piano-play's blackbox) with the same
/// blocks, events, preset, rate, threads and quality, writes it, and
/// compares the part the player kept with what it played.
fn replay(dir: &std::path::Path, output: &std::path::Path, threads_cli: usize) -> Result<(), Box<dyn std::error::Error>> {
    let text = std::fs::read_to_string(dir.join("events.txt"))?;
    let model = piano_host::load_preset(&dir.join("preset.toml"))?;
    let (mut sr, mut threads, mut quality, mut audio_from) = (48_000.0, 1usize, piano_core::Quality::default(), 0u64);
    let mut runs: Vec<(usize, usize)> = Vec::new();
    let mut events: Vec<(u64, Event)> = Vec::new();
    for line in text.lines() {
        let w: Vec<&str> = line.split_whitespace().collect();
        match w.as_slice() {
            ["sample_rate", v] => sr = v.parse()?,
            ["threads", v] => threads = v.parse()?,
            ["quality", v] => quality = piano_core::Quality::from_name(v).unwrap_or_default(),
            ["audio_from", v] => audio_from = v.parse()?,
            ["complete", "false"] => eprintln!("warning: the recording overflowed; the replay stops matching where it did"),
            ["B", len, count] => runs.push((len.parse()?, count.parse()?)),
            ["E", t, kind, rest @ ..] => {
                let t: u64 = t.parse()?;
                let n = |i: usize| -> Result<u8, Box<dyn std::error::Error>> { Ok(rest.get(i).ok_or("short event")?.parse()?) };
                let e = match *kind {
                    "on" => Event::NoteOn { note: n(0)?, velocity: n(1)? },
                    "off" => Event::NoteOff { note: n(0)?, velocity: n(1)? },
                    "cc" => Event::Control { cc: n(0)?, value: n(1)? },
                    "all_off" => Event::AllNotesOff,
                    "param" => {
                        let id = piano_core::LiveParam::from_path(rest.first().ok_or("short event")?).ok_or("unknown parameter")?;
                        Event::Param { id, value: rest.get(1).ok_or("short event")?.parse()? }
                    }
                    k => return Err(format!("unknown event kind {k}").into()),
                };
                events.push((t, e));
            }
            _ => {}
        }
    }
    let threads = if threads_cli > 1 { threads_cli } else { threads };
    let max_block = runs.iter().map(|r| r.0).max().unwrap_or(128);
    let cfg = EngineConfig { sample_rate: sr, max_block, model, quality, threads, ..Default::default() };
    let mut engine = Engine::new(cfg);
    let total: usize = runs.iter().map(|(l, c)| l * c).sum();
    let (mut l, mut r) = (vec![0.0f32; total], vec![0.0f32; total]);
    let (mut start, mut next) = (0usize, 0usize);
    let mut block_events: Vec<TimedEvent> = Vec::with_capacity(256);
    let t0 = std::time::Instant::now();
    for &(len, count) in &runs {
        for _ in 0..count {
            block_events.clear();
            while next < events.len() && (events[next].0 as usize) < start + len {
                block_events.push(TimedEvent { frame: (events[next].0 as usize - start) as u32, event: events[next].1 });
                next += 1;
            }
            engine.process_stereo(&block_events, &mut l[start..start + len], &mut r[start..start + len]);
            start += len;
        }
    }
    let spec = hound::WavSpec { channels: 2, sample_rate: sr as u32, bits_per_sample: 32, sample_format: hound::SampleFormat::Float };
    let mut w = hound::WavWriter::create(output, spec)?;
    for (a, b) in l.iter().zip(&r) {
        w.write_sample(*a)?;
        w.write_sample(*b)?;
    }
    w.finalize()?;
    println!("{}: {:.1} s, {} events, {} threads, rendered in {:.1} s", output.display(), total as f64 / sr, events.len(), threads, t0.elapsed().as_secs_f64());
    // The part the player kept.
    let mut kept = hound::WavReader::open(dir.join("audio.wav"))?;
    let rec: Vec<f32> = kept.samples::<f32>().collect::<Result<_, _>>()?;
    let mut worst = (0.0f32, 0usize);
    let mut first = None;
    for (i, pair) in rec.chunks(2).enumerate() {
        let t = audio_from as usize + i;
        if t >= total {
            break;
        }
        let d = (pair[0] - l[t]).abs().max((pair[1] - r[t]).abs());
        if d > 0.0 && first.is_none() {
            first = Some(t);
        }
        if d > worst.0 {
            worst = (d, t);
        }
    }
    match first {
        None => println!("identical to what the player played (its last {:.1} s)", rec.len() as f64 / 2.0 / sr),
        Some(t) => println!(
            "differs from what the player played: first at {:.3} s, largest {:.3e} at {:.3} s",
            t as f64 / sr,
            worst.0,
            worst.1 as f64 / sr
        ),
    }
    Ok(())
}

/// Stereo render (two microphones), interleaved.
fn render(tl: &Timeline, sr: u32, block: usize, tail: f64, model: piano_core::params::PianoModel, quality: piano_core::Quality, threads: usize) -> (Vec<f32>, usize) {
    let cfg = EngineConfig { sample_rate: sr as f64, max_block: block, model, quality, threads, ..Default::default() };
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
        Cmd::Replay { dir, output } => return replay(&dir, &output, cli.threads),
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
    let quality = piano_core::Quality::from_name(&cli.quality).ok_or_else(|| format!("unknown quality {:?}: overkill, full, balanced or light", cli.quality))?;
    let (audio, peak_voices) = render(&tl, cli.sr, cli.block, cli.tail, model, quality, cli.threads);
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
