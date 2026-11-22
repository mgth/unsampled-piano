//! The window: a keyboard to pick (and hear) a note, the note's measures
//! on every piano that recorded it, its partial profile, then one plot per
//! measure across the keyboard, its keys under the keyboard's.

use crate::audio::{Audio, Player};
use crate::jobs::{Cache, Done, Job, Workers};
use crate::measure::{self, Analysis, Id, DEFS, PARTIALS};
use crate::model::{self, ModelSpec};
use crate::plot::{self, Frame, Series, AXIS, RIGHT};
use crate::settings::{self, Settings};
use crate::sources::{is_black, note_name, parse_key, parse_layer, scan_dir, Role, Source, HIGH, LAYERS, LOW};
use egui::{Align2, Color32, FontId, Key, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, Ui, Vec2};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Band {
    MinMax,
    MeanSd,
    Off,
}

pub struct App {
    sources: Vec<Source>,
    model: Option<usize>,
    spec: Option<Arc<ModelSpec>>,
    results: HashMap<(usize, u8, u8), Analysis>,
    /// The model's renders released at 2 s, and their release measures.
    release_files: BTreeMap<(u8, u8), PathBuf>,
    release_results: HashMap<(u8, u8), Analysis>,
    /// The pianos that judge the release measures (those whose notes release
    /// the key within the file), in place of band 1.
    release_band: Vec<bool>,
    failed: HashMap<(usize, u8, u8), String>,
    rendering: HashSet<(u8, u8)>,
    workers: Workers,
    cache: Cache,
    unsaved: usize,
    audio: Result<Audio, String>,
    playing: Option<(usize, u8, u8)>,
    note: u8,
    layer: u8,
    /// The piano the keyboard plays.
    current: usize,
    visible: Vec<bool>,
    /// Pianos the band and the model's z are computed on (the real
    /// pianos by default), apart from those shown.
    in_band: Vec<bool>,
    /// A second group, empty by default, to set against the first (pianos
    /// sorted by ear, say). It has no part in the model's z.
    in_band2: Vec<bool>,
    shown: Vec<bool>,
    band: Band,
    match_levels: bool,
    autoplay: bool,
    seconds: f32,
    volume_db: f32,
    shot: Option<Shot>,
    /// Selections as last saved, and when they last changed since.
    saved: Settings,
    changed: Option<std::time::Instant>,
    /// Height of the part that does not scroll.
    fixed_height: f32,
}

/// Window capture for visual checks: with `PIANO_COMPARE_SHOT=<file.ppm>`
/// the window saves itself once the queues are empty (or after
/// `PIANO_COMPARE_SHOT_AFTER` seconds), scrolled down by
/// `PIANO_COMPARE_SCROLL` pixels, then closes. No audio is opened.
struct Shot {
    path: String,
    after: f64,
    scroll: f32,
    start: std::time::Instant,
    sent: bool,
    done: Arc<std::sync::atomic::AtomicBool>,
}

impl Shot {
    fn from_env() -> Option<Self> {
        let path = std::env::var("PIANO_COMPARE_SHOT").ok()?;
        let num = |k: &str, d: f64| std::env::var(k).ok().and_then(|v| v.parse().ok()).unwrap_or(d);
        Some(Self {
            path,
            after: num("PIANO_COMPARE_SHOT_AFTER", 600.0),
            scroll: num("PIANO_COMPARE_SCROLL", 0.0) as f32,
            start: std::time::Instant::now(),
            sent: false,
            done: Arc::default(),
        })
    }

