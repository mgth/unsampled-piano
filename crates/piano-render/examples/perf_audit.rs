//! Engine load across playing scenarios, as a real-time host sees it: every
//! buffer is timed, and the report gives the mean, the 99th and 99.9th
//! percentiles and the worst buffer, relative to the buffer's duration
//! (100 % = the deadline). A real-time host fails on the worst buffer, not
//! on the mean.
//!
//!   cargo run --release -p piano-render --example perf_audit -- \
//!       [--sr 48000] [--block 128] [--poly 128] [--preset p.toml]... \
//!       [--midi-dir data/midi] [--midi-seconds 60] [--only name,name] [--variants]
//!       [--threads N] [--quality overkill|full|balanced|light]
//!
//! `PIANO_ISA=generic|avx2` caps the SIMD level (smaller machines).
use piano_core::instrument::Instrument;
use piano_core::params::PianoModel;
use piano_core::{Engine, EngineConfig, Event};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

type Timeline = Vec<(f64, Event)>;

fn on(t: f64, note: u8, velocity: u8) -> (f64, Event) {
    (t, Event::NoteOn { note, velocity })
}
fn off(t: f64, note: u8) -> (f64, Event) {
    (t, Event::NoteOff { note, velocity: 64 })
}
fn pedal(t: f64, down: bool) -> (f64, Event) {
    (t, Event::Control { cc: 64, value: if down { 127 } else { 0 } })
}

