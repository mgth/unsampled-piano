//! The tools: list_params, get, set, load_preset, status. Each returns a
//! JSON object, sent both as text and as the structured result; a refused
//! call (unknown path, value out of range, unreadable preset) is a tool
//! error the assistant reads, not a protocol error.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use piano_params::{help, Kind, Param, PianoControl, Value};
use serde_json::{json, Value as Json};

use crate::Server;

/// Longest wait for a rebuild before answering (it takes ≈ 0.1 s after a
/// 150 ms pause; the audio thread swaps the new engine in).
const REBUILD_WAIT: Duration = Duration::from_secs(3);

type ToolResult = Result<Json, String>;

pub(crate) fn list<C: PianoControl>(server: &Server<C>) -> Vec<Json> {
    let presets: Vec<String> = server.presets.as_deref().map(discover).unwrap_or_default().into_iter().map(|p| p.id).collect();
    let mut name = json!({ "type": "string", "description": "Preset id from the preset folder (or a file name, when unique)." });
    if !presets.is_empty() {
        name["enum"] = json!(presets);
    }
    vec![
        json!({
            "name": "list_params",
            "title": "List parameters",
            "description": "Lists the instrument's parameters with their current value, default, unit, range \
                and kind (live: applied at once; rebuild: the instrument is rebuilt). Filter by a word of \
                the path or label (\"damper\", \"hardness\", \"mic\") or by group (first path segment, \
                \"general\" for top-level fields). A \"guessed\" range is only an order of magnitude around \
                the default.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "filter": { "type": "string", "description": "Case-insensitive word found in the path or label." },
                    "group": { "type": "string", "description": "First segment of the path: damper, soundboard, room, general…" },
                },
            },
            "annotations": { "readOnlyHint": true },
        }),
        json!({
            "name": "get",
            "title": "Read a parameter",
            "description": "One parameter: its value, default, unit, range, kind, and what it changes on the \
                piano (\"what\") and how the model implements it (\"how\").",
            "inputSchema": {
                "type": "object",
                "properties": { "path": { "type": "string", "description": "Parameter path, e.g. \"damper.fall_time\" or \"room.size[0]\"." } },
                "required": ["path"],
            },
            "annotations": { "readOnlyHint": true },
        }),
        json!({
            "name": "set",
            "title": "Change a parameter",
            "description": "Sets one parameter of the playing instrument; the user hears it at once. A live \
                parameter applies immediately; any other rebuilds the instrument in the background \
                (≈ 0.1 s, held notes keep sounding) and the reply comes once it is in. Values outside a \
                curated range are refused; outside a guessed range they are accepted with a warning.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Parameter path." },
                    "value": { "type": ["number", "boolean"], "description": "New value, in the parameter's unit (integers are rounded)." },
                },
                "required": ["path", "value"],
            },
            "annotations": { "readOnlyHint": false, "destructiveHint": false, "idempotentHint": true },
        }),
        json!({
            "name": "load_preset",
            "title": "Load a preset",
            "description": "Loads a preset into the playing instrument. An instrument preset (instruments/…) \
                keeps the current listening set-up (microphones, lid, room); a listening preset \
                (listening/…) changes only that set-up; a preset elsewhere replaces everything. Give a \
                name from the preset folder or a file path; \"as\" overrides how it is applied.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": name,
                    "path": { "type": "string", "description": "Path of a preset file (TOML), instead of a name." },
                    "as": { "type": "string", "enum": ["instrument", "listening", "full"], "description": "How to apply it (default: from its folder)." },
                },
            },
            "annotations": { "readOnlyHint": false, "destructiveHint": false, "idempotentHint": true },
        }),
        json!({
            "name": "status",
            "title": "Player status",
            "description": "Whether a rebuild is pending, how long the last build took, the voices sounding, \
                the audio thread's CPU load and the last error.",
            "inputSchema": { "type": "object", "properties": {} },
            "annotations": { "readOnlyHint": true },
        }),
    ]
}

