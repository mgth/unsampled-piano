//! The instrument's interface: pages of parameters with the per-key laws
//! they shape, presets, and the engine's state. It draws into any egui
//! [`Ui`] and acts through [`PianoControl`], so the standalone player and
//! the plugin wrappers only provide the window and the control.

mod mics;
pub mod mock;
pub mod sections;
pub mod shot;
mod sketches;
mod widgets;

use std::path::PathBuf;

use egui::{RichText, Ui};
use piano_core::params::PianoModel;
use piano_params::{PianoControl, Registry};

use sections::{section_of, View, SECTIONS};
pub use widgets::show as format_value;

/// Page index of the MIDI map (after the sections).
const MIDI_PAGE: usize = usize::MAX;

pub struct PianoUi {
    registry: Registry,
    /// Section of each registry parameter.
    section: Vec<usize>,
    page: usize,
    search: String,
    /// Parameter whose help is shown (registry index).
    about: Option<usize>,
    mics: mics::MicView,
    /// Parameter under the pointer (registry index), lighting its part in
    /// the page's sketch on the next frame.
    hovered: Option<usize>,
    /// Preset folder (TOML files), when the host has one: either flat, or
    /// `instruments/<family>/*.toml` and `listening/*.toml`.
    presets: Option<PathBuf>,
    preset_name: String,
    /// Last listening preset loaded in each listening group (Microphones,
    /// Room…): the groups combine, each shows its own choice.
    listening_names: std::collections::BTreeMap<String, String>,
    /// Result of the last preset action.
    message: Option<Result<String, String>>,
    /// CPU load shown: the peak over the last second (the status gives the
    /// peak since it was last read).
    load: LoadMeter,
}

#[derive(Default)]
struct LoadMeter {
    peak: f32,
    shown: f32,
    since: Option<std::time::Instant>,
}

impl LoadMeter {
    fn update(&mut self, load: f32) -> f32 {
        self.peak = self.peak.max(load);
        let now = std::time::Instant::now();
        if self.since.is_none_or(|t| now.duration_since(t).as_secs_f32() >= 1.0) {
            (self.shown, self.peak, self.since) = (self.peak, 0.0, Some(now));
        }
        self.shown
    }
}

impl PianoUi {
    pub fn new(presets: Option<PathBuf>) -> Self {
        let registry = Registry::new();
        let section = registry.params().iter().map(|p| section_of(&p.path)).collect();
        Self {
            registry,
            section,
            page: 0,
            search: String::new(),
            about: None,
            mics: mics::MicView::default(),
            hovered: None,
            presets,
            preset_name: String::new(),
            listening_names: Default::default(),
            message: None,
            load: LoadMeter::default(),
        }
    }

    /// The preset shown as loaded (its file name without `.toml`): in the
    /// menu of the listening group holding a file of that name, else as the
    /// instrument.
    pub fn set_preset_name(&mut self, name: &str) {
        if let Some(dir) = &self.presets {
            for group in subdirs(&dir.join("listening")) {
                if group.join(format!("{name}.toml")).is_file() {
                    let g = group.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                    self.listening_names.insert(g, name.into());
                    return;
                }
            }
        }
        self.preset_name = name.into();
    }

    /// Shows the help of the parameter at `path`, if it exists.
    pub fn show_help(&mut self, path: &str) {
        self.about = self.registry.params().iter().position(|p| p.path == path);
    }

    /// Opens the page with this title (case-insensitive), or "MIDI".
    pub fn open_page(&mut self, title: &str) {
        if title.eq_ignore_ascii_case("midi") {
            self.page = MIDI_PAGE;
            self.search.clear();
        } else if let Some(i) = SECTIONS.iter().position(|s| s.title.eq_ignore_ascii_case(title)) {
            self.page = i;
            self.search.clear();
        }
    }

    /// Draws the whole interface into `ui` (the window's root).
    pub fn show(&mut self, ui: &mut Ui, control: &mut dyn PianoControl) {
        let model = control.model();
        let status = control.status();
        if status.rebuilding {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(50));
        }

