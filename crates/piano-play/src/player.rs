//! The audio-thread side, shared by the audio backends: renders a host
//! buffer from the queued events (keyboard via midir, demo), the events the
//! backend delivers with their frame (graph MIDI input) and the file
//! timeline, and reports every event it plays (MIDI mirror). No allocation,
//! no lock.

use piano_control::EngineSlot;
use piano_core::upsample::Upsampler;
use piano_core::{Event, TimedEvent};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicU8, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// Engine block limit; host buffers are rendered in pieces of this size.
pub const MAX_BLOCK: usize = 1024;
const MAX_EVENTS: usize = 512;

#[derive(Default)]
pub struct Stats {
    /// Max callback time / buffer duration over the last period, ×1e4.
    pub load_max: AtomicU32,
    pub load_sum: AtomicU32,
    pub callbacks: AtomicU32,
    pub voices: AtomicU32,
    /// Samples rendered since the start.
    pub clock: AtomicU64,
}

/// The keyboard's MIDI messages as the player takes them: from one channel
/// or all, the sustain pedal turned round for keyboards whose pedal works
/// the other way. Set while playing (atomics: the audio thread and midir's
/// read them). MIDI files are not filtered.
#[derive(Default)]
pub struct KeyboardInput {
    /// 1–16, 0: every channel.
    channel: AtomicU8,
    invert_sustain: AtomicBool,
}

impl KeyboardInput {
    pub fn set(&self, channel: Option<u8>, invert_sustain: bool) {
        self.channel.store(channel.filter(|c| (1..=16).contains(c)).unwrap_or(0), Ordering::Relaxed);
        self.invert_sustain.store(invert_sustain, Ordering::Relaxed);
    }

    /// The event of a message from the keyboard, if the player takes it.
    pub fn decode(&self, msg: &[u8]) -> Option<Event> {
        let channel = self.channel.load(Ordering::Relaxed);
        let status = *msg.first()?;
        // Channel messages (0x80–0xEF) of another channel are left out.
        if channel != 0 && status < 0xF0 && status & 0x0F != channel - 1 {
            return None;
        }
        match piano_host::decode_midi(msg)? {
            Event::Control { cc: 64, value } if self.invert_sustain.load(Ordering::Relaxed) => {
                Some(Event::Control { cc: 64, value: 127 - value.min(127) })
            }
            e => Some(e),
        }
    }
}

/// Engine samples per output sample: the engine runs at 44.1/48 kHz behind
/// an output at 88.2–384 kHz (1 when `full_rate`, or at 44.1/48 kHz).
pub fn rate_factor(output_rate: f64, full_rate: bool) -> usize {
    let mut k = 1;
    while !full_rate && k < 8 && output_rate / (2 * k) as f64 >= 44_100.0 {
        k *= 2;
    }
    k
}

pub struct Player {
    /// Current engine, and the replaced ones finishing their notes.
    engine: EngineSlot,
    /// The output's rate (buffers, timeline and clock count its frames).
    sample_rate: f64,
    /// Output frames per engine sample, and the upsamplers (left, right).
    factor: usize,
    up: [Upsampler; 2],
    /// Upsampled output; its frames `held_at..held_at + held` are computed
    /// but not sent yet (the end of the last engine block).
    ul: Vec<f32>,
    ur: Vec<f32>,
    held_at: usize,
    held: usize,
    /// File events (absolute sample, event), sorted.
    timeline: Vec<(u64, Event)>,
    cursor: usize,
    clock: u64,
    queue: Option<rtrb::Consumer<Event>>,
    all: Vec<TimedEvent>,
    piece: Vec<TimedEvent>,
    l: Vec<f32>,
    r: Vec<f32>,
    stats: Arc<Stats>,
    blackbox: Option<Arc<crate::blackbox::BlackBox>>,
}

