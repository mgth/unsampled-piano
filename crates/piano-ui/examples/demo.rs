//! The interface on a mock control (no sound), to work on it alone. MIDI
//! controllers from every input port drive it (MIDI learn).
//!   cargo run -p piano-ui --example demo [preset-folder]
//! PIANO_UI_PAGE=<title> opens a page; PIANO_UI_LOAD=<preset.toml> starts
//! from a preset; PIANO_UI_MAP=<cc>=<path>,… maps controllers and
//! PIANO_UI_LEARN=<path> starts learning one; PIANO_UI_SHOT=<file.ppm> saves the
//! window once drawn, then quits (visual checks).
use std::sync::mpsc;

use piano_core::{params::PianoModel, Event};
use piano_ui::{mock::MockControl, PianoUi};

struct Demo {
    ui: PianoUi,
    control: MockControl,
    cc: mpsc::Receiver<(u8, u8)>,
    _ports: Vec<midir::MidiInputConnection<()>>,
    shot: piano_ui::shot::Shot,
}

impl eframe::App for Demo {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        while let Ok((cc, value)) = self.cc.try_recv() {
            self.control.on_cc(cc, value);
        }
        self.ui.show(ui, &mut self.control);
        self.shot.frame(ui.ctx());
    }
}

fn main() -> eframe::Result {
    let presets = std::env::args().nth(1).map(Into::into);
    let mut ui = PianoUi::new(presets);
    if let Ok(page) = std::env::var("PIANO_UI_PAGE") {
        ui.open_page(&page);
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1100.0, 760.0]).with_title("Piano"),
        ..Default::default()
    };
    let model = match std::env::var("PIANO_UI_LOAD") {
        Ok(p) => piano_host::load_preset(p.as_ref()).unwrap_or_else(|e| panic!("{e}")),
        Err(_) => PianoModel::default(),
    };
    let shot = piano_ui::shot::Shot::from_env();
    eframe::run_native(
        "piano-ui-demo",
        options,
        Box::new(move |cc| {
            let (tx, rx) = mpsc::channel();
            let ports = if shot.active() { Vec::new() } else { listen(tx, cc.egui_ctx.clone()) };
            let mut control = MockControl::new(model);
            use piano_params::PianoControl;
            for (cc, path) in std::env::var("PIANO_UI_MAP").unwrap_or_default().split(',').filter_map(|m| m.split_once('=')) {
                control.midi().unwrap().learn(path);
                control.on_cc(cc.parse().expect("CC number"), 64);
            }
            if let Ok(path) = std::env::var("PIANO_UI_LEARN") {
                control.midi().unwrap().learn(&path);
            }
            Ok(Box::new(Demo { ui, control, cc: rx, _ports: ports, shot }))
        }),
    )
}

/// Every MIDI input port, CCs sent to the window (which repaints).
fn listen(tx: mpsc::Sender<(u8, u8)>, ctx: egui::Context) -> Vec<midir::MidiInputConnection<()>> {
    let Ok(probe) = midir::MidiInput::new("piano-ui-demo") else { return Vec::new() };
    let mut conns = Vec::new();
    for port in probe.ports() {
        let name = probe.port_name(&port).unwrap_or_default();
        let Ok(input) = midir::MidiInput::new("piano-ui-demo") else { continue };
        let (tx, ctx) = (tx.clone(), ctx.clone());
        let conn = input.connect(
            &port,
            "piano-ui-demo",
            move |_, msg, _| {
                if let Some(Event::Control { cc, value }) = piano_host::decode_midi(msg) {
                    let _ = tx.send((cc, value));
                    ctx.request_repaint();
                }
            },
            (),
        );
        match conn {
            Ok(c) => {
                eprintln!("MIDI in: {name}");
                conns.push(c);
            }
            Err(e) => eprintln!("MIDI in {name}: {e}"),
        }
    }
    conns
}
