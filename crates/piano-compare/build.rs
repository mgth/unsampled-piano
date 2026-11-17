//! ENGINE_HASH: a hash of the engine's sources (crates/piano-core), by
//! content, local changes included. The model's renders are cached under
//! it, so a changed engine renders again and the same sources, in any
//! worktree, share their renders.

use std::path::{Path, PathBuf};

fn files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            files(&p, out);
        } else {
            out.push(p);
        }
    }
}

fn main() {
    let core = Path::new(env!("CARGO_MANIFEST_DIR")).join("../piano-core");
    let mut list = vec![core.join("Cargo.toml")];
    files(&core.join("src"), &mut list);
    list.sort();
    let mut h = 0xcbf29ce484222325u64;
    let mut eat = |bytes: &[u8]| {
        for &b in bytes {
            h = (h ^ b as u64).wrapping_mul(0x100000001b3);
        }
    };
    for p in &list {
        eat(p.strip_prefix(&core).unwrap_or(p).to_string_lossy().as_bytes());
        eat(&[0]);
        eat(&std::fs::read(p).unwrap_or_default());
        eat(&[0]);
    }
    println!("cargo:rustc-env=ENGINE_HASH={h:016x}");
    println!("cargo:rerun-if-changed={}", core.join("src").display());
    println!("cargo:rerun-if-changed={}", core.join("Cargo.toml").display());
}
