//! A parameter row (slider with edit gestures, reset, live/rebuild mark) and
//! a per-key curve.

use egui::{Color32, Pos2, RichText, Sense, Stroke, Ui, Vec2};
use piano_core::params::PianoModel;
use piano_params::{Kind, Param, PianoControl, Value};

use crate::sections::Curve;

/// One grid row: label, value, reset, kind. Edits go to `control` as a
/// gesture (a drag is one begin/end pair; typed values are a whole one).
/// Returns the label's response (clicked: show the parameter's help).
pub fn param_row(ui: &mut Ui, p: &Param, value: Option<Value>, control: &mut dyn PianoControl) -> egui::Response {
    let m = &p.meta;
    let modified = value.is_some_and(|v| v != p.default);
    let text = RichText::new(&m.label);
    let label = ui.add(egui::Label::new(if modified { text.color(ui.visuals().strong_text_color()) } else { text }).sense(Sense::click()));
    let label = label.on_hover_ui(|ui| {
        ui.set_max_width(340.0);
        if let Some(h) = piano_params::help(&p.path) {
            ui.label(h.what);
            ui.add_space(4.0);
        }
        ui.label(RichText::new(&p.path).monospace().weak());
        ui.label(RichText::new(format!("Default: {}", show(p.default.as_f64()))).weak());
        if !m.curated {
            ui.label(RichText::new("Range guessed from the default value").weak());
        }
        ui.label(RichText::new("Click for details").small().weak());
    });
    let Some(mut v) = value else {
        ui.label(RichText::new("off in this model").weak());
        for _ in 0..3 {
            ui.label("");
        }
        return label;
    };
    let r = match &mut v {
        Value::Float(x) => ui.add(
            egui::Slider::new(x, m.min..=m.max)
                .logarithmic(m.log)
                .clamping(egui::SliderClamping::Never)
                .suffix(if m.unit.is_empty() { String::new() } else { format!(" {}", m.unit) })
                .custom_formatter(|v, _| show(v)),
        ),
        Value::Int(x) => ui.add(
            egui::Slider::new(x, m.min as i64..=m.max as i64)
                .clamping(egui::SliderClamping::Never)
                .suffix(if m.unit.is_empty() { String::new() } else { format!(" {}", m.unit) }),
        ),
        Value::Bool(x) => ui.checkbox(x, ""),
    };
    if r.drag_started() {
        control.begin_edit(&p.path);
    }
    if r.changed() {
        if r.dragged() || r.drag_started() || r.drag_stopped() {
            control.set(&p.path, v);
        } else {
            control.begin_edit(&p.path);
            control.set(&p.path, v);
            control.end_edit(&p.path);
        }
    }
    if r.drag_stopped() {
        control.end_edit(&p.path);
    }
    let reset = reset_button(ui, v != p.default).on_hover_text("Back to the default");
    if reset.clicked() {
        control.begin_edit(&p.path);
        control.set(&p.path, p.default);
        control.end_edit(&p.path);
    }
    kind_mark(ui, matches!(p.kind, Kind::Live(_)));
    midi_button(ui, p, control);
    label
}

/// The help of a parameter: where it acts, what it does on the piano, then
/// how the model implements it.
pub fn about(ui: &mut Ui, p: &Param, value: Option<Value>, model: &PianoModel, key: &mut u8) {
    ui.horizontal(|ui| {
        kind_mark(ui, matches!(p.kind, Kind::Live(_)));
        ui.heading(&p.meta.label);
    });
    ui.label(RichText::new(&p.path).monospace().weak());
    let unit = if p.meta.unit.is_empty() { String::new() } else { format!(" {}", p.meta.unit) };
    let now = value.map_or("off in this model".into(), |v| format!("{}{unit}", show(v.as_f64())));
    ui.label(RichText::new(format!("{now} (default {}{unit})", show(p.default.as_f64()))).weak());
    ui.add_space(8.0);
    if let Some((sketch, _)) = crate::sketches::locate(&p.path) {
        crate::sketches::draw(ui, sketch, model, &p.path, key, ui.available_width().min(440.0));
        ui.add_space(8.0);
    }
    let Some(h) = piano_params::help(&p.path) else { return };
    ui.label(h.what);
    ui.add_space(8.0);
    egui::CollapsingHeader::new("Under the hood").id_salt("about-how").default_open(true).show(ui, |ui| {
        ui.label(h.how);
    });
}

