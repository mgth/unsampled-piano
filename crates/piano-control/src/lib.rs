//! Live control of a playing instrument, for any host (standalone player,
//! plugin): [`Controller`] (non-real-time, implements
//! [`piano_params::PianoControl`]) and [`EngineSlot`] (audio thread).
//!
//! - A live parameter ([`piano_core::LiveParam`]) goes to the engines as an
//!   [`piano_core::Event::Param`] at the next audio block.
//! - Any other edit changes the precomputed instrument: once the edits
//!   pause ([`DEBOUNCE`]), a worker thread builds a new instrument and
//!   engine and hands it to the audio thread, which swaps it in. Notes
//!   already playing stay on the old engine until they end (at most
//!   [`slot::RETIRE_AFTER_S`], then a short fade); new notes go to the new
//!   one; note offs, pedals and parameters reach both. Edits arriving
//!   during a build restart it with the latest model once it ends.
//! - Replaced engines come back to the worker, which frees them: the audio
//!   thread neither allocates, nor frees, nor locks.
//! - MIDI learn: the audio side translates mapped controllers into live
//!   parameter events and reports every controller; [`Controller::poll`]
//!   feeds them to [`MidiLearn`] and sends its route table back.

pub mod slot;

use std::panic::AssertUnwindSafe;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use piano_core::instrument::Instrument;
use piano_core::params::PianoModel;
use piano_core::{Engine, EngineConfig, Event, LiveParam};
use piano_params::midi::{CcAction, MidiLearn, MidiMap, RouteTable};
use piano_params::{PianoControl, Status, Value};

pub use slot::EngineSlot;

/// Quiet time after the last edit before a rebuild starts.
pub const DEBOUNCE: Duration = Duration::from_millis(150);
/// How often the idle worker frees the engines the audio thread returned.
const RECLAIM_POLL: Duration = Duration::from_millis(50);

/// Counters shared by the audio side, the worker and the controller.
#[derive(Default)]
pub struct Shared {
    pub voices: AtomicU32,
    /// Engines playing (current + finishing their notes).
    pub engines: AtomicU32,
    /// Maximum load ×1e4 since the last [`Controller::status`].
    pub load: AtomicU32,
    /// Generation of the last model requested and of the last one swapped in.
    requested: AtomicU64,
    built: AtomicU64,
    /// Instruments built by the worker (the initial one excluded).
    pub builds: AtomicU64,
}

/// What the worker reports besides the counters.
#[derive(Default)]
struct Report {
    last_build: Option<Duration>,
    error: Option<String>,
}

pub struct Controller {
    model: PianoModel,
    /// The edited model, for the worker: an engine built from an older
    /// model takes the live parameters edited meanwhile.
    edited: Arc<Mutex<PianoModel>>,
    params: rtrb::Producer<Event>,
    requests: Option<mpsc::Sender<(u64, PianoModel)>>,
    generation: u64,
    shared: Arc<Shared>,
    report: Arc<Mutex<Report>>,
    error: Option<String>,
    midi: MidiLearn,
    route_updates: rtrb::Producer<RouteTable>,
    cc_report: rtrb::Consumer<(u8, u8)>,
    worker: Option<std::thread::JoinHandle<()>>,
}

/// Builds the first instrument (here, ≈ 0.1 s) and returns the controller
/// and the audio side. `cfg` gives the sample rate, block size and
/// polyphony; `model` the instrument.
pub fn new(model: PianoModel, cfg: EngineConfig) -> (Controller, EngineSlot) {
    let cfg = EngineConfig { model, ..cfg };
    let t = Instant::now();
    let engine = Box::new(Engine::with_instrument(cfg, Arc::new(Instrument::new(model, cfg.sample_rate))));
    let report = Arc::new(Mutex::new(Report { last_build: Some(t.elapsed()), error: None }));
    let shared = Arc::new(Shared::default());
    let (params_tx, params_rx) = rtrb::RingBuffer::new(1024);
    let (engine_tx, engine_rx) = rtrb::RingBuffer::new(2);
    let (reclaim_tx, reclaim_rx) = rtrb::RingBuffer::new(16);
    let (routes_tx, routes_rx) = rtrb::RingBuffer::new(4);
    let (cc_tx, cc_rx) = rtrb::RingBuffer::new(1024);
    let (req_tx, req_rx) = mpsc::channel();
    let edited = Arc::new(Mutex::new(model));
    let worker = {
        let (edited, shared, report) = (edited.clone(), shared.clone(), report.clone());
        std::thread::Builder::new()
            .name("piano-rebuild".into())
            .spawn(move || worker(cfg, req_rx, engine_tx, reclaim_rx, edited, shared, report))
            .expect("spawn the rebuild worker")
    };
    let slot = EngineSlot::new(engine, engine_rx, reclaim_tx, params_rx, routes_rx, cc_tx, shared.clone());
    let controller = Controller {
        model,
        edited,
        params: params_tx,
        requests: Some(req_tx),
        generation: 0,
        shared,
        report,
        error: None,
        midi: MidiLearn::new(MidiMap::default()),
        route_updates: routes_tx,
        cc_report: cc_rx,
        worker: Some(worker),
    };
    (controller, slot)
}

impl Controller {
    /// Handles the controllers the audio side received (MIDI learn) and
    /// sends it the route table when the map changed. Call it regularly
    /// (the interface does at each frame; a headless host every ~50 ms).
    pub fn poll(&mut self) {
        while let Ok((cc, value)) = self.cc_report.pop() {
            match self.midi.on_cc(cc, value) {
                // Already applied by the audio thread.
                CcAction::Set { path, value, live: true } => {
                    self.update(&path, value);
                }
                CcAction::Set { path, value, live: false } => self.set(&path, value),
                CcAction::Learnt { .. } | CcAction::None => {}
            }
        }
        if let Some(t) = self.midi.take_routes() {
            // Full only if the audio thread stopped; the next change resends.
            let _ = self.route_updates.push(t);
        }
    }

