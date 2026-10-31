//! MIDI files played by the player: a song (events in seconds) that the
//! audio thread plays at its exact frame, with a transport the interface
//! drives (open, play, pause, stop, go to a time). The audio side neither
//! allocates nor frees: a replaced song goes back to the transport, which
//! frees it.

use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use piano_core::Event;

/// The pedals a song releases when it loads or pauses (not at its end: the
/// last chord rings as the file leaves it).
const PEDALS: [u8; 3] = [64, 66, 67];

#[derive(Clone, Default)]
pub struct Song {
    pub name: String,
    /// (time s, event), sorted.
    pub events: Vec<(f64, Event)>,
    /// Time of the last event (s).
    pub length: f64,
}

impl Song {
    pub fn new(name: String, events: Vec<(f64, Event)>) -> Self {
        let length = events.last().map_or(0.0, |e| e.0);
        Self { name, events, length }
    }

    /// A standard MIDI file, all tracks and channels merged.
    pub fn read(path: &Path) -> Result<Self, String> {
        let events = piano_host::read_midi_file(path)?;
        let name = path.file_stem().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        Ok(Self::new(name, events))
    }
}

pub enum Command {
    /// A new song, stopped at its start.
    Load(Box<Song>),
    Play,
    Pause,
    /// Go to this time (s), playing or not.
    Seek(f64),
}

/// The song's state for the interface, kept by the audio side.
#[derive(Default)]
pub struct SongState {
    /// Position in output frames, and the output's rate (f64 bits).
    frames: AtomicU64,
    rate: AtomicU64,
    playing: AtomicBool,
}

impl SongState {
    /// Position (s).
    pub fn position(&self) -> f64 {
        let rate = f64::from_bits(self.rate.load(Ordering::Relaxed));
        if rate > 0.0 { self.frames.load(Ordering::Relaxed) as f64 / rate } else { 0.0 }
    }

    pub fn playing(&self) -> bool {
        self.playing.load(Ordering::Relaxed)
    }
}

/// The audio side of the transport.
pub struct SongPlayer {
    commands: rtrb::Consumer<Command>,
    reclaim: rtrb::Producer<Box<Song>>,
    song: Option<Box<Song>>,
    /// Next event to play.
    cursor: usize,
    /// Position in output frames.
    frames: u64,
    playing: bool,
    state: Arc<SongState>,
}

impl SongPlayer {
    /// Takes the transport's commands, then calls `push(frame, event)` for
    /// the events due in the next `n` frames at `rate`, in order (the
    /// transport's own, releasing notes and pedals, at frame 0). When
    /// `push` returns false (no room left), the song's next events wait for
    /// the next call, then play at its start.
    pub fn advance(&mut self, n: usize, rate: f64, mut push: impl FnMut(u32, Event) -> bool) {
        while let Ok(c) = self.commands.pop() {
            match c {
                Command::Load(song) => {
                    if self.playing {
                        release(&mut push);
                    }
                    if let Some(old) = self.song.replace(song) {
                        // Full only if the transport is gone: then it leaks.
                        let _ = self.reclaim.push(old);
                    }
                    (self.cursor, self.frames, self.playing) = (0, 0, false);
                    release_pedals(&mut push);
                }
                Command::Play if !self.playing => {
                    // The pause let the pedals go: they come back as the
                    // song has them here (at its start they are up).
                    if self.song.as_ref().is_some_and(|s| self.cursor >= s.events.len()) {
                        self.seek(0.0, rate, &mut push);
                    } else if self.cursor > 0 {
                        self.pedals(&mut push);
                    }
                    self.playing = self.song.is_some();
                }
                Command::Pause if self.playing => {
                    self.playing = false;
                    release(&mut push);
                }
                Command::Seek(t) => {
                    if self.playing {
                        release(&mut push);
                    }
                    self.seek(t, rate, &mut push);
                }
                Command::Play | Command::Pause => {}
            }
        }
        if let (true, Some(song)) = (self.playing, &self.song) {
            let end = self.frames + n as u64;
            while let Some(&(t, event)) = song.events.get(self.cursor) {
                let at = (t * rate) as u64;
                if at >= end {
                    break;
                }
                if !push(at.saturating_sub(self.frames) as u32, event) {
                    break;
                }
                self.cursor += 1;
            }
            self.frames = end;
            if self.cursor >= song.events.len() {
                self.playing = false;
                self.frames = (song.length * rate) as u64;
            }
        }
        self.state.frames.store(self.frames, Ordering::Relaxed);
        self.state.rate.store(rate.to_bits(), Ordering::Relaxed);
        self.state.playing.store(self.playing, Ordering::Relaxed);
    }

    /// Goes to `t` with the pedals as the song has them there.
    fn seek(&mut self, t: f64, rate: f64, push: &mut impl FnMut(u32, Event) -> bool) {
        let Some(song) = &self.song else { return };
        let t = t.clamp(0.0, song.length);
        self.cursor = song.events.partition_point(|e| e.0 < t);
        self.frames = (t * rate) as u64;
        self.pedals(push);
    }

    /// The pedals as the song has them at its cursor.
    fn pedals(&self, push: &mut impl FnMut(u32, Event) -> bool) {
        let Some(song) = &self.song else { return };
        for cc in PEDALS {
            let value = song.events[..self.cursor].iter().rev().find_map(|e| match e.1 {
                Event::Control { cc: c, value } if c == cc => Some(value),
                _ => None,
            });
            push(0, Event::Control { cc, value: value.unwrap_or(0) });
        }
    }
}

