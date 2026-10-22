//! The instrument's interface: pages of parameters with the per-key laws
//! they shape, presets, and the engine's state. It draws into any egui
//! [`Ui`] and acts through [`PianoControl`], so the standalone player and
//! the plugin wrappers only provide the window and the control.

pub mod mock;
pub mod sections;
mod widgets;

use std::path::PathBuf;

use egui::{RichText, Ui};
use piano_core::params::PianoModel;
use piano_params::{PianoControl, Registry};

use sections::{section_of, SECTIONS};

pub struct PianoUi {
    registry: Registry,
    /// Section of each registry parameter.
    section: Vec<usize>,
    page: usize,
    search: String,
    /// Preset folder (TOML files), when the host has one.
    presets: Option<PathBuf>,
    preset_name: String,
    /// Result of the last preset action.
    message: Option<Result<String, String>>,
}

impl PianoUi {
    pub fn new(presets: Option<PathBuf>) -> Self {
        let registry = Registry::new();
        let section = registry.params().iter().map(|p| section_of(&p.path)).collect();
        Self { registry, section, page: 0, search: String::new(), presets, preset_name: String::new(), message: None }
    }

    /// Opens the page with this title (case-insensitive), if any.
    pub fn open_page(&mut self, title: &str) {
        if let Some(i) = SECTIONS.iter().position(|s| s.title.eq_ignore_ascii_case(title)) {
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

        egui::Panel::top("piano-top").show(ui, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.heading("Piano");
                ui.separator();
                self.preset_bar(ui, control, &model);
            });
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
                ui.label(format!("CPU {:.0} %", 100.0 * status.load));
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
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                widgets::kind_mark(ui, true);
                ui.label(RichText::new("live").small().weak());
                widgets::kind_mark(ui, false);
                ui.label(RichText::new("rebuild").small().weak());
            });
        });

        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| self.page(ui, control, &model));
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
            let s = &SECTIONS[self.page];
            ui.heading(s.title);
            if !s.curves.is_empty() {
                let reference = PianoModel::default();
                let gap = ui.spacing().item_spacing.x;
                let n = s.curves.len() as f32;
                let width = ((ui.available_width() - gap * (n - 1.0)) / n).clamp(160.0, 420.0);
                ui.horizontal_wrapped(|ui| {
                    for c in s.curves {
                        widgets::curve(ui, c, model, &reference, width);
                    }
                });
                ui.label(RichText::new("Faint: default instrument").small().weak());
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
        egui::Grid::new(("params", self.page, searching)).num_columns(4).striped(true).spacing([12.0, 4.0]).show(ui, |ui| {
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
                widgets::param_row(ui, &params[i], values[i], control);
                ui.end_row();
            }
        });
    }

    fn preset_bar(&mut self, ui: &mut Ui, control: &mut dyn PianoControl, model: &PianoModel) {
        if let Some(dir) = self.presets.clone() {
            let mut names: Vec<String> = std::fs::read_dir(&dir)
                .into_iter()
                .flatten()
                .flatten()
                .filter_map(|e| e.file_name().to_str()?.strip_suffix(".toml").map(String::from))
                .collect();
            names.sort();
            egui::ComboBox::from_id_salt("preset")
                .selected_text(if self.preset_name.is_empty() { "Preset…" } else { &self.preset_name })
                .show_ui(ui, |ui| {
                    for n in names {
                        if ui.selectable_label(n == self.preset_name, &n).clicked() {
                            let path = dir.join(format!("{n}.toml"));
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
            ui.add(egui::TextEdit::singleline(&mut self.preset_name).hint_text("name").desired_width(120.0));
            let name = self.preset_name.trim().to_string();
            let valid = !name.is_empty() && !name.contains(['/', '\\']);
            let path = dir.join(format!("{name}.toml"));
            let exists = valid && path.exists();
            let save = egui::Button::new(if exists { "Overwrite" } else { "Save" });
            if ui.add_enabled(valid, save).on_hover_text("Write the current settings as a preset").clicked() {
                self.message = Some(piano_host::save_preset(model, &path).map(|()| format!("Saved {}", path.display())));
            }
        }
        if ui.button("Defaults").on_hover_text("Back to the default instrument").clicked() {
            control.load(PianoModel::default());
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

/// "soundboard" → "Soundboard"; top-level fields → "General".
fn group_title(group: Option<&str>) -> String {
    let g = group.unwrap_or("general").replace('_', " ");
    let mut c = g.chars();
    c.next().map(|f| f.to_uppercase().chain(c).collect()).unwrap_or_default()
}
