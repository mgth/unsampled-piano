//! Loading an explicitly selected, local research bundle. Never a preset default.
use piano_core::experimental::Experiment;
use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Bundle {
    label: String,
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
