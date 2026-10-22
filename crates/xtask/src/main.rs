//! Plugin bundles with nice-plug's bundler:
//!   cargo xtask bundle piano-plugin --release
//!   cargo xtask bundle-universal piano-plugin --release   (macOS: Intel + Apple silicon)
//! write target/bundled/Unsampled Piano.{clap,vst3} in the platform's layout
//! (names from bundler.toml).
//!
//! nice-plug-xtask's own entry point looks for the outermost directory with
//! a Cargo.toml, which is the main checkout when this one is a worktree
//! nested in it: this one starts from its own workspace instead.

use std::path::{Path, PathBuf};

use nice_plug_xtask::Result;

const USAGE: &str = "usage: cargo xtask bundle|bundle-universal <package> [cargo build options]";

fn main() -> Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize()?;
    std::env::set_current_dir(&root)?;
    let target = std::env::var_os("CARGO_TARGET_DIR").map_or_else(|| root.join("target"), PathBuf::from);
    let mut args = std::env::args().skip(1);
    let (command, package) = match (args.next(), args.next()) {
        (Some(c), Some(p)) => (c, p),
        _ => anyhow_bail(USAGE)?,
    };
    let rest: Vec<String> = args.collect();
    let packages = [package.clone()];
    match command.as_str() {
        "bundle" => {
            nice_plug_xtask::build(&packages, &rest)?;
            nice_plug_xtask::bundle(&target, &package, &rest, false)
        }
        "bundle-universal" => {
            for triple in ["x86_64-apple-darwin", "aarch64-apple-darwin"] {
                let mut a = rest.clone();
                a.push(format!("--target={triple}"));
                nice_plug_xtask::build(&packages, &a)?;
            }
            nice_plug_xtask::bundle(&target, &package, &rest, true)
        }
        _ => anyhow_bail(USAGE),
    }
}

fn anyhow_bail<T>(msg: &str) -> Result<T> {
    Err(std::io::Error::other(msg.to_string()).into())
}
