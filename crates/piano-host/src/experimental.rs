//! Loading an explicitly selected, local research bundle. Never a preset default.
use piano_core::experimental::Experiment;
use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Bundle {
    label: String,
    physics_fingerprint: String,
    base: String,
    impulses: String,
    contacts: Vec<Contact>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Contact {
    note: u8,
    preset: String,
}

pub fn load(path: &Path) -> Result<Experiment, Box<dyn std::error::Error>> {
    let m: Bundle = toml::from_str(&std::fs::read_to_string(path)?)?;
    if m.physics_fingerprint != physics_fingerprint() {
        return Err("Rayleigh bundle belongs to another engine revision; regenerate its presets and filters".into());
    }
    let dir = path.parent().ok_or("bundle needs a parent directory")?;
    let model = crate::load_preset(&dir.join(m.base))?;
    let contacts = m
        .contacts
        .iter()
        .map(|c| Ok((c.note, crate::load_preset(&dir.join(&c.preset))?)))
        .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
    let file = dir.join(m.impulses);
    let size = 24 + 16 * 2 * 65536 * 8;
    if std::fs::metadata(&file)?.len() != size {
        return Err("unexpected Rayleigh impulse file size".into());
    }
    let bytes = std::fs::read(file)?;
    if &bytes[..8] != b"RAYL002\n" {
        return Err("unknown Rayleigh impulse format".into());
    }
    for (i, v) in [48000u32, 16, 2, 65536].into_iter().enumerate() {
        if u32::from_le_bytes(bytes[8 + 4 * i..12 + 4 * i].try_into()?) != v {
            return Err("incompatible Rayleigh impulse dimensions/rate".into());
        }
    }
    let ir = bytes[24..].chunks_exact(8).map(|x| f64::from_le_bytes(x.try_into().unwrap())).collect();
    Ok(Experiment::new(model, contacts, ir, m.label)?)
}

/// Hash the compiled physics sources, not the working directory. A stale bundle
/// must not quietly combine old transfer functions with a new instrument.
pub fn physics_fingerprint() -> String {
    let sources = [
        include_str!("../../piano-core/src/engine.rs"),
        include_str!("../../piano-core/src/experimental.rs"),
        include_str!("../../piano-core/src/instrument.rs"),
        include_str!("../../piano-core/src/note_model.rs"),
        include_str!("../../piano-core/src/params.rs"),
        include_str!("../../piano-core/src/soundboard.rs"),
        include_str!("../../piano-core/src/structure.rs"),
        include_str!("../../piano-core/src/board.rs"),
        include_str!("../../piano-core/src/eigen.rs"),
        include_str!("../../piano-core/src/hammer.rs"),
        include_str!("../../piano-core/src/unison.rs"),
        include_str!("../../piano-core/src/room.rs"),
        include_str!("../../piano-core/src/voice.rs"),
        include_str!("../../piano-core/src/modal.rs"),
        include_str!("../../piano-core/src/macros.rs"),
        include_str!("../../piano-core/src/live.rs"),
    ];
    let mut hash = 0xcbf29ce484222325u64;
    for source in sources { for byte in source.bytes().chain([0]) {
        hash = (hash ^ byte as u64).wrapping_mul(0x100000001b3);
    }}
    format!("{hash:016x}")
}
