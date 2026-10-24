//! The audio and MIDI set-up: which backend plays, on which output, with
//! which buffer, from which MIDI input. Started once from the command line
//! and `play.toml`, then changed while playing from the interface's
//! settings page:
//! - on the graph node, outputs and MIDI ports are reconnected in place;
//! - otherwise (another backend, a cpal device, a buffer size) the audio
//!   stops and starts again with a new engine built at the new sample rate
//!   from the current model, swapped into the shared controller.
//!
//! Lives on the main thread: a cpal stream may not leave it.

use crate::node;
use crate::player::{rate_factor, KeyboardInput, Player, Stats, MAX_BLOCK};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use piano_control::Controller;
use piano_core::params::PianoModel;
use piano_core::{EngineConfig, Event, Quality};
use piano_params::midi::MidiMap;
use piano_params::PianoControl;
use std::sync::{Arc, Mutex};

/// Buffer size when none is given (frames).
pub const BUFFER: u32 = 256;
/// Buffer sizes offered by the settings page.
pub const BUFFERS: [u32; 6] = [64, 128, 256, 512, 1024, 2048];

/// What the user chose (None: automatic).
#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    /// "node", or a cpal host (lowercase: "pipewire", "alsa", "wasapi"…).
    pub host: Option<String>,
    /// Output: substring of the device (cpal) or of the playback port names
    /// (node).
    pub device: Option<String>,
    pub buffer: Option<u32>,
    /// First channel (from 1) of the stereo pair (cpal).
    pub output_channel: u16,
    /// MIDI input: substring of the port name (None: the first keyboard).
    pub midi: Option<String>,
    /// Graph MIDI inputs the mirror output feeds (node).
    pub midi_out: Option<String>,
    /// Threads rendering the voices (1: the audio thread alone).
    pub threads: usize,
    pub quality: Quality,
    /// The keyboard's channel (1–16; None: every channel).
    pub midi_channel: Option<u8>,
    /// The keyboard's sustain pedal works the other way round.
    pub invert_sustain: bool,
    /// Compute the piano at the output's rate even above 48 kHz (otherwise
    /// at half, a quarter or an eighth of it, then upsampled).
    pub full_rate: bool,
}

impl Settings {
    /// The same, keyboard filter aside (what the filter changes needs no
    /// restart).
    fn without_keyboard(&self) -> Self {
        Self { midi_channel: None, invert_sustain: false, ..self.clone() }
    }
}

/// What may be chosen now, and what is connected.
#[derive(Clone, Debug, Default)]
pub struct Choices {
    /// (id, label) of the backends.
    pub backends: Vec<(String, String)>,
    pub outputs: Vec<String>,
    pub midi_inputs: Vec<String>,
    /// Graph MIDI inputs for the mirror (node only).
    pub mirror_targets: Vec<String>,
    /// Channels of the cpal output device.
    pub channels: u16,
    /// The backend running ("node" or the cpal host's id).
    pub backend: String,
    /// What the automatic choices resolved to.
    pub output_now: Option<String>,
    pub midi_now: Option<String>,
    /// "48000 Hz · 256 frames", or why nothing plays.
    pub info: String,
}

/// Where the audio goes, once opened.
enum Backend {
    Node(node::Node),
    Cpal { host: cpal::HostId, device: cpal::Device, config: cpal::SupportedStreamConfig },
}

/// What runs (one at a time: the size difference costs nothing).
#[allow(clippy::large_enum_variant)]
enum Live {
    Node { running: node::Running, names: node::PortNames, period: u32 },
    Cpal { _stream: cpal::Stream, _midi: Option<midir::MidiInputConnection<()>>, host: cpal::HostId, device: cpal::Device, frames: String },
}

/// Events from other threads (keyboard through midir, demo), played at the
/// start of the next buffer.
pub type Queue = rtrb::Producer<Event>;

pub struct Audio {
    pub settings: Settings,
    live: Option<Live>,
    /// The output's rate, and the engine's (lower above 48 kHz).
    pub sample_rate: f64,
    pub engine_rate: f64,
    name: String,
    gain: f32,
    stats: Arc<Stats>,
    output_now: Option<String>,
    midi_now: Option<String>,
    keyboard: Arc<KeyboardInput>,
}

