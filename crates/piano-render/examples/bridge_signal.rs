//! One note as the stage swap needs it (tools/stage_swap.py,
//! docs/stage_swap.md): the bridge velocity at the key's region (m/s), the
//! pressure at the two microphones (Pa; no room, no limiter), and the
//! board's transfer from that region's bridge acceleration to the
//! microphones on an FFT grid.
//!
//!   cargo run --release -p piano-render --example bridge_signal -- <preset.toml> <out prefix>
//!       --note N [--velocity V] [--hold S] [--tail S] [--listening L.toml …] [--transfer LEN]
//!
//! Writes raw little-endian f32 files: `<prefix>_bridge.f32` (the velocity,
//! m/s, 48 kHz), `<prefix>_mics.f32` (left and right interleaved, Pa) and,
//! with `--transfer`, `<prefix>_transfer.f32`: LEN/2 + 1 bins of the rfft
//! grid at 48 kHz, each as (re, im) for the left then the right microphone,
//! in Pa per m/s², the acceleration being the backward difference of the
//! velocity as the engine makes it for the strings.
use piano_core::board::{board_modes, mic_differentiator, mic_transfer, mic_transfer_folded, region_of, to_acceleration_weights, transfer, zones_for};
use piano_core::engine::SUB_BLOCK;
use piano_core::instrument::MICS;
use piano_core::params::PianoModel;
use piano_core::{Engine, EngineConfig, Event, TimedEvent};

