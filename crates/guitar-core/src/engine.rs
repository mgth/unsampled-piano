//! Real-time engine: one modal bank and one contact solver per string, MIDI
//! notes assigned to strings and frets.
//!
//! A note goes to the lowest fret that reaches it on a string not already
//! held, else it takes over the lowest-fret string (a guitar string plays one
//! note at a time). Pressing a new fret on a string that rings carries its
//! motion over to the new length ([`crate::project`]); then the right hand
//! plays it ([`Technique`]):
//!
//! - a pluck: ideal (the motion replaced by a triangle), or a fingertip or
//!   plectrum in the contact solver (it lands on the string, drags it,
//!   slips off at the pluck's force);
//! - a slap: the thumb strikes the string toward the frets;
//! - a pop: a finger pulls it away from the board and lets go.
//!
//! The left hand can also play on its own:
//!
//! - a tap or hammer-on: a finger drives the string onto the fret, and once
//!   it is pressed there the vibrating length becomes the fret's;
//! - a pull-off: the length grows to the lower fret, and the leaving finger
//!   plucks the string sideways.
//!
//! The frets are obstacles under the string ([`crate::frets`]). A string
//! runs in its modal bank as long as no fret can be touched:
//!
//! - the reach bound picks the frets to watch (most of the time none);
//! - for those, the bank runs in chunks of [`WATCH`] samples, and after each
//!   one the actual displacement there, plus the velocity bound times the
//!   chunk's length, must stay below the gap.
//!
//! Otherwise it runs in the contact solver, the frets within reach as
//! contacts, until no fret has pushed and no hand has touched it for
//! [`IDLE`]. Buzz, the slap's rattle and the click of a hammer-on come out
//! of these contacts.
//!
//! At note-off the fretting finger lifts: the string goes back to its open
//! length, damped by the fingertip still touching it behind the fret. The
//! strings' summed output goes through the output stage
//! ([`crate::output`]): pickups' circuit, piezo, or as it is.

use std::sync::Arc;

use piano_core::complex::C64;
use piano_core::denormal::DenormalGuard;
use piano_core::modal::ModalBank;

use crate::contact::{Contact, ContactSolver, Law, Motion, Release};
use crate::frets::reach;
use crate::instrument::Instrument;
use crate::output::OutputStage;
pub use crate::params::Technique;
use crate::params::PluckModel;
use crate::project::{add_rest_change, project};
use crate::string::{fret_length, StringTable, PAR, PERP};

/// Output-weighted energy below which a string stops being computed
/// (amplitude ≈ −120 dB re full scale).
const SILENCE: f32 = 1e-12;

/// Contact groups: the right hand, the left hand, the frets.
const RIGHT_HAND: u8 = 0;
const LEFT_HAND: u8 = 1;
const FRETS: u8 = 2;
/// Samples between checks of the frets within reach, in the solver, and
/// of the watched frets' clearance, in the bank.
const CHECK: usize = 16;
pub const WATCH: usize = 8;
/// The solver hands back to the bank after this long without a contact
/// force (s).
pub const IDLE: f64 = 0.01;
/// A fret becomes a contact when the string could reach its gap before the
/// next check: displacement there plus the interval times the velocity
/// bound (and the hands' speed), and stops being one below 90 % of it with
/// no force on it. While a hand drives the string these become 90 % and
/// 80 % (its push is not bounded by the string's own motion).
const ENGAGE: f64 = 0.9;
const DISENGAGE: f64 = 0.8;
const MAX_FRETS: usize = 36;
/// Where a tapping finger starts above the string (m), and how long it may
/// press before the fret takes over anyway (s).
const TAP_CLEARANCE: f64 = 3e-3;
const TAP_TIMEOUT: f64 = 0.03;
/// The fret takes over once string, fret and finger have touched this long (s).
const TAP_PRESSED: f64 = 1e-3;
/// Where the thumb starts above the string (m).
const SLAP_CLEARANCE: f64 = 2e-3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    NoteOn { note: u8, velocity: u8 },
    /// A note played with a given technique (NoteOn plucks).
    Note { note: u8, velocity: u8, technique: Technique },
    NoteOff { note: u8 },
    AllNotesOff,
}

/// Which table the string's bank runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stop {
    Fret(u8),
    /// Open length, the finger lifting off this fret.
    Released(u8),
}

/// A finger driving the string onto a fret.
#[derive(Clone, Copy, Debug)]
struct Hammer {
    fret: u8,
    fret_slot: usize,
    finger_slot: usize,
    pressed: usize,
    elapsed: usize,
    timeout: usize,
}

struct StringVoice {
    bank: ModalBank,
    solver: ContactSolver,
    /// Key holding the string down, if any.
    held: Option<u8>,
    stop: Stop,
    muted: bool,
    sounding: bool,
    in_contact: bool,
    /// The key went up while a hand touched the string: lift the finger
    /// once it is done.
    release_pending: bool,
    /// Solver slot of each fret that is a contact.
    fret_slots: [Option<usize>; MAX_FRETS],
    hammer: Option<Hammer>,
    /// Largest string–fret force since the last note (N), for checks.
    fret_force: f64,
    /// Frets watched in the bank: index in the table's frets, velocity
    /// bound there (m/s).
    watch: Vec<(usize, f64)>,
    /// Solver samples without any contact force.
    idle: usize,
    /// Per-polarisation amplitudes, scratch for projections, perpendicular
    /// amplitudes for the reach bound.
    zeta: Vec<[C64; 2]>,
    scratch: Vec<[C64; 2]>,
    amps: Vec<f64>,
}

pub struct Engine {
    inst: Arc<Instrument>,
    strings: Vec<StringVoice>,
    max_block: usize,
    stage: OutputStage,
    /// Perpendicular bridge force of the block (kept for piezo/acoustic
    /// mixes to come).
    force: Vec<f32>,
}

fn table(inst: &Instrument, s: usize, stop: Stop) -> &StringTable {
    match stop {
        Stop::Fret(f) => inst.table(s, f),
        Stop::Released(f) => inst.released(s, f),
    }
}

