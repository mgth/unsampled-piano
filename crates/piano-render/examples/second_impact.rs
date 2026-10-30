//! The hammer's force per key at mf (playback scale): its episodes (contact,
//! separation, re-contact) and the share of the force's spectrum above 4
//! and 8 kHz (the fizz of D#4–F4, docs/felt.md).
//!   cargo run --release -p piano-render --example second_impact -- [overlay.toml…]
use piano_core::engine::strike_force;
use piano_core::EngineConfig;
use std::path::Path;

fn main() {
    let mut cfg = EngineConfig::default();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    cfg.model = piano_host::load_listening(&root.join("presets/touch/disklavier_playback.toml"), &cfg.model).unwrap();
    for p in std::env::args().skip(1) {
        cfg.model = piano_host::load_listening(Path::new(&p), &cfg.model).unwrap();
    }
    let sr = cfg.sample_rate;
    let vel: u8 = std::env::var("VEL").ok().and_then(|v| v.parse().ok()).unwrap_or(56);
    let mut hf = vec![];
    let quiet = std::env::var("QUIET").is_ok();
    if !quiet { println!("key   T0 ms | episodes (start–end ms, peak N) | >4k dB  >8k dB"); }
    for note in 48u8..=77 {
        let f = strike_force(&cfg, note, vel);
        let t0 = 1000.0 / (440.0 * 2f64.powf((note as f64 - 69.0) / 12.0));
        let mut eps = vec![];
        let mut i = 0;
        while i < f.len() {
            if f[i] > 1e-9 {
                let s = i;
                let mut pk: f64 = 0.0;
                while i < f.len() && f[i] > 1e-9 { pk = pk.max(f[i]); i += 1; }
                eps.push(format!("{:.2}–{:.2} {:.1}", s as f64 / sr * 1e3, i as f64 / sr * 1e3, pk));
            } else { i += 1; }
        }
        let (a4, a8) = shares(&f, sr);
        // The first episode alone (the re-contacts left out).
        let first = f.iter().position(|&x| x > 1e-9).unwrap_or(0);
        let end = f[first..].iter().position(|&x| x <= 1e-9).map(|e| first + e).unwrap_or(f.len());
        let (_, b8) = shares(&f[..end], sr);
        hf.push((note, a8));
        if quiet { continue; }
        let names = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
        println!("{:<4} {:5.2} | {} | {:6.1} {:6.1} | first alone >8k {:6.1}", format!("{}{}", names[note as usize % 12], note / 12 - 1), t0, eps.join(" ; "), a4, a8, b8);
        if std::env::var("TRACE").ok().and_then(|t| t.parse::<u8>().ok()) == Some(note) {
            for (j, x) in f.iter().enumerate().skip((2.0e-3 * sr) as usize) { println!("  {:.3} ms {:.4} N", j as f64 / sr * 1e3, x); }
        }
    }
    // Key to key, as the bench's hf_even: each key of C3–B4 about the
    // median of the keys within ±6.
    let res: Vec<(u8, f64)> = hf.iter().filter(|(m, _)| (48..=71).contains(m)).map(|&(m, v)| {
        let mut nb: Vec<f64> = hf.iter().filter(|(j, _)| *j != m && j.abs_diff(m) <= 6).map(|x| x.1).collect();
        nb.sort_by(f64::total_cmp);
        let med = if nb.len() % 2 == 1 { nb[nb.len() / 2] } else { (nb[nb.len() / 2 - 1] + nb[nb.len() / 2]) / 2.0 };
        (m, v - med)
    }).collect();
    let rms = (res.iter().map(|r| r.1 * r.1).sum::<f64>() / res.len() as f64).sqrt();
    let top = res.iter().cloned().fold((0, f64::MIN), |a, b| if b.1 > a.1 { b } else { a });
    let mean = hf.iter().filter(|(m, _)| (48..=71).contains(m)).map(|x| x.1).sum::<f64>() / 24.0;
    let gaps: Vec<String> = res.iter().filter(|r| (61..=68).contains(&r.0)).map(|r| format!("{:+.0}", r.1)).collect();
    println!("force 8–11k key to key: rms {rms:.1} dB, max {:+.1} dB (key {}), mean {mean:.1} dB | C#4–G#4 {}", top.1, top.0, gaps.join(" "));
}

/// Share (dB) of the force spectrum above 4 kHz and at 8–11.3 kHz.
fn shares(f: &[f64], sr: f64) -> (f64, f64) {
        let n = 8192;
        let (mut tot, mut a4, mut a8) = (0.0, 0.0, 0.0);
        for k in 1..n / 2 {
            let w = 2.0 * std::f64::consts::PI * k as f64 / n as f64;
            let (mut re, mut im) = (0.0, 0.0);
            for (j, x) in f.iter().enumerate() { re += x * (w * j as f64).cos(); im -= x * (w * j as f64).sin(); }
            let p = re * re + im * im; let hz = k as f64 * sr / n as f64;
            tot += p; if hz >= 4000.0 { a4 += p; } if hz >= 8000.0 && hz < 11300.0 { a8 += p; }
        }
    (10.0 * (a4 / tot).log10(), 10.0 * (a8 / tot).log10())
}
