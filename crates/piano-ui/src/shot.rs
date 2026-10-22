//! Window capture for visual checks: with `PIANO_UI_SHOT=<file.ppm>`, the
//! window saves itself once drawn, then closes.

pub struct Shot {
    path: Option<String>,
    frames: u32,
}

impl Shot {
    pub fn from_env() -> Self {
        Self { path: std::env::var("PIANO_UI_SHOT").ok(), frames: 0 }
    }

    pub fn active(&self) -> bool {
        self.path.is_some()
    }

    /// Call at the end of each frame.
    pub fn frame(&mut self, ctx: &egui::Context) {
        let Some(path) = &self.path else { return };
        self.frames += 1;
        // Let the layout settle (and a playing engine report) first.
        if self.frames == 30 {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(30));
        let image = ctx.input(|i| {
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
            if let Err(e) = std::fs::write(path, ppm) {
                eprintln!("{path}: {e}");
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}
