//! The audio-thread side, shared by the audio backends: renders a host
//! buffer from the queued events (keyboard via midir, demo), the events the
//! backend delivers with their frame (graph MIDI input) and the file
//! timeline, and reports every event it plays (MIDI mirror). No allocation,
//! no lock.

use piano_control::EngineSlot;
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

pub struct Player {
    /// Current engine, and the replaced ones finishing their notes.
    engine: EngineSlot,
    sample_rate: f64,
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
}

impl Player {
    pub fn new(
        engine: EngineSlot,
        sample_rate: f64,
        timeline: Vec<(u64, Event)>,
        queue: Option<rtrb::Consumer<Event>>,
        stats: Arc<Stats>,
    ) -> Self {
        Self {
            engine,
            sample_rate,
            timeline,
            cursor: 0,
            clock: 0,
            queue,
            all: Vec::with_capacity(MAX_EVENTS),
            piece: Vec::with_capacity(MAX_EVENTS),
            l: vec![0.0; MAX_BLOCK],
            r: vec![0.0; MAX_BLOCK],
            stats,
        }
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
        let (mut done, mut i) = (0, 0);
        while done < frames {
            let n = (frames - done).min(MAX_BLOCK);
            self.piece.clear();
            while i < self.all.len() && (self.all[i].frame as usize) < done + n {
                let e = self.all[i];
                self.piece.push(TimedEvent { frame: e.frame - done as u32, event: e.event });
                i += 1;
            }
            self.engine.process_stereo(&self.piece, &mut self.l[..n], &mut self.r[..n]);
            out(done, &self.l[..n], &self.r[..n]);
            done += n;
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