fn speed_samples(distance: f64, speed: f64, sr: f64) -> usize {
    // A raised cosine over T peaks at π·d/(2T): arrive at `speed`.
    ((std::f64::consts::PI * distance.abs() / (2.0 * speed.max(1e-3))) * sr).round().max(1.0) as usize
}

impl StringVoice {
    /// ζ of the bank's state on `t` (zero when silent).
    fn read(&mut self, t: &StringTable) {
        for (n, p) in t.partials.iter().enumerate() {
            self.zeta[n] = if self.sounding {
                let (a, b) = (self.bank.state(2 * n), self.bank.state(2 * n + 1));
                p.polar_of([C64::new(a.0, a.1), C64::new(b.0, b.1)])
            } else {
                [C64::ZERO; 2]
            };
        }
    }

    /// Loads `t` in the bank with ζ as its state.
    fn write(&mut self, t: &StringTable) {
        self.bank.load(&t.modes);
        for (n, p) in t.partials.iter().enumerate() {
            let z = p.coupled_of(self.zeta[n]);
            self.bank.set_state(2 * n, z[0].re, z[0].im);
            self.bank.set_state(2 * n + 1, z[1].re, z[1].im);
        }
        self.bank.set_damping(if self.muted { 1.0 } else { 0.0 });
    }

    /// Starts the solver on the bank's state, if not running.
    fn begin(&mut self, t: &StringTable) {
        if self.in_contact {
            return;
        }
        self.read(t);
        self.solver.begin(t, &self.zeta, if self.muted { 1.0 } else { 0.0 });
        self.fret_slots = [None; MAX_FRETS];
        self.in_contact = true;
        self.idle = 0;
    }

    /// A hand still touches the string.
    fn busy(&self) -> bool {
        self.in_contact && (self.solver.has_group(RIGHT_HAND) || self.solver.has_group(LEFT_HAND))
    }

    /// Ends a contact episode: the solver's state back to the bank.
    fn end_contact(&mut self, inst: &Instrument, s: usize) {
        if !self.in_contact {
            return;
        }
        let t = table(inst, s, self.stop);
        for n in 0..t.len() {
            self.zeta[n] = [self.solver.complex_state(PERP, n), self.solver.complex_state(PAR, n)];
        }
        self.in_contact = false;
        self.hammer = None;
        self.write(t);
        if self.release_pending {
            self.release_pending = false;
            self.lift(inst, s);
        }
    }

    /// Moves the string to `stop`, its motion projected (a new fret, or the
    /// finger lifting). `sudden`: the string changes rest line at once (a
    /// hammer-on landing, a pull-off, a finger lifting), so it starts
    /// displaced from the new one; a finger pressing a fret before a pluck
    /// bends it there quasi-statically, without exciting it.
    fn restop(&mut self, inst: &Instrument, s: usize, stop: Stop, sudden: bool) {
        self.end_contact(inst, s);
        let (from, to) = (table(inst, s, self.stop), table(inst, s, stop));
        self.read(from);
        // Same length (an open string muted): the same modes, no projection.
        if from.length != to.length {
            if self.sounding {
                project(from, to, &self.zeta, &mut self.scratch);
            } else {
                self.scratch[..to.len()].fill([C64::ZERO; 2]);
            }
            if sudden {
                // Seen from its new rest line, the string starts displaced.
                add_rest_change(from, to, &mut self.scratch);
                self.sounding = true;
            }
            std::mem::swap(&mut self.zeta, &mut self.scratch);
        }
        self.stop = stop;
        self.write(to);
    }

    /// The fretting finger lifts off (or the hand mutes an open string).
    fn lift(&mut self, inst: &Instrument, s: usize) {
        if self.busy() {
            self.release_pending = true;
            return;
        }
        self.end_contact(inst, s);
        // Muted strings run their muted table (its output follows the mute).
        self.muted = true;
        match self.stop {
            Stop::Fret(f) => self.restop(inst, s, Stop::Released(f), true),
            Stop::Released(_) => self.bank.set_damping(1.0),
        }
    }

    /// Perpendicular amplitudes |ζ_n| now, from the solver or the bank.
    fn amplitudes(&mut self, t: &StringTable) {
        if self.in_contact {
            for n in 0..t.len() {
                self.amps[n] = self.solver.complex_state(PERP, n).norm();
            }
        } else {
            self.read(t);
            for n in 0..t.len() {
                self.amps[n] = self.zeta[n][PERP].norm();
            }
        }
    }

    /// In the bank: the frets the reach bound cannot rule out, to watch.
    /// In the solver: the frets that could be touched within the next
    /// [`CHECK`] samples become contacts, and contacts that could not and
    /// push nothing are dropped. While a hand drives the string, the reach
    /// bound alone decides (a thumb comes down fast), else the actual
    /// displacement plus the velocity bound times the interval.
    fn engage(&mut self, inst: &Instrument, s: usize) {
        self.watch.clear();
        if !self.sounding {
            return;
        }
        let t = table(inst, s, self.stop);
        self.amplitudes(t);
        let amps = &self.amps[..t.len()];
        let total: f64 = amps.iter().sum();
        let fb = &inst.model.fretboard;
        let law = Law::Unilateral { stiffness: fb.stiffness, exponent: fb.exponent, damping: fb.damping };
        let driven = self.busy() || self.hammer.is_some();
        let tau = CHECK as f64 / inst.sample_rate;
        let hand = if self.in_contact { self.solver.target_speed() } else { 0.0 };
        for (i, g) in t.frets.iter().enumerate() {
            let slot = if self.in_contact { self.fret_slots[g.fret as usize] } else { None };
            // Σ|ζ| bounds every fret's reach: most of the time, done here.
            if slot.is_none() && total < if self.in_contact { ENGAGE } else { 1.0 } * g.gap {
                continue;
            }
            let r = reach(t.length, g.x, amps.iter().copied());
            if !self.in_contact {
                if r >= g.gap {
                    let v = reach(t.length, g.x, amps.iter().zip(&t.partials).map(|(a, p)| a * p.omega));
                    self.watch.push((i, v));
                }
                continue;
            }
            let share = match (driven, slot.is_some()) {
                (true, true) => DISENGAGE,
                (true, false) | (false, true) => ENGAGE,
                (false, false) => 1.0,
            };
            let near = r >= share * g.gap && {
                let v = reach(t.length, g.x, amps.iter().zip(&t.partials).map(|(a, p)| a * p.omega));
                self.solver.displacement(g.x, PERP) + tau * (v + hand) >= share * g.gap
            };
            match slot {
                None if near => {
                    let c = Contact { x: g.x, pol: PERP, sign: 1.0, law, motion: Motion::Fixed(g.gap), release: Release::Never, group: FRETS };
                    self.fret_slots[g.fret as usize] = self.solver.add(c);
                }
                Some(sl) if !near && self.solver.force(sl) == 0.0 && self.hammer.is_none_or(|h| h.fret_slot != sl) => {
                    self.solver.remove(sl);
                    self.fret_slots[g.fret as usize] = None;
                }
                _ => {}
            }
        }
    }