/// The controller mapped to `p` ("CC 21"); click to learn one (again to
/// cancel), right-click to remove it.
fn midi_button(ui: &mut Ui, p: &Param, control: &mut dyn PianoControl) {
    let learnable = control.midi_learnable(&p.path);
    let Some(learn) = control.midi().filter(|_| learnable) else {
        ui.label("");
        return;
    };
    let cc = learn.map().for_path(&p.path).map(|m| m.cc);
    let waiting = learn.learning() == Some(p.path.as_str());
    let text = match (waiting, cc) {
        (true, _) => RichText::new("…").color(LIVE),
        (false, Some(cc)) => RichText::new(format!("CC {cc}")),
        (false, None) => RichText::new("MIDI").weak(),
    };
    let r = ui.add(egui::Button::new(text.small()).selected(waiting).frame(waiting || cc.is_some()));
    let r = r.on_hover_text(match cc {
        Some(_) => "Click to learn another controller, right-click to remove",
        None => "Click, then move a knob or fader over part of its travel",
    });
    if r.clicked() {
        if waiting { learn.cancel() } else { learn.learn(&p.path) }
    }
    if cc.is_some() {
        r.context_menu(|ui| {
            if ui.button("Remove the controller").clicked() {
                learn.unmap(&p.path);
            }
        });
    }
}

/// A circular arrow, drawn rather than a glyph (the plugin's fonts lack
/// one); empty space of the same size when hidden, to keep the column.
fn reset_button(ui: &mut Ui, visible: bool) -> egui::Response {
    let size = Vec2::splat(ui.spacing().interact_size.y);
    let sense = if visible { Sense::click() } else { Sense::hover() };
    let (rect, resp) = ui.allocate_exact_size(size, sense);
    if !visible || !ui.is_rect_visible(rect) {
        return resp;
    }
    let style = ui.style().interact(&resp);
    if resp.hovered() {
        ui.painter().rect_filled(rect, 3.0, style.bg_fill);
    }
    let stroke = Stroke::new(1.5, style.fg_stroke.color);
    let (c, r) = (rect.center(), 0.28 * rect.height());
    // ↺: three quarters of a circle turning anticlockwise, from the left
    // through the bottom and the right, up to the top.
    let (from, to) = (std::f32::consts::PI, -std::f32::consts::FRAC_PI_2);
    let points: Vec<Pos2> = (0..=24)
        .map(|i| from + (to - from) * i as f32 / 24.0)
        .map(|a| c + r * Vec2::angled(a))
        .collect();
    ui.painter().add(egui::Shape::line(points, stroke));
    // Arrowhead at the end (the top), pointing left along the arc.
    let tip = c + r * Vec2::angled(to);
    let h = 0.55 * r;
    ui.painter().add(egui::Shape::convex_polygon(
        vec![tip + Vec2::new(-h, 0.0), tip + Vec2::new(0.4 * h, -0.8 * h), tip + Vec2::new(0.4 * h, 0.8 * h)],
        style.fg_stroke.color,
        Stroke::NONE,
    ));
    resp
}

/// A small heart, drawn (the fonts lack one), in Ko-fi's red.
pub fn heart(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(12.0), Sense::hover());
    let (c, p) = (rect.center(), ui.painter());
    let red = Color32::from_rgb(255, 94, 91);
    p.circle_filled(c + Vec2::new(-2.6, -1.6), 2.9, red);
    p.circle_filled(c + Vec2::new(2.6, -1.6), 2.9, red);
    p.add(egui::Shape::convex_polygon(vec![c + Vec2::new(-5.3, -0.6), c + Vec2::new(5.3, -0.6), c + Vec2::new(0.0, 5.2)], red, Stroke::NONE));
}

/// Filled green dot: applied while playing; hollow: rebuilds the instrument.
pub fn kind_mark(ui: &mut Ui, live: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(12.0), Sense::hover());
    let c = rect.center();
    if live {
        ui.painter().circle_filled(c, 4.0, LIVE);
    } else {
        ui.painter().circle_stroke(c, 3.5, Stroke::new(1.0, ui.visuals().weak_text_color()));
    }
    resp.on_hover_text(if live { "Applied while playing" } else { "Rebuilds the instrument" })
}

pub const LIVE: Color32 = Color32::from_rgb(90, 190, 110);