    fn frame(&mut self, ctx: &egui::Context, idle: bool) {
        use std::sync::atomic::Ordering;
        ctx.request_repaint_after(std::time::Duration::from_millis(200));
        let t = self.start.elapsed().as_secs_f64();
        if !self.sent && t > 2.0 && (idle || t > self.after) {
            self.sent = true;
            let (path, done, ctx2) = (self.path.clone(), self.done.clone(), ctx.clone());
            let cb = egui::ScreenshotCallback::new(move |img| {
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
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(cb));
        }
        if self.done.load(Ordering::Relaxed) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

const PALETTE: [Color32; 11] = [
    Color32::from_rgb(78, 121, 167),
    Color32::from_rgb(89, 161, 79),
    Color32::from_rgb(156, 117, 95),
    Color32::from_rgb(118, 183, 178),
    Color32::from_rgb(237, 201, 72),
    Color32::from_rgb(176, 122, 161),
    Color32::from_rgb(255, 157, 167),
    Color32::from_rgb(140, 209, 125),
    Color32::from_rgb(134, 188, 182),
    Color32::from_rgb(242, 142, 43),
    Color32::from_rgb(186, 176, 172),
];
const MODEL_COLOR: Color32 = Color32::from_rgb(225, 60, 70);
/// The commercial D, the Concert grand's reference: its own colour, solid.
const D_COLOR: Color32 = Color32::from_rgb(190, 100, 235);
const D_KEY: &str = "commercial_d";

/// Coverage first: every octave, then every third key, then the others.
fn stage(m: u8) -> u8 {
    let i = m - LOW;
    if i.is_multiple_of(12) {
        0
    } else if i.is_multiple_of(3) {
        1
    } else {
        2
    }
}

impl App {
    #[allow(clippy::too_many_arguments)]
    pub fn new(ctx: &egui::Context, mut sources: Vec<Source>, spec: Option<ModelSpec>, cache_file: PathBuf, analysers: usize, renderers: usize, note: Option<u8>, layer: Option<u8>) -> Self {
        let shot = Shot::from_env();
        let spec = spec.map(Arc::new);
        let model = spec.as_ref().map(|s| {
            let mut src = Source {
                key: "model".into(),
                label: s.label.clone(),
                description: format!("Rendered by this build (renders cached by preset, engine sources and protocol), close microphones in a studio, Disklavier velocity scale (as the bench). Cached in {}", s.dir.display()),
                role: Role::Model,
                dynamics: true,
                dir: s.dir.clone(),
                held: None,
                files: Default::default(),
            };
            src.scan();
            sources.insert(0, src);
            0
        });
        for s in &mut sources {
            if s.role != Role::Model {
                s.scan();
            }
        }
        let workers = Workers::start(ctx.clone(), spec.clone(), analysers, renderers);
        let release_files = spec.as_ref().map(|s| scan_dir(&s.release_dir)).unwrap_or_default();
        let cache = Cache::open(cache_file);
        let n = sources.len();
        let mut app = Self {
            current: model.unwrap_or(0),
            sources,
            model,
            spec,
            results: HashMap::new(),
            release_files,
            release_results: HashMap::new(),
            release_band: Vec::new(),
            failed: HashMap::new(),
            rendering: HashSet::new(),
            workers,
            cache,
            unsaved: 0,
            audio: if shot.is_some() { Err("off while capturing".into()) } else { Audio::open() },
            playing: None,
            note: 60,
            layer: 8,
            visible: vec![true; n],
            in_band: Vec::new(),
            in_band2: vec![false; n],
            shown: vec![true; DEFS.len()],
            band: Band::MinMax,
            match_levels: true,
            autoplay: true,
            seconds: 3.0,
            volume_db: -6.0,
            shot,
            saved: Settings::default(),
            changed: None,
            fixed_height: 760.0,
        };
        app.in_band = app.sources.iter().map(|s| s.role == Role::Target).collect();
        app.release_band = app.sources.iter().map(|s| s.releases()).collect();
        let saved = settings::load();
        app.apply(&saved);
        app.saved = saved;
        // The command line wins over the last session.
        if let Some(n) = note {
            app.note = n;
        }
        if let Some(l) = layer {
            app.layer = l;
        }
        app.queue_all();
        app
    }

    fn apply(&mut self, st: &Settings) {
        if let Some(n) = st.note.as_deref().and_then(parse_key) {
            self.note = n;
        }
        if let Some(l) = st.layer.as_deref().and_then(parse_layer) {
            self.layer = l;
        }
        if let Some(i) = st.current.as_ref().and_then(|k| self.sources.iter().position(|s| &s.key == k)) {
            self.current = i;
        }
        for (i, s) in self.sources.iter().enumerate() {
            self.visible[i] = !st.hidden.contains(&s.key);
            if s.role != Role::Model {
                if st.band_in.contains(&s.key) {
                    self.in_band[i] = true;
                }
                if st.band_out.contains(&s.key) {
                    self.in_band[i] = false;
                }
                self.in_band2[i] = st.band2.contains(&s.key);
            }
        }
        for (i, d) in DEFS.iter().enumerate() {
            self.shown[i] = !st.hidden_measures.iter().any(|n| n == d.name);
        }
        self.band = match st.band_mode.as_deref() {
            Some("mean_sd") => Band::MeanSd,
            Some("off") => Band::Off,
            Some("min_max") => Band::MinMax,
            _ => self.band,
        };
        self.match_levels = st.match_levels.unwrap_or(self.match_levels);
        self.autoplay = st.autoplay.unwrap_or(self.autoplay);
        self.seconds = st.seconds.unwrap_or(self.seconds).clamp(0.5, 3.5);
        self.volume_db = st.volume_db.unwrap_or(self.volume_db).clamp(-40.0, 0.0);
        self.fixed_height = st.fixed_height.unwrap_or(self.fixed_height).max(160.0);
    }

    fn snapshot(&self) -> Settings {
        let keys = |f: &dyn Fn(usize) -> bool| -> Vec<String> { (0..self.sources.len()).filter(|&i| f(i)).map(|i| self.sources[i].key.clone()).collect() };
        let default_in = |i: usize| self.sources[i].role == Role::Target;
        Settings {
            note: Some(note_name(self.note)),
            layer: Some(layer_name(self.layer).to_string()),
            current: Some(self.sources[self.current].key.clone()),
            hidden: keys(&|i| !self.visible[i]),
            band_in: keys(&|i| self.in_band[i] && !default_in(i)),
            band_out: keys(&|i| !self.in_band[i] && default_in(i)),
            band2: keys(&|i| self.in_band2[i]),
            hidden_measures: DEFS.iter().enumerate().filter(|(i, _)| !self.shown[*i]).map(|(_, d)| d.name.to_string()).collect(),
            band_mode: Some(match self.band {
                Band::MinMax => "min_max",
                Band::MeanSd => "mean_sd",
                Band::Off => "off",
            }
            .into()),
            match_levels: Some(self.match_levels),
            autoplay: Some(self.autoplay),
            seconds: Some(self.seconds),
            volume_db: Some(self.volume_db),
            fixed_height: Some(self.fixed_height),
        }
    }

    /// Saves the selections half a second after they stop changing.
    fn persist(&mut self, ctx: &egui::Context, now: bool) {
        let snap = self.snapshot();
        if snap == self.saved {
            self.changed = None;
            return;
        }
        let t = *self.changed.get_or_insert_with(std::time::Instant::now);
        if now || t.elapsed().as_secs_f64() > 0.5 {
            settings::save(&snap);
            self.saved = snap;
            self.changed = None;
        } else {
            ctx.request_repaint_after(std::time::Duration::from_millis(600));
        }
    }

    fn queue_all(&mut self) {
        let mut jobs: Vec<Job> = Vec::new();
        for (si, s) in self.sources.iter().enumerate() {
            for (&(midi, layer), path) in &s.files {
                match self.cache.get(path) {
                    Some(a) => {
                        self.results.insert((si, midi, layer), a);
                    }
                    None => jobs.push(Job { src: si, midi, layer, path: path.clone(), velocity: 0, held: s.held, release: false }),
                }
            }
        }
        let (note, layer) = (self.note, self.layer);
        let order = |j: &Job| (j.midi != note, stage(j.midi), j.layer != layer, j.midi, j.src);
        jobs.sort_by_key(order);
        for j in jobs {
            self.workers.analyse(j);
        }
        if let (Some(mi), Some(spec)) = (self.model, &self.spec) {
            let mut renders = Vec::new();
            for m in LOW..=HIGH {
                for &(l, _, v) in &LAYERS {
                    if !self.sources[mi].files.contains_key(&(m, l)) {
                        let path = spec.dir.join(format!("{}v{l}.wav", note_name(m)));
                        renders.push(Job { src: mi, midi: m, layer: l, path, velocity: v, held: Some(model::HOLD), release: false });
                    }
                }
            }
            renders.sort_by_key(order);
            for j in renders {
                self.rendering.insert((j.midi, j.layer));
                self.workers.render(j);
            }
            // The release renders, after the others, where a piano that
            // releases its keys has the note.
            let mut later = Vec::new();
            for m in LOW..=HIGH {
                for &(l, _, v) in &LAYERS {
                    let judged = self.sources.iter().any(|s| s.releases() && s.files.contains_key(&(m, l)));
                    let job = |path: PathBuf| Job { src: mi, midi: m, layer: l, path, velocity: v, held: Some(model::RELEASE_HOLD), release: true };
                    match self.release_files.get(&(m, l)) {
                        Some(path) => match self.cache.get(path) {
                            Some(a) => {
                                self.release_results.insert((m, l), a);
                            }
                            None => self.workers.analyse(job(path.clone())),
                        },
                        None if judged => later.push(job(spec.release_dir.join(format!("{}v{l}.wav", note_name(m))))),
                        None => {}
                    }
                }
            }
            later.sort_by_key(order);
            for j in later {
                self.workers.render(j);
            }
        }
    }

    fn poll(&mut self) {
        while let Ok(d) = self.workers.rx.try_recv() {
            match d {
                Done::Measured { job, stamp, analysis } => {
                    self.cache.put(&job.path, stamp, analysis.clone());
                    self.unsaved += 1;
                    if job.release {
                        self.release_results.insert((job.midi, job.layer), analysis);
                    } else {
                        self.results.insert((job.src, job.midi, job.layer), analysis);
                    }
                }
                Done::Rendered { job } if job.release => {
                    self.release_files.insert((job.midi, job.layer), job.path.clone());
                    self.workers.analyse(Job { velocity: 0, ..job });
                }
                Done::Rendered { job } => {
                    self.rendering.remove(&(job.midi, job.layer));
                    self.sources[job.src].files.insert((job.midi, job.layer), job.path.clone());
                    let m = job.midi;
                    self.workers.analyse(Job { velocity: 0, ..job });
                    self.workers.prioritise(m);
                    self.workers.prioritise(self.note);
                }
                Done::Failed { job, error } => {
                    if !job.release {
                        self.rendering.remove(&(job.midi, job.layer));
                    }
                    self.failed.insert((job.src, job.midi, job.layer), error);
                }
            }
        }
        let (a, r, busy) = self.workers.pending();
        if self.cache.dirty && (self.unsaved >= 200 || a + r + busy == 0) {
            self.cache.save();
            self.unsaved = 0;
        }
    }

    fn color(&self, si: usize) -> Color32 {
        if self.sources[si].key == D_KEY {
            return D_COLOR;
        }
        match self.sources[si].role {
            Role::Model => MODEL_COLOR,
            _ => {
                let k = self.sources[..si].iter().filter(|s| s.role != Role::Model && s.key != D_KEY).count();
                PALETTE[k % PALETTE.len()]
            }
        }
    }

    /// One measure of one piano at one key and layer.
    fn value(&self, si: usize, midi: u8, layer: u8, id: Id) -> Option<f64> {
        if id == Id::Dynamics {
            if !self.sources[si].dynamics {
                return None;
            }
            let lv = |l| self.results.get(&(si, midi, l)).and_then(|a| a.level_db);
            return Some(lv(13)? - lv(4)?);
        }
        if Some(si) == self.model && measure::def(id).released() {
            return self.release_results.get(&(midi, layer))?.get(id);
        }
        self.results.get(&(si, midi, layer))?.get(id)
    }

    /// The pianos of band `n` for a measure: the release measures' first
    /// band is the pianos that release their keys, whatever is ticked.
    fn members(&self, n: usize, id: Id) -> &[bool] {
        match n {
            0 if measure::def(id).released() => &self.release_band,
            0 => &self.in_band,
            _ => &self.in_band2,
        }
    }

    /// Values of the pianos of a band at a key (transformed if log).
    fn band_values(&self, band: &[bool], midi: u8, id: Id) -> Vec<f64> {
        let d = measure::def(id);
        (0..self.sources.len())
            .filter(|&si| band[si])
            .filter_map(|si| self.value(si, midi, self.layer, id))
            .map(|v| tf(v, d.log))
            .filter(|v| v.is_finite())
            .collect()
    }

    /// The two bands with their fill and edge, the first drawn first.
    fn bands(&self, ui: &Ui, id: Option<Id>) -> [(&[bool], Color32, Color32); 2] {
        let edge = |n| band_ink(ui, n).gamma_multiply(0.75);
        let first = id.map_or(&self.in_band[..], |id| self.members(0, id));
        [(first, band_color(ui, 0), edge(0)), (&self.in_band2, band_color(ui, 1), edge(1))]
    }

    /// Model's distance to the pianos of band 1 at a key, in their spread.
    fn z(&self, midi: u8, id: Id) -> Option<f64> {
        let mi = self.model?;
        let d = measure::def(id);
        let v = tf(self.value(mi, midi, self.layer, id)?, d.log);
        let r = self.band_values(self.members(0, id), midi, id);
        if r.len() < d.judges() || !v.is_finite() {
            return None;
        }
        let (mean, sd) = mean_sd(&r);
        Some((v - mean) / sd.max(d.floor))
    }

    fn median_z(&self, midi: u8) -> Option<f64> {
        let mut z: Vec<f64> = DEFS.iter().enumerate().filter(|(i, _)| self.shown[*i]).filter_map(|(_, d)| self.z(midi, d.id)).map(f64::abs).collect();
        if z.is_empty() {
            return None;
        }
        z.sort_by(f64::total_cmp);
        Some(z[z.len() / 2])
    }

    fn play(&mut self, si: usize, midi: u8, layer: u8) {
        let Ok(audio) = &self.audio else { return };
        let Some(path) = self.sources[si].files.get(&(midi, layer)).cloned() else {
            if self.sources[si].role == Role::Model {
                self.workers.prioritise(midi);
            }
            return;
        };
        self.playing = Some((si, midi, layer));
        let player: Player = audio.player.clone();
        // A later request or Stop voids this one, however long it takes.
        let ticket = player.request();
        let (seconds, matched, volume) = (self.seconds as f64, self.match_levels, self.volume_db);
        std::thread::spawn(move || {
            if let Some((clip, sr, gain)) = crate::audio::prepare(&path, seconds, matched, volume) {
                player.play(ticket, &clip, sr, gain);
            }
        });
    }

    fn select(&mut self, midi: u8, play: bool) {
        self.note = midi.clamp(LOW, HIGH);
        self.workers.prioritise(self.note);
        if play {
            self.play(self.current, self.note, self.layer);
        }
    }

    fn keys(&mut self, ui: &Ui) {
        if ui.ctx().egui_wants_keyboard_input() {
            return;
        }
        let (left, right, up, down, space, k1, k2, k3) = ui.input(|i| {
            (
                i.key_pressed(Key::ArrowLeft),
                i.key_pressed(Key::ArrowRight),
                i.key_pressed(Key::ArrowUp),
                i.key_pressed(Key::ArrowDown),
                i.key_pressed(Key::Space),
                i.key_pressed(Key::Num1),
                i.key_pressed(Key::Num2),
                i.key_pressed(Key::Num3),
            )
        });
        if left && self.note > LOW {
            self.select(self.note - 1, self.autoplay);
        }
        if right && self.note < HIGH {
            self.select(self.note + 1, self.autoplay);
        }
        for (pressed, l) in [(k1, 4), (k2, 8), (k3, 13)] {
            if pressed {
                self.layer = l;
                if self.autoplay {
                    self.play(self.current, self.note, self.layer);
                }
            }
        }
        if up || down {
            let have: Vec<usize> = (0..self.sources.len()).filter(|&s| self.sources[s].files.contains_key(&(self.note, self.layer))).collect();
            if !have.is_empty() {
                let pos = have.iter().position(|&s| s == self.current);
                let next = match (pos, up) {
                    (Some(p), true) => have[(p + have.len() - 1) % have.len()],
                    (Some(p), false) => have[(p + 1) % have.len()],
                    (None, _) => have[0],
                };
                self.current = next;
                self.play(next, self.note, self.layer);
            }
        }
        if space {
            self.play(self.current, self.note, self.layer);
        }
    }

    fn toolbar(&mut self, ui: &mut Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Layer").strong());
            for &(l, name, v) in &LAYERS {
                if ui.selectable_label(self.layer == l, name).on_hover_text(format!("Layer {l} of the sets; the model plays velocity {v} (keys 1, 2, 3)")).clicked() {
                    self.layer = l;
                    if self.autoplay {
                        self.play(self.current, self.note, self.layer);
                    }
                }
            }
            ui.separator();
            if ui.button("<").on_hover_text("Previous key (left arrow)").clicked() && self.note > LOW {
                self.select(self.note - 1, self.autoplay);
            }
            ui.label(RichText::new(note_name(self.note)).strong().monospace());
            if ui.button(">").on_hover_text("Next key (right arrow)").clicked() && self.note < HIGH {
                self.select(self.note + 1, self.autoplay);
            }
            ui.separator();
            ui.checkbox(&mut self.autoplay, "Play on click").on_hover_text("A key, an arrow or a layer plays the selected piano (space plays it again, up and down change piano)");
            ui.checkbox(&mut self.match_levels, "Match levels").on_hover_text("Every recording is played at the same mean level over its first second: the sets' own levels are arbitrary");
            ui.add(egui::Slider::new(&mut self.seconds, 0.5..=3.5).suffix(" s").text("length"));
            ui.add(egui::Slider::new(&mut self.volume_db, -40.0..=0.0).suffix(" dB").text("volume"));
            if ui.button("Stop").clicked()
                && let Ok(a) = &self.audio {
                    a.player.stop();
                }
            ui.separator();
            ui.label("Band");
            egui::ComboBox::from_id_salt("band")
                .selected_text(match self.band {
                    Band::MinMax => "min to max",
                    Band::MeanSd => "mean ± sd",
                    Band::Off => "off",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.band, Band::MinMax, "min to max");
                    ui.selectable_value(&mut self.band, Band::MeanSd, "mean ± sd");
                    ui.selectable_value(&mut self.band, Band::Off, "off");
                })
                .response
                .on_hover_text("Shaded spread of the pianos ticked \"band 1\" and \"band 2\" in the list of pianos");
            ui.menu_button("Measures", |ui| {
                for (i, d) in DEFS.iter().enumerate() {
                    ui.checkbox(&mut self.shown[i], d.name);
                }
            });
        });
    }

