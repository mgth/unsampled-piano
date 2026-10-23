//! The microphones' page: the board from above and seen from the keyboard,
//! the room from above, with the two microphones (and the piano in the
//! room) dragged into place. Drawn from the model, to scale. A move is
//! applied when the drag ends, since it rebuilds the instrument.
//!
//! Board coordinates (m): x along the strings from the board's keyboard
//! edge towards the tail, y across from the treble side, z up. Both views
//! put the keyboard at the bottom and the treble on the right, as the
//! pianist sees it.

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
    /// `mics` x and y in shares of the board (they follow its size).
    relative: bool,
    lid_angle: f64,
    /// Level keeping the loudness of the other set-ups.
    gain_db: f64,
}

/// The listening presets' microphone set-ups, and the calibrated one (the
/// model's default), for hosts without the preset folder.
const SETUPS: [Setup; 4] = [
    Setup {
        name: "Close",
        hint: "40 cm above the bass and treble parts of the bridge, lid open at 35°",
        preset: Some("close"),
        mics: [(0.294118, 0.346154, 0.4), (0.705882, 0.653846, 0.4)],
        relative: true,
        lid_angle: 35.0,
        gain_db: -9.6,
    },
    Setup {
        name: "Concert",
        hint: "A pair in front of the opening, 2 m above the board, lid open at 35°",
        preset: Some("concert"),
        mics: [(0.323529, -1.230769, 2.0), (0.735294, -1.230769, 2.0)],
        relative: true,
        lid_angle: 35.0,
        gain_db: 3.4,
    },
    Setup {
        name: "Audience",
        hint: "The concert pair 1 m above the board, lid closed",
        preset: Some("audience_lid_closed"),
        mics: [(0.323529, -1.230769, 1.0), (0.735294, -1.230769, 1.0)],
        relative: true,
        lid_angle: 0.0,
        gain_db: 0.0,
    },
    Setup {
        name: "Above",
        hint: "1.5 m above the board, lid removed: the set-up the model was calibrated with",
        preset: None,
        mics: SoundboardParams::GRAND_2M.mics,
        relative: false,
        lid_angle: 0.0,
        gain_db: 0.0,
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
        let front = keyboard_front(model);
        let min = |init: f64, f: &dyn Fn(&[f64; 3]) -> f64| mics.iter().map(f).fold(init, f64::min);
        let max = |init: f64, f: &dyn Fn(&[f64; 3]) -> f64| mics.iter().map(f).fold(init, f64::max);
        let rim = if model.rim.enabled { model.rim.thickness } else { 0.0 };
        Self {
            y: (min((sb.b - lid_reach).min(-rim), &|m| m[1]) - 0.5, max(sb.b + rim, &|m| m[1]) + 0.4),
            x: (min(front, &|m| m[0]) - 0.3, max(sb.a + rim, &|m| m[0]) + 0.4),
            z: (min(-model.room.piano[2], &|m| m[2]) - 0.1, max(lid_top.max(0.6), &|m| m[2]) + 0.4),
        }
    }
}

/// How far the open lid reaches over the board from its hinge (y), and the
/// height of its edge (m).
fn lid_extent(sb: &SoundboardParams) -> (f64, f64) {
    if sb.lid_angle <= 0.0 {
        return (0.0, sb.lid_hinge_height);
    }
    let (th, len) = (sb.lid_angle.to_radians(), 1.05 * sb.b);
    (len * th.cos(), sb.lid_hinge_height + len * th.sin())
}

fn keyboard_front(model: &PianoModel) -> f64 {
    (if model.frame.enabled { model.frame.offset_x } else { -0.2 }) - KEYBOARD
}

