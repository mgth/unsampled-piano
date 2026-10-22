//! Real-time player. By default the piano is a node of the PipeWire graph
//! (through PipeWire's JACK API): stereo audio out, a MIDI input with
//! sample-accurate timing, and a MIDI output mirroring every event the piano
//! plays; the keyboard and the speakers are connected automatically. The
//! cpal backends (`--host pipewire|jack|alsa`) remain, with the keyboard read
//! through midir and played at the start of each buffer. MIDI files play from
//! a timeline owned by the audio thread, sample-accurately. The instrument
//! is built on the main thread; the audio thread neither allocates nor
//! locks. CPU load is reported once per second.

mod node;
mod player;
mod window;

use clap::Parser;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use piano_core::{EngineConfig, Event};
use piano_control::Controller;
use piano_params::midi::MidiMap;
use piano_params::PianoControl;
use player::{Player, Stats, MAX_BLOCK};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Parser)]
#[command(name = "unsampled-piano", about = "Play the physical piano from a MIDI keyboard or MIDI files")]
struct Args {
    /// Instrument preset (TOML); see `unsampled-piano-render dump-preset`.
    #[arg(long)]
    preset: Option<PathBuf>,
    /// MIDI input port to connect: substring of its name (default: the
    /// first hardware port).
    #[arg(long)]
    midi: Option<String>,
    /// Audio backend: node (default: the piano as a graph node with MIDI in
    /// and out, through PipeWire's JACK API), or a cpal host without MIDI
    /// ports: pipewire, jack, alsa.
    #[arg(long)]
    host: Option<String>,
    /// Name of the graph node.
    #[arg(long, default_value = "unsampled-piano")]
    name: String,
    /// Connect the MIDI mirror output to the graph's MIDI inputs whose name
    /// contains this (node backend), e.g. another synthesiser.
    #[arg(long)]
    midi_out: Option<String>,
    /// Audio output: substring of the device (cpal) or of the playback port
    /// names (node).
    #[arg(long)]
    device: Option<String>,
    /// Audio buffer size in frames (node: requested graph period).
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
    /// Play standard MIDI files, one after the other (the keyboard is only
    /// listened to if --midi is also given).
    #[arg(long, num_args = 1..)]
    file: Vec<PathBuf>,
    /// Start the first file at this time (s).
    #[arg(long, default_value_t = 0.0)]
    from: f64,
    /// Open the interface window (every parameter, presets, MIDI learn).
    #[arg(long)]
    ui: bool,
    /// Preset folder of the interface (default: the --preset file's, else
    /// ./presets).
    #[arg(long)]
    presets: Option<PathBuf>,
}

/// Silence between two files (s), and after the last one before quitting.
const GAP_S: f64 = 4.0;

/// Where the audio goes.
enum Backend {
    Node(node::Node),
    Cpal { device: cpal::Device, config: cpal::SupportedStreamConfig },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = Args::parse();
    apply_user_config(&mut args);
    let midi_in = midir::MidiInput::new("unsampled-piano")?;
    if args.list {
        let host = pick_host(args.host.as_deref().filter(|h| *h != "node"))?;
        println!("audio host: {:?}", host.id());
        for d in host.output_devices()? {
            println!("  output: {}", d.description().map(|x| x.to_string()).unwrap_or_default());
        }
        for p in midi_in.ports() {
            println!("  MIDI in: {}", midi_in.port_name(&p).unwrap_or_default());
        }
        if let Ok(n) = node::Node::open(&args.name, args.buffer) {
            let (midi, audio) = n.graph_ports();
            println!("graph (node backend), default sink: {:?}", default_sink_prefix());
            for p in midi {
                println!("  MIDI source: {p}");
            }
            for p in audio {
                println!("  audio input: {p}");
            }
        }
        return Ok(());
    }

    let backend = match args.host.as_deref() {
        None | Some("node") => match node::Node::open(&args.name, args.buffer) {
            Ok(n) => Backend::Node(n),
            Err(e) if args.host.is_none() => {
                eprintln!("graph node unavailable ({e}); falling back to cpal/PipeWire, without MIDI ports");
                cpal_backend(&args)?
            }
            Err(e) => return Err(e.into()),
        },
        Some(_) => cpal_backend(&args)?,
    };
    let sr = match &backend {
        Backend::Node(n) => n.sample_rate(),
        Backend::Cpal { config, .. } => config.sample_rate() as f64,
    };

