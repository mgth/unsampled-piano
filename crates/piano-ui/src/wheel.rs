//! The temperament wheel: one turn is an octave, C at the top and the
//! pitch rising clockwise. Each note is a spoke at its own pitch; an orange
//! block near the rim fills the gap to where equal temperament would put
//! it (the ticks outside the rim). The labels give each note's interval
//! above C: a just ratio where it is one, cents otherwise. On the Tuning
//! page the notes can be dragged, which makes the temperament a custom one.

use egui::{Align2, Color32, CursorIcon, FontId, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Vec2};
use piano_core::params::{PianoModel, TemperamentLaw};
use piano_params::{PianoControl, Value};

use crate::sketches::{HIGHLIGHT, Part};

const NAMES: [&str; 12] = ["C", "C#", "D", "Eb", "E", "F", "F#", "G", "G#", "A", "Bb", "B"];
/// The custom temperament's range (cents from equal temperament).
const RANGE: f64 = 50.0;
/// How far from equal temperament a note lies.
const OFF: Color32 = Color32::from_rgb(208, 116, 36);
/// Reach of a note's handle (points).
const GRAB: f32 = 12.0;

/// Cents above C of each note class: 0, 100, 200 … in equal temperament.
pub fn above_c(t: &TemperamentLaw) -> [f64; 12] {
    let c = t.class_cents();
    std::array::from_fn(|i| 100.0 * i as f64 + c[i] - c[0])
}

/// The just ratio `cents` above the unison, when it is one made of
/// fifths and at most one major third (4/3, 32/27, 5/4, 45/32 …).
pub fn ratio(cents: f64) -> Option<(u64, u64)> {
    let mut best: Option<(u64, u64)> = None;
    for threes in -8i32..=8 {
        for fives in -1i32..=1 {
            let x = threes as f64 * 3f64.log2() + fives as f64 * 5f64.log2();
            let twos = -x.floor() as i32;
            if (1200.0 * (x + twos as f64) - cents).abs() > 1e-6 {
                continue;
            }
            let part = |base: u64, e: i32| (base.pow(e.max(0) as u32), base.pow((-e).max(0) as u32));
            let (a, b, c) = (part(2, twos), part(3, threes), part(5, fives));
            let found = (a.0 * b.0 * c.0, a.1 * b.1 * c.1);
            if best.is_none_or(|(n, d)| found.0 * found.1 < n * d) {
                best = Some(found);
            }
        }
    }
    best
}

/// "4/3", "192.2", "700".
fn interval(cents: f64) -> String {
    match ratio(cents) {
        Some((n, d)) => format!("{n}/{d}"),
        None => {
            let v = (cents * 10.0).round() / 10.0;
            if v.fract() == 0.0 { format!("{v:.0}") } else { format!("{v:.1}") }
        }
    }
}

/// `t` as a custom temperament laid on the same key, note class `pc`
/// moved to `above` cents above C (within the custom temperament's range
/// from equal temperament). The other notes keep their intervals to C.
pub fn retuned(t: &TemperamentLaw, pc: usize, above: f64) -> TemperamentLaw {
    let now = above_c(t);
    let mut off: [f64; 12] = std::array::from_fn(|i| now[i] - 100.0 * i as f64);
    off[pc] = ((above - 100.0 * pc as f64).clamp(-RANGE, RANGE) * 10.0).round() / 10.0;
    // The custom table counts from the key: its first entry is the key's
    // note.
    let key = t.key.rem_euclid(12) as usize;
    TemperamentLaw { kind: 8, key: key as i32, cents: std::array::from_fn(|j| off[(j + key) % 12]) }
}

/// Sends what differs between `from` and `to`.
fn send(control: &mut dyn PianoControl, from: &TemperamentLaw, to: &TemperamentLaw) {
    for i in 0..12 {
        if from.cents[i] != to.cents[i] {
            control.set(&format!("temperament.cents[{i}]"), Value::Float(to.cents[i]));
        }
    }
    if from.key != to.key {
        control.set("temperament.key", Value::Int(to.key as i64));
    }
    if from.kind != to.kind {
        control.set("temperament.kind", Value::Int(to.kind as i64));
    }
}

/// The wheel at `width`, lit for the temperament's parameters, its tonic
/// for the key it is laid on.
pub fn draw(ui: &mut Ui, model: &PianoModel, lit: &[Part], width: f32) -> Response {
    show(ui, model, lit, width, None)
}

/// The wheel with its notes to drag: a note moved makes the temperament a
/// custom one; double-click brings a note back to equal temperament.
pub fn edit(ui: &mut Ui, control: &mut dyn PianoControl, model: &PianoModel, lit: &[Part], width: f32) -> Response {
    show(ui, model, lit, width, Some(control))
}

