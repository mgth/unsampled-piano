//! Real-time player. On Linux the piano is by default a node of the
//! PipeWire graph (through PipeWire's JACK API): stereo audio out, a MIDI
//! input with sample-accurate timing, and a MIDI output mirroring every
//! event the piano plays; the keyboard and the speakers are connected
//! automatically. Elsewhere, and with `--host`, it plays through cpal
//! (WASAPI or ASIO on Windows, CoreAudio on macOS; PipeWire, JACK or ALSA
//! on Linux), with the keyboard read through midir and played at the start
//! of each buffer. MIDI files play from a timeline owned by the audio
//! thread, sample-accurately. The instrument is built on the main thread;
//! the audio thread neither allocates nor locks. CPU load is reported once
//! per second. With the window, its "Audio and MIDI" page changes the
//! set-up while playing (see audio.rs).

// Windows: no console window when started from the Explorer (see console.rs).
#![cfg_attr(windows, windows_subsystem = "windows")]

mod audio;
mod blackbox;
mod console;
mod node;
mod player;
mod settings;
mod window;

use audio::{Audio, Queue, Settings, Start};
use clap::Parser;
use cpal::traits::HostTrait;
use piano_core::Event;
use piano_control::Controller;
use piano_params::midi::MidiMap;
use piano_params::PianoControl;
use player::Stats;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Parser)]
#[command(name = "unsampled-piano", about = "Play the physical piano from a MIDI keyboard or MIDI files")]
struct Args {
    /// Preset (TOML); see `unsampled-piano-render dump-preset`. A listening
    /// set-up or a touch (under `listening/` or `touch/`) is laid over the
    /// last sound; any other replaces it.
    #[arg(long)]
    preset: Option<PathBuf>,
    /// MIDI input port to connect: substring of its name (default: the
    /// first hardware port).
    #[arg(long)]
    midi: Option<String>,
    /// Audio backend: node (Linux default: the piano as a graph node with
    /// MIDI in and out, through PipeWire's JACK API), or a cpal host without
    /// MIDI ports: pipewire, jack, alsa (Linux), wasapi (Windows default),
    /// asio (Windows, built with the `asio` feature), coreaudio (macOS
    /// default).
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
    /// Audio buffer size in frames (node: requested graph period). Default
    /// 256; with ASIO, the size set in the driver's control panel.
    #[arg(long)]
    buffer: Option<u32>,
    /// First channel (from 1) of the stereo pair on a multichannel output
    /// (cpal backends).
    #[arg(long, default_value_t = 1)]
    output_channel: u16,
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
    /// Preset folder of the interface, where it saves (default: the
    /// --preset file's, else ./presets, else `presets` in the configuration
    /// directory); the shipped presets are listed after it.
    #[arg(long)]
    presets: Option<PathBuf>,
    /// Serve the instrument to AI assistants (MCP over HTTP: parameters,
    /// presets, status) on this address, 127.0.0.1:7426 when no value is
    /// given; the endpoint is http://ADDR/mcp.
    #[arg(long, value_name = "ADDR", num_args = 0..=1, default_missing_value = piano_mcp::DEFAULT_ADDR)]
    mcp: Option<String>,
    /// Threads rendering the voices (default 1, the audio thread alone);
    /// the sound is the same whatever the number.
    #[arg(long)]
    threads: Option<usize>,
    /// CPU quality: overkill, full (default), balanced or light (see
    /// docs/performance.md).
    #[arg(long)]
    quality: Option<String>,
    /// From play.toml only: start with the last sound (default true).
    #[arg(skip)]
    restore: Option<bool>,
    /// Listen to this MIDI channel only (1–16; default: every channel).
    #[arg(long, value_parser = clap::value_parser!(u8).range(1..=16))]
    midi_channel: Option<u8>,
    /// The keyboard's sustain pedal works the other way round (sounds held
    /// when it is up).
    #[arg(long)]
    invert_sustain: bool,
    /// Compute the piano at the output's rate even above 48 kHz. By default
    /// an output at 88.2–384 kHz gets the piano computed at 44.1/48 kHz and
    /// upsampled, for half to an eighth of the load.
    #[arg(long)]
    full_rate: bool,
    /// Flight recorder: keep every event the piano receives and the last
    /// minute of its sound, written into the configuration directory's
    /// `blackbox` folder when the player stops (`unsampled-piano-render
    /// replay` renders it again, identically). `blackbox = true` in
    /// play.toml.
    #[arg(long)]
    blackbox: bool,
    /// From play.toml only: the interface's theme, "dark", "light" or
    /// "system" (default).
    #[arg(skip)]
    theme: Option<String>,
}