    let model = match &args.preset {
        Some(p) => piano_host::load_preset(p)?,
        None => Default::default(),
    };
    let cfg = EngineConfig { sample_rate: sr, max_block: MAX_BLOCK, model, ..Default::default() };
    let (mut control, engine) = piano_control::new(model, cfg);
    eprintln!("instrument built at {sr} Hz in {:.0} ms", control.status().last_build.unwrap_or_default().as_secs_f64() * 1e3);
    let mut midi_map = MidiMapFile::new();
    if let Some(map) = midi_map.load() {
        eprintln!("MIDI controllers: {} mapped ({})", map.mappings.len(), midi_map.path.as_ref().map_or(String::new(), |p| p.display().to_string()));
        control.set_midi_map(map);
    }
    let control = Arc::new(Mutex::new(control));
    let watch = args.preset.clone().map(PresetWatch::new);

    let (timeline, pieces, end_s) = file_timeline(&args, sr)?;
    for (start, name, len) in &pieces {
        eprintln!("  {:>7}  {name} ({}:{:02})", clock_str(*start), (*len as u64) / 60, (*len as u64) % 60);
    }

    // Events from other threads (demo, keyboard through midir), played at
    // the start of the next buffer.
    let (tx, rx) = rtrb::RingBuffer::<Event>::new(1024);
    let listen = args.file.is_empty() || args.midi.is_some();
    let stats = Arc::new(Stats::default());
    let player = Player::new(engine, sr, timeline, Some(rx), stats.clone());
    let gain = 10f32.powf(args.gain_db / 20.0);

    let mut _midi_conn = None;
    let mut _stream = None;
    let mut _node = None;
    if args.demo {
        std::thread::spawn(move || demo(tx));
    } else if matches!(backend, Backend::Cpal { .. }) && listen {
        _midi_conn = midir_input(midi_in, args.midi.as_deref(), tx)?;
    }
    match backend {
        Backend::Node(n) => {
            let (midi_sources, audio_inputs) = n.graph_ports();
            let period = n.buffer_size();
            let (running, names) = n.start(player, gain)?;
            eprintln!("graph node \"{}\" ({period} frames/period)", args.name);
            eprintln!("  MIDI in:  {}", names.midi_in);
            eprintln!("  MIDI out: {} (mirror of what the piano plays)", names.midi_out);
            // Audio: --device, else the system's default sink.
            let prefix = match &args.device {
                Some(w) => Some(w.clone()),
                None => default_sink_prefix().map(|p| format!("{p}:")),
            };
            let mut sinks: Vec<&String> = match &prefix {
                Some(p) => audio_inputs.iter().filter(|x| x.contains(p.as_str())).collect(),
                None => Vec::new(),
            };
            if sinks.is_empty() && args.device.is_some() {
                // The configured output is not running: the default sink.
                let fallback = default_sink_prefix().map(|p| format!("{p}:"));
                eprintln!("  output {:?} not found, using the default sink {fallback:?}", args.device.as_deref().unwrap_or(""));
                if let Some(p) = fallback {
                    sinks = audio_inputs.iter().filter(|x| x.contains(p.as_str())).collect();
                }
            }
            match sinks.as_slice() {
                [] => eprintln!("  no output found ({prefix:?}): connect {} / {} in the graph", names.left, names.right),
                [one] => {
                    running.connect(&names.left, one)?;
                    running.connect(&names.right, one)?;
                    eprintln!("  audio → {one}");
                }
                [a, b, ..] => {
                    running.connect(&names.left, a)?;
                    running.connect(&names.right, b)?;
                    eprintln!("  audio → {a}, {b}");
                }
            }
            if let Some(want) = &args.midi_out {
                let targets = n_inputs_matching(&running, want);
                if targets.is_empty() {
                    eprintln!("  no MIDI input matching {want:?} for the mirror");
                }
                for t in targets {
                    running.connect(&names.midi_out, &t)?;
                    eprintln!("  mirror → {t}");
                }
            }
            // Keyboard: --midi, else the first hardware port (ALSA bridge).
            if listen && !args.demo {
                let src = match &args.midi {
                    Some(want) => midi_sources.iter().find(|p| p.contains(want.as_str())),
                    None => midi_sources.iter().find(|p| p.starts_with("Midi-Bridge:") && !p.contains("Through")),
                };
                match src {
                    Some(s) => {
                        running.connect(s, &names.midi_in)?;
                        eprintln!("  keyboard: {s}");
                    }
                    None => eprintln!("  no MIDI keyboard found: connect one to {} in the graph", names.midi_in),
                }
            }
            _node = Some(running);
            eprintln!("playing. Ctrl-C to quit.");
        }
        Backend::Cpal { device, config } => {
            let channels = config.channels() as usize;
            let mut player = player;
            let stream_config = cpal::StreamConfig {
                channels: config.channels(),
                sample_rate: config.sample_rate(),
                buffer_size: cpal::BufferSize::Fixed(args.buffer),
            };
            let stream = device.build_output_stream(
                stream_config,
                move |data: &mut [f32], _| {
                    let frames = data.len() / channels;
                    player.render(
                        frames,
                        std::iter::empty(),
                        |off, l, r| {
                            for (i, f) in data[off * channels..(off + l.len()) * channels].chunks_mut(channels).enumerate() {
                                f[0] = gain * l[i];
                                if channels > 1 {
                                    f[1] = gain * r[i];
                                }
                                for x in f.iter_mut().skip(2) {
                                    *x = 0.0;
                                }
                            }
                        },
                        |_, _| {},
                    );
                },
                |e| eprintln!("audio error: {e}"),
                None,
            )?;
            stream.play()?;
            _stream = Some(stream);
            eprintln!("playing ({} frames/buffer, {channels} channels). Ctrl-C to quit.", args.buffer);
        }
    }

