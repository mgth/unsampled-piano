//! Sketches locating the parameters on the instrument, drawn from the model
//! (felt depth, pedal curve, una corda shift follow the settings).
//!
//! The action: one key of a grand seen from the side, the player on the
//! left. The key lifts the hammer onto the string from below and, at its
//! back end, the damper off the string; the sustain pedal lifts every
//! damper through the damper tray. Not to scale.

use egui::{Align2, Color32, FontId, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Vec2};
use piano_core::params::PianoModel;

/// A part of the action a parameter acts on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    /// The key press setting the hammer's speed.
    Key,
    /// The hammer's top felt.
    Felt,
    /// The hammer's hard core.
    Core,
    /// The damper on the string, and its motion.
    Damper,
    SustainPedal,
    UnaCorda,
    /// The key bottoming on the keybed.
    KeybedKnock,
    /// The damper landing on the ringing string.
    DamperLanding,
    /// The damper tray knocking when the pedal moves.
    PedalKnock,
}

/// Where a parameter acts in the action (empty: not in the action).
pub fn parts_of(path: &str) -> &'static [Part] {
    use Part::*;
    const TABLE: &[(&str, &[Part])] = &[
        ("velocity.", &[Key]),
        ("hardness", &[Felt]),
        ("p_bass", &[Felt]),
        ("p_offset", &[Felt]),
        ("hysteresis", &[Felt]),
        ("contact_width_scale", &[Felt]),
        ("hammer_mass_scale", &[Felt]),
        ("core_", &[Core]),
        ("una_corda_softening", &[UnaCorda]),
        ("damper.pedal_", &[SustainPedal]),
        ("damper.", &[Damper]),
        ("noise.keybed", &[KeybedKnock]),
        ("noise.damper_", &[DamperLanding]),
        ("noise.pedal", &[PedalKnock]),
        ("noise.knock_time", &[KeybedKnock, PedalKnock]),
    ];
    // The first match wins: "damper.pedal_" before "damper.".
    TABLE.iter().find(|(p, _)| path.starts_with(p)).map_or(&[], |(_, parts)| parts)
}

/// Design space of the action drawing.
const W: f32 = 440.0;
const H: f32 = 254.0;

const STRING_Y: f32 = 30.0;
const STRIKE_X: f32 = 150.0;
const DAMPER_X: f32 = 300.0;
const KEY_Y: f32 = 130.0;
const KEYBED_Y: f32 = 150.0;
const TRAY_Y: f32 = 116.0;
const ROD_X: f32 = 355.0;
const PEDAL_Y: f32 = 176.0;

const HIGHLIGHT: Color32 = Color32::from_rgb(239, 159, 39);
const FELT: Color32 = Color32::from_rgb(222, 212, 188);
const CORE: Color32 = Color32::from_rgb(140, 120, 100);
const WOOD: Color32 = Color32::from_rgb(186, 117, 23);

