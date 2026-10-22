//! MIDI learn: controllers (CC) mapped to parameters.
//!
//! A CC mapped to a live parameter is translated on the audio thread
//! ([`RouteTable::translate`]) into an [`Event::Param`], sample-accurate and
//! working without the interface. Every CC is also reported to the non-real-
//! time side ([`MidiLearn::on_cc`]), which captures the controller being
//! learnt, sets the rebuild parameters, and keeps the edited model in step
//! with the live ones. The map belongs to the player's controllers, not to
//! the instrument: it is saved apart from presets ([`MidiMap::to_toml`]).

use piano_core::{Event, LiveParam};
use serde::{Deserialize, Serialize};

use crate::{Kind, Registry, Value};

/// Controllers never learnt: bank select, pedals the engine plays
/// (sustain, sostenuto, una corda) and channel mode messages.
pub fn reserved(cc: u8) -> bool {
    matches!(cc, 0 | 32 | 64 | 66 | 67 | 120..=127)
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Mapping {
    pub cc: u8,
    pub path: String,
    /// Values at CC 0 and 127 (min > max inverts the controller).
    pub min: f64,
    pub max: f64,
}

/// One controller per parameter and one parameter per controller.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MidiMap {
    #[serde(default, rename = "map")]
    pub mappings: Vec<Mapping>,
}

impl MidiMap {
    pub fn for_path(&self, path: &str) -> Option<&Mapping> {
        self.mappings.iter().find(|m| m.path == path)
    }

    pub fn for_cc(&self, cc: u8) -> Option<&Mapping> {
        self.mappings.iter().find(|m| m.cc == cc)
    }

    /// Maps `cc` to `path` over the parameter's display range, replacing
    /// what either had.
    pub fn assign(&mut self, registry: &Registry, cc: u8, path: &str) {
        let Some(p) = registry.get(path) else { return };
        self.mappings.retain(|m| m.cc != cc && m.path != path);
        self.mappings.push(Mapping { cc, path: path.into(), min: p.meta.min, max: p.meta.max });
        self.mappings.sort_by_key(|m| m.cc);
    }

    pub fn unmap(&mut self, path: &str) {
        self.mappings.retain(|m| m.path != path);
    }

    /// The audio thread's table: live parameters only.
    pub fn routes(&self, registry: &Registry) -> RouteTable {
        let mut t = RouteTable::default();
        for m in &self.mappings {
            if let Some(Kind::Live(id)) = registry.get(&m.path).map(|p| p.kind) {
                let log = registry.get(&m.path).is_some_and(|p| p.meta.log);
                t.0[m.cc as usize & 0x7f] = Some(LiveRoute { id, min: m.min as f32, max: m.max as f32, log });
            }
        }
        t
    }

    pub fn to_toml(&self) -> String {
        toml::to_string_pretty(self).expect("map serialises")
    }

    /// Mappings to unknown parameters (renamed since) are dropped.
    pub fn from_toml(text: &str, registry: &Registry) -> Result<Self, String> {
        let mut map: Self = toml::from_str(text).map_err(|e| e.to_string())?;
        map.mappings.retain(|m| m.cc < 128 && registry.get(&m.path).is_some());
        Ok(map)
    }
}