/// What to play besides the keyboard at the start.
pub struct Start {
    pub timeline: Vec<(u64, Event)>,
    /// Listen to a MIDI keyboard.
    pub listen: bool,
    /// Feed the queue from elsewhere (the demo) instead of midir.
    pub demo: Option<Box<dyn FnOnce(Queue) + Send>>,
}


/// Gives a voice worker of the engine the real-time priority the audio
/// thread has: a worker preempted by the OS makes the audio callback wait
/// for it. SCHED_FIFO at PipeWire's priority (88) within RLIMIT_RTPRIO on
/// Linux; MMCSS "Pro Audio" on Windows, a time-constraint policy on macOS.
fn promote_worker(max_block: usize, sample_rate: f64) {
    #[cfg(target_os = "linux")]
    {
        let _ = (max_block, sample_rate);
        // SAFETY: plain libc calls on the current thread.
        unsafe {
            let mut lim = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
            let max = if libc::getrlimit(libc::RLIMIT_RTPRIO, &mut lim) == 0 { lim.rlim_cur.min(88) as i32 } else { 0 };
            let param = libc::sched_param { sched_priority: max };
            if max < 1 || libc::pthread_setschedparam(libc::pthread_self(), libc::SCHED_FIFO, &param) != 0 {
                eprintln!("voice worker: no real-time priority (RLIMIT_RTPRIO {max})");
            }
        }
    }
    #[cfg(not(target_os = "linux"))]
    match audio_thread_priority::promote_current_thread_to_real_time(max_block as u32, sample_rate as u32) {
        // Kept for the thread's lifetime.
        Ok(handle) => std::mem::forget(handle),
        Err(e) => eprintln!("voice worker: no real-time priority ({e})"),
    }
}

impl Audio {
    /// Starts playing `model`; returns the controller of its engine.
    /// `timeline` needs the sample rate, so it is built from it.
    pub fn start(
        settings: Settings,
        name: &str,
        gain_db: f32,
        stats: Arc<Stats>,
        model: PianoModel,
        map: Option<MidiMap>,
        start: impl FnOnce(f64) -> Result<Start, String>,
    ) -> Result<(Self, Controller), String> {
        let backend = open(&settings, name)?;
        let sample_rate = backend_rate(&backend);
        let start = start(sample_rate)?;
        let keyboard = Arc::new(KeyboardInput::default());
        keyboard.set(settings.midi_channel, settings.invert_sustain);
        let mut audio = Self {
            settings,
            live: None,
            sample_rate,
            engine_rate: sample_rate,
            name: name.into(),
            gain: 10f32.powf(gain_db / 20.0),
            stats,
            output_now: None,
            midi_now: None,
            keyboard,
        };
        let control = audio.launch(backend, model, map, start)?;
        Ok((audio, control))
    }

    /// Changes the set-up while playing. The node reconnects in place when
    /// only its connections change; otherwise the audio restarts with a new
    /// controller (built from the current model and controllers) swapped
    /// into `control`. On failure the previous set-up comes back.
    pub fn apply(&mut self, new: Settings, control: &Arc<Mutex<Controller>>) -> Result<(), String> {
        if new == self.settings {
            return Ok(());
        }
        // The keyboard's channel and pedal: at once, while playing.
        self.keyboard.set(new.midi_channel, new.invert_sustain);
        if new.without_keyboard() == self.settings.without_keyboard() {
            self.settings = new;
            return Ok(());
        }
        // Still the graph node and the same engine: only its connections
        // change (its period was requested when it started: a new buffer
        // size waits for the next start).
        let same_engine =
            new.threads == self.settings.threads && new.quality == self.settings.quality && new.full_rate == self.settings.full_rate;
        let node_still = same_engine && new.host.as_deref().unwrap_or(default_backend()) == "node";
        if let (true, Some(Live::Node { .. })) = (node_still, &self.live) {
            self.settings = new;
            self.connect_node();
            return Ok(());
        }
        let old = std::mem::replace(&mut self.settings, new);
        match self.restart(control) {
            Ok(()) => Ok(()),
            Err(e) => {
                self.settings = old;
                let back = self.restart(control);
                Err(match back {
                    Ok(()) => format!("{e}; back to the previous set-up"),
                    Err(e2) => format!("{e}; the previous set-up failed too: {e2}"),
                })
            }
        }
    }

