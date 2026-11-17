//! Background work: measuring the recordings and rendering the model, on
//! low-priority threads, with an on-disk cache of the measures.

use crate::measure::{self, Analysis};
use crate::model::{self, ModelSpec, Renderer};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Condvar, Mutex, OnceLock};

#[derive(Clone, Debug)]
pub struct Job {
    pub src: usize,
    pub midi: u8,
    pub layer: u8,
    pub path: PathBuf,
    /// Model velocity, for a render.
    pub velocity: u8,
    /// Key released this many seconds after the onset.
    pub held: Option<f64>,
}

pub enum Done {
    Measured { job: Job, stamp: Stamp, analysis: Analysis },
    Rendered { job: Job },
    Failed { job: Job, error: String },
}

#[derive(Default)]
struct Queues {
    analyse: VecDeque<Job>,
    render: VecDeque<Job>,
    busy: usize,
    stop: bool,
}

struct Shared {
    q: Mutex<Queues>,
    cv: Condvar,
    spec: Option<Arc<ModelSpec>>,
    renderer: OnceLock<Renderer>,
    tx: Mutex<Sender<Done>>,
    ctx: egui::Context,
}

pub struct Workers {
    shared: Arc<Shared>,
    pub rx: Receiver<Done>,
}

impl Workers {
    pub fn start(ctx: egui::Context, spec: Option<Arc<ModelSpec>>, analysers: usize, renderers: usize) -> Self {
        let (tx, rx) = channel();
        let shared = Arc::new(Shared { q: Mutex::default(), cv: Condvar::new(), spec, renderer: OnceLock::new(), tx: Mutex::new(tx), ctx });
        for _ in 0..analysers {
            let s = shared.clone();
            std::thread::spawn(move || {
                low_priority(false);
                worker(&s, false)
            });
        }
        if shared.spec.is_some() {
            for _ in 0..renderers {
                let s = shared.clone();
                std::thread::spawn(move || {
                    low_priority(true);
                    worker(&s, true)
                });
            }
        }
        Self { shared, rx }
    }

    pub fn analyse(&self, job: Job) {
        self.shared.q.lock().unwrap().analyse.push_back(job);
        self.shared.cv.notify_all();
    }

    pub fn render(&self, job: Job) {
        self.shared.q.lock().unwrap().render.push_back(job);
        self.shared.cv.notify_all();
    }

    /// Moves this key's jobs to the front of both queues.
    pub fn prioritise(&self, midi: u8) {
        let mut guard = self.shared.q.lock().unwrap();
        let q = &mut *guard;
        for d in [&mut q.analyse, &mut q.render] {
            let (mut first, rest): (VecDeque<Job>, VecDeque<Job>) = d.drain(..).partition(|j| j.midi == midi);
            first.extend(rest);
            *d = first;
        }
    }

    /// (analyses queued, renders queued, jobs running).
    pub fn pending(&self) -> (usize, usize, usize) {
        let q = self.shared.q.lock().unwrap();
        (q.analyse.len(), q.render.len(), q.busy)
    }
}

impl Drop for Workers {
    fn drop(&mut self) {
        self.shared.q.lock().unwrap().stop = true;
        self.shared.cv.notify_all();
    }
}

fn worker(s: &Shared, render: bool) {
    loop {
        let job = {
            let mut q = s.q.lock().unwrap();
            loop {
                if q.stop {
                    return;
                }
                let next = if render { q.render.pop_front() } else { q.analyse.pop_front() };
                if let Some(j) = next {
                    q.busy += 1;
                    break j;
                }
                q = s.cv.wait(q).unwrap();
            }
        };
        let done = if render { do_render(s, job) } else { do_analyse(job) };
        s.q.lock().unwrap().busy -= 1;
        let _ = s.tx.lock().unwrap().send(done);
        s.ctx.request_repaint();
    }
}

fn do_render(s: &Shared, job: Job) -> Done {
    let spec = s.spec.as_ref().expect("render without a model");
    let r = s.renderer.get_or_init(|| Renderer::new(spec));
    let ch = r.note(job.midi, job.velocity);
    match model::write_wav(&job.path, &ch) {
        Ok(()) => Done::Rendered { job },
        Err(error) => Done::Failed { job, error },
    }
}

fn do_analyse(job: Job) -> Done {
    let stamp = match Stamp::of(&job.path) {
        Some(s) => s,
        None => return Done::Failed { error: "unreadable".into(), job },
    };
    match piano_analysis::io::read_channels(&job.path) {
        Ok((ch, sr)) => {
            let analysis = measure::analyse(&ch, sr, job.midi, job.held);
            Done::Measured { job, stamp, analysis }
        }
        Err(e) => Done::Failed { error: e.to_string(), job },
    }
}

/// Nice 19 for this thread (and the threads it starts); a renderer is also
/// kept to half the machine's cores (the engine builds on several threads).
fn low_priority(pin: bool) {
    #[cfg(target_os = "linux")]
    unsafe {
        let tid = libc::syscall(libc::SYS_gettid) as libc::id_t;
        libc::setpriority(libc::PRIO_PROCESS, tid, 19);
        let n = std::thread::available_parallelism().map_or(1, |n| n.get());
        if pin && n >= 16 {
            // The upper half of the physical cores and their SMT siblings
            // (8-15 and 24-31 on 32 threads).
            let mut set: libc::cpu_set_t = std::mem::zeroed();
            for c in (n / 4..n / 2).chain(3 * n / 4..n) {
                libc::CPU_SET(c, &mut set);
            }
            libc::sched_setaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &set);
        }
    }
    #[cfg(not(target_os = "linux"))]
    let _ = pin;
}

/// Identity of a file's contents, for the cache.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Stamp {
    pub size: u64,
    pub mtime: u64,
}

impl Stamp {
    pub fn of(path: &Path) -> Option<Self> {
        let m = std::fs::metadata(path).ok()?;
        let mtime = m.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs();
        Some(Self { size: m.len(), mtime })
    }
}

#[derive(Serialize, Deserialize)]
struct Entry {
    stamp: Stamp,
    analysis: Analysis,
}

#[derive(Serialize, Deserialize, Default)]
struct CacheFile {
    version: u32,
    entries: HashMap<String, Entry>,
}

/// Measures by file path, kept while the file is unchanged.
pub struct Cache {
    path: PathBuf,
    file: CacheFile,
    pub dirty: bool,
}

impl Cache {
    pub fn open(path: PathBuf) -> Self {
        let file = std::fs::read_to_string(&path)
            .ok()
            .and_then(|t| serde_json::from_str::<CacheFile>(&t).ok())
            .filter(|f| f.version == measure::VERSION)
            .unwrap_or(CacheFile { version: measure::VERSION, ..Default::default() });
        Self { path, file, dirty: false }
    }

    pub fn get(&self, path: &Path) -> Option<Analysis> {
        let e = self.file.entries.get(path.to_str()?)?;
        (Some(e.stamp) == Stamp::of(path)).then(|| e.analysis.clone())
    }

    pub fn put(&mut self, path: &Path, stamp: Stamp, analysis: Analysis) {
        if let Some(k) = path.to_str() {
            self.file.entries.insert(k.to_string(), Entry { stamp, analysis });
            self.dirty = true;
        }
    }

    pub fn save(&mut self) {
        if !self.dirty {
            return;
        }
        if let Some(d) = self.path.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        let tmp = self.path.with_extension("part");
        if let Ok(text) = serde_json::to_string(&self.file)
            && std::fs::write(&tmp, text).is_ok() && std::fs::rename(&tmp, &self.path).is_ok() {
                self.dirty = false;
            }
    }
}