/// Whether the open lid stands between the board's centre and the mic (it
/// then hears the board through the lid), as the radiation computes it.
fn behind_lid(sb: &SoundboardParams, m: [f64; 3]) -> bool {
    if sb.lid_angle <= 0.0 {
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
    (0.0..=1.05 * sb.b).contains(&t) && (-0.05..=sb.a + 0.05).contains(&x[0])
}

/// Edits putting the microphones at `moved` (m), in the model's units
/// (shares of the board when its positions are relative), for the
/// coordinates that changed.
fn mic_edits(raw: &SoundboardParams, moved: &[[f64; 3]; 2]) -> Vec<(String, f64)> {
    let size = [raw.a, raw.b, 1.0];
    let before = raw.resolved().mics.map(|(x, y, z)| [x, y, z]);
    let mut edits = Vec::new();
    for (i, m) in moved.iter().enumerate() {
        for k in (0..3).filter(|&k| cm(m[k]) != cm(before[i][k])) {
            let v = if raw.mics_relative && k < 2 { (cm(m[k]) / size[k] * 1e4).round() / 1e4 } else { cm(m[k]) };
            edits.push((format!("soundboard.mics[{i}][{k}]"), v));
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
}

impl Map {
    fn sx(&self, y: f64) -> f32 {
        self.left + (self.b.y.1 - y) as f32 * self.s
    }
    fn sy(&self, x: f64) -> f32 {
        self.plan_bottom - (x - self.b.x.0) as f32 * self.s
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
        self.b.x.0 + ((self.plan_bottom - py) / self.s) as f64
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
                let on = s.mics == raw.mics && s.relative == raw.mics_relative && s.lid_angle == raw.lid_angle && s.gain_db == raw.mic_gain_db;
                let mut hint = format!("{}. Also sets the microphone gain ({} dB) to keep a comparable loudness.", s.hint, s.gain_db);
                if let Some(p) = s.preset {
                    hint += &format!(" Same as the listening preset \u{201c}{p}\u{201d}.");
                }
                if ui.selectable_label(on, s.name).on_hover_text(hint).clicked() {
                    let mut edits = vec![
                        ("soundboard.lid_angle".to_string(), s.lid_angle),
                        ("soundboard.mic_gain_db".to_string(), s.gain_db),
                        ("soundboard.mics_relative".to_string(), if s.relative { 1.0 } else { 0.0 }),
                    ];
                    for (i, m) in s.mics.iter().enumerate() {
                        for (j, v) in [m.0, m.1, m.2].into_iter().enumerate() {
                            edits.push((format!("soundboard.mics[{i}][{j}]"), v));
                        }
                    }
                    edits.retain(|(p, v)| piano_params::get(model, p).is_none_or(|old| old.as_f64() != *v));
                    commit(control, &edits);
                }
            }
            ui.separator();
            if ui.button("Swap left and right").on_hover_text("Exchange the two microphones (the stereo image turns round)").clicked() {
                let m = raw.mics.map(|(x, y, z)| [x, y, z]);
                let edits: Vec<(String, f64)> =
                    (0..2).flat_map(|i| (0..3).map(move |j| (format!("soundboard.mics[{i}][{j}]"), m[1 - i][j]))).collect();
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
        let map = Map { b: bounds, s, left: rect.left(), plan_bottom: rect.top() + plan_h, elev_bottom: rect.bottom() };

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
        let keys = Rect::from_two_pos(map.plan(fx0 - KEYBOARD, 0.0), map.plan(fx0, sb.b));
        p.rect_filled(keys, 2.0, vis.extreme_bg_color);
        p.rect_stroke(keys, 2.0, thin, egui::StrokeKind::Middle);
        for k in 1..52 {
            let x = keys.left() + keys.width() * k as f32 / 52.0;
            p.line_segment([Pos2::new(x, keys.top()), Pos2::new(x, keys.bottom())], Stroke::new(0.5, weak));
        }
        p.text(keys.center_bottom() + Vec2::new(0.0, 2.0), Align2::CENTER_TOP, "Keyboard", font.clone(), weak);
        let pts = |poly: &[(f64, f64)]| poly.iter().map(|&(u, v)| map.plan(u * sb.a, v * sb.b)).collect::<Vec<_>>();
        p.add(Shape::line(pts(&sb.treble_bridge), Stroke::new(3.0, BRIDGE)));
        p.add(Shape::line(pts(&sb.bass_bridge), Stroke::new(3.0, BRIDGE)));
        let (reach, top) = lid_extent(sb);
        if reach > 0.0 {
            p.line_segment([map.plan(0.0, sb.b), map.plan(sb.a, sb.b)], Stroke::new(3.0, LID));
            p.text(map.plan(sb.a, sb.b) + Vec2::new(0.0, -4.0), Align2::CENTER_BOTTOM, "Hinge", font.clone(), weak);
        }
        if reach > 0.0 {
            let lid = Rect::from_two_pos(map.plan(0.0, sb.b), map.plan(sb.a, sb.b - reach));
            p.rect_filled(lid, 0.0, LID.gamma_multiply(0.3));
            dashed_rect(&p, lid, Stroke::new(1.5, LID));
            p.text(map.plan(sb.a, sb.b - reach) + Vec2::new(0.0, -4.0), Align2::CENTER_BOTTOM, "Lid", font.clone(), weak);
        }
        p.text(map.plan(sb.a / 2.0, 0.0) + Vec2::new(6.0, 0.0), Align2::LEFT_CENTER, "Treble", font.clone(), weak);
        p.text(map.plan(sb.a / 2.0, sb.b + rim) + Vec2::new(-6.0, 0.0), Align2::RIGHT_CENTER, "Bass", font.clone(), weak);
        p.text(Pos2::new(rect.left() + 2.0, rect.top() + 2.0), Align2::LEFT_TOP, "From above", font.clone(), ink);

        // Elevation, seen from the keyboard: floor, case, board, frame, lid.
        let sep = map.plan_bottom + GAP / 2.0;
        p.line_segment([Pos2::new(rect.left(), sep), Pos2::new(rect.right(), sep)], Stroke::new(0.5, weak));
        let floor = -model.room.piano[2];
        p.line_segment([map.elev(bounds.y.1, floor), map.elev(bounds.y.0, floor)], Stroke::new(1.5, weak));
        p.text(map.elev(bounds.y.1, floor) + Vec2::new(2.0, -2.0), Align2::LEFT_BOTTOM, "Floor", font.clone(), weak);
        if model.rim.enabled {
            let r = &model.rim;
            let case = Rect::from_two_pos(map.elev(-rim, r.top - r.height), map.elev(sb.b + rim, r.top));
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
        p.text(Pos2::new(rect.left() + 2.0, sep + 4.0), Align2::LEFT_TOP, "Seen from the keyboard", font.clone(), ink);
        // 1 m scale.
        let y0 = sep + 26.0;
        let x0 = rect.left() + 4.0;
        p.line_segment([Pos2::new(x0, y0), Pos2::new(x0 + s, y0)], Stroke::new(1.5, ink));
        p.text(Pos2::new(x0 + s + 4.0, y0), Align2::LEFT_CENTER, "1 m", font.clone(), ink);

        // Microphones, with their stand in the elevation; a pair seen as one
        // is drawn as one split handle.
        let shown = self.shown_mics(&committed);
        for i in 0..2 {
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
        let board = Rect::from_two_pos(at(piano[0], piano[1]), at(piano[0] + sb.a, piano[1] + sb.b));
        let grab = ui.interact(board.expand(4.0), ui.scope_id().with("piano-in-room"), Sense::drag()).on_hover_cursor(egui::CursorIcon::Grab);
        if grab.drag_started() {
            let mics = sb.mics.map(|(x, y, z)| [x, y, z]);
            self.drag = Some(Drag { grab: Grab::Piano, mics, piano: room.piano, bounds: Bounds::fit(model, sb, &mics) });
        }
        if let Some(d) = self.drag.as_mut().filter(|d| d.grab == Grab::Piano) {
            if grab.dragged() {
                let dv = grab.drag_delta();
                d.piano[0] = (d.piano[0] - (dv.y / r) as f64).clamp(0.0, (lx - sb.a).max(0.0));
                d.piano[1] = (d.piano[1] - (dv.x / r) as f64).clamp(0.0, (ly - sb.b).max(0.0));
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
            p.circle_filled(at(piano[0] + m[0], piano[1] + m[1]), 3.0, c);
        }
        resp.on_hover_text(format!(
            "{lx} × {ly} m. Drag the piano to move it. The circle is the critical distance ({rc:.1} m): beyond it the reverberation is louder than the direct sound."
        ));
        let (t_lo, t_hi) = room.t60();
        ui.label(RichText::new(format!("Reverberation {t_lo:.1} s (low), {t_hi:.1} s (high)")).small().weak());
    }

    fn readout(&self, ui: &mut Ui, model: &PianoModel, sb: &SoundboardParams) {
        let shown = self.shown_mics(&sb.mics.map(|(x, y, z)| [x, y, z]));
        let piano = model.room.piano;
        let (src, rc) = source_and_critical_distance(model, sb, piano);
        for (i, &m) in shown.iter().enumerate() {
            let d = ((piano[0] + m[0] - src[0]).powi(2) + (piano[1] + m[1] - src[1]).powi(2) + (piano[2] + m[2] - src[2]).powi(2)).sqrt();
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
/// distance at which reverberation equals the direct sound (tail level
/// included), as the room model sets them.
fn source_and_critical_distance(model: &PianoModel, sb: &SoundboardParams, piano: [f64; 3]) -> ([f64; 3], f64) {
    let room = &model.room;
    let src = [piano[0] + sb.a / 2.0, piano[1] + sb.b / 2.0, piano[2] + 0.1];
    let (t_lo, t_hi) = room.t60();
    let rc = 0.057 * (room.volume() / (0.5 * (t_lo + t_hi))).sqrt() / room.tail_gain.max(1e-3);
    (src, rc)
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
        let sb = SoundboardParams { lid_angle: 35.0, ..SoundboardParams::GRAND_2M };
        // Concert pair: in front of the opening, not behind the lid.
        assert!(!behind_lid(&sb, [0.55, -1.6, 2.0]));
        // High above the bass side: behind it.
        assert!(behind_lid(&sb, [0.85, 1.6, 1.8]));
        assert!(!behind_lid(&SoundboardParams { lid_angle: 0.0, ..sb }, [0.85, 1.6, 1.8]));
    }

    #[test]
    fn setups_match_the_listening_presets() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../presets/listening/Microphones");
        for s in SETUPS.iter() {
            let Some(name) = s.preset else { continue };
            let m = piano_host::load_listening(&dir.join(format!("{name}.toml")), &PianoModel::default()).unwrap();
            let sb = m.soundboard.unwrap();
            assert_eq!((sb.mics, sb.mics_relative, sb.lid_angle, sb.mic_gain_db), (s.mics, s.relative, s.lid_angle, s.gain_db), "{name}");
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
    fn setups_fit_the_curated_ranges() {
        let reg = piano_params::Registry::new();
        for s in &SETUPS {
            for (i, m) in s.mics.iter().enumerate() {
                for (j, v) in [m.0, m.1, m.2].into_iter().enumerate() {
                    let meta = &reg.get(&format!("soundboard.mics[{i}][{j}]")).unwrap().meta;
                    assert!(meta.curated && (meta.min..=meta.max).contains(&v), "{} mic {i}: {v}", s.name);
                }
            }
        }
    }

    #[test]
    fn default_setup_is_the_model_default() {
        let sb = PianoModel::default().soundboard.unwrap();
        let s = &SETUPS[3];
        assert_eq!((s.mics, s.relative, s.lid_angle, s.gain_db), (sb.mics, sb.mics_relative, sb.lid_angle, sb.mic_gain_db));
    }
}