/// Parameter value for a controller position, on the parameter's scale.
pub fn scale(min: f64, max: f64, log: bool, cc_value: u8) -> f64 {
    let t = cc_value.min(127) as f64 / 127.0;
    if log && min > 0.0 && max > 0.0 {
        min * (max / min).powf(t)
    } else {
        min + (max - min) * t
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LiveRoute {
    pub id: LiveParam,
    pub min: f32,
    pub max: f32,
    pub log: bool,
}

/// CC → live parameter, for the audio thread: plain data (no allocation),
/// sent whole when the map changes ([`MidiLearn::take_routes`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RouteTable(pub [Option<LiveRoute>; 128]);

impl Default for RouteTable {
    fn default() -> Self {
        Self([None; 128])
    }
}

impl RouteTable {
    /// A CC mapped to a live parameter becomes its [`Event::Param`]; any
    /// other event is unchanged.
    pub fn translate(&self, e: Event) -> Event {
        match e {
            Event::Control { cc, value } => match self.0[cc as usize & 0x7f] {
                Some(r) => {
                    let v = scale(r.min as f64, r.max as f64, r.log, value);
                    Event::Param { id: r.id, value: v as f32 }
                }
                None => e,
            },
            e => e,
        }
    }
}

/// What the host does with a reported CC.
#[derive(Clone, Debug, PartialEq)]
pub enum CcAction {
    None,
    /// The controller was learnt for this parameter.
    Learnt { cc: u8, path: String },
    /// Set the parameter. `live`: the audio thread already applied it, the
    /// host only updates its model; otherwise this edit rebuilds.
    Set { path: String, value: Value, live: bool },
}

/// Travel (CC steps) after which the controller moved is learnt: an X/Y
/// joystick sends both axes, the one not aimed at only jitters.
pub const LEARN_TRAVEL: u16 = 12;

/// The map being edited, the parameter waiting for a controller, and
/// whether the audio thread's table is stale.
pub struct MidiLearn {
    registry: Registry,
    map: MidiMap,
    learning: Option<String>,
    dirty: bool,
    /// Last value of each controller, and its travel since learning began.
    last: [Option<u8>; 128],
    travel: [u16; 128],
}

impl MidiLearn {
    pub fn new(map: MidiMap) -> Self {
        Self { registry: Registry::new(), map, learning: None, dirty: true, last: [None; 128], travel: [0; 128] }
    }

    pub fn map(&self) -> &MidiMap {
        &self.map
    }

    /// The next controller moved over [`LEARN_TRAVEL`] will drive `path`.
    pub fn learn(&mut self, path: &str) {
        self.learning = Some(path.into());
        self.travel = [0; 128];
    }

    pub fn cancel(&mut self) {
        self.learning = None;
    }

    pub fn learning(&self) -> Option<&str> {
        self.learning.as_deref()
    }

    pub fn unmap(&mut self, path: &str) {
        self.map.unmap(path);
        self.dirty = true;
    }

    /// Changes the values at CC 0 and 127.
    pub fn set_range(&mut self, path: &str, min: f64, max: f64) {
        if let Some(m) = self.map.mappings.iter_mut().find(|m| m.path == path) {
            (m.min, m.max) = (min, max);
            self.dirty = true;
        }
    }

    pub fn replace(&mut self, map: MidiMap) {
        self.map = map;
        self.dirty = true;
    }

    /// A CC received (non-real-time side, every one the audio side saw).
    pub fn on_cc(&mut self, cc: u8, value: u8) -> CcAction {
        let (cc, value) = (cc & 0x7f, value.min(127));
        let step = self.last[cc as usize].map_or(0, |l| l.abs_diff(value) as u16);
        self.last[cc as usize] = Some(value);
        if self.learning.is_some() && !reserved(cc) {
            self.travel[cc as usize] += step;
            if self.travel[cc as usize] >= LEARN_TRAVEL {
                let path = self.learning.take().expect("learning");
                self.map.assign(&self.registry, cc, &path);
                self.dirty = true;
                return CcAction::Learnt { cc, path };
            }
        }
        let Some(m) = self.map.for_cc(cc) else { return CcAction::None };
        let Some(p) = self.registry.get(&m.path) else { return CcAction::None };
        let x = scale(m.min, m.max, p.meta.log, value);
        let value = match p.default {
            Value::Float(_) => Value::Float(x),
            Value::Int(_) => Value::Int(x.round() as i64),
            Value::Bool(_) => Value::Bool(value >= 64),
        };
        CcAction::Set { path: m.path.clone(), value, live: matches!(p.kind, Kind::Live(_)) }
    }

    /// The audio thread's table, once after each change of the map.
    pub fn take_routes(&mut self) -> Option<RouteTable> {
        std::mem::take(&mut self.dirty).then(|| self.map.routes(&self.registry))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Turns a controller from 0 to 40 (enough to be learnt).
    fn turn(l: &mut MidiLearn, cc: u8) {
        for v in (0..=40).step_by(4) {
            l.on_cc(cc, v);
        }
    }

    #[test]
    fn joystick_axis_moved_most_is_learnt() {
        let mut l = MidiLearn::new(MidiMap::default());
        // Joystick resting at (70, 40), no return to centre; X is reported first.
        l.on_cc(16, 70);
        l.on_cc(17, 40);
        l.learn("hardness");
        // Aiming at Y: X jitters by a few steps on the way.
        for (x, y) in [(71, 44), (69, 48), (72, 52), (70, 56)] {
            assert_eq!(l.on_cc(16, x), CcAction::None);
            if let CcAction::Learnt { cc, .. } = l.on_cc(17, y) {
                assert_eq!(cc, 17);
                assert_eq!(l.map().for_path("hardness").map(|m| m.cc), Some(17));
                return;
            }
        }
        panic!("Y never learnt");
    }

    #[test]
    fn learn_then_drive() {
        let mut l = MidiLearn::new(MidiMap::default());
        assert!(l.take_routes().is_some());
        l.learn("output_gain_db");
        // Pedals are not learnt, and a pedal moved meanwhile is no action.
        assert_eq!(l.on_cc(64, 127), CcAction::None);
        assert_eq!(l.on_cc(64, 0), CcAction::None);
        assert_eq!(l.on_cc(21, 5), CcAction::None);
        assert_eq!(l.on_cc(21, 30), CcAction::Learnt { cc: 21, path: "output_gain_db".into() });
        assert_eq!(l.learning(), None);
        let routes = l.take_routes().expect("map changed");
        assert!(l.take_routes().is_none());
        // Audio side: −40…+20 dB over the controller.
        let e = routes.translate(Event::Control { cc: 21, value: 127 });
        assert_eq!(e, Event::Param { id: LiveParam::OutputGainDb, value: 20.0 });
        assert_eq!(routes.translate(Event::Control { cc: 22, value: 1 }), Event::Control { cc: 22, value: 1 });
        // Host side: the model follows, no second event.
        assert_eq!(l.on_cc(21, 0), CcAction::Set { path: "output_gain_db".into(), value: Value::Float(-40.0), live: true });
    }

    #[test]
    fn rebuild_parameters_and_one_to_one() {
        let mut l = MidiLearn::new(MidiMap::default());
        l.learn("hardness");
        turn(&mut l, 1);
        // Rebuild parameters stay off the audio thread's table.
        assert_eq!(l.take_routes().unwrap(), RouteTable::default());
        match l.on_cc(1, 127) {
            CcAction::Set { path, value: Value::Float(v), live: false } => assert!(path == "hardness" && (v - 2.0).abs() < 1e-9),
            a => panic!("{a:?}"),
        }
        // Log scale: the middle of the controller is the geometric mean.
        let mid = scale(0.02, 2.0, true, 127) * scale(0.02, 2.0, true, 0);
        assert!((mid - 0.04).abs() < 1e-12);
        // Learning another parameter on CC 1 takes it over.
        l.learn("hysteresis");
        turn(&mut l, 1);
        assert_eq!(l.map().mappings.len(), 1);
        assert_eq!(l.map().for_cc(1).map(|m| m.path.as_str()), Some("hysteresis"));
        l.learn("hysteresis");
        turn(&mut l, 2);
        assert_eq!(l.map().mappings.len(), 1);
        assert_eq!(l.map().for_path("hysteresis").map(|m| m.cc), Some(2));
    }

    #[test]
    fn integers_booleans_and_persistence() {
        let reg = Registry::new();
        let mut l = MidiLearn::new(MidiMap::default());
        l.learn("damper.highest_note");
        turn(&mut l, 30);
        l.learn("sympathetic.enabled");
        turn(&mut l, 31);
        assert!(matches!(l.on_cc(30, 64), CcAction::Set { value: Value::Int(_), .. }));
        assert!(matches!(l.on_cc(31, 100), CcAction::Set { value: Value::Bool(true), .. }));
        let text = l.map().to_toml();
        assert_eq!(&MidiMap::from_toml(&text, &reg).unwrap(), l.map());
        let stale = text.replace("damper.highest_note", "damper.gone");
        assert_eq!(MidiMap::from_toml(&stale, &reg).unwrap().mappings.len(), 1);
    }
}