impl Player {
    /// `sample_rate` is the output's; the engine runs at `sample_rate /
    /// factor`.
    pub fn new(
        engine: EngineSlot,
        sample_rate: f64,
        factor: usize,
        timeline: Vec<(u64, Event)>,
        queue: Option<rtrb::Consumer<Event>>,
        stats: Arc<Stats>,
    ) -> Self {
        let engine_rate = sample_rate / factor as f64;
        let up = || Upsampler::new(engine_rate, factor, MAX_BLOCK);
        let size = if factor > 1 { MAX_BLOCK * factor } else { 0 };
        Self {
            engine,
            sample_rate,
            factor,
            up: [up(), up()],
            ul: vec![0.0; size],
            ur: vec![0.0; size],
            held_at: 0,
            held: 0,
            timeline,
            cursor: 0,
            clock: 0,
            queue,
            all: Vec::with_capacity(MAX_EVENTS),
            piece: Vec::with_capacity(MAX_EVENTS),
            l: vec![0.0; MAX_BLOCK],
            r: vec![0.0; MAX_BLOCK],
            stats,
            blackbox: None,
        }
    }

    /// Records every engine block into the flight recorder.
    pub fn with_blackbox(mut self, b: Arc<crate::blackbox::BlackBox>) -> Self {
        self.blackbox = Some(b);
        self
    }

    /// Renders `frames` samples. `input` are events at frames relative to
    /// the buffer, in order; queued events play at its start, file events at
    /// their exact frame. `mirror(frame, event)` sees every event played, in
    /// order; `out(offset, left, right)` receives the audio in pieces.
    pub fn render(
        &mut self,
        frames: usize,
        input: impl Iterator<Item = (u32, Event)>,
        mut out: impl FnMut(usize, &[f32], &[f32]),
        mut mirror: impl FnMut(u32, Event),
    ) {
        let start = Instant::now();
        self.all.clear();
        if let Some(q) = self.queue.as_mut() {
            while self.all.len() < MAX_EVENTS / 4 {
                match q.pop() {
                    Ok(event) => self.all.push(TimedEvent { frame: 0, event }),
                    Err(_) => break,
                }
            }
        }
        for (frame, event) in input {
            if self.all.len() < MAX_EVENTS / 2 {
                self.all.push(TimedEvent { frame: frame.min(frames as u32 - 1), event });
            }
        }
        // File events due in this buffer (late ones, if it overflowed, at its
        // start).
        let end = self.clock + frames as u64;
        while self.cursor < self.timeline.len() && self.timeline[self.cursor].0 < end && self.all.len() < MAX_EVENTS {
            let (t, event) = self.timeline[self.cursor];
            self.all.push(TimedEvent { frame: t.saturating_sub(self.clock) as u32, event });
            self.cursor += 1;
        }
        // In place, no allocation.
        self.all.sort_unstable_by_key(|e| e.frame);
        for e in &self.all {
            mirror(e.frame, e.event);
        }
        let k = self.factor;
        let (mut done, mut i) = (0, 0);
        // What the last engine block left over.
        if self.held > 0 {
            let n = self.held.min(frames);
            let at = self.held_at;
            out(0, &self.ul[at..at + n], &self.ur[at..at + n]);
            (self.held_at, self.held, done) = (at + n, self.held - n, n);
        }
        while done < frames {
            // Engine samples covering the rest, at most one block.
            let n = (frames - done).div_ceil(k).min(MAX_BLOCK);
            self.piece.clear();
            while i < self.all.len() && (self.all[i].frame as usize) < done + n * k {
                let e = self.all[i];
                let frame = (e.frame as usize).saturating_sub(done) / k;
                self.piece.push(TimedEvent { frame: frame as u32, event: e.event });
                i += 1;
            }
            self.engine.process_stereo(&self.piece, &mut self.l[..n], &mut self.r[..n]);
            if let Some(b) = &self.blackbox {
                b.block(&self.piece, &self.l[..n], &self.r[..n]);
            }
            let sent = if k == 1 {
                out(done, &self.l[..n], &self.r[..n]);
                n
            } else {
                self.up[0].process(&self.l[..n], &mut self.ul[..n * k]);
                self.up[1].process(&self.r[..n], &mut self.ur[..n * k]);
                let m = (n * k).min(frames - done);
                out(done, &self.ul[..m], &self.ur[..m]);
                (self.held_at, self.held) = (m, n * k - m);
                m
            };
            done += sent;
        }
        self.clock = end;
        let st = &self.stats;
        let load = start.elapsed().as_secs_f64() * self.sample_rate / frames.max(1) as f64;
        let l4 = (load * 1e4) as u32;
        st.load_max.fetch_max(l4, Ordering::Relaxed);
        st.load_sum.fetch_add(l4, Ordering::Relaxed);
        st.callbacks.fetch_add(1, Ordering::Relaxed);
        st.voices.store(self.engine.active_voices() as u32, Ordering::Relaxed);
        self.engine.report_load(load as f32);
        st.clock.store(self.clock, Ordering::Relaxed);
    }
}

