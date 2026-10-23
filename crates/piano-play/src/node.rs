//! The piano as a node of the audio graph, through the JACK API (served by
//! PipeWire's libjack): stereo audio out, a MIDI input the piano plays from
//! with sample-accurate timing, and a MIDI output mirroring every event the
//! piano plays (keyboard, graph input, files).

use crate::player::{encode, Player};
use jack::{AudioOut, Client, ClientOptions, Control, MidiIn, MidiOut, Port, PortFlags, ProcessScope, RawMidi};

pub struct Node {
    client: Client,
    ports: Ports,
}

struct Ports {
    left: Port<AudioOut>,
    right: Port<AudioOut>,
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
}

impl jack::ProcessHandler for Process {
    fn process(&mut self, _: &Client, ps: &ProcessScope) -> Control {
        let frames = ps.n_frames() as usize;
        let Ports { left, right, midi_in, midi_out } = &mut self.ports;
        let input = midi_in.iter(ps).filter_map(|m| piano_host::decode_midi(m.bytes).map(|e| (m.time, e)));
        let (ol, or) = (left.as_mut_slice(ps), right.as_mut_slice(ps));
        let mut writer = midi_out.writer(ps);
        let gain = self.gain;
        self.player.render(
            frames,
            input,
            |off, l, r| {
                for (o, x) in ol[off..off + l.len()].iter_mut().zip(l) {
                    *o = gain * x;
                }
                for (o, x) in or[off..off + r.len()].iter_mut().zip(r) {
                    *o = gain * x;
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
    pub fn start(self, player: Player, gain: f32) -> Result<(Running, PortNames), jack::Error> {
        let names = PortNames {
            left: self.ports.left.name()?,
            right: self.ports.right.name()?,
            midi_in: self.ports.midi_in.name()?,
            midi_out: self.ports.midi_out.name()?,
        };
        let active = self.client.activate_async((), Process { player, ports: self.ports, gain })?;
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