/// The wheel's centre and radius in the rectangle it was given.
pub fn hub(rect: Rect) -> (Pos2, f32) {
    let r = (rect.width() / 2.0 - 54.0).clamp(40.0, 150.0);
    (Pos2::new(rect.center().x, rect.top() + r + 32.0), r)
}

/// Direction of a pitch `cents` above C: C at the top, clockwise.
pub fn dir(cents: f64) -> Vec2 {
    Vec2::angled((cents / 1200.0 * 360.0 - 90.0).to_radians() as f32)
}

fn show(ui: &mut Ui, model: &PianoModel, lit: &[Part], width: f32, control: Option<&mut dyn PianoControl>) -> Response {
    let t = &model.temperament;
    let above = above_c(t);
    let editable = control.is_some();
    let r = hub(Rect::from_min_size(Pos2::ZERO, Vec2::new(width, 0.0))).1;
    let caption_h = if editable { 50.0 } else { 36.0 };
    let sense = if editable { Sense::click_and_drag() } else { Sense::hover() };
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, 2.0 * r + 64.0 + caption_h), sense);
    let c = hub(rect).0;
    let handle = |pc: usize| c + r * dir(above[pc]);

    // The note under the pointer (C is the reference: it stays), the one
    // being dragged.
    let drag_id = resp.id.with("note");
    let mut dragged: Option<usize> = ui.data(|d| d.get_temp(drag_id));
    let near = |pos: Pos2| (1..12).map(|pc| (pc, handle(pc).distance(pos))).filter(|&(_, d)| d < GRAB).min_by(|a, b| a.1.total_cmp(&b.1)).map(|(pc, _)| pc);
    let hovered = if editable { resp.hover_pos().and_then(near) } else { None };
    if let Some(control) = control {
        if resp.drag_started() {
            // Where the button went down: the drag is only told from a
            // click 6 points further, by then maybe out of the handle's
            // reach.
            dragged = ui.input(|i| i.pointer.press_origin()).or(resp.interact_pointer_pos()).and_then(near);
            if let Some(pc) = dragged {
                control.begin_edit(&format!("temperament.cents[{pc}]"));
            }
        }
        if let (Some(pc), Some(pos)) = (dragged, resp.interact_pointer_pos().filter(|_| resp.dragged())) {
            let v = pos - c;
            if v.length() > 8.0 {
                let turn = (v.x.atan2(-v.y) as f64 / std::f64::consts::TAU).rem_euclid(1.0);
                // The nearest octave of the pointer's pitch to the note's.
                let from_equal = (turn * 1200.0 - 100.0 * pc as f64 + 600.0).rem_euclid(1200.0) - 600.0;
                send(control, t, &retuned(t, pc, 100.0 * pc as f64 + from_equal));
            }
        }
        if resp.drag_stopped()
            && let Some(pc) = dragged.take()
        {
            control.end_edit(&format!("temperament.cents[{pc}]"));
        }
        if resp.double_clicked()
            && let Some(pc) = hovered
        {
            send(control, t, &retuned(t, pc, 100.0 * pc as f64));
        }
        ui.data_mut(|d| match dragged {
            Some(pc) => {
                d.insert_temp(drag_id, pc);
            }
            None => d.remove::<usize>(drag_id),
        });
        if dragged.is_some() {
            ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
        } else if hovered.is_some() {
            ui.ctx().set_cursor_icon(CursorIcon::Grab);
        }
    }
    if !ui.is_rect_visible(rect) {
        return resp;
    }

    let p = ui.painter_at(rect);
    let vis = ui.visuals();
    let (ink, weak) = (vis.text_color(), vis.weak_text_color());
    let font = FontId::proportional(11.5);
    let small = FontId::proportional(10.0);
    let lit_notes = lit.contains(&Part::Notes);

    // The octave.
    p.circle_filled(c, r, ink.gamma_multiply(0.07));
    p.circle_stroke(c, r, if lit_notes { Stroke::new(2.0, HIGHLIGHT) } else { Stroke::new(1.0, weak) });

    let tonic = t.key.rem_euclid(12) as usize;
    let unequal = (1..=8).contains(&t.kind);
    let band = (0.10 * r).max(8.0);
    for pc in 0..12 {
        let equal = 100.0 * pc as f64;
        let active = hovered == Some(pc) || dragged == Some(pc);
        // Where equal temperament puts the note.
        p.line_segment([c + r * dir(equal), c + (r + 5.0) * dir(equal)], Stroke::new(1.0, weak));
        // From there to the note.
        let off = above[pc] - equal;
        if off.abs() > 0.05 {
            let steps = 2 + (off.abs() / 8.0) as usize;
            let mid = r - 3.0 - band / 2.0;
            let arc: Vec<Pos2> = (0..=steps).map(|s| c + mid * dir(equal + off * s as f64 / steps as f64)).collect();
            p.add(Shape::line(arc, Stroke::new(band, OFF)));
        }
        // The note: its spoke, its handle on the rim.
        let at = handle(pc);
        p.line_segment([c, at], Stroke::new(if active { 1.5 } else { 1.0 }, if active { ink } else { weak }));
        if unequal && pc == tonic {
            p.circle_stroke(at, 7.0, if lit.contains(&Part::Tonic) { Stroke::new(2.0, HIGHLIGHT) } else { Stroke::new(1.0, ink) });
        }
        p.circle_filled(at, if active { 4.5 } else { 3.5 }, if active { HIGHLIGHT } else { vis.panel_fill });
        p.circle_stroke(at, if active { 4.5 } else { 3.5 }, Stroke::new(1.0, ink));
        // Its name and its interval above C, outside the rim.
        let d = dir(above[pc]);
        let label = at + Vec2::new(d.x * 30.0, d.y * 19.0);
        p.text(label - Vec2::new(0.0, 6.0), Align2::CENTER_CENTER, NAMES[pc], font.clone(), ink);
        p.text(label + Vec2::new(0.0, 6.5), Align2::CENTER_CENTER, if pc == 0 { "1/1".into() } else { interval(above[pc]) }, small.clone(), weak);
    }

    let name = TemperamentLaw::NAMES.get(t.kind as usize).filter(|_| t.kind >= 0).unwrap_or(&TemperamentLaw::NAMES[0]);
    let caption = if unequal { format!("{name} on {}", NAMES[tonic]) } else { name.to_string() };
    let top = c.y + r + 34.0;
    p.text(Pos2::new(c.x, top), Align2::CENTER_TOP, caption, font, if lit_notes { ink } else { weak });
    p.text(Pos2::new(c.x, top + 16.0), Align2::CENTER_TOP, "One turn is an octave; orange: how far from equal temperament", small.clone(), weak);
    if editable {
        p.text(Pos2::new(c.x, top + 30.0), Align2::CENTER_TOP, "Drag a note to retune it; double-click: back to equal", small, weak);
    }
    resp
}

