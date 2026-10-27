//! Flight recorder: every event the engines received, at its exact sample
//! (the parameters edited included), every instrument rebuilt and when it
//! came in, and the length of every block rendered, since the player
//! started; and the last minute of its output. Written when the player stops
//! (`unsampled-piano-render replay` renders it again, identically), so that
//! a rare fault heard while playing can be studied offline.
//!
//! The audio thread only ever tries the lock (never waits, never
//! allocates): a block it cannot record leaves a gap, noted in the dump.

use piano_core::params::PianoModel;
use piano_core::{Event, TimedEvent};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

/// Output kept: the last minute.
const AUDIO_S: f64 = 60.0;
/// Events kept from the start (hours of playing).
const EVENTS: usize = 1 << 18;
/// Runs of equal block lengths.
const RUNS: usize = 1 << 16;
/// Instruments rebuilt (edits of parameters that are not live).
const SWAPS: usize = 256;
/// Dumps kept in the folder.
const KEEP: usize = 3;

struct Rec {
    sample_rate: f64,
    threads: usize,
    quality: String,
    model: Option<PianoModel>,
    /// (absolute engine sample, event).
    events: Vec<(u64, Event)>,
    /// (block length, how many in a row).
    runs: Vec<(u32, u32)>,
    /// (absolute engine sample, model): an engine rebuilt from it came in.
    swaps: Vec<(u64, PianoModel)>,
    clock: u64,
    /// Last `AUDIO_S` of output, a ring of stereo frames.
    audio: Vec<[f32; 2]>,
    head: usize,
    filled: usize,
    overflow: bool,
}

pub struct BlackBox {
    rec: Mutex<Rec>,
    /// Blocks the audio thread could not record (the lock was taken).
    missed: AtomicU64,
}

impl BlackBox {
    pub fn new() -> Self {
        Self {
            rec: Mutex::new(Rec {
                sample_rate: 48_000.0,
                threads: 1,
                quality: String::new(),
                model: None,
                events: Vec::with_capacity(EVENTS),
                runs: Vec::with_capacity(RUNS),
                swaps: Vec::with_capacity(SWAPS),
                clock: 0,
                audio: Vec::new(),
                head: 0,
                filled: 0,
                overflow: false,
            }),
            missed: AtomicU64::new(0),
        }
    }

    /// A new engine starts: the recording starts over (not on the audio
    /// thread: it allocates).
    pub fn start(&self, sample_rate: f64, threads: usize, quality: &str, model: PianoModel) {
        let mut r = self.rec.lock().unwrap();
        r.sample_rate = sample_rate;
        r.threads = threads;
        r.quality = quality.to_string();
        r.model = Some(model);
        r.events.clear();
        r.runs.clear();
        r.swaps.clear();
        r.clock = 0;
        r.audio = vec![[0.0; 2]; (AUDIO_S * sample_rate) as usize];
        r.head = 0;
        r.filled = 0;
        r.overflow = false;
        self.missed.store(0, Ordering::Relaxed);
    }

    /// One block, as the engine slot's journal gives it: the model of an
    /// engine that came in at its start, its events (relative to it), and
    /// its output. Audio thread: tries the lock only, allocates nothing.
    pub fn block(&self, swap: Option<&PianoModel>, events: &[TimedEvent], left: &[f32], right: &[f32]) {
        let Ok(mut r) = self.rec.try_lock() else {
            self.missed.fetch_add(1, Ordering::Relaxed);
            return;
        };
        let r = &mut *r;
        let n = left.len();
        if r.overflow || r.audio.is_empty() {
            r.clock += n as u64;
            return;
        }
        if let Some(m) = swap {
            if r.swaps.len() < r.swaps.capacity() {
                let t = r.clock;
                r.swaps.push((t, *m));
            } else {
                r.overflow = true;
            }
        }
        if r.events.len() + events.len() > r.events.capacity() {
            r.overflow = true;
        } else {
            for e in events {
                r.events.push((r.clock + e.frame as u64, e.event));
            }
        }
        let room = r.runs.len() < r.runs.capacity();
        match r.runs.last_mut() {
            Some((len, count)) if *len == n as u32 => *count += 1,
            _ if room => r.runs.push((n as u32, 1)),
            _ => r.overflow = true,
        }
        r.clock += n as u64;
        let cap = r.audio.len();
        for (l, x) in left.iter().zip(right) {
            let h = r.head;
            r.audio[h] = [*l, *x];
            r.head = (h + 1) % cap;
        }
        r.filled = (r.filled + n).min(cap);
    }