    fn restart(&mut self, control: &Arc<Mutex<Controller>>) -> Result<(), String> {
        let (model, map) = {
            let mut c = control.lock().unwrap();
            (c.model(), c.midi().map(|l| l.map().clone()))
        };
        // The device must be free before it opens again.
        self.live = None;
        let backend = open(&self.settings, &self.name)?;
        self.sample_rate = backend_rate(&backend);
        let start = Start { timeline: Vec::new(), listen: true, demo: None };
        let new = self.launch(backend, model, map, start)?;
        // Inside the mutex: the interface, the supervisor and the MCP server
        // hold this Arc.
        *control.lock().unwrap() = new;
        Ok(())
    }

    /// Builds the engine at the backend's rate (or a fraction of it, see
    /// [`rate_factor`]), starts the backend and connects it.
    fn launch(&mut self, backend: Backend, model: PianoModel, map: Option<MidiMap>, start: Start) -> Result<Controller, String> {
        let sr = backend_rate(&backend);
        // Overkill computes at the output's rate too.
        let factor = rate_factor(sr, self.settings.full_rate || self.settings.quality == Quality::Overkill);
        self.engine_rate = sr / factor as f64;
        let cfg = EngineConfig {
            sample_rate: self.engine_rate,
            max_block: MAX_BLOCK,
            model,
            threads: self.settings.threads.max(1),
            worker_init: Some(promote_worker),
            quality: self.settings.quality,
            ..Default::default()
        };
        let (mut control, engine) = piano_control::new(model, cfg);
        if let Some(map) = map {
            control.set_midi_map(map);
        }
        let (tx, rx) = rtrb::RingBuffer::<Event>::new(1024);
        let player = Player::new(engine, sr, factor, start.timeline, Some(rx), self.stats.clone());
        let mut midi_conn = None;
        match start.demo {
            Some(demo) => {
                std::thread::spawn(move || demo(tx));
            }
            None if matches!(backend, Backend::Cpal { .. }) && start.listen => {
                let (conn, port) = midir_input(self.settings.midi.as_deref(), tx, self.keyboard.clone())?;
                midi_conn = conn;
                self.midi_now = port;
            }
            None => {}
        }
        match backend {
            Backend::Node(n) => {
                let period = n.buffer_size();
                let (running, names) = n.start(player, self.gain, self.keyboard.clone()).map_err(|e| e.to_string())?;
                eprintln!("graph node \"{}\" ({period} frames/period)", self.name);
                eprintln!("  MIDI in:  {}", names.midi_in);
                eprintln!("  MIDI out: {} (mirror of what the piano plays)", names.midi_out);
                self.live = Some(Live::Node { running, names, period });
                self.connect_node_with(start.listen);
            }
            Backend::Cpal { host, device, config } => {
                let (stream, frames) = cpal_play(host, &device, &config, &self.settings, player, self.gain)?;
                self.output_now = Some(describe(&device));
                self.live = Some(Live::Cpal { _stream: stream, _midi: midi_conn, host, device, frames });
            }
        }
        match factor {
            1 => eprintln!("playing at {sr} Hz. Ctrl-C to quit."),
            k => eprintln!("playing at {sr} Hz, the piano computed at {} Hz (×{k}; --full-rate to compute at {sr} Hz). Ctrl-C to quit.", self.engine_rate),
        }
        Ok(control)
    }

    fn connect_node(&mut self) {
        self.connect_node_with(true);
    }