pub(crate) fn call<C: PianoControl>(server: &Server<C>, params: &Json) -> Result<Json, (i64, String)> {
    let name = params.get("name").and_then(Json::as_str).ok_or((-32602, "tools/call: missing name".to_string()))?;
    let args = params.get("arguments").cloned().unwrap_or_else(|| json!({}));
    let result = match name {
        "list_params" => list_params(server, &args),
        "get" => get(server, &args),
        "set" => set(server, &args),
        "load_preset" => load_preset(server, &args),
        "status" => Ok(status(server)),
        _ => return Err((-32602, format!("unknown tool: {name}"))),
    };
    Ok(match result {
        Ok(v) => json!({
            "content": [{ "type": "text", "text": serde_json::to_string_pretty(&v).expect("JSON") }],
            "structuredContent": v,
            "isError": false,
        }),
        Err(e) => json!({ "content": [{ "type": "text", "text": e }], "isError": true }),
    })
}

fn list_params<C: PianoControl>(server: &Server<C>, args: &Json) -> ToolResult {
    let filter = args.get("filter").and_then(Json::as_str).map(str::to_lowercase);
    let group = args.get("group").and_then(Json::as_str);
    let model = server.control.lock().unwrap().model();
    let values = server.registry.read(&model);
    let params: Vec<Json> = server
        .registry
        .params()
        .iter()
        .zip(values)
        .filter(|(p, _)| group.is_none_or(|g| p.group() == g))
        .filter(|(p, _)| filter.as_ref().is_none_or(|f| p.path.to_lowercase().contains(f) || p.meta.label.to_lowercase().contains(f)))
        .map(|(p, v)| describe(p, v))
        .collect();
    if params.is_empty() {
        let mut groups: Vec<&str> = server.registry.params().iter().map(Param::group).collect();
        groups.dedup();
        return Err(format!("no parameter matches; groups: {}", groups.join(", ")));
    }
    Ok(json!({ "count": params.len(), "params": params }))
}

fn get<C: PianoControl>(server: &Server<C>, args: &Json) -> ToolResult {
    let p = param(server, args)?;
    let value = piano_params::get(&server.control.lock().unwrap().model(), &p.path);
    let mut out = describe(p, value);
    if let Some(h) = help(&p.path) {
        out["what"] = json!(h.what);
        out["how"] = json!(h.how);
    }
    Ok(out)
}

fn set<C: PianoControl>(server: &Server<C>, args: &Json) -> ToolResult {
    let p = param(server, args)?;
    let value = match args.get("value") {
        Some(Json::Bool(b)) => Value::Bool(*b),
        Some(Json::Number(n)) => match n.as_i64() {
            Some(i) => Value::Int(i),
            None => Value::Float(n.as_f64().ok_or("value: not a finite number")?),
        },
        _ => return Err("value: a number or a boolean".into()),
    };
    let mut warning = None;
    if !matches!(p.default, Value::Bool(_)) {
        let (x, m) = (value.as_f64(), &p.meta);
        if !(m.min..=m.max).contains(&x) {
            let range = format!("[{}, {}] {}", m.min, m.max, m.unit);
            if m.curated {
                return Err(format!("{}: {x} is outside its range {}", p.path, range.trim_end()));
            }
            warning = Some(format!("{x} is outside the guessed range {}: listen for artefacts", range.trim_end()));
        }
    }
    let mut control = server.control.lock().unwrap();
    let previous = piano_params::get(&control.model(), &p.path)
        .ok_or_else(|| format!("{}: not in the current instrument (its section is switched off)", p.path))?;
    piano_params::set(&control.model(), &p.path, value)?;
    control.begin_edit(&p.path);
    control.set(&p.path, value);
    control.end_edit(&p.path);
    let now = piano_params::get(&control.model(), &p.path);
    drop(control);
    let mut out = json!({ "path": p.path, "previous": json_value(Some(previous)), "value": json_value(now) });
    match p.kind {
        Kind::Live(_) => out["applied"] = json!("immediately"),
        Kind::Rebuild => merge(&mut out, wait_rebuild(server)),
    }
    if let Some(w) = warning {
        out["warning"] = json!(w);
    }
    Ok(out)
}