    /// Writes the recording into a new folder of `root` (keeping the last
    /// few), and returns it.
    pub fn dump(&self, root: &Path) -> std::io::Result<Option<PathBuf>> {
        let (head, meta, events, runs, swaps, audio, model) = {
            let r = self.rec.lock().unwrap();
            let Some(model) = r.model else { return Ok(None) };
            if r.clock == 0 {
                return Ok(None);
            }
            let cap = r.audio.len();
            let audio: Vec<[f32; 2]> = (0..r.filled).map(|i| r.audio[(r.head + cap - r.filled + i) % cap]).collect();
            let meta = (r.sample_rate, r.threads, r.quality.clone(), r.clock, r.overflow, self.missed.load(Ordering::Relaxed));
            (r.clock - r.filled as u64, meta, r.events.clone(), r.runs.clone(), r.swaps.clone(), audio, model)
        };
        let (sample_rate, threads, quality, clock, overflow, gaps) = meta;
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
        let dir = root.join(format!("{stamp}"));
        std::fs::create_dir_all(&dir)?;
        piano_host::save_preset(&model, &dir.join("preset.toml")).map_err(std::io::Error::other)?;
        let mut f = std::io::BufWriter::new(std::fs::File::create(dir.join("events.txt"))?);
        writeln!(f, "# unsampled-piano flight recorder: replay with `unsampled-piano-render replay {}`", dir.display())?;
        writeln!(f, "sample_rate {sample_rate}")?;
        writeln!(f, "threads {threads}")?;
        writeln!(f, "quality {quality}")?;
        writeln!(f, "samples {clock}")?;
        writeln!(f, "audio_from {head}")?;
        writeln!(f, "complete {}", !overflow && gaps == 0)?;
        writeln!(f, "# blocks: length count")?;
        for (len, count) in &runs {
            writeln!(f, "B {len} {count}")?;
        }
        writeln!(f, "# instruments rebuilt: sample at which they came in, preset")?;
        for (k, (t, m)) in swaps.iter().enumerate() {
            let name = format!("rebuilt_{k}.toml");
            piano_host::save_preset(m, &dir.join(&name)).map_err(std::io::Error::other)?;
            writeln!(f, "S {t} {name}")?;
        }
        writeln!(f, "# events: sample kind values")?;
        for (t, e) in &events {
            match e {
                Event::NoteOn { note, velocity } => writeln!(f, "E {t} on {note} {velocity}")?,
                Event::NoteOff { note, velocity } => writeln!(f, "E {t} off {note} {velocity}")?,
                Event::Control { cc, value } => writeln!(f, "E {t} cc {cc} {value}")?,
                Event::AllNotesOff => writeln!(f, "E {t} all_off")?,
                Event::Param { id, value } => writeln!(f, "E {t} param {} {value}", id.path())?,
            }
        }
        if gaps > 0 {
            writeln!(f, "# {gaps} blocks not recorded")?;
        }
        f.flush()?;
        write_wav(&dir.join("audio.wav"), sample_rate as u32, &audio)?;
        prune(root);
        Ok(Some(dir))
    }
}

impl Default for BlackBox {
    fn default() -> Self {
        Self::new()
    }
}

/// 32-bit float stereo WAV.
fn write_wav(path: &Path, rate: u32, frames: &[[f32; 2]]) -> std::io::Result<()> {
    let mut f = std::io::BufWriter::new(std::fs::File::create(path)?);
    let data = (frames.len() * 8) as u32;
    f.write_all(b"RIFF")?;
    f.write_all(&(36 + data).to_le_bytes())?;
    f.write_all(b"WAVEfmt ")?;
    f.write_all(&16u32.to_le_bytes())?;
    f.write_all(&3u16.to_le_bytes())?;
    f.write_all(&2u16.to_le_bytes())?;
    f.write_all(&rate.to_le_bytes())?;
    f.write_all(&(rate * 8).to_le_bytes())?;
    f.write_all(&8u16.to_le_bytes())?;
    f.write_all(&32u16.to_le_bytes())?;
    f.write_all(b"data")?;
    f.write_all(&data.to_le_bytes())?;
    for [l, r] in frames {
        f.write_all(&l.to_le_bytes())?;
        f.write_all(&r.to_le_bytes())?;
    }
    f.flush()
}

/// Keeps the last `KEEP` dumps (folders named by their time).
fn prune(root: &Path) {
    let Ok(rd) = std::fs::read_dir(root) else { return };
    let mut dirs: Vec<(u64, PathBuf)> =
        rd.flatten().filter_map(|e| Some((e.file_name().to_str()?.parse::<u64>().ok()?, e.path()))).filter(|(_, p)| p.join("events.txt").exists()).collect();
    dirs.sort();
    while dirs.len() > KEEP {
        let (_, p) = dirs.remove(0);
        let _ = std::fs::remove_dir_all(p);
    }
}
