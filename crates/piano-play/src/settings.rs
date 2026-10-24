//! The interface's "Audio and MIDI" page: the player's backend, output,
//! buffer and MIDI ports. A choice is a request the window applies once it
//! has drawn (the audio restarts, or the node reconnects: see
//! [`crate::audio`]), then saves to `play.toml`.

use crate::audio::{host_label, Choices, Settings, BUFFER, BUFFERS};
use piano_core::Quality;
use egui::{RichText, Ui};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub enum Request {
    Apply(Settings),
    /// Read the devices and ports again.
    Refresh,
    /// Serve the instrument to AI assistants (from now; off at the next
    /// start).
    Mcp(bool),
    /// Start with the last sound.
    Restore(bool),
    /// The interface's theme: "system", "dark" or "light".
    Theme(String),
}

/// What the page shows and asks, shared with the window.
pub struct PageState {
    pub settings: Settings,
    pub choices: Choices,
    pub error: Option<String>,
    /// Where the settings are saved.
    pub file: Option<PathBuf>,
    pub request: Option<Request>,
    /// The MCP server: wanted at start, and its URL once running.
    pub mcp: bool,
    pub mcp_url: Option<String>,
    pub restore: bool,
    /// "system", "dark" or "light".
    pub theme: String,
}

pub const THEMES: [(&str, &str); 3] = [("system", "As the system"), ("dark", "Dark"), ("light", "Light")];

/// What a quality level leaves out, for its menu entry.
fn quality_text(q: Quality) -> &'static str {
    match q {
        Quality::Overkill => "Overkill: no shortcut at all, even inaudible ones; computed at the output's rate",
        Quality::Full => "Full: the whole model, shortcuts judged inaudible",
        Quality::Balanced => "Balanced: sympathy below 2 kHz, phantom partials up to C3, 96 voices",
        Quality::Light => "Light: no sympathy, phantom partials, frame or rim; one polarisation",
    }
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

            ui.label("MIDI channel");
            ui.horizontal(|ui| {
                let text = |c: Option<u8>| c.map_or("All channels".to_string(), |c| format!("Channel {c}"));
                egui::ComboBox::from_id_salt("channel").selected_text(text(s.midi_channel)).show_ui(ui, |ui| {
                    for c in std::iter::once(None).chain((1..=16).map(Some)) {
                        if ui.selectable_label(s.midi_channel == c, text(c)).clicked() {
                            want = Some(Settings { midi_channel: c, ..s.clone() });
                        }
                    }
                });
                let mut invert = s.invert_sustain;
                let r = ui.checkbox(&mut invert, "Invert the sustain pedal");
                if r.on_hover_text("For a keyboard whose sustain pedal sounds held when it is up: its values are turned round").changed() {
                    want = Some(Settings { invert_sustain: invert, ..s.clone() });
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
        ui.add_space(14.0);
        ui.label(RichText::new("Performance").strong());
        egui::Grid::new("performance-settings").num_columns(2).spacing([16.0, 8.0]).show(ui, |ui| {
            ui.label("Voice threads");
            ui.horizontal(|ui| {
                let cores = std::thread::available_parallelism().map_or(4, |n| n.get()).min(8);
                let text = |n: usize| if n == 1 { "1 (the audio thread alone)".to_string() } else { format!("{n}") };
                egui::ComboBox::from_id_salt("threads").selected_text(text(s.threads)).show_ui(ui, |ui| {
                    for n in 1..=cores {
                        if ui.selectable_label(s.threads == n, text(n)).clicked() {
                            want = Some(Settings { threads: n, ..s.clone() });
                        }
                    }
                });
                ui.label(RichText::new("2 take about a third off the load and its spikes").weak())
                    .on_hover_text("The voices are computed on other cores, the sound is the same bit for bit. The extra threads spin briefly after each block (one core per thread while playing).");
            });
            ui.end_row();
            ui.label("CPU quality");
            ui.vertical(|ui| {
                egui::ComboBox::from_id_salt("quality").selected_text(quality_text(s.quality)).width(420.0).show_ui(ui, |ui| {
                    for q in Quality::ALL {
                        if ui.selectable_label(s.quality == q, quality_text(q)).clicked() {
                            want = Some(Settings { quality: q, ..s.clone() });
                        }
                    }
                });
                if !matches!(s.quality, Quality::Full | Quality::Overkill) {
                    let note = "Overrides those parameters while it is on; presets keep their own values.";
                    ui.label(RichText::new(note).small().weak());
                }
            });
            ui.end_row();
            ui.label("Sample rate");
            ui.horizontal(|ui| {
                let mut full = s.full_rate;
                let tip = "Above 48 kHz (88.2, 96, 176.4, 192 kHz…), the piano is computed at 44.1 or 48 kHz and upsampled: half the load or less, the same sound up to 20 kHz. Checked, it is computed at the output's rate.";
                if ui.checkbox(&mut full, "Compute at the output's rate above 48 kHz").on_hover_text(tip).changed() {
                    want = Some(Settings { full_rate: full, ..s.clone() });
                }
                ui.label(RichText::new("twice the load at 96 kHz, nothing more to hear").weak());
            });
            ui.end_row();
        });
        ui.label(RichText::new("Changing the threads, the quality or the sample rate rebuilds the engine (about 0.1 s). Balanced and Light have not been compared by ear yet.").small().weak());

        ui.add_space(14.0);
        ui.label(RichText::new("Application").strong());
        ui.horizontal(|ui| {
            ui.label("Theme");
            let shown = THEMES.iter().find(|t| t.0 == st.theme).map_or("As the system", |t| t.1);
            egui::ComboBox::from_id_salt("theme").selected_text(shown).show_ui(ui, |ui| {
                for (id, label) in THEMES {
                    if ui.selectable_label(st.theme == id, label).clicked() {
                        st.request = Some(Request::Theme(id.to_string()));
                    }
                }
            });
        });
        let mut restore = st.restore;
        if ui.checkbox(&mut restore, "Start with the last sound").on_hover_text("Without a preset on the command line, the player starts with the instrument and listening set-up it had when it was closed").changed() {
            st.request = Some(Request::Restore(restore));
        }
        ui.horizontal(|ui| {
            let mut mcp = st.mcp;
            if ui.checkbox(&mut mcp, "Serve the instrument to AI assistants (MCP)").changed() {
                st.request = Some(Request::Mcp(mcp));
            }
            match (&st.mcp_url, st.mcp) {
                (Some(url), true) => ui.label(RichText::new(url).monospace().weak()),
                (Some(_), false) => ui.label(RichText::new("stops at the next start").weak()),
                _ => ui.label(""),
            };
        });

        if let Some(f) = &st.file {
            ui.label(RichText::new(format!("Saved in {}; options given on the command line take precedence.", f.display())).small().weak());
        }
        if let Some(w) = want {
            st.request = Some(Request::Apply(w));
        }
    }
}