fn load_preset<C: PianoControl>(server: &Server<C>, args: &Json) -> ToolResult {
    let presets = server.presets.as_deref().map(discover).unwrap_or_default();
    let available = || presets.iter().map(|p| p.id.as_str()).collect::<Vec<_>>().join(", ");
    let (path, kind) = match (args.get("name").and_then(Json::as_str), args.get("path").and_then(Json::as_str)) {
        (Some(name), _) => {
            let by_stem: Vec<&Preset> = presets.iter().filter(|p| p.path.file_stem().is_some_and(|s| s == name)).collect();
            match presets.iter().find(|p| p.id == name) {
                Some(p) => (p.path.clone(), p.kind),
                None => match by_stem.as_slice() {
                    [p] => (p.path.clone(), p.kind),
                    [] => return Err(format!("no preset {name:?}; available: {}", available())),
                    _ => return Err(format!("{name:?} is ambiguous: {}", by_stem.iter().map(|p| p.id.as_str()).collect::<Vec<_>>().join(", "))),
                },
            }
        }
        (None, Some(path)) => {
            let path = PathBuf::from(path);
            let kind = kind_of(&path);
            (path, kind)
        }
        (None, None) => return Err(format!("give a name or a path; available: {}", available())),
    };
    let kind = match args.get("as").and_then(Json::as_str) {
        None => kind,
        Some("instrument") => PresetKind::Instrument,
        Some("listening") => PresetKind::Listening,
        Some("full") => PresetKind::Full,
        Some(other) => return Err(format!("as: {other:?} is not instrument, listening or full")),
    };
    let mut control = server.control.lock().unwrap();
    let model = control.model();
    let new = match kind {
        PresetKind::Instrument => piano_host::load_instrument(&path, &model),
        PresetKind::Listening => piano_host::load_listening(&path, &model),
        PresetKind::Full => piano_host::load_preset(&path),
    }?;
    control.load(new);
    drop(control);
    let mut out = json!({ "loaded": path.display().to_string(), "as": kind.name() });
    merge(&mut out, wait_rebuild(server));
    Ok(out)
}

fn status<C: PianoControl>(server: &Server<C>) -> Json {
    let st = server.control.lock().unwrap().status();
    json!({
        "rebuilding": st.rebuilding,
        "last_build_ms": st.last_build.map(|d| (d.as_secs_f64() * 1e3).round()),
        "voices": st.voices,
        "cpu_load_percent": (st.load as f64 * 1e3).round() / 10.0,
        "error": st.error,
        "presets": server.presets.as_ref().map(|p| p.display().to_string()),
    })
}

