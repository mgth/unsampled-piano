//! unsampled-piano-compare: the model against the recorded real pianos,
//! note by note (docs/compare_tool.md).

mod app;
mod audio;
mod jobs;
mod measure;
mod model;
mod plot;
mod settings;
mod sources;

use clap::Parser;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "unsampled-piano-compare", about = "Compare the model with the recorded real pianos, key by key")]
struct Cli {
    /// Instrument preset of the model (default: the Concert grand).
    #[arg(long)]
    preset: Option<PathBuf>,
    /// The engine's default instrument instead of a preset.
    #[arg(long, conflicts_with = "preset")]
    default_instrument: bool,
    /// Leave the model out (only the recordings).
    #[arg(long)]
    no_model: bool,
    /// Engine quality of the model's renders: overkill, full, balanced, light.
    #[arg(long, default_value = "full")]
    quality: String,
    /// Another note set to show: NAME=DIR (<note>v<layer>.wav|flac);
    /// append :target to count it as a real piano.
    #[arg(long = "set", value_name = "NAME=DIR[:target]")]
    sets: Vec<String>,
    /// Folder of the measure cache and of the model's renders
    /// (default: out/compare-tool in the main checkout).
    #[arg(long)]
    cache: Option<PathBuf>,
    /// Threads measuring the recordings.
    #[arg(long, default_value_t = 4)]
    analysers: usize,
    /// Notes of the model rendered at once (each uses about 3 cores).
    #[arg(long, default_value_t = 2)]
    renderers: usize,
    /// Key shown first (default: the last one).
    #[arg(long)]
    note: Option<String>,
    /// Layer shown first: pp, mf or ff (default: the last one).
    #[arg(long)]
    layer: Option<String>,
}

/// The repository: the folder holding bench/references.json, from the
/// current directory up, else this crate's.
fn repo_root() -> PathBuf {
    if let Ok(cwd) = std::env::current_dir()
        && let Some(r) = cwd.ancestors().find(|a| a.join("bench/references.json").is_file()) {
            return r.to_path_buf();
        }
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap_or_else(|_| PathBuf::from("."))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let root = repo_root();
    let main = sources::main_checkout(&root);
    let cache = cli.cache.unwrap_or_else(|| main.join("out").join("compare-tool"));
    let mut srcs = sources::bench_sources(&root)?;
    for s in &cli.sets {
        let (name, rest) = s.split_once('=').ok_or("--set takes NAME=DIR")?;
        let (dir, target) = match rest.strip_suffix(":target") {
            Some(d) => (d, true),
            None => (rest, false),
        };
        srcs.push(sources::Source {
            key: name.into(),
            label: name.into(),
            description: format!("--set {s}"),
            role: if target { sources::Role::Target } else { sources::Role::Info },
            dynamics: false,
            held: None,
            dir: PathBuf::from(dir),
            files: Default::default(),
        });
    }
    let missing: Vec<String> = srcs.iter().filter(|s| !s.dir.is_dir()).map(|s| format!("{} ({})", s.key, s.dir.display())).collect();
    if !missing.is_empty() {
        eprintln!("note sets not found, left out: {}", missing.join(", "));
        srcs.retain(|s| s.dir.is_dir());
    }
    let spec = if cli.no_model {
        None
    } else {
        let quality = piano_core::Quality::from_name(&cli.quality).ok_or_else(|| format!("unknown quality {:?}", cli.quality))?;
        let default = root.join("presets/instruments/Concert grand/concert_grand.toml");
        let preset = match (&cli.preset, cli.default_instrument) {
            (Some(p), _) => Some(p.clone()),
            (None, false) if default.is_file() => Some(default),
            _ => None,
        };
        let spec = model::load(&root, preset.as_deref(), quality, &cache)?;
        eprintln!("model: {} (renders in {})", spec.label, spec.dir.display());
        Some(spec)
    };
    let cache_file = cache.join(format!("measures-v{}.json", measure::VERSION));
    let viewport = egui::ViewportBuilder::default()
        .with_inner_size([1500.0, 950.0])
        .with_min_inner_size([900.0, 500.0])
        .with_title("Unsampled Piano: compare with real pianos")
        .with_app_id("unsampled-piano-compare");
    let options = eframe::NativeOptions { viewport, ..Default::default() };
    let (analysers, renderers) = (cli.analysers.clamp(1, 8), cli.renderers.clamp(1, 4));
    let note = match &cli.note {
        Some(n) => Some(sources::parse_key(n).ok_or("--note: a key from A0 to C8")?),
        None => None,
    };
    let layer = match &cli.layer {
        Some(l) => Some(sources::parse_layer(l).ok_or("--layer: pp, mf or ff")?),
        None => None,
    };
    eframe::run_native(
        "unsampled-piano-compare",
        options,
        Box::new(move |cc| Ok(Box::new(app::App::new(&cc.egui_ctx, srcs, spec, cache_file, analysers, renderers, note, layer)))),
    )?;
    Ok(())
}
