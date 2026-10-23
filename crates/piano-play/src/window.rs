//! The interface window (`--ui`): the instrument's parameters, presets and
//! MIDI learn, drawn by piano-ui on the shared controller. It runs on the
//! main thread; the supervising loop keeps polling the controller when the
//! window is hidden.

use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use piano_control::Controller;
use piano_ui::PianoUi;

/// The application's icon: assets/icon.svg rendered at 256 px
/// (`rsvg-convert -w 256 -h 256 assets/icon.svg -o assets/icon-256.png`).
const ICON: &[u8] = include_bytes!("../../../assets/icon-256.png");

/// Repaints the window from other threads once it exists.
pub type Repaint = Arc<OnceLock<egui::Context>>;

struct Window {
    ui: PianoUi,
    control: Arc<Mutex<Controller>>,
    shot: piano_ui::shot::Shot,
}

impl eframe::App for Window {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let mut control = self.control.lock().unwrap();
        control.poll();
        self.ui.show(ui, &mut *control);
        // Voices and CPU load.
        ui.ctx().request_repaint_after(Duration::from_millis(250));
        drop(control);
        self.shot.frame(ui.ctx());
    }
}

/// Opens the window and returns when it is closed.
pub fn run(
    control: Arc<Mutex<Controller>>,
    repaint: Repaint,
    presets: Option<PathBuf>,
    preset: Option<String>,
    title: &str,
) -> eframe::Result {
    let mut ui = PianoUi::new(presets);
    if let Some(p) = preset {
        ui.set_preset_name(&p);
    }
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1100.0, 760.0])
        .with_min_inner_size([760.0, 480.0])
        .with_title(title)
        .with_app_id("unsampled-piano");
    if let Ok(icon) = eframe::icon_data::from_png_bytes(ICON) {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions { viewport, ..Default::default() };
    eframe::run_native(
        "unsampled-piano",
        options,
        Box::new(move |cc| {
            let _ = repaint.set(cc.egui_ctx.clone());
            Ok(Box::new(Window { ui, control, shot: piano_ui::shot::Shot::from_env() }))
        }),
    )
}