    /// (Re)connects the node's output, keyboard and mirror per the settings.
    fn connect_node_with(&mut self, listen: bool) {
        let Some(Live::Node { running, names, .. }) = &self.live else { return };
        let (midi_sources, audio_inputs) = running.graph_ports();
        for port in [&names.left, &names.right, &names.midi_in, &names.midi_out] {
            running.disconnect_all(port);
        }
        // Audio: the device asked for, else the system's default sink.
        let default = || default_sink_prefix().map(|p| format!("{p}:"));
        let matching = |p: &str| audio_inputs.iter().filter(|x| x.contains(p)).cloned().collect::<Vec<_>>();
        let mut sinks = self.settings.device.as_deref().map(matching).unwrap_or_default();
        if sinks.is_empty() {
            if let Some(want) = &self.settings.device {
                eprintln!("  output {want:?} not found, using the default sink");
            }
            sinks = default().map(|p| matching(&p)).unwrap_or_default();
        }
        self.output_now = match sinks.as_slice() {
            [] => {
                eprintln!("  no output found: connect {} / {} in the graph", names.left, names.right);
                None
            }
            [one] => {
                let _ = running.connect(&names.left, one);
                let _ = running.connect(&names.right, one);
                eprintln!("  audio → {one}");
                Some(output_group(one))
            }
            [a, b, ..] => {
                let _ = running.connect(&names.left, a);
                let _ = running.connect(&names.right, b);
                eprintln!("  audio → {a}, {b}");
                Some(output_group(a))
            }
        };
        if let Some(want) = &self.settings.midi_out {
            for t in running.midi_inputs().into_iter().filter(|p| p.contains(want.as_str())) {
                let _ = running.connect(&names.midi_out, &t);
                eprintln!("  mirror → {t}");
            }
        }
        // Keyboard: the port asked for, else the first hardware port.
        self.midi_now = None;
        if listen {
            let src = match &self.settings.midi {
                Some(want) => midi_sources.iter().find(|p| p.contains(want.as_str())),
                None => midi_sources.iter().find(|p| p.starts_with("Midi-Bridge:") && !p.contains("Through")),
            };
            match src {
                Some(s) => {
                    let _ = running.connect(s, &names.midi_in);
                    eprintln!("  keyboard: {s}");
                    self.midi_now = Some(s.clone());
                }
                None => eprintln!("  no MIDI keyboard found: connect one to {} in the graph", names.midi_in),
            }
        }
    }

    /// "96000 Hz (piano at 48000 Hz)", or the one rate.
    fn rates(&self) -> String {
        match self.engine_rate < self.sample_rate {
            true => format!("{} Hz (piano at {} Hz)", self.sample_rate, self.engine_rate),
            false => format!("{} Hz", self.sample_rate),
        }
    }

    /// The lists the settings page offers, and the state now.
    pub fn choices(&self) -> Choices {
        let mut c = Choices { backends: backends(), output_now: self.output_now.clone(), midi_now: self.midi_now.clone(), ..Default::default() };
        match &self.live {
            Some(Live::Node { running, period, .. }) => {
                let (midi, audio) = running.graph_ports();
                c.backend = "node".into();
                c.outputs = dedup(audio.iter().map(|p| output_group(p)));
                c.midi_inputs = midi;
                c.mirror_targets = running.midi_inputs();
                c.info = format!("{} · {period} frames · PipeWire graph", self.rates());
            }
            Some(Live::Cpal { host, device, frames, .. }) => {
                c.backend = host_id(*host);
                if let Ok(h) = cpal::host_from_id(*host) {
                    c.outputs = h.output_devices().map(|d| d.map(|d| describe(&d)).collect()).unwrap_or_default();
                }
                c.channels = device.default_output_config().map(|x| x.channels()).unwrap_or(2);
                c.midi_inputs = midir::MidiInput::new("unsampled-piano-list")
                    .map(|m| m.ports().iter().filter_map(|p| m.port_name(p).ok()).collect())
                    .unwrap_or_default();
                c.info = format!("{} · {frames} frames · {}", self.rates(), host_label(&c.backend));
            }
            None => c.info = "No audio output".into(),
        }
        c
    }
}

/// A node's playback port without its channel ("Speakers:playback_FL" →
/// "Speakers:playback"): the output offered for both channels.
fn output_group(port: &str) -> String {
    match port.rfind([' ', '_']) {
        Some(i) if port[..i].contains(':') => port[..i].to_string(),
        _ => port.to_string(),
    }
}

fn dedup(it: impl Iterator<Item = String>) -> Vec<String> {
    let mut v: Vec<String> = Vec::new();
    for x in it {
        if !v.contains(&x) {
            v.push(x);
        }
    }
    v
}

/// The backends of this system: the graph node on Linux, then cpal's hosts.
fn backends() -> Vec<(String, String)> {
    let mut v = Vec::new();
    if cfg!(target_os = "linux") {
        v.push(("node".to_string(), host_label("node")));
    }
    for id in cpal::available_hosts() {
        let id = host_id(id);
        v.push((id.clone(), host_label(&id)));
    }
    v
}

