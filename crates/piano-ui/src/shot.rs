//! Window capture for visual checks: with `PIANO_UI_SHOT=<file.ppm>`, the
//! window saves itself once drawn, then closes.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct Shot {
    path: Option<String>,
    frames: u32,
    done: Arc<AtomicBool>,
}

impl Shot {
    pub fn from_env() -> Self {
        Self { path: std::env::var("PIANO_UI_SHOT").ok(), frames: 0, done: Arc::default() }
    }

    pub fn active(&self) -> bool {
        self.path.is_some()
    }

    /// Call at the end of each frame.
    pub fn frame(&mut self, ctx: &egui::Context) {
        let Some(path) = self.path.clone() else { return };
        self.frames += 1;
        ctx.request_repaint_after(std::time::Duration::from_millis(30));
        // Let the layout settle (and a playing engine report) first.
        if self.frames == 30 {
            let (done, ctx2) = (self.done.clone(), ctx.clone());
            let shot = egui::ScreenshotCallback::new(move |img| {
                let mut ppm = format!("P6 {} {} 255\n", img.width(), img.height()).into_bytes();
                for p in &img.pixels {
                    ppm.extend_from_slice(&[p.r(), p.g(), p.b()]);
                }
                if let Err(e) = std::fs::write(&path, ppm) {
                    eprintln!("{path}: {e}");
                }
                done.store(true, Ordering::Relaxed);
                ctx2.request_repaint();
            });
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(shot));
        }
        if self.done.load(Ordering::Relaxed) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}