    /// In the bank, after a chunk of `n` samples: whether a watched fret
    /// could be touched within the next one (then the solver takes over).
    fn threatened(&mut self, inst: &Instrument, s: usize, n: usize) -> bool {
        let t = table(inst, s, self.stop);
        // The perpendicular amplitudes only.
        for (n, p) in t.partials.iter().enumerate() {
            let (a, b) = (self.bank.state(2 * n), self.bank.state(2 * n + 1));
            self.zeta[n][PERP] = p.e[PERP][0] * C64::new(a.0, a.1) + p.e[PERP][1] * C64::new(b.0, b.1);
        }
        let tau = n as f64 / inst.sample_rate;
        for &(i, v) in &self.watch {
            let g = t.frets[i];
            let th = std::f64::consts::PI * g.x / t.length;
            let c2 = 2.0 * th.cos();
            let (mut s0, mut s1) = (0.0, th.sin());
            let mut y = 0.0;
            for z in &self.zeta[..t.len()] {
                y += s1 * z[PERP].re;
                (s0, s1) = (s1, c2 * s1 - s0);
            }
            if y + tau * v >= g.gap {
                return true;
            }
        }
        false
    }

    /// Into the solver from the bank, the frets within reach as contacts.
    fn enter(&mut self, inst: &Instrument, s: usize) {
        let t = table(inst, s, self.stop);
        self.begin(t);
        self.engage(inst, s);
    }

    /// After a chunk of solver samples: the hammer, the frets, the end.
    fn after_chunk(&mut self, inst: &Instrument, s: usize, n: usize) {
        let mut pushing = self.busy() || self.hammer.is_some();
        for sl in self.fret_slots.iter().flatten() {
            let f = self.solver.force(*sl);
            self.fret_force = self.fret_force.max(f);
            pushing |= f > 0.0;
        }
        self.idle = if pushing { 0 } else { self.idle + n };
        if let Some(mut h) = self.hammer {
            h.elapsed += n;
            if self.solver.force(h.fret_slot) > 0.0 && self.solver.force(h.finger_slot) > 0.0 {
                h.pressed += n;
            }
            self.hammer = Some(h);
            if h.pressed as f64 >= TAP_PRESSED * inst.sample_rate || h.elapsed >= h.timeout {
                self.complete_hammer(inst, s, h.fret);
                return;
            }
        }
        self.engage(inst, s);
        if self.in_contact && (!self.solver.is_active() || self.idle as f64 >= IDLE * inst.sample_rate) {
            // Free again: back to the bank, watching what is within reach.
            self.end_contact(inst, s);
            self.engage(inst, s);
        }
    }

    /// The string is pressed onto the fret: its vibrating length is now the
    /// fret's, the finger behind it.
    fn complete_hammer(&mut self, inst: &Instrument, s: usize, fret: u8) {
        let pending = std::mem::take(&mut self.release_pending);
        self.hammer = None;
        self.end_contact(inst, s);
        self.muted = false;
        self.restop(inst, s, Stop::Fret(fret), true);
        if pending {
            self.lift(inst, s);
        }
        self.engage(inst, s);
    }

    /// A hand drags the string at `x` in direction `dir` (perpendicular,
    /// parallel) through a spring–dashpot and slips off at `force`.
    #[allow(clippy::too_many_arguments)]
    fn hand_pluck(&mut self, t: &StringTable, x: f64, force: f64, dir: (f64, f64), law: (f64, f64, f64), drag: f64, sr: f64, group: u8) {
        let (stiffness, damping, slip_time) = law;
        let k_string = t.tension * t.length / (x * (t.length - x));
        self.begin(t);
        let drag = ((drag * sr) as usize).max(1);
        let slip = ((slip_time * sr).round() as usize).max(1);
        // Aim twice as far as the slip needs (string and contact in
        // series): the force limit is reached about halfway through.
        let travel = 2.0 * force * (1.0 / k_string + 1.0 / stiffness);
        let law = Law::Spring { stiffness, damping };
        let release = Release::Force { limit: force, slip, timeout: drag };
        for (pol, d) in [(PERP, dir.0), (PAR, dir.1)] {
            let to = self.solver.displacement(x, pol) + travel * d;
            self.solver.add(Contact { x, pol, sign: 1.0, law, motion: Motion::Ramp { to, samples: drag }, release, group });
        }
    }
}

impl Engine {
    /// Allocates everything up front; nothing allocates afterwards.
    pub fn new(inst: Arc<Instrument>, max_block: usize) -> Self {
        let np = inst.max_partials;
        let strings = (0..inst.model.strings.len())
            .map(|_| StringVoice {
                bank: ModalBank::new(inst.sample_rate, 2 * np, max_block),
                solver: ContactSolver::new(np, inst.sample_rate),
                held: None,
                stop: Stop::Fret(0),
                muted: false,
                sounding: false,
                in_contact: false,
                release_pending: false,
                fret_slots: [None; MAX_FRETS],
                hammer: None,
                fret_force: 0.0,
                watch: Vec::with_capacity(MAX_FRETS),
                idle: 0,
                zeta: vec![[C64::ZERO; 2]; np],
                scratch: vec![[C64::ZERO; 2]; np],
                amps: vec![0.0; np],
            })
            .collect();
        let stage = OutputStage::new(&inst.model.output, inst.sample_rate);
        Self { inst, strings, max_block, stage, force: vec![0.0; max_block] }
    }

    pub fn instrument(&self) -> &Arc<Instrument> {
        &self.inst
    }

