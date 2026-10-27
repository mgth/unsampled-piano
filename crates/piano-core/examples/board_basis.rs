//! How many drive profiles rebuild every key's drive of the board's low
//! modes: share of the key-shape matrix (88 keys × modes below `f`) kept by
//! its first r principal profiles, and the error per octave band with 16,
//! against the nearest of the 16 bridge regions.
//!
//!   cargo run --release -p piano-core --example board_basis [f_hz]
use piano_core::board::{region_center_note, region_of, REGIONS};
use piano_core::params::PianoModel;
use std::f64::consts::TAU;

fn main() {
    let f_max: f64 = std::env::args().nth(1).and_then(|v| v.parse().ok()).unwrap_or(1000.0);
    let sb = PianoModel::default().board().unwrap();
    let low: Vec<usize> = (0..sb.modes.len()).filter(|&i| sb.modes[i].omega / TAU < f_max).collect();
    let n = low.len();
    let keys: Vec<Vec<f64>> = (21u8..=108).map(|k| {
        let s = sb.bridge_shapes(k);
        low.iter().map(|&i| s[i]).collect()
    }).collect();
    let reg: Vec<Vec<f64>> = (0..REGIONS).map(|r| {
        let s = sb.bridge_shapes(region_center_note(r));
        low.iter().map(|&i| s[i]).collect()
    }).collect();
    // Gram matrix over the modes: Σ_keys Φ Φᵀ.
    let mut g = vec![0.0; n * n];
    for k in &keys {
        for i in 0..n {
            for j in 0..n {
                g[i * n + j] += k[i] * k[j];
            }
        }
    }
    let (vals, vecs) = piano_core::eigen::symmetric(&g, n);
    let total: f64 = vals.iter().sum();
    let mut acc = 0.0;
    print!("{n} modes below {f_max} Hz; energy kept by r profiles:");
    for (r, v) in vals.iter().enumerate() {
        acc += v;
        if [4, 8, 12, 16, 24, 32].contains(&(r + 1)) {
            print!("  {}: {:.2} %", r + 1, 100.0 * acc / total);
        }
    }
    println!();
    // Error per band: projection on 16 profiles vs nearest region.
    let r = REGIONS.min(n);
    let mut lo = 50.0;
    while lo < f_max {
        let hi = (2.0 * lo).min(f_max);
        let idx: Vec<usize> = (0..n).filter(|&i| (lo..hi).contains(&(sb.modes[low[i]].omega / TAU))).collect();
        let (mut ep, mut en, mut s) = (0.0, 0.0, 0.0);
        for (ki, k) in keys.iter().enumerate() {
            let c: Vec<f64> = (0..r).map(|j| (0..n).map(|i| vecs[j][i] * k[i]).sum()).collect();
            let near = &reg[region_of(21 + ki as u8)];
            for &i in &idx {
                let proj: f64 = (0..r).map(|j| c[j] * vecs[j][i]).sum();
                ep += (proj - k[i]).powi(2);
                en += (near[i] - k[i]).powi(2);
                s += k[i] * k[i];
            }
        }
        if s > 0.0 {
            println!("  {lo:5.0}–{hi:5.0} Hz ({:3} modes): error with 16 profiles {:5.1} %, nearest region {:4.0} %", idx.len(), 100.0 * (ep / s).sqrt(), 100.0 * (en / s).sqrt());
        }
        lo = hi;
    }
}