/// Synthetic scenarios: (name, events, duration in s).
fn scenarios() -> Vec<(&'static str, Timeline, f64)> {
    let mut v: Vec<(&'static str, Timeline, f64)> = Vec::new();
    v.push(("idle", vec![], 3.0));
    v.push(("pedal held, silent", vec![pedal(0.0, true)], 3.0));
    v.push(("one note C4 mf", vec![on(0.1, 60, 80), off(3.0, 60)], 4.0));
    // Ten keys struck in the same buffer: the onset spike.
    let chord = |lo: u8, step: u8| -> Timeline {
        let mut t = vec![pedal(0.0, true)];
        t.extend((0..10).map(|i| on(0.1, lo + step * i, 127)));
        t
    };
    v.push(("10-key chord ff, bass/mid", chord(28, 5), 4.0));
    v.push(("10-key chord ff, treble", chord(84, 2), 2.0));
    // Two hands of five keys re-struck every 60 ms (contact solver busy).
    let mut rep = vec![pedal(0.0, true)];
    for i in 0..80 {
        let t = 0.05 + 0.06 * i as f64;
        for &n in &[36u8, 43, 48, 52, 55, 72, 76, 79, 84, 88] {
            rep.push(on(t, n, 100 + (i % 27) as u8));
        }
    }
    v.push(("10 keys re-struck every 60 ms", rep, 5.0));
    // Thirty keys ringing while the pedal pumps every 150 ms.
    let mut pump = vec![pedal(0.0, true)];
    pump.extend((0..30).map(|i| on(0.05, 30 + 2 * i, 90)));
    for i in 0..30 {
        pump.push(pedal(0.2 + 0.15 * i as f64, i % 2 == 1));
    }
    v.push(("30 keys, pedal pumping", pump, 5.0));
    // Glissando up and down the keyboard, no pedal.
    let mut gl = Vec::new();
    for i in 0..88u8 {
        gl.push(on(0.05 + 0.012 * i as f64, 21 + i, 100));
        gl.push(off(0.15 + 0.012 * i as f64, 21 + i));
        gl.push(on(1.2 + 0.012 * i as f64, 108 - i, 100));
        gl.push(off(1.3 + 0.012 * i as f64, 108 - i));
    }
    v.push(("glissando, no pedal", gl, 3.5));
    // The whole keyboard ff, one key every 30 ms, pedal down (stress_cost).
    let mut st = vec![pedal(0.0, true)];
    st.extend((0..88u8).map(|i| on(0.03 * i as f64, 21 + i, 127)));
    v.push(("whole keyboard ff, pedal", st, 4.64));
    v
}

struct Stats {
    rt: f64,
    mean: f64,
    p99: f64,
    p999: f64,
    max: f64,
    voices: usize,
    modes: usize,
    /// (load, time in s) of the worst buffers.
    worst: Vec<(f64, f64)>,
}

fn run(inst: &Arc<Instrument>, cfg: EngineConfig, tl: &Timeline, dur: f64) -> Stats {
    let mut e = Engine::with_instrument(cfg, inst.clone());
    let (sr, bs) = (cfg.sample_rate, cfg.max_block);
    let (mut l, mut r) = (vec![0.0f32; bs], vec![0.0f32; bs]);
    let mut evs = Vec::with_capacity(1024);
    let (mut clock, mut cur) = (0u64, 0usize);
    let n_blocks = (dur * sr / bs as f64).ceil() as usize;
    let mut loads = Vec::with_capacity(n_blocks);
    let (mut voices, mut modes) = (0, 0);
    let total = Instant::now();
    for _ in 0..n_blocks {
        evs.clear();
        while cur < tl.len() && ((tl[cur].0 * sr) as u64) < clock + bs as u64 {
            let frame = ((tl[cur].0 * sr) as u64).saturating_sub(clock) as u32;
            evs.push(piano_core::TimedEvent { frame, event: tl[cur].1 });
            cur += 1;
        }
        let t = Instant::now();
        e.process_stereo(&evs, &mut l, &mut r);
        loads.push(t.elapsed().as_secs_f64() * sr / bs as f64);
        clock += bs as u64;
        voices = voices.max(e.active_voices());
        modes = modes.max(e.active_modes());
    }
    let el = total.elapsed().as_secs_f64();
    let mut worst: Vec<(f64, f64)> = loads.iter().enumerate().map(|(i, &l)| (l, (i * bs) as f64 / sr)).collect();
    worst.sort_by(|a, b| b.0.total_cmp(&a.0));
    worst.truncate(6);
    let mean = loads.iter().sum::<f64>() / loads.len() as f64;
    loads.sort_by(f64::total_cmp);
    let q = |p: f64| loads[((loads.len() - 1) as f64 * p).round() as usize];
    Stats { rt: dur / el, mean, p99: q(0.99), p999: q(0.999), max: q(1.0), voices, modes, worst }
}

fn print(label: &str, s: &Stats) {
    println!(
        "{label:34} {:6.1}×rt  mean {:5.1} %  p99 {:5.1} %  p99.9 {:5.1} %  max {:6.1} %  voices {:3}  modes {:6}",
        s.rt,
        100.0 * s.mean,
        100.0 * s.p99,
        100.0 * s.p999,
        100.0 * s.max,
        s.voices,
        s.modes
    );
    if std::env::var_os("PERF_WORST").is_some() {
        let w: Vec<String> = s.worst.iter().map(|(l, t)| format!("{:.0} % at {t:.3} s", 100.0 * l)).collect();
        println!("{:34} worst: {}", "", w.join(", "));
    }
}

/// Feature toggles of the model: what each part of the physics costs.
fn variants(m: PianoModel) -> Vec<(&'static str, PianoModel)> {
    let mut v = vec![("full model", m)];
    let mut x = m;
    x.room.enabled = false;
    v.push(("no room", x));
    let mut x = m;
    x.sympathetic.enabled = false;
    v.push(("no sympathy", x));
    let mut x = m;
    x.sympathetic.max_freq = 1500.0;
    v.push(("sympathy below 1.5 kHz", x));
    let mut x = m;
    x.nonlinear.enabled = false;
    v.push(("no phantom partials", x));
    let mut x = m;
    x.frame.enabled = false;
    x.rim.enabled = false;
    v.push(("no frame, no rim", x));
    let mut x = m;
    x.polarization.enabled = false;
    v.push(("one polarisation", x));
    let mut x = m;
    x.soundboard = None;
    v.push(("no soundboard (bridge force)", x));
    v
}

fn main() {
    let mut sr = 48_000.0;
    let mut block = 128;
    let mut poly = 128;
    let mut presets: Vec<PathBuf> = Vec::new();
    let mut midi_dir: Option<PathBuf> = None;
    let mut midi_seconds = 60.0;
    let mut only: Option<Vec<String>> = None;
    let mut with_variants = false;
    let mut threads = 1;
    let mut quality = piano_core::Quality::Full;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut val = || args.next().expect("missing value");
        match a.as_str() {
            "--sr" => sr = val().parse().unwrap(),
            "--block" => block = val().parse().unwrap(),
            "--poly" => poly = val().parse().unwrap(),
            "--preset" => presets.push(val().into()),
            "--midi-dir" => midi_dir = Some(val().into()),
            "--midi-seconds" => midi_seconds = val().parse().unwrap(),
            "--only" => only = Some(val().split(',').map(str::to_owned).collect()),
            "--variants" => with_variants = true,
            "--threads" => threads = val().parse().unwrap(),
            "--quality" => quality = piano_core::Quality::from_name(&val()).expect("overkill, full, balanced or light"),
            _ => panic!("unknown argument {a}"),
        }
    }
    let isa = piano_core::modal::Isa::detect();
    println!("{sr} Hz, {block}-frame buffers, polyphony {poly}, modal ISA {isa:?}, {threads} thread(s), quality {}", quality.name());
    let models: Vec<(String, PianoModel)> = if presets.is_empty() {
        vec![("default".into(), PianoModel::default())]
    } else {
        presets
            .iter()
            .map(|p| (p.file_stem().unwrap().to_string_lossy().into_owned(), piano_host::load_preset(p).unwrap()))
            .collect()
    };
    let mut sc = scenarios();
    if let Some(dir) = &midi_dir {
        let mut files: Vec<_> = std::fs::read_dir(dir).unwrap().flatten().map(|e| e.path()).collect();
        files.sort();
        for f in files.into_iter().filter(|f| f.extension().is_some_and(|x| x == "mid")) {
            let tl: Timeline =
                piano_host::read_midi_file(&f).unwrap().into_iter().filter(|e| e.0 < midi_seconds).collect();
            let dur = tl.last().map_or(0.0, |e| e.0) + 2.0;
            let name: &'static str = Box::leak(f.file_stem().unwrap().to_string_lossy().into_owned().into_boxed_str());
            sc.push((name, tl, dur));
        }
    }
    let wanted = |n: &str| only.as_ref().is_none_or(|o| o.iter().any(|w| n.contains(w.as_str())));
    for (mname, model) in models {
        let list = if with_variants { variants(model) } else { vec![("full model", model)] };
        for (vname, m) in list {
            let t = Instant::now();
            let inst = Arc::new(quality.instrument(&m, sr));
            let build = t.elapsed().as_secs_f64();
            let n_board = inst.board.as_ref().map_or(0, |b| b.len());
            println!("\n== {mname} / {vname}: built in {:.0} ms, {n_board} board modes", 1e3 * build);
            let cfg = EngineConfig { sample_rate: sr, max_block: block, polyphony: poly, model: m, threads, quality, ..Default::default() };
            for (name, tl, dur) in sc.iter().filter(|s| wanted(s.0)) {
                print(name, &run(&inst, cfg, tl, *dur));
            }
        }
    }
}
