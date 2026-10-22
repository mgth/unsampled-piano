//! Every parameter of the instrument, named by its field path in the preset
//! ("damper.fall_time", "room.size[0]"), read from the model's serde form:
//! a field added to [`PianoModel`] becomes a parameter without being listed
//! here. The path is the stable name used by presets, the interface, MIDI
//! learn and host automation (whose numeric id is [`param_id`]).
//!
//! A parameter is either live ([`LiveParam`]: sent to the engine as an
//! event while playing) or needs the instrument rebuilt.

mod control;
mod meta;

use std::collections::HashMap;

use piano_core::params::PianoModel;
pub use piano_core::LiveParam;
pub use control::{PianoControl, Status};
pub use meta::Meta;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Value {
    Float(f64),
    Int(i64),
    Bool(bool),
}

impl Value {
    pub fn as_f64(self) -> f64 {
        match self {
            Value::Float(v) => v,
            Value::Int(v) => v as f64,
            Value::Bool(v) => v as u8 as f64,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Applied by the running engine ([`piano_core::Event::Param`]).
    Live(LiveParam),
    /// Shapes the precomputed instrument: changing it rebuilds it.
    Rebuild,
}

#[derive(Clone, Debug)]
pub struct Param {
    pub path: String,
    pub id: u32,
    pub kind: Kind,
    /// Value in the default instrument.
    pub default: Value,
    pub meta: Meta,
}

impl Param {
    /// First path segment ("damper"), or "general" for top-level fields.
    pub fn group(&self) -> &str {
        match self.path.split_once(['.', '[']) {
            Some((g, _)) => g,
            None => "general",
        }
    }
}

/// Stable numeric id of a parameter path (FNV-1a), for plugin hosts.
pub fn param_id(path: &str) -> u32 {
    path.bytes().fold(0x811c_9dc5u32, |h, b| (h ^ b as u32).wrapping_mul(0x0100_0193))
}

/// All the parameters of the default instrument, sorted by path.
pub struct Registry {
    params: Vec<Param>,
    by_path: HashMap<String, usize>,
    by_id: HashMap<u32, usize>,
}

impl Registry {
    pub fn new() -> Self {
        let mut leaves = Vec::new();
        flatten(&to_toml(&PianoModel::default()), String::new(), &mut leaves);
        let params: Vec<Param> = leaves
            .into_iter()
            .map(|(path, default)| Param {
                id: param_id(&path),
                kind: LiveParam::from_path(&path).map_or(Kind::Rebuild, Kind::Live),
                meta: Meta::of(&path, default),
                default,
                path,
            })
            .collect();
        let by_path = params.iter().enumerate().map(|(i, p)| (p.path.clone(), i)).collect();
        let by_id = params.iter().enumerate().map(|(i, p)| (p.id, i)).collect();
        Self { params, by_path, by_id }
    }

    pub fn params(&self) -> &[Param] {
        &self.params
    }

    pub fn get(&self, path: &str) -> Option<&Param> {
        self.by_path.get(path).map(|&i| &self.params[i])
    }

    pub fn by_id(&self, id: u32) -> Option<&Param> {
        self.by_id.get(&id).map(|&i| &self.params[i])
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

/// Value of a parameter in `model`; None if the path does not exist there
/// (unknown, or under a section the model disables, like a `None` soundboard).
pub fn get(model: &PianoModel, path: &str) -> Option<Value> {
    if let Some(p) = LiveParam::from_path(path) {
        return Some(Value::Float(p.get(model)));
    }
    let tree = to_toml(model);
    leaf(lookup(&tree, &parse(path)?)?)
}

/// `model` with one parameter changed. The value is converted to the
/// field's type (integers rounded); errors name the path.
pub fn set(model: &PianoModel, path: &str, value: Value) -> Result<PianoModel, String> {
    if let Some(p) = LiveParam::from_path(path) {
        let mut m = *model;
        p.set(&mut m, value.as_f64());
        return Ok(m);
    }
    let mut tree = to_toml(model);
    let segs = parse(path).ok_or_else(|| format!("{path}: malformed path"))?;
    let slot = lookup_mut(&mut tree, &segs).ok_or_else(|| format!("{path}: no such parameter"))?;
    *slot = match (&*slot, value) {
        (toml::Value::Float(_), v) => toml::Value::Float(v.as_f64()),
        (toml::Value::Integer(_), v) => toml::Value::Integer(v.as_f64().round() as i64),
        (toml::Value::Boolean(_), v) => toml::Value::Boolean(v.as_f64() != 0.0),
        _ => return Err(format!("{path}: not a value")),
    };
    tree.try_into().map_err(|e| format!("{path}: {e}"))
}

fn to_toml(model: &PianoModel) -> toml::Value {
    toml::Value::try_from(model).expect("model serialises")
}

fn leaf(v: &toml::Value) -> Option<Value> {
    match *v {
        toml::Value::Float(x) => Some(Value::Float(x)),
        toml::Value::Integer(x) => Some(Value::Int(x)),
        toml::Value::Boolean(x) => Some(Value::Bool(x)),
        _ => None,
    }
}

fn flatten(v: &toml::Value, path: String, out: &mut Vec<(String, Value)>) {
    match v {
        toml::Value::Table(t) => {
            for (k, v) in t {
                let p = if path.is_empty() { k.clone() } else { format!("{path}.{k}") };
                flatten(v, p, out);
            }
        }
        toml::Value::Array(a) => {
            for (i, v) in a.iter().enumerate() {
                flatten(v, format!("{path}[{i}]"), out);
            }
        }
        v => out.extend(leaf(v).map(|x| (path, x))),
    }
}

enum Seg<'a> {
    Key(&'a str),
    Index(usize),
}

/// "a.b[2][1]" → Key(a), Key(b), Index(2), Index(1).
fn parse(path: &str) -> Option<Vec<Seg<'_>>> {
    let mut segs = Vec::new();
    for part in path.split('.') {
        let (key, mut rest) = part.split_at(part.find('[').unwrap_or(part.len()));
        if key.is_empty() {
            return None;
        }
        segs.push(Seg::Key(key));
        while let Some(r) = rest.strip_prefix('[') {
            let (i, r) = r.split_once(']')?;
            segs.push(Seg::Index(i.parse().ok()?));
            rest = r;
        }
        if !rest.is_empty() {
            return None;
        }
    }
    Some(segs)
}

fn lookup<'a>(mut v: &'a toml::Value, segs: &[Seg]) -> Option<&'a toml::Value> {
    for s in segs {
        v = match s {
            Seg::Key(k) => v.as_table()?.get(*k)?,
            Seg::Index(i) => v.as_array()?.get(*i)?,
        };
    }
    Some(v)
}

fn lookup_mut<'a>(mut v: &'a mut toml::Value, segs: &[Seg]) -> Option<&'a mut toml::Value> {
    for s in segs {
        v = match s {
            Seg::Key(k) => v.as_table_mut()?.get_mut(*k)?,
            Seg::Index(i) => v.as_array_mut()?.get_mut(*i)?,
        };
    }
    Some(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_parameter_reads_and_writes() {
        let reg = Registry::new();
        let model = PianoModel::default();
        assert!(reg.params().len() > 100, "{} parameters", reg.params().len());
        for p in reg.params() {
            assert_eq!(get(&model, &p.path), Some(p.default), "{}", p.path);
            let v = match p.default {
                Value::Float(x) => Value::Float(if x == 0.0 { 0.5 } else { x * 0.75 }),
                Value::Int(x) => Value::Int(x - 1),
                Value::Bool(x) => Value::Bool(!x),
            };
            let m = set(&model, &p.path, v).unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(get(&m, &p.path), Some(v), "{}", p.path);
        }
    }

    #[test]
    fn ids_are_unique_and_live_params_registered() {
        let reg = Registry::new();
        for p in reg.params() {
            assert_eq!(reg.by_id(p.id).map(|q| &q.path), Some(&p.path), "id collision on {}", p.path);
        }
        for &l in LiveParam::ALL {
            assert_eq!(reg.get(l.path()).map(|p| p.kind), Some(Kind::Live(l)), "{}", l.path());
        }
    }

    #[test]
    fn paths_and_errors() {
        let reg = Registry::new();
        assert!(reg.get("room.size[0]").is_some());
        assert_eq!(reg.get("damper.fall_time").map(|p| p.group()), Some("damper"));
        assert_eq!(reg.get("hardness").map(|p| p.group()), Some("general"));
        let m = PianoModel::default();
        assert!(set(&m, "damper.nope", Value::Float(1.0)).is_err());
        assert!(set(&m, "damper", Value::Float(1.0)).is_err());
        assert!(get(&m, "room.size[9]").is_none());
        let off = PianoModel { soundboard: None, ..m };
        assert!(get(&off, "soundboard.loss_factor").is_none());
    }
}
