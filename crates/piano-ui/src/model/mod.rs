//! The Model page: the instrument as the model builds it, to scale, in
//! three ways.
//!
//! - Static: every part where the parameters put it, with its numbers;
//!   pointing at a parameter elsewhere lights its part, clicking a part's
//!   name opens its page.
//! - Live: the same instrument with the engine's state on it, as it plays
//!   ([`piano_params::PianoControl::observe`]): keys and pedals down,
//!   hammers on the strings, dampers lifted, the strings' energy (their
//!   motion slowed down and enlarged to be seen) and the soundboard's
//!   motion from its lowest modes.
//! - Editor: the plan with handles on the geometry (board size, bridges,
//!   agraffes, frame, microphones) and the geometric parameters beside it;
//!   moving them rebuilds the instrument as any other edit.
//!
//! The plan (an orthographic view of the board's face) is drawn by egui
//! everywhere. The 3D view, which turns around the instrument, is drawn by
//! the GPU where the host enables it ([`ModelView::enable_gpu`], feature
//! `gpu`): the standalone player does; the plugin not yet.

mod cam;
#[cfg(feature = "gpu")]
mod gpu;
mod paint;
pub mod scene;

use std::sync::Arc;
use std::time::Instant;

use egui::{Align2, Color32, FontId, Rect, RichText, Sense, Stroke, Ui, Vec2};
use piano_core::observe::OBSERVED_MODES;
use piano_core::params::PianoModel;
use piano_core::soundboard::{Soundboard, SoundboardParams};
use piano_core::Observation;
use piano_params::{PianoControl, Registry, Value};

use crate::simple::Shown;
use crate::sketches::{self, Part};
use cam::{Mat4, Orbit, V3};
use scene::{Live, Options, Scene, BOARD_GRID};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Static,
    Live,
    Editor,
}

/// The geometry the editor lists beside the plan.
const GEOMETRY: &[(&str, &[&str])] = &[
    ("Soundboard", &["soundboard.a", "soundboard.b", "soundboard.outline_below", "soundboard.main_bridge_from"]),
    ("Frame", &["frame.offset_x", "frame.agraffe_x", "frame.a", "frame.b", "frame.height"]),
    ("Rim", &["rim.thickness", "rim.height", "rim.top"]),
    ("Lid", &["soundboard.lid_angle", "soundboard.lid_hinge_height"]),
    ("Along the strings", &["strike_ratio", "damper.position", "damper.highest_note"]),
];

pub struct ModelView {
    pub mode: Mode,
    three_d: bool,
    orbit: Option<Orbit>,
    /// Upright or grand the orbit was framed for.
    framed_vertical: bool,
    lid: bool,
    frame: bool,
    mics: bool,
    labels: bool,
    #[cfg(feature = "gpu")]
    gpu: Option<gpu::Gpu>,
    levels: Levels,
    shapes: Option<BoardShapes>,
    /// The handle being dragged, and the plan's matrix and rect when the
    /// drag began (the plan refits to the edited instrument only after).
    drag: Option<(usize, Mat4, Rect)>,
    /// The editor's handles as last drawn: name and place on the screen.
    pub(crate) handles_shown: Vec<(&'static str, egui::Pos2)>,
    start: Instant,
}

impl Default for ModelView {
    fn default() -> Self {
        Self {
            mode: Mode::Static,
            three_d: true,
            orbit: None,
            framed_vertical: false,
            lid: true,
            frame: true,
            mics: true,
            labels: true,
            #[cfg(feature = "gpu")]
            gpu: None,
            levels: Levels::default(),
            shapes: None,
            drag: None,
            handles_shown: Vec::new(),
            start: Instant::now(),
        }
    }
}

/// Running references of the live view's levels: the loudest of the last
/// seconds sets the top of the scale.
/// The engine's state as shown: falling back slowly like a level meter
/// (the board's beats and the dips between two looks do not flicker), the
/// blows held long enough to be seen.
struct Levels {
    keys: f32,
    board: f32,
    last: Option<Instant>,
    shown: Observation,
    /// Seconds each hammer stays shown on its strings.
    hammer: [f32; 128],
}

impl Default for Levels {
    fn default() -> Self {
        Self { keys: 0.0, board: 0.0, last: None, shown: Observation::default(), hammer: [0.0; 128] }
    }
}

/// Decay of the references (per second), and the ranges shown (dB).
const REF_DECAY: f32 = 0.8;
const KEY_RANGE_DB: f32 = 50.0;
const BOARD_RANGE_DB: f32 = 40.0;
/// Time constant of the meters' fall (s, on amplitudes), and how long a
/// blow (1–3 ms on the strings) stays shown.
const RELEASE_S: f32 = 0.3;
const BLOW_S: f32 = 0.08;

/// The board's lowest mode shapes on the view's grid, for the soundboard
/// as the engine built it.
struct BoardShapes {
    params: SoundboardParams,
    /// `phi[mode * grid + point]`.
    phi: Vec<f32>,
    modes: usize,
}

impl BoardShapes {
    fn new(params: SoundboardParams) -> Self {
        let sb = Soundboard::new(params);
        let (gx, gy) = BOARD_GRID;
        let modes = sb.modes.len().min(OBSERVED_MODES);
        let mut phi = Vec::with_capacity(modes * gx * gy);
        for md in &sb.modes[..modes] {
            for j in 0..gy {
                for i in 0..gx {
                    let (x, y) = (params.a * i as f64 / (gx - 1) as f64, params.b * j as f64 / (gy - 1) as f64);
                    phi.push(sb.shape(md, x, y) as f32);
                }
            }
        }
        Self { params, phi, modes }
    }