    fn status(&self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let (a, r, busy) = self.workers.pending();
            if a + r + busy > 0 {
                ui.spinner();
                ui.label(format!("measuring {a} recordings, {r} model notes to render"));
            } else {
                ui.label(RichText::new(format!("{} recordings measured", self.results.len())).weak());
            }
            if !self.failed.is_empty() {
                let text: Vec<String> = self.failed.iter().take(20).map(|((s, m, l), e)| format!("{} {}v{l}: {e}", self.sources[*s].key, note_name(*m))).collect();
                ui.label(RichText::new(format!("{} failed", self.failed.len())).color(Color32::LIGHT_RED)).on_hover_text(text.join("\n"));
            }
            ui.separator();
            match &self.audio {
                Ok(a) => {
                    let now = self.playing.filter(|_| a.player.position().is_some());
                    match now {
                        Some((s, m, l)) => ui.label(format!("playing {} {} {}", self.sources[s].label, note_name(m), layer_name(l))),
                        None => ui.label(RichText::new(format!("output: {} ({} Hz)", a.device, a.player.rate)).weak()),
                    };
                    if now.is_some() {
                        ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
                    }
                }
                Err(e) => {
                    ui.label(RichText::new(format!("no audio: {e}")).color(Color32::LIGHT_RED));
                }
            }
        });
    }

    /// x of a key's centre within [left, right] (white keys evenly spaced).
    fn key_x(m: u8, left: f32, width: f32) -> (f32, f32) {
        let whites_before = (LOW..m).filter(|&k| !is_black(k)).count() as f32;
        let w = width / 52.0;
        if is_black(m) {
            (left + whites_before * w, w * 0.62)
        } else {
            (left + (whites_before + 0.5) * w, w)
        }
    }

    fn keyboard(&mut self, ui: &mut Ui) {
        let width = ui.available_width();
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, 112.0), Sense::click());
        let kb = Rect::from_min_max(Pos2::new(rect.left() + AXIS, rect.top()), Pos2::new(rect.right() - RIGHT, rect.bottom() - 16.0));
        let strip = Rect::from_min_max(Pos2::new(kb.left(), kb.bottom() + 4.0), Pos2::new(kb.right(), rect.bottom()));
        let painter = ui.painter_at(rect);
        let dark = ui.visuals().dark_mode;
        let accent = ui.visuals().selection.bg_fill;
        let have = |m: u8| self.sources[self.current].files.contains_key(&(m, self.layer));
        let key_rect = |m: u8| {
            let (cx, w) = Self::key_x(m, kb.left(), kb.width());
            if is_black(m) {
                Rect::from_min_max(Pos2::new(cx - w / 2.0, kb.top()), Pos2::new(cx + w / 2.0, kb.top() + kb.height() * 0.62))
            } else {
                Rect::from_min_max(Pos2::new(cx - w / 2.0, kb.top()), Pos2::new(cx + w / 2.0, kb.bottom()))
            }
        };
        let hover = resp.hover_pos().and_then(|p| {
            (LOW..=HIGH).filter(|&m| is_black(m)).chain((LOW..=HIGH).filter(|&m| !is_black(m))).find(|&m| key_rect(m).contains(p))
        });
        for black in [false, true] {
            for m in (LOW..=HIGH).filter(|&m| is_black(m) == black) {
                let r = key_rect(m);
                let base = if black { Color32::from_gray(25) } else { Color32::from_gray(245) };
                let mut fill = if m == self.note { accent } else { base };
                if !have(m) && m != self.note {
                    fill = if black { Color32::from_gray(70) } else { Color32::from_gray(if dark { 150 } else { 200 }) };
                }
                if hover == Some(m) && m != self.note {
                    fill = fill.lerp_to_gamma(accent, 0.35);
                }
                painter.rect_filled(r.shrink2(Vec2::new(0.5, 0.0)), 2.0, fill);
                painter.rect_stroke(r, 2.0, Stroke::new(1.0, Color32::from_gray(90)), StrokeKind::Inside);
                if m % 12 == 0 {
                    painter.text(Pos2::new(r.center().x, r.bottom() - 4.0), Align2::CENTER_BOTTOM, note_name(m), FontId::proportional(9.0), Color32::from_gray(60));
                }
            }
        }
        // The model's median |z| over the shown measures, key by key.
        for m in LOW..=HIGH {
            let (cx, _) = Self::key_x(m, kb.left(), kb.width());
            let w = kb.width() / 52.0 * 0.55;
            let r = Rect::from_center_size(Pos2::new(cx, strip.center().y), Vec2::new(w, strip.height() - 2.0));
            painter.rect_filled(r, 1.0, self.median_z(m).map_or(Color32::from_gray(80).gamma_multiply(0.3), z_color));
        }
        let strip_resp = ui.interact(strip, ui.scope_id().with("z-strip"), Sense::hover());
        strip_resp.on_hover_text("The model's median |z| over the measures shown, key by key: green within band 1 (<= 1), amber close (<= 2), red off");
        if let Some(m) = hover {
            let have_list: Vec<&str> = self.sources.iter().filter(|s| s.files.contains_key(&(m, self.layer))).map(|s| s.label.as_str()).collect();
            resp.clone().on_hover_text(format!("{} {}: {} piano(s)\n{}", note_name(m), layer_name(self.layer), have_list.len(), have_list.join("\n")));
        }
        if resp.clicked()
            && let Some(m) = hover {
                self.select(m, self.autoplay);
            }
    }

    fn note_table(&mut self, ui: &mut Ui) {
        let (note, layer) = (self.note, self.layer);
        ui.horizontal(|ui| {
            ui.heading(format!("{} {}", note_name(note), layer_name(layer)));
            ui.label(RichText::new(format!("{:.1} Hz", measure::f_et(note))).weak());
        });
        let rows: Vec<usize> = (0..self.sources.len()).filter(|&s| self.sources[s].files.contains_key(&(note, layer)) || self.sources[s].role == Role::Model).collect();
        let shown: Vec<&measure::Def> = DEFS.iter().enumerate().filter(|(i, _)| self.shown[*i]).map(|(_, d)| d).collect();
        let mut clicked: Option<usize> = None;
        egui::Grid::new("note-table").striped(true).spacing(Vec2::new(14.0, 1.0)).show(ui, |ui| {
            ui.label("");
            ui.label(RichText::new("Piano").strong());
            for d in &shown {
                let unit = if d.unit.is_empty() { String::new() } else { format!("\n{}", d.unit) };
                ui.label(RichText::new(format!("{}{unit}", d.name)).strong()).on_hover_text(d.help);
            }
            ui.end_row();
            for &si in &rows {
                let s = &self.sources[si];
                let avail = s.files.contains_key(&(note, layer));
                if ui.add_enabled(avail, egui::Button::new(">").small()).on_hover_text("Play").clicked() {
                    clicked = Some(si);
                }
                let mut name = RichText::new(&s.label).color(self.color(si));
                if si == self.current {
                    name = name.strong().underline();
                }
                if s.role == Role::Info {
                    name = name.italics();
                }
                let tip = format!("{}\n{}", s.description, s.files.get(&(note, layer)).map_or("not rendered yet".to_string(), |p| p.display().to_string()));
                if ui.add(egui::Label::new(name).sense(Sense::click())).on_hover_text(tip).clicked() && avail {
                    clicked = Some(si);
                }
                for d in &shown {
                    let v = self.value(si, note, layer, d.id);
                    let text = match v {
                        Some(v) => RichText::new(fmt_value(v, d)).monospace(),
                        None if self.results.contains_key(&(si, note, layer)) => RichText::new("–").weak(),
                        None if s.role == Role::Model && self.rendering.contains(&(note, layer)) => RichText::new("rendering").weak().italics(),
                        None => RichText::new("…").weak(),
                    };
                    if s.role == Role::Model {
                        let z = self.z(note, d.id);
                        let r = ui.label(match z {
                            Some(z) => text.color(z_color(z.abs())),
                            None => text,
                        });
                        if let Some(z) = z {
                            r.on_hover_text(format!("z = {z:+.2} (in spreads of the pianos in band 1)"));
                        }
                    } else {
                        ui.label(text);
                    }
                }
                ui.end_row();
            }
            for n in 0..2 {
                // The second band has its row once it holds a piano.
                if n == 1 && !self.in_band2.contains(&true) {
                    continue;
                }
                ui.label("");
                ui.label(RichText::new(format!("Band {}", n + 1)).color(band_label(ui, n))).on_hover_text("Mean ± standard deviation of the pianos in this band (a log measure: geometric); for the release drops, band 1 is the pianos that release their keys (Liang, MAPS)");
                for d in &shown {
                    let r = self.band_values(self.members(n, d.id), note, d.id);
                    if r.len() >= d.judges() {
                        let (m, sd) = mean_sd(&r);
                        let txt = if d.log {
                            format!("{} ×/ {:.2}", fmt_value(10f64.powf(m), d), 10f64.powf(sd))
                        } else {
                            format!("{} ± {:.*}", fmt_value(m, d), d.decimals, sd)
                        };
                        ui.label(RichText::new(txt).monospace().weak());
                    } else {
                        ui.label("");
                    }
                }
                ui.end_row();
            }
        });
        if let Some(si) = clicked {
            self.current = si;
            self.play(si, note, layer);
        }
    }

    fn legend(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            ui.label(RichText::new("Pianos").strong());
            band_buttons(ui, 0, &mut self.in_band, &self.sources, "The pianos of the first band; the model's z is computed on them");
            band_buttons(ui, 1, &mut self.in_band2, &self.sources, "A second group of pianos to set against the first; the model's z ignores it");
            egui::Grid::new("legend").spacing(Vec2::new(6.0, 0.0)).show(ui, |ui| {
                ui.label("");
                ui.label(RichText::new("show").small().weak()).on_hover_text("Drawn in the plots");
                ui.label(RichText::new("band 1").small().color(band_label(ui, 0))).on_hover_text("Counted in the first band and in the model's z (at least two pianos at a key)");
                ui.label(RichText::new("band 2").small().color(band_label(ui, 1))).on_hover_text("Counted in the second band (at least two pianos at a key)");
                ui.label("");
                ui.end_row();
                for si in 0..self.sources.len() {
                    let s = &self.sources[si];
                    let color = self.color(si);
                    let n = s.files.keys().filter(|k| k.1 == self.layer).count();
                    let (r, _) = ui.allocate_exact_size(Vec2::new(18.0, 10.0), Sense::hover());
                    if s.role == Role::Info && s.key != D_KEY {
                        ui.painter().extend(egui::Shape::dashed_line(&[r.left_center(), r.right_center()], Stroke::new(2.0, color), 4.0, 2.0));
                    } else {
                        ui.painter().line_segment([r.left_center(), r.right_center()], Stroke::new(if s.role == Role::Model { 3.0 } else { 2.0 }, color));
                    }
                    let kind = match s.role {
                        Role::Target => "",
                        Role::Info => " (info)",
                        Role::Model => "",
                    };
                    let held = s.held.map_or(String::new(), |h| format!("\nKey released {h:.2} s after the onset: decays fitted before it"));
                    let tip = format!("{}{held}\n{n} keys at this layer\n{}", s.description, s.dir.display());
                    let model = s.role == Role::Model;
                    let label = format!("{}{kind}", s.label);
                    ui.checkbox(&mut self.visible[si], "");
                    // The model is what the bands judge: never in them.
                    ui.add_enabled(!model, egui::Checkbox::without_text(&mut self.in_band[si]));
                    ui.add_enabled(!model, egui::Checkbox::without_text(&mut self.in_band2[si]));
                    ui.label(label).on_hover_text(tip);
                    ui.end_row();
                }
            });
        });
    }

    fn partial_plot(&self, ui: &mut Ui, height: f32) {
        let (note, layer) = (self.note, self.layer);
        ui.vertical(|ui| {
            ui.label(RichText::new(format!("Partials of {} {}, dB re the strongest (0.02 to 0.32 s)", note_name(note), layer_name(layer))).strong());
            let width = ui.available_width().max(200.0);
            let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
            let f = Frame { rect: Rect::from_min_max(rect.min + Vec2::new(40.0, 6.0), rect.max - Vec2::new(8.0, 20.0)), x: (0.5, PARTIALS as f64 + 0.5), y: (measure::PARTIAL_FLOOR_DB as f64, 3.0) };
            let (grid, text) = (ui.visuals().widgets.noninteractive.bg_stroke.color, ui.visuals().weak_text_color());
            plot::axes(ui, &f, false, grid, text);
            for k in [1, 4, 8, 12, 16, 20, 24, 28, 32] {
                ui.painter().text(f.px(k as f64, f.y.0) + Vec2::new(0.0, 3.0), Align2::CENTER_TOP, k.to_string(), FontId::proportional(10.0), text);
            }
            if self.band != Band::Off {
                for (members, color, edge) in self.bands(ui, None) {
                    let mut band = Vec::new();
                    for k in 0..PARTIALS {
                        let r: Vec<f64> = (0..self.sources.len())
                            .filter(|&s| members[s])
                            .filter_map(|s| self.results.get(&(s, note, layer))?.partials.get(k).copied().flatten())
                            .map(|v| v as f64)
                            .collect();
                        if r.len() >= 2 {
                            band.push(band_of(&r, self.band, (k + 1) as f64));
                        }
                    }
                    plot::band(ui, &f, &band, color, edge, 1.0);
                }
            }
            let series = self.sources_in_order().filter_map(|si| {
                let a = self.results.get(&(si, note, layer))?;
                let pts: Vec<(f64, f64)> = a.partials.iter().enumerate().filter_map(|(k, v)| v.map(|v| ((k + 1) as f64, v as f64))).collect();
                Some(self.series(si, pts, 1.0))
            });
            let series: Vec<Series> = series.collect();
            for s in &series {
                plot::series(ui, &f, s);
            }
            if let Some(p) = resp.hover_pos()
                && let Some((si, pi)) = plot::nearest(&f, &series, p) {
                    let (k, v) = series[si].points[pi];
                    plot::tooltip_box(ui, p, &format!("{}\npartial {k:.0}: {v:.1} dB", series[si].label, ), ui.visuals().extreme_bg_color, ui.visuals().text_color());
                }
        });
    }

    /// Shown sources, the model last (drawn on top).
    fn sources_in_order(&self) -> impl Iterator<Item = usize> + '_ {
        let n = self.sources.len();
        (0..n).filter(move |&s| self.visible[s] && self.sources[s].role != Role::Model).chain((0..n).filter(move |&s| self.visible[s] && self.sources[s].role == Role::Model))
    }

    fn series(&self, si: usize, points: Vec<(f64, f64)>, join: f64) -> Series<'_> {
        let s = &self.sources[si];
        let (width, color) = match s.role {
            Role::Model => (2.5, MODEL_COLOR),
            Role::Target => (1.3, self.color(si)),
            Role::Info if s.key == D_KEY => (1.8, D_COLOR),
            Role::Info => (1.2, self.color(si).gamma_multiply(0.8)),
        };
        Series { color, width, dashed: s.role == Role::Info && s.key != D_KEY, label: &s.label, points, join }
    }

    /// One measure across the keyboard; returns a clicked key.
    fn measure_plot(&self, ui: &mut Ui, d: &measure::Def) -> Option<u8> {
        let layer = self.layer;
        let width = ui.available_width();
        // Title, with the model's standing.
        ui.horizontal(|ui| {
            let unit = if d.unit.is_empty() { String::new() } else { format!(" ({})", d.unit) };
            ui.add_space(AXIS - 8.0);
            ui.label(RichText::new(format!("{}{unit}", d.name)).strong()).on_hover_text(d.help);
            let zs: Vec<f64> = (LOW..=HIGH).filter_map(|m| self.z(m, d.id)).collect();
            if !zs.is_empty() {
                let within = zs.iter().filter(|z| z.abs() <= 1.0).count() as f64 / zs.len() as f64;
                let off = zs.iter().filter(|z| z.abs() > 2.0).count();
                let rms = (zs.iter().map(|z| z * z).sum::<f64>() / zs.len() as f64).sqrt();
                let bias = zs.iter().sum::<f64>() / zs.len() as f64;
                ui.label(RichText::new(format!("model: {:.0} % within band 1, {off} keys off (|z| > 2), rms z {rms:.2}, mean z {bias:+.2} on {} keys", 100.0 * within, zs.len())).weak())
                    .on_hover_text("z: the model's distance to the mean of the pianos in band 1, in units of their spread (at least a floor per measure)");
            }
        });
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, 170.0), Sense::click());
        let inner = Rect::from_min_max(Pos2::new(rect.left() + AXIS, rect.top() + 4.0), Pos2::new(rect.right() - RIGHT, rect.bottom() - 4.0));
        let x_of = |m: u8| Self::key_x(m, inner.left(), inner.width()).0 as f64;
        let series: Vec<Series> = self
            .sources_in_order()
            .map(|si| {
                let pts: Vec<(f64, f64)> = (LOW..=HIGH)
                    .filter_map(|m| self.value(si, m, layer, d.id).map(|v| (x_of(m), tf(v, d.log))))
                    .filter(|p| p.1.is_finite())
                    .collect();
                // Join keys up to 3 semitones apart (sets sampled every 3rd key).
                self.series(si, pts, (inner.width() / 52.0 * 2.2) as f64)
            })
            .collect();
        let mut bands = Vec::new();
        if self.band != Band::Off {
            for (members, color, edge) in self.bands(ui, Some(d.id)) {
                let mut band = Vec::new();
                for m in LOW..=HIGH {
                    let r = self.band_values(members, m, d.id);
                    if r.len() >= d.judges() {
                        band.push(band_of(&r, self.band, x_of(m)));
                    }
                }
                bands.push((band, color, edge));
            }
        }
        let mut all: Vec<f64> = series.iter().flat_map(|s| s.points.iter().map(|p| p.1)).chain(bands.iter().flat_map(|(b, ..)| b.iter().flat_map(|b| [b.1, b.2]))).collect();
        let (y0, y1) = plot::y_range(&mut all);
        let f = Frame { rect: inner, x: (inner.left() as f64, inner.right() as f64), y: (y0, y1) };
        let (grid, text) = (ui.visuals().widgets.noninteractive.bg_stroke.color, ui.visuals().weak_text_color());
        // The selected key.
        let sx = x_of(self.note) as f32;
        ui.painter().rect_filled(Rect::from_min_max(Pos2::new(sx - inner.width() / 104.0, inner.top()), Pos2::new(sx + inner.width() / 104.0, inner.bottom())), 0.0, ui.visuals().selection.bg_fill.gamma_multiply(0.35));
        plot::axes(ui, &f, d.log, grid, text);
        // C of each octave.
        for m in (LOW..=HIGH).filter(|m| m % 12 == 0) {
            let x = x_of(m) as f32;
            ui.painter().line_segment([Pos2::new(x, inner.top()), Pos2::new(x, inner.bottom())], Stroke::new(1.0, grid.gamma_multiply(0.6)));
        }
        if (d.id == Id::Tuning || d.id == Id::Tilt || d.id == Id::DecayEarly || d.id == Id::DecayLate)
            && y0 < 0.0 && y1 > 0.0 {
                ui.painter().line_segment([f.px(f.x.0, 0.0), f.px(f.x.1, 0.0)], Stroke::new(1.0, text.gamma_multiply(0.6)));
            }
        for (band, color, edge) in &bands {
            plot::band(ui, &f, band, *color, *edge, (inner.width() / 52.0 * 2.2) as f64);
        }
        for s in &series {
            plot::series(ui, &f, s);
        }
        let key_at = |x: f32| (LOW..=HIGH).min_by(|a, b| (x_of(*a) as f32 - x).abs().total_cmp(&(x_of(*b) as f32 - x).abs()));
        if let Some(p) = resp.hover_pos() {
            match plot::nearest(&f, &series, p) {
                Some((si, pi)) => {
                    let (x, y) = series[si].points[pi];
                    let m = key_at(x as f32).unwrap_or(LOW);
                    let v = if d.log { 10f64.powf(y) } else { y };
                    plot::tooltip_box(ui, p, &format!("{}\n{} {}: {} {}", series[si].label, note_name(m), layer_name(layer), fmt_value(v, d), d.unit), ui.visuals().extreme_bg_color, ui.visuals().text_color());
                }
                None => {
                    if let Some(m) = key_at(p.x) {
                        let x = x_of(m) as f32;
                        ui.painter().line_segment([Pos2::new(x, inner.top()), Pos2::new(x, inner.bottom())], Stroke::new(1.0, text.gamma_multiply(0.5)));
                        ui.painter().text(Pos2::new(x + 4.0, inner.top() + 2.0), Align2::LEFT_TOP, note_name(m), FontId::proportional(11.0), text);
                    }
                }
            }
        }
        if resp.clicked() {
            return resp.interact_pointer_pos().and_then(|p| key_at(p.x));
        }
        None
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        self.poll();
        self.keys(ui);
        egui::Panel::top("toolbar").show(ui, |ui| {
            ui.add_space(4.0);
            self.toolbar(ui);
            ui.add_space(2.0);
        });
        egui::Panel::bottom("status").show(ui, |ui| self.status(ui));
        egui::CentralPanel::default().show(ui, |ui| {
            // Keyboard, the key's table, the pianos and the partials stay in
            // view; the measures scroll below. The split can be dragged.
            let top = egui::Panel::top("fixed")
                .frame(egui::Frame::NONE)
                .resizable(true)
                .min_size(160.0)
                .default_size(self.fixed_height)
                .show(ui, |ui| {
                    self.keyboard(ui);
                    ui.add_space(4.0);
                    egui::ScrollArea::vertical().id_salt("fixed-scroll").auto_shrink([false, false]).show(ui, |ui| {
                        egui::ScrollArea::horizontal().id_salt("table-scroll").show(ui, |ui| self.note_table(ui));
                        ui.add_space(6.0);
                        ui.horizontal_top(|ui| {
                            let legend = ui.allocate_ui(Vec2::new(300.0, 0.0), |ui| self.legend(ui)).response.rect.height();
                            // Not a separator: it would stretch the row to the window's height.
                            ui.add_space(12.0);
                            self.partial_plot(ui, (legend - 24.0).max(180.0));
                        });
                    });
                });
            self.fixed_height = top.response.rect.height().round();
            let mut area = egui::ScrollArea::vertical().auto_shrink([false, false]);
            if let Some(s) = self.shot.as_ref().filter(|s| s.scroll > 0.0) {
                area = area.vertical_scroll_offset(s.scroll);
            }
            area.show(ui, |ui| {
                ui.add_space(4.0);
                let mut clicked = None;
                for (i, d) in DEFS.iter().enumerate() {
                    if self.shown[i] {
                        if let Some(m) = self.measure_plot(ui, d) {
                            clicked = Some(m);
                        }
                        ui.add_space(6.0);
                    }
                }
                if let Some(m) = clicked {
                    self.select(m, self.autoplay);
                }
            });
        });
        let (a, r, busy) = self.workers.pending();
        let ctx = ui.ctx().clone();
        self.persist(&ctx, false);
        if let Some(s) = &mut self.shot {
            s.frame(&ctx, a + r + busy == 0);
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        let ctx = egui::Context::default();
        self.persist(&ctx, true);
        self.cache.save();
    }
}