/// The backend used when none is chosen.
pub fn default_backend() -> &'static str {
    if cfg!(target_os = "linux") { "node" } else { "" }
}

fn host_id(id: cpal::HostId) -> String {
    format!("{id:?}").to_lowercase()
}

pub fn host_label(id: &str) -> String {
    match id {
        "node" => "PipeWire graph (audio and MIDI ports)".into(),
        "pipewire" => "PipeWire".into(),
        "jack" => "JACK".into(),
        "alsa" => "ALSA".into(),
        "wasapi" => "WASAPI".into(),
        "asio" => "ASIO".into(),
        "coreaudio" => "Core Audio".into(),
        other => other.into(),
    }
}

fn backend_rate(b: &Backend) -> f64 {
    match b {
        Backend::Node(n) => n.sample_rate(),
        Backend::Cpal { config, .. } => config.sample_rate() as f64,
    }
}

/// Opens the backend the settings ask for. On Linux the graph node is the
/// default, cpal/PipeWire its fallback.
fn open(s: &Settings, name: &str) -> Result<Backend, String> {
    match s.host.as_deref() {
        None if !cfg!(target_os = "linux") => cpal_backend(s),
        None | Some("node") => match node::Node::open(name, s.buffer.unwrap_or(BUFFER)) {
            Ok(n) => Ok(Backend::Node(n)),
            Err(e) if s.host.is_none() => {
                eprintln!("graph node unavailable ({e}); falling back to cpal/PipeWire, without MIDI ports");
                cpal_backend(s)
            }
            Err(e) => Err(format!("graph node: {e}")),
        },
        Some(_) => cpal_backend(s),
    }
}

pub fn pick_host(want: Option<&str>) -> Result<cpal::Host, String> {
    let ids = cpal::available_hosts();
    let find = |name: &str| ids.iter().find(|id| format!("{id:?}").to_lowercase().contains(name)).copied();
    let id = match want {
        Some(w) => find(&w.to_lowercase()).ok_or_else(|| format!("audio host {w:?} not available: {ids:?}"))?,
        None => find("pipewire").unwrap_or_else(|| cpal::default_host().id()),
    };
    cpal::host_from_id(id).map_err(|e| e.to_string())
}

pub fn describe(d: &cpal::Device) -> String {
    d.description().map(|x| x.to_string()).unwrap_or_default()
}

fn cpal_backend(s: &Settings) -> Result<Backend, String> {
    let host = pick_host(s.host.as_deref().filter(|h| *h != "node"))?;
    let named = |want: &str| host.output_devices().ok()?.find(|d| describe(d).contains(want));
    let usable = |d: &cpal::Device| d.default_output_config().is_ok();
    let device = match &s.device {
        Some(want) => named(want).ok_or_else(|| format!("no output device matching {want:?}"))?,
        None => host
            .default_output_device()
            .filter(usable)
            .or_else(|| named("PipeWire").filter(usable))
            .or_else(|| named("PulseAudio").filter(usable))
            .ok_or("no usable audio output device")?,
    };
    let config = device.default_output_config().map_err(|e| e.to_string())?;
    let (sr, ch, fmt) = (config.sample_rate(), config.channels(), config.sample_format());
    eprintln!("audio output: {} ({:?}, {sr} Hz, {ch} channels, {fmt:?})", describe(&device), host.id());
    Ok(Backend::Cpal { host: host.id(), device, config })
}

