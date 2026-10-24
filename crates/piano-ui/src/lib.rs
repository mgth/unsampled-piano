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

/// The author's page for support.
const KOFI: &str = "https://ko-fi.com/mgth";

/// Page index of the MIDI map (after the sections).
const MIDI_PAGE: usize = usize::MAX;
/// Page index of the host's own page ([`HostPage`]).
const HOST_PAGE: usize = usize::MAX - 1;

/// A page the host adds to the interface, listed last: the standalone
/// player's audio and MIDI settings (a plugin's host handles those).
pub trait HostPage: Send {
    /// Its name in the page list.
    fn title(&self) -> &str;
    /// Draws it into the central panel (scrolled).
    fn show(&mut self, ui: &mut Ui);
}

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
    /// Key whose string the string sketch shows.
    key: u8,
    /// Preset folder (TOML files), when the host has one: either flat, or
    /// `instruments/<family>/*.toml` and `listening/*.toml`. Saving writes
    /// here.
    presets: Option<PathBuf>,
    /// Read-only folders of the same layout listed after it (the presets
    /// installed with the program); a preset of the same name in an earlier
    /// folder hides theirs.
    factory: Vec<PathBuf>,
    preset_name: String,
    /// Last listening preset loaded in each listening group (Microphones,
    /// Room…): the groups combine, each shows its own choice.
    listening_names: std::collections::BTreeMap<String, String>,
    /// The touch chosen (`touch/*.toml`: a velocity law laid over the
    /// instrument, kept when another instrument loads), and the
    /// instrument's own law it replaced.
    touch: Option<(String, PathBuf)>,
    touch_base: Option<piano_core::params::VelocityLaw>,
    /// Result of the last preset action.
    message: Option<Result<String, String>>,
    /// CPU load shown: the peak over the last second (the status gives the
    /// peak since it was last read).
    load: LoadMeter,
    host_page: Option<Box<dyn HostPage>>,
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
            key: 60,
            presets,
            factory: Vec::new(),
            preset_name: String::new(),
            listening_names: Default::default(),
            touch: None,
            touch_base: None,
            message: None,
            load: LoadMeter::default(),
            host_page: None,
        }
    }

    /// Adds the host's page, listed after the others.
    pub fn set_host_page(&mut self, page: Box<dyn HostPage>) {
        self.host_page = Some(page);
    }

    /// Adds read-only preset folders, listed after the one saving writes to
    /// (the same folder twice is listed once).
    pub fn add_preset_dirs(&mut self, dirs: impl IntoIterator<Item = PathBuf>) {
        let same = |a: &PathBuf, b: &PathBuf| a == b || a.canonicalize().ok().is_some_and(|x| b.canonicalize().ok() == Some(x));
        for d in dirs {
            if !self.presets.iter().chain(&self.factory).any(|x| same(x, &d)) {
                self.factory.push(d);
            }
        }
    }

    /// Every preset folder, the one saving writes to first.
    fn preset_roots(&self) -> Vec<PathBuf> {
        self.presets.iter().chain(&self.factory).cloned().collect()
    }

    /// The preset shown as loaded (its file name without `.toml`): in the
    /// menu of the listening group holding a file of that name, else as the
    /// instrument.
    pub fn set_preset_name(&mut self, name: &str) {
        if let Some(path) = merged(self.preset_roots().iter().map(|r| tomls(&r.join("touch")))).into_iter().find(|f| stem(f) == name) {
            self.touch = Some((name.into(), path));
            return;
        }
        for (group, files) in groups(&self.preset_roots(), "listening", "Listening") {
            if files.iter().any(|f| stem(f) == name) {
                self.listening_names.insert(group, name.into());
                return;
            }
        }
        self.preset_name = name.into();
    }

    /// Lays the chosen touch over a model about to load (an instrument, the
    /// defaults), remembering its own velocity law.
    fn with_touch(&mut self, m: PianoModel) -> PianoModel {
        let Some((_, path)) = &self.touch else { return m };
        self.touch_base = Some(m.velocity);
        piano_host::load_listening(path, &m).unwrap_or(m)
    }

    /// Chooses a touch over `model` (None: back to the instrument's own
    /// velocity law).
    fn set_touch(&mut self, control: &mut dyn PianoControl, model: &PianoModel, touch: Option<(String, PathBuf)>) -> Result<String, String> {
        // The instrument's own law, under any touch already laid over it.
        let mut base = *model;
        if let (Some(_), Some(v)) = (&self.touch, self.touch_base) {
            base.velocity = v;
        }
        let msg = match &touch {
            Some((name, path)) => {
                let m = piano_host::load_listening(path, &base)?;
                control.load(m);
                format!("Touch: {name}")
            }
            None => {
                control.load(base);
                "The instrument's own touch".to_string()
            }
        };
        self.touch_base = touch.as_ref().map(|_| base.velocity);
        self.touch = touch;
        Ok(msg)
    }

    /// Shows the help of the parameter at `path`, if it exists.
    pub fn show_help(&mut self, path: &str) {
        self.about = self.registry.params().iter().position(|p| p.path == path);
    }

    /// Opens the page with this title (case-insensitive), "MIDI" or the
    /// host page's.
    pub fn open_page(&mut self, title: &str) {
        if title.eq_ignore_ascii_case("midi") {
            self.page = MIDI_PAGE;
            self.search.clear();
        } else if self.host_page.as_ref().is_some_and(|h| h.title().eq_ignore_ascii_case(title)) {
            self.page = HOST_PAGE;
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
        if learning.is_some()
            && ui.input(|i| i.key_pressed(egui::Key::Escape))
            && let Some(l) = control.midi()
        {
            l.cancel();
        }

        egui::Panel::top("piano-top").show(ui, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.heading("Unsampled Piano");
                ui.separator();
                self.preset_bar(ui, control, &model);
            });
            if let Some(path) = &learning {
                let label = self.registry.get(path).map_or(path.as_str(), |p| p.meta.label.as_str());
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("Move a controller over part of its travel for \u{201c}{label}\u{201d}…")).color(widgets::LIVE));
                    if ui.small_button("Cancel").clicked()
                        && let Some(l) = control.midi()
                    {
                        l.cancel();
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
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.hyperlink_to("Support on Ko-fi", KOFI)
                        .on_hover_text("Unsampled Piano is free software: a coffee keeps it going");
                    widgets::heart(ui);
                });
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
            if let Some(h) = &self.host_page {
                ui.add_space(6.0);
                if ui.selectable_label(self.page == HOST_PAGE && self.search.is_empty(), h.title()).clicked() {
                    self.page = HOST_PAGE;
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
                    widgets::about(ui, p, piano_params::get(&model, &p.path), &model, &mut self.key);
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
                } else if let (HOST_PAGE, true, Some(h)) = (self.page, self.search.is_empty(), self.host_page.as_mut()) {
                    h.show(ui);
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
            let drawn: &[sketches::Sketch] = match s.view {
                View::Sketches(list) => list,
                _ => &[],
            };
            if !drawn.is_empty() || !s.curves.is_empty() {
                self.figures(ui, drawn, s.curves, model);
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

    /// A page's sketches and curves, packed in rows: sketches 380 px wide,
    /// curves sharing the width (at least 160 px each).
    fn figures(&mut self, ui: &mut Ui, drawn: &[sketches::Sketch], curves: &[sections::Curve], model: &PianoModel) {
        enum Fig<'a> {
            Sketch(sketches::Sketch),
            Curve(&'a sections::Curve),
        }
        let gap = ui.spacing().item_spacing.x;
        let avail = ui.available_width();
        let n = curves.len() as f32;
        let curve_w = ((avail - gap * (n - 1.0)) / n.max(1.0)).clamp(160.0, 420.0);
        let sketch_w = 380.0f32.min(avail);
        let figs: Vec<(Fig, f32)> = drawn
            .iter()
            .map(|&s| (Fig::Sketch(s), sketch_w))
            .chain(curves.iter().map(|c| (Fig::Curve(c), curve_w)))
            .collect();
        let mut rows: Vec<Vec<&(Fig, f32)>> = vec![vec![]];
        let mut used = 0.0;
        for f in &figs {
            if used > 0.0 && used + gap + f.1 > avail {
                rows.push(vec![]);
                used = 0.0;
            }
            used += if used > 0.0 { gap } else { 0.0 } + f.1;
            rows.last_mut().unwrap().push(f);
        }
        let reference = PianoModel::default();
        let path = self.hovered.map_or(String::new(), |i| self.registry.params()[i].path.clone());
        for row in rows {
            ui.horizontal_top(|ui| {
                for (fig, w) in row {
                    match fig {
                        Fig::Sketch(s) => {
                            sketches::draw(ui, *s, model, &path, &mut self.key, *w)
                                .on_hover_text("Hover a parameter to see where it acts; click its name for details");
                        }
                        Fig::Curve(c) => widgets::curve(ui, c, model, &reference, *w),
                    }
                }
            });
        }
        if !curves.is_empty() {
            ui.label(RichText::new("Faint: default instrument").small().weak());
        }
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
        let roots = self.preset_roots();
        if !roots.is_empty() {
            let structured = roots.iter().any(|d| d.join("instruments").is_dir() || d.join("listening").is_dir());
            if structured {
                // Instruments, grouped by family (sub-folders; loose files last).
                let families = groups(&roots, "instruments", "Other");
                egui::ComboBox::from_id_salt("instrument")
                    .selected_text(if self.preset_name.is_empty() { "Instrument…" } else { &self.preset_name })
                    .height(600.0)
                    .show_ui(ui, |ui| {
                        for (family, files) in &families {
                            ui.label(RichText::new(family).strong());
                            for path in files {
                                let n = stem(path);
                                if ui.selectable_label(n == self.preset_name, format!("   {n}")).clicked() {
                                    self.message = Some(match piano_host::load_instrument(path, model) {
                                        Ok(m) => {
                                            control.load(self.with_touch(m));
                                            self.preset_name = n.clone();
                                            Ok(format!("Loaded {}", path.display()))
                                        }
                                        Err(e) => Err(e),
                                    });
                                }
                            }
                        }
                    });
                // Listening: one menu per group (sub-folder: Microphones,
                // Room…); each preset changes only its own part, so the
                // groups combine.
                for (group, files) in &groups(&roots, "listening", "Listening") {
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
                // Touch: how the velocities become hammer speeds, over any
                // instrument (e.g. MIDI recorded from pianists).
                let touches = merged(roots.iter().map(|r| tomls(&r.join("touch"))));
                if !touches.is_empty() {
                    let current = self.touch.as_ref().map(|t| t.0.clone());
                    let mut chosen = None;
                    egui::ComboBox::from_id_salt("touch")
                        .selected_text(current.clone().unwrap_or_else(|| "Touch…".into()))
                        .show_ui(ui, |ui| {
                            if ui.selectable_label(current.is_none(), "Instrument's own").clicked() {
                                chosen = Some(None);
                            }
                            for path in &touches {
                                let n = stem(path);
                                if ui.selectable_label(current.as_deref() == Some(n.as_str()), &n).clicked() {
                                    chosen = Some(Some((n, path.clone())));
                                }
                            }
                        })
                        .response
                        .on_hover_text("How MIDI velocities become hammer speeds, laid over the instrument (kept when another instrument loads)");
                    if let Some(t) = chosen {
                        self.message = Some(self.set_touch(control, model, t));
                    }
                }
            } else {
                egui::ComboBox::from_id_salt("preset")
                    .selected_text(if self.preset_name.is_empty() { "Preset…" } else { &self.preset_name })
                    .show_ui(ui, |ui| {
                        for path in merged(roots.iter().map(|d| tomls(d))) {
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
        }
        // Saving: into the first folder (an installed preset of the same
        // name stays, hidden by the saved one).
        if let Some(dir) = self.presets.clone() {
            let structured = roots.iter().any(|d| d.join("instruments").is_dir() || d.join("listening").is_dir());
            let target = if structured { dir.join("instruments") } else { dir };
            ui.add(egui::TextEdit::singleline(&mut self.preset_name).hint_text("name").desired_width(120.0));
            let name = self.preset_name.trim().to_string();
            let valid = !name.is_empty() && !name.contains(['/', '\\']);
            let path = target.join(format!("{name}.toml"));
            let exists = valid && path.exists();
            let save = egui::Button::new(if exists { "Overwrite" } else { "Save" });
            if ui.add_enabled(valid, save).on_hover_text("Write the current settings as a preset").clicked() {
                let _ = std::fs::create_dir_all(&target);
                self.message = Some(piano_host::save_preset(model, &path).map(|()| format!("Saved {}", path.display())));
            }
        }
        if ui.button("Defaults").on_hover_text("Back to the default instrument (the listening set-up and the touch stay)").clicked() {
            let m = self.with_touch(piano_host::with_listening(&PianoModel::default(), model));
            control.load(m);
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

/// The presets of `sub` (instruments, listening) over every folder, by
/// group: one per sub-folder name, sorted, then the loose files under
/// `loose`. In each group a name appears once, from the first folder.
fn groups(roots: &[PathBuf], sub: &str, loose: &str) -> Vec<(String, Vec<PathBuf>)> {
    let mut named: std::collections::BTreeMap<String, Vec<Vec<PathBuf>>> = Default::default();
    for root in roots {
        for d in subdirs(&root.join(sub)) {
            named.entry(stem_dir(&d)).or_default().push(tomls(&d));
        }
    }
    let mut out: Vec<(String, Vec<PathBuf>)> = named.into_iter().map(|(g, lists)| (g, merged(lists))).collect();
    let rest = merged(roots.iter().map(|r| tomls(&r.join(sub))));
    if !rest.is_empty() {
        out.push((loose.into(), rest));
    }
    out
}

/// Files of several folders, sorted by name, each name once (the first).
fn merged(lists: impl IntoIterator<Item = Vec<PathBuf>>) -> Vec<PathBuf> {
    let mut seen = std::collections::BTreeMap::new();
    for list in lists {
        for p in list {
            seen.entry(stem(&p)).or_insert(p);
        }
    }
    seen.into_values().collect()
}

fn stem_dir(dir: &std::path::Path) -> String {
    dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The saved folder and a shipped one merge by group, the saved preset
    /// hiding the shipped one of the same name.
    #[test]
    fn preset_folders_merge() {
        let base = std::env::temp_dir().join(format!("piano-ui-presets-{}", std::process::id()));
        let (user, shipped) = (base.join("user"), base.join("shipped"));
        for (dir, file) in [
            (&shipped, "instruments/Grand/concert.toml"),
            (&shipped, "instruments/Grand/studio.toml"),
            (&shipped, "listening/Room/hall.toml"),
            (&user, "instruments/Grand/concert.toml"),
            (&user, "instruments/Upright/mine.toml"),
        ] {
            let path = dir.join(file);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, "").unwrap();
        }
        let mut ui = PianoUi::new(Some(user.clone()));
        ui.add_preset_dirs([shipped.clone(), user.clone()]);
        assert_eq!(ui.preset_roots(), vec![user.clone(), shipped.clone()]);
        let names = groups(&ui.preset_roots(), "instruments", "Other");
        assert_eq!(names[0].0, "Grand");
        assert_eq!(names[0].1, vec![user.join("instruments/Grand/concert.toml"), shipped.join("instruments/Grand/studio.toml")]);
        assert_eq!(names[1].0, "Upright");
        ui.set_preset_name("hall");
        assert_eq!(ui.listening_names.get("Room").map(String::as_str), Some("hall"));
        std::fs::remove_dir_all(&base).unwrap();
    }
}

#[cfg(test)]
mod touch_tests {
    use super::*;
    use crate::mock::MockControl;

    #[test]
    fn a_touch_lies_over_any_instrument() {
        let dir = std::env::temp_dir().join(format!("piano-ui-touch-{}", std::process::id()));
        let touch = dir.join("touch").join("fast.toml");
        std::fs::create_dir_all(touch.parent().unwrap()).unwrap();
        std::fs::write(&touch, "[velocity]\nvmax = 9.0\n").unwrap();
        let mut ui = PianoUi::new(Some(dir.clone()));
        let own = PianoModel::default().velocity;
        let mut control = MockControl::new(PianoModel::default());

        let m = control.model();
        ui.set_touch(&mut control, &m, Some(("fast".into(), touch.clone()))).unwrap();
        assert_eq!(control.model().velocity.vmax, 9.0);
        // The defaults (or another instrument) load with the touch over them.
        let again = ui.with_touch(PianoModel::default());
        assert_eq!((again.velocity.vmax, again.velocity.vmin), (9.0, own.vmin));
        // Back to the instrument's own law.
        let m = control.model();
        ui.set_touch(&mut control, &m, None).unwrap();
        assert_eq!(control.model().velocity, own);
        assert_eq!(ui.with_touch(PianoModel::default()).velocity, own);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
