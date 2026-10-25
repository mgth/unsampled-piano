//! The "Simple" page: the musician's controls (`[macros]`, crate
//! piano_core::macros), each a slider from −1 to +1 centred on the preset as
//! it is, between the words for its two ends. They move groups of the
//! advanced parameters on top of the preset's values, which the other pages
//! keep showing.

use egui::{RichText, Sense, Stroke, Ui};
use piano_params::{Kind, Param, PianoControl, Registry, Value};

use crate::widgets;

/// The controls by what they act on: the sound, the instrument, the
/// listening set-up.
const GROUPS: &[(&str, &[&str])] = &[
    ("Sound", &["tone", "touch", "sustain", "resonance", "release", "unisons", "mechanism"]),
    ("Instrument", &["size", "stretch"]),
    ("Listening", &["lid", "space"]),
];

/// What the page's controls did this frame (indices into the registry).
#[derive(Default)]
pub struct Shown {
    /// A name pointed at, or a slider pointed at or moved.
    pub hovered: Option<usize>,
    /// A name clicked (its help, again to close it).
    pub clicked: Option<usize>,
    /// A value moved or reset (its help opens).
    pub touched: Option<usize>,
}

/// "Tone (soft – bright)" → ("Tone", "soft", "bright"); a label without
/// its ends keeps them empty.
pub fn ends(label: &str) -> (&str, &str, &str) {
    let Some((name, rest)) = label.split_once(" (") else {
        return (label, "", "");
    };
    let rest = rest.trim_end_matches(')');
    let (low, high) = rest.split_once(" – ").unwrap_or((rest, ""));
    (name, low, high)
}

pub fn show(ui: &mut Ui, registry: &Registry, values: &[Option<Value>], control: &mut dyn PianoControl) -> Shown {
    let mut shown = Shown::default();
    let index = |name: &str| registry.params().iter().position(|p| p.path == format!("macros.{name}"));
    let moved: Vec<usize> =
        (0..registry.params().len()).filter(|&i| registry.params()[i].path.starts_with("macros.") && values[i].is_some_and(|v| v.as_f64() != 0.0)).collect();

    ui.label(
        RichText::new(
            "Each control moves a group of parameters together, on top of the preset: at 0 the preset plays as \
             it is. The other pages show the preset's own values, before these controls.",
        )
        .weak(),
    );
    ui.add_space(4.0);
    let all = ui.add_enabled(!moved.is_empty(), egui::Button::new("All back to the preset"));
    if all.on_hover_text("Every control back to 0").clicked() {
        for &i in &moved {
            widgets::set_once(&registry.params()[i].path, Value::Float(0.0), control);
        }
    }
    ui.add_space(8.0);

    egui::Grid::new("simple").num_columns(8).spacing([8.0, 6.0]).show(ui, |ui| {
        for (k, (title, names)) in GROUPS.iter().enumerate() {
            if k > 0 {
                ui.end_row();
            }
            ui.label(RichText::new(*title).strong());
            ui.end_row();
            for i in names.iter().filter_map(|n| index(n)) {
                let Some(Value::Float(v)) = values[i] else {
                    continue;
                };
                row(ui, i, &registry.params()[i], v, control, &mut shown);
                ui.end_row();
            }
        }
    });
    shown
}

/// Name, low end, slider, high end, value, reset, live mark, MIDI.
fn row(ui: &mut Ui, i: usize, p: &Param, mut v: f64, control: &mut dyn PianoControl, shown: &mut Shown) {
    let (name, low, high) = ends(&p.meta.label);
    let text = RichText::new(name);
    let label = ui.add(egui::Label::new(if v != 0.0 { text.color(ui.visuals().strong_text_color()) } else { text }).sense(Sense::click()));
    let label = label.on_hover_ui(|ui| {
        ui.set_max_width(340.0);
        if let Some(h) = piano_params::help(&p.path) {
            ui.label(h.what);
            ui.add_space(4.0);
        }
        ui.label(RichText::new(&p.path).monospace().weak());
        ui.label(RichText::new("Click for details").small().weak());
    });
    ui.label(RichText::new(low).weak());

    // The fill runs over the rail from the centre (the preset) to the
    // handle's edge.
    ui.spacing_mut().slider_width = 190.0;
    let r = ui.add(egui::Slider::new(&mut v, -1.0..=1.0).show_value(false).step_by(0.01));
    let rect = r.rect;
    let radius = rect.height() / 2.5;
    let x = |t: f64| rect.left() + radius + (rect.width() - 2.0 * radius) * ((t as f32 + 1.0) / 2.0);
    let y = rect.center().y;
    let to = x(v) - radius * 0.8 * (v as f32).signum();
    if (to - x(0.0)) * (v as f32) > 0.0 {
        ui.painter().line_segment([egui::pos2(x(0.0), y), egui::pos2(to, y)], Stroke::new(4.0, ui.visuals().selection.bg_fill));
    }
    let tick = ui.visuals().weak_text_color();
    ui.painter().line_segment([egui::pos2(x(0.0), y - 0.45 * rect.height()), egui::pos2(x(0.0), y - 0.25 * rect.height())], Stroke::new(1.0, tick));
    widgets::edit(&r, &p.path, Value::Float(v), control);
    let r = r.on_hover_text(format!("{low} ← preset → {high}; double-click: back to the preset"));
    // The slider senses drags only: a double click is read from the pointer.
    let double = ui.input(|i| i.pointer.button_double_clicked(egui::PointerButton::Primary));
    let reset_by_click = double && r.hovered() && v != 0.0;

    ui.label(high);
    ui.label(RichText::new(if v == 0.0 { "0".into() } else { format!("{v:+.2}") }).monospace().weak());
    let reset = widgets::reset_button(ui, v != 0.0).on_hover_text("Back to the preset (0)");
    if reset.clicked() || reset_by_click {
        widgets::set_once(&p.path, Value::Float(0.0), control);
    }
    widgets::kind_mark(ui, matches!(p.kind, Kind::Live(_)));
    widgets::midi_button(ui, p, control);

    if label.hovered() || r.hovered() || r.dragged() || r.has_focus() {
        shown.hovered = Some(i);
    }
    if label.clicked() {
        shown.clicked = Some(i);
    }
    if r.drag_started() || r.changed() || reset.clicked() || reset_by_click {
        shown.touched = Some(i);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_give_both_ends() {
        assert_eq!(ends("Tone (soft – bright)"), ("Tone", "soft", "bright"));
        assert_eq!(ends("Size (baby – concert grand)"), ("Size", "baby", "concert grand"));
        assert_eq!(ends("Plain"), ("Plain", "", ""));
    }

    /// Every control of the model is on the page, in one of its groups.
    #[test]
    fn every_control_has_its_row() {
        let reg = Registry::new();
        let listed: Vec<String> = GROUPS.iter().flat_map(|g| g.1.iter().map(|n| format!("macros.{n}"))).collect();
        let model: Vec<&str> = reg.params().iter().map(|p| p.path.as_str()).filter(|p| p.starts_with("macros.")).collect();
        assert_eq!(model.len(), listed.len());
        for p in model {
            assert!(listed.iter().any(|l| l == p), "{p} has no row on the Simple page");
            let (_, low, high) = ends(&reg.get(p).unwrap().meta.label);
            assert!(!low.is_empty() && !high.is_empty(), "{p}: its label names no ends");
        }
    }
}
