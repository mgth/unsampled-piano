//! The pianos compared: the bench's note sets (bench/references.json) and
//! the model. A note set holds `<note>v<layer>.{wav,flac}`, layers 4/8/13
//! (pp/mf/ff), as tools/split_notes.py lays them out.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const LAYERS: [(u8, &str, u8); 3] = [(4, "pp", 45), (8, "mf", 56), (13, "ff", 100)];
pub const LOW: u8 = 21;
pub const HIGH: u8 = 108;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Role {
    /// A real piano: enters the band and the model's z.
    Target,
    /// Shown, never a target (an upright, another model).
    Info,
    /// Our model.
    Model,
}

pub struct Source {
    pub key: String,
    pub label: String,
    pub description: String,
    pub role: Role,
    /// Levels tied to known velocities (ff − pp is meaningful).
    pub dynamics: bool,
    pub dir: PathBuf,
    /// Seconds after the onset at which these recordings release the key,
    /// when that falls within the measures (3.3 s).
    pub held: Option<f64>,
    /// (midi, layer) → file.
    pub files: BTreeMap<(u8, u8), PathBuf>,
}

impl Source {
    pub fn scan(&mut self) {
        self.files.clear();
        let Ok(rd) = std::fs::read_dir(&self.dir) else { return };
        for e in rd.flatten() {
            let p = e.path();
            let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("");
            if !matches!(ext, "wav" | "flac") {
                continue;
            }
            let Some(stem) = p.file_stem().and_then(|s| s.to_str()) else { continue };
            let Some((n, l)) = stem.rsplit_once('v') else { continue };
            if let (Some(m), Ok(l)) = (piano_analysis::io::parse_note_name(n), l.parse::<u8>())
                && (LOW..=HIGH).contains(&m) && LAYERS.iter().any(|x| x.0 == l) {
                    self.files.insert((m, l), p);
                }
        }
    }
}

pub fn note_name(m: u8) -> String {
    const N: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
    format!("{}{}", N[m as usize % 12], m as i32 / 12 - 1)
}

pub fn parse_key(s: &str) -> Option<u8> {
    piano_analysis::io::parse_note_name(s).filter(|m| (LOW..=HIGH).contains(m))
}

/// Layer number of "pp", "mf" or "ff".
pub fn parse_layer(s: &str) -> Option<u8> {
    LAYERS.iter().find(|l| l.1 == s).map(|l| l.0)
}

pub fn is_black(m: u8) -> bool {
    matches!(m % 12, 1 | 3 | 6 | 8 | 10)
}

fn label(key: &str) -> &str {
    match key {
        "iowa" => "Iowa · Steinway B",
        "salamander" => "Salamander · Yamaha C5",
        "headroom" => "Headroom · Yamaha C3",
        "splendid" => "Splendid · Steinway",
        "helgesson" => "Helgesson · Yamaha CF-3",
        "liang" => "Liang · Disklavier grand",
        "plath" => "Plath · concert grand",
        "bivib_grand" => "BiVib · Disklavier grand",
        "bivib_upright" => "BiVib · upright",
        "maps_close" => "MAPS · upright",
        "commercial" => "Commercial (bench set)",
        _ => key,
    }
}

/// The main checkout of a worktree (where out/ lives), or `root` itself.
pub fn main_checkout(root: &Path) -> PathBuf {
    let git = root.join(".git");
    if let Ok(text) = std::fs::read_to_string(&git) {
        // "gitdir: <main>/.git/worktrees/<name>"
        if let Some(dir) = text.trim().strip_prefix("gitdir:") {
            let dir = PathBuf::from(dir.trim());
            if let Some(main) = dir.ancestors().find(|a| a.file_name().is_some_and(|n| n == ".git")).and_then(|g| g.parent()) {
                return main.to_path_buf();
            }
        }
    }
    root.to_path_buf()
}

/// Reads bench/references.json; set directories are looked up in the
/// repository, then in the main checkout.
pub fn bench_sources(root: &Path) -> Result<Vec<Source>, String> {
    let path = root.join("bench").join("references.json");
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let json: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    let main = main_checkout(root);
    let mut out = Vec::new();
    let Some(sources) = json.get("sources").and_then(|s| s.as_object()) else { return Ok(out) };
    for (key, s) in sources {
        let rel = s.get("dir").and_then(|d| d.as_str()).unwrap_or_default();
        let dir = [root.join(rel), main.join(rel)].into_iter().find(|d| d.is_dir()).unwrap_or_else(|| main.join(rel));
        let opts: Vec<&str> = s.get("options").and_then(|o| o.as_array()).map(|a| a.iter().filter_map(|x| x.as_str()).collect()).unwrap_or_default();
        let role = if s.get("role").and_then(|r| r.as_str()) == Some("info") { Role::Info } else { Role::Target };
        // Liang's and MAPS's isolated notes are released 1.9 to 2.0 s after
        // their onset (their level drops 15 to 26 dB there); the others hold
        // past 3.3 s.
        let held = matches!(key.as_str(), "liang" | "maps_close").then_some(1.85);
        out.push(Source {
            held,
            key: key.clone(),
            label: label(key).to_string(),
            description: s.get("description").and_then(|d| d.as_str()).unwrap_or_default().to_string(),
            role,
            dynamics: !opts.contains(&"nodyn"),
            dir,
            files: BTreeMap::new(),
        });
    }
    // The commercial modelled piano's D, the Concert grand's reference
    // (docs/steinway_d.md), which the bench does not hold: all 88 keys.
    let d = main.join("out/catalog_clean/steinway_d");
    if d.is_dir() {
        out.push(Source {
            key: "commercial_d".into(),
            label: "Commercial D".into(),
            description: "The commercial modelled piano's D (the Concert grand's reference), recorded dry on 2026-11-01, effects off: all 88 keys at velocities 60 (shown as mf) and 100 (ff), held 3.5 s (docs/steinway_d.md, docs/instrument_range.md). Another model: info only".into(),
            role: Role::Info,
            dynamics: false,
            held: None,
            dir: d,
            files: BTreeMap::new(),
        });
    }
    // Real pianos first, then the others; the bench's order within.
    out.sort_by_key(|s| s.role != Role::Target);
    Ok(out)
}
