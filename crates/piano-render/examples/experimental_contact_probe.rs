//! Contact diagnostics using the actual experimental instrument, including its
//! frame admittance. Usage: experimental_contact_probe BUNDLE NOTE VELOCITY OUT
//! OUT contains sample,s0,s1,s2 at 48 kHz; stderr reports contact statistics.
#[cfg(not(feature = "experimental-rayleigh"))]
fn main() {
    eprintln!("requires --features experimental-rayleigh");
    std::process::exit(1);
}

#[cfg(feature = "experimental-rayleigh")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use piano_core::{
        EngineConfig,
        engine::{contact_oversampling, hammer_velocity},
        voice::Voice,
    };
    use std::{io::Write, path::Path};
    let a: Vec<String> = std::env::args().collect();
    if a.len() != 5 {
        return Err("BUNDLE NOTE VELOCITY OUT".into());
    }
    let experiment = Box::leak(Box::new(piano_host::experimental::load(Path::new(&a[1]))?));
    let cfg = EngineConfig {
        model: experiment.model,
        experiment: Some(experiment),
        ..Default::default()
    };
    let instrument = cfg.instrument();
    let note: u8 = a[2].parse()?;
    let velocity: u8 = a[3].parse()?;
    if !(21..=108).contains(&note) || !(1..=127).contains(&velocity) {
        return Err("invalid note/velocity".into());
    }
    let nm = instrument.table.get(note);
    let mut voice = Voice::new(48000., cfg.max_block);
    voice.setup(nm, false);
    voice.strike(
        nm,
        hammer_velocity(&cfg, velocity),
        contact_oversampling(&cfg, note),
        0.,
        0.,
        0.,
    );
    let mut output = std::io::BufWriter::new(std::fs::File::create(&a[4])?);
    writeln!(output, "sample,s0,s1,s2")?;
    let mut sample = 0;
    let mut audio = [0f32; 1];
    while voice.contact.is_active() {
        if sample >= 48000 {
            return Err("contact exceeded one second".into());
        }
        if voice.contact.run(&mut audio, None) == 0 {
            break;
        }
        let forces = voice.contact.string_forces();
        if !forces.iter().all(|x| x.is_finite()) {
            return Err("non-finite contact force".into());
        }
        writeln!(output, "{sample},{},{},{}", forces[0], forces[1], forces[2])?;
        sample += 1;
    }
    eprintln!(
        "note={note},velocity={velocity},duration_ms={},peak_n={},compression_mm={},samples={sample}",
        voice.contact.contact_time() * 1000.,
        voice.contact.max_force(),
        voice.contact.max_compression() * 1000.
    );
    Ok(())
}
