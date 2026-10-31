//! Research only: isolated strikes, primary bridge force, and modal loss budget.
//! fundamental_probe PRESET OUT NOTES_CSV VELOCITIES_CSV HOLD TAIL
//! WAVs are unnormalised. Force is in N, audio is the normal engine output.
use piano_core::{
    Engine, EngineConfig, Event, TimedEvent, engine::SUB_BLOCK, instrument::Instrument,
};
use std::{io::Write, path::Path, sync::Arc};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<String> = std::env::args().collect();
    if a.len() != 7 {
        return Err("fundamental_probe PRESET OUT NOTES VELOCITIES HOLD TAIL".into());
    }
    let model = piano_host::load_preset(Path::new(&a[1]))?;
    let notes: Vec<u8> = a[3].split(',').map(str::parse).collect::<Result<_, _>>()?;
    let velocities: Vec<u8> = a[4].split(',').map(str::parse).collect::<Result<_, _>>()?;
    let hold: f64 = a[5].parse()?;
    let tail: f64 = a[6].parse()?;
    if notes.iter().any(|n| !(21..=108).contains(n))
        || velocities.iter().any(|v| !(1..=127).contains(v))
        || !hold.is_finite()
        || hold <= 0.
        || !tail.is_finite()
        || tail < 0.
    {
        return Err("invalid notes, velocities or durations".into());
    }
    let cfg = EngineConfig {
        model,
        max_block: SUB_BLOCK,
        ..Default::default()
    };
    let inst = Arc::new(Instrument::new(model, cfg.sample_rate));
    let sb = inst.model.board().ok_or("soundboard required")?;
    let frame = piano_core::structure::FrameAdmittance::new(&inst.model.frame, &sb.params);
    let out = Path::new(&a[2]);
    std::fs::create_dir_all(out)?;
    let mut budget = std::fs::File::create(out.join("modes.csv"))?;
    writeln!(
        budget,
        "note,k,mode,freq,sigma,string_sigma,bridge_g,edge_g,frame_g,out_weight,t60"
    )?;
    for note in notes {
        let nm = inst.table.get(note);
        let shapes = sb.bridge_shapes(note);
        for (i, m) in nm.modes.iter().take(nm.n_osc * 8).enumerate() {
            let omega = std::f64::consts::TAU * m.freq_hz;
            let fg = frame
                .as_ref()
                .map_or(0., |f| f.at(sb.params.key_across(note), omega).re);
            writeln!(
                budget,
                "{note},{},{},{},{},{},{},{},{},{},{}",
                i / nm.n_osc + 1,
                i % nm.n_osc,
                m.freq_hz,
                m.sigma,
                inst.model.losses.sigma(note, m.freq_hz),
                sb.bridge_admittance(&shapes, omega).re,
                sb.edge_conductance(note, omega),
                fg,
                m.out_re.hypot(m.out_im),
                1000f64.ln() / m.sigma
            )?;
        }
        for &velocity in &velocities {
            let mut engine = Engine::with_instrument(cfg, inst.clone());
            let spec = hound::WavSpec {
                channels: 2,
                sample_rate: 48000,
                bits_per_sample: 32,
                sample_format: hound::SampleFormat::Float,
            };
            let mut audio =
                hound::WavWriter::create(out.join(format!("n{note}v{velocity}.wav")), spec)?;
            let mut force = hound::WavWriter::create(
                out.join(format!("n{note}v{velocity}-force.wav")),
                hound::WavSpec {
                    channels: 1,
                    ..spec
                },
            )?;
            let off = (hold * cfg.sample_rate).round() as usize;
            let total = ((hold + tail) * cfg.sample_rate).ceil() as usize;
            for start in (0..total).step_by(SUB_BLOCK) {
                let len = (total - start).min(SUB_BLOCK);
                let (mut l, mut r) = ([0.; SUB_BLOCK], [0.; SUB_BLOCK]);
                let mut events = Vec::new();
                if start == 0 {
                    events.push(TimedEvent {
                        frame: 0,
                        event: Event::NoteOn { note, velocity },
                    });
                }
                if off >= start && off < start + len {
                    events.push(TimedEvent {
                        frame: (off - start) as u32,
                        event: Event::NoteOff { note, velocity: 64 },
                    });
                }
                engine.process_stereo(&events, &mut l[..len], &mut r[..len]);
                for i in 0..len {
                    assert!(l[i].is_finite() && r[i].is_finite());
                    audio.write_sample(l[i])?;
                    audio.write_sample(r[i])?;
                    // Sum the primary force over all regions: with one struck key,
                    // this retains its region crossfade. Sympathetic force is separate.
                    let f: f32 = (0..piano_core::board::REGIONS)
                        .map(|j| engine.bridge_force(j)[i])
                        .sum();
                    force.write_sample(f)?;
                }
            }
            audio.finalize()?;
            force.finalize()?;
        }
    }
    Ok(())
}
