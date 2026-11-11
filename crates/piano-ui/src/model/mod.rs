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
//! `gpu`): the standalone player does; the plugin not yet. Its realistic
//! look (materials, a studio's lights, shadows and reflections) is an
//! option, heavier on the GPU.

mod cam;
#[cfg(feature = "gpu")]
mod gpu;
#[cfg(feature = "gpu")]
mod real;
mod paint;
#[cfg(test)]
mod raster;
pub mod scene;

use std::sync::Arc;
use std::time::Instant;

use egui::{Align2, Color32, FontId, Rect, RichText, Sense, Stroke, Ui, Vec2};
use piano_core::observe::OBSERVED_MODES;
use piano_core::params::PianoModel;
use piano_core::soundboard::{Soundboard, SoundboardParams, KEY_LINE_X};
use piano_core::Observation;
use piano_params::{PianoControl, Registry, Value};

use crate::simple::Shown;
use crate::sketches::{self, Part};
use cam::{Mat4, Orbit, V3};
use scene::{Layer, Live, Options, Scene, BOARD_GRID};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Static,
    Live,
    Editor,
}

/// The geometry the editor lists beside the plan.
const GEOMETRY: &[(&str, &[&str])] = &[
    ("Soundboard", &["soundboard.a", "soundboard.b", "soundboard.outline_below", "soundboard.main_bridge_from", "soundboard.main_string_slant"]),
    ("Frame", &["frame.offset_x", "frame.agraffe_x", "frame.a", "frame.b", "frame.height"]),
    ("Rim", &["rim.thickness", "rim.height", "rim.top"]),
    ("Lid", &["soundboard.lid_angle", "soundboard.lid_removed", "soundboard.lid_hinge_height"]),
    ("Along the strings", &["strike_ratio", "damper.position", "damper.highest_note"]),
];

pub struct ModelView {
    pub mode: Mode,
    three_d: bool,
    /// The 3D view in its realistic look.
    pub(crate) realistic: bool,
    orbit: Option<Orbit>,
    /// Upright or grand the orbit was framed for.
    framed_vertical: bool,
    lid: bool,
    case: bool,
    /// A grand's fallboard shut over the keys.
    pub(crate) fallboard: bool,
    /// How see-through a grand's front above the fallboard and its cover
    /// over the pin block are (0 solid … 1 left out).
    pub(crate) front_clear: f32,
    frame: bool,
    mics: bool,
    labels: bool,
    #[cfg(feature = "gpu")]
    gpu: Option<gpu::Gpu>,
    levels: Levels,
    shapes: Option<BoardShapes>,
    /// The board's modes as the numbers count them, for its parameters
    /// (building them takes longer than a frame).
    board_counts: Option<(SoundboardParams, BoardCounts)>,
    /// The handle being dragged, and the plan's matrix and rect when the
    /// drag began (the plan refits to the edited instrument only after).
    drag: Option<(usize, Mat4, Rect)>,
    /// The editor's handles as last drawn: name and place on the screen.
    pub(crate) handles_shown: Vec<(&'static str, egui::Pos2)>,
    /// What goes beside the static or live view, or under it, as last
    /// drawn.
    pub(crate) side_shown: Option<Rect>,
    /// A press on the live 3D view's keys under way (it plays them rather
    /// than turning the view), and the key it holds down.
    keying: bool,
    playing: Option<u8>,
    /// The layers ([`Layer::PARTS`]) last built, and what from.
    built: [Option<(Inputs, Arc<Scene>)>; 3],
    start: Instant,
}

/// What a layer of the scene was built from: built again only when it
/// changes.
#[derive(PartialEq)]
struct Inputs {
    model: PianoModel,
    key: u8,
    lit: Vec<Part>,
    show: [bool; 5],
    front: f32,
    ink: egui::Color32,
    /// The engine's state as the layer shows it, to the colours'
    /// resolution, and the strings' clock while they move.
    state: Vec<u8>,
    time: f32,
}

impl Inputs {
    fn new(model: &PianoModel, o: &Options, live: Option<&Live>, layer: Layer) -> Self {
        let mut state = Vec::new();
        let mut time = 0.0;
        let q = |x: f32, steps: f32| (x.clamp(0.0, 1.0) * steps).round() as u8;
        match (live, layer) {
            // The keys and hammers glow faintly with their strings' level:
            // a few steps of it.
            (Some(l), Layer::Played) => {
                for m in 21..=108 {
                    let k = &l.obs.keys[m];
                    state.extend([k.down as u8 | (k.hammer as u8) << 1, q(k.damper, 255.0), q(l.keys[m], 12.0)]);
                }
                state.extend([q(l.obs.sustain, 255.0), q(l.obs.soft, 255.0), l.obs.sostenuto as u8]);
            }
            (Some(l), Layer::Sounding) => {
                state.extend(l.keys[21..=108].iter().map(|&k| q(k, 255.0)));
                state.extend(l.board.into_iter().flatten().map(|&x| q(x, 255.0)));
                // A string barely sounding is drawn straight.
                if l.keys.iter().any(|&k| k > 0.02) {
                    time = l.time;
                }
            }
            _ => {}
        }
        Self { model: *model, key: o.key, lit: o.lit.to_vec(), show: [o.lid, o.frame, o.mics, o.case, o.fallboard], front: o.front, ink: o.ink, state, time }
    }
}

/// The scene as drawn, in its layers ([`Layer::PARTS`]).
#[derive(Clone)]
pub(crate) struct Layers(pub [Arc<Scene>; 3]);

impl Layers {
    fn still(&self) -> &Scene {
        &self.0[0]
    }

