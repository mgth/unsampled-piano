//! Real-time player. The instrument is built on the main thread; the MIDI
//! callback pushes decoded events into a lock-free queue; the audio callback
//! drains it at the start of each buffer and renders (no allocation, no
//! lock). CPU load is reported once per second.

use clap::Parser;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use piano_core::instrument::Instrument;
use piano_core::{Engine, EngineConfig, Event, TimedEvent};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Parser)]
#[command(about = "Play the physical piano from a MIDI keyboard")]
struct Args {
    /// Instrument preset (TOML); see `piano-render dump-preset`.
    #[arg(long)]
    preset: Option<PathBuf>,
    /// MIDI input port: substring of its name (default: first real port).
    #[arg(long)]
    midi: Option<String>,
    /// Audio host: pipewire (default when available), jack or alsa.
    #[arg(long)]
    host: Option<String>,
    /// Audio output device: substring of its name (default: the system
    /// default, else PipeWire, else PulseAudio).
    #[arg(long)]
    device: Option<String>,
    /// Audio buffer size in frames.
    #[arg(long, default_value_t = 256)]
    buffer: u32,
    /// Output gain (dB).
    #[arg(long, default_value_t = 0.0)]
    gain_db: f32,
    /// List audio devices and MIDI ports, then exit.
    #[arg(long)]
    list: bool,
    /// Play a built-in sequence instead of listening to MIDI.
    #[arg(long)]
    demo: bool,
}

/// Engine block limit; host buffers are rendered in pieces of this size.
const MAX_BLOCK: usize = 1024;
const MAX_EVENTS: usize = 256;

#[derive(Default)]
struct Stats {
    /// Max callback time / buffer duration over the last period, ×1e4.
    load_max: AtomicU32,
    load_sum: AtomicU32,
    callbacks: AtomicU32,
    voices: AtomicU32,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let host = pick_host(args.host.as_deref())?;
    let midi_in = midir::MidiInput::new("piano-play")?;
    if args.list {
        println!("audio host: {:?}", host.id());
        for d in host.output_devices()? {
            println!("  output: {}", d.description().map(|x| x.to_string()).unwrap_or_default());
        }
        for p in midi_in.ports() {
            println!("  MIDI in: {}", midi_in.port_name(&p).unwrap_or_default());
        }
        return Ok(());
    }

    let named = |want: &str| {
        host.output_devices().ok()?.find(|d| d.description().map(|x| x.to_string()).unwrap_or_default().contains(want))
    };
    let usable = |d: &cpal::Device| d.default_output_config().is_ok();
    let device = match &args.device {
        Some(want) => named(want).ok_or_else(|| format!("no output device matching {want:?} (see --list)"))?,
        None => host
            .default_output_device()
            .filter(usable)
            .or_else(|| named("PipeWire").filter(usable))
            .or_else(|| named("PulseAudio").filter(usable))
            .ok_or("no usable audio output device (see --list, --device)")?,
    };
    eprintln!("audio output: {}", device.description().map(|x| x.to_string()).unwrap_or_default());
    let supported = device.default_output_config()?;
    let sr = supported.sample_rate() as f64;
    let channels = supported.channels() as usize;
    if supported.sample_format() != cpal::SampleFormat::F32 {
        return Err(format!("unsupported sample format {:?}", supported.sample_format()).into());
    }
    let model = match &args.preset {
        Some(p) => piano_host::load_preset(p)?,
        None => Default::default(),
    };
    eprintln!("building the instrument at {sr} Hz…");
    let t = Instant::now();
    let inst = Arc::new(Instrument::new(model, sr));
    eprintln!("built in {:.2} s", t.elapsed().as_secs_f64());
    let cfg = EngineConfig { sample_rate: sr, max_block: MAX_BLOCK, model, ..Default::default() };
    let mut engine = Engine::with_instrument(cfg, inst);

    let (mut tx, mut rx) = rtrb::RingBuffer::<Event>::new(1024);
    let _midi_conn = if args.demo {
        std::thread::spawn(move || demo(tx));
        None
    } else {
        let ports = midi_in.ports();
        let name = |p: &midir::MidiInputPort| midi_in.port_name(p).unwrap_or_default();
        let port = match &args.midi {
            Some(want) => ports.iter().find(|p| name(p).contains(want.as_str())),
            None => ports.iter().find(|p| !name(p).contains("Through")),
        };
        match port.cloned() {
            Some(p) => {
                eprintln!("MIDI input: {}", name(&p));
                let conn = midi_in.connect(
                    &p,
                    "piano-play-in",
                    move |_, msg, _| {
                        if let Some(ev) = piano_host::decode_midi(msg) {
                            let _ = tx.push(ev);
                        }
                    },
                    (),
                )?;
                Some(conn)
            }
            None => {
                eprintln!("no MIDI input port found (use --list / --midi, or --demo)");
                None
            }
        }
    };