pub(crate) fn tf(v: f64, log: bool) -> f64 {
    if log { v.log10() } else { v }
}

pub(crate) fn mean_sd(v: &[f64]) -> (f64, f64) {
    let n = v.len() as f64;
    let m = v.iter().sum::<f64>() / n;
    (m, (v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / n).sqrt())
}

fn band_of(r: &[f64], band: Band, x: f64) -> (f64, f64, f64) {
    match band {
        Band::MeanSd => {
            let (m, sd) = mean_sd(r);
            (x, m - sd, m + sd)
        }
        _ => (x, r.iter().copied().fold(f64::INFINITY, f64::min), r.iter().copied().fold(f64::NEG_INFINITY, f64::max)),
    }
}

/// A band's hue: grey-blue for the first, amber for the second.
fn band_ink(ui: &Ui, n: usize) -> Color32 {
    match (n, ui.visuals().dark_mode) {
        (0, true) => Color32::from_rgb(160, 170, 190),
        (0, false) => Color32::from_rgb(90, 110, 140),
        (_, true) => Color32::from_rgb(240, 170, 60),
        (_, false) => Color32::from_rgb(200, 125, 20),
    }
}

/// A band's name: the plots' amber is too pale for text on a light panel.
fn band_label(ui: &Ui, n: usize) -> Color32 {
    if n == 1 && !ui.visuals().dark_mode { Color32::from_rgb(155, 90, 10) } else { band_ink(ui, n) }
}

