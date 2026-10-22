//! Lists every parameter: path, live or rebuild, default, range.
//!   cargo run -p piano-params --example list_params
use piano_params::{Kind, Registry};

fn main() {
    let reg = Registry::new();
    for p in reg.params() {
        let kind = match p.kind {
            Kind::Live(_) => "live",
            Kind::Rebuild => "rebuild",
        };
        let m = &p.meta;
        let range = format!("[{}, {}]{}{}", m.min, m.max, if m.log { " log" } else { "" }, if m.curated { "" } else { " ?" });
        println!("{:<40} {:<8} {:>12} {:<6} {range}", p.path, kind, p.default.as_f64(), m.unit);
    }
    let live = reg.params().iter().filter(|p| matches!(p.kind, Kind::Live(_))).count();
    println!("{} parameters, {live} live", reg.params().len());
}