#[cfg(test)]
mod tests {
    use super::*;

    fn law(kind: i32, key: i32) -> TemperamentLaw {
        TemperamentLaw { kind, key, ..TemperamentLaw::EQUAL }
    }

    #[test]
    fn notes_lie_at_their_intervals_above_c() {
        assert_eq!(above_c(&law(0, 0)), std::array::from_fn(|i| 100.0 * i as f64));
        // Pythagorean: every note a just ratio, by fifths from C.
        let p = above_c(&law(7, 0)).map(ratio);
        assert_eq!((p[5], p[7], p[3], p[2], p[6]), (Some((4, 3)), Some((3, 2)), Some((32, 27)), Some((9, 8)), Some((729, 512))));
        // Quarter-comma meantone: C–E pure, D half-way (no ratio).
        let m = above_c(&law(5, 0));
        assert_eq!(ratio(m[4]), Some((5, 4)));
        assert_eq!(ratio(m[2]), None);
        assert!((m[2] - 193.157).abs() < 0.01, "{}", m[2]);
        // Kirnberger III: F# a pure third and two pure fifths above C.
        assert_eq!(ratio(above_c(&law(2, 0))[6]), Some((45, 32)));
        // Werckmeister III, as shown.
        let w = above_c(&law(1, 0));
        assert_eq!((interval(w[2]), interval(w[5]), interval(w[7]), interval(w[1])), ("192.2".into(), "4/3".into(), "696.1".into(), "256/243".into()));
        assert_eq!(interval(700.0), "700");
    }

    #[test]
    fn a_dragged_note_moves_alone() {
        for from in [law(1, 0), law(5, 2), TemperamentLaw { kind: 8, key: 3, cents: std::array::from_fn(|i| i as f64 - 4.0) }] {
            let (was, to) = (above_c(&from), retuned(&from, 4, 386.3));
            let now = above_c(&to);
            assert_eq!((to.kind, to.key), (8, from.key));
            for pc in 0..12 {
                let want = if pc == 4 { 386.3 } else { was[pc] };
                assert!((now[pc] - want).abs() < 1e-9, "{pc}: {} vs {want}", now[pc]);
            }
        }
        // Within the custom temperament's range.
        assert_eq!(retuned(&law(0, 0), 4, 300.0).cents[4], -50.0);
        // Laid on D: E is the table's third entry.
        assert_eq!(retuned(&law(0, 2), 4, 300.0).cents[2], -50.0);
    }
}
