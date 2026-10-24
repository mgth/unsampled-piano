//! Parallel map for the instrument build (off the audio thread): the note
//! table and the radiation integrals are independent per key and per mode.
//! [`Pool`]: persistent workers for the audio thread (voices in parallel).

use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// `(0..n).map(f).collect()` over half the hardware threads, in order.
/// Items are handed out one by one (their costs differ widely: bass keys,
/// low modes). Half: an instrument is rebuilt while the piano plays, and a
/// build on every hardware thread also takes the audio thread's SMT
/// sibling, slowing it down for the build's duration.
pub fn map<T: Send, F: Fn(usize) -> T + Sync>(n: usize, f: F) -> Vec<T> {
    let threads = std::thread::available_parallelism()
        .map_or(1, |p| (p.get() / 2).max(1))
        .min(n);
    if threads <= 1 {
        return (0..n).map(f).collect();
    }
    let next = AtomicUsize::new(0);
    let mut parts: Vec<Vec<(usize, T)>> = std::thread::scope(|s| {
        let workers: Vec<_> = (0..threads)
            .map(|_| {
                s.spawn(|| {
                    let mut out = Vec::new();
                    loop {
                        let i = next.fetch_add(1, Ordering::Relaxed);
                        if i >= n {
                            break out;
                        }
                        out.push((i, f(i)));
                    }
                })
            })
            .collect();
        workers
            .into_iter()
            .map(|w| w.join().expect("build worker panicked"))
            .collect()
    });
    let mut slots: Vec<Option<T>> = (0..n).map(|_| None).collect();
    for (i, v) in parts.iter_mut().flat_map(std::mem::take) {
        slots[i] = Some(v);
    }
    slots
        .into_iter()
        .map(|v| v.expect("every item mapped"))
        .collect()
}

/// How long an idle worker spins before parking: covers the gap between
/// the sub-blocks of one host buffer, not the wait for the next buffer.
const SPIN: Duration = Duration::from_micros(200);

type Job = *const (dyn Fn(usize) + Sync);

/// The work of one [`Pool::run`] in one word, so that an item can only be
/// claimed for the run it belongs to: generation (32 bits), item count and
/// next item (16 bits each).
fn ticket(generation: u64, n: usize, next: usize) -> u64 {
    (generation << 32) | ((n as u64) << 16) | next as u64
}

struct Shared {
    ticket: AtomicU64,
    job: UnsafeCell<Job>,
    done: AtomicUsize,
    stop: AtomicBool,
}

// SAFETY: `job` is written by the owner before the Release store of the
// run's ticket and read by a worker only after claiming an item of that
// run (Acquire); the owner does not return from `run`, so the closure stays
// alive, before every item it handed out is done.
unsafe impl Sync for Shared {}
unsafe impl Send for Shared {}

/// A fixed set of worker threads running `f(0..n)` with the calling thread,
/// items handed out one at a time. No allocation and no lock in [`Pool::run`]
/// (real-time safe apart from waking parked workers). Results must not
/// depend on which thread runs an item: each item writes its own output.
pub struct Pool {
    shared: Arc<Shared>,
    workers: Vec<JoinHandle<()>>,
    generation: u64,
}

impl Pool {
    /// `threads` counts the calling thread: `threads − 1` workers.
    pub fn new(threads: usize) -> Self {
        Self::with_init(threads, || {})
    }

    /// Like [`Self::new`]; each worker first runs `init` (e.g. to take the
    /// audio thread's real-time priority).
    pub fn with_init(threads: usize, init: impl Fn() + Send + Sync + Clone + 'static) -> Self {
        let noop: &'static (dyn Fn(usize) + Sync) = &|_| {};
        let shared = Arc::new(Shared {
            ticket: AtomicU64::new(0),
            job: UnsafeCell::new(noop as Job),
            done: AtomicUsize::new(0),
            stop: AtomicBool::new(false),
        });
        let workers = (1..threads.max(1))
            .map(|i| {
                let (sh, init) = (shared.clone(), init.clone());
                std::thread::Builder::new()
                    .name(format!("piano-voice-{i}"))
                    .spawn(move || {
                        init();
                        worker(&sh)
                    })
                    .expect("spawn voice worker")
            })
            .collect();
        Self { shared, workers, generation: 0 }
    }

    pub fn threads(&self) -> usize {
        self.workers.len() + 1
    }

    /// Runs `f(i)` for every i in 0..n (n < 65536) and returns when all are done.
    pub fn run(&mut self, n: usize, f: &(dyn Fn(usize) + Sync)) {
        assert!(n < 1 << 16);
        if n == 0 {
            return;
        }
        let sh = &*self.shared;
        self.generation = (self.generation + 1) & 0xffff_ffff;
        sh.done.store(0, Ordering::Relaxed);
        // SAFETY: the previous run's items are all done and no worker can
        // claim one of this run before the ticket below is published.
        unsafe { *sh.job.get() = std::mem::transmute::<&(dyn Fn(usize) + Sync), Job>(f) };
        sh.ticket.store(ticket(self.generation, n, 0), Ordering::Release);
        for w in &self.workers {
            w.thread().unpark();
        }
        take_items(sh, self.generation);
        while sh.done.load(Ordering::Acquire) < n {
            std::hint::spin_loop();
        }
    }
}

/// Claims and runs items of run `generation` until there are none left.
fn take_items(sh: &Shared, generation: u64) {
    loop {
        let t = sh.ticket.load(Ordering::Acquire);
        let (n, i) = (((t >> 16) & 0xffff) as usize, (t & 0xffff) as usize);
        if t >> 32 != generation || i >= n {
            return;
        }
        if sh.ticket.compare_exchange_weak(t, t + 1, Ordering::AcqRel, Ordering::Relaxed).is_ok() {
            // SAFETY: see `Shared`.
            let f = unsafe { &**sh.job.get() };
            f(i);
            sh.done.fetch_add(1, Ordering::Release);
        }
    }
}

fn worker(sh: &Shared) {
    let _ftz = crate::denormal::DenormalGuard::new();
    let mut seen = 0u64;
    loop {
        let t = Instant::now();
        let generation = loop {
            let g = sh.ticket.load(Ordering::Acquire) >> 32;
            if g != seen || sh.stop.load(Ordering::Relaxed) {
                break g;
            }
            if t.elapsed() < SPIN {
                std::hint::spin_loop();
            } else {
                std::thread::park_timeout(Duration::from_millis(50));
            }
        };
        if sh.stop.load(Ordering::Relaxed) {
            return;
        }
        seen = generation;
        take_items(sh, generation);
    }
}

impl Drop for Pool {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::Relaxed);
        for w in self.workers.drain(..) {
            w.thread().unpark();
            let _ = w.join();
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn keeps_order() {
        let v = super::map(1000, |i| i * i);
        assert!(v.iter().enumerate().all(|(i, &x)| x == i * i));
        assert!(super::map(0, |i| i).is_empty());
    }

    #[test]
    fn pool_runs_every_item_once() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let mut pool = super::Pool::new(4);
        for n in [0usize, 1, 3, 100, 1000] {
            let hits: Vec<AtomicUsize> = (0..n).map(|_| AtomicUsize::new(0)).collect();
            pool.run(n, &|i| {
                hits[i].fetch_add(1, Ordering::Relaxed);
            });
            assert!(hits.iter().all(|h| h.load(Ordering::Relaxed) == 1));
        }
    }
}
