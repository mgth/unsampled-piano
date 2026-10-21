//! Contact duration and peak force versus contact oversampling at 48 kHz
//! (same modes): does the base rate suffice?
use piano_core::engine::{contact_oversampling, strike_stats, EngineConfig};

fn main() {
    let cfg = EngineConfig::default();
    println!("note vel | OF=1: t(ms)  F(N) | OF=2: t(ms)  F(N) | OF=4  t(ms)  F(N) | OF=16 t(ms)  F(N) | ΔF OF1  ΔF auto(OF={{}})");
    for note in [21u8, 60, 84, 96, 108] {
        for vel in [20u8, 127] {
            let r: Vec<(f64, f64)> = [1, 2, 4, 16].iter().map(|&os| strike_stats(&cfg, note, vel, Some(os))).collect();
            let auto = contact_oversampling(&cfg, note);
            let ra = strike_stats(&cfg, note, vel, None);
            print!("{note:4} {vel:3} |");
            for (t, f) in &r {
                print!(" {:10.3} {:5.1} |", t * 1e3, f);
            }
            let rf = r[3].1;
            println!(" {:+6.1} %  {:+6.1} % (OF={auto})", 100.0 * (r[0].1 - rf) / rf, 100.0 * (ra.1 - rf) / rf);
        }
    }
}
