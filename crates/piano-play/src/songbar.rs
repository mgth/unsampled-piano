//! The window's MIDI file bar: open a file (or drop it on the window),
//! play, pause, stop, and move through it (crate::song).

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use egui::{RichText, Ui};

use crate::song::{Command, Song, Transport};

/// How long the slider stays where it was let go, while the audio side
/// gets there.
const SEEK_HOLD: Duration = Duration::from_millis(300);

pub struct SongBar {
    transport: Arc<Mutex<Transport>>,
    /// The file dialog, open in its own thread.
    picking: Option<mpsc::Receiver<Option<PathBuf>>>,
    /// Folder of the last file opened.
    dir: Option<PathBuf>,
    error: Option<String>,
    /// The slider's time while dragged, or just let go.
    scrub: Option<f64>,
    seeked: Option<Instant>,
}

impl SongBar {
    pub fn new(transport: Arc<Mutex<Transport>>) -> Self {
        Self { transport, picking: None, dir: None, error: None, scrub: None, seeked: None }
    }

    fn open(&mut self, path: &Path) {
        match Song::read(path) {
            Ok(song) => {
                self.transport.lock().unwrap().load(song, true);
                self.dir = path.parent().map(Path::to_path_buf);
                self.error = None;
            }
            Err(e) => self.error = Some(e),
        }
    }

    /// The system's file dialog. Off the interface's thread where it may
    /// (the window keeps drawing); macOS wants its dialogs on the main one.
    fn pick(&mut self) {
        let mut dialog = rfd::FileDialog::new().set_title("Open a MIDI file").add_filter("MIDI files", &["mid", "midi", "MID", "MIDI"]);
        if let Some(d) = &self.dir {
            dialog = dialog.set_directory(d);
        }
        if cfg!(target_os = "macos") {
            if let Some(p) = dialog.pick_file() {
                self.open(&p);
            }
            return;
        }
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(dialog.pick_file());
        });
        self.picking = Some(rx);
    }
}

enum Icon {
    Play,
    Pause,
    Stop,
}

/// A button with a transport sign drawn on it (the interface's fonts have
/// none of them).
fn icon_button(ui: &mut Ui, icon: Icon) -> egui::Response {
    let h = ui.spacing().interact_size.y;
    let r = ui.add(egui::Button::new("").min_size(egui::vec2(h * 1.4, h)));
    let color = ui.style().interact(&r).fg_stroke.color;
    let c = r.rect.center();
    let s = h * 0.24;
    let p = ui.painter();
    match icon {
        Icon::Play => {
            let pts = vec![c + egui::vec2(-0.8 * s, -s), c + egui::vec2(1.1 * s, 0.0), c + egui::vec2(-0.8 * s, s)];
            p.add(egui::Shape::convex_polygon(pts, color, egui::Stroke::NONE));
        }
        Icon::Pause => {
            for x in [-0.55 * s, 0.55 * s] {
                p.rect_filled(egui::Rect::from_center_size(c + egui::vec2(x, 0.0), egui::vec2(0.5 * s, 2.0 * s)), 1.0, color);
            }
        }
        Icon::Stop => {
            p.rect_filled(egui::Rect::from_center_size(c, egui::vec2(1.8 * s, 1.8 * s)), 1.0, color);
        }
    }
    r
}

fn is_midi(path: &Path) -> bool {
    path.extension().is_some_and(|e| e.eq_ignore_ascii_case("mid") || e.eq_ignore_ascii_case("midi"))
}

/// m:ss
fn clock(t: f64) -> String {
    let s = t.max(0.0) as u64;
    format!("{}:{:02}", s / 60, s % 60)
}

impl piano_ui::HostBar for SongBar {
    fn show(&mut self, ui: &mut Ui) {
        if let Some(rx) = &self.picking {
            match rx.try_recv() {
                Ok(path) => {
                    self.picking = None;
                    if let Some(p) = path {
                        self.open(&p);
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => self.picking = None,
                Err(mpsc::TryRecvError::Empty) => ui.ctx().request_repaint_after(Duration::from_millis(100)),
            }
        }
        let dropped = ui.ctx().input(|i| i.raw.dropped_files.iter().map(|f| f.path()).find(|p| is_midi(p)).map(Path::to_path_buf));
        if let Some(p) = dropped {
            self.open(&p);
        }

        let transport = self.transport.clone();
        let mut t = transport.lock().unwrap();
        t.free();
        let (position, playing) = (t.state().position(), t.state().playing());
        let song = t.song().map(|s| (s.name.clone(), s.length));
        let mut pick = false;
        ui.horizontal(|ui| {
            let open = ui.add_enabled(self.picking.is_none(), egui::Button::new("Open MIDI file…"));
            pick = open.on_hover_text("Play a standard MIDI file (.mid); a file dropped on the window plays too").clicked();
            let Some((name, length)) = song else {
                ui.label(RichText::new("No MIDI file").weak());
                if let Some(e) = &self.error {
                    ui.colored_label(ui.visuals().error_fg_color, e);
                }
                return;
            };
            ui.separator();
            if playing {
                if icon_button(ui, Icon::Pause).on_hover_text("Pause").clicked() {
                    t.send(Command::Pause);
                }
                ui.ctx().request_repaint_after(Duration::from_millis(100));
            } else if icon_button(ui, Icon::Play).on_hover_text("Play").clicked() {
                t.send(Command::Play);
            }
            if icon_button(ui, Icon::Stop).on_hover_text("Stop and go back to the start").clicked() {
                t.send(Command::Pause);
                t.send(Command::Seek(0.0));
                (self.scrub, self.seeked) = (Some(0.0), Some(Instant::now()));
            }
            ui.label(RichText::new(&name).strong());
            if let Some(e) = &self.error {
                ui.colored_label(ui.visuals().error_fg_color, e);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if self.seeked.is_some_and(|at| at.elapsed() > SEEK_HOLD) {
                    (self.scrub, self.seeked) = (None, None);
                }
                let mut at = self.scrub.unwrap_or(position).min(length);
                ui.label(format!("{} / {}", clock(at), clock(length)));
                ui.spacing_mut().slider_width = ui.available_width().max(60.0);
                let r = ui.add(egui::Slider::new(&mut at, 0.0..=length.max(0.001)).show_value(false));
                if r.dragged() {
                    (self.scrub, self.seeked) = (Some(at), None);
                }
                if r.drag_stopped() || (r.changed() && !r.dragged()) {
                    t.send(Command::Seek(at));
                    (self.scrub, self.seeked) = (Some(at), Some(Instant::now()));
                }
            });
        });
        // The dialog opens the file it gives: on macOS, before returning.
        drop(t);
        if pick {
            self.pick();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The bar's texts are in the interface's fonts.
    #[test]
    fn the_buttons_have_their_glyphs() {
        let ctx = egui::Context::default();
        ctx.run_ui(Default::default(), |_| {}).textures_delta.clear();
        let font = egui::FontId::proportional(14.0);
        let missing: Vec<char> = ctx.fonts_mut(|f| "Open MIDI file… No MIDI file 0:00 / 1:23".chars().filter(|&c| !f.has_glyph(&font, c)).collect());
        assert!(missing.is_empty(), "no glyph for {missing:?}");
    }

    #[test]
    fn midi_files_by_their_extension() {
        assert!(is_midi(Path::new("a/b.MID")) && is_midi(Path::new("c.midi")));
        assert!(!is_midi(Path::new("c.toml")) && !is_midi(Path::new("mid")));
        assert_eq!(clock(125.7), "2:05");
    }
}
