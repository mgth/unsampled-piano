//! The interface's "Audio and MIDI" page: the player's backend, output,
//! buffer and MIDI ports. A choice is a request the window applies once it
//! has drawn (the audio restarts, or the node reconnects: see
//! [`crate::audio`]), then saves to `play.toml`.

use crate::audio::{host_label, Choices, Settings, BUFFER, BUFFERS};
use egui::{RichText, Ui};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub enum Request {
    Apply(Settings),
    /// Read the devices and ports again.
    Refresh,
}

/// What the page shows and asks, shared with the window.
pub struct PageState {
    pub settings: Settings,
    pub choices: Choices,
    pub error: Option<String>,
    /// Where the settings are saved.
    pub file: Option<PathBuf>,
    pub request: Option<Request>,
}

pub struct SettingsPage(pub Arc<Mutex<PageState>>);

pub const TITLE: &str = "Audio and MIDI";

impl piano_ui::HostPage for SettingsPage {
    fn title(&self) -> &str {
        TITLE
    }

    fn show(&mut self, ui: &mut Ui) {
        let mut st = self.0.lock().unwrap();
        let st = &mut *st;
        ui.heading(TITLE);
        ui.label(RichText::new(&st.choices.info).weak());
        ui.add_space(8.0);
        let c = &st.choices;
        let s = st.settings.clone();
        let node = c.backend == "node";
        let mut want: Option<Settings> = None;
        let now = |x: &Option<String>| x.as_deref().map_or(String::new(), |v| format!(" (now {v})"));

        egui::Grid::new("audio-settings").num_columns(2).spacing([16.0, 8.0]).show(ui, |ui| {
            ui.label("Audio backend");
            let shown = match &s.host {
                Some(h) => host_label(h),
                None => format!("Automatic: {}", host_label(&c.backend)),
            };
            egui::ComboBox::from_id_salt("backend").selected_text(shown).width(320.0).show_ui(ui, |ui| {
                if ui.selectable_label(s.host.is_none(), "Automatic").clicked() {
                    want = Some(Settings { host: None, device: None, ..s.clone() });
                }
                for (id, label) in &c.backends {
                    if ui.selectable_label(s.host.as_deref() == Some(id.as_str()), label).clicked() {
                        want = Some(Settings { host: Some(id.clone()), device: None, ..s.clone() });
                    }
                }
            });
            ui.end_row();

            ui.label("Output");
            let shown = s.device.clone().unwrap_or_else(|| format!("System default{}", now(&c.output_now)));
            egui::ComboBox::from_id_salt("output").selected_text(shown).width(320.0).show_ui(ui, |ui| {
                if ui.selectable_label(s.device.is_none(), "System default").clicked() {
                    want = Some(Settings { device: None, ..s.clone() });
                }
                for d in &c.outputs {
                    if ui.selectable_label(s.device.as_deref() == Some(d.as_str()), d).clicked() {
                        want = Some(Settings { device: Some(d.clone()), ..s.clone() });
                    }
                }
            });
            ui.end_row();

            if !node && c.channels > 2 {
                ui.label("Channels");
                let pair = |first: u16| format!("{}–{}", first, first + 1);
                egui::ComboBox::from_id_salt("channels").selected_text(pair(s.output_channel)).show_ui(ui, |ui| {
                    for first in (1..c.channels).step_by(2) {
                        if ui.selectable_label(s.output_channel == first, pair(first)).clicked() {
                            want = Some(Settings { output_channel: first, ..s.clone() });
                        }
                    }
                });
                ui.end_row();
            }

            ui.label("Buffer");
            ui.horizontal(|ui| {
                let shown = s.buffer.map_or(format!("Default ({BUFFER})"), |b| format!("{b} frames"));
                egui::ComboBox::from_id_salt("buffer").selected_text(shown).show_ui(ui, |ui| {
                    if ui.selectable_label(s.buffer.is_none(), format!("Default ({BUFFER})")).clicked() {
                        want = Some(Settings { buffer: None, ..s.clone() });
                    }
                    for b in BUFFERS {
                        if ui.selectable_label(s.buffer == Some(b), format!("{b} frames")).clicked() {
                            want = Some(Settings { buffer: Some(b), ..s.clone() });
                        }
                    }
                });
                if node {
                    ui.label(RichText::new("the graph's period: applies at the next start").weak());
                }
            });
            ui.end_row();

            ui.label("MIDI input");
            let shown = s.midi.clone().unwrap_or_else(|| format!("First keyboard{}", now(&c.midi_now)));
            egui::ComboBox::from_id_salt("midi").selected_text(shown).width(320.0).show_ui(ui, |ui| {
                if ui.selectable_label(s.midi.is_none(), "First keyboard").clicked() {
                    want = Some(Settings { midi: None, ..s.clone() });
                }
                for m in &c.midi_inputs {
                    if ui.selectable_label(s.midi.as_deref() == Some(m.as_str()), m).clicked() {
                        want = Some(Settings { midi: Some(m.clone()), ..s.clone() });
                    }
                }
            });
            ui.end_row();

            if node {
                ui.label("MIDI mirror").on_hover_text("Sends every note the piano plays to another MIDI input of the graph");
                let shown = s.midi_out.clone().unwrap_or_else(|| "Not connected".into());
                egui::ComboBox::from_id_salt("mirror").selected_text(shown).width(320.0).show_ui(ui, |ui| {
                    if ui.selectable_label(s.midi_out.is_none(), "Not connected").clicked() {
                        want = Some(Settings { midi_out: None, ..s.clone() });
                    }
                    for m in &c.mirror_targets {
                        if ui.selectable_label(s.midi_out.as_deref() == Some(m.as_str()), m).clicked() {
                            want = Some(Settings { midi_out: Some(m.clone()), ..s.clone() });
                        }
                    }
                });
                ui.end_row();
            }
        });

        ui.add_space(8.0);
        if ui.button("Refresh the lists").on_hover_text("Read the devices and ports again (after plugging one in)").clicked() {
            st.request = Some(Request::Refresh);
        }
        if let Some(e) = &st.error {
            ui.add_space(4.0);
            ui.colored_label(ui.visuals().error_fg_color, e);
        }
        ui.add_space(12.0);
        let note = "Changing the backend or a sound card's device restarts the audio and rebuilds the instrument (about 0.1 s).";
        ui.label(RichText::new(note).small().weak());
        if let Some(f) = &st.file {
            ui.label(RichText::new(format!("Saved in {}; options given on the command line take precedence.", f.display())).small().weak());
        }
        if let Some(w) = want {
            st.request = Some(Request::Apply(w));
        }
    }
}