/// Four significant digits; scientific outside 1e-3…1e5.
pub fn show(v: f64) -> String {
    let a = v.abs();
    if v == 0.0 {
        "0".into()
    } else if !(1e-3..1e5).contains(&a) {
        format!("{v:.3e}")
    } else {
        let decimals = (3 - a.log10().floor() as i32).max(0) as usize;
        let s = format!("{v:.decimals$}");
        if s.contains('.') { s.trim_end_matches('0').trim_end_matches('.').into() } else { s }
    }
}

pub fn note_name(n: u8) -> String {
    const NAMES: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
    format!("{}{}", NAMES[n as usize % 12], n as i32 / 12 - 1)
}

/// The law of `model` across its x range, over the default model's (faint).
pub fn curve(ui: &mut Ui, c: &Curve, model: &PianoModel, reference: &PianoModel, width: f32) {
    let ys = |m: &PianoModel| -> Vec<f64> { (c.x.0..=c.x.1).map(|x| (c.f)(m, x)).collect() };
    let (cur, def) = (ys(model), ys(reference));
    let tf = |y: f64| if c.log { y.max(1e-300).log10() } else { y };
    let (lo, hi) = cur.iter().chain(&def).map(|&y| tf(y)).filter(|y| y.is_finite()).fold((f64::MAX, f64::MIN), |(lo, hi), y| (lo.min(y), hi.max(y)));
    let (lo, hi) = if hi - lo < 1e-12 { (lo - 0.5, hi + 0.5) } else { (lo, hi) };

    ui.vertical(|ui| {
        ui.label(RichText::new(if c.unit.is_empty() { c.title.to_string() } else { format!("{} ({})", c.title, c.unit) }).small());
        let (resp, painter) = ui.allocate_painter(Vec2::new(width, 110.0), Sense::hover());
        let rect = resp.rect;
        let vis = ui.visuals();
        painter.rect_stroke(rect, 2.0, Stroke::new(1.0, vis.widgets.noninteractive.bg_stroke.color), egui::StrokeKind::Inside);
        let inner = rect.shrink(4.0);
        let n = cur.len().max(2) - 1;
        let pt = |i: usize, y: f64| {
            let fx = i as f32 / n as f32;
            let fy = ((tf(y) - lo) / (hi - lo)) as f32;
            Pos2::new(inner.left() + fx * inner.width(), inner.bottom() - fy * inner.height())
        };
        let line = |ys: &[f64]| ys.iter().enumerate().map(|(i, &y)| pt(i, y)).filter(|p| p.y.is_finite()).collect::<Vec<_>>();
        painter.add(egui::Shape::line(line(&def), Stroke::new(1.0, vis.weak_text_color())));
        painter.add(egui::Shape::line(line(&cur), Stroke::new(1.5, vis.selection.stroke.color)));
        let font = egui::FontId::proportional(10.0);
        let txt = vis.weak_text_color();
        let back = |t: f64| if c.log { 10f64.powf(t) } else { t };
        painter.text(inner.left_top(), egui::Align2::LEFT_TOP, show(back(hi)), font.clone(), txt);
        painter.text(inner.left_bottom(), egui::Align2::LEFT_BOTTOM, show(back(lo)), font.clone(), txt);
        if let Some(pos) = resp.hover_pos() {
            let i = (((pos.x - inner.left()) / inner.width()).clamp(0.0, 1.0) * n as f32).round() as usize;
            let x = c.x.0 + i as u8;
            let p = pt(i, cur[i]);
            painter.line_segment([Pos2::new(p.x, inner.top()), Pos2::new(p.x, inner.bottom())], Stroke::new(1.0, txt));
            painter.circle_filled(p, 3.0, vis.selection.stroke.color);
            let xs = if c.x == (1, 127) { format!("vel {x}") } else { note_name(x) };
            let label = format!("{xs}: {}", show(cur[i]));
            let anchor = if p.x > inner.center().x { egui::Align2::RIGHT_TOP } else { egui::Align2::LEFT_TOP };
            painter.text(Pos2::new(p.x + if p.x > inner.center().x { -4.0 } else { 4.0 }, inner.top()), anchor, label, font, vis.text_color());
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formatting() {
        assert_eq!(show(0.012), "0.012");
        assert_eq!(show(440.0), "440");
        assert_eq!(show(1.0e-7), "1.000e-7");
        assert_eq!(show(-3.708095), "-3.708");
        assert_eq!(note_name(60), "C4");
        assert_eq!(note_name(21), "A0");
    }
}