    /// The instrument's extent.
    fn body(&self) -> (V3, V3) {
        self.0.iter().filter(|s| !s.tris.is_empty()).map(|s| s.body).reduce(|a, b| (std::array::from_fn(|i| a.0[i].min(b.0[i])), std::array::from_fn(|i| a.1[i].max(b.1[i])))).unwrap_or(self.still().body)
    }

    /// All as one, for the painter's drawing (which sorts all of it).
    fn merged(&self) -> Scene {
        Scene::merged(&self.0.each_ref().map(|s| &**s))
    }

    fn key_at(&self, from: V3, dir: V3) -> Option<(u8, f32)> {
        self.0.iter().find_map(|s| s.key_at(from, dir))
    }
}

impl Default for ModelView {
    fn default() -> Self {
        Self {
            mode: Mode::Live,
            three_d: true,
            realistic: true,
            orbit: None,
            framed_vertical: false,
            lid: true,
            case: true,
            fallboard: false,
            front_clear: 0.0,
            frame: true,
            mics: true,
            labels: true,
            #[cfg(feature = "gpu")]
            gpu: None,
            levels: Levels::default(),
            shapes: None,
            board_counts: None,
            drag: None,
            handles_shown: Vec::new(),
            side_shown: None,
            keying: false,
            playing: None,
            built: [None, None, None],
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

/// How hard a key pressed in the live view is struck: as at its front,
/// down to as at the back of its visible part.
const FRONT_VELOCITY: f32 = 127.0;
const BACK_VELOCITY: f32 = 20.0;

/// The velocity of a press at `along` a key (0 front … 1 back).
fn click_velocity(along: f32) -> u8 {
    (FRONT_VELOCITY + (BACK_VELOCITY - FRONT_VELOCITY) * along.clamp(0.0, 1.0)).round() as u8
}

/// Decay of the references (per second), and the ranges shown (dB).
const REF_DECAY: f32 = 0.8;
const KEY_RANGE_DB: f32 = 50.0;
const BOARD_RANGE_DB: f32 = 40.0;
/// The lowest the keys' reference comes down to: the energy below which
/// the engine frees a voice at the bottom of the range, so that after a
/// long rest what is left in the strings stays dark (a soft note lies a
/// little above).
fn key_floor() -> f32 {
    piano_core::engine::Shortcuts::DEFAULT.voice_silence * 10f32.powf(KEY_RANGE_DB / 10.0)
}
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
    model.with_macros().board_params().map(|s| s.resolved())
}

/// Level 0–1 of `x` against `reference` over `range_db` (`db_per`: 10 for
/// energies, 20 for amplitudes).
fn level(x: f32, reference: f32, range_db: f32, db_per: f32) -> f32 {
    if x <= 0.0 || reference <= 0.0 {
        return 0.0;
    }
    (1.0 + db_per * (x / reference).log10() / range_db).clamp(0.0, 1.0)
}

/// Where the names go, each given its point on the screen and its size:
/// beside its point, on the first side (up right, down right, up left, down
/// left) where it covers no other name or point, further up or down its
/// side if none is free. Always in the same order, so that a still view's
/// names stay put.
fn place_labels(items: &[(egui::Pos2, Vec2)], bounds: Rect) -> Vec<Rect> {
    /// From the point to the name, and rows tried up or down its side.
    const GAP: f32 = 2.0;
    const ROWS: usize = 12;
    let mut placed: Vec<Rect> = Vec::with_capacity(items.len());
    for (i, &(p, size)) in items.iter().enumerate() {
        let candidates: Vec<Rect> = (0..ROWS)
            .flat_map(|k| {
                let d = GAP + k as f32 * (size.y + 1.0);
                [
                    (Align2::LEFT_BOTTOM, Vec2::new(GAP, -d)),
                    (Align2::LEFT_TOP, Vec2::new(GAP, d)),
                    (Align2::RIGHT_BOTTOM, Vec2::new(-GAP, -d)),
                    (Align2::RIGHT_TOP, Vec2::new(-GAP, d)),
                ]
            })
            .map(|(align, by)| align.anchor_size(p + by, size))
            .collect();
        let clear_of_names = |r: &Rect| bounds.contains_rect(*r) && !placed.iter().any(|o| o.expand(1.0).intersects(*r));
        let clear_of_points = |r: &Rect| !items.iter().enumerate().any(|(j, o)| j != i && r.expand(2.0).contains(o.0));
        let place = candidates
            .iter()
            .find(|r| clear_of_names(r) && clear_of_points(r))
            .or_else(|| candidates.iter().find(|r| clear_of_names(r)))
            .copied()
            // No room at all: in the view at least.
            .unwrap_or_else(|| {
                let r = candidates[0];
                let over = |lo: f32, hi: f32, min: f32, max: f32| (min - lo).max(0.0) + (max - hi).min(0.0);
                r.translate(Vec2::new(over(r.left(), r.right(), bounds.left(), bounds.right()), over(r.top(), r.bottom(), bounds.top(), bounds.bottom())))
            });
        placed.push(place);
    }
    placed
}

/// A handle of the editor: a point of the plan (plate metres) and the
/// parameters its two coordinates set, with their scale (value = coordinate
/// / scale − offset). A main bridge point is drawn where its slanting
/// string lands ([`SoundboardParams::main_string_slant`]): its coordinates
/// are turned back by `slant` (rad) about the keys' line before they set
/// the parameters.
struct Handle {
    name: &'static str,
    at: (f64, f64),
    x: Option<(String, f64, f64)>,
    y: Option<(String, f64, f64)>,
    slant: f64,
}

impl Handle {
    /// A plan point (m) turned by the handle's slant about the keys' line,
    /// forwards (as drawn) or back (as the parameters have it).
    fn turn(&self, (x, y): (f64, f64), back: bool) -> (f64, f64) {
        let (s, c) = self.slant.sin_cos();
        if back {
            let x = KEY_LINE_X + (x - KEY_LINE_X) / c;
            (x, y - (x - KEY_LINE_X) * s)
        } else {
            (KEY_LINE_X + (x - KEY_LINE_X) * c, y + (x - KEY_LINE_X) * s)
        }
    }
}

fn handles(model: &PianoModel) -> Vec<Handle> {
    let Some(sb) = model.soundboard else { return Vec::new() };
    let mut h = vec![
        Handle { name: "Board length", at: (sb.a, sb.b / 2.0), x: Some(("soundboard.a".into(), 1.0, 0.0)), y: None, slant: 0.0 },
        Handle { name: "Board width", at: (sb.a / 2.0, sb.b), x: None, y: Some(("soundboard.b".into(), 1.0, 0.0)), slant: 0.0 },
    ];
    let point = |name, path: &str, i: usize, (u, v): (f64, f64), slant: f64| {
        let mut h = Handle {
            name,
            at: (u * sb.a, v * sb.b),
            x: Some((format!("soundboard.{path}[{i}][0]"), sb.a, 0.0)),
            y: Some((format!("soundboard.{path}[{i}][1]"), sb.b, 0.0)),
            slant,
        };
        h.at = h.turn(h.at, false);
        h
    };
    let lo = sb.main_bridge_from as f64;
    for (i, &p) in sb.treble_bridge.iter().enumerate() {
        h.push(point("Bridge point", "treble_bridge", i, p, sb.string_slant(108.0 - (108.0 - lo) * i as f64 / 4.0)));
    }
    for (i, &p) in sb.bass_bridge.iter().enumerate() {
        h.push(point("Bass bridge point", "bass_bridge", i, p, 0.0));
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
                slant: 0.0,
            });
        }
    }
    let f = &model.frame;
    if f.enabled {
        h.push(Handle { name: "Agraffes", at: (scene::agraffe_x(model), sb.b * 0.08), x: Some(("frame.agraffe_x".into(), 1.0, f.offset_x)), y: None, slant: 0.0 });
        h.push(Handle { name: "Frame end", at: (f.offset_x + f.a, f.b * 0.75), x: Some(("frame.a".into(), 1.0, f.offset_x)), y: None, slant: 0.0 });
    }
    let (list, field) = if sb.vertical { (sb.mics_upright, "mics_upright") } else { (sb.mics, "mics") };
    for (i, &(x, y, _)) in list.iter().enumerate() {
        let (sa, sbb) = if sb.mics_relative { (sb.a, sb.b) } else { (1.0, 1.0) };
        h.push(Handle {
            name: if i == 0 { "Mic L" } else { "Mic R" },
            at: (x * sa, y * sbb),
            x: Some((format!("soundboard.{field}[{i}][0]"), sa, 0.0)),
            y: Some((format!("soundboard.{field}[{i}][1]"), sbb, 0.0)),
            slant: 0.0,
        });
    }
    h
}

