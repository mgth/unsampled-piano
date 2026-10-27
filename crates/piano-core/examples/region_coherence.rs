//! Correlation of the drive shapes of neighbouring bridge regions, per
//! octave band: where a key between two regions may take the mean of the
//! two (coherent) and where only the nearest one.
use piano_core::board::{region_center_note, REGIONS};
use piano_core::params::PianoModel;
use std::f64::consts::TAU;

fn main() {
    let sb = PianoModel::default().board().unwrap();
    let shapes: Vec<Vec<f64>> = (0..REGIONS).map(|r| sb.bridge_shapes(region_center_note(r))).collect();
    let mut lo = 50.0;
    while lo < 6400.0 {
        let hi = 2.0 * lo;
        let idx: Vec<usize> = (0..sb.modes.len()).filter(|&i| sb.modes[i].m > 0 && (lo..hi).contains(&(sb.modes[i].omega / TAU))).collect();
        let mut c = Vec::new();
        for r in 0..REGIONS - 1 {
            let (mut ab, mut aa, mut bb) = (0.0, 0.0, 0.0);
            for &i in &idx {
                let (a, b) = (shapes[r][i], shapes[r + 1][i]);
                ab += a * b;
                aa += a * a;
                bb += b * b;
            }
            c.push(ab / (aa * bb).sqrt().max(1e-30));
        }
        let mean = c.iter().sum::<f64>() / c.len() as f64;
        let mid = c[5..10].iter().sum::<f64>() / 5.0;
        println!("{lo:6.0}–{hi:6.0} Hz: {} modes, correlation of neighbouring regions {mean:.2} (middle {mid:.2})", idx.len());
        lo = hi;
    }
}
