//! Every parameter as JSON, for the website's reference: path, page,
//! live or rebuild, default, label, unit, range and help, plus each page's
//! per-key curves evaluated on the default instrument.
//!   cargo run -p piano-ui --example param_reference > website/src/data/params.json
use piano_core::params::PianoModel;
use piano_params::{help, Kind, Registry, Value};
use piano_ui::sections::{section_of, SECTIONS};
use serde_json::{json, Value as Json};

fn value(v: Value) -> Json {
    match v {
        Value::Float(x) => json!(x),
        Value::Int(x) => json!(x),
        Value::Bool(x) => json!(x),
    }
}

fn main() {
    let reg = Registry::new();
    let model = PianoModel::default();
    let params: Vec<Json> = reg
        .params()
        .iter()
        .map(|p| {
            let h = help(&p.path);
            json!({
                "path": p.path,
                "id": p.id,
                "page": SECTIONS[section_of(&p.path)].title,
                "live": matches!(p.kind, Kind::Live(_)),
                "type": match p.default { Value::Float(_) => "float", Value::Int(_) => "int", Value::Bool(_) => "bool" },
                "default": value(p.default),
                "label": p.meta.label,
                "unit": p.meta.unit,
                "min": p.meta.min,
                "max": p.meta.max,
                "log": p.meta.log,
                "curated": p.meta.curated,
                "what": h.map(|h| h.what),
                "how": h.map(|h| h.how),
            })
        })
        .collect();
    let pages: Vec<Json> = SECTIONS
        .iter()
        .filter(|s| s.title != "Other")
        .map(|s| {
            let curves: Vec<Json> = s
                .curves
                .iter()
                .map(|c| {
                    let points: Vec<[f64; 2]> = (c.x.0..=c.x.1).map(|x| [x as f64, (c.f)(&model, x)]).collect();
                    json!({ "title": c.title, "unit": c.unit, "log": c.log, "points": points })
                })
                .collect();
            json!({ "title": s.title, "curves": curves })
        })
        .collect();
    let out = json!({ "pages": pages, "params": params });
    println!("{}", serde_json::to_string_pretty(&out).expect("serialises"));
}