fn band_color(ui: &Ui, n: usize) -> Color32 {
    let [r, g, b, _] = band_ink(ui, n).to_array();
    let alpha = match (n, ui.visuals().dark_mode) {
        (0, true) => 40,
        (0, false) => 45,
        _ => 60,
    };
    Color32::from_rgba_unmultiplied(r, g, b, alpha)
}

/// A band's name and its shortcuts for filling it.
fn band_buttons(ui: &mut Ui, n: usize, band: &mut Vec<bool>, sources: &[Source], help: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(format!("band {}:", n + 1)).color(band_label(ui, n))).on_hover_text(help);
        if ui.small_button("real").on_hover_text("The real pianos (the bench's targets)").clicked() {
            *band = sources.iter().map(|s| s.role == Role::Target).collect();
        }
        if ui.small_button("all").on_hover_text("Every piano but the model").clicked() {
            *band = sources.iter().map(|s| s.role != Role::Model).collect();
        }
        if ui.small_button("none").clicked() {
            band.fill(false);
        }
    });
}

fn z_color(z: f64) -> Color32 {
    if z <= 1.0 {
        Color32::from_rgb(80, 175, 95)
    } else if z <= 2.0 {
        Color32::from_rgb(225, 175, 50)
    } else {
        Color32::from_rgb(225, 75, 70)
    }
}

fn fmt_value(v: f64, d: &measure::Def) -> String {
    if d.id == Id::Inharm {
        format!("{v:.2e}")
    } else {
        format!("{v:.*}", d.decimals)
    }
}

fn layer_name(l: u8) -> &'static str {
    LAYERS.iter().find(|x| x.0 == l).map_or("?", |x| x.1)
}
