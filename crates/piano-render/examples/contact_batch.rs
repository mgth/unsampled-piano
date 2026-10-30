//! Research renderer: one instrument, independent strikes for a note/velocity grid.
//! contact_batch PRESET OUTDIR NOTES_CSV VELOCITIES_CSV HOLD TAIL
//! No gain normalization, note-specific processing, or production defaults changed.
use piano_core::{Engine, EngineConfig, Event, TimedEvent, instrument::Instrument};
use std::{path::Path, sync::Arc};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<String> = std::env::args().collect();
    if a.len() != 7 {
        return Err("contact_batch PRESET OUTDIR NOTES_CSV VELOCITIES_CSV HOLD TAIL".into());
    }
    let model = piano_host::load_preset(Path::new(&a[1]))?;
    let parse =
        |s: &str| -> Result<Vec<u8>, Box<dyn std::error::Error>> { Ok(s.split(',').map(str::parse).collect::<Result<Vec<_>, _>>()?) };
    let notes = parse(&a[3])?;
    let velocities = parse(&a[4])?;
    let hold: f64 = a[5].parse()?;
    let tail: f64 = a[6].parse()?;
    if !hold.is_finite() || !tail.is_finite() || hold <= 0. || tail < 0. {
        return Err("invalid duration".into());
    }
    let cfg = EngineConfig { model, ..Default::default() };
    let inst = Arc::new(Instrument::new(model, cfg.sample_rate));
    std::fs::create_dir_all(&a[2])?;
    for &note in &notes {
        for &velocity in &velocities {
            let mut engine = Engine::with_instrument(cfg, inst.clone());
            let total = ((hold + tail) * cfg.sample_rate).ceil() as usize;
            let off = (hold * cfg.sample_rate).round() as usize;
            let file = Path::new(&a[2]).join(format!("n{note}v{velocity}.wav"));
            let spec = hound::WavSpec {
                channels: 2,
                sample_rate: cfg.sample_rate as u32,
                bits_per_sample: 32,
                sample_format: hound::SampleFormat::Float,
            };
            let mut wav = hound::WavWriter::create(file, spec)?;
            for start in (0..total).step_by(128) {
                let len = (total - start).min(128);
                let (mut l, mut r) = ([0f32; 128], [0f32; 128]);
                let mut events = Vec::new();
                if start == 0 {
                    events.push(TimedEvent { frame: 0, event: Event::NoteOn { note, velocity } });
                }
                if off >= start && off < start + len {
                    events.push(TimedEvent { frame: (off - start) as u32, event: Event::NoteOff { note, velocity: 64 } });
                }
                engine.process_stereo(&events, &mut l[..len], &mut r[..len]);
                for i in 0..len {
                    wav.write_sample(l[i])?;
                    wav.write_sample(r[i])?;
                }
            }
            wav.finalize()?;
        }
    }
    Ok(())
}