/// Silence between two files (s), and after the last one before quitting.
const GAP_S: f64 = 4.0;

fn main() {
    let terminal = console::attach();
    let mut args = Args::parse();
    // Started from the Explorer or the Finder (no terminal): the window.
    if !terminal {
        args.ui = true;
    }
    if let Err(e) = run(args) {
        console::fatal(&e.to_string(), terminal);
        std::process::exit(1);
    }
}

fn run(mut args: Args) -> Result<(), Box<dyn std::error::Error>> {
    apply_user_config(&mut args);
    if args.list {
        return list(&args);
    }

    // The last sound (unless switched off), with a listening set-up or a
    // touch asked for laid over it; another preset replaces it.
    let last = LastSound::new();
    let model = match &args.preset {
        Some(p) if !piano_host::is_overlay(p) => piano_host::load_preset(p)?,
        p => {
            let base = last.load(restore_last(&args)).unwrap_or_default();
            match p {
                Some(p) => piano_host::load_listening(p, &base)?,
                None => base,
            }
        }
    };
    let quality = match args.quality.as_deref() {
        Some(q) => piano_core::Quality::from_name(q).ok_or_else(|| format!("unknown quality {q:?}: overkill, full, balanced or light"))?,
        None => Default::default(),
    };
    let mut midi_map = MidiMapFile::new();
    let map = midi_map.load();
    if let Some(m) = &map {
        eprintln!("MIDI controllers: {} mapped ({})", m.mappings.len(), midi_map.path.as_ref().map_or(String::new(), |p| p.display().to_string()));
    }
    let settings = Settings {
        host: args.host.clone(),
        device: args.device.clone(),
        buffer: args.buffer,
        output_channel: args.output_channel,
        midi: args.midi.clone(),
        midi_out: args.midi_out.clone(),
        threads: args.threads.unwrap_or(1).max(1),
        quality,
        midi_channel: args.midi_channel,
        invert_sustain: args.invert_sustain,
        full_rate: args.full_rate,
        blackbox: args.blackbox,
    };
    let stats = Arc::new(Stats::default());
    // The keyboard, unless files play alone.
    let listen = args.file.is_empty() || args.midi.is_some();
    let (mut pieces, mut end_s) = (Vec::new(), 0.0);
    let (audio, control) = Audio::start(settings, &args.name, args.gain_db, stats.clone(), model, map, |sr| {
        let (timeline, p, e) = file_timeline(&args, sr)?;
        (pieces, end_s) = (p, e);
        let demo: Option<Box<dyn FnOnce(Queue) + Send>> = if args.demo { Some(Box::new(demo)) } else { None };
        Ok(Start { timeline, listen, demo })
    })?;
    let sr = audio.sample_rate;
    eprintln!("instrument built at {} Hz in {:.0} ms", audio.engine_rate, control.status().last_build.unwrap_or_default().as_secs_f64() * 1e3);
    for (start, name, len) in &pieces {
        eprintln!("  {:>7}  {name} ({}:{:02})", clock_str(*start), (*len as u64) / 60, (*len as u64) % 60);
    }
    let control = Arc::new(Mutex::new(control));
    let watch = args.preset.clone().map(PresetWatch::new);

    // Saved presets: --presets, the --preset file's folder, ./presets (a
    // checkout), else the configuration directory's.
    let presets = args.presets.clone().or_else(|| args.preset.as_ref().and_then(|p| p.parent()).map(Path::to_path_buf));
    let presets = presets.or_else(|| Some(PathBuf::from("presets")).filter(|p| p.is_dir()));
    let presets = presets.or_else(|| piano_host::config_dir().map(|d| d.join("presets")));
    let mut mcp_url = None;
    if let Some(addr) = &args.mcp {
        match piano_mcp::serve(addr, control.clone(), presets.clone()) {
            Ok(url) => {
                eprintln!("MCP server: {url}");
                mcp_url = Some(url);
            }
            Err(e) => eprintln!("{e}"),
        }
    }

    let sup = Supervisor { control: control.clone(), watch, midi_map, last, stats, pieces, end_s, sr, repaint: Default::default(), doubled_told: 0 };
    let blackbox = audio.blackbox.clone();
    if args.blackbox {
        dump_on_interrupt(blackbox.clone());
    }
    if !args.ui {
        sup.run(true);
        drop(audio);
        dump(&blackbox);
        return Ok(());
    }
    let repaint = sup.repaint.clone();
    // Keeps polling (MIDI learn, preset file) while the window is hidden.
    std::thread::spawn(move || sup.run(false));
    // Shipped presets, listed after: next to the program (zip), in the
    // macOS bundle's resources, or where an installer put them.
    let exe_dir = std::env::current_exe().ok().and_then(|e| e.parent().map(Path::to_path_buf));
    let beside = exe_dir.into_iter().flat_map(|d| [d.join("presets"), d.join("../Resources/presets")]).filter(|p| p.is_dir());
    let shipped: Vec<PathBuf> = beside.chain(piano_host::installed_preset_dirs()).collect();
    let preset = args.preset.as_ref().and_then(|p| p.file_stem()).map(|s| s.to_string_lossy().into_owned());
    let title = if args.name == "unsampled-piano" { "Unsampled Piano".to_string() } else { format!("Unsampled Piano — {}", args.name) };
    let app = window::AppSettings {
        file: piano_host::config_dir().map(|d| d.join("play.toml")),
        mcp_url,
        restore: restore_last(&args),
        theme: args.theme.clone().unwrap_or_else(|| "system".into()),
    };
    window::run(control, repaint, presets, shipped, preset, &title, audio, app)?;
    dump(&blackbox);
    Ok(())
}