/// The action at `width`, `lit` parts highlighted; `path` picks the key
/// whose felt depth is drawn (bass, middle or treble).
pub fn action(ui: &mut Ui, model: &PianoModel, lit: &[Part], path: &str, width: f32) -> Response {
    let k = width / W;
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(W, H) * k, Sense::hover());
    if !ui.is_rect_visible(rect) {
        return resp;
    }
    let p = ui.painter_at(rect);
    let at = |x: f32, y: f32| rect.min + Vec2::new(x, y) * k;
    let r = |x0: f32, y0: f32, x1: f32, y1: f32| Rect::from_two_pos(at(x0, y0), at(x1, y1));
    let vis = ui.visuals();
    let (ink, weak) = (vis.text_color(), vis.weak_text_color());
    let font = FontId::proportional((10.0 * k).max(9.0));
    let lit = |part: Part| lit.contains(&part);
    let line = |w: f32, c: Color32| Stroke::new(w * k.max(0.6), c);
    let outline = |part: Part| if lit(part) { line(2.0, HIGHLIGHT) } else { line(1.0, weak) };
    let fill = |part: Part, base: Color32| if lit(part) { HIGHLIGHT.gamma_multiply(0.55) } else { base };
    let label = |x: f32, y: f32, align: Align2, text: &str, part: Option<Part>| {
        let c = if part.is_some_and(lit) { ink } else { weak };
        p.text(at(x, y), align, text, font.clone(), c);
    };

    // Frame and string, from the agraffe towards the bridge.
    p.add(Shape::convex_polygon(vec![at(54.0, 18.0), at(66.0, 18.0), at(60.0, STRING_Y)], weak, Stroke::NONE));
    p.line_segment([at(60.0, STRING_Y), at(W - 4.0, STRING_Y)], line(2.0, ink));
    label(W - 4.0, STRING_Y + 3.0, Align2::RIGHT_TOP, "String, to the bridge", None);
    label(62.0, 16.0, Align2::CENTER_BOTTOM, "Agraffe", None);

    // Keybed, key, balance point.
    p.rect_filled(r(10.0, KEYBED_Y, W - 10.0, KEYBED_Y + 10.0), 1.0, vis.widgets.noninteractive.bg_fill);
    p.rect_stroke(r(10.0, KEYBED_Y, W - 10.0, KEYBED_Y + 10.0), 1.0, line(1.0, weak), egui::StrokeKind::Inside);
    label(12.0, KEYBED_Y + 12.0, Align2::LEFT_TOP, "Keybed", Some(Part::KeybedKnock));
    p.rect_filled(r(20.0, KEY_Y, 330.0, KEY_Y + 12.0), 1.0, WOOD.gamma_multiply(0.5));
    p.rect_filled(r(20.0, KEY_Y - 3.0, 75.0, KEY_Y + 12.0), 1.0, fill(Part::Key, Color32::from_gray(235)));
    p.rect_stroke(r(20.0, KEY_Y - 3.0, 75.0, KEY_Y + 12.0), 1.0, outline(Part::Key), egui::StrokeKind::Inside);
    p.add(Shape::convex_polygon(vec![at(185.0, KEY_Y + 12.0), at(193.0, KEYBED_Y), at(177.0, KEYBED_Y)], weak, Stroke::NONE));
    label(110.0, KEY_Y - 2.0, Align2::LEFT_BOTTOM, "Key", Some(Part::Key));
    // The finger, and the hammer's flight it sets.
    arrow(&p, at(47.0, 98.0), at(47.0, KEY_Y - 6.0), outline(Part::Key));
    label(56.0, 100.0, Align2::LEFT_TOP, "Velocity", Some(Part::Key));
    arrow(&p, at(STRIKE_X - 24.0, 84.0), at(STRIKE_X - 24.0, 46.0), outline(Part::Key));
    if lit(Part::Key) {
        let v = &model.velocity;
        let text = format!("{:.1}–{:.1} m/s", v.vmin, v.vmax);
        label(STRIKE_X - 28.0, 66.0, Align2::RIGHT_CENTER, &text, Some(Part::Key));
    }

    // Hammer: shank from its flange, wooden molding, core, top felt (its
    // depth exaggerated, from the model).
    p.line_segment([at(STRIKE_X, 80.0), at(255.0, 88.0)], line(3.0, WOOD));
    p.circle_stroke(at(255.0, 88.0), 3.5 * k, line(1.0, weak));
    p.line_segment([at(212.0, 85.0), at(212.0, KEY_Y)], line(2.0, weak));
    let (depth, key) = felt_depth(model, path);
    let t = (depth as f32 * 8.0).clamp(1.5, 10.0);
    let (top, bottom, rx) = (STRING_Y + 2.0, 76.0, 15.0);
    p.rect_filled(r(STRIKE_X - 11.0, 72.0, STRIKE_X + 11.0, 84.0), 1.0, WOOD);
    let felt = felt_shade(model.hardness);
    p.add(ellipse(at(STRIKE_X, (top + bottom) / 2.0), rx * k, (bottom - top) / 2.0 * k, fill(Part::Felt, felt), outline(Part::Felt)));
    let core_top = top + t;
    p.add(ellipse(
        at(STRIKE_X, (core_top + bottom) / 2.0),
        (rx - 0.45 * t) * k,
        (bottom - core_top) / 2.0 * k,
        fill(Part::Core, CORE),
        outline(Part::Core),
    ));
    label(STRIKE_X + 20.0, 46.0, Align2::LEFT_CENTER, "Hammer", Some(Part::Felt));
    if lit(Part::Felt) || lit(Part::Core) {
        let text = format!("Top felt {depth:.2} mm ({key})");
        label(STRIKE_X + 20.0, 60.0, Align2::LEFT_CENTER, &text, Some(Part::Core));
    }

    // Damper: felt on the string, wire, lever lifted by the back of the key,
    // tray lifted by the pedal rod.
    let damper = r(DAMPER_X - 11.0, 12.0, DAMPER_X + 11.0, STRING_Y);
    p.rect_filled(r(DAMPER_X - 9.0, 6.0, DAMPER_X + 9.0, 12.0), 1.0, WOOD);
    p.rect_filled(damper, 1.0, fill(Part::Damper, FELT));
    p.rect_stroke(damper, 1.0, outline(Part::Damper), egui::StrokeKind::Inside);
    p.line_segment([at(DAMPER_X, STRING_Y), at(DAMPER_X, 112.0)], line(1.0, weak));
    p.line_segment([at(268.0, 112.0), at(345.0, 112.0)], line(2.0, weak));
    p.circle_stroke(at(345.0, 112.0), 3.0 * k, line(1.0, weak));
    p.line_segment([at(320.0, KEY_Y), at(320.0, 114.0)], line(1.5, weak));
    label(DAMPER_X + 14.0, 20.0, Align2::LEFT_CENTER, "Damper", Some(Part::Damper));
    if lit(Part::Damper) {
        let d = &model.damper;
        let text = format!("falls in {:.0} ms, lifts in {:.0} ms", 1e3 * d.fall_time, 1e3 * d.lift_time);
        label(DAMPER_X + 14.0, 44.0, Align2::LEFT_CENTER, &text, Some(Part::Damper));
        arrow(&p, at(DAMPER_X - 18.0, 2.0), at(DAMPER_X - 18.0, 24.0), line(1.5, HIGHLIGHT));
    }
    let tray = r(262.0, TRAY_Y, ROD_X + 4.0, TRAY_Y + 4.0);
    p.rect_filled(tray, 1.0, fill(Part::SustainPedal, weak));
    p.line_segment([at(ROD_X, TRAY_Y + 4.0), at(ROD_X, PEDAL_Y)], outline(Part::SustainPedal));
    let pedal = r(ROD_X - 40.0, PEDAL_Y, ROD_X + 30.0, PEDAL_Y + 6.0);
    p.rect_filled(pedal, 2.0, fill(Part::SustainPedal, weak));
    label(ROD_X - 44.0, PEDAL_Y + 3.0, Align2::RIGHT_CENTER, "Sustain pedal", Some(Part::SustainPedal));
    label(ROD_X + 8.0, TRAY_Y - 6.0, Align2::LEFT_CENTER, "Damper tray", Some(Part::SustainPedal));

    // Noises.
    knock(&p, at(36.0, KEYBED_Y), 7.0 * k, lit(Part::KeybedKnock), weak);
    knock(&p, at(DAMPER_X, STRING_Y), 7.0 * k, lit(Part::DamperLanding), weak);
    knock(&p, at(290.0, TRAY_Y + 2.0), 7.0 * k, lit(Part::PedalKnock), weak);

    // Below: the una corda from above, and the sustain pedal's travel.
    let y0 = 196.0;
    label(20.0, y0, Align2::LEFT_TOP, "Una corda, from above", Some(Part::UnaCorda));
    for (i, sy) in [214.0, 222.0, 230.0].into_iter().enumerate() {
        let c = if i == 0 && lit(Part::UnaCorda) { HIGHLIGHT } else { ink };
        p.line_segment([at(24.0, sy), at(170.0, sy)], line(1.0, c));
    }
    p.rect_stroke(r(80.0, 209.0, 96.0, 235.0), 2.0, line(1.0, weak), egui::StrokeKind::Inside);
    p.rect_filled(r(104.0, 217.0, 120.0, 243.0), 2.0, fill(Part::UnaCorda, felt));
    arrow(&p, at(97.0, 222.0), at(103.0, 230.0), line(1.0, weak));
    if lit(Part::UnaCorda) {
        let text = format!("softer by {:.0} %", 100.0 * model.una_corda_softening);
        label(126.0, 238.0, Align2::LEFT_CENTER, &text, Some(Part::UnaCorda));
    }
    pedal_travel(&p, r(240.0, 213.0, W - 14.0, 237.0), model, lit(Part::SustainPedal), ink, weak, &font);
    label(240.0, y0, Align2::LEFT_TOP, "Damper contact by pedal travel", Some(Part::SustainPedal));
    resp
}

