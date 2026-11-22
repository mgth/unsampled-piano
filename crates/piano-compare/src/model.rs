//! The model's notes, rendered as the bench renders them: close microphones
//! in a studio, played on the Disklavier's velocity scale, each note alone
//! in a fresh engine. Renders are cached on disk by a hash of the full
//! preset, of the engine's sources (build.rs) and of the rendering below,
//! so a changed preset, engine or protocol renders again and an unchanged
//! one never.

use piano_core::{instrument::Instrument, params::PianoModel, Engine, EngineConfig, Event, Quality, TimedEvent};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub const SR: u32 = 48_000;
/// Key held, then released: the measures stop at 3.3 s.
pub const HOLD: f64 = 3.5;
pub const TAIL: f64 = 0.5;
/// The second protocol, for the release measures only: the key released
/// 2 s after the note-on, as Liang's and MAPS's Disklavier notes are, and
/// the render stopped once the 250 ms after it are in (with a margin to
/// find the release).
pub const RELEASE_HOLD: f64 = 2.0;
pub const RELEASE_TAIL: f64 = 0.5;
const BLOCK: usize = 128;
/// Change it when `Renderer::note` renders differently.
const PROTOCOL: u32 = 1;
/// Change it when `Renderer::note_released` renders differently.
const RELEASE_PROTOCOL: u32 = 1;

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
    /// Folder of its renders released at [`RELEASE_HOLD`], under their own
    /// key: the renders held 3.5 s keep theirs.
    pub release_dir: PathBuf,
}

pub fn load(root: &Path, preset: Option<&Path>, quality: Quality, cache: &Path) -> Result<ModelSpec, String> {
    let mut model = match preset {
        Some(p) => piano_host::load_preset(p)?,
        None => PianoModel::default(),
    };
    for l in LISTENING {
        model = piano_host::load_listening(&root.join(l), &model)?;
    }
    let velocities: Vec<u8> = crate::sources::LAYERS.iter().map(|l| l.2).collect();
    let key = format!(
        "{}\nengine {}\nprotocol {PROTOCOL} sr {SR} hold {HOLD} tail {TAIL} block {BLOCK} velocities {velocities:?}",
        piano_host::preset_toml(&model),
        env!("ENGINE_HASH"),
    );
    let hash = |key: &str| format!("{}-{:012x}", quality.name(), key.bytes().fold(0xcbf29ce484222325u64, |h, b| (h ^ b as u64).wrapping_mul(0x100000001b3)) & 0xffff_ffff_ffff);
    let release_key = format!("{key}\nrelease protocol {RELEASE_PROTOCOL} hold {RELEASE_HOLD} tail {RELEASE_TAIL}");
    let name = preset.and_then(|p| p.file_stem()).and_then(|s| s.to_str()).unwrap_or("default").to_string();
    let folder = |h: String| format!("{name}-{h}").replace(' ', "_");
    Ok(ModelSpec {
        label: format!("Model · {name}"),
        model,
        quality,
        dir: cache.join("model").join(folder(hash(&key))),
        release_dir: cache.join("model-release").join(folder(hash(&release_key))),
    })
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

    /// Stereo channels of one note, held [`HOLD`] s.
    pub fn note(&self, midi: u8, velocity: u8) -> [Vec<f32>; 2] {
        self.render(midi, velocity, HOLD, TAIL)
    }

    /// The same note released [`RELEASE_HOLD`] s after the note-on.
    pub fn note_released(&self, midi: u8, velocity: u8) -> [Vec<f32>; 2] {
        self.render(midi, velocity, RELEASE_HOLD, RELEASE_TAIL)
    }

    fn render(&self, midi: u8, velocity: u8, hold: f64, tail: f64) -> [Vec<f32>; 2] {
        let cfg = EngineConfig { sample_rate: SR as f64, max_block: BLOCK, quality: self.quality, ..Default::default() };
        let mut engine = Engine::with_instrument(cfg, self.inst.clone());
        let total = ((hold + tail) * SR as f64).ceil() as usize;
        let frames = total.next_multiple_of(BLOCK);
        let (mut l, mut r) = (vec![0.0f32; frames], vec![0.0f32; frames]);
        let off = (hold * SR as f64).round() as usize;
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