/// Writes the flight recorder (crate::blackbox) into the configuration
/// directory's `blackbox` folder.
fn dump(b: &blackbox::BlackBox) {
    let Some(root) = piano_host::config_dir().map(|d| d.join("blackbox")) else { return };
    match b.dump(&root) {
        Ok(Some(dir)) => eprintln!("flight recorder: {}", dir.display()),
        Ok(None) => {}
        Err(e) => eprintln!("flight recorder: {e}"),
    }
}

/// Ctrl-C (or SIGTERM): the flight recorder is written before quitting.
fn dump_on_interrupt(b: Arc<blackbox::BlackBox>) {
    #[cfg(unix)]
    {
        use std::sync::atomic::AtomicBool;
        static STOP: AtomicBool = AtomicBool::new(false);
        extern "C" fn on_signal(_: libc::c_int) {
            STOP.store(true, Ordering::SeqCst);
        }
        // SAFETY: the handler only stores an atomic.
        unsafe {
            libc::signal(libc::SIGINT, on_signal as *const () as libc::sighandler_t);
            libc::signal(libc::SIGTERM, on_signal as *const () as libc::sighandler_t);
        }
        std::thread::spawn(move || {
            while !STOP.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_millis(50));
            }
            dump(&b);
            std::process::exit(130);
        });
    }
    #[cfg(not(unix))]
    let _ = b;
}

/// `--list`: the audio devices, MIDI ports and graph ports.
fn list(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    // Every host, except on Linux where the default one is enough.
    let hosts = match args.host.as_deref().filter(|h| *h != "node") {
        None if !cfg!(target_os = "linux") => cpal::available_hosts().into_iter().filter_map(|id| cpal::host_from_id(id).ok()).collect(),
        want => vec![audio::pick_host(want)?],
    };
    for host in hosts {
        println!("audio host: {:?}", host.id());
        match host.output_devices() {
            Ok(devices) => {
                for d in devices {
                    println!("  output: {}", audio::describe(&d));
                }
            }
            Err(e) => println!("  ({e})"),
        }
    }
    let midi_in = midir::MidiInput::new("unsampled-piano")?;
    for p in midi_in.ports() {
        println!("  MIDI in: {}", midi_in.port_name(&p).unwrap_or_default());
    }
    if let Ok(n) = node::Node::open(&args.name, args.buffer.unwrap_or(audio::BUFFER)) {
        let (midi, audio_in) = n.graph_ports();
        println!("graph (node backend), default sink: {:?}", audio::default_sink_prefix());
        for p in midi {
            println!("  MIDI source: {p}");
        }
        for p in audio_in {
            println!("  audio input: {p}");
        }
    }
    Ok(())
}