    /// String and fret playing `note` now, if it sounds.
    pub fn position_of(&self, note: u8) -> Option<(usize, u8)> {
        let i = self.strings.iter().position(|s| s.held == Some(note))?;
        match (self.strings[i].stop, self.strings[i].hammer) {
            (_, Some(h)) => Some((i, h.fret)),
            (Stop::Fret(f), None) => Some((i, f)),
            (Stop::Released(_), None) => None,
        }
    }

    /// Largest string–fret force since the last note on any string (N).
    pub fn fret_force(&self) -> f64 {
        self.strings.iter().map(|s| s.fret_force).fold(0.0, f64::max)
    }

    pub fn handle(&mut self, ev: Event) {
        match ev {
            Event::NoteOn { note, velocity } => self.handle(Event::Note { note, velocity, technique: Technique::Pluck }),
            Event::Note { note, velocity: 0, .. } | Event::NoteOff { note } => self.release(note),
            Event::Note { note, velocity, technique } => self.play(note, velocity, technique),
            Event::AllNotesOff => {
                for s in 0..self.strings.len() {
                    self.strings[s].held = None;
                    self.strings[s].lift(&self.inst, s);
                }
            }
        }
    }

    /// Lowest fret reaching `note` on a free string, else on any.
    fn place(&self, note: u8) -> Option<(usize, u8)> {
        let frets = self.inst.model.frets;
        let mut best: Option<(usize, u8, bool)> = None;
        for (s, p) in self.inst.model.strings.iter().enumerate() {
            if note < p.open_note || note - p.open_note > frets {
                continue;
            }
            let (fret, free) = (note - p.open_note, self.strings[s].held.is_none());
            let better = match best {
                None => true,
                Some((_, bf, bfree)) => (free && !bfree) || (free == bfree && fret < bf),
            };
            if better {
                best = Some((s, fret, free));
            }
        }
        best.map(|(s, f, _)| (s, f))
    }

    /// A held string that reaches `note` above (`up`) or below its fret:
    /// (string, new fret, current fret), the nearest move.
    fn legato(&self, note: u8, up: bool) -> Option<(usize, u8, u8)> {
        let frets = self.inst.model.frets;
        self.strings
            .iter()
            .enumerate()
            .filter_map(|(s, v)| {
                let (Some(_), Stop::Fret(cur), None) = (v.held, v.stop, v.hammer) else { return None };
                let open = self.inst.model.strings[s].open_note;
                let f = note.checked_sub(open).filter(|&f| f <= frets)?;
                ((up && f > cur) || (!up && f < cur)).then_some((s, f, cur))
            })
            .min_by_key(|&(_, f, cur)| f.abs_diff(cur))
    }

    fn play(&mut self, note: u8, velocity: u8, technique: Technique) {
        match technique {
            Technique::Tap => {
                let target = self.legato(note, true).map(|(s, f, _)| (s, f)).or_else(|| self.place(note));
                if let Some((s, f)) = target
                    && self.tap(s, f, note, velocity)
                {
                    return;
                }
                self.play(note, velocity, Technique::Pluck);
            }
            Technique::PullOff => match self.legato(note, false) {
                Some((s, g, cur)) => self.pull_off(s, g, cur, note, velocity),
                None => self.play(note, velocity, Technique::Pluck),
            },
            _ => {
                // A repeated key re-plucks its own string at its fret, even
                // when a lower fret has come free elsewhere: moving would
                // leave this string ringing with no key to mute it.
                let Some((s, fret)) = self.position_of(note).or_else(|| self.place(note)) else { return };
                self.right_hand(s, fret, note, velocity, technique);
            }
        }
    }

    fn right_hand(&mut self, s: usize, fret: u8, note: u8, velocity: u8, technique: Technique) {
        let inst = &*self.inst;
        let (pl, tq) = (inst.model.pluck, inst.model.techniques);
        let (vel, sr) = (velocity as f64 / 127.0, inst.sample_rate);
        let v = &mut self.strings[s];
        v.muted = false;
        v.release_pending = false;
        v.fret_force = 0.0;
        v.restop(inst, s, Stop::Fret(fret), false);
        v.held = Some(note);
        v.sounding = true;
        let t = inst.table(s, fret);
        let x = pl.position.min(0.9 * t.length);
        let finger = (pl.finger_stiffness, pl.finger_damping, pl.finger_release);
        match (technique, pl.model) {
            (Technique::Pluck, PluckModel::Ideal) => {
                let h = pl.max_force * vel / (t.tension * t.length / (x * (t.length - x)));
                for (z, a) in v.zeta.iter_mut().zip(&t.shape) {
                    *z = [C64::new(a * h * pl.angle.cos(), 0.0), C64::new(a * h * pl.angle.sin(), 0.0)];
                }
                v.write(t);
            }
            (Technique::Pluck, model) => {
                let law = if model == PluckModel::Plectrum { (pl.plectrum_stiffness, pl.plectrum_damping, pl.plectrum_release) } else { finger };
                v.hand_pluck(t, x, pl.max_force * vel, (pl.angle.cos(), pl.angle.sin()), law, pl.drag_time, sr, RIGHT_HAND);
            }
            (Technique::Pop, _) => {
                // Hooked under the string and pulled away from the board.
                v.hand_pluck(t, x, tq.pop_force * vel, (-1.0, 0.0), finger, pl.drag_time, sr, RIGHT_HAND);
            }
            _ => {
                // Slap: the thumb comes down on the string from above and
                // goes on past its rest, then back.
                v.begin(t);
                let x = tq.slap_position.min(0.9 * t.length);
                let y0 = v.solver.displacement(x, PERP);
                let (start, to) = (y0 - SLAP_CLEARANCE, y0 + tq.slap_depth);
                let samples = speed_samples(to - start, tq.slap_speed * vel, sr);
                let back = ((tq.slap_return * sr) as usize).max(1);
                let law = Law::Unilateral { stiffness: tq.thumb_stiffness, exponent: 1.0, damping: tq.thumb_damping };
                v.solver.add(Contact { x, pol: PERP, sign: -1.0, law, motion: Motion::Stroke { start, to, samples, back }, release: Release::After(samples + back), group: RIGHT_HAND });
            }
        }
        v.engage(inst, s);
    }

