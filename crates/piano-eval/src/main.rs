//! Renders the notes and velocities of a reference sample set (named
//! `<note>v<layer>`) with the model and measures both with the same
//! analysers. One CSV row per (source, note, layer, partial).

use clap::Parser;
use piano_analysis::{distance, inharm, io};
use piano_core::{Engine, EngineConfig, Event, TimedEvent};
use std::io::Write;
use std::path::PathBuf;

#[derive(Parser)]
struct Args {
    /// Directory of reference samples.
    reference: PathBuf,
    #[arg(long, default_value = "eval.csv")]
    out: PathBuf,
    /// MIDI velocity used for each layer, comma-separated (16 values for Salamander).
    #[arg(long, value_delimiter = ',', default_values_t = [13u8, 30, 35, 40, 45, 48, 53, 60, 68, 76, 84, 92, 100, 108, 116, 124])]
    velocities: Vec<u8>,
    /// Seconds rendered per note.
    #[arg(long, default_value_t = 2.0)]
    seconds: f64,
    /// Partials reported.
    #[arg(long, default_value_t = 16)]
    partials: u32,
    /// Model overrides (defaults: the engine's).
    #[arg(long)]
    p_offset: Option<f64>,
    #[arg(long)]
    hardness: Option<f64>,
    #[arg(long)]
    vmin: Option<f64>,
    #[arg(long)]
    vmax: Option<f64>,
    /// Skip the reference measurements (faster parameter sweeps).
    #[arg(long)]
    model_only: bool,
}

struct Measure {
    peak_db: f64,
    centroid: f64,
    f0: f64,
    b: f64,
    levels: Vec<(u32, f64)>,
}

fn measure(x: &[f32], sr: f64, note: u8, partials: u32) -> Option<Measure> {
    let f_nom = 440.0 * 2f64.powf((note as f64 - 69.0) / 12.0);
    let fit = inharm::estimate(x, sr, f_nom, inharm::Options::default())?;
    let peak = x.iter().fold(0.0f32, |m, v| m.max(v.abs()));
    Some(Measure {
        peak_db: 20.0 * (peak.max(1e-12) as f64).log10(),
        centroid: distance::centroid(x, sr, fit.onset, (0.05 * sr) as usize),
        f0: fit.f0,
        b: fit.b,
        levels: fit.partials.iter().filter(|p| p.k <= partials).map(|p| (p.k, p.level_db)).collect(),
    })
}

fn render(cfg: EngineConfig, note: u8, velocity: u8, seconds: f64) -> Vec<f32> {
    let mut e = Engine::new(cfg);
    let bs = cfg.max_block;
    let n = (seconds * cfg.sample_rate) as usize;
    let mut out = vec![0.0f32; n.next_multiple_of(bs)];
    for (i, blk) in out.chunks_mut(bs).enumerate() {
        let ev = [TimedEvent { frame: 0, event: Event::NoteOn { note, velocity } }];
        e.process(if i == 0 { &ev } else { &[] }, blk);
    }
    out.truncate(n);
    out
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a = Args::parse();
    let mut cfg = EngineConfig::default();
    if let Some(v) = a.p_offset {
        cfg.model.p_offset = v;
    }
    if let Some(v) = a.hardness {
        cfg.model.hardness = v;
    }
    if let Some(v) = a.vmin {
        cfg.hammer_vmin = v;
    }
    if let Some(v) = a.vmax {
        cfg.hammer_vmax = v;
    }
    let mut jobs = Vec::new();
    for e in std::fs::read_dir(&a.reference)? {
        let path = e?.path();
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
        let Some((n, v)) = stem.rsplit_once('v') else { continue };
        let (Some(note), Ok(layer)) = (io::parse_note_name(n), v.parse::<usize>()) else { continue };
        if (1..=a.velocities.len()).contains(&layer) {
            jobs.push((path, note, layer));
        }
    }
    jobs.sort_by_key(|j| (j.1, j.2));
    let next = std::sync::atomic::AtomicUsize::new(0);
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let mut lines: Vec<(u8, usize, Vec<String>)> = std::thread::scope(|s| {
        let hs: Vec<_> = (0..threads)
            .map(|_| {
                s.spawn(|| {
                    let mut res = Vec::new();
                    loop {
                        let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        let Some((path, note, layer)) = jobs.get(i) else { break };
                        let vel = a.velocities[layer - 1];
                        let mut rows = Vec::new();
                        let (ct, fmax) = piano_core::engine::strike_stats(&cfg, *note, vel, None);
                        let model = render(cfg, *note, vel, a.seconds);
                        let reference = if a.model_only { None } else { io::read_mono(path, None).ok() };
                        let srcs = [("model", Some((model, cfg.sample_rate))), ("reference", reference)];
                        for (src, sig) in srcs {
                            let Some((x, sr)) = sig else { continue };
                            let Some(m) = measure(&x, sr, *note, a.partials) else { continue };
                            for (k, lvl) in &m.levels {
                                rows.push(format!(
                                    "{src},{note},{layer},{vel},{:.2},{:.1},{:.4},{:.4e},{:.3e},{:.1},{k},{lvl:.2}",
                                    m.peak_db, m.centroid, m.f0, m.b, ct, fmax
                                ));
                            }
                        }
                        res.push((*note, *layer, rows));
                    }
                    res
                })
            })
            .collect();
        hs.into_iter().flat_map(|h| h.join().unwrap()).collect()
    });
    lines.sort_by_key(|l| (l.0, l.1));
    let mut f = std::io::BufWriter::new(std::fs::File::create(&a.out)?);
    writeln!(f, "source,midi,layer,velocity,peak_dbfs,centroid_hz,f0,B,contact_s,max_force_n,k,level_db")?;
    for (_, _, rows) in lines {
        for r in rows {
            writeln!(f, "{r}")?;
        }
    }
    eprintln!("wrote {}", a.out.display());
    Ok(())
}
