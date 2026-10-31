//! The microphones' page: the board from above and seen from the keyboard,
//! the room from above, with the two microphones (and the piano in the
//! room) dragged into place. Drawn from the model, to scale. A move is
//! applied when the drag ends, since it rebuilds the instrument.
//!
//! Board coordinates (m): x along the strings from the board's keyboard
//! edge towards the tail, y across from the treble side, z up. Both views
//! put the keyboard at the bottom and the treble on the right, as the
//! pianist sees it.

use crate::model::scene::{clip, signed_area, triangulate};
use egui::{Align2, Color32, FontId, Pos2, Rect, RichText, Sense, Shape, Stroke, Ui, Vec2};
use piano_core::params::PianoModel;
use piano_core::soundboard::SoundboardParams;
use piano_params::{PianoControl, Value};

const MIC_COLORS: [Color32; 2] = [Color32::from_rgb(127, 119, 221), Color32::from_rgb(29, 158, 117)];
const NAMES: [&str; 2] = ["Left", "Right"];
const WOOD: Color32 = Color32::from_rgb(186, 117, 23);
const BRIDGE: Color32 = Color32::from_rgb(205, 95, 50);
const LID: Color32 = Color32::from_rgb(140, 140, 150);
/// Keyboard depth in front of the frame (m), for the drawing.
const KEYBOARD: f64 = 0.15;
const HANDLE: f32 = 9.0;

struct Setup {
    name: &'static str,
    hint: &'static str,
    /// The listening preset it matches (presets/listening/Microphones).
    preset: Option<&'static str>,
    mics: [(f64, f64, f64); 2],
    /// The positions for an upright (`mics_upright`).
    upright: [(f64, f64, f64); 2],
    /// `mics` x and y in shares of the board (they follow its size).
    relative: bool,
    /// 0: the lid shut on the case.
    lid_angle: f64,
    lid_removed: bool,
    /// Level keeping the loudness of the other set-ups.
    gain_db: f64,
    /// Pick-up pattern: 1 omni, 0.5 cardioid.
    pattern: f64,
}

/// The listening presets' microphone set-ups, and the calibrated one (the
/// model's default), for hosts without the preset folder.
const SETUPS: [Setup; 4] = [
    Setup {
        name: "Close",
        hint: "Cardioids 40 cm above the bass and treble parts of the bridge, lid open at 35°",
        preset: Some("close"),
        mics: [(0.294118, 0.346154, 0.4), (0.705882, 0.653846, 0.4)],
        upright: [(0.705882, 0.653846, 0.4), (0.294118, 0.346154, 0.4)],
        relative: true,
        lid_angle: 35.0,
        lid_removed: false,
        gain_db: -9.6,
        pattern: 0.5,
    },
    Setup {
        name: "Concert",
        hint: "An omni pair in front of the opening, 2 m above the board, lid open at 35°",
        preset: Some("concert"),
        mics: [(0.323529, -1.230769, 2.0), (0.735294, -1.230769, 2.0)],
        upright: [(-0.2, 0.7, 2.0), (-0.2, 0.3, 2.0)],
        relative: true,
        lid_angle: 35.0,
        lid_removed: false,
        gain_db: 3.4,
        pattern: 1.0,
    },
    Setup {
        name: "Audience",
        hint: "The concert pair 1 m above the board, lid shut",
        preset: Some("audience_lid_closed"),
        mics: [(0.323529, -1.230769, 1.0), (0.735294, -1.230769, 1.0)],
        upright: [(0.0, 0.7, 1.6), (0.0, 0.3, 1.6)],
        relative: true,
        lid_angle: 0.0,
        lid_removed: false,
        gain_db: 0.0,
        pattern: 1.0,
    },
    Setup {
        name: "Above",
        hint: "1.5 m above the board, lid removed: the set-up the model was calibrated with",
        preset: None,
        mics: SoundboardParams::GRAND_2M.mics,
        upright: SoundboardParams::GRAND_2M.mics_upright,
        relative: false,
        lid_angle: 0.0,
        lid_removed: true,
        gain_db: 0.0,
        pattern: 1.0,
    },
];

#[derive(Clone, Copy, PartialEq)]
enum Grab {
    /// The microphones under the pointer: one, or both when their handles
    /// overlap in that view (a pair on one stand moves together).
    Mics([bool; 2]),
    Piano,
}

/// A drag in progress: the positions shown, and the views frozen as they
/// were when it started (refitted once it is applied).
#[derive(Clone, Copy)]
struct Drag {
    grab: Grab,
    mics: [[f64; 3]; 2],
    piano: [f64; 3],
    bounds: Bounds,
}

#[derive(Default)]
pub struct MicView {
    drag: Option<Drag>,
    /// Where the handles were drawn ([plan, elevation][mic]), for tests.
    handles: [[Pos2; 2]; 2],
}

/// Extent of the board views (m): y across, x along, z up.
#[derive(Clone, Copy)]
struct Bounds {
    y: (f64, f64),
    x: (f64, f64),
    z: (f64, f64),
}

impl Bounds {
    /// The instrument, the lid, the floor and the microphones, with room
    /// around to drag further.
    fn fit(model: &PianoModel, sb: &SoundboardParams, mics: &[[f64; 3]; 2]) -> Self {
        let (lid_reach, lid_top) = lid_extent(sb);
        let front = if sb.vertical { 0.0 } else { keyboard_front(model) };
        // Below a flat board, the floor; behind a standing one, its wall.
        let under = if sb.vertical { -sb.wall_distance.max(0.0) } else { -model.room.piano[2] };
        let min = |init: f64, f: &dyn Fn(&[f64; 3]) -> f64| mics.iter().map(f).fold(init, f64::min);
        let max = |init: f64, f: &dyn Fn(&[f64; 3]) -> f64| mics.iter().map(f).fold(init, f64::max);
        let rim = if model.rim.enabled { model.rim.thickness } else { 0.0 };
        Self {
            y: (min((sb.b - lid_reach).min(-rim), &|m| m[1]) - 0.5, max(sb.b + rim, &|m| m[1]) + 0.4),
            x: (min(front, &|m| m[0]) - 0.3, max(sb.a + rim, &|m| m[0]) + 0.4),
            z: (min(under, &|m| m[2]) - 0.1, max(lid_top.max(0.6), &|m| m[2]) + 0.4),
        }
    }
}

