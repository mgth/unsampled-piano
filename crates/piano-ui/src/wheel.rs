//! The temperament wheel: the circle of fifths, C at the top and the sharps
//! clockwise. The outer ring is the twelve fifths, each between its two
//! notes, coloured by how far it lies from a pure fifth (3:2); the inner
//! dots are the major thirds (5:4) of each key. Green is pure, yellow the
//! equal temperament's thirds, red a wolf. Values in cents from pure.

use egui::{Align2, Color32, FontId, Pos2, Response, Sense, Shape, Stroke, Ui, Vec2};
use piano_core::params::{PianoModel, TemperamentLaw};

use crate::sketches::{HIGHLIGHT, Part};

/// The note classes in fifths from C (pitch class of each).
const CIRCLE: [usize; 12] = [0, 7, 2, 9, 4, 11, 6, 1, 8, 3, 10, 5];
const NAMES: [&str; 12] = ["C", "G", "D", "A", "E", "B", "F#", "C#", "G#", "Eb", "Bb", "F"];

/// A pure fifth and a pure major third, in cents.
fn pure(ratio: f64) -> f64 {
    1200.0 * ratio.log2()
}

/// The fifth above each note of the circle (C–G, G–D … F–C), in cents
/// from pure: equal temperament −1.96, a meantone's wolf about +35.
pub fn fifths(t: &TemperamentLaw) -> [f64; 12] {
    let c = t.class_cents();
    std::array::from_fn(|i| 700.0 + c[(CIRCLE[i] + 7) % 12] - c[CIRCLE[i]] - pure(1.5))
}

/// The major third above each note of the circle, in cents from pure:
/// equal temperament +13.7, a meantone's 0.
pub fn thirds(t: &TemperamentLaw) -> [f64; 12] {
    let c = t.class_cents();
    std::array::from_fn(|i| 400.0 + c[(CIRCLE[i] + 4) % 12] - c[CIRCLE[i]] - pure(1.25))
}

/// Green when pure, through yellow and orange, red at `wolf` cents off.
fn shade(off: f64, wolf: f64) -> Color32 {
    let stops = [(0.0, (60, 170, 90)), (0.35, (200, 190, 60)), (0.65, (230, 140, 40)), (1.0, (215, 60, 50))];
    let x = (off.abs() / wolf).clamp(0.0, 1.0);
    let k = stops.windows(2).position(|w| x <= w[1].0).unwrap_or(2);
    let ((x0, a), (x1, b)) = (stops[k], stops[k + 1]);
    let t = ((x - x0) / (x1 - x0)) as f32;
    let m = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t) as u8;
    Color32::from_rgb(m(a.0, b.0), m(a.1, b.1), m(a.2, b.2))
}

/// "−5.4", "0", "+35.7" (cents, one decimal, the sign always).
fn cents(v: f64) -> String {
    let v = (v * 10.0).round() / 10.0;
    if v == 0.0 { "0".into() } else { format!("{}{:.1}", if v > 0.0 { "+" } else { "−" }, v.abs()) }
}

/// "−3", "0", "+14" (whole cents).
fn whole(v: f64) -> String {
    let v = v.round();
    if v == 0.0 { "0".into() } else { format!("{}{}", if v > 0.0 { "+" } else { "−" }, v.abs()) }
}