    let sup = Supervisor { control: control.clone(), watch, midi_map, stats, pieces, end_s, sr, repaint: Default::default() };
    if !args.ui {
        sup.run(true);
        return Ok(());
    }
    let repaint = sup.repaint.clone();
    // Keeps polling (MIDI learn, preset file) while the window is hidden.
    std::thread::spawn(move || sup.run(false));
    let presets = args.presets.clone().or_else(|| args.preset.as_ref().and_then(|p| p.parent()).map(Path::to_path_buf));
    let presets = presets.or_else(|| Some(PathBuf::from("presets")).filter(|p| p.is_dir()));
    let preset = args.preset.as_ref().and_then(|p| p.file_stem()).map(|s| s.to_string_lossy().into_owned());
    window::run(control, repaint, presets, preset, &format!("Piano — {}", args.name))?;
    Ok(())
}

/// The non-real-time side while playing: MIDI learn and rebuild edits
/// (controller polling), the preset file, the MIDI map file, the end of the
/// files, and (without a window) a load report every second.
struct Supervisor {
    control: Arc<Mutex<Controller>>,
    watch: Option<PresetWatch>,
    midi_map: MidiMapFile,
    stats: Arc<Stats>,
    pieces: Vec<(f64, String, f64)>,
    end_s: f64,
    sr: f64,
    repaint: window::Repaint,
}

impl Supervisor {
    /// `report`: print the load and stop at the end of the files (no window;
    /// the window shows the load and reads the controller's status itself).
    fn run(mut self, report: bool) {
        let mut next_piece = 0;
        let mut last_report = Instant::now();
        let mut was_rebuilding = false;
        let mut seen = String::new();
        loop {
            // Every 50 ms; the report every second.
            std::thread::sleep(Duration::from_millis(50));
            {
                let mut control = self.control.lock().unwrap();
                control.poll();
                if let Some(w) = self.watch.as_mut()
                    && let Some(m) = w.changed()
                {
                    eprintln!("preset changed: rebuilding the instrument (notes keep playing)");
                    control.load(m);
                }
                if let Some(map) = control.midi().map(|l| l.map().clone()) {
                    self.midi_map.save_if_changed(&map);
                }
                // Controllers move sliders: redraw when the model changed.
                if let Some(ctx) = self.repaint.get() {
                    let now = format!("{:?}", control.model());
                    if now != seen {
                        seen = now;
                        ctx.request_repaint();
                    }
                }
                if report {
                    let st = control.status();
                    if was_rebuilding && !st.rebuilding {
                        match &st.error {
                            Some(e) => eprintln!("rebuild failed: {e}"),
                            None => eprintln!("instrument rebuilt in {:.0} ms", st.last_build.unwrap_or_default().as_secs_f64() * 1e3),
                        }
                    }
                    was_rebuilding = st.rebuilding;
                }
            }
            if !report || last_report.elapsed() < Duration::from_secs(1) {
                continue;
            }
            last_report = Instant::now();
            let now = self.stats.clock.load(Ordering::Relaxed) as f64 / self.sr;
            while next_piece < self.pieces.len() && now >= self.pieces[next_piece].0 {
                eprintln!("▶ {}", self.pieces[next_piece].1);
                next_piece += 1;
            }
            if !self.pieces.is_empty() && now >= self.end_s {
                eprintln!("end of the files");
                return;
            }
            let st = &self.stats;
            let n = st.callbacks.swap(0, Ordering::Relaxed).max(1);
            let avg = st.load_sum.swap(0, Ordering::Relaxed) as f64 / n as f64 / 100.0;
            let max = st.load_max.swap(0, Ordering::Relaxed) as f64 / 100.0;
            eprintln!("CPU load avg {avg:5.1} %  max {max:5.1} %  voices {:3}", st.voices.load(Ordering::Relaxed));
        }
    }
}