/// Top-felt depth (mm) shown for a parameter, and the key it is at.
fn felt_depth(model: &PianoModel, path: &str) -> (f64, &'static str) {
    match path {
        "core_depth_bass" => (model.core_depth_bass, "A0"),
        "core_depth_treble" => (model.core_depth_treble, "C8"),
        _ => (model.core_depth_mid, "C4"),
    }
}

/// Lighter felt for softer voicing (hardness on a log scale around 0.3).
fn felt_shade(hardness: f64) -> Color32 {
    let t = ((hardness.max(1e-3) / 0.3).log10() / 1.2).clamp(-1.0, 1.0) as f32;
    let mix = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * (t + 1.0) / 2.0) as u8;
    Color32::from_rgb(mix(245, 190), mix(238, 170), mix(220, 140))
}

/// Damper contact against pedal travel (0 → 1), the half-pedal zone shaded.
fn pedal_travel(p: &egui::Painter, rect: Rect, model: &PianoModel, lit: bool, ink: Color32, weak: Color32, font: &FontId) {
    let d = &model.damper;
    let x = |t: f64| rect.left() + t as f32 * rect.width();
    let half = Rect::from_x_y_ranges(x(d.pedal_free)..=x(d.pedal_contact), rect.y_range());
    p.rect_filled(half, 0.0, if lit { HIGHLIGHT.gamma_multiply(0.3) } else { weak.gamma_multiply(0.15) });
    p.rect_stroke(rect, 0.0, Stroke::new(1.0, weak), egui::StrokeKind::Inside);
    let pts: Vec<Pos2> = (0..=48)
        .map(|i| i as f64 / 48.0)
        .map(|t| Pos2::new(x(t), rect.bottom() - d.pedal_engagement(t) as f32 * rect.height()))
        .collect();
    p.add(Shape::line(pts, Stroke::new(1.5, if lit { HIGHLIGHT } else { ink })));
    p.text(rect.left_bottom() + Vec2::new(0.0, 2.0), Align2::LEFT_TOP, "up", font.clone(), weak);
    p.text(rect.right_bottom() + Vec2::new(0.0, 2.0), Align2::RIGHT_TOP, "down", font.clone(), weak);
}