impl ModelView {
    /// Sets the 3D view's camera: yaw, pitch, distance and the point
    /// looked at ([`Orbit`]).
    pub(crate) fn set_orbit(&mut self, v: &[f32]) {
        if let &[yaw, pitch, dist, x, y, z] = v {
            self.orbit = Some(Orbit { yaw, pitch, dist, target: [x, y, z] });
        }
    }

    /// Leaves out the names, lid, case, frame or mics.
    pub(crate) fn hide(&mut self, part: &str) {
        match part {
            "names" => self.labels = false,
            "lid" => self.lid = false,
            "case" => self.case = false,
            "frame" => self.frame = false,
            "mics" => self.mics = false,
            _ => {}
        }
    }

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

    /// Why the realistic look does not work here, once it was tried.
    fn real_failed(&self) -> Option<String> {
        #[cfg(feature = "gpu")]
        return self.gpu.as_ref().and_then(|g| g.real_failed());
        #[cfg(not(feature = "gpu"))]
        None
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
        // Before the options, whose row can run wider than the page.
        let avail = ui.available_width();
        ui.horizontal_wrapped(|ui| {
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
                Ok(()) => three.on_hover_text(
                    "Turn around the instrument: drag to turn, right-drag to move, scroll to come closer, double-click to frame it again. Live, press the keys to play them",
                ),
                Err(e) => three.on_disabled_hover_text(e.as_str()),
            };
            ui.add_enabled(gpu.is_ok() && self.three_d, egui::Checkbox::new(&mut self.realistic, "Realistic")).on_hover_text(
                "The 3D view as a photograph: each part's material, a studio's lights, soft shadows, reflections, a floor. Heavier on the GPU",
            );
            ui.checkbox(&mut self.labels, "Names");
            ui.checkbox(&mut self.lid, "Lid");
            ui.checkbox(&mut self.case, "Case");
            ui.add_enabled(self.case, egui::Checkbox::new(&mut self.fallboard, "Fallboard"))
                .on_hover_text("A grand's fallboard shut over the keys, the name on it")
                .on_disabled_hover_text("The fallboard is the case's: tick Case");
            // Wrapped by hand: a slider and its text do not wrap by themselves.
            if ui.available_size_before_wrap().x < 270.0 {
                ui.end_row();
            }
            ui.add_enabled(
                self.case,
                egui::Slider::new(&mut self.front_clear, 0.0..=1.0).text("Front see-through").custom_formatter(|v, _| format!("{:.0} %", v * 100.0)),
            )
            .on_hover_text("A grand's front above the fallboard and its cover over the tuning pins: solid, see-through, or left out");
            ui.checkbox(&mut self.frame, "Frame");
            ui.checkbox(&mut self.mics, "Mics");
        });
        if let Err(e) = &gpu
            && self.three_d
            && self.gpu_enabled()
        {
            ui.label(RichText::new(format!("{e} The plan is shown instead.")).small().weak());
        }
        if let Some(e) = self.real_failed()
            && self.three_d
            && self.realistic
        {
            ui.label(RichText::new(format!("{e} The plain look is shown instead.")).small().weak());
        }
        ui.add_space(4.0);
        let three_d = self.three_d && gpu.is_ok();
        if self.mode != Mode::Live || !three_d {
            self.release(control);
        }

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
        let options = Options { key: *key, lit, lid: self.lid, frame: self.frame, mics: self.mics, case: self.case, fallboard: self.fallboard, front: 1.0 - self.front_clear, ink: ui.visuals().text_color() };
        let layers = self.layers(model, live.as_ref(), &options);
        // Framed again for the other kind of piano (by the view, which
        // knows its shape).
        if self.framed_vertical != layers.still().vertical {
            self.orbit = None;
            self.framed_vertical = layers.still().vertical;
        }