    /// RMS velocity at each grid point from the modes' amplitudes.
    fn field(&self, obs: &Observation) -> Vec<f32> {
        let g = BOARD_GRID.0 * BOARD_GRID.1;
        let mut v = vec![0.0f32; g];
        for (k, &a) in obs.board[..obs.board_modes.min(self.modes)].iter().enumerate() {
            if a == 0.0 {
                continue;
            }
            for (out, phi) in v.iter_mut().zip(&self.phi[k * g..(k + 1) * g]) {
                *out += (a * phi) * (a * phi);
            }
        }
        v.iter_mut().for_each(|x| *x = x.sqrt());
        v
    }
}

/// The soundboard the engine builds from `model` (the musician's controls
/// applied, the microphones in metres).
fn built_board(model: &PianoModel) -> Option<SoundboardParams> {
    model.with_macros().soundboard.map(|s| s.resolved())
}

/// Level 0–1 of `x` against `reference` over `range_db` (`db_per`: 10 for
/// energies, 20 for amplitudes).
fn level(x: f32, reference: f32, range_db: f32, db_per: f32) -> f32 {
    if x <= 0.0 || reference <= 0.0 {
        return 0.0;
    }
    (1.0 + db_per * (x / reference).log10() / range_db).clamp(0.0, 1.0)
}

/// A handle of the editor: a point of the plan (plate metres) and the
/// parameters its two coordinates set, with their scale (value = coordinate
/// / scale − offset).
struct Handle {
    name: &'static str,
    at: (f64, f64),
    x: Option<(String, f64, f64)>,
    y: Option<(String, f64, f64)>,
}

fn handles(model: &PianoModel) -> Vec<Handle> {
    let Some(sb) = model.soundboard else { return Vec::new() };
    let mut h = vec![
        Handle { name: "Board length", at: (sb.a, sb.b / 2.0), x: Some(("soundboard.a".into(), 1.0, 0.0)), y: None },
        Handle { name: "Board width", at: (sb.a / 2.0, sb.b), x: None, y: Some(("soundboard.b".into(), 1.0, 0.0)) },
    ];
    let point = |name, path: &str, i: usize, (u, v): (f64, f64)| Handle {
        name,
        at: (u * sb.a, v * sb.b),
        x: Some((format!("soundboard.{path}[{i}][0]"), sb.a, 0.0)),
        y: Some((format!("soundboard.{path}[{i}][1]"), sb.b, 0.0)),
    };
    for (i, &p) in sb.treble_bridge.iter().enumerate() {
        h.push(point("Bridge point", "treble_bridge", i, p));
    }
    for (i, &p) in sb.bass_bridge.iter().enumerate() {
        h.push(point("Bass bridge point", "bass_bridge", i, p));
    }
    if piano_core::soundboard::Outline::new(&sb).is_some() {
        // Repeated vertices (the unused ones) share one handle: the first.
        for (i, &(u, v)) in sb.outline.iter().enumerate() {
            if i > 0 && sb.outline[..i].contains(&(u, v)) {
                continue;
            }
            h.push(Handle {
                name: "Outline point",
                at: (u * sb.a, v * sb.b),
                x: Some((format!("soundboard.outline[{i}][0]"), sb.a, 0.0)),
                y: Some((format!("soundboard.outline[{i}][1]"), sb.b, 0.0)),
            });
        }
    }
    let f = &model.frame;
    if f.enabled {
        h.push(Handle { name: "Agraffes", at: (scene::agraffe_x(model), sb.b * 0.08), x: Some(("frame.agraffe_x".into(), 1.0, f.offset_x)), y: None });
        h.push(Handle { name: "Frame end", at: (f.offset_x + f.a, f.b * 0.75), x: Some(("frame.a".into(), 1.0, f.offset_x)), y: None });
    }
    let (list, field) = if sb.vertical { (sb.mics_upright, "mics_upright") } else { (sb.mics, "mics") };
    for (i, &(x, y, _)) in list.iter().enumerate() {
        let (sa, sbb) = if sb.mics_relative { (sb.a, sb.b) } else { (1.0, 1.0) };
        h.push(Handle {
            name: if i == 0 { "Mic L" } else { "Mic R" },
            at: (x * sa, y * sbb),
            x: Some((format!("soundboard.{field}[{i}][0]"), sa, 0.0)),
            y: Some((format!("soundboard.{field}[{i}][1]"), sbb, 0.0)),
        });
    }
    h
}

impl ModelView {
    /// Lets the 3D view draw on the GPU (the host paints with egui_glow).
    #[cfg(feature = "gpu")]
    pub fn enable_gpu(&mut self) {
        self.gpu.get_or_insert_with(gpu::Gpu::default);
    }

