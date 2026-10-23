//! Audio side: the engine playing now, and the engines it replaced, which
//! finish their notes. No allocation, no lock, no free: new engines arrive
//! built through a ring, replaced ones leave through another to be dropped
//! by the worker.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use piano_core::{Engine, Event, TimedEvent};
use piano_params::midi::RouteTable;

use crate::Shared;

/// A replaced engine plays its notes for at most this long, then fades out.
pub const RETIRE_AFTER_S: f64 = 2.0;
const FADE_S: f64 = 0.1;
/// Fade of the oldest replaced engine when more than `KEEP` are playing.
const FAST_FADE_S: f64 = 0.01;
const KEEP: usize = 2;
const SLOTS: usize = KEEP + 1;
/// A replaced engine leaves once no voice sounds and its output stayed
/// under −100 dBFS for 0.1 s (room tail included).
const QUIET_LEVEL: f32 = 1e-5;
const QUIET_S: f64 = 0.1;
const MAX_EVENTS: usize = 1024;
/// Pedals (sustain, sostenuto, una corda) replayed into a new engine.
const PEDALS: [u8; 3] = [64, 66, 67];

struct Retiring {
    engine: Box<Engine>,
    /// Samples played since it was replaced.
    age: usize,
    /// Fade-out: (samples left, length).
    fade: Option<(usize, usize)>,
    /// Samples of silence so far.
    quiet: usize,
}

pub struct EngineSlot {
    current: Box<Engine>,
    retiring: [Option<Retiring>; SLOTS],
    incoming: rtrb::Consumer<Box<Engine>>,
    reclaim: rtrb::Producer<Box<Engine>>,
    /// Replaced engines waiting for room in `reclaim`.
    unsent: [Option<Box<Engine>>; SLOTS + 1],
    params: rtrb::Consumer<Event>,
    /// MIDI learn: controllers mapped to live parameters (replaced whole
    /// from `route_updates`), and every controller received, reported to
    /// the non-real-time side.
    routes: RouteTable,
    route_updates: rtrb::Consumer<RouteTable>,
    cc_report: rtrb::Producer<(u8, u8)>,
    /// Last value of each pedal controller seen.
    pedals: [Option<u8>; PEDALS.len()],
    ev_new: Vec<TimedEvent>,
    ev_old: Vec<TimedEvent>,
    l: Vec<f32>,
    r: Vec<f32>,
    sample_rate: f64,
    shared: Arc<Shared>,
}

impl EngineSlot {
    pub(crate) fn new(
        engine: Box<Engine>,
        incoming: rtrb::Consumer<Box<Engine>>,
        reclaim: rtrb::Producer<Box<Engine>>,
        params: rtrb::Consumer<Event>,
        route_updates: rtrb::Consumer<RouteTable>,
        cc_report: rtrb::Producer<(u8, u8)>,
        shared: Arc<Shared>,
    ) -> Self {
        let cfg = *engine.config();
        Self {
            current: engine,
            retiring: std::array::from_fn(|_| None),
            incoming,
            reclaim,
            unsent: std::array::from_fn(|_| None),
            params,
            routes: RouteTable::default(),
            route_updates,
            cc_report,
            pedals: [None; PEDALS.len()],
            ev_new: Vec::with_capacity(MAX_EVENTS),
            ev_old: Vec::with_capacity(MAX_EVENTS),
            l: vec![0.0; cfg.max_block],
            r: vec![0.0; cfg.max_block],
            sample_rate: cfg.sample_rate,
            shared,
        }
    }

    /// The engine new notes go to.
    pub fn current(&self) -> &Engine {
        &self.current
    }

    /// Engines playing: the current one and those finishing their notes.
    pub fn engines(&self) -> usize {
        1 + self.retiring.iter().flatten().count()
    }

    pub fn active_voices(&self) -> usize {
        self.current.active_voices() + self.retiring.iter().flatten().map(|r| r.engine.active_voices()).sum::<usize>()
    }

    /// Audio callback time over buffer duration, for [`crate::Controller`]'s
    /// status (the maximum since it was last read).
    pub fn report_load(&self, load: f32) {
        self.shared.load.fetch_max((load * 1e4) as u32, Ordering::Relaxed);
    }