/// How far the open lid reaches over the board from its hinge (y), and the
/// height of its edge (m).
fn lid_extent(sb: &SoundboardParams) -> (f64, f64) {
    if !open(sb) {
        return (0.0, sb.lid_hinge_height);
    }
    // From the hinge to the lid's edge farthest from it.
    let len = sb.b - sb.lid_outline().points.iter().map(|p| p.1).fold(sb.b, f64::min);
    let th = sb.lid_angle.to_radians();
    (len * th.cos(), sb.lid_hinge_height + len * th.sin())
}

/// The lid seen from above, counter-clockwise in board coordinates: its
/// outline as the radiation has it ([`SoundboardParams::lid_outline`]),
/// open turned about its hinge (y = b), what lies past the hinge being the
/// rim's top. Empty when there is none.
fn lid_plan(sb: &SoundboardParams) -> Vec<(f64, f64)> {
    let lid = sb.lid_outline().points;
    let mut top = if shut(sb) {
        lid
    } else if open(sb) {
        let far = 10.0 * (sb.a + sb.b);
        let cos = sb.lid_angle.to_radians().cos();
        clip(&lid, &[(-far, -far), (far, -far), (far, sb.b), (-far, sb.b)]).into_iter().map(|(x, y)| (x, sb.b - (sb.b - y) * cos)).collect()
    } else {
        Vec::new()
    };
    if signed_area(&top) < 0.0 {
        top.reverse();
    }
    top
}

/// Whether the lid lies shut on the case.
fn shut(sb: &SoundboardParams) -> bool {
    !sb.vertical && !sb.lid_removed && sb.lid_angle <= 0.0
}

/// Whether the lid stands open.
fn open(sb: &SoundboardParams) -> bool {
    !sb.lid_removed && sb.lid_angle > 0.0
}

fn keyboard_front(model: &PianoModel) -> f64 {
    (if model.frame.enabled { model.frame.offset_x } else { -0.2 }) - KEYBOARD
}

/// Whether the open lid stands between the board's centre and the mic (it
/// then hears the board through the lid), as the radiation computes it.
fn behind_lid(sb: &SoundboardParams, m: [f64; 3]) -> bool {
    if !open(sb) {
        return false;
    }
    let th = sb.lid_angle.to_radians();
    let p0 = [0.0, sb.b, sb.lid_hinge_height];
    let up = [0.0, -th.cos(), th.sin()];
    let n = [0.0, -th.sin(), -th.cos()];
    let dot = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let rel = |q: [f64; 3]| [q[0] - p0[0], q[1] - p0[1], q[2] - p0[2]];
    let c = [sb.a / 2.0, sb.b / 2.0, 0.0];
    let (fc, fm) = (dot(rel(c), n), dot(rel(m), n));
    if fm >= 0.0 || fc <= 0.0 {
        return false;
    }
    let u = fc / (fc - fm);
    let x = [c[0] + u * (m[0] - c[0]), c[1] + u * (m[1] - c[1]), c[2] + u * (m[2] - c[2])];
    let t = dot(rel(x), up);
    t >= 0.0 && sb.lid_outline().contains(x[0], sb.b - t)
}

/// The pair of positions in use: the upright's own on a standing board.
fn pair(raw: &SoundboardParams) -> (&'static str, [(f64, f64, f64); 2]) {
    if raw.vertical { ("mics_upright", raw.mics_upright) } else { ("mics", raw.mics) }
}

/// Edits putting the microphones at `moved` (m), in the model's units
/// (shares of the board when its positions are relative), for the
/// coordinates that changed, on the pair in use.
fn mic_edits(raw: &SoundboardParams, moved: &[[f64; 3]; 2]) -> Vec<(String, f64)> {
    let field = pair(raw).0;
    let size = [raw.a, raw.b, 1.0];
    let before = raw.resolved().mics.map(|(x, y, z)| [x, y, z]);
    let mut edits = Vec::new();
    for (i, m) in moved.iter().enumerate() {
        for k in (0..3).filter(|&k| cm(m[k]) != cm(before[i][k])) {
            let v = if raw.mics_relative && k < 2 { (cm(m[k]) / size[k] * 1e4).round() / 1e4 } else { cm(m[k]) };
            edits.push((format!("soundboard.{field}[{i}][{k}]"), v));
        }
    }
    edits
}

/// Applies edits as one gesture each; a rebuild follows once they pause.
fn commit(control: &mut dyn PianoControl, edits: &[(String, f64)]) {
    for (path, v) in edits {
        control.begin_edit(path);
        control.set(path, Value::Float(*v));
        control.end_edit(path);
    }
}

