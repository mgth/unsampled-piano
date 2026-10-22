//! The non-real-time side while the plugin lives, every 50 ms: a restored
//! state rebuilds the instrument, the host's live values reach the edited
//! model, the controller is polled (MIDI learn, rebuild edits), and the
//! model and MIDI map go back to the state the host saves.

use std::sync::atomic::Ordering;
use std::time::Duration;

use piano_params::{PianoControl, Value};

use crate::Shared;

pub fn run(shared: &Shared) {
    let params = &shared.params;
    let mut seen = String::new();
    while !shared.stop.load(Ordering::Relaxed) {
        std::thread::sleep(Duration::from_millis(50));
        let mut guard = shared.control.lock().unwrap();
        let Some(control) = guard.as_mut() else { continue };
        if params.restored.swap(false, Ordering::AcqRel) {
            control.load(params.model());
            control.set_midi_map(params.midi.read().unwrap().clone());
        }
        // Automation moved a live parameter: the edited model follows (the
        // audio thread already applied it).
        let model = control.model();
        for (id, p) in &params.live {
            let v = p.value() as f64;
            if (id.get(&model) - v).abs() > 1e-6 * v.abs().max(1e-6) {
                control.set(id.path(), Value::Float(v));
            }
        }
        control.poll();
        let model = control.model();
        let now = format!("{model:?}");
        if now != seen {
            seen = now;
            *params.model.write().unwrap() = model;
            shared.repaint.request_repaint();
        }
        if let Some(map) = control.midi().map(|l| l.map().clone())
            && map != *params.midi.read().unwrap()
        {
            *params.midi.write().unwrap() = map;
        }
    }
}
