//! What the host sees and saves. The live parameters are host parameters
//! (automatable, in the host's generic editor); the whole instrument and
//! the MIDI map are saved with the project as state.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::RwLock;

use nice_plug::prelude::*;
use piano_core::params::PianoModel;
use piano_params::midi::MidiMap;
use piano_params::{LiveParam, Registry};

pub struct PianoParams {
    pub live: Vec<(LiveParam, FloatParam)>,
    /// The instrument as edited, live fields included (the host parameters
    /// prevail for those).
    pub model: RwLock<PianoModel>,
    pub midi: RwLock<MidiMap>,
    /// The host restored a state: the instrument must be rebuilt from it.
    pub restored: AtomicBool,
}

impl Default for PianoParams {
    fn default() -> Self {
        let registry = Registry::new();
        let model = PianoModel::default();
        let live = LiveParam::ALL
            .iter()
            .map(|&id| {
                let meta = &registry.get(id.path()).expect("live parameter registered").meta;
                let (min, max) = (meta.min as f32, meta.max as f32);
                let range = if meta.log {
                    // Log-like: the middle of the knob near the geometric mean.
                    FloatRange::Skewed { min, max, factor: FloatRange::skew_factor(-1.5) }
                } else {
                    FloatRange::Linear { min, max }
                };
                let p = FloatParam::new(meta.label.clone(), id.get(&model) as f32, range)
                    .with_unit(unit_suffix(meta.unit))
                    .with_value_to_string(std::sync::Arc::new(|v| piano_ui::format_value(v as f64)));
                (id, p)
            })
            .collect();
        Self { live, model: RwLock::new(model), midi: RwLock::new(MidiMap::default()), restored: AtomicBool::new(false) }
    }
}

impl PianoParams {
    pub fn live(&self, id: LiveParam) -> &FloatParam {
        &self.live.iter().find(|(l, _)| *l == id).expect("every live parameter").1
    }

    /// The edited model with the host's values of the live parameters.
    pub fn model(&self) -> PianoModel {
        let mut m = *self.model.read().unwrap();
        for (id, p) in &self.live {
            id.set(&mut m, p.value() as f64);
        }
        m
    }
}

/// The unit as shown after the value (" dB"); units are static strings.
fn unit_suffix(unit: &'static str) -> &'static str {
    match unit {
        "dB" => " dB",
        "s" => " s",
        "m/s" => " m/s",
        "kg" => " kg",
        "N" => " N",
        "N·s/m" => " N·s/m",
        _ => "",
    }
}

/// Host group of a live parameter: its first path segment.
fn group(id: LiveParam) -> String {
    match id.path().split_once('.') {
        Some((g, _)) => g.to_string(),
        None => String::new(),
    }
}

// SAFETY: the pointers point into `live`, which is never resized after
// construction and lives as long as `self` (the plugin keeps it in an Arc).
unsafe impl Params for PianoParams {
    fn param_map(&self) -> Vec<(String, ParamPtr, String)> {
        self.live.iter().map(|(id, p)| (id.path().to_string(), p.as_ptr(), group(*id))).collect()
    }

    fn serialize_fields(&self) -> BTreeMap<String, String> {
        let mut out = BTreeMap::new();
        if let Ok(s) = serde_json::to_string(&*self.model.read().unwrap()) {
            out.insert("model".into(), s);
        }
        if let Ok(s) = serde_json::to_string(&*self.midi.read().unwrap()) {
            out.insert("midi".into(), s);
        }
        out
    }

    fn deserialize_fields(&self, serialized: &BTreeMap<String, String>) {
        if let Some(m) = serialized.get("model").and_then(|s| serde_json::from_str::<PianoModel>(s).ok()) {
            *self.model.write().unwrap() = m;
        }
        if let Some(mut map) = serialized.get("midi").and_then(|s| serde_json::from_str::<MidiMap>(s).ok()) {
            // Parameters renamed since the project was saved.
            let registry = Registry::new();
            map.mappings.retain(|m| m.cc < 128 && registry.get(&m.path).is_some());
            *self.midi.write().unwrap() = map;
        }
        self.restored.store(true, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A project saved before `shank_freq_mid` existed keeps its uniform
    /// shank (the field reads 0); a new one keeps the stiffer middle.
    #[test]
    fn a_state_saved_before_the_stiffer_shank_keeps_its_uniform_shank() {
        let mut v = serde_json::to_value(PianoModel::default()).unwrap();
        v.as_object_mut().unwrap().remove("shank_freq_mid").expect("a top-level field");
        let old: PianoModel = serde_json::from_value(v).unwrap();
        assert_eq!(old.shank_freq_mid, 0.0);
        assert_eq!(old.shank_freq_at(63.0), old.shank_freq);
        let new: PianoModel = serde_json::from_str(&serde_json::to_string(&PianoModel::default()).unwrap()).unwrap();
        assert_eq!(new.shank_freq_mid, PianoModel::default().shank_freq_mid);
    }

    /// A project saved before `damper.behind_strike` existed keeps its
    /// dampers at `position` (the field reads 0).
    #[test]
    fn a_state_saved_before_the_damper_line_keeps_its_dampers_place() {
        let mut v = serde_json::to_value(PianoModel::default()).unwrap();
        v["damper"].as_object_mut().unwrap().remove("behind_strike").expect("a damper field");
        let old: PianoModel = serde_json::from_value(v).unwrap();
        assert_eq!(old.damper.behind_strike, 0.0);
        let new: PianoModel = serde_json::from_str(&serde_json::to_string(&PianoModel::default()).unwrap()).unwrap();
        assert_eq!(new.damper.behind_strike, PianoModel::default().damper.behind_strike);
    }

    /// A project saved before `lid_removed` keeps its lid: an angle of 0
    /// was a removed lid, `lid_shut` = 1 a shut one.
    #[test]
    fn a_state_saved_before_the_removed_lid_keeps_its_lid() {
        let old = |angle: f64, shut: f64| {
            let mut v = serde_json::to_value(PianoModel::default()).unwrap();
            let sb = v["soundboard"].as_object_mut().unwrap();
            sb.remove("lid_removed").expect("a soundboard field");
            sb.insert("lid_angle".into(), angle.into());
            sb.insert("lid_shut".into(), shut.into());
            let sb = serde_json::from_value::<PianoModel>(v).unwrap().soundboard.unwrap();
            (sb.lid_removed, sb.lid_angle)
        };
        assert_eq!(old(0.0, 0.0), (true, 0.0));
        assert_eq!(old(35.0, 0.0), (false, 35.0));
        assert_eq!(old(35.0, 1.0), (false, 0.0));
        let new: PianoModel = serde_json::from_str(&serde_json::to_string(&PianoModel::default()).unwrap()).unwrap();
        assert_eq!(new.soundboard, PianoModel::default().soundboard);
    }
}