    /// A finger strikes the string just behind `fret` and presses it on;
    /// false when that fret is not under the vibrating part.
    fn tap(&mut self, s: usize, fret: u8, note: u8, velocity: u8) -> bool {
        let inst = &*self.inst;
        let tq = inst.model.techniques;
        let (vel, sr) = (velocity as f64 / 127.0, inst.sample_rate);
        let v = &mut self.strings[s];
        v.end_contact(inst, s);
        if let Stop::Released(_) = v.stop {
            // The lifted finger leaves: the open string, unmuted.
            v.muted = false;
            v.restop(inst, s, Stop::Fret(0), false);
        }
        let t = table(inst, s, v.stop);
        let Some(g) = t.frets.iter().find(|g| g.fret == fret).copied() else { return false };
        v.muted = false;
        v.release_pending = false;
        v.fret_force = 0.0;
        v.write(t);
        v.begin(t);
        let fb = &inst.model.fretboard;
        let fret_law = Law::Unilateral { stiffness: fb.stiffness, exponent: fb.exponent, damping: fb.damping };
        let Some(fret_slot) = v.solver.add(Contact { x: g.x, pol: PERP, sign: 1.0, law: fret_law, motion: Motion::Fixed(g.gap), release: Release::Never, group: FRETS }) else {
            return false;
        };
        v.fret_slots[fret as usize] = Some(fret_slot);
        let x = (g.x + tq.tap_offset).min(t.length - 1e-3);
        let y0 = v.solver.displacement(x, PERP);
        let (start, to) = (y0 - TAP_CLEARANCE, g.gap + tq.tap_press);
        let samples = speed_samples(to - start, tq.tap_speed * vel, sr);
        let law = Law::Unilateral { stiffness: tq.tap_stiffness, exponent: 1.0, damping: tq.tap_damping };
        let Some(finger_slot) = v.solver.add(Contact { x, pol: PERP, sign: -1.0, law, motion: Motion::Stroke { start, to, samples, back: 0 }, release: Release::Never, group: LEFT_HAND }) else {
            return false;
        };
        let timeout = samples + (TAP_TIMEOUT * sr) as usize;
        v.hammer = Some(Hammer { fret, fret_slot, finger_slot, pressed: 0, elapsed: 0, timeout });
        v.held = Some(note);
        v.sounding = true;
        v.engage(inst, s);
        true
    }

    /// The finger on `from` leaves for `to` (lower) on string `s`, plucking
    /// the string sideways as it goes.
    fn pull_off(&mut self, s: usize, to: u8, from: u8, note: u8, velocity: u8) {
        let inst = &*self.inst;
        let (pl, tq) = (inst.model.pluck, inst.model.techniques);
        let (vel, sr) = (velocity as f64 / 127.0, inst.sample_rate);
        let v = &mut self.strings[s];
        v.muted = false;
        v.release_pending = false;
        v.fret_force = 0.0;
        v.restop(inst, s, Stop::Fret(to), true);
        let t = inst.table(s, to);
        let x = (fret_length(inst.model.scale_length, from) + tq.tap_offset).min(0.95 * t.length);
        let finger = (pl.finger_stiffness, pl.finger_damping, pl.finger_release);
        v.hand_pluck(t, x, tq.pulloff_force * vel, (0.0, 1.0), finger, pl.drag_time / 2.0, sr, LEFT_HAND);
        v.held = Some(note);
        v.sounding = true;
        v.engage(inst, s);
    }

    fn release(&mut self, note: u8) {
        for s in 0..self.strings.len() {
            if self.strings[s].held == Some(note) {
                self.strings[s].held = None;
                self.strings[s].lift(&self.inst, s);
            }
        }
    }

    /// Writes `out.len()` samples (mono, the instrument's output) into
    /// `out`. Real-time safe.
    pub fn process(&mut self, out: &mut [f32]) {
        let _ftz = DenormalGuard::new();
        out.fill(0.0);
        let inst = &*self.inst;
        for block in out.chunks_mut(self.max_block) {
            let len = block.len();
            let force = &mut self.force[..len];
            force.fill(0.0);
            for (s, v) in self.strings.iter_mut().enumerate() {
                if !v.sounding {
                    continue;
                }
                if !v.in_contact {
                    v.engage(inst, s);
                }
                let mut pos = 0;
                while pos < len {
                    if v.in_contact {
                        let n = (len - pos).min(CHECK);
                        let done = v.solver.run(&mut block[pos..pos + n], &mut force[pos..pos + n]);
                        pos += done;
                        v.after_chunk(inst, s, done);
                        if done == 0 && v.in_contact {
                            v.end_contact(inst, s);
                            v.engage(inst, s);
                        }
                    } else if !v.watch.is_empty() {
                        let n = (len - pos).min(WATCH);
                        v.bank.process2(None, &mut block[pos..pos + n], &mut force[pos..pos + n]);
                        pos += n;
                        if v.threatened(inst, s, WATCH) {
                            v.enter(inst, s);
                        }
                    } else {
                        v.bank.process2(None, &mut block[pos..], &mut force[pos..]);
                        pos = len;
                    }
                }
                if !v.in_contact {
                    v.bank.drop_tail(SILENCE);
                    if v.bank.energy() < SILENCE {
                        v.bank.reset();
                        v.sounding = false;
                    }
                }
            }
            self.stage.process(block);
        }
    }

    /// Strings still computed.
    pub fn sounding(&self) -> usize {
        self.strings.iter().filter(|s| s.sounding).count()
    }