/// Starts the cpal stream; returns it and its buffer size (frames).
fn cpal_play(
    host: cpal::HostId,
    device: &cpal::Device,
    config: &cpal::SupportedStreamConfig,
    s: &Settings,
    player: Player,
    gain: f32,
) -> Result<(cpal::Stream, String), String> {
    let channels = config.channels();
    let first = s.output_channel.max(1) - 1;
    if first >= channels {
        return Err(format!("output channel {} beyond the device's {channels}", s.output_channel));
    }
    // ASIO: the driver's size unless asked (its sizes may go by steps).
    let asio = host_id(host).contains("asio");
    let buffer_size = match (s.buffer, config.buffer_size()) {
        (None, _) if asio => cpal::BufferSize::Default,
        (want, cpal::SupportedBufferSize::Range { min, max }) => cpal::BufferSize::Fixed(want.unwrap_or(BUFFER).clamp(*min, *max)),
        (want, cpal::SupportedBufferSize::Unknown) => cpal::BufferSize::Fixed(want.unwrap_or(BUFFER)),
    };
    let stream_config = cpal::StreamConfig { channels, sample_rate: config.sample_rate(), buffer_size };
    let first = first as usize;
    use cpal::SampleFormat as F;
    let stream = match config.sample_format() {
        F::F32 => cpal_stream::<f32>(device, stream_config, player, gain, first),
        F::F64 => cpal_stream::<f64>(device, stream_config, player, gain, first),
        F::I16 => cpal_stream::<i16>(device, stream_config, player, gain, first),
        F::I24 => cpal_stream::<cpal::I24>(device, stream_config, player, gain, first),
        F::I32 => cpal_stream::<i32>(device, stream_config, player, gain, first),
        F::U16 => cpal_stream::<u16>(device, stream_config, player, gain, first),
        f => return Err(format!("unsupported sample format {f:?}")),
    }
    .map_err(|e| e.to_string())?;
    stream.play().map_err(|e| e.to_string())?;
    let frames = stream.buffer_size().map_or("?".to_string(), |n| n.to_string());
    eprintln!("  {frames} frames/buffer, channels {}–{}", first + 1, (first + 2).min(channels as usize));
    Ok((stream, frames))
}

/// The cpal output stream in the device's sample format: the stereo pair on
/// channels `first` and `first + 1`, silence on the others.
fn cpal_stream<T: cpal::SizedSample + cpal::FromSample<f32>>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    mut player: Player,
    gain: f32,
    first: usize,
) -> Result<cpal::Stream, cpal::Error> {
    let channels = config.channels as usize;
    device.build_output_stream(
        config,
        move |data: &mut [T], _| {
            let frames = data.len() / channels;
            player.render(
                frames,
                std::iter::empty(),
                |off, l, r| {
                    for (i, f) in data[off * channels..(off + l.len()) * channels].chunks_mut(channels).enumerate() {
                        f.fill(T::EQUILIBRIUM);
                        f[first] = T::from_sample(gain * l[i]);
                        if let Some(x) = f.get_mut(first + 1) {
                            *x = T::from_sample(gain * r[i]);
                        }
                    }
                },
                |_, _| {},
            );
        },
        |e| eprintln!("audio error: {e}"),
        None,
    )
}

/// Keyboard through midir (cpal backends): events go to the queue. Returns
/// the connection and the port's name.
#[allow(clippy::type_complexity)]
fn midir_input(
    want: Option<&str>,
    mut tx: Queue,
    keyboard: Arc<KeyboardInput>,
) -> Result<(Option<midir::MidiInputConnection<()>>, Option<String>), String> {
    let midi_in = midir::MidiInput::new("unsampled-piano").map_err(|e| e.to_string())?;
    let ports = midi_in.ports();
    let name = |p: &midir::MidiInputPort| midi_in.port_name(p).unwrap_or_default();
    let port = match want {
        Some(w) => ports.iter().find(|p| name(p).contains(w)),
        None => ports.iter().find(|p| !name(p).contains("Through")),
    };
    let Some(p) = port.cloned() else {
        eprintln!("no MIDI input port found (use --list / --midi, or --demo)");
        return Ok((None, None));
    };
    let port_name = name(&p);
    eprintln!("MIDI input: {port_name}");
    let conn = midi_in
        .connect(
            &p,
            "unsampled-piano-in",
            move |_, msg, _| {
                if let Some(ev) = keyboard.decode(msg) {
                    let _ = tx.push(ev);
                }
            },
            (),
        )
        .map_err(|e| e.to_string())?;
    Ok((Some(conn), Some(port_name)))
}