/// The MIDI controller map, `$XDG_CONFIG_HOME/unsampled-piano/midi.toml` (else
/// `~/.config/unsampled-piano/midi.toml`): the player's controllers, kept apart
/// from the instrument presets.
struct MidiMapFile {
    path: Option<PathBuf>,
    saved: MidiMap,
}

impl MidiMapFile {
    fn new() -> Self {
        Self { path: config_dir().map(|d| d.join("midi.toml")), saved: MidiMap::default() }
    }

    fn load(&mut self) -> Option<MidiMap> {
        let text = std::fs::read_to_string(self.path.as_ref()?).ok()?;
        match MidiMap::from_toml(&text, &piano_params::Registry::new()) {
            Ok(map) => {
                self.saved = map.clone();
                Some(map)
            }
            Err(e) => {
                eprintln!("{}: {e}", self.path.as_ref()?.display());
                None
            }
        }
    }

    fn save_if_changed(&mut self, map: &MidiMap) {
        if *map == self.saved {
            return;
        }
        self.saved = map.clone();
        let Some(path) = &self.path else { return };
        let text = format!("# MIDI controllers mapped by unsampled-piano's interface (MIDI learn).\n\n{}", map.to_toml());
        if let Err(e) = path.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|()| std::fs::write(path, text)) {
            eprintln!("{}: {e}", path.display());
        }
    }
}

/// `$XDG_CONFIG_HOME/unsampled-piano`, else `~/.config/unsampled-piano`.
fn config_dir() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .map(|d| d.join("unsampled-piano"))
}

/// The preset file, reloaded when its modification time changes.
struct PresetWatch {
    path: PathBuf,
    stamp: Option<std::time::SystemTime>,
}

impl PresetWatch {
    fn new(path: PathBuf) -> Self {
        let stamp = Self::stamp(&path);
        Self { path, stamp }
    }

    fn stamp(path: &PathBuf) -> Option<std::time::SystemTime> {
        std::fs::metadata(path).and_then(|m| m.modified()).ok()
    }

    /// The new model when the file changed and parses (else reports why).
    fn changed(&mut self) -> Option<piano_core::params::PianoModel> {
        let stamp = Self::stamp(&self.path);
        if stamp == self.stamp {
            return None;
        }
        self.stamp = stamp;
        match piano_host::load_preset(&self.path) {
            Ok(m) => Some(m),
            Err(e) => {
                eprintln!("preset not reloaded: {e}");
                None
            }
        }
    }
}

fn cpal_backend(args: &Args) -> Result<Backend, Box<dyn std::error::Error>> {
    let host = pick_host(args.host.as_deref())?;
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
    let config = device.default_output_config()?;
    if config.sample_format() != cpal::SampleFormat::F32 {
        return Err(format!("unsupported sample format {:?}", config.sample_format()).into());
    }
    Ok(Backend::Cpal { device, config })
}

/// Keyboard through midir (cpal backends): events go to the queue.
fn midir_input(
    midi_in: midir::MidiInput,
    want: Option<&str>,
    mut tx: rtrb::Producer<Event>,
) -> Result<Option<midir::MidiInputConnection<()>>, Box<dyn std::error::Error>> {
    let ports = midi_in.ports();
    let name = |p: &midir::MidiInputPort| midi_in.port_name(p).unwrap_or_default();
    let port = match want {
        Some(w) => ports.iter().find(|p| name(p).contains(w)),
        None => ports.iter().find(|p| !name(p).contains("Through")),
    };
    let Some(p) = port.cloned() else {
        eprintln!("no MIDI input port found (use --list / --midi, or --demo)");
        return Ok(None);
    };
    eprintln!("MIDI input: {}", name(&p));
    let conn = midi_in.connect(
        &p,
        "unsampled-piano-in",
        move |_, msg, _| {
            if let Some(ev) = piano_host::decode_midi(msg) {
                let _ = tx.push(ev);
            }
        },
        (),
    )?;
    Ok(Some(conn))
}