fn cm(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

/// Screen mapping of the board views: plan above, elevation below, the same
/// scale and the same horizontal axis (y, treble on the right).
struct Map {
    b: Bounds,
    s: f32,
    left: f32,
    plan_bottom: f32,
    elev_bottom: f32,
    /// x grows downwards: a standing board, seen from the front, has its
    /// agraffes at the top.
    flip: bool,
    plan_top: f32,
}

impl Map {
    fn sx(&self, y: f64) -> f32 {
        self.left + (self.b.y.1 - y) as f32 * self.s
    }
    fn sy(&self, x: f64) -> f32 {
        if self.flip {
            self.plan_top + (x - self.b.x.0) as f32 * self.s
        } else {
            self.plan_bottom - (x - self.b.x.0) as f32 * self.s
        }
    }
    fn ez(&self, z: f64) -> f32 {
        self.elev_bottom - (z - self.b.z.0) as f32 * self.s
    }
    fn plan(&self, x: f64, y: f64) -> Pos2 {
        Pos2::new(self.sx(y), self.sy(x))
    }
    fn elev(&self, y: f64, z: f64) -> Pos2 {
        Pos2::new(self.sx(y), self.ez(z))
    }
    fn y_at(&self, px: f32) -> f64 {
        self.b.y.1 - ((px - self.left) / self.s) as f64
    }
    fn x_at(&self, py: f32) -> f64 {
        if self.flip {
            self.b.x.0 + ((py - self.plan_top) / self.s) as f64
        } else {
            self.b.x.0 + ((self.plan_bottom - py) / self.s) as f64
        }
    }
    fn z_at(&self, py: f32) -> f64 {
        self.b.z.0 + ((self.elev_bottom - py) / self.s) as f64
    }
}

const GAP: f32 = 18.0;

impl MicView {
    pub fn show(&mut self, ui: &mut Ui, control: &mut dyn PianoControl, model: &PianoModel) {
        let Some(raw) = model.soundboard else {
            ui.label("This model has no soundboard: it outputs the bridge force, without microphones.");
            return;
        };
        // Drawn in metres whatever the model's units.
        let sb = raw.resolved();
        let committed = sb.mics.map(|(x, y, z)| [x, y, z]);
        self.setups(ui, control, model, &raw);
        ui.add_space(6.0);

        let bounds = self.drag.map_or_else(|| Bounds::fit(model, &sb, &committed), |d| d.bounds);
        let room_width = 200.0;
        let avail = ui.available_width();
        let side_by_side = avail >= 640.0;
        let width = if side_by_side { (avail - room_width - 24.0).min(620.0) } else { avail.min(620.0) };
        let (yspan, hspan) = (bounds.y.1 - bounds.y.0, bounds.x.1 - bounds.x.0 + bounds.z.1 - bounds.z.0);
        let s = (width / yspan as f32).min((540.0 - GAP) / hspan as f32);
        let size = Vec2::new(yspan as f32 * s, hspan as f32 * s + GAP);

        ui.horizontal_top(|ui| {
            self.board_views(ui, control, model, &raw, bounds, s, size);
            if side_by_side {
                ui.add_space(16.0);
                ui.vertical(|ui| self.room_view(ui, control, model, &sb, room_width));
            }
        });
        if !side_by_side {
            ui.add_space(10.0);
            self.room_view(ui, control, model, &sb, room_width.min(avail));
        }
        ui.add_space(8.0);
        self.readout(ui, model, &sb);
    }

    fn setups(&mut self, ui: &mut Ui, control: &mut dyn PianoControl, model: &PianoModel, raw: &SoundboardParams) {
        ui.horizontal_wrapped(|ui| {
            ui.label("Set-up:");
            for s in &SETUPS {
                let on = (if raw.vertical { s.upright == raw.mics_upright } else { s.mics == raw.mics })
                    && s.relative == raw.mics_relative
                    && s.lid_angle == raw.lid_angle
                    && s.lid_removed == raw.lid_removed
                    && s.gain_db == raw.mic_gain_db
                    && s.pattern == raw.mic_pattern;
                let mut hint = format!("{}. Also sets the microphone gain ({} dB) to keep a comparable loudness.", s.hint, s.gain_db);
                if let Some(p) = s.preset {
                    hint += &format!(" Same as the listening preset \u{201c}{p}\u{201d}.");
                }
                if ui.selectable_label(on, s.name).on_hover_text(hint).clicked() {
                    let mut edits = vec![
                        ("soundboard.lid_angle".to_string(), s.lid_angle),
                        ("soundboard.lid_removed".to_string(), if s.lid_removed { 1.0 } else { 0.0 }),
                        ("soundboard.mic_gain_db".to_string(), s.gain_db),
                        ("soundboard.mic_pattern".to_string(), s.pattern),
                        ("soundboard.mics_relative".to_string(), if s.relative { 1.0 } else { 0.0 }),
                    ];
                    for (field, mics) in [("mics", s.mics), ("mics_upright", s.upright)] {
                        for (i, m) in mics.iter().enumerate() {
                            for (j, v) in [m.0, m.1, m.2].into_iter().enumerate() {
                                edits.push((format!("soundboard.{field}[{i}][{j}]"), v));
                            }
                        }
                    }
                    edits.retain(|(p, v)| piano_params::get(model, p).is_none_or(|old| old.as_f64() != *v));
                    commit(control, &edits);
                }
            }
            ui.separator();
            if ui.button("Swap left and right").on_hover_text("Exchange the two microphones (the stereo image turns round)").clicked() {
                let (field, m) = pair(raw);
                let m = m.map(|(x, y, z)| [x, y, z]);
                let edits: Vec<(String, f64)> =
                    (0..2).flat_map(|i| (0..3).map(move |j| (format!("soundboard.{field}[{i}][{j}]"), m[1 - i][j]))).collect();
                commit(control, &edits);
            }
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn board_views(
        &mut self,
        ui: &mut Ui,
        control: &mut dyn PianoControl,
        model: &PianoModel,
        raw: &SoundboardParams,
        bounds: Bounds,
        s: f32,
        size: Vec2,
    ) {
        let sb = &raw.resolved();
        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
        let plan_h = (bounds.x.1 - bounds.x.0) as f32 * s;
        let map = Map {
            b: bounds,
            s,
            left: rect.left(),
            plan_bottom: rect.top() + plan_h,
            elev_bottom: rect.bottom(),
            flip: sb.vertical,
            plan_top: rect.top(),
        };

        // Drags first, so that the drawing shows where the pointer is.
        let committed = sb.mics.map(|(x, y, z)| [x, y, z]);
        let shown = self.shown_mics(&committed);
        for (v, view) in ["plan", "elev"].into_iter().enumerate() {
            let at = |m: &[f64; 3]| if view == "plan" { map.plan(m[0], m[1]) } else { map.elev(m[1], m[2]) };
            self.handles[v] = [at(&shown[0]), at(&shown[1])];
            let together = at(&shown[0]).distance(at(&shown[1])) < 1.5 * HANDLE;
            // The right mic is drawn over the left one: its handle first.
            for i in [1, 0] {
                if together && i == 0 {
                    continue;
                }
                let id = ui.scope_id().with(("piano-mic", view, i));
                let r = ui.interact(Rect::from_center_size(at(&shown[i]), Vec2::splat(2.5 * HANDLE)), id, Sense::drag());
                let r = r.on_hover_cursor(egui::CursorIcon::Grab);
                let r = if together {
                    r.on_hover_text("Both microphones, seen one behind the other: they move together. Separate them from above, or set one below.")
                } else {
                    r
                };
                if r.drag_started() {
                    let which = if together { [true, true] } else { [i == 0, i == 1] };
                    self.drag = Some(Drag { grab: Grab::Mics(which), mics: shown, piano: model.room.piano, bounds });
                }
                let Some(d) = self.drag.as_mut().filter(|d| matches!(d.grab, Grab::Mics(_))) else { continue };
                let Grab::Mics(which) = d.grab else { continue };
                if let (true, Some(p)) = (r.dragged(), r.interact_pointer_pos()) {
                    // The grabbed mic follows the pointer, the other one of a
                    // pair keeps its offset.
                    let mut target = d.mics[i];
                    target[1] = map.y_at(p.x).clamp(bounds.y.0, bounds.y.1);
                    if view == "plan" {
                        target[0] = map.x_at(p.y).clamp(bounds.x.0, bounds.x.1);
                    } else {
                        target[2] = map.z_at(p.y).clamp(0.05, bounds.z.1);
                    }
                    let delta = [target[0] - d.mics[i][0], target[1] - d.mics[i][1], target[2] - d.mics[i][2]];
                    for j in (0..2).filter(|&j| which[j]) {
                        for (c, dc) in d.mics[j].iter_mut().zip(delta) {
                            *c += dc;
                        }
                        d.mics[j][2] = d.mics[j][2].max(0.05);
                    }
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                }
                if r.drag_stopped() {
                    let moved = d.mics;
                    self.drag = None;
                    commit(control, &mic_edits(raw, &moved));
                }
            }
        }

        let p = ui.painter_at(rect);
        let vis = ui.visuals();
        let (ink, weak) = (vis.text_color(), vis.weak_text_color());
        let font = FontId::proportional(11.0);
        let thin = Stroke::new(1.0, weak);
        let rim = if model.rim.enabled { model.rim.thickness } else { 0.0 };

        // Plan: case, board, frame, keyboard, bridges, hinge, lid.
        let board = Rect::from_two_pos(map.plan(0.0, 0.0), map.plan(sb.a, sb.b));
        if rim > 0.0 {
            let case = Rect::from_two_pos(map.plan(-rim, -rim), map.plan(sb.a + rim, sb.b + rim));
            p.rect_filled(case, 4.0, weak.gamma_multiply(0.35));
        }
        p.rect_filled(board, 0.0, WOOD.gamma_multiply(0.35));
        p.rect_stroke(board, 0.0, Stroke::new(1.0, WOOD), egui::StrokeKind::Middle);
        let f = &model.frame;
        let fx0 = if f.enabled { f.offset_x } else { -0.2 };
        if f.enabled {
            let frame = Rect::from_two_pos(map.plan(f.offset_x, 0.0), map.plan(f.offset_x + f.a, f.b));
            dashed_rect(&p, frame, thin);
            p.line_segment([map.plan(f.offset_x + f.agraffe_x, 0.0), map.plan(f.offset_x + f.agraffe_x, sb.b)], thin);
        }
        if !sb.vertical {
            let keys = Rect::from_two_pos(map.plan(fx0 - KEYBOARD, 0.0), map.plan(fx0, sb.b));
            p.rect_filled(keys, 2.0, vis.extreme_bg_color);
            p.rect_stroke(keys, 2.0, thin, egui::StrokeKind::Middle);
            for k in 1..52 {
                let x = keys.left() + keys.width() * k as f32 / 52.0;
                p.line_segment([Pos2::new(x, keys.top()), Pos2::new(x, keys.bottom())], Stroke::new(0.5, weak));
            }
            p.text(keys.center_bottom() + Vec2::new(0.0, 2.0), Align2::CENTER_TOP, "Keyboard", font.clone(), weak);
        }
        let pts = |main: bool| sb.bridge_line(main).into_iter().map(|(x, y)| map.plan(x, y)).collect::<Vec<_>>();
        p.add(Shape::line(pts(true), Stroke::new(3.0, BRIDGE)));
        p.add(Shape::line(pts(false), Stroke::new(3.0, BRIDGE)));
        let (reach, top) = lid_extent(sb);
        // The lid, its shape and overhang the radiation's (`behind_lid`).
        let lid = lid_plan(sb);
        if !lid.is_empty() {
            let mut mesh = egui::Mesh::default();
            for &(x, y) in &lid {
                mesh.colored_vertex(map.plan(x, y), LID.gamma_multiply(0.3));
            }
            for [i, j, k] in triangulate(&lid) {
                mesh.add_triangle(i as u32, j as u32, k as u32);
            }
            p.add(Shape::mesh(mesh));
            let mut edge: Vec<Pos2> = lid.iter().map(|&(x, y)| map.plan(x, y)).collect();
            edge.push(edge[0]);
            p.extend(Shape::dashed_line(&edge, Stroke::new(1.5, LID), 4.0, 3.0));
        }
        if reach > 0.0 {
            let along = lid.iter().filter(|q| q.1 >= sb.b - 1e-9).map(|q| q.0);
            let (x0, x1) = along.fold((sb.a, 0.0), |(lo, hi): (f64, f64), x| (lo.min(x), hi.max(x)));
            p.line_segment([map.plan(x0, sb.b), map.plan(x1, sb.b)], Stroke::new(3.0, LID));
            p.text(map.plan(x1, sb.b) + Vec2::new(0.0, -4.0), Align2::CENTER_BOTTOM, "Hinge", font.clone(), weak);
            // At the far end of its tail edge.
            let far = lid.iter().copied().fold((f64::MIN, sb.b), |m, q| if q.0 > m.0 + 1e-6 || (q.0 > m.0 - 1e-6 && q.1 < m.1) { q } else { m });
            p.text(map.plan(far.0, far.1) + Vec2::new(0.0, -4.0), Align2::CENTER_BOTTOM, "Lid", font.clone(), weak);
        }
        if shut(sb) {
            p.text(map.plan(sb.a, sb.b) + Vec2::new(0.0, -4.0), Align2::CENTER_BOTTOM, "Lid shut", font.clone(), weak);
        }
        p.text(map.plan(sb.a / 2.0, 0.0) + Vec2::new(6.0, 0.0), Align2::LEFT_CENTER, "Treble", font.clone(), weak);
        p.text(map.plan(sb.a / 2.0, sb.b + rim) + Vec2::new(-6.0, 0.0), Align2::RIGHT_CENTER, "Bass", font.clone(), weak);
        let title = if sb.vertical { "Facing the board (standing)" } else { "From above" };
        p.text(Pos2::new(rect.left() + 2.0, rect.top() + 2.0), Align2::LEFT_TOP, title, font.clone(), ink);

        // Elevation, seen from the keyboard: floor, case, board, frame, lid.
        let sep = map.plan_bottom + GAP / 2.0;
        p.line_segment([Pos2::new(rect.left(), sep), Pos2::new(rect.right(), sep)], Stroke::new(0.5, weak));
        // Under a flat board, the floor (the mics' stands reach it); behind
        // a standing one, its wall.
        let floor = -model.room.piano[2];
        if !sb.vertical {
            p.line_segment([map.elev(bounds.y.1, floor), map.elev(bounds.y.0, floor)], Stroke::new(1.5, weak));
            p.text(map.elev(bounds.y.1, floor) + Vec2::new(2.0, -2.0), Align2::LEFT_BOTTOM, "Floor", font.clone(), weak);
        } else if sb.wall_distance > 0.0 {
            let wz = -sb.wall_distance;
            p.line_segment([map.elev(bounds.y.1, wz), map.elev(bounds.y.0, wz)], Stroke::new(2.0, weak));
            p.text(map.elev(bounds.y.1, wz) + Vec2::new(2.0, -2.0), Align2::LEFT_BOTTOM, "Wall", font.clone(), weak);
        }
        if model.rim.enabled {
            let r = &model.rim;
            // Up to the lid's hinge on a grand, the lid resting on it.
            let top = if sb.vertical { r.top } else { r.top.max(sb.lid_hinge_height) };
            let case = Rect::from_two_pos(map.elev(-rim, r.top - r.height), map.elev(sb.b + rim, top));
            p.rect_filled(case, 2.0, weak.gamma_multiply(0.35));
        }
        p.line_segment([map.elev(0.0, 0.0), map.elev(sb.b, 0.0)], Stroke::new(3.0, WOOD));
        if f.enabled {
            p.line_segment([map.elev(0.0, f.height), map.elev(f.b, f.height)], Stroke::new(1.0, weak));
        }
        let hinge = map.elev(sb.b, sb.lid_hinge_height);
        if reach > 0.0 {
            p.circle_filled(hinge, 3.0, LID);
            p.line_segment([hinge, map.elev(sb.b - reach, top)], Stroke::new(3.0, LID));
            p.text(map.elev(sb.b - reach, top) + Vec2::new(4.0, 0.0), Align2::LEFT_CENTER, format!("Lid {:.0}°", sb.lid_angle), font.clone(), weak);
        }
        if shut(sb) {
            p.line_segment([hinge, map.elev(0.0, sb.lid_hinge_height)], Stroke::new(3.0, LID));
            p.text(map.elev(0.0, sb.lid_hinge_height) + Vec2::new(4.0, 0.0), Align2::LEFT_CENTER, "Lid shut", font.clone(), weak);
        }
        let title = if sb.vertical { "From above (height: distance in front)" } else { "Seen from the keyboard" };
        p.text(Pos2::new(rect.left() + 2.0, sep + 4.0), Align2::LEFT_TOP, title, font.clone(), ink);
        // 1 m scale.
        let y0 = sep + 26.0;
        let x0 = rect.left() + 4.0;
        p.line_segment([Pos2::new(x0, y0), Pos2::new(x0 + s, y0)], Stroke::new(1.5, ink));
        p.text(Pos2::new(x0 + s + 4.0, y0), Align2::LEFT_CENTER, "1 m", font.clone(), ink);

        // Microphones, with their stand in the elevation; a pair seen as one
        // is drawn as one split handle.
        let shown = self.shown_mics(&committed);
        for i in (0..2).filter(|_| !sb.vertical) {
            let m = shown[i];
            p.line_segment([map.elev(m[1], m[2]), map.elev(m[1], floor)], Stroke::new(1.0, MIC_COLORS[i].gamma_multiply(0.6)));
        }
        for view in ["plan", "elev"] {
            let at = |m: &[f64; 3]| if view == "plan" { map.plan(m[0], m[1]) } else { map.elev(m[1], m[2]) };
            let (a, b) = (at(&shown[0]), at(&shown[1]));
            if a.distance(b) < 1.5 * HANDLE {
                p.circle_filled(b, HANDLE + 2.0, MIC_COLORS[1]);
                let half: Vec<Pos2> = (0..=12)
                    .map(|k| std::f32::consts::FRAC_PI_2 + std::f32::consts::PI * k as f32 / 12.0)
                    .map(|t| b + (HANDLE + 2.0) * Vec2::angled(t))
                    .collect();
                p.add(Shape::convex_polygon(half, MIC_COLORS[0], Stroke::NONE));
                p.text(b, Align2::CENTER_CENTER, "L R", FontId::proportional(10.0), Color32::WHITE);
            } else {
                for (i, at) in [a, b].into_iter().enumerate() {
                    p.circle_filled(at, HANDLE, MIC_COLORS[i]);
                    p.text(at, Align2::CENTER_CENTER, &NAMES[i][..1], FontId::proportional(11.0), Color32::WHITE);
                }
            }
        }
        if let Some(Drag { grab: Grab::Mics(which), .. }) = self.drag {
            for i in (0..2).filter(|&i| which[i]) {
                let m = shown[i];
                let at = map.plan(m[0], m[1]) + Vec2::new(HANDLE + 4.0, 0.0);
                p.text(at, Align2::LEFT_CENTER, format!("x {:.2}  y {:.2}  z {:.2} m", m[0], m[1], m[2]), font.clone(), ink);
            }
        }
    }

    /// The room from above, the piano dragged in it.
    fn room_view(&mut self, ui: &mut Ui, control: &mut dyn PianoControl, model: &PianoModel, sb: &SoundboardParams, width: f32) {
        let room = &model.room;
        ui.label(RichText::new("The room from above").small());
        if !room.enabled {
            ui.label(RichText::new("No room: the microphones hear the piano alone.").weak());
            return;
        }
        let [lx, ly, _] = room.size;
        let r = (width / ly as f32).min(width / lx as f32);
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(ly as f32 * r, lx as f32 * r), Sense::hover());
        // Same orientation as the board views: x up, y to the left.
        let at = |x: f64, y: f64| Pos2::new(rect.left() + (ly - y) as f32 * r, rect.bottom() - x as f32 * r);
        let piano = match self.drag {
            Some(Drag { grab: Grab::Piano, piano, .. }) => piano,
            _ => room.piano,
        };
        // The board's footprint (a standing board's is its thickness).
        let (lo, hi) = footprint(model, sb, piano);
        let board = Rect::from_two_pos(at(lo[0], lo[1]), at(hi[0], hi[1])).expand(1.5);
        let grab = ui.interact(board.expand(4.0), ui.scope_id().with("piano-in-room"), Sense::drag()).on_hover_cursor(egui::CursorIcon::Grab);
        if grab.drag_started() {
            let mics = sb.mics.map(|(x, y, z)| [x, y, z]);
            self.drag = Some(Drag { grab: Grab::Piano, mics, piano: room.piano, bounds: Bounds::fit(model, sb, &mics) });
        }
        if let Some(d) = self.drag.as_mut().filter(|d| d.grab == Grab::Piano) {
            if grab.dragged() {
                let dv = grab.drag_delta();
                let (lo, hi) = footprint(model, sb, [0.0, 0.0, piano[2]]);
                d.piano[0] = (d.piano[0] - (dv.y / r) as f64).clamp(-lo[0], (lx - hi[0]).max(-lo[0]));
                d.piano[1] = (d.piano[1] - (dv.x / r) as f64).clamp(-lo[1], (ly - hi[1]).max(-lo[1]));
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
            }
            if grab.drag_stopped() {
                let edits: Vec<(String, f64)> = (0..2)
                    .filter(|&j| cm(d.piano[j]) != room.piano[j])
                    .map(|j| (format!("room.piano[{j}]"), cm(d.piano[j])))
                    .collect();
                self.drag = None;
                commit(control, &edits);
            }
        }
        let vis = ui.visuals();
        let p = ui.painter_at(rect.expand(1.0));
        p.rect_filled(rect, 2.0, vis.extreme_bg_color);
        p.rect_stroke(rect, 2.0, Stroke::new(1.0, vis.weak_text_color()), egui::StrokeKind::Inside);
        let (src, rc) = source_and_critical_distance(model, sb, piano);
        p.circle_stroke(at(src[0], src[1]), rc as f32 * r, Stroke::new(1.0, vis.weak_text_color()));
        p.rect_filled(board, 0.0, WOOD.gamma_multiply(0.6));
        let shown = self.shown_mics(&sb.mics.map(|(x, y, z)| [x, y, z]));
        for (m, c) in shown.iter().zip(MIC_COLORS) {
            let q = in_room(model, sb, piano, m);
            p.circle_filled(at(q[0], q[1]), 3.0, c);
        }
        resp.on_hover_text(format!(
            "{lx} × {ly} m. Drag the piano to move it. The circle is the critical distance for these microphones ({rc:.1} m): beyond it they hear the reverberation louder than the direct sound."
        ));
        let (t_lo, t_hi) = room.t60();
        ui.label(RichText::new(format!("Reverberation {t_lo:.1} s (low), {t_hi:.1} s (high)")).small().weak());
    }

    fn readout(&self, ui: &mut Ui, model: &PianoModel, sb: &SoundboardParams) {
        let shown = self.shown_mics(&sb.mics.map(|(x, y, z)| [x, y, z]));
        let piano = model.room.piano;
        let (src, rc) = source_and_critical_distance(model, sb, piano);
        for (i, &m) in shown.iter().enumerate() {
            let q = in_room(model, sb, piano, &m);
            let d = ((q[0] - src[0]).powi(2) + (q[1] - src[1]).powi(2) + (q[2] - src[2]).powi(2)).sqrt();
            let mut text = format!("{}: {:.2} m from the board's centre", NAMES[i], d);
            if model.room.enabled {
                let drr = 20.0 * (rc / d.max(0.3)).log10();
                text += &if drr >= 0.0 {
                    format!(", direct sound {drr:.0} dB above the reverberation")
                } else {
                    format!(", reverberation {:.0} dB above the direct sound", -drr)
                };
            }
            if behind_lid(sb, m) {
                text += ", behind the lid (hears the board through it)";
            } else if shut(sb) && m[2] >= 0.0 {
                text += ", hears the board through the shut lid";
            }
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
                ui.painter().circle_filled(r.center(), 5.0, MIC_COLORS[i]);
                ui.label(text);
            });
        }
        // Seen from the keyboard, the treble is on the right: a left mic
        // further right than the right one gives the audience's image.
        if shown[0][1] < shown[1][1] - 0.05 {
            ui.label(
                RichText::new(
                    "The left microphone is on the treble side: the stereo image is reversed from the pianist's \
                     (as heard from the audience). Swap them for the pianist's.",
                )
                .small()
                .weak(),
            );
        }
        ui.label(RichText::new("Drag the microphones in either view; the instrument is rebuilt when you let go.").small().weak());
    }

    /// The microphones as shown: the model's, or where a drag holds them.
    fn shown_mics(&self, committed: &[[f64; 3]; 2]) -> [[f64; 3]; 2] {
        match self.drag {
            Some(Drag { grab: Grab::Mics(_), mics, .. }) => mics,
            _ => *committed,
        }
    }
}

/// The room's source point (board centre, room coordinates) and the
/// distance at which the mics hear as much reverberation as direct sound,
/// as the room model sets them: the room's critical distance, shortened by
/// the tail level and lengthened by the piano's directivity Q and the mics'
/// pattern (their random-incidence efficiency a² + (1 − a)²/3).
/// A point in board coordinates, in the room with the piano at `piano`
/// (a standing board turned upright, as the room computes it).
pub(crate) fn in_room(model: &PianoModel, sb: &SoundboardParams, piano: [f64; 3], m: &[f64; 3]) -> [f64; 3] {
    let rp = piano_core::room::RoomParams { piano, ..model.room };
    piano_core::room::to_room(&rp, sb, (m[0], m[1], m[2]))
}

/// The board's extent in the room (lowest and highest corner).
pub(crate) fn footprint(model: &PianoModel, sb: &SoundboardParams, piano: [f64; 3]) -> ([f64; 3], [f64; 3]) {
    let corners = [[0.0, 0.0, 0.0], [sb.a, 0.0, 0.0], [0.0, sb.b, 0.0], [sb.a, sb.b, 0.0]].map(|c| in_room(model, sb, piano, &c));
    let pick = |f: fn(f64, f64) -> f64| std::array::from_fn(|i| corners.iter().map(|c| c[i]).reduce(f).unwrap_or(0.0));
    (pick(f64::min), pick(f64::max))
}

pub(crate) fn source_and_critical_distance(model: &PianoModel, sb: &SoundboardParams, piano: [f64; 3]) -> ([f64; 3], f64) {
    use piano_core::room;
    let rp = room::RoomParams { piano, ..model.room };
    (room::to_room(&rp, sb, room::source_point(sb)), room::critical_distance(&rp, sb))
}

fn dashed_rect(p: &egui::Painter, r: Rect, stroke: Stroke) {
    let pts = [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom(), r.left_top()];
    p.extend(Shape::dashed_line(&pts, stroke, 4.0, 3.0));
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{mock::MockControl, PianoUi};
    use egui::{Event, PointerButton, RawInput};

    /// Runs the interface on the microphones' page, one frame per event list.
    fn frames(ui: &mut PianoUi, control: &mut MockControl, ctx: &egui::Context, events: Vec<Vec<Event>>) {
        for ev in events {
            let input = RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100.0, 800.0))),
                events: ev,
                ..Default::default()
            };
            ctx.run_ui(input, |root| ui.show(root, control)).drop_without_applying_deltas();
        }
    }

    /// Press at `from`, move by `by` in steps, release.
    fn drag(from: Pos2, by: Vec2) -> Vec<Vec<Event>> {
        let button = |pos, pressed| Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Default::default() };
        let mut ev = vec![vec![Event::PointerMoved(from)], vec![button(from, true)]];
        ev.extend((1..=8).map(|k| vec![Event::PointerMoved(from + by * k as f32 / 8.0)]));
        ev.push(vec![button(from + by, false)]);
        ev.push(vec![]);
        ev
    }

    fn mics(c: &MockControl) -> [(f64, f64, f64); 2] {
        c.model().soundboard.unwrap().mics
    }

    #[test]
    fn dragging_moves_the_microphones() {
        let ctx = egui::Context::default();
        let mut ui = PianoUi::new(None);
        ui.open_page("Microphones");
        let mut control = MockControl::new(PianoModel::default());
        frames(&mut ui, &mut control, &ctx, vec![vec![], vec![]]);
        let before = mics(&control);

        // Left mic, in the plan, 40 px to the right: towards the treble (y
        // down), same x and height; nothing else moves.
        let from = ui.mics.handles[0][0];
        frames(&mut ui, &mut control, &ctx, drag(from, Vec2::new(40.0, 0.0)));
        let after = mics(&control);
        assert!(after[0].1 < before[0].1 - 0.1, "{before:?} → {after:?}");
        assert_eq!((after[0].0, after[0].2, after[1]), (before[0].0, before[0].2, before[1]));
        assert!(ui.mics.drag.is_none());

        // Right mic raised in the elevation.
        let from = ui.mics.handles[1][1];
        frames(&mut ui, &mut control, &ctx, drag(from, Vec2::new(0.0, -30.0)));
        let raised = mics(&control);
        assert!(raised[1].2 > after[1].2 + 0.1 && raised[1].1 == after[1].1 && raised[0] == after[0], "{after:?} → {raised:?}");
    }

    #[test]
    fn a_pair_seen_as_one_moves_together() {
        let ctx = egui::Context::default();
        let mut ui = PianoUi::new(None);
        ui.open_page("Microphones");
        let mut control = MockControl::new(PianoModel::default());
        frames(&mut ui, &mut control, &ctx, vec![vec![]]);
        // The concert set-up: the pair overlaps when seen from the keyboard.
        control.set("soundboard.mics_relative", Value::Bool(true));
        for (i, m) in SETUPS[1].mics.iter().enumerate() {
            for (j, v) in [m.0, m.1, m.2].into_iter().enumerate() {
                control.set(&format!("soundboard.mics[{i}][{j}]"), Value::Float(v));
            }
        }
        frames(&mut ui, &mut control, &ctx, vec![vec![], vec![]]);
        let from = ui.mics.handles[1][1];
        frames(&mut ui, &mut control, &ctx, drag(from, Vec2::new(0.0, 30.0)));
        // Lowered together, the rest kept in shares of the board.
        let m = mics(&control);
        assert!(m[0].2 < 1.9 && m[0].2 == m[1].2, "{m:?}");
        assert_eq!((m[0].0, m[1].0, m[0].1), (0.323529, 0.735294, -1.230769));
        assert!(control.model().soundboard.unwrap().mics_relative);
    }

    #[test]
    fn lid_shadow() {
        let sb = SoundboardParams { lid_angle: 35.0, lid_removed: false, ..SoundboardParams::GRAND_2M };
        // Concert pair: in front of the opening, not behind the lid.
        assert!(!behind_lid(&sb, [0.55, -1.6, 2.0]));
        // High above the bass side: behind it.
        assert!(behind_lid(&sb, [0.85, 1.6, 1.8]));
        assert!(!behind_lid(&SoundboardParams { lid_angle: 0.0, ..sb }, [0.85, 1.6, 1.8]));
        assert!(!behind_lid(&SoundboardParams { lid_removed: true, ..sb }, [0.85, 1.6, 1.8]));
    }

    #[test]
    fn lid_plan_has_the_case_shape() {
        use piano_core::soundboard::LID_OVERHANG;
        let sb = SoundboardParams { lid_angle: 35.0, lid_removed: false, ..SoundboardParams::GRAND_2M };
        let lid = lid_plan(&sb);
        let (reach, _) = lid_extent(&sb);
        // Over the rim at the front, its bentside cutting off the corner
        // the rectangle over the board had at the tail.
        assert!(lid.iter().any(|q| q.0 <= -LID_OVERHANG + 1e-9), "{lid:?}");
        let rect = (sb.a + 2.0 * LID_OVERHANG) * reach;
        assert!(signed_area(&lid) > 0.5 * rect && signed_area(&lid) < 0.85 * rect, "{} of {rect}", signed_area(&lid));
        // Shut, the whole outline; removed, none.
        let shut = lid_plan(&SoundboardParams { lid_angle: 0.0, ..sb });
        assert!((signed_area(&shut) - signed_area(&sb.lid_outline().points).abs()).abs() < 1e-9);
        assert!(lid_plan(&SoundboardParams { lid_removed: true, ..sb }).is_empty());
    }

    #[test]
    fn setups_match_the_listening_presets() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../presets/listening/Microphones");
        for s in SETUPS.iter() {
            let Some(name) = s.preset else { continue };
            let m = piano_host::load_listening(&dir.join(format!("{name}.toml")), &PianoModel::default()).unwrap();
            let sb = m.soundboard.unwrap();
            assert_eq!(
                (sb.mics, sb.mics_upright, sb.mics_relative, sb.lid_angle, sb.lid_removed, sb.mic_gain_db, sb.mic_pattern),
                (s.mics, s.upright, s.relative, s.lid_angle, s.lid_removed, s.gain_db, s.pattern),
                "{name}"
            );
        }
    }

    #[test]
    fn relative_positions_stay_relative() {
        let raw = SoundboardParams { mics_relative: true, mics: [(0.25, 0.5, 1.0), (0.75, 0.5, 1.0)], ..SoundboardParams::GRAND_2M };
        let mut moved = raw.resolved().mics.map(|(x, y, z)| [x, y, z]);
        moved[1][1] -= 0.13;
        assert_eq!(mic_edits(&raw, &moved), vec![("soundboard.mics[1][1]".to_string(), 0.4)]);
    }

    #[test]
    fn an_upright_moves_its_own_pair() {
        let raw = SoundboardParams { vertical: true, mics_relative: true, mics_upright: [(0.0, 0.7, 1.6), (0.0, 0.3, 1.6)], ..SoundboardParams::GRAND_2M };
        let mut moved = raw.resolved().mics.map(|(x, y, z)| [x, y, z]);
        assert_eq!(moved[0], [0.0, 0.7 * raw.b, 1.6]);
        moved[0][2] = 1.2;
        assert_eq!(mic_edits(&raw, &moved), vec![("soundboard.mics_upright[0][2]".to_string(), 1.2)]);
    }

    #[test]
    fn setups_fit_the_curated_ranges() {
        let reg = piano_params::Registry::new();
        for s in &SETUPS {
            for (field, mics) in [("mics", s.mics), ("mics_upright", s.upright)] {
                for (i, m) in mics.iter().enumerate() {
                    for (j, v) in [m.0, m.1, m.2].into_iter().enumerate() {
                        let meta = &reg.get(&format!("soundboard.{field}[{i}][{j}]")).unwrap().meta;
                        assert!(meta.curated && (meta.min..=meta.max).contains(&v), "{} {field} {i}: {v}", s.name);
                    }
                }
            }
        }
    }

    #[test]
    fn default_setup_is_the_model_default() {
        let sb = PianoModel::default().soundboard.unwrap();
        let s = &SETUPS[3];
        assert_eq!(
            (s.mics, s.upright, s.relative, s.lid_angle, s.lid_removed, s.gain_db, s.pattern),
            (sb.mics, sb.mics_upright, sb.mics_relative, sb.lid_angle, sb.lid_removed, sb.mic_gain_db, sb.mic_pattern)
        );
    }

    #[test]
    fn a_standing_board_stands_in_the_room() {
        let m = PianoModel::default();
        let sb = SoundboardParams { vertical: true, wall_distance: 0.2, ..m.soundboard.unwrap() };
        let (lo, hi) = footprint(&m, &sb, m.room.piano);
        // Its width along the room's x, its height up from the foot, flat
        // against its wall.
        assert!((hi[0] - lo[0] - sb.b).abs() < 1e-9 && (hi[2] - lo[2] - sb.a).abs() < 1e-9 && hi[1] == lo[1], "{lo:?} {hi:?}");
        let mic = in_room(&m, &sb, m.room.piano, &[0.5, 0.6, 1.0]);
        assert!((mic[1] - (sb.wall_distance + 1.0)).abs() < 1e-9, "{mic:?}");
    }

    #[test]
    fn critical_distance_follows_directivity_and_pattern() {
        let m = PianoModel::default();
        let m = PianoModel { room: piano_core::room::RoomParams { directivity: 3.0, ..m.room }, ..m };
        let sb = m.soundboard.unwrap();
        let (_, omni) = source_and_critical_distance(&m, &sb, m.room.piano);
        let cardioid = SoundboardParams { mic_pattern: 0.5, ..sb };
        let (_, card) = source_and_critical_distance(&m, &cardioid, m.room.piano);
        // A cardioid hears the diffuse room 1/3 as much: r_c × √3.
        assert!((card / omni - 3f64.sqrt()).abs() < 1e-9, "{omni} {card}");
        let beamed = PianoModel { room: piano_core::room::RoomParams { directivity: 12.0, ..m.room }, ..m };
        let (_, q) = source_and_critical_distance(&beamed, &sb, m.room.piano);
        assert!((q / omni - 2.0).abs() < 1e-9, "{omni} {q}");
    }
}