    /// Replaces the MIDI map (e.g. loaded from the user's configuration).
    pub fn set_midi_map(&mut self, map: MidiMap) {
        self.midi.replace(map);
        self.poll();
    }

    pub fn shared(&self) -> &Arc<Shared> {
        &self.shared
    }

    /// Updates the edited model only. Returns whether it changed.
    fn update(&mut self, path: &str, value: Value) -> bool {
        match piano_params::set(&self.model, path, value) {
            Ok(m) => {
                self.model = m;
                *self.edited.lock().unwrap() = m;
                self.error = None;
                true
            }
            Err(e) => {
                self.error = Some(e);
                false
            }
        }
    }

    fn request_rebuild(&mut self) {
        self.generation += 1;
        self.shared.requested.store(self.generation, Ordering::Relaxed);
        if let Some(tx) = &self.requests {
            let _ = tx.send((self.generation, self.model));
        }
    }
}

impl PianoControl for Controller {
    fn model(&self) -> PianoModel {
        self.model
    }

    fn begin_edit(&mut self, _path: &str) {}

    fn set(&mut self, path: &str, value: Value) {
        if !self.update(path, value) {
            return;
        }
        match LiveParam::from_path(path) {
            // A full queue only loses the immediate change: the model keeps
            // it for the next rebuild.
            Some(id) => {
                let _ = self.params.push(Event::Param { id, value: value.as_f64() as f32 });
            }
            None => self.request_rebuild(),
        }
    }

    fn end_edit(&mut self, _path: &str) {}

    fn load(&mut self, model: PianoModel) {
        self.model = model;
        *self.edited.lock().unwrap() = model;
        self.error = None;
        self.request_rebuild();
    }

    fn status(&self) -> Status {
        let report = self.report.lock().unwrap();
        Status {
            rebuilding: self.shared.requested.load(Ordering::Relaxed) != self.shared.built.load(Ordering::Relaxed),
            last_build: report.last_build,
            voices: self.shared.voices.load(Ordering::Relaxed),
            load: self.shared.load.swap(0, Ordering::Relaxed) as f32 / 1e4,
            error: self.error.clone().or_else(|| report.error.clone()),
        }
    }

    fn midi(&mut self) -> Option<&mut MidiLearn> {
        Some(&mut self.midi)
    }
}

impl Drop for Controller {
    fn drop(&mut self) {
        // Closing the request channel stops the worker.
        self.requests = None;
        if let Some(w) = self.worker.take() {
            let _ = w.join();
        }
    }
}

/// Rebuild thread: waits for edits to pause, builds, hands the engine over,
/// and frees the engines the audio thread returns.
fn worker(
    cfg: EngineConfig,
    requests: mpsc::Receiver<(u64, PianoModel)>,
    mut engines: rtrb::Producer<Box<Engine>>,
    mut reclaim: rtrb::Consumer<Box<Engine>>,
    edited: Arc<Mutex<PianoModel>>,
    shared: Arc<Shared>,
    report: Arc<Mutex<Report>>,
) {
    let free = |reclaim: &mut rtrb::Consumer<Box<Engine>>| while reclaim.pop().is_ok() {};
    let mut pending: Option<(u64, PianoModel)> = None;
    loop {
        let (mut generation, mut model) = match pending.take() {
            Some(p) => p,
            None => match requests.recv_timeout(RECLAIM_POLL) {
                Ok(p) => p,
                Err(RecvTimeoutError::Timeout) => {
                    free(&mut reclaim);
                    continue;
                }
                Err(RecvTimeoutError::Disconnected) => return,
            },
        };
        // Wait for the edits to pause (a slider being dragged).
        loop {
            match requests.recv_timeout(DEBOUNCE) {
                Ok(p) => (generation, model) = p,
                Err(RecvTimeoutError::Timeout) => break,
                Err(RecvTimeoutError::Disconnected) => return,
            }
            free(&mut reclaim);
        }
        let t = Instant::now();
        let built = std::panic::catch_unwind(AssertUnwindSafe(|| {
            let inst = Arc::new(Instrument::new(model, cfg.sample_rate));
            Box::new(Engine::with_instrument(EngineConfig { model, ..cfg }, inst))
        }));
        let mut engine = match built {
            Ok(e) => e,
            Err(_) => {
                report.lock().unwrap().error = Some("the instrument could not be built from these parameters".into());
                shared.built.store(generation, Ordering::Relaxed);
                continue;
            }
        };
        {
            let mut r = report.lock().unwrap();
            r.last_build = Some(t.elapsed());
            r.error = None;
        }
        shared.builds.fetch_add(1, Ordering::Relaxed);
        // Edits made during the build: start again from the latest.
        if let Some(p) = requests.try_iter().last() {
            pending = Some(p);
            continue;
        }
        engine.set_live(&edited.lock().unwrap());
        // Hand it over; the audio side takes it at its next block.
        loop {
            match engines.push(engine) {
                Ok(()) => break,
                Err(rtrb::PushError::Full(e)) => {
                    engine = e;
                    free(&mut reclaim);
                    std::thread::sleep(Duration::from_millis(10));
                    if let Some(p) = requests.try_iter().last() {
                        pending = Some(p);
                        break;
                    }
                }
            }
        }
        if pending.is_none() {
            shared.built.store(generation, Ordering::Relaxed);
        }
        free(&mut reclaim);
    }
}