/// The non-real-time side while playing: MIDI learn and rebuild edits
/// (controller polling), the preset file, the MIDI map file, the end of the
/// files, and (without a window) a load report every second.
struct Supervisor {
    control: Arc<Mutex<Controller>>,
    watch: Option<PresetWatch>,
    midi_map: MidiMapFile,
    last: LastSound,
    stats: Arc<Stats>,
    pieces: Vec<(f64, String, f64)>,
    end_s: f64,
    sr: f64,
    repaint: window::Repaint,
    /// Doubled MIDI notes already reported.
    doubled_told: u32,
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
                    && let Some(m) = w.changed(&control.model())
                {
                    eprintln!("preset changed: rebuilding the instrument (notes keep playing)");
                    control.load(m);
                }
                if let Some(map) = control.midi().map(|l| l.map().clone()) {
                    self.midi_map.save_if_changed(&map);
                }
                self.last.save_when_settled(&control.model());
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
            {
                let doubled = self.control.lock().unwrap().status().doubled_notes;
                if doubled > 0 && self.doubled_told == 0 {
                    eprintln!("doubled MIDI notes: the keyboard reaches the piano twice (two channels or two connections); the doubles are ignored (--midi-channel to listen to one)");
                }
                self.doubled_told = doubled;
            }
            let st = &self.stats;
            let n = st.callbacks.swap(0, Ordering::Relaxed).max(1);
            let avg = st.load_sum.swap(0, Ordering::Relaxed) as f64 / n as f64 / 100.0;
            let max = st.load_max.swap(0, Ordering::Relaxed) as f64 / 100.0;
            eprintln!("CPU load avg {avg:5.1} %  max {max:5.1} %  voices {:3}", st.voices.load(Ordering::Relaxed));
        }
    }
}

/// The MIDI controller map, `midi.toml` in the configuration directory
/// (`piano_host::config_dir`): the player's controllers, kept apart from the
/// instrument presets.
struct MidiMapFile {
    path: Option<PathBuf>,
    saved: MidiMap,
}

