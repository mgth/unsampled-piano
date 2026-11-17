//! The model's notes, rendered as the bench renders them: close microphones
//! in a studio, played on the Disklavier's velocity scale, each note alone
//! in a fresh engine. Renders are cached on disk by a hash of the full
//! preset, so a changed preset renders again and an unchanged one never.

use piano_core::{instrument::Instrument, params::PianoModel, Engine, EngineConfig, Event, Quality, TimedEvent};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub const SR: u32 = 48_000;
/// Key held, then released: the measures stop at 3.3 s.
pub const HOLD: f64 = 3.5;
pub const TAIL: f64 = 0.5;
const BLOCK: usize = 128;

/// Listening presets of the bench (tools/bench.py LISTENING, PLAYBACK).
pub const LISTENING: [&str; 3] = [
    "archive/presets/listening/Microphones/close.toml",
    "archive/presets/listening/Room/studio.toml",
    "archive/presets/touch/disklavier_playback.toml",
];

pub struct ModelSpec {
    pub label: String,
    pub model: PianoModel,
    pub quality: Quality,
    /// Folder of this preset's renders.
    pub dir: PathBuf,
}

pub fn load(root: &Path, preset: Option<&Path>, quality: Quality, cache: &Path) -> Result<ModelSpec, String> {
    let mut model = match preset {
        Some(p) => piano_host::load_preset(p)?,
        None => PianoModel::default(),
    };
    for l in LISTENING {
        model = piano_host::load_listening(&root.join(l), &model)?;
    }
    let text = piano_host::preset_toml(&model);
    let hash = format!("{}-{:012x}", quality.name(), text.bytes().fold(0xcbf29ce484222325u64, |h, b| (h ^ b as u64).wrapping_mul(0x100000001b3)) & 0xffff_ffff_ffff);
    let name = preset.and_then(|p| p.file_stem()).and_then(|s| s.to_str()).unwrap_or("default").to_string();
    Ok(ModelSpec { label: format!("Model · {name}"), model, quality, dir: cache.join("model").join(format!("{name}-{hash}").replace(' ', "_")) })
}

/// Builds the instrument once; each note gets a fresh engine on it.
pub struct Renderer {
    inst: Arc<Instrument>,
    quality: Quality,
}

impl Renderer {
    pub fn new(spec: &ModelSpec) -> Self {
        Self { inst: Arc::new(spec.quality.instrument(&spec.model, SR as f64)), quality: spec.quality }
    }

    /// Stereo channels of one note.
    pub fn note(&self, midi: u8, velocity: u8) -> [Vec<f32>; 2] {
        let cfg = EngineConfig { sample_rate: SR as f64, max_block: BLOCK, quality: self.quality, ..Default::default() };
        let mut engine = Engine::with_instrument(cfg, self.inst.clone());
        let total = ((HOLD + TAIL) * SR as f64).ceil() as usize;
        let frames = total.next_multiple_of(BLOCK);
        let (mut l, mut r) = (vec![0.0f32; frames], vec![0.0f32; frames]);
        let off = (HOLD * SR as f64).round() as usize;
        for (bi, (bl, br)) in l.chunks_mut(BLOCK).zip(r.chunks_mut(BLOCK)).enumerate() {
            let start = bi * BLOCK;
            let mut ev = Vec::new();
            if start == 0 {
                ev.push(TimedEvent { frame: 0, event: Event::NoteOn { note: midi, velocity } });
            }
            if (start..start + BLOCK).contains(&off) {
                ev.push(TimedEvent { frame: (off - start) as u32, event: Event::NoteOff { note: midi, velocity: 64 } });
            }
            engine.process_stereo(&ev, bl, br);
        }
        l.truncate(total);
        r.truncate(total);
        [l, r]
    }
}

pub fn write_wav(path: &Path, ch: &[Vec<f32>; 2]) -> Result<(), String> {
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("part");
    let spec = hound::WavSpec { channels: 2, sample_rate: SR, bits_per_sample: 32, sample_format: hound::SampleFormat::Float };
    let mut w = hound::WavWriter::create(&tmp, spec).map_err(|e| e.to_string())?;
    for (a, b) in ch[0].iter().zip(&ch[1]) {
        w.write_sample(*a).map_err(|e| e.to_string())?;
        w.write_sample(*b).map_err(|e| e.to_string())?;
    }
    w.finalize().map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())
}
