//! Soundboard stage capture and room processing for the offline prototype.
//! rayleigh_capture capture PRESET OUT NOTE VELOCITIES HOLD TAIL [audit10]
//! audit10 retains the Full instrument, doubles contact steps and disables
//! engine silence shortcuts. This is a research sensitivity check, not a preset.
//! rayleigh_capture room PRESET IN.wav OUT.wav
use piano_core::{
    board::BoardBank, engine::SUB_BLOCK, instrument::Instrument, params::PianoModel, Engine,
    EngineConfig, Event, TimedEvent,
};
use std::{io::Write, path::Path, sync::Arc};
fn wav(
    path: &Path,
    channels: u16,
) -> Result<hound::WavWriter<std::io::BufWriter<std::fs::File>>, hound::Error> {
    hound::WavWriter::create(
        path,
        hound::WavSpec {
            channels,
            sample_rate: 48000,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        },
    )
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<String> = std::env::args().collect();
    if a.len() < 5 {
        return Err("capture PRESET OUT NOTE VELOCITIES HOLD TAIL | room PRESET IN OUT".into());
    }
    let original = piano_host::load_preset(Path::new(&a[2]))?;
    if a[1] == "room" {
        let model = original.with_macros();
        let model = PianoModel {
            soundboard: model.soundboard.map(|p| p.resolved()),
            ..model
        };
        let mut input = hound::WavReader::open(&a[3])?;
        assert_eq!(input.spec().channels, 2);
        assert_eq!(input.spec().sample_rate, 48000);
        let x = input.samples::<f32>().collect::<Result<Vec<_>, _>>()?;
        let mut l: Vec<f32> = x.iter().step_by(2).copied().collect();
        let mut r: Vec<f32> = x.iter().skip(1).step_by(2).copied().collect();
        if model.room.enabled {
            let mut room = piano_core::room::Room::new(
                &model.room,
                &model.soundboard.ok_or("no board")?,
                48000.,
            );
            room.process(&mut l, &mut r);
        }
        let mut out = wav(Path::new(&a[4]), 2)?;
        for (l, r) in l.into_iter().zip(r) {
            out.write_sample(l)?;
            out.write_sample(r)?;
        }
        out.finalize()?;
        return Ok(());
    }
    assert_eq!(a[1], "capture");
    assert!(a.len() == 8 || a.len() == 9);
    let quality = match a.get(8).map(String::as_str) {
        None => piano_core::engine::Quality::Full,
        Some("audit10") => piano_core::engine::Quality::Overkill,
        Some(_) => return Err("unknown capture audit mode".into()),
    };
    let model = PianoModel {
        room: piano_core::room::RoomParams {
            enabled: false,
            ..original.room
        },
        ..original
    };
    let cfg = EngineConfig {
        quality,
        model,
        max_block: SUB_BLOCK,
        threads: 1,
        ..Default::default()
    };
    let inst = Arc::new(Instrument::new(model, 48000.));
    assert!(inst.low.is_none());
    assert_eq!(inst.model.soundboard.unwrap().string_load_free, 1.);
    let n: u8 = a[4].parse()?;
    let velocities = a[5]
        .split(',')
        .map(str::parse::<u8>)
        .collect::<Result<Vec<_>, _>>()?;
    let hold: f64 = a[6].parse()?;
    let tail: f64 = a[7].parse()?;
    let total = ((hold + tail) * 48000.).ceil() as usize;
    let off = (hold * 48000.).round() as usize;
    let out = Path::new(&a[3]);
    std::fs::create_dir_all(out)?;
    for v in velocities {
        let mut engine = Engine::with_instrument(cfg, inst.clone());
        let mut bank = BoardBank::new(48000., inst.board.as_ref().unwrap(), SUB_BLOCK);
        let mut sym = BoardBank::new(48000., inst.board.as_ref().unwrap(), SUB_BLOCK);
        let mut last = [0f32; 2];
        let mut writer = wav(&out.join(format!("n{n}v{v}.wav")), 4)?;
        let mut force = std::io::BufWriter::new(std::fs::File::create(
            out.join(format!("n{n}v{v}.force.f32")),
        )?);
        for start in (0..total).step_by(SUB_BLOCK) {
            // The capture accessors are defined only for full sub-blocks. Extra
            // samples are computed then cropped at output, preserving all events.
            let mut ev = Vec::new();
            if start == 0 {
                ev.push(TimedEvent {
                    frame: 0,
                    event: Event::NoteOn {
                        note: n,
                        velocity: v,
                    },
                });
            }
            if off >= start && off < start + SUB_BLOCK {
                ev.push(TimedEvent {
                    frame: (off - start) as u32,
                    event: Event::NoteOff {
                        note: n,
                        velocity: 64,
                    },
                });
            }
            let (mut l, mut r) = ([0f32; SUB_BLOCK], [0f32; SUB_BLOCK]);
            engine.process_stereo(&ev, &mut l, &mut r);
            let inputs: Vec<Vec<f32>> = (0..16).map(|r| engine.bridge_force(r).to_vec()).collect();
            let si: Vec<Vec<f32>> = (0..16)
                .map(|r| engine.sympathetic_bridge_force(r).to_vec())
                .collect();
            let (mut pl, mut pr) = ([0f32; SUB_BLOCK], [0f32; SUB_BLOCK]);
            bank.process(&inputs, SUB_BLOCK, [&mut pl, &mut pr], None, None);
            sym.process(&si, SUB_BLOCK, [&mut pl, &mut pr], None, None);
            let scale = engine.pressure_scale();
            for i in 0..SUB_BLOCK {
                let p = [pl[i], pr[i]];
                let old = [
                    (p[0] - last[0]) * 48000. * scale,
                    (p[1] - last[1]) * 48000. * scale,
                ];
                last = p;
                if start + i < total {
                    for x in [l[i], r[i], old[0], old[1]] {
                        writer.write_sample(x)?;
                    }
                    for r in 0..16 {
                        force.write_all(&(inputs[r][i] + si[r][i]).to_le_bytes())?;
                    }
                }
            }
        }
        writer.finalize()?;
        force.flush()?;
        std::fs::write(
            out.join("scale.txt"),
            format!("{}\n", engine.pressure_scale()),
        )?;
    }
    Ok(())
}