impl MidiMapFile {
    fn new() -> Self {
        Self { path: piano_host::config_dir().map(|d| d.join("midi.toml")), saved: MidiMap::default() }
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

/// The sound the player had last, `last.toml` in the configuration
/// directory: saved once it has not changed for 2 s, loaded at start
/// (`restore` in play.toml, on by default) unless the preset given
/// replaces it.
struct LastSound {
    path: Option<PathBuf>,
    /// What the file holds, and what changed since (when).
    saved: String,
    pending: Option<(String, Instant)>,
}

impl LastSound {
    fn new() -> Self {
        Self { path: piano_host::config_dir().map(|d| d.join("last.toml")), saved: String::new(), pending: None }
    }

    fn load(&self, wanted: bool) -> Option<piano_core::params::PianoModel> {
        let path = self.path.as_ref().filter(|p| wanted && p.is_file())?;
        match piano_host::load_preset(path) {
            Ok(m) => {
                eprintln!("last sound: {}", path.display());
                Some(m)
            }
            Err(e) => {
                eprintln!("{e}");
                None
            }
        }
    }

    fn save_when_settled(&mut self, model: &piano_core::params::PianoModel) {
        let Some(path) = &self.path else { return };
        let text = piano_host::preset_toml(model);
        if text == self.saved {
            self.pending = None;
            return;
        }
        match &self.pending {
            Some((t, since)) if *t == text => {
                if since.elapsed() >= Duration::from_secs(2) {
                    let written = path.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|()| std::fs::write(path, &text));
                    if let Err(e) = written {
                        eprintln!("{}: {e}", path.display());
                    }
                    self.saved = text;
                    self.pending = None;
                }
            }
            _ => self.pending = Some((text, Instant::now())),
        }
    }
}

/// Whether to start with the last sound: `restore` in play.toml, on by
/// default.
fn restore_last(args: &Args) -> bool {
    args.restore.unwrap_or(true)
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

    /// The new model when the file changed and parses (else reports why):
    /// a listening set-up or a touch over `current`.
    fn changed(&mut self, current: &piano_core::params::PianoModel) -> Option<piano_core::params::PianoModel> {
        let stamp = Self::stamp(&self.path);
        if stamp == self.stamp {
            return None;
        }
        self.stamp = stamp;
        match piano_host::load_over(&self.path, current) {
            Ok(m) => Some(m),
            Err(e) => {
                eprintln!("preset not reloaded: {e}");
                None
            }
        }
    }
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

/// Personal defaults from `play.toml` in the configuration directory
/// (`piano_host::config_dir`), used for what the command line leaves out:
/// `host`, `device`, `midi`, `midi_out`, `name`, `preset`, `presets`
/// (strings), `buffer`, `output_channel`, `gain_db` (numbers), `ui`
/// (boolean), `mcp` (true for the default address, or the address as a
/// string).
fn apply_user_config(args: &mut Args) {
    let Some(path) = piano_host::config_dir().map(|d| d.join("play.toml")) else { return };
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
    if !given("--host") {
        args.host = args.host.take().or(string("host"));
    }
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
        args.buffer = Some(b as u32);
    }
    if let (false, Some(c)) = (given("--output-channel"), number("output_channel")) {
        args.output_channel = c as u16;
    }
    if let (false, Some(g)) = (given("--gain-db"), number("gain_db")) {
        args.gain_db = g as f32;
    }
    if let (false, Some(t)) = (given("--threads"), number("threads")) {
        args.threads = Some(t as usize);
    }
    if !given("--quality") {
        args.quality = args.quality.take().or(string("quality"));
    }
    args.restore = table.get("restore").and_then(|v| v.as_bool());
    args.theme = string("theme");
    if let (false, Some(c)) = (given("--midi-channel"), number("midi_channel")) {
        args.midi_channel = Some(c as u8).filter(|c| (1..=16).contains(c));
    }
    if !given("--invert-sustain") && table.get("invert_sustain").and_then(|v| v.as_bool()) == Some(true) {
        args.invert_sustain = true;
    }
    if !given("--full-rate") && table.get("full_rate").and_then(|v| v.as_bool()) == Some(true) {
        args.full_rate = true;
    }
    if !given("--blackbox") && table.get("blackbox").and_then(|v| v.as_bool()) == Some(true) {
        args.blackbox = true;
    }
    if !given("--presets") {
        args.presets = args.presets.take().or(string("presets").map(PathBuf::from));
    }
    if table.get("ui").and_then(|v| v.as_bool()) == Some(true) {
        args.ui = true;
    }
    if !given("--mcp") {
        args.mcp = match table.get("mcp") {
            Some(toml::Value::Boolean(true)) => Some(piano_mcp::DEFAULT_ADDR.into()),
            Some(toml::Value::String(a)) => Some(a.clone()),
            _ => args.mcp.take(),
        };
    }
    eprintln!("defaults from {}", path.display());
}

/// h:mm:ss
fn clock_str(t: f64) -> String {
    let s = t as u64;
    format!("{}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
}

/// A short built-in sequence (no keyboard needed): chords with pedal.
fn demo(mut tx: Queue) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_last_sound_is_saved_once_settled() {
        let dir = std::env::temp_dir().join(format!("piano-play-last-{}", std::process::id()));
        let path = dir.join("last.toml");
        let mut last = LastSound { path: Some(path.clone()), saved: String::new(), pending: None };
        let model = piano_core::params::PianoModel { output_gain_db: -7.5, ..Default::default() };
        // Still changing: nothing written yet.
        last.save_when_settled(&model);
        assert!(!path.exists());
        // Unchanged for 2 s: written, and read back at the next start.
        let since = &mut last.pending.as_mut().unwrap().1;
        *since = since.checked_sub(Duration::from_secs(3)).unwrap();
        last.save_when_settled(&model);
        assert!(path.exists());
        assert_eq!(last.load(true).map(|m| m.output_gain_db), Some(-7.5));
        assert!(last.load(false).is_none());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
