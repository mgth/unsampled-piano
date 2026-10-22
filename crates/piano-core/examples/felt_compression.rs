//! Peak felt compression versus the depth of the top felt over the hard
//! core, per key and velocity (pp 40 / mf 60 / ff 100): where the core
//! starts to work (analysis of the two-layer hammer).
use piano_core::engine::strike_stats_full;
use piano_core::EngineConfig;

fn main() {
    let cfg = EngineConfig::default();
    println!("key   depth   crush pp / mf / ff (mm)   contact ff (ms), force ff (N)");
    for note in (21u8..=96).step_by(6) {
        let depth = cfg.model.note(note).hammer.core_depth * 1e3;
        let s: Vec<(f64, f64, f64)> = [40u8, 60, 100].iter().map(|&v| strike_stats_full(&cfg, note, v, None)).collect();
        println!(
            "{note:3}  {depth:5.2}   {:5.2} {:5.2} {:5.2}   {:5.2} {:5.0}",
            s[0].2 * 1e3,
            s[1].2 * 1e3,
            s[2].2 * 1e3,
            s[2].0 * 1e3,
            s[2].1
        );
    }
}