fn ellipse(c: Pos2, rx: f32, ry: f32, fill: Color32, stroke: Stroke) -> Shape {
    let pts = (0..32).map(|i| i as f32 / 32.0 * std::f32::consts::TAU).map(|a| c + Vec2::new(rx * a.cos(), ry * a.sin())).collect();
    Shape::convex_polygon(pts, fill, stroke)
}

fn arrow(p: &egui::Painter, from: Pos2, to: Pos2, stroke: Stroke) {
    p.line_segment([from, to], stroke);
    let dir = (to - from).normalized();
    let side = Vec2::new(-dir.y, dir.x);
    let h = 3.0 + 2.0 * stroke.width;
    p.add(Shape::convex_polygon(vec![to, to - dir * h + side * h * 0.6, to - dir * h - side * h * 0.6], stroke.color, Stroke::NONE));
}

/// An impact: short rays around a point.
fn knock(p: &egui::Painter, c: Pos2, r: f32, lit: bool, weak: Color32) {
    let (color, w) = if lit { (HIGHLIGHT, 2.0) } else { (weak.gamma_multiply(0.6), 1.0) };
    for i in 0..8 {
        let a = i as f32 / 8.0 * std::f32::consts::TAU;
        let d = Vec2::angled(a);
        p.line_segment([c + d * 0.4 * r, c + d * r], Stroke::new(w, color));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parts_name_real_parameters() {
        let reg = piano_params::Registry::new();
        assert_eq!(parts_of("damper.pedal_free"), &[Part::SustainPedal]);
        assert_eq!(parts_of("damper.fall_time"), &[Part::Damper]);
        assert_eq!(parts_of("core_depth_mid"), &[Part::Core]);
        assert_eq!(parts_of("hardness_slope_bass"), &[Part::Felt]);
        assert!(parts_of("output_gain_db").is_empty());
        // Every part is reached by some parameter.
        for part in [
            Part::Key,
            Part::Felt,
            Part::Core,
            Part::Damper,
            Part::SustainPedal,
            Part::UnaCorda,
            Part::KeybedKnock,
            Part::DamperLanding,
            Part::PedalKnock,
        ] {
            assert!(reg.params().iter().any(|p| parts_of(&p.path).contains(&part)), "{part:?}");
        }
    }

    #[test]
    fn felt_shades_stay_in_range() {
        for h in [0.0, 0.02, 0.3, 2.0, 100.0] {
            let _ = felt_shade(h);
        }
    }
}