    /// Renders `left.len()` (≤ the engines' max block) samples, overwriting
    /// `left` and `right`. Events are relative to the block, in order. Note
    /// ons go to the current engine; note offs, controllers and parameters
    /// to every engine. Controllers are reported for MIDI learn, then those
    /// mapped to a live parameter become [`Event::Param`].
    pub fn process_stereo(&mut self, events: &[TimedEvent], left: &mut [f32], right: &mut [f32]) {
        let len = left.len();
        while let Ok(t) = self.route_updates.pop() {
            self.routes = t;
        }
        let mut swapped = false;
        while let Ok(engine) = self.incoming.pop() {
            let old = std::mem::replace(&mut self.current, engine);
            if swapped {
                // Replaced within the same block: it never played.
                self.send_back(old);
            } else {
                self.retire(old);
            }
            swapped = true;
        }

        self.ev_new.clear();
        self.ev_old.clear();
        if swapped {
            for (cc, v) in PEDALS.iter().zip(self.pedals) {
                if let Some(value) = v {
                    self.ev_new.push(TimedEvent { frame: 0, event: Event::Control { cc: *cc, value } });
                }
            }
        }
        while self.ev_new.len() < MAX_EVENTS / 2 {
            let Ok(event) = self.params.pop() else { break };
            self.ev_new.push(TimedEvent { frame: 0, event });
            self.ev_old.push(TimedEvent { frame: 0, event });
        }
        for &e in events {
            if self.ev_new.len() == MAX_EVENTS {
                break;
            }
            if let Event::Control { cc, value } = e.event {
                let _ = self.cc_report.push((cc, value));
                if let Some(i) = PEDALS.iter().position(|&p| p == cc) {
                    self.pedals[i] = Some(value);
                }
            }
            let e = TimedEvent { frame: e.frame, event: self.routes.translate(e.event) };
            self.ev_new.push(e);
            if !matches!(e.event, Event::NoteOn { .. }) {
                self.ev_old.push(e);
            }
        }

        self.current.process_stereo(&self.ev_new, left, right);
        let quiet_after = (QUIET_S * self.sample_rate) as usize;
        let retire_after = (RETIRE_AFTER_S * self.sample_rate) as usize;
        let fade_len = (FADE_S * self.sample_rate) as usize;
        for i in 0..SLOTS {
            let Some(r) = self.retiring[i].as_mut() else { continue };
            let (l, rr) = (&mut self.l[..len], &mut self.r[..len]);
            r.engine.process_stereo(&self.ev_old, l, rr);
            let peak = l.iter().chain(rr.iter()).fold(0.0f32, |m, x| m.max(x.abs()));
            let done = match r.fade {
                Some((left_n, total)) => {
                    for (k, ((o_l, o_r), (x, y))) in left.iter_mut().zip(right.iter_mut()).zip(l.iter().zip(rr.iter())).enumerate() {
                        let g = left_n.saturating_sub(k) as f32 / total as f32;
                        *o_l += g * x;
                        *o_r += g * y;
                    }
                    let left_n = left_n.saturating_sub(len);
                    r.fade = Some((left_n, total));
                    left_n == 0
                }
                None => {
                    for (o, x) in left.iter_mut().zip(l.iter()).chain(right.iter_mut().zip(rr.iter())) {
                        *o += x;
                    }
                    r.age += len;
                    if peak < QUIET_LEVEL && r.engine.sounding_voices() == 0 {
                        r.quiet += len;
                    } else {
                        r.quiet = 0;
                    }
                    if r.age >= retire_after {
                        r.fade = Some((fade_len, fade_len));
                    }
                    r.quiet >= quiet_after
                }
            };
            if done && let Some(r) = self.retiring[i].take() {
                self.send_back(r.engine);
            }
        }
        for i in 0..self.unsent.len() {
            if let Some(e) = self.unsent[i].take()
                && let Err(rtrb::PushError::Full(e)) = self.reclaim.push(e)
            {
                self.unsent[i] = Some(e);
            }
        }
        self.shared.voices.store(self.active_voices() as u32, Ordering::Relaxed);
        self.shared.engines.store(self.engines() as u32, Ordering::Relaxed);
    }

    /// The current engine was just replaced: it finishes its notes.
    fn retire(&mut self, engine: Box<Engine>) {
        let playing = self.retiring.iter().flatten().filter(|r| r.fade.is_none()).count();
        if playing >= KEEP {
            // Too many: the oldest still playing fades out quickly.
            let fast = (FAST_FADE_S * self.sample_rate) as usize;
            if let Some(oldest) =
                self.retiring.iter_mut().flatten().filter(|r| r.fade.is_none()).max_by_key(|r| r.age)
            {
                oldest.fade = Some((fast, fast));
            }
        }
        let slot = match self.retiring.iter().position(Option::is_none) {
            Some(i) => i,
            None => {
                // Every slot busy fading (swaps faster than the fades): cut
                // the one closest to the end of its fade.
                let i = (0..SLOTS)
                    .min_by_key(|&i| self.retiring[i].as_ref().and_then(|r| r.fade).map_or(usize::MAX, |f| f.0))
                    .unwrap_or(0);
                if let Some(r) = self.retiring[i].take() {
                    self.send_back(r.engine);
                }
                i
            }
        };
        self.retiring[slot] = Some(Retiring { engine, age: 0, fade: None, quiet: 0 });
    }

    /// Hands a replaced engine to the worker, which frees it.
    fn send_back(&mut self, engine: Box<Engine>) {
        if let Err(rtrb::PushError::Full(engine)) = self.reclaim.push(engine) {
            match self.unsent.iter().position(Option::is_none) {
                Some(i) => self.unsent[i] = Some(engine),
                // Unreachable in practice (the ring outsizes the slots and
                // the worker drains it): freeing here beats losing track.
                None => drop(engine),
            }
        }
    }
}