/// JACK port prefix (node description, else name) of PipeWire's default
/// audio sink, from `pw-metadata` and `pw-cli`.
pub fn default_sink_prefix() -> Option<String> {
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

/// The settings into `play.toml` (see [`save_keys`]).
pub fn save(settings: &Settings, path: &std::path::Path) -> Result<(), String> {
    let quote = |s: &str| toml::Value::String(s.into()).to_string();
    save_keys(
        path,
        &[
            ("host", settings.host.as_deref().map(quote)),
            ("device", settings.device.as_deref().map(quote)),
            ("buffer", settings.buffer.map(|b| b.to_string())),
            ("output_channel", (settings.output_channel != 1).then(|| settings.output_channel.to_string())),
            ("midi", settings.midi.as_deref().map(quote)),
            ("midi_out", settings.midi_out.as_deref().map(quote)),
            ("threads", (settings.threads > 1).then(|| settings.threads.to_string())),
            ("quality", (settings.quality != Quality::Full).then(|| quote(settings.quality.name()))),
            ("midi_channel", settings.midi_channel.map(|c| c.to_string())),
            ("invert_sustain", settings.invert_sustain.then(|| "true".to_string())),
            ("full_rate", settings.full_rate.then(|| "true".to_string())),
        ],
    )
}

/// Keys of `play.toml` (values in TOML), line by line so that the user's
/// comments and other keys stay: a key set replaces its line (or is
/// appended), a key set to None loses it.
pub fn save_keys(path: &std::path::Path, entries: &[(&str, Option<String>)]) -> Result<(), String> {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let key_of = |line: &str| line.split('=').next().map(str::trim).filter(|k| !k.starts_with('#')).map(str::to_string);
    let mut lines: Vec<String> = Vec::new();
    let mut written = Vec::new();
    for line in text.lines() {
        match key_of(line).and_then(|k| entries.iter().find(|(e, _)| *e == k)) {
            Some((k, Some(v))) => {
                lines.push(format!("{k} = {v}"));
                written.push(*k);
            }
            Some((_, None)) => {}
            None => lines.push(line.to_string()),
        }
    }
    for (k, v) in entries {
        if let (Some(v), false) = (v, written.contains(k)) {
            lines.push(format!("{k} = {v}"));
        }
    }
    let mut out = lines.join("\n");
    out.push('\n');
    path.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|()| std::fs::write(path, out)).map_err(|e| format!("{}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The engine's voice workers run with real-time scheduling, where the
    /// system allows it (RLIMIT_RTPRIO > 0).
    #[cfg(target_os = "linux")]
    #[test]
    fn voice_workers_are_real_time() {
        let limit = std::fs::read_to_string("/proc/self/limits").unwrap_or_default();
        let rtprio = limit.lines().find(|l| l.starts_with("Max realtime priority")).and_then(|l| l.split_whitespace().nth(3)).and_then(|v| v.parse::<u32>().ok());
        if rtprio.unwrap_or(0) == 0 {
            eprintln!("RLIMIT_RTPRIO is 0 here: nothing to check");
            return;
        }
        let cfg = EngineConfig { threads: 3, worker_init: Some(promote_worker), quality: Quality::Light, ..Default::default() };
        let _engine = piano_core::Engine::new(cfg);
        std::thread::sleep(std::time::Duration::from_millis(100));
        let mut workers = 0;
        for task in std::fs::read_dir("/proc/self/task").unwrap().flatten() {
            let comm = std::fs::read_to_string(task.path().join("comm")).unwrap_or_default();
            if comm.trim().starts_with("piano-voice") {
                // Field 41 of stat: the scheduling policy (1 = SCHED_FIFO, 2 = SCHED_RR).
                let stat = std::fs::read_to_string(task.path().join("stat")).unwrap();
                let policy: u32 = stat.rsplit(')').next().unwrap().split_whitespace().nth(38).unwrap().parse().unwrap();
                assert!(policy == 1 || policy == 2, "{comm}: policy {policy}");
                workers += 1;
            }
        }
        assert_eq!(workers, 2);
    }

    fn settings(host: Option<&str>) -> Settings {
        Settings {
            host: host.map(str::to_string),
            device: None,
            buffer: None,
            output_channel: 1,
            midi: None,
            midi_out: None,
            threads: 1,
            quality: Quality::Full,
            midi_channel: None,
            invert_sustain: false,
            full_rate: false,
        }
    }

    #[test]
    fn outputs_group_their_channels() {
        assert_eq!(output_group("Built-in Audio Analog Stereo:playback_FL"), "Built-in Audio Analog Stereo:playback");
        assert_eq!(output_group("Stereo DJ X-Fade:In B Left"), "Stereo DJ X-Fade:In B");
        assert_eq!(output_group("plain"), "plain");
    }

    /// Against the running audio system: reconnects the node, switches to
    /// cpal/PipeWire and back, recovers from a missing device, and keeps
    /// the model edited before a restart. No note plays.
    #[test]
    #[ignore = "needs a running PipeWire (JACK API) server: cargo test -p piano-play -- --ignored"]
    fn switches_while_playing() {
        use piano_params::Value;
        let s = Settings { host: Some("node".into()), midi: Some("zz-none".into()), ..settings(None) };
        let start = |_| Ok(Start { timeline: Vec::new(), listen: true, demo: None });
        let (mut a, c) = Audio::start(s.clone(), "piano-switch-test", 0.0, Arc::default(), PianoModel::default(), None, start).unwrap();
        let control = Arc::new(Mutex::new(c));
        control.lock().unwrap().set("output_gain_db", Value::Float(-6.0));
        assert_eq!(a.choices().backend, "node");

        // Another output: reconnected in place, same controller.
        let before = Arc::as_ptr(control.lock().unwrap().shared());
        if let Some(o) = a.choices().outputs.last().cloned() {
            a.apply(Settings { device: Some(o.clone()), ..s.clone() }, &control).unwrap();
            assert_eq!(a.choices().output_now.as_deref(), Some(o.as_str()));
            assert_eq!(Arc::as_ptr(control.lock().unwrap().shared()), before);
        }

        // cpal: a new engine at the device's rate, the edit kept.
        let cpal = Settings { host: Some("pipewire".into()), device: None, ..s.clone() };
        a.apply(cpal.clone(), &control).unwrap();
        assert_eq!(a.choices().backend, "pipewire");
        assert_ne!(Arc::as_ptr(control.lock().unwrap().shared()), before);
        assert_eq!(control.lock().unwrap().model().output_gain_db, -6.0);

        // A device that does not exist: an error, the previous set-up back.
        let err = a.apply(Settings { device: Some("zz-no-such-device".into()), ..cpal.clone() }, &control).unwrap_err();
        assert!(err.contains("back to the previous set-up"), "{err}");
        assert_eq!((a.choices().backend.as_str(), &a.settings), ("pipewire", &cpal));

        a.apply(s.clone(), &control).unwrap();
        assert_eq!(a.choices().backend, "node");
        assert_eq!(control.lock().unwrap().model().output_gain_db, -6.0);

        // The keyboard's channel and pedal: no restart.
        let before = Arc::as_ptr(control.lock().unwrap().shared());
        a.apply(Settings { midi_channel: Some(2), invert_sustain: true, ..s.clone() }, &control).unwrap();
        assert_eq!(Arc::as_ptr(control.lock().unwrap().shared()), before);
        assert_eq!(a.keyboard.decode(&[0xB1, 64, 127]), Some(Event::Control { cc: 64, value: 0 }));
        assert_eq!(a.keyboard.decode(&[0xB0, 64, 127]), None);
        a.apply(s.clone(), &control).unwrap();

        // Threads and quality on the node: a new engine, the edit kept.
        let before = Arc::as_ptr(control.lock().unwrap().shared());
        a.apply(Settings { threads: 2, quality: Quality::Light, ..s.clone() }, &control).unwrap();
        assert_ne!(Arc::as_ptr(control.lock().unwrap().shared()), before);
        assert_eq!((a.choices().backend.as_str(), control.lock().unwrap().model().output_gain_db), ("node", -6.0));
    }

    #[test]
    fn settings_keep_the_rest_of_the_file() {
        let dir = std::env::temp_dir().join(format!("piano-play-save-{}", std::process::id()));
        let path = dir.join("play.toml");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&path, "# my set-up\ndevice = \"Old\"\nui = true\nbuffer = 512\n").unwrap();
        let s = Settings { host: Some("alsa".into()), device: Some("New \"one\"".into()), output_channel: 3, ..settings(None) };
        save(&s, &path).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        // The comment and the other keys stay in place, the device's line too.
        assert!(text.starts_with("# my set-up\ndevice = ") && text.contains("\nui = true\n"), "{text}");
        assert!(!text.contains("buffer") && text.contains("host = \"alsa\"") && text.contains("output_channel = 3"), "{text}");
        let table: toml::Table = text.parse().unwrap();
        assert_eq!(table["device"].as_str(), Some("New \"one\""));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
