//! The interface window (`--ui`): the instrument's parameters, presets and
//! MIDI learn, drawn by piano-ui on the shared controller, and the player's
//! own "Audio and MIDI" page. It runs on the main thread, which also owns
//! the audio set-up (a cpal stream may not leave it); the supervising loop
//! keeps polling the controller when the window is hidden.

use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use piano_control::Controller;
use piano_ui::PianoUi;

use crate::audio::{self, Audio};
use crate::settings::{PageState, Request, SettingsPage};

/// The application's icon: assets/icon.svg rendered at 256 px
/// (`rsvg-convert -w 256 -h 256 assets/icon.svg -o assets/icon-256.png`).
const ICON: &[u8] = include_bytes!("../../../assets/icon-256.png");

/// Repaints the window from other threads once it exists.
pub type Repaint = Arc<OnceLock<egui::Context>>;

struct Window {
    ui: PianoUi,
    control: Arc<Mutex<Controller>>,
    shot: piano_ui::shot::Shot,
    audio: Audio,
    page: Arc<Mutex<PageState>>,
}

impl eframe::App for Window {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let mut control = self.control.lock().unwrap();
        control.poll();
        self.ui.show(ui, &mut *control);
        // Voices and CPU load.
        ui.ctx().request_repaint_after(Duration::from_millis(250));
        drop(control);
        // The settings page's request, with the controller free: a restart
        // swaps a new one into it.
        let request = self.page.lock().unwrap().request.take();
        if let Some(r) = request {
            self.answer(r);
            ui.ctx().request_repaint();
        }
        self.shot.frame(ui.ctx());
    }
}

impl Window {
    fn answer(&mut self, request: Request) {
        let mut error = None;
        if let Request::Apply(s) = request {
            if let Err(e) = self.audio.apply(s, &self.control) {
                error = Some(e);
            }
            let file = self.page.lock().unwrap().file.clone();
            if let Some(f) = file
                && let Err(e) = audio::save(&self.audio.settings, &f)
            {
                error = Some(e);
            }
        }
        let choices = self.audio.choices();
        let mut st = self.page.lock().unwrap();
        st.settings = self.audio.settings.clone();
        st.choices = choices;
        st.error = error;
    }
}

/// Opens the window and returns when it is closed. `play_toml`: where the
/// audio and MIDI settings are saved.
#[allow(clippy::too_many_arguments)]
pub fn run(
    control: Arc<Mutex<Controller>>,
    repaint: Repaint,
    presets: Option<PathBuf>,
    shipped: Vec<PathBuf>,
    preset: Option<String>,
    title: &str,
    audio: Audio,
    play_toml: Option<PathBuf>,
) -> eframe::Result {
    let mut ui = PianoUi::new(presets);
    ui.add_preset_dirs(shipped);
    if let Some(p) = preset {
        ui.set_preset_name(&p);
    }
    let page = Arc::new(Mutex::new(PageState {
        settings: audio.settings.clone(),
        choices: audio.choices(),
        error: None,
        file: play_toml,
        request: None,
    }));
    ui.set_host_page(Box::new(SettingsPage(page.clone())));
    if let Ok(p) = std::env::var("PIANO_UI_PAGE") {
        ui.open_page(&p);
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
            Ok(Box::new(Window { ui, control, shot: piano_ui::shot::Shot::from_env(), audio, page }))
        }),
    )
}