    fn gpu_enabled(&self) -> bool {
        #[cfg(feature = "gpu")]
        return self.gpu.is_some();
        #[cfg(not(feature = "gpu"))]
        false
    }

    /// Whether the 3D view can be shown, and why not.
    fn gpu_state(&self) -> Result<(), String> {
        #[cfg(feature = "gpu")]
        {
            match &self.gpu {
                Some(g) => g.failed().map_or(Ok(()), Err),
                None => Err("The 3D view is not available in this host yet.".into()),
            }
        }
        #[cfg(not(feature = "gpu"))]
        Err("The 3D view is not available in this host yet.".into())
    }

    /// Draws the page; `lit` are the parts of the parameter pointed at.
    /// Returns what its parameter rows did, and a page to open.
    #[allow(clippy::too_many_arguments)]
    pub fn show(
        &mut self,
        ui: &mut Ui,
        control: &mut dyn PianoControl,
        model: &PianoModel,
        registry: &Registry,
        values: &[Option<Value>],
        key: &mut u8,
        lit: &[Part],
    ) -> (Shown, Option<&'static str>) {
        let gpu = self.gpu_state();
        ui.horizontal(|ui| {
            for (m, name, tip) in [
                (Mode::Static, "Static", "The instrument as the parameters build it"),
                (Mode::Live, "Live", "The instrument as it plays: keys, hammers, dampers, strings and soundboard"),
                (Mode::Editor, "Editor", "Drag the board, bridges, agraffes, frame and microphones on the plan"),
            ] {
                ui.selectable_value(&mut self.mode, m, name).on_hover_text(tip);
            }
            ui.separator();
            let three = ui.add_enabled(gpu.is_ok(), egui::Checkbox::new(&mut self.three_d, "3D"));
            match &gpu {
                Ok(()) => three.on_hover_text("Turn around the instrument: drag to turn, right-drag to move, scroll to come closer, double-click to frame it again"),
                Err(e) => three.on_disabled_hover_text(e.as_str()),
            };
            ui.checkbox(&mut self.labels, "Names");
            ui.checkbox(&mut self.lid, "Lid");
            ui.checkbox(&mut self.frame, "Frame");
            ui.checkbox(&mut self.mics, "Mics");
        });
        if let Err(e) = &gpu
            && self.three_d
            && self.gpu_enabled()
        {
            ui.label(RichText::new(format!("{e} The plan is shown instead.")).small().weak());
        }
        ui.add_space(4.0);
        let three_d = self.three_d && gpu.is_ok();

        let obs = if self.mode == Mode::Live { control.observe() } else { None };
        if self.mode == Mode::Live {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(16));
            if obs.is_none() {
                ui.label(RichText::new("This host does not report the engine's state: the instrument is shown at rest.").weak());
            }
        }
        let obs = obs.map(|o| self.smooth(&o));
        let field = obs.as_ref().and_then(|o| self.board_field(model, o));
        let key_levels = obs.as_ref().map(|o| self.key_levels(o));
        let live = obs.as_ref().map(|o| Live { obs: o, keys: key_levels.as_ref().unwrap(), board: field.as_deref(), time: self.start.elapsed().as_secs_f32() / 8.0 });
        let options = Options { key: *key, lit, lid: self.lid, frame: self.frame, mics: self.mics, ink: ui.visuals().text_color() };
        let scene = Arc::new(scene::build(model, live.as_ref(), &options));
        if self.orbit.is_none() || self.framed_vertical != scene.vertical {
            self.orbit = Some(Orbit::framing(scene.body.0, scene.body.1));
            self.framed_vertical = scene.vertical;
        }

