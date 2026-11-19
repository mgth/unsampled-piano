//! The tool without its window: every note of the model rendered and measured
//! as in the window, then its z over all the measures, written as JSON. It
//! lets a preset be judged from a script (docs/compare_tool.md).

use crate::app::{mean_sd, tf};
use crate::jobs::{Cache, Stamp};
use crate::measure::{self, Analysis, Def, Id, DEFS};
use crate::model::{self, ModelSpec, Renderer};
use crate::sources::{Role, Source, HIGH, LAYERS, LOW};
use serde_json::json;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

/// A z counts as off beyond this, as the red of the window.
const OFF: f64 = 2.0;
/// In the mean gap, no measure weighs more than this: a reference that reads
/// its noise floor must not decide alone.
const CLIP: f64 = 3.0;
const REGISTERS: [(&str, u8, u8); 4] = [("bass", 21, 47), ("middle", 48, 71), ("treble", 72, 95), ("top", 96, 108)];

#[derive(Default, Clone, Copy, Debug, PartialEq)]
struct Tally {
    count: usize,
    off: usize,
    sum: f64,
}

impl Tally {
    fn add(&mut self, z: f64) {
        self.count += 1;
        self.off += usize::from(z.abs() > OFF);
        self.sum += z.abs().min(CLIP);
    }

    fn mean(&self) -> f64 {
        if self.count == 0 { 0.0 } else { self.sum / self.count as f64 }
    }

    fn json(&self) -> serde_json::Value {
        json!({ "count": self.count, "off": self.off, "mean": self.mean() })
    }
}

/// Runs `work` over the items on `threads` threads.
fn parallel<T: Sync, R: Send>(items: &[T], threads: usize, work: impl Fn(&T) -> R + Sync) -> Vec<R> {
    let (next, out) = (AtomicUsize::new(0), Mutex::new(Vec::with_capacity(items.len())));
    std::thread::scope(|s| {
        for _ in 0..threads.max(1) {
            s.spawn(|| {
                while let Some(item) = items.get(next.fetch_add(1, Ordering::Relaxed)) {
                    let r = work(item);
                    out.lock().unwrap().push(r);
                }
            });
        }
    });
    out.into_inner().unwrap()
}

type Results = HashMap<(u8, u8), Analysis>;

fn value(results: &Results, dynamics: bool, midi: u8, layer: u8, d: &Def) -> Option<f64> {
    if d.id == Id::Dynamics {
        if !dynamics {
            return None;
        }
        let level = |l| results.get(&(midi, l)).and_then(|a| a.level_db);
        return Some(level(13)? - level(4)?);
    }
    results.get(&(midi, layer))?.get(d.id)
}

/// The recordings' measures, from the cache or measured now.
fn recordings(source: &Source, cache: &Mutex<Cache>, threads: usize) -> Result<Results, String> {
    let mut out = Results::new();
    let mut todo: Vec<((u8, u8), PathBuf)> = Vec::new();
    for (&key, path) in &source.files {
        match cache.lock().unwrap().get(path) {
            Some(a) => {
                out.insert(key, a);
            }
            None => todo.push((key, path.clone())),
        }
    }
    let done = parallel(&todo, threads, |(key, path): &((u8, u8), PathBuf)| {
        let (channels, sr) = piano_analysis::io::read_channels(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok::<_, String>((*key, path.clone(), measure::analyse(&channels, sr, key.0, source.held)))
    });
    for r in done {
        let (key, path, analysis) = r?;
        if let Some(stamp) = Stamp::of(&path) {
            cache.lock().unwrap().put(&path, stamp, analysis.clone());
        }
        out.insert(key, analysis);
    }
    Ok(out)
}

/// Every note of the model, rendered and measured in memory.
fn model_notes(spec: &ModelSpec, threads: usize) -> Results {
    let renderer = Renderer::new(spec);
    let notes: Vec<(u8, u8, u8)> =
        (LOW..=HIGH).flat_map(|m| LAYERS.iter().map(move |&(l, _, v)| (m, l, v))).collect();
    parallel(&notes, threads, |&(midi, layer, velocity)| {
        let channels = renderer.note(midi, velocity);
        ((midi, layer), measure::analyse(&channels, f64::from(model::SR), midi, Some(model::HOLD)))
    })
    .into_iter()
    .collect()
}

pub fn run(
    mut sources: Vec<Source>,
    spec: &ModelSpec,
    cache_file: PathBuf,
    without: &[String],
    threads: usize,
    out: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    for key in without {
        if !sources.iter().any(|s| s.key == *key && s.role == Role::Target) {
            return Err(format!("--band-without: {key} is not a real piano of the bench").into());
        }
    }
    for s in &mut sources {
        s.scan();
    }
    let band: Vec<&Source> = sources.iter().filter(|s| s.role == Role::Target && !without.contains(&s.key)).collect();
    let cache = Mutex::new(Cache::open(cache_file));
    let mut measured = Vec::with_capacity(band.len());
    for s in &band {
        measured.push((recordings(s, &cache, threads)?, s.dynamics));
    }
    cache.lock().unwrap().save();
    let model = model_notes(spec, threads);

    let (mut all, mut by_measure, mut by_register) =
        (Tally::default(), vec![Tally::default(); DEFS.len()], [Tally::default(); REGISTERS.len()]);
    for midi in LOW..=HIGH {
        let register = REGISTERS.iter().position(|r| (r.1..=r.2).contains(&midi)).unwrap_or(0);
        for &(layer, ..) in &LAYERS {
            for (i, d) in DEFS.iter().enumerate() {
                let Some(v) = value(&model, true, midi, layer, d).map(|v| tf(v, d.log)).filter(|v| v.is_finite()) else { continue };
                let r: Vec<f64> = measured
                    .iter()
                    .filter_map(|(res, dynamics)| value(res, *dynamics, midi, layer, d))
                    .map(|x| tf(x, d.log))
                    .filter(|x| x.is_finite())
                    .collect();
                if r.len() < 2 {
                    continue;
                }
                let (mean, sd) = mean_sd(&r);
                let z = (v - mean) / sd.max(d.floor);
                all.add(z);
                by_measure[i].add(z);
                by_register[register].add(z);
            }
        }
    }
    let measures: serde_json::Map<String, serde_json::Value> =
        DEFS.iter().zip(&by_measure).map(|(d, t)| (d.name.to_string(), t.json())).collect();
    let registers: serde_json::Map<String, serde_json::Value> =
        REGISTERS.iter().zip(&by_register).map(|(r, t)| (r.0.to_string(), t.json())).collect();
    let report = json!({
        "off": all.off, "mean": all.mean(), "count": all.count,
        "band": band.iter().map(|s| s.key.as_str()).collect::<Vec<_>>(),
        "measures": measures, "registers": registers,
    });
    std::fs::write(out, serde_json::to_string_pretty(&report)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_far_measure_counts_once_and_weighs_no_more_than_the_clip() {
        let mut t = Tally::default();
        for z in [0.5, -2.5, 30.0] {
            t.add(z);
        }
        assert_eq!((t.count, t.off), (3, 2));
        assert!((t.mean() - (0.5 + 2.5 + 3.0) / 3.0).abs() < 1e-12);
    }
}
