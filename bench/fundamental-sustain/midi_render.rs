//! Offline MIDI playback through the experimental engine used by the H1 bench.
//! Arguments: bundle.toml input.mid output.wav [tail seconds]
use piano_core::{Engine, EngineConfig, TimedEvent, experimental::LATENCY};
use std::{path::Path, time::Instant};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<String> = std::env::args().collect();
    if a.get(1).is_some_and(|s| s == "--fingerprint") {
        println!("{}", piano_host::experimental::physics_fingerprint());
        return Ok(());
    }
    if a.len() < 4 {
        return Err("bundle input.mid output.wav [tail]".into());
    }
    let experiment = Box::leak(Box::new(piano_host::experimental::load(Path::new(&a[1]))?));
    let tl = piano_host::read_midi_file(Path::new(&a[2]))?;
    if tl.is_empty() {
        return Err("empty MIDI timeline".into());
    }
    let tail: f64 = a.get(4).map_or(Ok(5.), |v| v.parse())?;
    let block = 128;
    let mut engine = Engine::new(EngineConfig {
        model: experiment.model,
        experiment: Some(experiment),
        max_block: block,
        threads: 1,
        ..Default::default()
    });
    let total = ((tl.last().unwrap().0 + tail) * 48000.).ceil() as usize;
    let mut writer = hound::WavWriter::create(
        &a[3],
        hound::WavSpec {
            channels: 2,
            sample_rate: 48000,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        },
    )?;
    let (mut l, mut r) = (vec![0.; block], vec![0.; block]);
    let mut events = Vec::with_capacity(256);
    let (mut next, mut peak, mut voices) = (0, 0f32, 0);
    let clock = Instant::now();
    for start in (0..total + LATENCY).step_by(block) {
        events.clear();
        while next < tl.len() {
            let frame = (tl[next].0 * 48000.).round() as usize;
            if frame >= start + block {
                break;
            }
            events.push(TimedEvent {
                frame: frame.saturating_sub(start) as u32,
                event: tl[next].1,
            });
            next += 1;
        }
        engine.process_stereo(&events, &mut l, &mut r);
        voices = voices.max(engine.active_voices());
        for i in 0..block {
            if start + i < LATENCY || start + i >= total + LATENCY {
                continue;
            }
            for x in [l[i], r[i]] {
                if !x.is_finite() {
                    return Err("nonfinite audio".into());
                }
                peak = peak.max(x.abs());
                writer.write_sample(x)?;
            }
        }
    }
    writer.finalize()?;
    assert_eq!(next, tl.len());
    println!(
        "{}: {:.2}s, peak {:.3} dBFS, {voices} voices, {} events, {:.2}s render",
        a[3],
        total as f64 / 48000.,
        20. * peak.log10(),
        tl.len(),
        clock.elapsed().as_secs_f64()
    );
    Ok(())
}