    let stats = Arc::new(Stats::default());
    let st = stats.clone();
    let gain = 10f32.powf(args.gain_db / 20.0);
    let (mut l, mut r) = (vec![0.0f32; MAX_BLOCK], vec![0.0f32; MAX_BLOCK]);
    let mut events: Vec<TimedEvent> = Vec::with_capacity(MAX_EVENTS);
    let config = cpal::StreamConfig {
        channels: supported.channels(),
        sample_rate: supported.sample_rate(),
        buffer_size: cpal::BufferSize::Fixed(args.buffer),
    };
    let stream = device.build_output_stream(
        config,
        move |data: &mut [f32], _| {
            let start = Instant::now();
            let frames = data.len() / channels;
            events.clear();
            while events.len() < MAX_EVENTS {
                match rx.pop() {
                    Ok(event) => events.push(TimedEvent { frame: 0, event }),
                    Err(_) => break,
                }
            }
            let mut done = 0;
            while done < frames {
                let n = (frames - done).min(MAX_BLOCK);
                let ev = if done == 0 { &events[..] } else { &[] };
                engine.process_stereo(ev, &mut l[..n], &mut r[..n]);
                for (i, f) in data[done * channels..(done + n) * channels].chunks_mut(channels).enumerate() {
                    f[0] = gain * l[i];
                    if channels > 1 {
                        f[1] = gain * r[i];
                    }
                    for x in f.iter_mut().skip(2) {
                        *x = 0.0;
                    }
                }
                done += n;
            }
            let load = start.elapsed().as_secs_f64() * sr / frames.max(1) as f64;
            let l4 = (load * 1e4) as u32;
            st.load_max.fetch_max(l4, Ordering::Relaxed);
            st.load_sum.fetch_add(l4, Ordering::Relaxed);
            st.callbacks.fetch_add(1, Ordering::Relaxed);
            st.voices.store(engine.active_voices() as u32, Ordering::Relaxed);
        },
        |e| eprintln!("audio error: {e}"),
        None,
    )?;
    stream.play()?;
    eprintln!("playing ({} frames/buffer, {channels} channels). Ctrl-C to quit.", args.buffer);
    loop {
        std::thread::sleep(Duration::from_secs(1));
        let n = stats.callbacks.swap(0, Ordering::Relaxed).max(1);
        let avg = stats.load_sum.swap(0, Ordering::Relaxed) as f64 / n as f64 / 100.0;
        let max = stats.load_max.swap(0, Ordering::Relaxed) as f64 / 100.0;
        eprintln!("CPU load avg {avg:5.1} %  max {max:5.1} %  voices {:3}", stats.voices.load(Ordering::Relaxed));
    }
}

fn pick_host(want: Option<&str>) -> Result<cpal::Host, Box<dyn std::error::Error>> {
    let ids = cpal::available_hosts();
    let find = |name: &str| ids.iter().find(|id| format!("{id:?}").to_lowercase().contains(name)).copied();
    let id = match want {
        Some(w) => find(&w.to_lowercase()).ok_or_else(|| format!("audio host {w:?} not available: {ids:?}"))?,
        None => find("pipewire").unwrap_or_else(|| cpal::default_host().id()),
    };
    Ok(cpal::host_from_id(id)?)
}

/// A short built-in sequence (no keyboard needed): chords with pedal.
fn demo(mut tx: rtrb::Producer<Event>) {
    let wait = |s: f64| std::thread::sleep(Duration::from_secs_f64(s));
    wait(0.5);
    for round in 0..3 {
        let _ = tx.push(Event::Control { cc: 64, value: 127 });
        for (i, chord) in [[48u8, 55, 64], [45, 52, 60], [41, 48, 57], [43, 50, 59]].iter().enumerate() {
            for &n in chord {
                let _ = tx.push(Event::NoteOn { note: n + 12 * (round % 2), velocity: 70 + 15 * i as u8 });
            }
            wait(0.9);
            for &n in chord {
                let _ = tx.push(Event::NoteOff { note: n + 12 * (round % 2), velocity: 64 });
            }
        }
        let _ = tx.push(Event::Control { cc: 64, value: 0 });
        wait(1.0);
    }
}
