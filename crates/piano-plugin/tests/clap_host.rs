//! The built plugin loaded as a DAW would (CLAP, through clack-host): it
//! instantiates, activates, sounds a note, and follows host automation of
//! a live parameter. Debug builds also abort if `process` allocates
//! (nice-plug's assert_process_allocs).
//!
//! `cargo test` links the plugin's rlib, not its cdylib: build it first,
//!   cargo build -p piano-plugin && cargo test -p piano-plugin
//! The test is skipped (with a message) when the cdylib is missing or older
//! than the workspace's sources.

use clack_host::events::event_types::{NoteOffEvent, NoteOnEvent, ParamValueEvent};
use clack_host::prelude::*;
use clack_host::utils::ClapId;

const SR: f64 = 48_000.0;
const BLOCK: usize = 512;

struct Shared;

impl SharedHandler<'_> for Shared {
    fn request_restart(&self) {}
    fn request_process(&self) {}
    fn request_callback(&self) {}
}

struct Host;

impl HostHandlers for Host {
    type Shared<'a> = Shared;
    type MainThread<'a> = ();
    type AudioProcessor<'a> = ();
}

/// The cdylib next to this test's binary (target/<profile>/): .so, .dll
/// or .dylib.
fn plugin_path() -> std::path::PathBuf {
    let exe = std::env::current_exe().unwrap();
    let name = format!("{}piano_plugin{}", std::env::consts::DLL_PREFIX, std::env::consts::DLL_SUFFIX);
    exe.parent().and_then(|d| d.parent()).unwrap().join(name)
}

/// Why the built plugin cannot be tested, if so: missing, or older than a
/// source file of the workspace's crates (tests and examples aside).
fn unusable_plugin() -> Option<String> {
    let path = plugin_path();
    let built = match std::fs::metadata(&path).and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(_) => return Some(format!("{} not built", path.display())),
    };
    let crates = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let mut stack = vec![crates.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for e in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                // Only what goes into the plugin.
                if !p.file_name().is_some_and(|n| ["tests", "examples", "benches"].contains(&n.to_str().unwrap_or(""))) {
                    stack.push(p);
                }
            } else if p.extension().is_some_and(|x| x == "rs" || x == "toml")
                && e.metadata().and_then(|m| m.modified()).is_ok_and(|t| t > built)
            {
                return Some(format!("{} is older than {}", path.display(), p.display()));
            }
        }
    }
    None
}

/// nice-plug's CLAP id of a parameter.
fn param_id(id: &str) -> ClapId {
    let h = id.bytes().fold(0u32, |h, b| h.wrapping_mul(31).wrapping_add(b as u32));
    ClapId::new(h & !(1 << 31))
}

/// Block where the automation is sent (≈ 0.5 s).
const AUTOMATION_BLOCK: usize = (0.5 * SR) as usize / BLOCK;

/// Renders `seconds`: middle C held from the start, and `gain_db` (if any)
/// automated at [`AUTOMATION_BLOCK`]. Returns the left channel.
fn render(seconds: f64, gain_db: Option<f64>) -> Vec<f32> {
    let host_info = HostInfo::new("piano tests", "piano", "https://example.com", "0.1").unwrap();
    // SAFETY: our own plugin, built by this workspace.
    let entry = unsafe { PluginEntry::load(plugin_path()) }.expect("plugin loads");
    let factory = entry.get_plugin_factory().expect("plugin factory");
    let descriptor = factory.plugin_descriptors().next().expect("one plugin");
    assert_eq!(descriptor.id().unwrap().to_bytes(), b"fr.mgth.unsampled-piano");
    let id = descriptor.id().unwrap().to_owned();
    let mut instance = PluginInstance::<Host>::new(|_| Shared, |_| (), &entry, &id, &host_info).expect("instance");
    let config = PluginAudioConfiguration { sample_rate: SR, min_frames_count: 1, max_frames_count: BLOCK as u32 };
    let processor = instance.activate(|_, _| (), config).expect("activates");
    let mut processor = processor.start_processing().expect("starts");

    let blocks = (seconds * SR) as usize / BLOCK;
    let mut out = Vec::with_capacity(blocks * BLOCK);
    let mut events = EventBuffer::with_capacity(8);
    let mut out_events = EventBuffer::with_capacity(8);
    let (mut l, mut r) = (vec![0.0f32; BLOCK], vec![0.0f32; BLOCK]);
    let mut out_ports = AudioPorts::with_capacity(2, 1);
    let note = Pckn::new(0u16, 0u16, 60u16, 0u32);
    for b in 0..blocks {
        events.clear();
        if b == 0 {
            events.push(&NoteOnEvent::new(0, note, 0.7));
        }
        if let Some(db) = gain_db.filter(|_| b == AUTOMATION_BLOCK) {
            // nice-plug's CLAP parameters are normalised (0–1); the gain
            // range is −40…+20 dB, linear.
            let normalized = (db + 40.0) / 60.0;
            events.push(&ParamValueEvent::new(0, param_id("output_gain_db"), Pckn::match_all(), normalized));
        }
        if b == blocks - 1 {
            events.push(&NoteOffEvent::new(0, note, 0.5));
        }
        let input = InputAudioBuffers::empty();
        let mut output = out_ports.with_output_buffers([AudioPortBuffer {
            latency: 0,
            channels: AudioPortBufferType::f32_output_only([l.as_mut_slice(), r.as_mut_slice()].into_iter()),
        }]);
        processor
            .process(&input, &mut output, &InputEvents::from_buffer(&events), &mut OutputEvents::from_buffer(&mut out_events), None, None)
            .expect("processes");
        out.extend_from_slice(&l);
    }
    let processor = processor.stop_processing();
    instance.deactivate(processor);
    out
}

fn rms(x: &[f32]) -> f64 {
    (x.iter().map(|&v| v as f64 * v as f64).sum::<f64>() / x.len() as f64).sqrt()
}

#[test]
fn plays_and_follows_automation() {
    if let Some(why) = unusable_plugin() {
        eprintln!("SKIPPED: {why}; run `cargo build -p piano-plugin` before the tests");
        return;
    }
    let dry = render(1.2, None);
    assert!(dry.iter().all(|x| x.is_finite()));
    let level = rms(&dry[..(0.5 * SR) as usize]);
    assert!(level > 1e-3, "middle C too quiet: rms {level}");

    let wet = render(1.2, Some(-20.0));
    let n = AUTOMATION_BLOCK * BLOCK;
    // Same instrument, same note: identical until the automation.
    assert!(dry[..n].iter().zip(&wet[..n]).all(|(a, b)| a == b));
    // −20 dB once the smoothing has settled (the room keeps a little of
    // the louder tail: tolerance).
    let tail = n + (0.2 * SR) as usize;
    let ratio = rms(&wet[tail..]) / rms(&dry[tail..]);
    assert!((0.08..0.13).contains(&ratio), "gain ratio {ratio}");
}
