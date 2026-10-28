//! Contact of one key's strike over the velocities, with a preset:
//! `cargo run --release -p piano-host --example strike_sweep -- preset.toml 84 [vel...]`
//! (with velocities: the force at each sample, as a bar per 0.1 ms row).
use piano_core::engine::{hammer_velocity, strike_force, strike_forces, strike_stats_full, EngineConfig};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let model = piano_host::load_preset(std::path::Path::new(&args[1])).unwrap();
    let note: u8 = args[2].parse().unwrap();
    let cfg = EngineConfig { model, ..Default::default() };
    if args.len() > 3 {
        for v in &args[3..] {
            let vel: u8 = v.parse().unwrap();
            let f = strike_force(&cfg, note, vel);
            let peak = f.iter().cloned().fold(0.0, f64::max);
            println!("velocity {vel}: force per sample (1/48 ms), peak {peak:.1} N");
            let line: String = f.iter().map(|x| b" .:-=+*#%@"[((x / peak) * 9.0).round() as usize] as char).collect();
            println!("{line}");
            println!("  every 5 samples (N): {}", f.iter().step_by(5).map(|x| format!("{x:.2}")).collect::<Vec<_>>().join(" "));
            for s in 0..3 {
                let fs: Vec<f64> = strike_forces(&cfg, note, vel).iter().map(|x| x[s]).collect();
                println!("  string {s}, every 5 samples: {}", fs.iter().step_by(5).take(24).map(|x| format!("{x:.2}")).collect::<Vec<_>>().join(" "));
            }
            // Spectrum of the pulse: its nulls against the partials.
            let n = 4096;
            let peak_bin = |lo: f64, hi: f64| {
                let mut best = (0.0, f64::MAX);
                for k in 1..n / 2 {
                    let fr = k as f64 * cfg.sample_rate / n as f64;
                    if fr < lo || fr > hi {
                        continue;
                    }
                    let (mut re, mut im) = (0.0, 0.0);
                    for (i, x) in f.iter().enumerate() {
                        let ph = -std::f64::consts::TAU * (k * i) as f64 / n as f64;
                        re += x * ph.cos();
                        im += x * ph.sin();
                    }
                    let a = (re * re + im * im).sqrt();
                    if a < best.1 {
                        best = (fr, a);
                    }
                }
                best.0
            };
            let dc: f64 = f.iter().sum();
            let level = |fr: f64| {
                let (mut re, mut im) = (0.0, 0.0);
                for (i, x) in f.iter().enumerate() {
                    let ph = -std::f64::consts::TAU * fr * i as f64 / cfg.sample_rate;
                    re += x * ph.cos();
                    im += x * ph.sin();
                }
                20.0 * ((re * re + im * im).sqrt() / dc).log10()
            };
            println!(
                "  first null {:.0} Hz, second {:.0} Hz; dB re DC at k1..k6: {}",
                peak_bin(300.0, 4000.0),
                peak_bin(peak_bin(300.0, 4000.0) * 1.3, 9000.0),
                (1..=6).map(|k| format!("{:.1}", level(k as f64 * 1046.5))).collect::<Vec<_>>().join(" ")
            );
        }
        return;
    }
    println!("vel  v(m/s)  t(ms)   F(N)   u(mm)");
    for vel in (4u8..=127).step_by(3) {
        let (t, f, u) = strike_stats_full(&cfg, note, vel, None);
        println!("{vel:3} {:7.3} {:6.3} {:6.1} {:7.3}", hammer_velocity(&cfg, vel), t * 1e3, f, u * 1e3);
    }
}