        let mut shown = Shown::default();
        let mut open = None;
        let avail = ui.available_width();
        match self.mode {
            Mode::Editor => {
                let plan_w = (avail * 0.58).clamp(300.0, 720.0);
                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(plan_w);
                        outline_buttons(ui, control, model);
                        self.editor_plan(ui, control, model, registry, &scene, plan_w);
                        if three_d {
                            ui.add_space(6.0);
                            open = self.view_3d(ui, &scene, plan_w, plan_w * 0.7).or(open);
                        }
                    });
                    ui.add_space(12.0);
                    ui.vertical(|ui| {
                        shown = geometry_rows(ui, registry, values, control);
                    });
                });
            }
            Mode::Static | Mode::Live => {
                let side = 300.0;
                let w = if avail > 700.0 { avail - side - 16.0 } else { avail };
                let h = (w * 0.68).clamp(320.0, 760.0);
                ui.horizontal_top(|ui| {
                    open = if three_d { self.view_3d(ui, &scene, w, h) } else { self.plan(ui, &scene, w, h) };
                    if avail > 700.0 {
                        ui.add_space(16.0);
                        ui.vertical(|ui| {
                            ui.set_width(side);
                            ui.set_max_width(side);
                            if self.mode == Mode::Live {
                                live_numbers(ui, obs.as_ref(), model);
                            } else {
                                numbers(ui, model);
                            }
                        });
                    }
                });
                if avail <= 700.0 {
                    if self.mode == Mode::Live { live_numbers(ui, obs.as_ref(), model) } else { numbers(ui, model) }
                }
            }
        }
        (shown, open)
    }

    /// The state shown from the state read: energies and board motions
    /// rise at once and fall with [`RELEASE_S`], blows last [`BLOW_S`]; the
    /// references follow the loudest of the last seconds.
    fn smooth(&mut self, obs: &Observation) -> Observation {
        let lv = &mut self.levels;
        let dt = lv.last.map_or(0.0, |t| t.elapsed().as_secs_f32()).min(0.5);
        lv.last = Some(Instant::now());
        let fall = (-dt / RELEASE_S).exp();
        let mut shown = *obs;
        for ((k, prev), hold) in shown.keys.iter_mut().zip(&lv.shown.keys).zip(&mut lv.hammer) {
            if k.energy.is_finite() {
                k.energy = k.energy.max(prev.energy * fall * fall);
            }
            *hold = if k.hammer { BLOW_S } else { (*hold - dt).max(0.0) };
            k.hammer = *hold > 0.0;
        }
        let n = shown.board_modes.min(lv.shown.board_modes);
        for (a, prev) in shown.board[..n].iter_mut().zip(&lv.shown.board[..n]) {
            *a = a.max(prev * fall);
        }
        lv.shown = shown;
        lv.board *= REF_DECAY.powf(dt);
        lv.keys *= REF_DECAY.powf(dt);
        shown
    }

    fn board_field(&mut self, model: &PianoModel, obs: &Observation) -> Option<Vec<f32>> {
        let params = built_board(model)?;
        if self.shapes.as_ref().is_none_or(|s| s.params != params) {
            self.shapes = Some(BoardShapes::new(params));
        }
        let v = self.shapes.as_ref()?.field(obs);
        let peak = v.iter().cloned().fold(0.0, f32::max);
        self.levels.board = self.levels.board.max(peak);
        let r = self.levels.board;
        Some(v.into_iter().map(|x| level(x, r, BOARD_RANGE_DB, 20.0)).collect())
    }

    fn key_levels(&mut self, obs: &Observation) -> [f32; 128] {
        let peak = obs.keys.iter().map(|k| k.energy).filter(|e| e.is_finite()).fold(0.0, f32::max);
        self.levels.keys = self.levels.keys.max(peak);
        let r = self.levels.keys;
        std::array::from_fn(|m| level(obs.keys[m].energy, r, KEY_RANGE_DB, 10.0))
    }

    /// The board's face, orthographic.
    fn plan(&mut self, ui: &mut Ui, scene: &Scene, w: f32, h: f32) -> Option<&'static str> {
        let (rect, _) = ui.allocate_exact_size(Vec2::new(w, h), Sense::hover());
        let m = cam::face_view(scene.min, scene.max, scene.vertical, rect.aspect_ratio());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 4.0, ui.visuals().extreme_bg_color);
        paint::draw(&painter, rect, scene, &m, 1.0);
        self.labels(ui, rect, scene, &m)
    }

    /// The instrument in 3D, the camera turned by the pointer.
    fn view_3d(&mut self, ui: &mut Ui, scene: &Arc<Scene>, w: f32, h: f32) -> Option<&'static str> {
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, h), Sense::click_and_drag());
        let orbit = self.orbit.get_or_insert_with(|| Orbit::framing(scene.body.0, scene.body.1));
        if resp.dragged_by(egui::PointerButton::Primary) {
            let d = resp.drag_delta();
            orbit.yaw -= d.x * 0.008;
            orbit.pitch = (orbit.pitch + d.y * 0.008).clamp(-1.2, 1.55);
        }
        if resp.dragged_by(egui::PointerButton::Secondary) || resp.dragged_by(egui::PointerButton::Middle) {
            let d = resp.drag_delta();
            orbit.pan(d.x, d.y, rect.height());
        }
        if resp.hovered() {
            let scroll = ui.ctx().input_mut(|i| std::mem::take(&mut i.smooth_scroll_delta).y);
            orbit.dist = (orbit.dist * (-scroll * 0.002).exp()).clamp(0.2, 30.0);
        }
        if resp.double_clicked() {
            *orbit = Orbit::framing(scene.body.0, scene.body.1);
        }
        let m = orbit.matrix(rect.aspect_ratio());
        let bg = ui.visuals().extreme_bg_color;
        #[cfg(feature = "gpu")]
        if let Some(g) = &self.gpu {
            g.paint(ui, rect, scene.clone(), m, bg);
        }
        #[cfg(not(feature = "gpu"))]
        {
            let painter = ui.painter_at(rect);
            painter.rect_filled(rect, 4.0, bg);
            paint::draw(&painter, rect, scene, &m, 1.0);
        }
        self.labels(ui, rect, scene, &m)
    }

    /// The parts' names, clickable: their page.
    fn labels(&self, ui: &mut Ui, rect: Rect, scene: &Scene, m: &Mat4) -> Option<&'static str> {
        if !self.labels {
            return None;
        }
        let font = FontId::proportional(11.0);
        let painter = ui.painter_at(rect);
        let vis = ui.visuals();
        let mut open = None;
        for (i, l) in scene.labels.iter().enumerate() {
            let Some((p, _)) = paint::to_screen(m, rect, l.at) else { continue };
            if !rect.shrink(4.0).contains(p) {
                continue;
            }
            let galley = painter.layout_no_wrap(l.text.clone(), font.clone(), vis.text_color());
            let r = Align2::LEFT_BOTTOM.anchor_size(p + Vec2::new(4.0, -4.0), galley.size()).expand(2.0);
            let resp = ui.interact(r, ui.scope_id().with(("model-label", i)), Sense::click());
            let fill = if resp.hovered() { vis.selection.bg_fill } else { vis.extreme_bg_color.gamma_multiply(0.8) };
            painter.rect_filled(r, 3.0, fill);
            painter.circle_filled(p, 2.0, vis.text_color());
            painter.galley(r.min + Vec2::splat(2.0), galley, vis.text_color());
            if resp.on_hover_text(format!("Open the {} page", l.page)).clicked() {
                open = Some(l.page);
            }
        }
        open
    }

    /// The plan with the geometry's handles.
    fn editor_plan(&mut self, ui: &mut Ui, control: &mut dyn PianoControl, model: &PianoModel, registry: &Registry, scene: &Scene, w: f32) {
        let h = w * 0.85;
        let (rect, _) = ui.allocate_exact_size(Vec2::new(w, h), Sense::hover());
        let fitted = cam::face_view(scene.min, scene.max, scene.vertical, rect.aspect_ratio());
        let m = match self.drag {
            Some((_, m, r)) if r.size() == rect.size() => m,
            _ => fitted,
        };
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 4.0, ui.visuals().extreme_bg_color);
        paint::draw(&painter, rect, scene, &m, 1.0);
        let Some(sb) = model.soundboard else {
            painter.text(rect.center(), Align2::CENTER_CENTER, "This model has no soundboard to edit.", FontId::proportional(13.0), ui.visuals().text_color());
            return;
        };
        let place = |x: f64, y: f64| -> V3 { if sb.vertical { [-y as f32, 0.0, (0.2 + sb.a - x) as f32] } else { [-y as f32, x as f32, 0.06] } };
        let screen = |x: f64, y: f64| paint::to_screen(&m, rect, place(x, y)).map_or(rect.center(), |s| s.0);
        // Screen motion of one metre along the plate's x and y.
        let o = screen(0.0, 0.0);
        let (ex, ey) = (screen(1.0, 0.0) - o, screen(0.0, 1.0) - o);
        let det = ex.x * ey.y - ex.y * ey.x;
        let to_plate = |d: Vec2| -> (f64, f64) {
            if det.abs() < 1e-6 {
                return (0.0, 0.0);
            }
            (((d.x * ey.y - d.y * ey.x) / det) as f64, ((ex.x * d.y - ex.y * d.x) / det) as f64)
        };
        let hs = handles(model);
        self.handles_shown = hs.iter().map(|h| (h.name, screen(h.at.0, h.at.1))).collect();
        let vis = ui.visuals().clone();
        let mut hovered_name = None;
        for (i, hd) in hs.iter().enumerate() {
            let p = screen(hd.at.0, hd.at.1);
            let r = Rect::from_center_size(p, Vec2::splat(14.0));
            let resp = ui.interact(r, ui.scope_id().with(("model-handle", i)), Sense::drag());
            let paths = || hd.x.iter().chain(&hd.y).map(|(p, _, _)| p.as_str());
            if resp.drag_started() {
                self.drag = Some((i, m, rect));
                paths().for_each(|p| control.begin_edit(p));
            }
            if resp.dragged() && let Some(pos) = resp.interact_pointer_pos() {
                let (dx, dy) = to_plate(pos - o);
                for ((path, s, off), v) in hd.x.iter().map(|a| (a, dx)).chain(hd.y.iter().map(|a| (a, dy))) {
                    let Some(param) = registry.get(path) else { continue };
                    let value = (v / s - off).clamp(param.meta.min, param.meta.max);
                    control.set(path, Value::Float(value));
                }
            }
            if resp.drag_stopped() {
                self.drag = None;
                paths().for_each(|p| control.end_edit(p));
            }
            let active = resp.hovered() || resp.dragged();
            let fill = if hd.name.starts_with("Mic") { Color32::from_rgb(90, 150, 220) } else { sketches::HIGHLIGHT };
            painter.circle(p, if active { 7.0 } else { 5.0 }, if active { fill } else { fill.gamma_multiply(0.8) }, Stroke::new(1.5, vis.text_color()));
            if active {
                hovered_name = Some((p, hd));
            }
        }
        if let Some((p, hd)) = hovered_name {
            let values: Vec<String> = hd
                .x
                .iter()
                .chain(&hd.y)
                .filter_map(|(path, _, _)| piano_params::get(model, path).map(|v| format!("{} = {}", registry.get(path).map_or(path.as_str(), |p| p.meta.label.as_str()), crate::format_value(v.as_f64()))))
                .collect();
            let text = format!("{}\n{}", hd.name, values.join("\n"));
            painter.text(p + Vec2::new(10.0, -10.0), Align2::LEFT_BOTTOM, text, FontId::proportional(11.0), vis.strong_text_color());
        }
        ui.label(RichText::new("Drag a handle: each move rebuilds the instrument once it pauses.").small().weak());
    }
}