const SR: f64 = 48_000.0;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let preset = std::path::Path::new(&args[1]);
    let prefix = args[2].clone();
    let (mut note, mut velocity, mut hold, mut tail, mut len) = (33u8, 100u8, 4.8f64, 0.4f64, 0usize);
    let mut zones: Option<(f64, f64, usize)> = None;
    let mut listening = Vec::new();
    let mut i = 3;
    while i < args.len() {
        let v = args.get(i + 1).cloned().unwrap_or_default();
        match args[i].as_str() {
            "--note" => note = v.parse().unwrap(),
            "--velocity" => velocity = v.parse().unwrap(),
            "--hold" => hold = v.parse().unwrap(),
            "--tail" => tail = v.parse().unwrap(),
            "--transfer" => len = v.parse().unwrap(),
            "--listening" => listening.push(v.clone()),
            "--zones" => {
                // "N": N zones over the board's path range; "W:N": from 0 m.
                zones = Some(match v.split_once(':') {
                    Some((w, n)) => (-1.0, w.parse().unwrap(), n.parse().unwrap()),
                    None => (0.0, 0.0, v.parse().unwrap()),
                });
            }
            a => panic!("unknown argument {a}"),
        }
        i += 2;
    }
    let mut model = piano_host::load_preset(preset).unwrap();
    for l in &listening {
        model = piano_host::load_listening(std::path::Path::new(l), &model).unwrap();
    }
    model.room.enabled = false;
    model.limiter = 0.0;
    let mic_gain = 10f64.powf(model.soundboard.map_or(0.0, |s| s.mic_gain_db) / 20.0) as f32;
    let r = region_of(note);

    // The note, block by block.
    let cfg = EngineConfig { sample_rate: SR, max_block: SUB_BLOCK, model, threads: 1, ..Default::default() };
    let mut engine = Engine::new(cfg);
    let total = ((hold + tail) * SR).ceil() as usize;
    let blocks = total.div_ceil(SUB_BLOCK);
    let off = (hold * SR).round() as usize;
    let (mut bridge, mut mics) = (Vec::with_capacity(blocks * SUB_BLOCK), Vec::with_capacity(2 * blocks * SUB_BLOCK));
    let (mut l, mut rr) = (vec![0.0f32; SUB_BLOCK], vec![0.0f32; SUB_BLOCK]);
    for b in 0..blocks {
        let start = b * SUB_BLOCK;
        let mut ev = Vec::new();
        if b == 0 {
            ev.push(TimedEvent { frame: 0, event: Event::NoteOn { note, velocity } });
        }
        if off >= start && off < start + SUB_BLOCK {
            ev.push(TimedEvent { frame: (off - start) as u32, event: Event::NoteOff { note, velocity: 64 } });
        }
        engine.process_stereo(&ev, &mut l, &mut rr);
        let scale = engine.pressure_scale() * mic_gain;
        bridge.extend_from_slice(engine.bridge_velocity(r));
        for (a, b) in l.iter().zip(&rr) {
            mics.push(a / scale);
            mics.push(b / scale);
        }
    }
    write_f32(&format!("{prefix}_bridge.f32"), &bridge);
    write_f32(&format!("{prefix}_mics.f32"), &mics);
    let peak_v = bridge.iter().fold(0.0f32, |m, v| m.max(v.abs()));
    let peak_p = mics.iter().fold(0.0f32, |m, v| m.max(v.abs()));
    eprintln!("note {note} velocity {velocity} region {r}: bridge velocity peak {peak_v:.4} m/s, pressure peak {peak_p:.3} Pa");

    if len > 0 {
        // The board as the engine runs it: region inputs, the damped
        // strings' load, acceleration weights at the microphones.
        let model = engine.instrument().model;
        let model = PianoModel { soundboard: model.soundboard.map(|s| s.resolved()), ..model };
        let sb = model.board().expect("a preset with a soundboard");
        let mut modes = board_modes(&sb, SR);
        to_acceleration_weights(&mut modes, SR);
        // Each physical microphone as heard, its delay zones and case's slot
        // folded in (switches of the preset); the other slots silent.
        let split = zones_for(&sb, SR).map(|z| z.0);
        let n_phys = split.map_or(MICS, |z| z.mics);
        let bins = len / 2 + 1;
        let mut out = Vec::with_capacity(bins * 4);
        for j in 0..bins {
            if j == 0 {
                out.extend_from_slice(&[0.0, 0.0, 0.0, 0.0]);
                continue;
            }
            let w = std::f64::consts::TAU * j as f64 * SR / len as f64;
            let acc = transfer(&modes, r, piano_core::board::BRIDGE + r, w, SR) * mic_differentiator(w, SR);
            for mic in 0..MICS {
                let h = if mic < n_phys { mic_transfer_folded(&modes, split.as_ref(), r, mic, w, SR) / acc } else { piano_core::complex::C64::ZERO };
                out.push(h.re as f32);
                out.push(h.im as f32);
            }
        }
        write_f32(&format!("{prefix}_transfer.f32"), &out);
        eprintln!("transfer: {bins} bins, {} modes", modes.len());

        // The same transfer with the radiation split in delay zones
        // (Soundboard::radiation_zoned), each zone delayed by its centre.
        if let Some((from, width, nz)) = zones {
            use piano_core::complex::C64;
            let (from, width) = if from < 0.0 {
                (0.0, width)
            } else {
                let (lo, hi) = sb.mic_path_range();
                eprintln!("paths {lo:.2}–{hi:.2} m");
                (lo, (hi - lo) / nz as f64)
            };
            let rad = sb.radiation_zoned(from, width, nz, false);
            let n_mics = sb.params.mics.as_slice().len().min(MICS);
            let mut h = vec![C64::ZERO; bins * n_mics];
            for z in 0..nz {
                let mut mz = board_modes(&sb, SR);
                for (i, m) in mz.iter_mut().enumerate() {
                    let md = &sb.modes[i];
                    let shape = C64::new(1.0, sb.sigma(md) / md.omega);
                    for mic in 0..n_mics {
                        let w = rad[i][mic * nz + z] * shape;
                        m.outputs[mic] = (w.re, w.im);
                    }
                }
                to_acceleration_weights(&mut mz, SR);
                let delay = z as f64 * width / 343.0;
                for j in 1..bins {
                    let w = std::f64::consts::TAU * j as f64 * SR / len as f64;
                    let acc = transfer(&modes, r, piano_core::board::BRIDGE + r, w, SR) * mic_differentiator(w, SR);
                    for mic in 0..n_mics {
                        h[j * n_mics + mic] += mic_transfer(&mz, r, mic, w, SR) / acc * C64::polar(1.0, -w * delay);
                    }
                }
            }
            let out: Vec<f32> = h.iter().flat_map(|c| [c.re as f32, c.im as f32]).collect();
            write_f32(&format!("{prefix}_transfer_zoned.f32"), &out);
            eprintln!("zoned transfer: {nz} zones of {width} m");
        }
    }
}

fn write_f32(path: &str, data: &[f32]) {
    let bytes: Vec<u8> = data.iter().flat_map(|v| v.to_le_bytes()).collect();
    std::fs::write(path, bytes).unwrap();
}