    /// Strings in the contact solver now.
    pub fn in_contact(&self) -> usize {
        self.strings.iter().filter(|s| s.in_contact).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::GuitarModel;

    const SR: f64 = 48_000.0;

    fn engine_with(m: GuitarModel) -> Engine {
        Engine::new(Arc::new(Instrument::new(m, SR)), 256)
    }

    fn engine() -> Engine {
        engine_with(GuitarModel::default())
    }

    fn rms(x: &[f32]) -> f64 {
        (x.iter().map(|&v| (v as f64).powi(2)).sum::<f64>() / x.len() as f64).sqrt()
    }

    fn render(e: &mut Engine, seconds: f64) -> Vec<f32> {
        let mut buf = vec![0.0f32; (seconds * SR) as usize];
        e.process(&mut buf);
        buf
    }

    #[test]
    fn open_c_chord_takes_first_position() {
        let mut e = engine();
        for n in [48, 52, 55, 60, 64] {
            e.handle(Event::NoteOn { note: n, velocity: 90 });
        }
        // C on the A string 3rd fret, E on D 2nd, open G, C on B 1st, open E.
        let pos: Vec<_> = [48, 52, 55, 60, 64].iter().map(|&n| e.position_of(n).unwrap()).collect();
        assert_eq!(pos, [(1, 3), (2, 2), (3, 0), (4, 1), (5, 0)]);
    }

    #[test]
    fn same_string_note_moves_to_the_next_string() {
        let mut e = engine();
        e.handle(Event::NoteOn { note: 64, velocity: 90 });
        e.handle(Event::NoteOn { note: 65, velocity: 90 });
        assert_eq!(e.position_of(64), Some((5, 0)));
        assert_eq!(e.position_of(65), Some((4, 6)));
    }

    #[test]
    fn repeated_key_stays_on_its_string_and_mutes() {
        // F4 goes to the B string (6th fret) while the E string is held;
        // once E4 is released, repeating F4 must not move it to the free
        // E string's 1st fret and leave the B string ringing unmuted.
        let mut e = engine();
        e.handle(Event::NoteOn { note: 64, velocity: 90 });
        e.handle(Event::NoteOn { note: 65, velocity: 90 });
        assert_eq!(e.position_of(65), Some((4, 6)));
        e.handle(Event::NoteOff { note: 64 });
        render(&mut e, 0.5);
        e.handle(Event::NoteOn { note: 65, velocity: 90 });
        assert_eq!(e.position_of(65), Some((4, 6)));
        render(&mut e, 0.3);
        e.handle(Event::NoteOff { note: 65 });
        render(&mut e, 1.0);
        assert_eq!(e.sounding(), 0, "a string still rings with no key holding it");
    }

    #[test]
    fn pluck_rings_then_mute_stops_it() {
        for model in [PluckModel::Ideal, PluckModel::Finger] {
            let mut m = GuitarModel::default();
            m.pluck.model = model;
            let mut e = engine_with(m);
            e.handle(Event::NoteOn { note: 45, velocity: 100 });
            let buf = render(&mut e, 0.5);
            assert!(buf.iter().all(|v| v.is_finite()));
            let ringing = rms(&buf[buf.len() - 4800..]);
            assert!(ringing > 1e-3, "{model:?}: open A after 0.5 s: rms {ringing}");
            e.handle(Event::NoteOff { note: 45 });
            let buf = render(&mut e, 0.5);
            let muted = rms(&buf[buf.len() - 4800..]);
            assert!(muted < ringing * 1e-3, "{model:?}: muted {muted} vs ringing {ringing}");
        }
    }

    #[test]
    fn finger_and_ideal_plucks_are_alike() {
        // The fingertip lets go of nearly the same triangle: same level
        // within a few dB once the drag is over.
        let level = |model| {
            let mut m = GuitarModel::default();
            m.pluck.model = model;
            let mut e = engine_with(m);
            e.handle(Event::NoteOn { note: 52, velocity: 100 });
            let buf = render(&mut e, 0.5);
            rms(&buf[(0.1 * SR) as usize..])
        };
        let (ideal, finger) = (level(PluckModel::Ideal), level(PluckModel::Finger));
        let db = 20.0 * (finger / ideal).log10();
        assert!(db.abs() < 3.0, "finger vs ideal: {db} dB");
    }

    #[test]
    fn free_decay_is_plausible_and_silent_strings_stop() {
        let mut e = engine();
        e.handle(Event::NoteOn { note: 64, velocity: 100 });
        let first = render(&mut e, 0.1);
        render(&mut e, 2.0);
        let at2s = render(&mut e, 0.1);
        // A rigid-ish bridge decays slower than a real guitar, but the high
        // partials go first: lose 10–40 dB in 2 s.
        let drop = 20.0 * (rms(&first[2400..]) / rms(&at2s)).log10();
        assert!((10.0..40.0).contains(&drop), "drop over 2 s: {drop} dB");
        e.handle(Event::AllNotesOff);
        render(&mut e, 1.0);
        assert_eq!(e.sounding(), 0);
    }

    #[test]
    fn lifting_a_fretted_finger_mutes_it() {
        let mut e = engine();
        e.handle(Event::NoteOn { note: 45, velocity: 100 });
        e.handle(Event::NoteOn { note: 50, velocity: 100 });
        e.handle(Event::NoteOn { note: 55, velocity: 100 });
        e.handle(Event::NoteOn { note: 47, velocity: 100 });
        // B2 on the low E, 7th fret.
        assert_eq!(e.position_of(47), Some((0, 7)));
        let ringing = rms(&render(&mut e, 0.5)[19200..]);
        e.handle(Event::AllNotesOff);
        let buf = render(&mut e, 0.6);
        assert!(buf.iter().all(|v| v.is_finite()));
        let muted = rms(&buf[buf.len() - 4800..]);
        assert!(muted < ringing * 1e-2, "muted {muted} vs ringing {ringing}");
    }

    #[test]
    fn legato_on_one_string_projects_its_motion() {
        // Same string, new fret: the string keeps ringing at the new pitch
        // until plucked; here the finger re-plucks, so the level stays.
        let mut e = engine();
        e.handle(Event::NoteOn { note: 40, velocity: 100 });
        render(&mut e, 0.3);
        // Every string held except the low E: the next note must go there.
        for n in [45, 50, 55, 59, 64] {
            e.handle(Event::NoteOn { note: n, velocity: 1 });
        }
        e.handle(Event::NoteOff { note: 40 });
        e.handle(Event::NoteOn { note: 43, velocity: 100 });
        assert_eq!(e.position_of(43), Some((0, 3)));
        let buf = render(&mut e, 0.3);
        assert!(buf.iter().all(|v| v.is_finite()));
        assert!(rms(&buf[4800..]) > 1e-3);
    }

    /// Share of the signal's power in its first difference, a brightness
    /// proxy (≈ (2πf/fs)² for a sine).
    fn brightness(x: &[f32]) -> f64 {
        let d: f64 = x.windows(2).map(|w| ((w[1] - w[0]) as f64).powi(2)).sum();
        d / x.iter().map(|&v| (v as f64).powi(2)).sum::<f64>()
    }

    fn presets() -> [(&'static str, GuitarModel); 3] {
        [("acoustic", GuitarModel::steel_acoustic()), ("electric", GuitarModel::electric_solid()), ("bass", GuitarModel::electric_bass())]
    }

    #[test]
    fn presets_are_in_tune_and_stiff_as_measured() {
        use crate::string::midi_hz;
        for (name, m) in presets() {
            let inst = Instrument::new(m, SR);
            for (i, s) in inst.model.strings.iter().enumerate() {
                let t = inst.table(i, 0);
                assert!((t.f0 / midi_hz(s.open_note as f64) - 1.0).abs() < 1e-9, "{name} string {i}");
                // Guitars 1e-5 … 2e-4, basses up to ≈ 1e-3.
                assert!((5e-6..1.5e-3).contains(&t.b), "{name} string {i}: B = {:e}", t.b);
            }
        }
    }

    #[test]
    fn electric_outputs_sound_at_instrument_level_and_sustain() {
        for (name, m) in presets() {
            let low = m.strings[0].open_note;
            let mut e = engine_with(m);
            e.handle(Event::NoteOn { note: low, velocity: 100 });
            let first = render(&mut e, 0.2);
            render(&mut e, 1.8);
            let later = render(&mut e, 0.2);
            assert!(first.iter().chain(&later).all(|v| v.is_finite()), "{name}");
            let peak = first.iter().fold(0.0f32, |a, v| a.max(v.abs()));
            assert!((0.01..2.0).contains(&peak), "{name}: peak {peak}");
            let drop = 20.0 * (rms(&first[4800..]) / rms(&later)).log10();
            // A solid body sustains: well under the acoustic's loss.
            if name != "acoustic" {
                assert!(drop < 20.0, "{name}: {drop} dB lost in 2 s");
            }
        }
    }

    #[test]
    fn plectrum_is_brighter_than_finger() {
        let b = |model| {
            let mut m = GuitarModel::default();
            m.pluck.model = model;
            let mut e = engine_with(m);
            e.handle(Event::NoteOn { note: 52, velocity: 100 });
            brightness(&render(&mut e, 0.15)[2400..])
        };
        let (finger, plectrum) = (b(PluckModel::Finger), b(PluckModel::Plectrum));
        assert!(plectrum > 1.3 * finger, "plectrum {plectrum} vs finger {finger}");
    }

    #[test]
    fn bridge_pickup_is_brighter_than_neck_pickup() {
        use crate::params::Output;
        let b = |pickup| {
            let mut m = GuitarModel::electric_solid();
            if let Output::Magnetic(o) = &mut m.output {
                o.selected = vec![pickup];
            }
            let mut e = engine_with(m);
            e.handle(Event::NoteOn { note: 45, velocity: 100 });
            brightness(&render(&mut e, 0.3)[2400..])
        };
        let (bridge, neck) = (b(0), b(2));
        assert!(bridge > 1.5 * neck, "bridge {bridge} vs neck {neck}");
    }

    #[test]
    fn muted_pickup_output_is_the_velocity_of_the_muted_string() {
        // Low E released from the 12th fret, only its fundamental moving
        // (0.1 mm), no bridge coupling: the muted bank's EMF must match the
        // contact solver's, which computes the velocity from the state.
        use crate::contact::{Contact, Law, Motion, Release};
        use crate::params::{Admittance, SupportParams};
        let mut m = GuitarModel::electric_solid();
        m.support = SupportParams { perpendicular: Admittance::ZERO, parallel: Admittance::ZERO, cross: Admittance::ZERO };
        let inst = Instrument::new(m, SR);
        let t = inst.released(0, 12);
        assert!(t.muted && t.velocity_out);
        let mut zeta = vec![[C64::ZERO; 2]; t.len()];
        zeta[0][PERP] = C64::new(1e-4, 0.0);
        let mut bank = ModalBank::new(SR, 2 * t.len(), 64);
        bank.load(&t.modes);
        for (n, p) in t.partials.iter().enumerate() {
            let z = p.coupled_of(zeta[n]);
            bank.set_state(2 * n, z[0].re, z[0].im);
            bank.set_state(2 * n + 1, z[1].re, z[1].im);
        }
        bank.set_damping(1.0);
        let (mut a, mut a2) = (vec![0.0f32; 8], vec![0.0f32; 8]);
        bank.process2(None, &mut a, &mut a2);
        let mut sol = ContactSolver::new(t.len(), SR);
        sol.begin(t, &zeta, 1.0);
        sol.add(Contact { x: 0.1, pol: PERP, sign: 1.0, law: Law::Spring { stiffness: 0.0, damping: 0.0 }, motion: Motion::Fixed(0.0), release: Release::Never, group: 0 });
        let (mut b, mut b2) = (vec![0.0f32; 8], vec![0.0f32; 8]);
        sol.run(&mut b, &mut b2);
        for (x, y) in a.iter().zip(&b) {
            assert!((x - y).abs() < 1e-4 * y.abs(), "bank {x} vs velocity {y}");
        }
        // And ringing (not muted), the plain table agrees the same way.
        let t = inst.table(0, 12);
        bank.load(&t.modes);
        for (n, p) in t.partials.iter().enumerate() {
            let z = p.coupled_of(zeta[n]);
            bank.set_state(2 * n, z[0].re, z[0].im);
            bank.set_state(2 * n + 1, z[1].re, z[1].im);
        }
        bank.process2(None, &mut a, &mut a2);
        sol.begin(t, &zeta, 0.0);
        sol.add(Contact { x: 0.1, pol: PERP, sign: 1.0, law: Law::Spring { stiffness: 0.0, damping: 0.0 }, motion: Motion::Fixed(0.0), release: Release::Never, group: 0 });
        sol.run(&mut b, &mut b2);
        for (x, y) in a.iter().zip(&b) {
            assert!((x - y).abs() < 1e-4 * y.abs(), "ringing: bank {x} vs velocity {y}");
        }
    }

    fn play(e: &mut Engine, note: u8, velocity: u8, technique: Technique) {
        e.handle(Event::Note { note, velocity, technique });
    }

    #[test]
    fn soft_plucks_never_touch_a_fret() {
        for (name, m) in presets() {
            let low = m.strings[0].open_note;
            let mut e = engine_with(m);
            e.handle(Event::NoteOn { note: low, velocity: 40 });
            render(&mut e, 1.0);
            assert_eq!(e.fret_force(), 0.0, "{name}");
            // And the string is back in its bank, not left in the solver.
            assert_eq!(e.in_contact(), 0, "{name}");
        }
    }

    #[test]
    fn a_hard_pluck_toward_the_board_buzzes_on_low_action() {
        let mut m = GuitarModel::electric_solid();
        m.pluck.angle = 0.0;
        m.pluck.max_force = 6.0;
        m.fretboard.action_bass = 0.8e-3;
        let mut e = engine_with(m);
        e.handle(Event::NoteOn { note: 40, velocity: 127 });
        let buf = render(&mut e, 0.5);
        assert!(buf.iter().all(|v| v.is_finite()));
        assert!(e.fret_force() > 0.0, "no buzz");
    }

    #[test]
    fn slap_and_pop_hit_the_frets() {
        for technique in [Technique::Slap, Technique::Pop] {
            let mut e = engine_with(GuitarModel::electric_bass());
            play(&mut e, 28, 120, technique);
            let buf = render(&mut e, 0.5);
            assert!(buf.iter().all(|v| v.is_finite()), "{technique:?}");
            assert!(e.fret_force() > 0.0, "{technique:?}: never hit a fret");
            assert!(rms(&buf[2400..]) > 1e-3, "{technique:?}: silent");
        }
        // A slap is brighter than a finger pluck of the same note.
        let b = |technique| {
            let mut e = engine_with(GuitarModel::electric_bass());
            play(&mut e, 28, 110, technique);
            brightness(&render(&mut e, 0.3)[1200..])
        };
        let (slap, pluck) = (b(Technique::Slap), b(Technique::Pluck));
        assert!(slap > 1.5 * pluck, "slap {slap} vs pluck {pluck}");
    }

    #[test]
    fn tapping_frets_the_string_and_sounds_its_pitch() {
        // Tap A3 (57) from rest, nothing held: where a pluck would go, the
        // G string's 2nd fret.
        let mut e = engine_with(GuitarModel::electric_solid());
        play(&mut e, 57, 110, Technique::Tap);
        assert_eq!(e.position_of(57), Some((3, 2)));
        let buf = render(&mut e, 0.4);
        assert!(buf.iter().all(|v| v.is_finite()));
        assert!(e.fret_force() > 0.0, "the string never met its fret");
        // Pressed: the string now runs on the 2nd fret.
        assert_eq!(e.strings[3].stop, Stop::Fret(2));
        assert!(e.strings[3].hammer.is_none());
        let tail = &buf[9600..];
        assert!(rms(tail) > 1e-4, "tap too quiet: {}", rms(tail));
        // Pitch: the tail repeats at the note's period (autocorrelation;
        // inharmonicity and the whole-sample lag keep it below 1), and not
        // at half or two thirds of it.
        let period = SR / crate::string::midi_hz(57.0);
        let r = |lag: f64| {
            let l = lag.round() as usize;
            let (a, b) = (&tail[..tail.len() - l], &tail[l..]);
            let num: f64 = a.iter().zip(b).map(|(x, y)| (x * y) as f64).sum();
            num / (a.iter().map(|&x| (x as f64).powi(2)).sum::<f64>() * b.iter().map(|&x| (x as f64).powi(2)).sum::<f64>()).sqrt()
        };
        let (at, half, two_thirds) = (r(period), r(period / 2.0), r(period * 2.0 / 3.0));
        assert!(at > 0.5 && at > half && at > two_thirds, "autocorrelation {at} at the period, {half} at half, {two_thirds} at 2/3");
    }

    #[test]
    fn hammer_on_and_pull_off_stay_on_the_string() {
        // C3 on the A string's 3rd fret, held.
        let mut e = engine_with(GuitarModel::electric_solid());
        e.handle(Event::NoteOn { note: 48, velocity: 100 });
        assert_eq!(e.position_of(48), Some((1, 3)));
        render(&mut e, 0.2);
        // Hammer-on to the 5th fret of the same string.
        play(&mut e, 50, 100, Technique::Tap);
        render(&mut e, 0.2);
        assert_eq!(e.position_of(50), Some((1, 5)));
        assert_eq!(e.strings[1].stop, Stop::Fret(5));
        // Pull-off back to the 3rd: same string, still sounding.
        play(&mut e, 48, 100, Technique::PullOff);
        assert_eq!(e.position_of(48), Some((1, 3)));
        let buf = render(&mut e, 0.3);
        assert!(buf.iter().all(|v| v.is_finite()));
        assert!(rms(&buf[4800..]) > 1e-3);
        // The old keys no longer hold anything: releasing them mutes nothing.
        e.handle(Event::NoteOff { note: 50 });
        assert_eq!(e.position_of(48), Some((1, 3)));
        e.handle(Event::NoteOff { note: 48 });
        render(&mut e, 1.0);
        assert_eq!(e.sounding(), 0);
    }

    #[test]
    fn a_hammer_on_lands_without_a_spike() {
        // The string pressed onto the fret is at rest on its new line: no
        // fret beyond it is hit, the output stays at the pluck's level.
        let mut e = engine_with(GuitarModel::electric_solid());
        e.handle(Event::NoteOn { note: 48, velocity: 100 });
        let before = render(&mut e, 0.3);
        play(&mut e, 50, 100, Technique::Tap);
        let after = render(&mut e, 0.3);
        assert_eq!(e.position_of(50), Some((1, 5)));
        let peak = |x: &[f32]| x.iter().fold(0.0f32, |m, v| m.max(v.abs()));
        assert!(peak(&after) < 2.0 * peak(&before), "hammer-on peak {} vs pluck {}", peak(&after), peak(&before));
        assert!(e.fret_force() < 20.0, "fret force {} N", e.fret_force());
    }
}