/// Waits (lock released) until the rebuild requested is playing, or fails.
fn wait_rebuild<C: PianoControl>(server: &Server<C>) -> Json {
    let start = Instant::now();
    loop {
        let st = server.control.lock().unwrap().status();
        // A failed build's error stays until the next build succeeds: only
        // read it once this one is over.
        if !st.rebuilding {
            return match st.error {
                Some(e) => json!({ "applied": "rebuild failed", "error": e }),
                None => json!({ "applied": "instrument rebuilt", "build_ms": st.last_build.map(|d| (d.as_secs_f64() * 1e3).round()) }),
            };
        }
        if start.elapsed() > REBUILD_WAIT {
            return json!({ "applied": "still rebuilding (check status)" });
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn param<'a, C: PianoControl>(server: &'a Server<C>, args: &Json) -> Result<&'a Param, String> {
    let path = args.get("path").and_then(Json::as_str).ok_or("path: missing")?;
    server.registry.get(path).ok_or_else(|| {
        // Suggest paths sharing the last word ("fall_time" → "damper.fall_time").
        let word = path.rsplit(['.', ']']).find(|w| !w.is_empty()).unwrap_or(path).split('[').next().unwrap_or("").to_lowercase();
        let near: Vec<&str> = server.registry.params().iter().map(|p| p.path.as_str()).filter(|p| !word.is_empty() && p.to_lowercase().contains(&word)).take(10).collect();
        if near.is_empty() { format!("no parameter {path:?} (see list_params)") } else { format!("no parameter {path:?}; did you mean {}?", near.join(", ")) }
    })
}

fn describe(p: &Param, value: Option<Value>) -> Json {
    let m = &p.meta;
    let mut out = json!({
        "path": p.path,
        "label": m.label,
        "value": json_value(value),
        "default": json_value(Some(p.default)),
        "kind": match p.kind { Kind::Live(_) => "live", Kind::Rebuild => "rebuild" },
    });
    if !m.unit.is_empty() {
        out["unit"] = json!(m.unit);
    }
    if !matches!(p.default, Value::Bool(_)) {
        out["range"] = json!([m.min, m.max]);
        out["range_kind"] = json!(if m.curated { "curated" } else { "guessed" });
    }
    out
}

/// None (a switched-off section) is null.
fn json_value(v: Option<Value>) -> Json {
    match v {
        Some(Value::Float(x)) => json!(x),
        Some(Value::Int(x)) => json!(x),
        Some(Value::Bool(x)) => json!(x),
        None => Json::Null,
    }
}

fn merge(out: &mut Json, more: Json) {
    if let (Some(o), Json::Object(m)) = (out.as_object_mut(), more) {
        o.extend(m);
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum PresetKind {
    Instrument,
    Listening,
    Full,
}

impl PresetKind {
    fn name(self) -> &'static str {
        match self {
            Self::Instrument => "instrument",
            Self::Listening => "listening",
            Self::Full => "full",
        }
    }
}

#[derive(Debug)]
struct Preset {
    /// Path from the preset folder without the extension
    /// ("instruments/Character/honky_tonk").
    id: String,
    path: PathBuf,
    kind: PresetKind,
}

/// The presets of a folder laid out like the interface expects:
/// `instruments/[<family>/]*.toml` and `listening/[<group>/]*.toml`, else
/// loose `*.toml` files (whole presets).
fn discover(dir: &Path) -> Vec<Preset> {
    let mut out = Vec::new();
    let roots = [("instruments", PresetKind::Instrument), ("listening", PresetKind::Listening)];
    if roots.iter().any(|(r, _)| dir.join(r).is_dir()) {
        for (root, kind) in roots {
            let base = dir.join(root);
            let mut files = tomls(&base);
            files.extend(subdirs(&base).iter().flat_map(|d| tomls(d)));
            out.extend(files.into_iter().map(|path| Preset { id: id(dir, &path), path, kind }));
        }
    } else {
        out.extend(tomls(dir).into_iter().map(|path| Preset { id: id(dir, &path), path, kind: PresetKind::Full }));
    }
    out
}

fn id(dir: &Path, path: &Path) -> String {
    let rel = path.strip_prefix(dir).unwrap_or(path).with_extension("");
    rel.components().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/")
}

/// From the folder a file sits in.
fn kind_of(path: &Path) -> PresetKind {
    let parts: Vec<_> = path.parent().into_iter().flat_map(Path::components).map(|c| c.as_os_str().to_string_lossy().into_owned()).collect();
    match parts.iter().rev().find(|c| *c == "listening" || *c == "instruments").map(String::as_str) {
        Some("listening") => PresetKind::Listening,
        Some(_) => PresetKind::Instrument,
        None => PresetKind::Full,
    }
}

fn subdirs(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
    v.sort();
    v
}

fn tomls(dir: &Path) -> Vec<PathBuf> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preset_kinds() {
        assert_eq!(kind_of(Path::new("presets/listening/Room/dry.toml")), PresetKind::Listening);
        assert_eq!(kind_of(Path::new("/x/instruments/Grand/a.toml")), PresetKind::Instrument);
        assert_eq!(kind_of(Path::new("presets/default.toml")), PresetKind::Full);
    }

    #[test]
    fn repo_presets() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../presets");
        let ids: Vec<(String, PresetKind)> = discover(&dir).into_iter().map(|p| (p.id, p.kind)).collect();
        assert!(ids.contains(&("instruments/Character/honky_tonk".into(), PresetKind::Instrument)), "{ids:?}");
        assert!(ids.contains(&("listening/Room/dry".into(), PresetKind::Listening)), "{ids:?}");
        assert!(!ids.iter().any(|(i, _)| i == "default"), "loose files are not offered in a structured folder");
    }
}
