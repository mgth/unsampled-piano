//! The editor: piano-ui on the plugin's controller. Live parameters are
//! host parameters, edited through the host (gestures recorded as
//! automation); the others go to the controller and rebuild.

use std::sync::Arc;
use std::time::Duration;

use nice_plug::context::gui::{GuiContext, ParamSetter};
use nice_plug::editor::dpi::LogicalSize;
use nice_plug::prelude::*;
use nice_plug_egui::resizable_window::ResizableWindow;
use nice_plug_egui::{EguiEditor, EguiEditorState, EguiNiceSettings, NiceEguiApp};
use piano_control::Controller;
use piano_core::params::PianoModel;
use piano_params::midi::MidiLearn;
use piano_params::{LiveParam, PianoControl, Status, Value};
use piano_ui::PianoUi;

use crate::{PianoParams, Shared};

const SIZE: LogicalSize<f32> = LogicalSize::new(1100.0, 760.0);
const MIN_SIZE: LogicalSize<f32> = LogicalSize::new(760.0, 480.0);

pub fn state() -> Arc<EguiEditorState> {
    EguiEditorState::from_size(SIZE, 1.0)
}

pub fn create(shared: Arc<Shared>, state: Arc<EguiEditorState>) -> Option<EguiEditor<PianoEditor>> {
    let mut ui = PianoUi::new(presets_dir());
    ui.add_preset_dirs(piano_host::installed_preset_dirs());
    let editor = PianoEditor { ui, shared: shared.clone(), gui: None };
    let hint = ResizeHint::resizable().with_min_logical_size(MIN_SIZE);
    nice_plug_egui::create_egui_editor(state, shared.repaint.clone(), EguiNiceSettings::new().with_resize_hint(hint), editor)
}

pub struct PianoEditor {
    ui: PianoUi,
    shared: Arc<Shared>,
    gui: Option<GuiContext>,
}

impl NiceEguiApp for PianoEditor {
    fn build(
        &mut self,
        _egui_ctx: egui::Context,
        gui: GuiContext,
        _frame: &mut nice_plug_egui::Frame,
    ) -> Result<(), nice_plug_egui::baseview::HandlerError> {
        self.gui = Some(gui);
        Ok(())
    }

    fn editor_closed(&mut self) {
        self.gui = None;
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut nice_plug_egui::Frame) {
        let Some(gui) = &self.gui else { return };
        let setter = gui.param_setter();
        let mut guard = self.shared.control.lock().unwrap();
        ResizableWindow::new("piano").min_size(egui::vec2(MIN_SIZE.width, MIN_SIZE.height)).show(ui, |ui| {
            match guard.as_mut() {
                Some(ctl) => {
                    ctl.poll();
                    let mut control = PluginControl { ctl, params: &self.shared.params, setter: &setter };
                    self.ui.show(ui, &mut control);
                }
                None => {
                    ui.centered_and_justified(|ui| ui.label("The piano starts when the host activates the plugin."));
                }
            }
        });
        // Voices and CPU load.
        ui.ctx().request_repaint_after(Duration::from_millis(250));
    }
}

struct PluginControl<'a> {
    ctl: &'a mut Controller,
    params: &'a PianoParams,
    setter: &'a ParamSetter<'a>,
}

impl PianoControl for PluginControl<'_> {
    fn model(&self) -> PianoModel {
        let mut m = self.ctl.model();
        for (id, p) in &self.params.live {
            id.set(&mut m, p.value() as f64);
        }
        m
    }

    fn begin_edit(&mut self, path: &str) {
        if let Some(id) = LiveParam::from_path(path) {
            self.setter.begin_set_parameter(self.params.live(id));
        }
    }

    fn set(&mut self, path: &str, value: Value) {
        match LiveParam::from_path(path) {
            Some(id) => self.setter.set_parameter(self.params.live(id), value.as_f64() as f32),
            None => self.ctl.set(path, value),
        }
    }

    fn end_edit(&mut self, path: &str) {
        if let Some(id) = LiveParam::from_path(path) {
            self.setter.end_set_parameter(self.params.live(id));
        }
    }

    /// A preset: one rebuild, and the host parameters take its live values.
    fn load(&mut self, model: PianoModel) {
        self.ctl.load(model);
        for (id, p) in &self.params.live {
            self.setter.begin_set_parameter(p);
            self.setter.set_parameter(p, id.get(&model) as f32);
            self.setter.end_set_parameter(p);
        }
    }

    fn status(&self) -> Status {
        self.ctl.status()
    }

    fn midi(&mut self) -> Option<&mut MidiLearn> {
        self.ctl.midi()
    }

    /// The host maps its own controllers to its parameters.
    fn midi_learnable(&self, path: &str) -> bool {
        LiveParam::from_path(path).is_none()
    }
}

/// Presets saved by the user: `presets` in the configuration's play.toml,
/// else the folder of its `preset`, else `presets` in the configuration
/// directory (created by the first save). The installed ones come after.
fn presets_dir() -> Option<std::path::PathBuf> {
    let dir = piano_host::config_dir()?;
    let table: Option<toml::Table> = std::fs::read_to_string(dir.join("play.toml")).ok().and_then(|t| t.parse().ok());
    let from_config = table.as_ref().and_then(|t| {
        let s = |k: &str| t.get(k).and_then(|v| v.as_str()).map(std::path::PathBuf::from);
        s("presets").or_else(|| s("preset").and_then(|p| p.parent().map(|d| d.to_path_buf())))
    });
    Some(from_config.filter(|d| d.is_dir()).unwrap_or(dir.join("presets")))
}
