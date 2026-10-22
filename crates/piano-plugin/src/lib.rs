//! The physical piano as a CLAP / VST3 instrument (nice-plug).
//!
//! The host plays the engine slot of piano-control: notes and pedals from
//! its MIDI input, the live parameters from its automatable parameters
//! (sent as `Event::Param` when they change). Everything else in the
//! instrument is state saved with the project: editing it rebuilds the
//! instrument in the background while the notes keep playing. The editor
//! is piano-ui; a supervising thread polls the controller (MIDI learn,
//! restored states) when it is closed.

mod editor;
mod params;
mod supervisor;

use std::num::NonZeroU32;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use nice_plug::midi::Key;
use nice_plug::prelude::*;
use nice_plug_egui::{EguiEditor, EguiEditorState, RepaintNotifier};
use piano_control::{Controller, EngineSlot};
use piano_core::{EngineConfig, Event, TimedEvent};

pub use params::PianoParams;

/// Engine block; host buffers are rendered in pieces of this size.
const MAX_BLOCK: usize = 1024;
const MAX_EVENTS: usize = 1024;

/// What the plugin, its editor and the supervising thread share.
pub struct Shared {
    pub params: Arc<PianoParams>,
    /// The non-real-time side of the instrument, once the host activated it.
    pub control: Mutex<Option<Controller>>,
    pub repaint: RepaintNotifier,
    stop: AtomicBool,
}

pub struct Piano {
    shared: Arc<Shared>,
    slot: Option<EngineSlot>,
    sample_rate: f64,
    /// Live parameter values last sent to the engine.
    sent: Vec<f32>,
    events: Vec<TimedEvent>,
    piece: Vec<TimedEvent>,
    editor_state: Arc<EguiEditorState>,
    supervisor: Option<std::thread::JoinHandle<()>>,
}

impl Default for Piano {
    fn default() -> Self {
        let params = Arc::new(PianoParams::default());
        let shared = Arc::new(Shared {
            params,
            control: Mutex::new(None),
            repaint: RepaintNotifier::new(),
            stop: AtomicBool::new(false),
        });
        Self {
            shared,
            slot: None,
            sample_rate: 48_000.0,
            sent: Vec::new(),
            events: Vec::with_capacity(MAX_EVENTS),
            piece: Vec::with_capacity(MAX_EVENTS),
            editor_state: editor::state(),
            supervisor: None,
        }
    }
}

impl Drop for Piano {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.supervisor.take() {
            let _ = t.join();
        }
    }
}

impl Plugin for Piano {
    const NAME: &'static str = "Physical Piano";
    const VENDOR: &'static str = "mgth";
    const URL: &'static str = "https://github.com/mgth/piano";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: None,
        main_output_channels: NonZeroU32::new(2),
        ..AudioIOLayout::const_default()
    }];
    // Notes, and the pedals as controllers.
    const MIDI_INPUT: MidiConfig = MidiConfig::MidiCCs;
    const INPUT_EVENT_CAPACITY: usize = MAX_EVENTS;

    type Editor = EguiEditor<editor::PianoEditor>;
    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.shared.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Self::Editor> {
        editor::create(self.shared.clone(), self.editor_state.clone())
    }

    /// Builds the instrument for the host's sample rate (≈ 0.1 s), from the
    /// state the host restored if any.
    fn activate(
        &mut self,
        _layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl ActivateContext<Self>,
    ) -> bool {
        let params = &self.shared.params;
        params.restored.store(false, Ordering::Release);
        let model = params.model();
        self.sample_rate = buffer_config.sample_rate as f64;
        let cfg = EngineConfig { sample_rate: self.sample_rate, max_block: MAX_BLOCK, model, ..Default::default() };
        let (mut control, slot) = piano_control::new(model, cfg);
        control.set_midi_map(params.midi.read().unwrap().clone());
        // The engine starts with the model, live values included.
        self.sent = params.live.iter().map(|(_, p)| p.value()).collect();
        *self.shared.control.lock().unwrap() = Some(control);
        self.slot = Some(slot);
        if self.supervisor.is_none() {
            let shared = self.shared.clone();
            self.supervisor = Some(std::thread::spawn(move || supervisor::run(&shared)));
        }
        true
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let start = Instant::now();
        let frames = buffer.samples();
        let Some(slot) = self.slot.as_mut() else {
            for ch in buffer.as_slice() {
                ch.fill(0.0);
            }
            return ProcessStatus::Normal;
        };
        self.events.clear();
        // Automation and edits of the live parameters, at the block's start.
        for ((id, p), sent) in self.shared.params.live.iter().zip(self.sent.iter_mut()) {
            let v = p.value();
            if v != *sent {
                *sent = v;
                self.events.push(TimedEvent { frame: 0, event: Event::Param { id: *id, value: v } });
            }
        }
        while let Some(e) = context.next_event() {
            let (timing, event) = match e {
                NoteEvent::NoteOn { timing, key: Key::Number(note), velocity, .. } => {
                    (timing, Event::NoteOn { note, velocity: ((velocity * 127.0).round() as u8).clamp(1, 127) })
                }
                NoteEvent::NoteOff { timing, key: Key::Number(note), velocity, .. } => {
                    (timing, Event::NoteOff { note, velocity: (velocity * 127.0).round().clamp(0.0, 127.0) as u8 })
                }
                // Every key released (CLAP wildcard).
                NoteEvent::NoteOff { timing, key: Key::Wildcard, .. } => (timing, Event::AllNotesOff),
                NoteEvent::MidiCC { timing, cc, value, .. } => {
                    (timing, Event::Control { cc, value: (value * 127.0).round().clamp(0.0, 127.0) as u8 })
                }
                _ => continue,
            };
            if self.events.len() < MAX_EVENTS {
                self.events.push(TimedEvent { frame: timing.min(frames.saturating_sub(1) as u32), event });
            }
        }
        let [left, right, ..] = buffer.as_slice() else { return ProcessStatus::Normal };
        let (mut done, mut i) = (0, 0);
        while done < frames {
            let n = (frames - done).min(MAX_BLOCK);
            self.piece.clear();
            while i < self.events.len() && (self.events[i].frame as usize) < done + n {
                let e = self.events[i];
                self.piece.push(TimedEvent { frame: e.frame - done as u32, event: e.event });
                i += 1;
            }
            slot.process_stereo(&self.piece, &mut left[done..done + n], &mut right[done..done + n]);
            done += n;
        }
        slot.report_load((start.elapsed().as_secs_f64() * self.sample_rate / frames.max(1) as f64) as f32);
        // Strings and room ring after the last note.
        ProcessStatus::KeepAlive
    }
}

impl ClapPlugin for Piano {
    const CLAP_ID: &'static str = "fr.mgth.physical-piano";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Physical model of a grand piano");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::Instrument, ClapFeature::Synthesizer, ClapFeature::Stereo];
}

impl Vst3Plugin for Piano {
    const VST3_CLASS_ID: [u8; 16] = *b"MgthPhysPiano001";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Instrument, Vst3SubCategory::Piano];
}

nice_export_clap!(Piano);
nice_export_vst3!(Piano);
