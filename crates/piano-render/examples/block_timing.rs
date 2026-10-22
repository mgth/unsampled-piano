//! Per-second load of the engine on a MIDI file, as the real-time player
//! would see it (128-frame buffers, sample-accurate events): average and
//! worst buffer time relative to the buffer duration.
//!   cargo run --release -p piano-render --example block_timing -- <file.mid> [preset.toml]
use piano_core::{Engine, EngineConfig, TimedEvent};
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let tl = piano_host::read_midi_file(std::path::Path::new(&args[1])).unwrap();
    let model = args.get(2).map(|p| piano_host::load_preset(std::path::Path::new(p)).unwrap()).unwrap_or_default();
    let sr = 48_000.0;
    let bs = 128usize;
    let cfg = EngineConfig { sample_rate: sr, max_block: bs, model, ..Default::default() };
    let mut e = Engine::new(cfg);
    let (mut l, mut r) = (vec![0.0f32; bs], vec![0.0f32; bs]);
    let end = tl.last().unwrap().0 + 3.0;
    let (mut clock, mut cur) = (0u64, 0usize);
    let mut evs = Vec::with_capacity(512);
    let (mut sum, mut max, mut n) = (0.0f64, 0.0f64, 0);
    let per_s = (sr as usize) / bs;
    while (clock as f64) < end * sr {
        evs.clear();
        while cur < tl.len() && ((tl[cur].0 * sr) as u64) < clock + bs as u64 {
            evs.push(TimedEvent { frame: ((tl[cur].0 * sr) as u64).saturating_sub(clock) as u32, event: tl[cur].1 });
            cur += 1;
        }
        let t = Instant::now();
        e.process_stereo(&evs, &mut l, &mut r);
        let load = t.elapsed().as_secs_f64() * sr / bs as f64;
        sum += load;
        max = max.max(load);
        n += 1;
        clock += bs as u64;
        if n == per_s {
            println!("{:6.1} s  avg {:6.1} %  max {:6.1} %  voices {}", clock as f64 / sr, 100.0 * sum / n as f64, 100.0 * max, e.active_voices());
            (sum, max, n) = (0.0, 0.0, 0);
        }
    }
}
