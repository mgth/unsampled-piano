//! Render the very same experimental engine used by the UI, or stress it.
//! rayleigh_live BUNDLE OUT NOTES VELOCITIES HOLD TAIL BLOCK THREADS
//! rayleigh_live BUNDLE OUT stress BLOCK THREADS
#[cfg(not(feature = "experimental-rayleigh"))]
fn main() {
    eprintln!("build with --features experimental-rayleigh");
}
#[cfg(feature = "experimental-rayleigh")]
mod allocation {
    use std::{
        alloc::{GlobalAlloc, Layout, System},
        sync::atomic::{AtomicBool, AtomicUsize, Ordering},
    };
    pub struct Counter;
    static WATCH: AtomicBool = AtomicBool::new(false);
    static COUNT: AtomicUsize = AtomicUsize::new(0);
    unsafe impl GlobalAlloc for Counter {
        unsafe fn alloc(&self, l: Layout) -> *mut u8 {
            if WATCH.load(Ordering::Relaxed) {
                COUNT.fetch_add(1, Ordering::Relaxed);
            }
            unsafe { System.alloc(l) }
        }
        unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
            if WATCH.load(Ordering::Relaxed) {
                COUNT.fetch_add(1, Ordering::Relaxed);
            }
            unsafe { System.dealloc(p, l) }
        }
    }
    pub fn check(f: impl FnOnce()) {
        COUNT.store(0, Ordering::Relaxed);
        WATCH.store(true, Ordering::SeqCst);
        f();
        WATCH.store(false, Ordering::SeqCst);
        assert_eq!(COUNT.load(Ordering::Relaxed), 0, "audio callback allocated/freed memory");
    }
}
#[cfg(feature = "experimental-rayleigh")]
#[global_allocator]
static ALLOC: allocation::Counter = allocation::Counter;
#[cfg(feature = "experimental-rayleigh")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use piano_core::{Engine, EngineConfig, Event, TimedEvent, experimental::LATENCY};
    use std::{path::Path, sync::Arc, time::Instant};
    let a: Vec<String> = std::env::args().collect();
    let e = Box::leak(Box::new(piano_host::experimental::load(Path::new(&a[1]))?));
    let stress = a[3] == "stress";
    let block: usize = a[if stress { 4 } else { 7 }].parse()?;
    let threads = a[if stress { 5 } else { 8 }].parse()?;
    let cfg = EngineConfig {
        model: e.model,
        max_block: block,
        threads,
        experiment: Some(e),
        ..Default::default()
    };
    let inst = Arc::new(cfg.instrument());
    std::fs::create_dir_all(&a[2])?;
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: 48000,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    if stress {
        let mut engine = Engine::with_instrument(cfg, inst);
        let mut events = Vec::new();
        // Repeated wide chords, repeated notes, sustain, half pedal, soft,
        // sostenuto and all-notes-off, followed by a full decay tail.
        for k in 0..12u64 {
            let t = k * 12000;
            for (j, note) in [36, 43, 48, 55, 60, 64, 67, 72, 79, 97, 99, 102, 108].into_iter().enumerate() {
                events.push((
                    t + j as u64 * 97,
                    Event::NoteOn {
                        note,
                        velocity: if k % 3 == 0 {
                            45
                        } else if k % 3 == 1 {
                            56
                        } else {
                            100
                        },
                    },
                ));
                events.push((t + 8000 + j as u64 * 97, Event::NoteOff { note, velocity: 64 }));
            }
            events.push((
                t,
                Event::Control {
                    cc: 64,
                    value: if k < 9 { 127 } else { 40 },
                },
            ));
            events.push((
                t + 3000,
                Event::Control {
                    cc: 67,
                    value: if k % 2 == 0 { 127 } else { 0 },
                },
            ));
            events.push((
                t + 4000,
                Event::Control {
                    cc: 66,
                    value: if k % 2 == 0 { 127 } else { 0 },
                },
            ));
        }
        for cc in [64, 66, 67] {
            events.push((144000, Event::Control { cc, value: 0 }));
        }
        events.push((144000, Event::AllNotesOff));
        events.sort_by_key(|x| x.0);
        let total = 48000 * 7;
        let (mut l, mut r) = (vec![0.; block], vec![0.; block]);
        let mut ev = Vec::with_capacity(256);
        let mut times = Vec::new();
        let mut pos = 0;
        let mut peak = 0f32;
        let mut maxvoices = 0;
        let mut writer = hound::WavWriter::create(Path::new(&a[2]).join("stress.wav"), spec)?;
        for start in (0..total).step_by(block) {
            let len = block.min(total - start);
            ev.clear();
            while pos < events.len() && events[pos].0 < (start + len) as u64 {
                ev.push(TimedEvent {
                    frame: (events[pos].0 - start as u64) as u32,
                    event: events[pos].1,
                });
                pos += 1;
            }
            let begin = Instant::now();
            allocation::check(|| engine.process_stereo(&ev, &mut l[..len], &mut r[..len]));
            times.push(begin.elapsed().as_secs_f64() * 48000.0 / len as f64);
            maxvoices = maxvoices.max(engine.active_voices());
            for i in 0..len {
                for x in [l[i], r[i]] {
                    assert!(x.is_finite());
                    peak = peak.max(x.abs());
                    writer.write_sample(x)?;
                }
            }
        }
        writer.finalize()?;
        times.sort_by(f64::total_cmp);
        let mean = times.iter().sum::<f64>() / times.len() as f64;
        println!(
            "stress block={block} threads={threads} mean_load={mean:.4} p99_load={:.4} max_load={:.4} overruns={} peak={peak:.5} voices={maxvoices}",
            times[times.len() * 99 / 100],
            times.last().unwrap(),
            times.iter().filter(|&&x| x > 1.).count()
        );
        return Ok(());
    }
    let notes = a[3].split(',').map(str::parse::<u8>).collect::<Result<Vec<_>, _>>()?;
    let velocities = a[4].split(',').map(str::parse::<u8>).collect::<Result<Vec<_>, _>>()?;
    let hold: f64 = a[5].parse()?;
    let tail: f64 = a[6].parse()?;
    let original = ((hold + tail) * 48000.).ceil() as usize;
    let total = original + LATENCY;
    let off = (hold * 48000.).round() as usize;
    for n in notes {
        for &v in &velocities {
            let mut engine = Engine::with_instrument(cfg, inst.clone());
            let (mut l, mut r) = (vec![0.; block], vec![0.; block]);
            let mut writer = hound::WavWriter::create(Path::new(&a[2]).join(format!("n{n}v{v}.wav")), spec)?;
            let mut events = Vec::with_capacity(2);
            for start in (0..total).step_by(block) {
                let len = block.min(total - start);
                events.clear();
                if start == 0 {
                    events.push(TimedEvent {
                        frame: 0,
                        event: Event::NoteOn { note: n, velocity: v },
                    });
                }
                if off >= start && off < start + len {
                    events.push(TimedEvent {
                        frame: (off - start) as u32,
                        event: Event::NoteOff { note: n, velocity: 64 },
                    });
                }
                allocation::check(|| engine.process_stereo(&events, &mut l[..len], &mut r[..len]));
                for i in 0..len {
                    if start + i >= LATENCY {
                        writer.write_sample(l[i])?;
                        writer.write_sample(r[i])?;
                    }
                }
            }
            writer.finalize()?;
            println!("rendered {n}:{v}");
        }
    }
    Ok(())
}