/// The board's outline: a grand's, or the full rectangle. Each sets every
/// vertex in one gesture; the instrument rebuilds once.
fn outline_buttons(ui: &mut Ui, control: &mut dyn PianoControl, model: &PianoModel) {
    let Some(sb) = model.soundboard else { return };
    let rect = [(0.0, 0.0); piano_core::soundboard::OUTLINE_POINTS];
    let outlined = piano_core::soundboard::Outline::new(&sb).is_some();
    ui.horizontal(|ui| {
        ui.label("Board outline:");
        let set = |control: &mut dyn PianoControl, points: &[(f64, f64)]| {
            let paths: Vec<String> = (0..points.len()).flat_map(|i| [format!("soundboard.outline[{i}][0]"), format!("soundboard.outline[{i}][1]")]).collect();
            paths.iter().for_each(|p| control.begin_edit(p));
            for (path, v) in paths.iter().zip(points.iter().flat_map(|&(u, v)| [u, v])) {
                control.set(path, Value::Float(v));
            }
            paths.iter().for_each(|p| control.end_edit(p));
        };
        if ui
            .selectable_label(outlined, "Grand's")
            .on_hover_text("The bentside curving in towards the tail; the board's modes below the outline's corner frequency become the outlined board's own")
            .clicked()
            && !outlined
        {
            set(control, &SoundboardParams::GRAND_OUTLINE);
        }
        if ui.selectable_label(!outlined, "Rectangle").on_hover_text("The full a × b board").clicked() && outlined {
            set(control, &rect);
        }
    });
}