/// Files back to back with a gap: (sample, event) timeline, the pieces
/// (start s, name, length s) and the end time. Pedals are released at the
/// start of each file.
#[allow(clippy::type_complexity)]
fn file_timeline(args: &Args, sr: f64) -> Result<(Vec<(u64, Event)>, Vec<(f64, String, f64)>, f64), String> {
    let mut timeline: Vec<(u64, Event)> = Vec::new();
    let mut pieces = Vec::new();
    let mut t0 = 0.5;
    for (i, f) in args.file.iter().enumerate() {
        let evs = piano_host::read_midi_file(f)?;
        let skip = if i == 0 { args.from } else { 0.0 };
        let len = evs.last().map_or(0.0, |e| e.0) - skip;
        let at = |t: f64| ((t0 + t.max(0.0)) * sr) as u64;
        for cc in [64, 66, 67] {
            timeline.push((at(0.0), Event::Control { cc, value: 0 }));
        }
        for (t, ev) in evs {
            if t < skip {
                // Keep the controllers' state, drop the notes.
                if let Event::Control { .. } = ev {
                    timeline.push((at(0.0), ev));
                }
                continue;
            }
            timeline.push((at(t - skip), ev));
        }
        let name = f.file_stem().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        pieces.push((t0, name, len));
        t0 += len + GAP_S;
    }
    Ok((timeline, pieces, t0))
}

/// Personal defaults from `$XDG_CONFIG_HOME/unsampled-piano/play.toml` (else
/// `~/.config/unsampled-piano/play.toml`), used for what the command line leaves out:
/// `device`, `midi`, `midi_out`, `name`, `preset`, `presets` (strings),
/// `buffer`, `gain_db` (numbers), `ui` (boolean).
fn apply_user_config(args: &mut Args) {
    let Some(path) = config_dir().map(|d| d.join("play.toml")) else { return };
    let Ok(text) = std::fs::read_to_string(&path) else { return };
    let table: toml::Table = match text.parse() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{}: {e}", path.display());
            return;
        }
    };
    let cli: Vec<String> = std::env::args().collect();
    let given = |flag: &str| cli.iter().any(|a| a == flag || a.starts_with(&format!("{flag}=")));
    let string = |k: &str| table.get(k).and_then(|v| v.as_str()).map(str::to_string);
    let number = |k: &str| table.get(k).and_then(|v| v.as_float().or(v.as_integer().map(|i| i as f64)));
    if !given("--device") {
        args.device = args.device.take().or(string("device"));
    }
    if !given("--midi") {
        args.midi = args.midi.take().or(string("midi"));
    }
    if !given("--midi-out") {
        args.midi_out = args.midi_out.take().or(string("midi_out"));
    }
    if !given("--preset") {
        args.preset = args.preset.take().or(string("preset").map(PathBuf::from));
    }
    if let (false, Some(n)) = (given("--name"), string("name")) {
        args.name = n;
    }
    if let (false, Some(b)) = (given("--buffer"), number("buffer")) {
        args.buffer = b as u32;
    }
    if let (false, Some(g)) = (given("--gain-db"), number("gain_db")) {
        args.gain_db = g as f32;
    }
    if !given("--presets") {
        args.presets = args.presets.take().or(string("presets").map(PathBuf::from));
    }
    if table.get("ui").and_then(|v| v.as_bool()) == Some(true) {
        args.ui = true;
    }
    eprintln!("defaults from {}", path.display());
}

/// MIDI input ports of the graph containing `want`.
fn n_inputs_matching(running: &node::Running, want: &str) -> Vec<String> {
    running.midi_inputs().into_iter().filter(|p| p.contains(want)).collect()
}

/// JACK port prefix (node description, else name) of PipeWire's default
/// audio sink, from `pw-metadata` and `pw-cli`.
fn default_sink_prefix() -> Option<String> {
    let run = |cmd: &str, args: &[&str]| {
        let out = std::process::Command::new(cmd).args(args).output().ok()?;
        String::from_utf8(out.stdout).ok()
    };
    let meta = run("pw-metadata", &["0", "default.audio.sink"])?;
    let name = meta.split("\"name\":\"").nth(1)?.split('"').next()?.to_string();
    let nodes = run("pw-cli", &["ls", "Node"]).unwrap_or_default();
    let quoted = format!("node.name = \"{name}\"");
    let desc = nodes.split("\tid ").find(|b| b.contains(&quoted)).and_then(|b| {
        let line = b.lines().find(|l| l.trim_start().starts_with("node.description = "))?;
        Some(line.split('"').nth(1)?.to_string())
    });
    Some(desc.unwrap_or(name))
}

/// h:mm:ss
fn clock_str(t: f64) -> String {
    let s = t as u64;
    format!("{}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
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