        let learning = control.midi().and_then(|l| l.learning().map(String::from));
        if learning.is_some() && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            control.midi().map(|l| l.cancel());
        }

        egui::Panel::top("piano-top").show(ui, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.heading("Piano");
                ui.separator();
                self.preset_bar(ui, control, &model);
            });
            if let Some(path) = &learning {
                let label = self.registry.get(path).map_or(path.as_str(), |p| p.meta.label.as_str());
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("Move a controller over part of its travel for \u{201c}{label}\u{201d}…")).color(widgets::LIVE));
                    if ui.small_button("Cancel").clicked() {
                        control.midi().map(|l| l.cancel());
                    }
                    ui.label(RichText::new("(Esc)").weak());
                });
            }
            ui.add_space(2.0);
        });

        egui::Panel::bottom("piano-status").show(ui, |ui| {
            ui.horizontal(|ui| {
                if status.rebuilding {
                    ui.spinner();
                    ui.label("Rebuilding the instrument…");
                } else if let Some(t) = status.last_build {
                    ui.label(RichText::new(format!("Instrument built in {} ms", t.as_millis())).weak());
                }
                ui.separator();
                ui.label(format!("{} voices", status.voices));
                ui.separator();
                ui.label(format!("CPU {:.0} %", 100.0 * self.load.update(status.load))).on_hover_text("Audio callback time over buffer duration, peak of the last second");
                if let Some(e) = &status.error {
                    ui.separator();
                    ui.colored_label(ui.visuals().error_fg_color, e);
                }
            });
        });

        egui::Panel::left("piano-pages").resizable(false).show(ui, |ui| {
            ui.add_space(6.0);
            ui.add(egui::TextEdit::singleline(&mut self.search).hint_text("Search…").desired_width(140.0));
            ui.add_space(6.0);
            for (i, s) in SECTIONS.iter().enumerate() {
                if !self.section.contains(&i) {
                    continue;
                }
                if ui.selectable_label(self.page == i && self.search.is_empty(), s.title).clicked() {
                    self.page = i;
                    self.search.clear();
                }
            }
            if control.midi().is_some() {
                ui.add_space(6.0);
                if ui.selectable_label(self.page == MIDI_PAGE && self.search.is_empty(), "MIDI").clicked() {
                    self.page = MIDI_PAGE;
                    self.search.clear();
                }
            }
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                widgets::kind_mark(ui, true);
                ui.label(RichText::new("live").small().weak());
                widgets::kind_mark(ui, false);
                ui.label(RichText::new("rebuild").small().weak());
            });
        });

        if let Some(i) = self.about {
            let mut open = true;
            egui::Panel::right("piano-about").resizable(true).default_size(320.0).min_size(220.0).show(ui, |ui| {
                ui.add_space(6.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                    open = !ui.small_button("Close").clicked();
                });
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let p = &self.registry.params()[i];
                    widgets::about(ui, p, piano_params::get(&model, &p.path), &model);
                });
            });
            if !open {
                self.about = None;
            }
        }

        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                if self.page == MIDI_PAGE && self.search.is_empty() {
                    self.midi_page(ui, control);
                } else {
                    self.page(ui, control, &model);
                }
            });
        });
    }

    fn page(&mut self, ui: &mut Ui, control: &mut dyn PianoControl, model: &PianoModel) {
        let values = self.registry.read(model);
        let needle = self.search.to_lowercase();
        let searching = !needle.is_empty();
        let shown: Vec<usize> = (0..self.registry.params().len())
            .filter(|&i| {
                let p = &self.registry.params()[i];
                if searching {
                    p.path.to_lowercase().contains(&needle) || p.meta.label.to_lowercase().contains(&needle)
                } else {
                    self.section[i] == self.page
                }
            })
            .collect();

        if !searching {
            let s = &SECTIONS[self.page.min(SECTIONS.len() - 1)];
            ui.heading(s.title);
            if s.view == View::Mics {
                self.mics.show(ui, control, model);
                ui.add_space(8.0);
            }
            let action = s.view == View::Action;
            if action || !s.curves.is_empty() {
                let reference = PianoModel::default();
                let gap = ui.spacing().item_spacing.x;
                let n = s.curves.len() as f32;
                let avail = ui.available_width();
                let sketch = if action { 380.0f32.min(avail) } else { 0.0 };
                // Curves beside the sketch when they keep 200 px each, else below.
                let beside = action && avail - sketch - gap * n >= 200.0 * n;
                let rest = if beside { avail - sketch - gap } else { avail };
                let width = ((rest - gap * (n - 1.0)) / n.max(1.0)).clamp(160.0, 420.0);
                let curves = |ui: &mut Ui| {
                    for c in s.curves {
                        widgets::curve(ui, c, model, &reference, width);
                    }
                };
                ui.horizontal_top(|ui| {
                    if action {
                        let path = self.hovered.map_or("", |i| self.registry.params()[i].path.as_str());
                        sketches::action(ui, model, sketches::parts_of(path), path, sketch)
                            .on_hover_text("Hover a parameter to see where it acts; click its name for details");
                    }
                    if beside || !action {
                        curves(ui);
                    }
                });
                if action && !beside && !s.curves.is_empty() {
                    ui.horizontal_wrapped(curves);
                }
                if !s.curves.is_empty() {
                    ui.label(RichText::new("Faint: default instrument").small().weak());
                }
            }
            ui.add_space(8.0);
        } else if shown.is_empty() {
            ui.label("No parameter matches.");
        }

        ui.spacing_mut().slider_width = 240.0;
        // One grid for the page (aligned columns), a header row per
        // sub-section (the path before the last '.'), switches first.
        let params = self.registry.params();
        let group = |i: usize| params[i].path.rsplit_once('.').map(|(g, _)| g);
        let mut shown = shown;
        shown.sort_by_key(|&i| (group(i), !matches!(params[i].default, piano_params::Value::Bool(_)), &params[i].path));
        let mut hovered = None;
        egui::Grid::new(("params", self.page, searching)).num_columns(5).striped(true).spacing([12.0, 4.0]).show(ui, |ui| {
            let mut last = None;
            for (k, &i) in shown.iter().enumerate() {
                let g = group(i);
                if k == 0 || g != last {
                    if k > 0 {
                        ui.end_row();
                    }
                    ui.label(RichText::new(group_title(g)).strong());
                    ui.end_row();
                    last = g;
                }
                let label = widgets::param_row(ui, &params[i], values[i], control);
                if label.clicked() {
                    self.about = if self.about == Some(i) { None } else { Some(i) };
                }
                if label.hovered() {
                    hovered = Some(i);
                }
                ui.end_row();
            }
        });
        self.hovered = hovered;
    }

    /// The controllers mapped, with the values at CC 0 and 127.
    fn midi_page(&mut self, ui: &mut Ui, control: &mut dyn PianoControl) {
        ui.heading("MIDI controllers");
        let Some(learn) = control.midi() else { return };
        ui.label(
            RichText::new(
                "Click the MIDI button of a parameter, then move a knob or fader. Live parameters follow the \
                 controller as you play; the others rebuild the instrument when it stops moving.",
            )
            .weak(),
        );
        ui.add_space(8.0);
        if learn.map().mappings.is_empty() {
            ui.label("No controller mapped yet.");
            return;
        }
        let mut remove = None;
        let mut range = None;
        egui::Grid::new("midi-map").num_columns(5).striped(true).spacing([12.0, 4.0]).show(ui, |ui| {
            ui.label(RichText::new("CC").strong());
            ui.label(RichText::new("Parameter").strong());
            ui.label(RichText::new("At 0").strong());
            ui.label(RichText::new("At 127").strong());
            ui.label("");
            ui.end_row();
            for m in &learn.map().mappings {
                let Some(p) = self.registry.get(&m.path) else { continue };
                ui.label(format!("{}", m.cc));
                ui.horizontal(|ui| {
                    widgets::kind_mark(ui, matches!(p.kind, piano_params::Kind::Live(_)));
                    ui.label(&p.meta.label).on_hover_text(&m.path);
                });
                let (mut lo, mut hi) = (m.min, m.max);
                let speed = (p.meta.max - p.meta.min).abs() / 300.0;
                let unit = if p.meta.unit.is_empty() { String::new() } else { format!(" {}", p.meta.unit) };
                let a = ui.add(egui::DragValue::new(&mut lo).speed(speed).custom_formatter(|v, _| widgets::show(v)).suffix(&unit));
                let b = ui.add(egui::DragValue::new(&mut hi).speed(speed).custom_formatter(|v, _| widgets::show(v)).suffix(&unit));
                if a.changed() || b.changed() {
                    range = Some((m.path.clone(), lo, hi));
                }
                if ui.small_button("Remove").clicked() {
                    remove = Some(m.path.clone());
                }
                ui.end_row();
            }
        });
        if let Some((path, lo, hi)) = range {
            learn.set_range(&path, lo, hi);
        }
        if let Some(path) = remove {
            learn.unmap(&path);
        }
    }

    fn preset_bar(&mut self, ui: &mut Ui, control: &mut dyn PianoControl, model: &PianoModel) {
        if let Some(dir) = self.presets.clone() {
            let instruments = dir.join("instruments");
            let listening = dir.join("listening");
            let structured = instruments.is_dir() || listening.is_dir();
            if structured {
                // Instruments, grouped by family (sub-folders; loose files last).
                let mut families: Vec<(String, Vec<PathBuf>)> = subdirs(&instruments)
                    .into_iter()
                    .map(|d| (d.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(), tomls(&d)))
                    .collect();
                let loose = tomls(&instruments);
                if !loose.is_empty() {
                    families.push(("Other".into(), loose));
                }
                egui::ComboBox::from_id_salt("instrument")
                    .selected_text(if self.preset_name.is_empty() { "Instrument…" } else { &self.preset_name })
                    .height(600.0)
                    .show_ui(ui, |ui| {
                        for (family, files) in &families {
                            ui.label(RichText::new(family).strong());
                            for path in files {
                                let n = stem(path);
                                if ui.selectable_label(n == self.preset_name, format!("   {n}")).clicked() {
                                    self.message = Some(piano_host::load_instrument(path, model).map(|m| {
                                        control.load(m);
                                        self.preset_name = n.clone();
                                        format!("Loaded {}", path.display())
                                    }));
                                }
                            }
                        }
                    });
                // Listening: one menu per group (sub-folder: Microphones,
                // Room…); each preset changes only its own part, so the
                // groups combine.
                let mut groups: Vec<(String, Vec<PathBuf>)> = subdirs(&listening)
                    .into_iter()
                    .map(|d| (d.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(), tomls(&d)))
                    .collect();
                let loose = tomls(&listening);
                if !loose.is_empty() {
                    groups.push(("Listening".into(), loose));
                }
                for (group, files) in &groups {
                    let current = self.listening_names.get(group).cloned().unwrap_or_default();
                    egui::ComboBox::from_id_salt(("listening", group.as_str()))
                        .selected_text(if current.is_empty() { format!("{group}…") } else { current.clone() })
                        .show_ui(ui, |ui| {
                            for path in files {
                                let n = stem(path);
                                if ui.selectable_label(n == current, &n).clicked() {
                                    self.message = Some(piano_host::load_listening(path, model).map(|m| {
                                        control.load(m);
                                        self.listening_names.insert(group.clone(), n.clone());
                                        format!("{group}: {}", path.display())
                                    }));
                                }
                            }
                        });
                }
            } else {
                egui::ComboBox::from_id_salt("preset")
                    .selected_text(if self.preset_name.is_empty() { "Preset…" } else { &self.preset_name })
                    .show_ui(ui, |ui| {
                        for path in tomls(&dir) {
                            let n = stem(&path);
                            if ui.selectable_label(n == self.preset_name, &n).clicked() {
                                self.message = Some(match piano_host::load_preset(&path) {
                                    Ok(m) => {
                                        control.load(m);
                                        self.preset_name = n;
                                        Ok(format!("Loaded {}", path.display()))
                                    }
                                    Err(e) => Err(e),
                                });
                            }
                        }
                    });
            }
            ui.add(egui::TextEdit::singleline(&mut self.preset_name).hint_text("name").desired_width(120.0));
            let name = self.preset_name.trim().to_string();
            let valid = !name.is_empty() && !name.contains(['/', '\\']);
            let path = if structured { instruments.join(format!("{name}.toml")) } else { dir.join(format!("{name}.toml")) };
            let exists = valid && path.exists();
            let save = egui::Button::new(if exists { "Overwrite" } else { "Save" });
            if ui.add_enabled(valid, save).on_hover_text("Write the current settings as a preset").clicked() {
                if structured {
                    let _ = std::fs::create_dir_all(&instruments);
                }
                self.message = Some(piano_host::save_preset(model, &path).map(|()| format!("Saved {}", path.display())));
            }
        }
        if ui.button("Defaults").on_hover_text("Back to the default instrument (the listening set-up stays)").clicked() {
            control.load(piano_host::with_listening(&PianoModel::default(), model));
            self.preset_name.clear();
            self.message = None;
        }
        match &self.message {
            Some(Ok(m)) => {
                ui.label(RichText::new(m).weak());
            }
            Some(Err(e)) => {
                ui.colored_label(ui.visuals().error_fg_color, e);
            }
            None => {}
        }
    }
}

/// Sub-folders of a folder, sorted.
fn subdirs(dir: &std::path::Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
    v.sort();
    v
}

/// The `.toml` files of a folder, sorted.
fn tomls(dir: &std::path::Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "toml") && p.is_file())
        .collect();
    v.sort();
    v
}

fn stem(path: &std::path::Path) -> String {
    path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
}

/// "soundboard" → "Soundboard"; top-level fields → "General".
fn group_title(group: Option<&str>) -> String {
    let g = group.unwrap_or("general").replace('_', " ");
    let mut c = g.chars();
    c.next().map(|f| f.to_uppercase().chain(c).collect()).unwrap_or_default()
}
