//! Real-time engine: one modal bank and one contact solver per string, MIDI
//! notes assigned to strings and frets.
//!
//! A note goes to the lowest fret that reaches it on a string not already
//! held, else it takes over the lowest-fret string (a guitar string plays one
//! note at a time). Pressing a new fret on a string that rings carries its
//! motion over to the new length ([`crate::project`]); then the right hand
//! plucks: ideally (the motion replaced by a triangle), or with a fingertip
//! or a plectrum in the contact solver (it lands on the string, drags it,
//! slips off at the pluck's force). At note-off the fretting finger lifts:
//! the string goes back to its open length, damped by the fingertip still
//! touching it behind the fret. The strings' summed output goes through the
//! output stage ([`crate::output`]): pickups' circuit, piezo, or as it is.

use std::sync::Arc;

use piano_core::complex::C64;
use piano_core::denormal::DenormalGuard;
use piano_core::modal::ModalBank;

use crate::contact::{Contact, ContactSolver, Law, Motion, Release};
use crate::instrument::Instrument;
use crate::output::OutputStage;
use crate::params::PluckModel;
use crate::project::project;
use crate::string::{StringTable, PAR, PERP};

/// Output-weighted energy below which a string stops being computed
/// (amplitude ≈ −120 dB re full scale).
const SILENCE: f32 = 1e-12;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    NoteOn { note: u8, velocity: u8 },
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

struct StringVoice {
    bank: ModalBank,
    solver: ContactSolver,
    /// Key holding the string down, if any.
    held: Option<u8>,
    stop: Stop,
    muted: bool,
    sounding: bool,
    in_contact: bool,
    /// The key went up during a contact: lift the finger once it ends.
    release_pending: bool,
    /// Per-polarisation amplitudes, and scratch for projections.
    zeta: Vec<[C64; 2]>,
    scratch: Vec<[C64; 2]>,
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

    /// Ends a contact episode: the solver's state back to the bank.
    fn end_contact(&mut self, inst: &Instrument, s: usize) {
        let t = table(inst, s, self.stop);
        for n in 0..t.len() {
            self.zeta[n] = [self.solver.complex_state(PERP, n), self.solver.complex_state(PAR, n)];
        }
        self.in_contact = false;
        self.write(t);
        if self.release_pending {
            self.release_pending = false;
            self.lift(inst, s);
        }
    }

    /// Moves the string to `stop`, its motion projected (a new fret, or the
    /// finger lifting).
    fn restop(&mut self, inst: &Instrument, s: usize, stop: Stop) {
        if self.in_contact {
            self.end_contact(inst, s);
        }
        let (from, to) = (table(inst, s, self.stop), table(inst, s, stop));
        self.read(from);
        if self.sounding && !std::ptr::eq(from, to) {
            project(from, to, &self.zeta, &mut self.scratch);
            std::mem::swap(&mut self.zeta, &mut self.scratch);
        }
        self.stop = stop;
        self.write(to);
    }

    /// The fretting finger lifts off (or the hand mutes an open string).
    fn lift(&mut self, inst: &Instrument, s: usize) {
        if self.in_contact {
            self.release_pending = true;
            return;
        }
        self.muted = true;
        match self.stop {
            Stop::Fret(f) => self.restop(inst, s, Stop::Released(f)),
            Stop::Released(_) => self.bank.set_damping(1.0),
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
                zeta: vec![[C64::ZERO; 2]; np],
                scratch: vec![[C64::ZERO; 2]; np],
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
        match self.strings[i].stop {
            Stop::Fret(f) => Some((i, f)),
            Stop::Released(_) => None,
        }
    }

    pub fn handle(&mut self, ev: Event) {
        match ev {
            Event::NoteOn { note, velocity: 0 } => self.release(note),
            Event::NoteOn { note, velocity } => self.pluck(note, velocity),
            Event::NoteOff { note } => self.release(note),
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

    fn pluck(&mut self, note: u8, velocity: u8) {
        // A repeated key re-plucks its own string at its fret, even when a
        // lower fret has come free elsewhere: moving would leave this string
        // ringing with no key to mute it.
        let Some((s, fret)) = self.position_of(note).or_else(|| self.place(note)) else { return };
        let inst = &*self.inst;
        let pl = inst.model.pluck;
        let force = pl.max_force * velocity as f64 / 127.0;
        let (cp, cq) = (pl.angle.cos(), pl.angle.sin());
        let v = &mut self.strings[s];
        v.muted = false;
        v.release_pending = false;
        v.restop(inst, s, Stop::Fret(fret));
        let t = inst.table(s, fret);
        let x = pl.position.min(0.9 * t.length);
        // The string's stiffness at the pluck point, T·L/(x(L − x)).
        let k_string = t.tension * t.length / (x * (t.length - x));
        let (stiffness, damping, slip_time) = match pl.model {
            PluckModel::Ideal => {
                let h = force / k_string;
                for (z, a) in v.zeta.iter_mut().zip(&t.shape) {
                    *z = [C64::new(a * h * cp, 0.0), C64::new(a * h * cq, 0.0)];
                }
                v.write(t);
                v.held = Some(note);
                v.sounding = true;
                return;
            }
            PluckModel::Finger => (pl.finger_stiffness, pl.finger_damping, pl.finger_release),
            PluckModel::Plectrum => (pl.plectrum_stiffness, pl.plectrum_damping, pl.plectrum_release),
        };
        v.read(t);
        v.solver.begin(t, &v.zeta, 0.0);
        let sr = inst.sample_rate;
        let drag = ((pl.drag_time * sr) as usize).max(1);
        let slip = ((slip_time * sr).round() as usize).max(1);
        // Aim twice as far as the slip needs (string and contact in
        // series): the force limit is reached about halfway through.
        let travel = 2.0 * force * (1.0 / k_string + 1.0 / stiffness);
        let law = Law::Spring { stiffness, damping };
        let release = Release::Force { limit: force, slip, timeout: drag };
        for (pol, dir) in [(PERP, cp), (PAR, cq)] {
            v.solver.add(Contact { x, pol, sign: 1.0, law, motion: Motion::Ramp { to: travel * dir, samples: drag }, release, group: 0 });
        }
        v.in_contact = true;
        v.held = Some(note);
        v.sounding = true;
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
        for block in out.chunks_mut(self.max_block) {
            let force = &mut self.force[..block.len()];
            force.fill(0.0);
            for s in 0..self.strings.len() {
                let v = &mut self.strings[s];
                if !v.sounding {
                    continue;
                }
                let mut done = 0;
                if v.in_contact {
                    done = v.solver.run(block, force);
                    if !v.solver.is_active() {
                        v.end_contact(&self.inst, s);
                    }
                }
                if done < block.len() {
                    v.bank.process2(None, &mut block[done..], &mut force[done..]);
                    v.bank.drop_tail(SILENCE);
                    if !v.in_contact && v.bank.energy() < SILENCE {
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
}
