//! Load while a parameter is being edited during playing: thirty keys ring
//! with the pedal down while a slider moves every 250 ms for 3 s, so the
//! replaced engines finish their notes next to the new ones. Per-buffer
//! load as in perf_audit, for 1, 2 and 4 threads.
//!   cargo run --release -p piano-control --example edit_load
use piano_core::params::PianoModel;
use piano_core::{EngineConfig, Event, TimedEvent};
use piano_params::{PianoControl, Value};
use std::time::{Duration, Instant};

#[cfg(target_os = "linux")]
fn real_time(prio: i32) -> bool {
    unsafe extern "C" {
        fn pthread_self() -> usize;
        fn pthread_setschedparam(thread: usize, policy: i32, param: *const i32) -> i32;
    }
    // SAFETY: SCHED_FIFO (1) for the calling thread; sched_param is one int.
    unsafe { pthread_setschedparam(pthread_self(), 1, &prio) == 0 }
}
#[cfg(not(target_os = "linux"))]
fn real_time(_: i32) -> bool {
    false
}

fn main() {
    // Like the player's audio thread (PipeWire's priority): the rebuilds'
    // threads cannot preempt it.
    let rt = std::env::args().any(|a| a == "--rt") && real_time(88);
    println!("audio thread real-time: {rt}");
    let bs = 128;
    for threads in [1, 2, 4] {
        fn promote(_: usize, _: f64) {
            real_time(88);
        }
        let cfg = EngineConfig { max_block: bs, threads, worker_init: Some(promote), ..Default::default() };
        let (mut ctl, mut slot) = piano_control::new(PianoModel::default(), cfg);
        let (mut l, mut r) = (vec![0.0f32; bs], vec![0.0f32; bs]);
        let mut loads = Vec::new();
        let mut engines = 0;
        let mut faults: Vec<(f64, u64)> = Vec::new();
        let per_s = 48_000 / bs;
        let start = Instant::now();
        for b in 0..5 * per_s {
            let mut ev = Vec::new();
            if b == 0 {
                ev.push(TimedEvent { frame: 0, event: Event::Control { cc: 64, value: 127 } });
                ev.extend((0..30).map(|i| TimedEvent { frame: 1, event: Event::NoteOn { note: 30 + 2 * i, velocity: 90 } }));
            }
            if b % (per_s / 4) == per_s / 8 && b < 3 * per_s {
                ctl.set("hardness", Value::Float(0.3 + 0.01 * (b % 7) as f64));
            }
            let f0 = minor_faults();
            let t = Instant::now();
            slot.process_stereo(&ev, &mut l, &mut r);
            let load = t.elapsed().as_secs_f64() * 48_000.0 / bs as f64;
            faults.push((load, minor_faults() - f0));
            loads.push(load);
            engines = engines.max(slot.engines());
            // Play in real time, so that the rebuilds land as they would.
            let due = Duration::from_secs_f64((b + 1) as f64 * bs as f64 / 48_000.0);
            if let Some(w) = due.checked_sub(start.elapsed()) {
                std::thread::sleep(w);
            }
        }
        faults.sort_by(|a, b| b.0.total_cmp(&a.0));
        let worst: Vec<String> = faults.iter().take(5).map(|(l, f)| format!("{:.0} %: {f} page faults", 100.0 * l)).collect();
        println!("  worst buffers: {}", worst.join(", "));
        let mean = loads.iter().sum::<f64>() / loads.len() as f64;
        loads.sort_by(f64::total_cmp);
        let q = |p: f64| loads[((loads.len() - 1) as f64 * p) as usize];
        println!("{threads} thread(s): mean {:5.1} %  p99 {:5.1} %  p99.9 {:5.1} %  worst {:5.1} %  (up to {engines} engines)", 100.0 * mean, 100.0 * q(0.99), 100.0 * q(0.999), 100.0 * q(1.0));
    }
}

/// Minor page faults of the process so far (Linux).
fn minor_faults() -> u64 {
    std::fs::read_to_string("/proc/self/stat").ok().and_then(|s| s.rsplit(')').next().and_then(|r| r.split_whitespace().nth(7)).and_then(|v| v.parse().ok())).unwrap_or(0)
}