/// The editor's parameter rows (the geometry without handles too).
fn geometry_rows(ui: &mut Ui, registry: &Registry, values: &[Option<Value>], control: &mut dyn PianoControl) -> Shown {
    let mut shown = Shown::default();
    egui::Grid::new("model-geometry").num_columns(5).striped(true).spacing([10.0, 4.0]).show(ui, |ui| {
        for (group, path) in GEOMETRY.iter().flat_map(|(g, paths)| paths.iter().enumerate().map(move |(k, p)| ((k == 0).then_some(*g), p))) {
            if let Some(g) = group {
                ui.label(RichText::new(g).strong());
                ui.end_row();
            }
            let Some(i) = registry.params().iter().position(|p| p.path == *path) else { continue };
            let row = crate::widgets::param_row(ui, &registry.params()[i], values[i], control);
            if row.label.clicked() {
                shown.clicked = Some(i);
            }
            if row.touched {
                shown.touched = Some(i);
            }
            if row.label.hovered() || row.pointing {
                shown.hovered = Some(i);
            }
            ui.end_row();
        }
    });
    shown
}

/// The model in numbers, beside the static view.
fn numbers(ui: &mut Ui, model: &PianoModel) {
    ui.label(RichText::new("In numbers").strong());
    egui::Grid::new("model-numbers").num_columns(2).spacing([10.0, 3.0]).show(ui, |ui| {
        let mut row = |name: &str, value: String| {
            ui.label(RichText::new(name).weak());
            ui.label(value);
            ui.end_row();
        };
        match built_board(model) {
            Some(sb) => {
                row("Soundboard", format!("{:.2} × {:.2} m{}", sb.a, sb.b, if sb.vertical { ", standing" } else { "" }));
                match piano_core::soundboard::Outline::new(&sb) {
                    Some(o) => row("Outline", format!("{:.0} % of the rectangle, own modes under {:.0} Hz", 100.0 * o.area() / (sb.a * sb.b), sb.outline_below)),
                    None => row("Outline", "the full rectangle".into()),
                }
                let board = Soundboard::new(sb);
                let below = |f: f64| board.modes.iter().filter(|m| m.omega / std::f64::consts::TAU < f).count();
                let first = board.modes.iter().find(|m| m.m > 0).map_or(0.0, |m| m.omega / std::f64::consts::TAU);
                row("Board modes", format!("{} under 1 kHz ({} in all)", below(1000.0), board.modes.len()));
                row("First board mode", format!("{first:.0} Hz"));
                row("Bass bridge", format!("A0 to {}", sketches::note_name(sb.main_bridge_from - 1)));
                if sb.lid_angle > 0.0 && !sb.vertical {
                    row("Lid", format!("open {:.0}°", sb.lid_angle));
                }
                for (i, (x, y, z)) in sb.mics.iter().enumerate() {
                    row(if i == 0 { "Mic L" } else { "Mic R" }, format!("{x:.2}, {y:.2}, {z:.2} m"));
                }
            }
            None => row("Soundboard", "none (bridge force only)".into()),
        }
        for m in [21u8, 48, 60, 84, 108] {
            let n = model.note(m);
            row(
                &format!("{} string", sketches::note_name(m)),
                format!("{:.2} m, {:.0} N, {}×", n.length, n.tension, n.strings),
            );
        }
        let f = &model.frame;
        row("Frame", if f.enabled { format!("{:.2} × {:.2} m, {:.0} cm up", f.a, f.b, f.height * 100.0) } else { "off".into() });
        let r = &model.rim;
        row("Rim", if r.enabled { format!("{:.0} cm high, {:.1} cm thick", r.height * 100.0, r.thickness * 100.0) } else { "off".into() });
        row("Dampers", format!("up to {}", sketches::note_name(model.damper.highest_note)));
    });
    ui.add_space(6.0);
    ui.label(
        RichText::new("Board and case follow the model's outline (a rectangle unless one is set); legs, keyboard and pedals only situate the instrument. Click a name to open its page.")
            .small()
            .weak(),
    );
}

