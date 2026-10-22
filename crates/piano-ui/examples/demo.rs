//! The interface on a mock control (no sound), to work on it alone.
//!   cargo run -p piano-ui --example demo [preset-folder]
//! PIANO_UI_PAGE=<title> opens a page; PIANO_UI_LOAD=<preset.toml> starts
//! from a preset; PIANO_UI_SHOT=<file.ppm> saves the
//! window once drawn, then quits (visual checks).
use piano_core::params::PianoModel;
use piano_ui::{mock::MockControl, PianoUi};

struct Demo {
    ui: PianoUi,
    control: MockControl,
    shot: Option<String>,
    frames: u32,
}

impl eframe::App for Demo {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.ui.show(ui, &mut self.control);
        let Some(path) = self.shot.clone() else { return };
        self.frames += 1;
        if self.frames == 5 {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
        }
        ui.ctx().request_repaint();
        let image = ui.ctx().input(|i| {
            i.events.iter().find_map(|e| match e {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(img) = image {
            let mut ppm = format!("P6 {} {} 255\n", img.width(), img.height()).into_bytes();
            for p in &img.pixels {
                ppm.extend_from_slice(&[p.r(), p.g(), p.b()]);
            }
            std::fs::write(&path, ppm).expect("screenshot written");
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
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
    let demo = Demo { ui, control: MockControl::new(model), shot: std::env::var("PIANO_UI_SHOT").ok(), frames: 0 };
    eframe::run_native("piano-ui-demo", options, Box::new(|_| Ok(Box::new(demo))))
}