/// The wheel at `width`, the fifths lit for the temperament's parameters
/// and its tonic for the key it is laid on.
pub fn draw(ui: &mut Ui, model: &PianoModel, lit: &[Part], width: f32) -> Response {
    let t = &model.temperament;
    let (fifth, third) = (fifths(t), thirds(t));
    let size = width.min(340.0);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, size + 34.0), Sense::hover());
    if !ui.is_rect_visible(rect) {
        return resp;
    }
    let p = ui.painter_at(rect);
    let vis = ui.visuals();
    let (ink, weak) = (vis.text_color(), vis.weak_text_color());
    let font = FontId::proportional((size / 30.0).clamp(9.0, 12.0));
    let small = FontId::proportional((size / 36.0).clamp(8.5, 10.5));
    let c = Pos2::new(rect.center().x, rect.top() + size / 2.0);
    let h = size / 2.0;
    // Angle of note i of the circle: C at the top, clockwise.
    let dir = |i: f32| Vec2::angled((i * 30.0 - 90.0).to_radians());

    // The fifths: a band between each two notes.
    let (ring, band) = (0.70 * h, 0.15 * h);
    let lit_fifths = lit.contains(&Part::Fifths);
    if lit_fifths {
        p.circle_stroke(c, ring + band / 2.0 + 3.0, Stroke::new(2.0, HIGHLIGHT));
    }
    for (i, &off) in fifth.iter().enumerate() {
        let (a, b) = (i as f32 + 0.08, i as f32 + 0.92);
        let arc: Vec<Pos2> = (0..=16).map(|s| c + ring * dir(a + (b - a) * s as f32 / 16.0)).collect();
        p.add(Shape::line(arc, Stroke::new(band, shade(off, 24.0))));
        p.text(c + ring * dir(i as f32 + 0.5), Align2::CENTER_CENTER, cents(off), small.clone(), Color32::from_gray(25));
    }

    // The notes, the tonic marked.
    let tonic = t.key.rem_euclid(12) as usize;
    let unequal = (1..=8).contains(&t.kind);
    for (i, name) in NAMES.iter().enumerate() {
        let at = c + 0.90 * h * dir(i as f32);
        let is_tonic = unequal && CIRCLE[i] == tonic;
        if is_tonic {
            let ring = if lit.contains(&Part::Tonic) { Stroke::new(2.0, HIGHLIGHT) } else { Stroke::new(1.0, weak) };
            p.circle_stroke(at, 0.075 * h, ring);
        }
        p.text(at, Align2::CENTER_CENTER, *name, font.clone(), if is_tonic { ink } else { weak });
    }

    // The major thirds, one dot per key.
    for (i, &off) in third.iter().enumerate() {
        let at = c + 0.45 * h * dir(i as f32);
        p.circle_filled(at, 0.055 * h, shade(off, 30.0));
        p.text(c + 0.30 * h * dir(i as f32), Align2::CENTER_CENTER, whole(off), small.clone(), weak);
    }
    p.text(c, Align2::CENTER_CENTER, "3rds", small.clone(), weak);

    let name = TemperamentLaw::NAMES.get(t.kind as usize).filter(|_| t.kind >= 0).unwrap_or(&TemperamentLaw::NAMES[0]);
    let caption = if unequal { format!("{name} on {}", NAMES[CIRCLE.iter().position(|&pc| pc == tonic).unwrap_or(0)]) } else { name.to_string() };
    p.text(Pos2::new(c.x, rect.top() + size + 4.0), Align2::CENTER_TOP, caption, font.clone(), if lit_fifths { ink } else { weak });
    p.text(
        Pos2::new(c.x, rect.top() + size + 20.0),
        Align2::CENTER_TOP,
        "Outer ring: fifths; dots: major thirds; cents from pure",
        small,
        weak,
    );
    resp
}

#[cfg(test)]
mod tests {
    use super::*;

    fn law(kind: i32, key: i32) -> TemperamentLaw {
        TemperamentLaw { kind, key, ..TemperamentLaw::EQUAL }
    }

    #[test]
    fn the_circle_closes_on_the_pythagorean_comma() {
        let comma = 1200.0 * (3f64.powi(12) / 2f64.powi(19)).log2();
        for kind in 0..=7 {
            let sum: f64 = fifths(&law(kind, 0)).iter().sum();
            assert!((sum + comma).abs() < 1e-9, "{kind}: {sum}");
        }
    }

    #[test]
    fn fifths_and_thirds_of_known_temperaments() {
        let equal = (fifths(&law(0, 0)), thirds(&law(0, 0)));
        assert!(equal.0.iter().all(|f| (f + 1.955).abs() < 0.01) && equal.1.iter().all(|t| (t - 13.686).abs() < 0.01));
        // Quarter-comma meantone: pure thirds in the usual keys, the wolf
        // between G# and Eb.
        let (f, t) = (fifths(&law(5, 0)), thirds(&law(5, 0)));
        assert!(t[0].abs() < 0.01, "C–E {}", t[0]);
        assert!((f[8] - 35.7).abs() < 0.1, "G#–Eb {}", f[8]);
        // Laid on D, the wolf moves two fifths on.
        assert!((fifths(&law(5, 2))[10] - 35.7).abs() < 0.1);
        // Pythagorean: eleven pure fifths.
        assert_eq!(fifths(&law(7, 0)).iter().filter(|f| f.abs() < 1e-9).count(), 11);
    }
}
