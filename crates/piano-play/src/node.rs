//! The piano as a node of the audio graph, through the JACK API (served by
//! PipeWire's libjack): stereo audio out (`out_left`, `out_right`; with one
//! output per microphone, microphone 1 and 2 there and the others on
//! `out_3`…`out_8`), a MIDI input the piano plays from
//! with sample-accurate timing, and a MIDI output mirroring every event the
//! piano plays (keyboard, graph input, files).

use crate::player::{encode, KeyboardInput, Player};
use std::sync::Arc;
use jack::{AudioOut, Client, ClientOptions, Control, MidiIn, MidiOut, Port, PortFlags, ProcessScope, RawMidi};

pub struct Node {
    client: Client,
    ports: Ports,
}

struct Ports {
    left: Port<AudioOut>,
    right: Port<AudioOut>,
    /// Channels 3…8 (one output per microphone).
    more: Vec<Port<AudioOut>>,
    midi_in: Port<MidiIn>,
    midi_out: Port<MidiOut>,
}

/// Full port names, for connecting.
pub struct PortNames {
    pub left: String,
    pub right: String,
    pub midi_in: String,
    pub midi_out: String,
}

struct Process {
    player: Player,
    ports: Ports,
    gain: f32,
    keyboard: Arc<KeyboardInput>,
}

impl jack::ProcessHandler for Process {
    fn process(&mut self, _: &Client, ps: &ProcessScope) -> Control {
        let frames = ps.n_frames() as usize;
        let Ports { left, right, more, midi_in, midi_out } = &mut self.ports;
        let keyboard = &self.keyboard;
        let input = midi_in.iter(ps).filter_map(|m| keyboard.decode(m.bytes).map(|e| (m.time, e)));
        let mut ports = [left, right].into_iter().chain(more.iter_mut()).map(|p| p.as_mut_slice(ps));
        let mut outs: [&mut [f32]; piano_control::slot::MAX_CHANNELS] = std::array::from_fn(|_| ports.next().unwrap_or(&mut []));
        let mut writer = midi_out.writer(ps);
        let gain = self.gain;
        self.player.render(
            frames,
            input,
            |off, chans| {
                for (c, o) in outs.iter_mut().enumerate().filter(|(_, o)| o.len() >= off) {
                    let end = (off + chans.first().map_or(0, |x| x.len())).min(o.len());
                    match chans.get(c) {
                        Some(x) => {
                            for (o, x) in o[off..end].iter_mut().zip(x.iter()) {
                                *o = gain * x;
                            }
                        }
                        None => o[off..end].fill(0.0),
                    }
                }
            },
            |time, e| {
                if let Some(b) = encode(e) {
                    let _ = writer.write(&RawMidi { time, bytes: &b });
                }
            },
        );
        Control::Continue
    }
}

impl Node {
    /// Opens the client (not yet running) and its ports. `buffer` is the
    /// requested period (frames), passed to PipeWire as the node latency.
    pub fn open(name: &str, buffer: u32) -> Result<Self, jack::Error> {
        if std::env::var_os("PIPEWIRE_LATENCY").is_none() {
            // SAFETY: called on the main thread before any other is spawned.
            unsafe { std::env::set_var("PIPEWIRE_LATENCY", format!("{buffer}/48000")) };
        }
        let (client, _) = Client::new(name, ClientOptions::NO_START_SERVER)?;
        let ports = Ports {
            left: client.register_port("out_left", AudioOut::default())?,
            right: client.register_port("out_right", AudioOut::default())?,
            more: (3..=piano_control::slot::MAX_CHANNELS).map(|c| client.register_port(&format!("out_{c}"), AudioOut::default())).collect::<Result<_, _>>()?,
            midi_in: client.register_port("midi_in", MidiIn::default())?,
            midi_out: client.register_port("midi_out", MidiOut::default())?,
        };
        Ok(Self { client, ports })
    }

    pub fn sample_rate(&self) -> f64 {
        self.client.sample_rate() as f64
    }

    pub fn buffer_size(&self) -> u32 {
        self.client.buffer_size()
    }

    /// MIDI sources and audio inputs of the graph (other nodes' ports).
    pub fn graph_ports(&self) -> (Vec<String>, Vec<String>) {
        let own = format!("{}:", self.client.name());
        let get = |kind, flags| {
            let mut v = self.client.ports(None, Some(kind), flags);
            v.retain(|p: &String| !p.starts_with(&own));
            v
        };
        (get("midi", PortFlags::IS_OUTPUT), get("audio", PortFlags::IS_INPUT))
    }

    /// Starts processing; returns the running client (drop it to stop).
    pub fn start(self, player: Player, gain: f32, keyboard: Arc<KeyboardInput>) -> Result<(Running, PortNames), jack::Error> {
        let names = PortNames {
            left: self.ports.left.name()?,
            right: self.ports.right.name()?,
            midi_in: self.ports.midi_in.name()?,
            midi_out: self.ports.midi_out.name()?,
        };
        let player = player.with_channels(piano_control::slot::MAX_CHANNELS);
        let active = self.client.activate_async((), Process { player, ports: self.ports, gain, keyboard })?;
        Ok((Running(active), names))
    }
}

pub struct Running(jack::AsyncClient<(), Process>);

impl Running {
    pub fn connect(&self, from: &str, to: &str) -> Result<(), jack::Error> {
        self.0.as_client().connect_ports_by_name(from, to)
    }

    /// MIDI inputs of the other nodes.
    pub fn midi_inputs(&self) -> Vec<String> {
        self.others("midi", PortFlags::IS_INPUT)
    }

    /// MIDI sources and audio inputs of the other nodes, as now.
    pub fn graph_ports(&self) -> (Vec<String>, Vec<String>) {
        (self.others("midi", PortFlags::IS_OUTPUT), self.others("audio", PortFlags::IS_INPUT))
    }

    fn others(&self, kind: &str, flags: PortFlags) -> Vec<String> {
        let c = self.0.as_client();
        let own = format!("{}:", c.name());
        let mut v = c.ports(None, Some(kind), flags);
        v.retain(|p| !p.starts_with(&own));
        v
    }

    /// Removes every connection of one of our ports.
    pub fn disconnect_all(&self, port: &str) {
        let c = self.0.as_client();
        let Some(p) = c.port_by_name(port) else { return };
        for other in p.get_connections() {
            // Our outputs are the source, our inputs the destination.
            let _ = c.disconnect_ports_by_name(port, &other).or_else(|_| c.disconnect_ports_by_name(&other, port));
        }
    }
}