/// Raw MIDI (channel 1) of an event, for the mirror output; None for the
/// engine's own events (parameter changes).
pub fn encode(e: Event) -> Option<[u8; 3]> {
    match e {
        Event::NoteOn { note, velocity } => Some([0x90, note & 0x7f, velocity & 0x7f]),
        Event::NoteOff { note, velocity } => Some([0x80, note & 0x7f, velocity & 0x7f]),
        Event::Control { cc, value } => Some([0xb0, cc & 0x7f, value & 0x7f]),
        Event::AllNotesOff => Some([0xb0, 123, 0]),
        Event::Param { .. } => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_rate_stays_at_48_khz() {
        let f = |rate, full| rate_factor(rate, full);
        assert_eq!([f(44_100.0, false), f(48_000.0, false), f(64_000.0, false)], [1, 1, 1]);
        assert_eq!([f(88_200.0, false), f(96_000.0, false), f(192_000.0, false), f(384_000.0, false)], [2, 2, 4, 8]);
        assert_eq!(f(96_000.0, true), 1);
    }

    /// A few notes through a player: the output, sent in buffers of
    /// `sizes` (cycled), and the engine's rate.
    fn play(rate: f64, factor: usize, sizes: &[usize]) -> Vec<f32> {
        use piano_core::{EngineConfig, Quality};
        let model = piano_core::params::PianoModel::default();
        let cfg = EngineConfig {
            sample_rate: rate / factor as f64,
            max_block: MAX_BLOCK,
            model,
            quality: Quality::Light,
            ..Default::default()
        };
        let (_control, engine) = piano_control::new(model, cfg);
        let at = |s: f64| (s * rate) as u64;
        let timeline = vec![
            (at(0.01), Event::NoteOn { note: 60, velocity: 90 }),
            (at(0.0503), Event::NoteOn { note: 67, velocity: 70 }),
            (at(0.2), Event::NoteOff { note: 60, velocity: 64 }),
        ];
        let mut player = Player::new(engine, rate, factor, timeline, None, Arc::default());
        let total = at(0.3) as usize;
        let mut out = vec![0.0; total];
        let mut done = 0;
        for &n in sizes.iter().cycle() {
            let n = n.min(total - done);
            player.render(n, std::iter::empty(), |off, l, _| out[done + off..done + off + l.len()].copy_from_slice(l), |_, _| {});
            done += n;
            if done == total {
                break;
            }
        }
        out
    }

    /// The engine starts its 64-sample sub-blocks at each call, so the
    /// buffers are chosen to give it the same calls: odd sizes leave one
    /// upsampled frame for the next buffer, which then needs one less.
    #[test]
    fn upsampled_output_does_not_depend_on_the_buffers() {
        let a = play(96_000.0, 2, &[256]);
        let b = play(96_000.0, 2, &[255, 257, 1023, 1025]);
        assert!(a.iter().any(|x| x.abs() > 1e-3), "no sound");
        assert_eq!(a, b);
    }

    #[test]
    fn upsampled_output_is_the_engine_at_48_khz() {
        let up = play(96_000.0, 2, &[512]);
        let base = play(48_000.0, 1, &[256]);
        let rms = |x: &[f32]| (x.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / x.len() as f64).sqrt();
        // Same energy (one output sample in two is an engine sample, the
        // filter's delay aside).
        let ratio = rms(&up) / rms(&base);
        assert!((ratio - 1.0).abs() < 0.01, "{ratio}");
        let delay = Upsampler::new(48_000.0, 2, 1).latency();
        let err: f64 = base.iter().zip(up.iter().skip(delay).step_by(2)).map(|(a, b)| ((a - b) as f64).powi(2)).sum::<f64>();
        let rel = (err / base.len() as f64).sqrt() / rms(&base);
        assert!(rel < 1e-3, "{rel}");
    }

    /// The flight recorder keeps what the engine played, block for block,
    /// in buffers of irregular sizes, and writes it out
    /// (`unsampled-piano-render replay` renders it again: see its output in
    /// the folder printed).
    #[test]
    fn flight_recorder_keeps_what_was_played() {
        use piano_core::{EngineConfig, Quality};
        let rate = 48_000.0;
        let model = piano_core::params::PianoModel::default();
        let cfg = EngineConfig { sample_rate: rate, max_block: MAX_BLOCK, model, quality: Quality::Light, threads: 2, ..Default::default() };
        let (_control, engine) = piano_control::new(model, cfg);
        let at = |s: f64| (s * rate) as u64;
        let timeline = vec![
            (at(0.01), Event::NoteOn { note: 60, velocity: 90 }),
            (at(0.0503), Event::NoteOn { note: 67, velocity: 70 }),
            (at(0.0503), Event::NoteOn { note: 76, velocity: 110 }),
            (at(0.2), Event::NoteOff { note: 60, velocity: 64 }),
            (at(0.31), Event::NoteOn { note: 67, velocity: 100 }),
            (at(0.45), Event::Control { cc: 64, value: 127 }),
            (at(0.5), Event::NoteOn { note: 48, velocity: 60 }),
        ];
        let bb = Arc::new(crate::blackbox::BlackBox::new());
        bb.start(rate, 2, Quality::Light.name(), model);
        let mut player = Player::new(engine, rate, 1, timeline, None, Arc::default()).with_blackbox(bb.clone());
        let total = at(0.8) as usize;
        let mut out = vec![0.0; total];
        let mut done = 0;
        for &n in [256, 128, 333, 1024, 64, 777].iter().cycle() {
            let n = n.min(total - done);
            player.render(n, std::iter::empty(), |off, l, _| out[done + off..done + off + l.len()].copy_from_slice(l), |_, _| {});
            done += n;
            if done == total {
                break;
            }
        }
        let root = std::env::temp_dir().join(format!("piano-blackbox-test-{}", std::process::id()));
        let dir = bb.dump(&root).unwrap().expect("a recording");
        eprintln!("flight recording: {}", dir.display());
        let text = std::fs::read_to_string(dir.join("events.txt")).unwrap();
        assert_eq!(text.lines().filter(|l| l.starts_with("E ")).count(), 7);
        let wav = std::fs::read(dir.join("audio.wav")).unwrap();
        let samples: Vec<f32> = wav[44..].chunks(4).map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]])).collect();
        let left: Vec<f32> = samples.iter().step_by(2).copied().collect();
        assert_eq!(left, out);
        assert!(out.iter().any(|x| x.abs() > 1e-3), "no sound");
    }

    #[test]
    fn keyboard_channel_and_pedal() {
        let k = KeyboardInput::default();
        // Every channel, the pedal as it comes.
        assert_eq!(k.decode(&[0x93, 60, 100]), Some(Event::NoteOn { note: 60, velocity: 100 }));
        assert_eq!(k.decode(&[0xB0, 64, 127]), Some(Event::Control { cc: 64, value: 127 }));
        // Channel 4 only (status low nibble 3), the sustain pedal inverted.
        k.set(Some(4), true);
        assert_eq!(k.decode(&[0x90, 60, 100]), None);
        assert_eq!(k.decode(&[0x93, 60, 100]), Some(Event::NoteOn { note: 60, velocity: 100 }));
        assert_eq!(k.decode(&[0xB3, 64, 127]), Some(Event::Control { cc: 64, value: 0 }));
        assert_eq!(k.decode(&[0xB3, 64, 40]), Some(Event::Control { cc: 64, value: 87 }));
        assert_eq!(k.decode(&[0xB3, 67, 127]), Some(Event::Control { cc: 67, value: 127 }));
        k.set(None, false);
        assert_eq!(k.decode(&[0xB5, 64, 127]), Some(Event::Control { cc: 64, value: 127 }));
    }
}