        let mut shown = Shown::default();
        let mut open = None;
        match self.mode {
            Mode::Editor => {
                // What the rows beside it need, as far as it can.
                let plan_w = (avail - 440.0).clamp(280.0, 720.0);
                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(plan_w);
                        outline_buttons(ui, control, model);
                        self.editor_plan(ui, control, model, registry, &layers.merged(), plan_w);
                        if three_d {
                            ui.add_space(6.0);
                            open = self.view_3d(ui, &layers, plan_w, plan_w * 0.7, None).or(open);
                        }
                    });
                    ui.add_space(12.0);
                    ui.vertical(|ui| {
                        shown = geometry_rows(ui, registry, values, control);
                    });
                });
            }
            Mode::Static | Mode::Live => {
                // Live: the musician's controls beside the instrument
                // playing, what it does now under them.
                let live = self.mode == Mode::Live;
                let side = if live { 340.0 } else { 300.0 };
                // Down to a narrow view live: the help panel beside it must
                // not send the controls away under it.
                let beside = avail > side + if live { 240.0 } else { 400.0 };
                // The gap, and the spacing on both sides of it.
                let gap = 16.0 + 2.0 * ui.spacing().item_spacing.x;
                let w = if beside { avail - side - gap } else { avail };
                // Down to the bottom of the page as it shows (what lies
                // above it measured from the page's top, not the screen's,
                // so that scrolling does not grow it).
                let above = ui.cursor().top() - ui.max_rect().top();
                let h = (ui.clip_rect().height() - above - 8.0).max(320.0);
                ui.horizontal_top(|ui| {
                    let keys = (self.mode == Mode::Live).then_some(&mut *control);
                    open = if three_d { self.view_3d(ui, &layers, w, h, keys) } else { self.plan(ui, &layers.merged(), w, h) };
                    if beside {
                        ui.add_space(16.0);
                        let column = ui.vertical(|ui| {
                            ui.set_width(side);
                            ui.set_max_width(side);
                            shown = self.side(ui, control, model, registry, values, obs.as_ref());
                        });
                        self.side_shown = Some(column.response.rect);
                    }
                });
                if !beside {
                    let under = ui.vertical(|ui| {
                        shown = self.side(ui, control, model, registry, values, obs.as_ref());
                    });
                    self.side_shown = Some(under.response.rect);
                }
            }
        }
        (shown, open)
    }

    /// What goes beside the static or live view or under it: the numbers,
    /// or live the musician's controls (compact either way: under the view
    /// the page is narrow) and the engine's state.
    #[allow(clippy::too_many_arguments)]
    fn side(
        &mut self,
        ui: &mut Ui,
        control: &mut dyn PianoControl,
        model: &PianoModel,
        registry: &Registry,
        values: &[Option<Value>],
        obs: Option<&Observation>,
    ) -> Shown {
        if self.mode != Mode::Live {
            numbers(ui, model, &mut self.board_counts);
            return Shown::default();
        }
        ui.label(RichText::new("Simple").strong());
        let shown = crate::simple::show(ui, registry, values, control, true);
        ui.add_space(10.0);
        ui.separator();
        live_numbers(ui, obs, model);
        shown
    }

    /// The scene's layers, each built again only when what it shows
    /// changed: the still one when the instrument or the options do, the
    /// others when the engine's state does too, as they draw it.
    fn layers(&mut self, model: &PianoModel, live: Option<&Live>, o: &Options) -> Layers {
        Layers(std::array::from_fn(|i| {
            let layer = Layer::PARTS[i];
            let live = live.filter(|_| layer != Layer::Still);
            let inputs = Inputs::new(model, o, live, layer);
            match &self.built[i] {
                Some((kept, s)) if *kept == inputs => s.clone(),
                _ => {
                    let s = Arc::new(scene::build_layer(model, live, o, layer));
                    self.built[i] = Some((inputs, s.clone()));
                    s
                }
            }
        }))
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
        let r = self.levels.keys.max(key_floor());
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

    /// Lets go of the key the live view holds down.
    fn release(&mut self, control: &mut dyn PianoControl) {
        if let Some(n) = self.playing.take() {
            control.play(n, 0);
        }
        self.keying = false;
    }

    /// Holds down `key` (note, place along it), letting go of the one held
    /// before; None lets go.
    fn hold(&mut self, control: &mut dyn PianoControl, key: Option<(u8, f32)>) {
        if key.map(|k| k.0) == self.playing {
            return;
        }
        if let Some(n) = self.playing.take() {
            control.play(n, 0);
        }
        if let Some((n, along)) = key {
            control.play(n, click_velocity(along));
            self.playing = Some(n);
        }
    }

    /// The instrument in 3D, the camera turned by the pointer; with a
    /// `control` (live), a press on the keys plays them, louder nearer
    /// their front, the key under the pointer held down as it moves along
    /// them.
    fn view_3d(&mut self, ui: &mut Ui, layers: &Layers, w: f32, h: f32, control: Option<&mut (dyn PianoControl + '_)>) -> Option<&'static str> {
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, h), Sense::click_and_drag());
        let body = layers.body();
        let cam = *self.orbit.get_or_insert_with(|| Orbit::framing(body.0, body.1, rect.aspect_ratio()));
        let mut on_key = false;
        if let Some(control) = control {
            let key = |p: egui::Pos2| {
                rect.contains(p).then(|| {
                    let (x, y) = ((p.x - rect.left()) / rect.width() * 2.0 - 1.0, 1.0 - (p.y - rect.top()) / rect.height() * 2.0);
                    let (from, dir) = cam.ray(rect.aspect_ratio(), x, y);
                    layers.key_at(from, dir)
                })?
            };
            // The primary button's presses and releases in their order (a
            // whole click may come within one frame), and whether the
            // window lost the focus (the release may then go elsewhere).
            let (buttons, blurred, pos, down) = ui.input(|i| {
                let buttons: Vec<(egui::Pos2, bool)> = i
                    .events
                    .iter()
                    .filter_map(|e| match e {
                        egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, .. } => Some((*pos, *pressed)),
                        _ => None,
                    })
                    .collect();
                let blurred = i.events.iter().any(|e| matches!(e, egui::Event::WindowFocused(false)));
                (buttons, blurred, i.pointer.latest_pos(), i.pointer.primary_down())
            });
            if blurred {
                self.release(control);
            } else {
                for (p, pressed) in buttons {
                    if !pressed {
                        self.keying = false;
                        self.hold(control, None);
                    } else if let Some(k) = key(p).filter(|_| resp.contains_pointer()) {
                        self.keying = true;
                        self.hold(control, Some(k));
                    }
                }
                self.keying &= down;
                let held = pos.filter(|_| self.keying).and_then(key);
                self.hold(control, held);
            }
            let under = pos.and_then(key);
            on_key = under.is_some();
            if self.keying || (on_key && resp.hovered() && !resp.dragged()) {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
        }
        let orbit = self.orbit.as_mut().expect("framed above");
        if resp.dragged_by(egui::PointerButton::Primary) && !self.keying {
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
        if resp.double_clicked() && !on_key {
            *orbit = Orbit::framing(body.0, body.1, rect.aspect_ratio());
        }
        let m = orbit.matrix(rect.aspect_ratio());
        let bg = ui.visuals().extreme_bg_color;
        #[cfg(feature = "gpu")]
        if let Some(g) = &self.gpu {
            let real = self.realistic.then(|| orbit.camera(rect.aspect_ratio()));
            g.paint(ui, rect, layers.0.clone(), m, bg, real);
        }
        #[cfg(not(feature = "gpu"))]
        {
            let painter = ui.painter_at(rect);
            painter.rect_filled(rect, 4.0, bg);
            paint::draw(&painter, rect, &layers.merged(), &m, 1.0);
        }
        self.labels(ui, rect, layers.still(), &m)
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
        let shown: Vec<_> = scene
            .labels
            .iter()
            .enumerate()
            .filter_map(|(i, l)| {
                let (p, _) = paint::to_screen(m, rect, l.at)?;
                rect.shrink(4.0).contains(p).then(|| (i, l, p, painter.layout_no_wrap(l.text.clone(), font.clone(), vis.text_color())))
            })
            .collect();
        let items: Vec<_> = shown.iter().map(|(_, _, p, galley)| (*p, galley.size() + Vec2::splat(4.0))).collect();
        let places = place_labels(&items, rect.shrink(2.0));
        // The leaders under every name.
        for (&(p, _), r) in items.iter().zip(&places) {
            if r.distance_to_pos(p) > 4.0 {
                painter.line_segment([p, r.clamp(p)], Stroke::new(1.0, vis.weak_text_color()));
            }
        }
        for ((i, l, p, galley), r) in shown.into_iter().zip(places) {
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
        let place = |x: f64, y: f64| -> V3 { if sb.vertical { [-y as f32, 0.0, (scene::UPRIGHT_BASE + sb.a - x) as f32] } else { [-y as f32, x as f32, 0.06] } };
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
                let (dx, dy) = hd.turn(to_plate(pos - o), true);
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

/// The board's modes: under 1 kHz, in all, and the first one's frequency.
#[derive(Clone, Copy)]
struct BoardCounts {
    below_1k: usize,
    all: usize,
    first: f64,
}

impl BoardCounts {
    fn new(sb: SoundboardParams) -> Self {
        let board = Soundboard::new(sb);
        let hz = |m: &piano_core::soundboard::PlateMode| m.omega / std::f64::consts::TAU;
        Self {
            below_1k: board.modes.iter().filter(|m| hz(m) < 1000.0).count(),
            all: board.modes.len(),
            first: board.modes.iter().find(|m| m.m > 0).map_or(0.0, hz),
        }
    }
}

/// The model in numbers, beside the static view.
fn numbers(ui: &mut Ui, model: &PianoModel, counts: &mut Option<(SoundboardParams, BoardCounts)>) {
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
                if counts.as_ref().is_none_or(|c| c.0 != sb) {
                    *counts = Some((sb, BoardCounts::new(sb)));
                }
                let c = counts.as_ref().map(|c| c.1).unwrap();
                row("Board modes", format!("{} under 1 kHz ({} in all)", c.below_1k, c.all));
                row("First board mode", format!("{:.0} Hz", c.first));
                row("Bass bridge", format!("A0 to {}", sketches::note_name(sb.main_bridge_from - 1)));
                if !sb.vertical {
                    let lid = if sb.lid_removed {
                        "removed".into()
                    } else if sb.lid_angle <= 0.0 {
                        "shut".into()
                    } else {
                        format!("open {:.0}°", sb.lid_angle)
                    };
                    row("Lid", lid);
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
        RichText::new("Board and case follow the model's outline (a rectangle unless one is set); legs, keyboard, a grand's action and pedals only situate the instrument (untick Case to see the action). Click a name to open its page.")
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

    fn view_layers(model: &PianoModel, o: &Options) -> Layers {
        Layers(Layer::PARTS.map(|l| Arc::new(scene::build_layer(model, None, o, l))))
    }

    /// Records the notes played.
    struct Keys(Vec<(u8, u8)>);

    impl PianoControl for Keys {
        fn model(&self) -> PianoModel {
            PianoModel::default()
        }
        fn begin_edit(&mut self, _: &str) {}
        fn set(&mut self, _: &str, _: Value) {}
        fn end_edit(&mut self, _: &str) {}
        fn load(&mut self, _: PianoModel) {}
        fn status(&self) -> piano_params::Status {
            Default::default()
        }
        fn play(&mut self, note: u8, velocity: u8) {
            self.0.push((note, velocity));
        }
    }

    #[test]
    fn layers_are_built_again_only_when_what_they_show_changes() {
        let model = PianoModel::default();
        let o = Options { key: 60, lit: &[], lid: true, frame: true, mics: true, case: true, fallboard: false, front: 1.0, ink: egui::Color32::WHITE };
        let mut view = ModelView::default();
        let mut obs = Observation::default();
        let board = vec![0.0f32; BOARD_GRID.0 * BOARD_GRID.1];
        let live = |view: &mut ModelView, obs: &Observation, levels: &[f32; 128], time: f32| -> [Arc<Scene>; 3] {
            let l = Live { obs, keys: levels, board: Some(&board), time };
            view.layers(&model, Some(&l), &o).0
        };
        let same = |a: &[Arc<Scene>; 3], b: &[Arc<Scene>; 3]| [0, 1, 2].map(|i| Arc::ptr_eq(&a[i], &b[i]));
        let mut levels = [0.0f32; 128];
        let a = live(&mut view, &obs, &levels, 0.0);
        // At rest, the clock and a fading energy change nothing drawn.
        obs.keys[60].energy = 1e-9;
        let b = live(&mut view, &obs, &levels, 0.5);
        assert_eq!(same(&a, &b), [true; 3]);
        // A key down: what is played only.
        obs.keys[60].down = true;
        let c = live(&mut view, &obs, &levels, 0.5);
        assert_eq!(same(&b, &c), [true, false, true]);
        // Its strings sounding, on in time: what sounds only.
        levels[60] = 0.5;
        let d = live(&mut view, &obs, &levels, 0.5);
        let e = live(&mut view, &obs, &levels, 0.51);
        assert_eq!(same(&d, &e), [true, true, false]);
        // Another option: all.
        let f = view.layers(&model, None, &Options { lid: false, ..o }).0;
        assert_eq!(same(&e, &f), [false; 3]);
    }

    #[test]
    fn pressing_the_keys_plays_them_and_elsewhere_turns_the_view() {
        use egui::{Event, PointerButton, Pos2, RawInput};
        let model = PianoModel::default();
        let options = Options { key: 60, lit: &[], lid: true, frame: true, mics: true, case: true, fallboard: false, front: 1.0, ink: egui::Color32::WHITE };
        let layers = view_layers(&model, &options);
        let scene = layers.0[1].clone();
        let ctx = egui::Context::default();
        let mut view = ModelView::default();
        let mut keys = Keys(Vec::new());
        let size = Vec2::new(800.0, 600.0);
        let mut rect = Rect::NOTHING;
        let mut frame = |view: &mut ModelView, keys: &mut Keys, events: Vec<Event>| {
            let time = Some(ctx.input(|i| i.time) + 1.0 / 60.0);
            let input = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 800.0))), events, time, ..Default::default() };
            ctx.run_ui(input, |ui| {
                rect = Rect::from_min_size(ui.cursor().min, size);
                view.view_3d(ui, &layers, size.x, size.y, Some(keys));
            })
            .drop_without_applying_deltas();
            rect
        };
        let rect = frame(&mut view, &mut keys, vec![]);
        let orbit = view.orbit.unwrap();
        let m = orbit.matrix(rect.aspect_ratio());
        // The middle of a white key's top, in front.
        let on = |note: u8| {
            let (_, lo, hi) = *scene.keys.iter().find(|k| k.0 == note).unwrap();
            paint::to_screen(&m, rect, [(lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0, hi[2]]).unwrap().0
        };
        let button = |pos, pressed| Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Default::default() };
        let (c4, d4) = (on(60), on(62));
        frame(&mut view, &mut keys, vec![Event::PointerMoved(c4)]);
        frame(&mut view, &mut keys, vec![button(c4, true)]);
        // Near its front: loud.
        let v = keys.0[0].1;
        assert!(keys.0[0].0 == 60 && v > 100, "{:?}", keys.0);
        // Along the keys: the next one, the view kept still.
        for k in 1..=4 {
            frame(&mut view, &mut keys, vec![Event::PointerMoved(c4 + (d4 - c4) * k as f32 / 4.0)]);
        }
        frame(&mut view, &mut keys, vec![button(d4, false)]);
        assert_eq!(keys.0.iter().map(|k| k.0).collect::<Vec<_>>(), [60, 60, 62, 62]);
        assert!(keys.0[1].1 == 0 && keys.0[3].1 == 0 && keys.0[2].1.abs_diff(v) <= 3, "{:?}", keys.0);
        assert_eq!(view.orbit, Some(orbit));
        // At the back of its visible part: soft.
        keys.0.clear();
        let (_, lo, hi) = *scene.keys.iter().rfind(|k| k.0 == 60).unwrap();
        let back = paint::to_screen(&m, rect, [(lo[0] + hi[0]) / 2.0, hi[1] - 0.005, hi[2]]).unwrap().0;
        frame(&mut view, &mut keys, vec![Event::PointerMoved(back)]);
        frame(&mut view, &mut keys, vec![button(back, true)]);
        frame(&mut view, &mut keys, vec![button(back, false)]);
        assert!(keys.0.len() == 2 && keys.0[0].1 < 30, "{:?}", keys.0);

        // A whole click within one frame: struck, then let go.
        keys.0.clear();
        frame(&mut view, &mut keys, vec![Event::PointerMoved(c4), button(c4, true), button(c4, false)]);
        assert!(keys.0.len() == 2 && keys.0[0].0 == 60 && keys.0[0].1 > 100 && keys.0[1] == (60, 0), "{:?}", keys.0);
        // The window losing the focus while a key is held lets it go, and
        // only a new press plays again.
        keys.0.clear();
        frame(&mut view, &mut keys, vec![button(c4, true)]);
        frame(&mut view, &mut keys, vec![Event::WindowFocused(false)]);
        assert!(keys.0.len() == 2 && keys.0[1] == (60, 0) && view.playing.is_none(), "{:?}", keys.0);
        frame(&mut view, &mut keys, vec![Event::WindowFocused(true), Event::PointerMoved(d4)]);
        assert_eq!(keys.0.len(), 2, "{:?}", keys.0);
        frame(&mut view, &mut keys, vec![button(d4, false)]);

        // Pressed off the keys (the view's top corner): a drag turns it.
        keys.0.clear();
        let corner = rect.min + Vec2::splat(10.0);
        frame(&mut view, &mut keys, vec![Event::PointerMoved(corner)]);
        frame(&mut view, &mut keys, vec![button(corner, true)]);
        for k in 1..=6 {
            frame(&mut view, &mut keys, vec![Event::PointerMoved(corner + Vec2::new(10.0 * k as f32, 0.0))]);
        }
        frame(&mut view, &mut keys, vec![button(corner + Vec2::new(60.0, 0.0), false)]);
        assert!(keys.0.is_empty(), "{:?}", keys.0);
        assert_ne!(view.orbit.unwrap().yaw, orbit.yaw);
    }

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
    fn names_never_overlap() {
        let bounds = Rect::from_min_size(egui::pos2(0.0, 0.0), Vec2::new(600.0, 500.0));
        let size = Vec2::new(70.0, 17.0);
        // Points nearly on each other, others at the view's corners.
        let mut items: Vec<_> = (0..8).map(|i| (egui::pos2(300.0 + i as f32, 250.0 + 2.0 * i as f32), size)).collect();
        items.extend([(egui::pos2(5.0, 5.0), size), (egui::pos2(595.0, 495.0), size), (egui::pos2(595.0, 5.0), size)]);
        let places = place_labels(&items, bounds);
        for (i, a) in places.iter().enumerate() {
            assert!(bounds.contains_rect(*a), "{i}: {a:?}");
            for b in &places[..i] {
                assert!(!a.intersects(*b), "{a:?} on {b:?}");
            }
        }
        // A lone name stays where it always was, up right of its point.
        let r = place_labels(&[(egui::pos2(100.0, 100.0), size)], bounds)[0];
        assert_eq!(r.left_bottom(), egui::pos2(102.0, 98.0));
    }

    #[test]
    fn levels_span_their_range() {
        assert_eq!(level(1.0, 1.0, 40.0, 20.0), 1.0);
        assert!((level(0.1, 1.0, 40.0, 20.0) - 0.5).abs() < 1e-6);
        assert_eq!(level(0.0, 1.0, 40.0, 20.0), 0.0);
    }

    #[test]
    fn a_long_rest_does_not_light_what_is_left() {
        let mut v = ModelView::default();
        // The reference long come down after the last note, only what is
        // left in the strings to see.
        v.levels.keys = 1e-30;
        let mut o = Observation::default();
        o.keys[100].energy = piano_core::engine::Shortcuts::DEFAULT.voice_silence;
        assert_eq!(v.key_levels(&o)[100], 0.0);
        // Then a soft note.
        o.keys[60].energy = 1.5;
        let l = v.key_levels(&o);
        assert_eq!(l[100], 0.0);
        assert!(l[60] > 0.9, "a soft note still shows: {}", l[60]);
    }
}
