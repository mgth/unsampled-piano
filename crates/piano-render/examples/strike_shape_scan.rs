//! Hammer force per key: contacts (separate episodes of force), their
//! lengths, and the share of the force after the first contact.
//!   cargo run --release -p piano-render --example strike_shape_scan -- lo hi [velocities…]
//! With EPS=1 in the environment, each contact is listed (times, impulse, peak).
use piano_core::engine::strike_forces;
use piano_core::EngineConfig;

fn main() {
    let cfg = EngineConfig::default();
    let a: Vec<String> = std::env::args().skip(1).collect();
    let (lo, hi): (u8, u8) = (a[0].parse().unwrap(), a[1].parse().unwrap());
    let vels: Vec<u8> = if a.len() > 2 { a[2..].iter().map(|s| s.parse().unwrap()).collect() } else { vec![45, 70, 100] };
    let dt = 1.0 / cfg.sample_rate;
    for note in lo..=hi {
        let mut line = format!("{note}");
        for &v in &vels {
            let fs = strike_forces(&cfg, note, v);
            let tot: Vec<f64> = fs.iter().map(|f| f.iter().sum()).collect();
            let peak = tot.iter().cloned().fold(0.0, f64::max);
            // episodes above 1 % of the peak
            let mut eps = Vec::new();
            let mut start = None;
            for (i, &f) in tot.iter().enumerate() {
                match (start, f > 0.01 * peak) {
                    (None, true) => start = Some(i),
                    (Some(s), false) => { eps.push((s, i)); start = None; }
                    _ => {}
                }
            }
            if let Some(s) = start { eps.push((s, tot.len())); }
            let imp: Vec<f64> = eps.iter().map(|&(s, e)| tot[s..e].iter().sum::<f64>() * dt).collect();
            let all: f64 = imp.iter().sum();
            let late = if all > 0.0 { 1.0 - imp[0] / all } else { 0.0 };
            // ripple: number of local maxima above 20 % of the peak
            let mut lm = 0;
            for i in 1..tot.len().saturating_sub(1) {
                if tot[i] > tot[i - 1] && tot[i] >= tot[i + 1] && tot[i] > 0.2 * peak { lm += 1; }
            }
            let per: Vec<String> = (0..3).map(|j| format!("{:.0}", fs.iter().map(|f| f[j]).fold(0.0, f64::max))).collect();
            if std::env::var("EPS").is_ok() {
                let d: Vec<String> = eps.iter().zip(&imp).map(|(&(s, e), i)| format!("{:.2}-{:.2}ms {:.1}mNs {:.1}N", s as f64 * dt * 1e3, e as f64 * dt * 1e3, i * 1e3, tot[s..e].iter().cloned().fold(0.0, f64::max))).collect();
                line += &format!(" | v{v}: {}", d.join(", "));
                continue;
            }
            line += &format!(" | v{v}: {} contacts {:.2} ms, late {:.0}%, maxima {lm}, peak {:.0} N [{}] end {:.2} ms",
                eps.len(), (eps[0].1 - eps[0].0) as f64 * dt * 1e3, late * 100.0, peak, per.join(" "), tot.len() as f64 * dt * 1e3);
        }
        println!("{line}");
    }
}