/// What the engine does now, beside the live view.
fn live_numbers(ui: &mut Ui, obs: Option<&Observation>, model: &PianoModel) {
    ui.label(RichText::new("Now").strong());
    let Some(o) = obs else {
        ui.label(RichText::new("No engine state.").weak());
        return;
    };
    egui::Grid::new("model-live").num_columns(2).spacing([10.0, 3.0]).show(ui, |ui| {
        let mut row = |name: &str, value: String| {
            ui.label(RichText::new(name).weak());
            ui.label(value);
            ui.end_row();
        };
        let down: Vec<String> = (21..=108u8).filter(|&m| o.keys[m as usize].down).map(sketches::note_name).collect();
        row("Keys down", if down.is_empty() { "none".into() } else { down.join(" ") });
        let sounding = (21..=108usize).filter(|&m| o.keys[m].energy > 0.0).count();
        row("Strings moving", format!("{sounding}"));
        let lifted = (21..=model.damper.highest_note as usize).filter(|&m| o.keys[m].damper < 0.5).count();
        row("Dampers lifted", format!("{lifted}"));
        row("Sustain pedal", format!("{:.0} %", o.sustain * 100.0));
        row("Soft pedal", format!("{:.0} %", o.soft * 100.0));
        row("Sostenuto", if o.sostenuto { "down".into() } else { "up".into() });
    });
    ui.add_space(6.0);
    ui.label(
        RichText::new(
            "Strings: brighter and wider as their energy grows (motion slowed and enlarged). \
             Soundboard: its velocity from its lowest modes, lighter where it moves most, against the loudest of the last seconds.",
        )
        .small()
        .weak(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handles_name_real_parameters() {
        let reg = Registry::new();
        let mut m = PianoModel::default();
        m.soundboard.as_mut().unwrap().outline = SoundboardParams::GRAND_OUTLINE;
        assert_eq!(handles(&m).iter().filter(|h| h.name == "Outline point").count(), 10);
        for h in handles(&m) {
            for (path, _, _) in h.x.iter().chain(&h.y) {
                assert!(reg.get(path).is_some(), "{} sets {path}, which is no parameter", h.name);
            }
        }
        for path in GEOMETRY.iter().flat_map(|g| g.1) {
            assert!(reg.get(path).is_some(), "{path}");
        }
    }

    #[test]
    fn board_field_follows_the_modes() {
        let p = built_board(&PianoModel::default()).unwrap();
        let s = BoardShapes::new(p);
        let mut o = Observation::default();
        assert!(s.field(&o).iter().all(|&v| v == 0.0));
        o.board[0] = 1.0;
        o.board_modes = 1;
        let f = s.field(&o);
        assert!(f.iter().any(|&v| v > 0.0) && f.iter().all(|v| v.is_finite()));
    }

    #[test]
    fn the_meters_fall_slowly_and_blows_stay_seen() {
        let mut v = ModelView::default();
        let mut o = Observation::default();
        o.keys[60].energy = 1.0;
        o.keys[60].hammer = true;
        o.board[0] = 1.0;
        o.board_modes = 1;
        v.smooth(&o);
        // The next look catches a dip, and the blow is over.
        std::thread::sleep(std::time::Duration::from_millis(20));
        let dip = Observation { board_modes: 1, ..Observation::default() };
        let s = v.smooth(&dip);
        assert!(s.board[0] > 0.8 && s.keys[60].energy > 0.7, "{} {}", s.board[0], s.keys[60].energy);
        assert!(s.keys[60].hammer);
    }

    #[test]
    fn levels_span_their_range() {
        assert_eq!(level(1.0, 1.0, 40.0, 20.0), 1.0);
        assert!((level(0.1, 1.0, 40.0, 20.0) - 0.5).abs() < 1e-6);
        assert_eq!(level(0.0, 1.0, 40.0, 20.0), 0.0);
    }
}
