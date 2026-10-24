//! Real-time engine: one modal bank per string, MIDI notes assigned to
//! strings and frets.
//!
//! A note goes to the lowest fret that reaches it on a string not already
//! held, else it takes over the lowest-fret string (a guitar string plays one
//! note at a time). A pluck replaces the string's motion: the finger or
//! plectrum stops it before releasing it. Note-off lays the mute on it.

use std::sync::Arc;

use piano_core::denormal::DenormalGuard;
use piano_core::modal::ModalBank;

use crate::instrument::Instrument;

/// Output-weighted energy below which a string stops being computed
/// (amplitude ≈ −120 dB re full scale).
const SILENCE: f32 = 1e-12;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    NoteOn { note: u8, velocity: u8 },
    NoteOff { note: u8 },
    AllNotesOff,
}

struct StringVoice {
    bank: ModalBank,
    /// Key holding the string down, if any.
    held: Option<u8>,
    fret: u8,
    sounding: bool,
}

pub struct Engine {
    inst: Arc<Instrument>,
    strings: Vec<StringVoice>,
    max_block: usize,
}

impl Engine {
    /// Allocates everything up front; nothing allocates afterwards.
    pub fn new(inst: Arc<Instrument>, max_block: usize) -> Self {
        let strings = (0..inst.model.strings.len())
            .map(|_| StringVoice {
                bank: ModalBank::new(inst.sample_rate, inst.max_modes, max_block),
                held: None,
                fret: 0,
                sounding: false,
            })
            .collect();
        Self { inst, strings, max_block }
    }

    pub fn instrument(&self) -> &Arc<Instrument> {
        &self.inst
    }

    /// String and fret playing `note` now, if it sounds.
    pub fn position_of(&self, note: u8) -> Option<(usize, u8)> {
        self.strings.iter().position(|s| s.held == Some(note)).map(|i| (i, self.strings[i].fret))
    }

    pub fn handle(&mut self, ev: Event) {
        match ev {
            Event::NoteOn { note, velocity: 0 } => self.release(note),
            Event::NoteOn { note, velocity } => self.pluck(note, velocity),
            Event::NoteOff { note } => self.release(note),
            Event::AllNotesOff => {
                for s in &mut self.strings {
                    s.held = None;
                    s.bank.set_damping(1.0);
                }
            }
        }
    }

    fn pluck(&mut self, note: u8, velocity: u8) {
        // A repeated key re-plucks its own string rather than another one.
        self.release(note);
        let mut best: Option<(usize, u8)> = None;
        for (s, fret) in self.inst.positions(note) {
            best.get_or_insert((s, fret));
            if self.strings[s].held.is_none() {
                best = Some((s, fret));
                break;
            }
        }
        let Some((s, fret)) = best else { return };
        let table = &self.inst.tables[s][fret as usize];
        let h = self.inst.model.pluck.max_displacement * velocity as f64 / 127.0;
        let v = &mut self.strings[s];
        v.bank.load(&table.modes);
        for (i, a) in table.shape.iter().enumerate() {
            v.bank.set_state(i, a * h, 0.0);
        }
        v.held = Some(note);
        v.fret = fret;
        v.sounding = true;
    }

    fn release(&mut self, note: u8) {
        for s in self.strings.iter_mut().filter(|s| s.held == Some(note)) {
            s.held = None;
            s.bank.set_damping(1.0);
        }
    }

    /// Writes `out.len()` samples (mono) into `out`. Real-time safe.
    pub fn process(&mut self, out: &mut [f32]) {
        let _ftz = DenormalGuard::new();
        out.fill(0.0);
        for block in out.chunks_mut(self.max_block) {
            for s in self.strings.iter_mut().filter(|s| s.sounding) {
                s.bank.process(None, block);
                s.bank.drop_tail(SILENCE);
                if s.bank.energy() < SILENCE {
                    s.bank.reset();
                    s.sounding = false;
                }
            }
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

    fn engine() -> Engine {
        Engine::new(Arc::new(Instrument::new(GuitarModel::default(), SR)), 256)
    }

    fn rms(x: &[f32]) -> f64 {
        (x.iter().map(|&v| (v as f64).powi(2)).sum::<f64>() / x.len() as f64).sqrt()
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
    fn pluck_rings_then_mute_stops_it() {
        let mut e = engine();
        e.handle(Event::NoteOn { note: 45, velocity: 100 });
        let mut buf = vec![0.0f32; SR as usize / 2];
        e.process(&mut buf);
        assert!(buf.iter().all(|v| v.is_finite()));
        let ringing = rms(&buf[buf.len() - 4800..]);
        assert!(ringing > 1e-3, "open A after 0.5 s: rms {ringing}");
        e.handle(Event::NoteOff { note: 45 });
        e.process(&mut buf);
        let muted = rms(&buf[buf.len() - 4800..]);
        assert!(muted < ringing * 1e-3, "muted {muted} vs ringing {ringing}");
    }

    #[test]
    fn free_decay_is_plausible_and_silent_strings_stop() {
        let mut e = engine();
        e.handle(Event::NoteOn { note: 64, velocity: 100 });
        let mut first = vec![0.0f32; 4800];
        e.process(&mut first);
        let mut later = vec![0.0f32; SR as usize];
        e.process(&mut later);
        e.process(&mut later);
        let mut at2s = vec![0.0f32; 4800];
        e.process(&mut at2s);
        // A rigid-bridge string decays slower than on a guitar, but its
        // high partials go first: lose 10–40 dB in 2 s.
        let drop = 20.0 * (rms(&first) / rms(&at2s)).log10();
        assert!((10.0..40.0).contains(&drop), "drop over 2 s: {drop} dB");
        e.handle(Event::AllNotesOff);
        e.process(&mut later);
        assert_eq!(e.sounding(), 0);
    }
}
