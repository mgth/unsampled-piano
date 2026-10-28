//! Research only: capture the contact and each stage of an isolated strike.
//! contact_stages PRESET OUT NOTE VELOCITIES [CONTACT_SUBSTEPS]
//! Four-channel WAV: left, right, bridge force (N), bridge velocity (m/s).
//! CONTACT_SUBSTEPS affects only the isolated force diagnostic, not engine audio.
use piano_core::{
    Engine, EngineConfig, Event, TimedEvent,
    engine::{SUB_BLOCK, contact_oversampling, hammer_velocity},
    instrument::Instrument,
    voice::Voice,
};
use std::{io::Write, path::Path, sync::Arc};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<String> = std::env::args().collect();
    if !(5..=6).contains(&a.len()) {
        return Err("contact_stages PRESET OUT NOTE VELOCITIES [CONTACT_SUBSTEPS]".into());
    }
    let model = piano_host::load_preset(Path::new(&a[1]))?;
    let note: u8 = a[3].parse()?;
    let cfg = EngineConfig {
        model,
        max_block: SUB_BLOCK,
        ..Default::default()
    };
    let os: usize = a
        .get(5)
        .map(|s| s.parse())
        .transpose()?
        .unwrap_or(contact_oversampling(&cfg, note));
    if os == 0 {
        return Err("CONTACT_SUBSTEPS must be positive".into());
    }
    let inst = Arc::new(Instrument::new(model, cfg.sample_rate));
    let nm = inst.table.get(note);
    let out = Path::new(&a[2]);
    std::fs::create_dir_all(out)?;
    let mut stats = std::fs::File::create(out.join("stats.csv"))?;
    writeln!(stats, "velocity,substeps,duration_ms,compression_mm,peak_n")?;
    for velocity in a[4].split(',').map(str::parse::<u8>) {
        let velocity = velocity?;
        let mut voice = Voice::new(cfg.sample_rate, SUB_BLOCK);
        voice.setup(nm, false);
        voice.strike(nm, hammer_velocity(&cfg, velocity), os, 0.0, 0.0, 0.0);
        let mut forces = std::fs::File::create(out.join(format!("v{velocity}-force.csv")))?;
        writeln!(forces, "sample,s0,s1,s2")?;
        let mut sample = 0;
        while voice.contact.is_active() {
            let mut x = [0f32];
            if voice.contact.run(&mut x, None) == 0 {
                break;
            }
            let f = voice.contact.string_forces();
            writeln!(forces, "{sample},{},{},{}", f[0], f[1], f[2])?;
            sample += 1;
        }
        writeln!(
            stats,
            "{velocity},{os},{},{},{}",
            voice.contact.contact_time() * 1000.,
            voice.contact.max_compression() * 1000.,
            voice.contact.max_force()
        )?;
        let mut modes = std::fs::File::create(out.join(format!("v{velocity}-modes.csv")))?;
        writeln!(modes, "k,oscillator,freq,re,im")?;
        for (i, p) in nm.phys.iter().enumerate() {
            let z = voice.contact.complex_state(i);
            writeln!(
                modes,
                "{},{},{},{},{}",
                p.k,
                i % nm.n_osc,
                p.freq,
                z.re,
                z.im
            )?;
        }
        let mut engine = Engine::with_instrument(cfg, inst.clone());
        let spec = hound::WavSpec {
            channels: 4,
            sample_rate: cfg.sample_rate as u32,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        let mut wav = hound::WavWriter::create(out.join(format!("v{velocity}.wav")), spec)?;
        for start in (0..12000).step_by(SUB_BLOCK) {
            let (mut l, mut r) = ([0f32; SUB_BLOCK], [0f32; SUB_BLOCK]);
            let events = if start == 0 {
                vec![TimedEvent {
                    frame: 0,
                    event: Event::NoteOn { note, velocity },
                }]
            } else {
                vec![]
            };
            engine.process_stereo(&events, &mut l, &mut r);
            for i in 0..SUB_BLOCK {
                for x in [
                    l[i],
                    r[i],
                    engine.bridge_force(nm.region)[i],
                    engine.bridge_velocity(nm.region)[i],
                ] {
                    wav.write_sample(x)?;
                }
            }
        }
        wav.finalize()?;
    }
    Ok(())
}