fn release_pedals(push: &mut impl FnMut(u32, Event) -> bool) {
    for cc in PEDALS {
        push(0, Event::Control { cc, value: 0 });
    }
}

/// The song's notes stop: keys up, pedals released (the strings are damped
/// as by a pianist letting go).
fn release(push: &mut impl FnMut(u32, Event) -> bool) {
    push(0, Event::AllNotesOff);
    release_pedals(push);
}

/// The interface's side: sends commands, keeps the song to give it again
/// to the audio side after a restart, frees the songs it replaced.
#[derive(Default)]
pub struct Transport {
    commands: Option<rtrb::Producer<Command>>,
    reclaim: Option<rtrb::Consumer<Box<Song>>>,
    song: Option<Song>,
    state: Arc<SongState>,
}

impl Transport {
    /// A new audio side (a player starting), playing the current song from
    /// where it stood.
    pub fn connect(&mut self) -> SongPlayer {
        let (position, playing) = (self.state.position(), self.state.playing());
        let (tx, rx) = rtrb::RingBuffer::new(64);
        let (reclaim_tx, reclaim_rx) = rtrb::RingBuffer::new(16);
        (self.commands, self.reclaim) = (Some(tx), Some(reclaim_rx));
        if let Some(song) = self.song.clone() {
            self.send(Command::Load(Box::new(song)));
            self.send(Command::Seek(position));
            if playing {
                self.send(Command::Play);
            }
        }
        SongPlayer { commands: rx, reclaim: reclaim_tx, song: None, cursor: 0, frames: 0, playing: false, state: self.state.clone() }
    }

    pub fn send(&mut self, c: Command) {
        // Full only if the audio side stopped.
        if let Some(tx) = self.commands.as_mut() {
            let _ = tx.push(c);
        }
        self.free();
    }

    /// Loads a song (stopped at its start); `play` starts it.
    pub fn load(&mut self, song: Song, play: bool) {
        self.song = Some(song.clone());
        self.send(Command::Load(Box::new(song)));
        if play {
            self.send(Command::Play);
        }
    }

    pub fn song(&self) -> Option<&Song> {
        self.song.as_ref()
    }

    pub fn state(&self) -> &SongState {
        &self.state
    }

    /// Frees the songs the audio side gave back.
    pub fn free(&mut self) {
        if let Some(rx) = self.reclaim.as_mut() {
            while rx.pop().is_ok() {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(p: &mut SongPlayer, n: usize, rate: f64) -> Vec<(u32, Event)> {
        let mut out = Vec::new();
        p.advance(n, rate, |f, e| {
            out.push((f, e));
            true
        });
        out
    }

    #[test]
    fn plays_pauses_and_seeks() {
        let rate = 1000.0;
        let on = |n| Event::NoteOn { note: n, velocity: 80 };
        let song = Song::new(
            "t".into(),
            vec![(0.010, on(60)), (0.020, Event::Control { cc: 64, value: 127 }), (0.150, on(62)), (0.300, on(64))],
        );
        let mut t = Transport::default();
        let mut p = t.connect();
        t.load(song, true);
        let first = run(&mut p, 100, rate);
        // Pedals released on load, then the first two events at their frame.
        assert_eq!(&first[3..], &[(10, on(60)), (20, Event::Control { cc: 64, value: 127 })]);
        assert!(t.state().playing());
        assert!((t.state().position() - 0.1).abs() < 1e-9);
        t.send(Command::Pause);
        let paused = run(&mut p, 100, rate);
        assert_eq!(paused[0], (0, Event::AllNotesOff));
        assert!(!t.state().playing() && (t.state().position() - 0.1).abs() < 1e-9);
        // Played again where it paused: the pedal comes back down.
        t.send(Command::Play);
        let again = run(&mut p, 10, rate);
        assert!(again.contains(&(0, Event::Control { cc: 64, value: 127 })), "{again:?}");
        t.send(Command::Pause);
        run(&mut p, 10, rate);
        // Back to 0.2 s: the pedal is down there.
        t.send(Command::Seek(0.2));
        t.send(Command::Play);
        let resumed = run(&mut p, 50, rate);
        assert!(resumed.contains(&(0, Event::Control { cc: 64, value: 127 })));
        assert!(!resumed.iter().any(|e| e.1 == on(62)));
        // To the end: stopped there; play starts again.
        let end = run(&mut p, 200, rate);
        assert!(end.contains(&(50, on(64))));
        assert!(!t.state().playing());
        t.send(Command::Play);
        assert!(run(&mut p, 20, rate).contains(&(10, on(60))));
    }

    /// A restarted audio side gets the song again, where it stood.
    #[test]
    fn a_new_player_takes_the_song_where_it_was() {
        let song = Song::new("t".into(), vec![(0.5, Event::NoteOn { note: 60, velocity: 80 })]);
        let mut t = Transport::default();
        let mut p = t.connect();
        t.load(song, true);
        run(&mut p, 200, 1000.0);
        let mut p = t.connect();
        let evs = run(&mut p, 800, 2000.0);
        assert!(evs.contains(&(600, Event::NoteOn { note: 60, velocity: 80 })), "{evs:?}");
    }
}
