//! Frequency-domain diagnostic: integrate the driven surface acceleration.
//! This does not change the engine or render a candidate. Lid/wall excluded.
//! driven_rayleigh PRESET NOTE FREQUENCIES_CSV
use piano_core::{
    board::{self, MICS},
    complex::C64,
    params::PianoModel,
};
use std::f64::consts::TAU;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let model = piano_host::load_preset(std::path::Path::new(&args[1]))?.with_macros();
    let model = PianoModel {
        soundboard: model.soundboard.map(|s| s.resolved()),
        ..model
    };
    let sb = model.board().ok_or("no board")?;
    let p = &sb.params;
    assert_eq!(p.lid_angle, 0.0, "diagnostic excludes lid");
    assert_eq!(p.wall_distance, 0.0, "diagnostic excludes wall");
    assert!(p.mics.iter().all(|m| m.2 > 0.0), "above-board only");
    let r = board::region_of(args[2].parse()?);
    let sr = 48000.;
    let modes = board::board_modes(&sb, sr);
    let freqs = args[3]
        .split(',')
        .map(str::parse::<f64>)
        .collect::<Result<Vec<_>, _>>()?;
    let nx = (p.a / p.grid_step).ceil() as usize;
    let ny = (p.b / p.grid_step).ceil() as usize;
    let dx = p.a / nx as f64;
    let dy = p.b / ny as f64;
    // Unit-shape velocity response of each discrete mechanical mode. The
    // spatial radiation kernel multiplies the complete (real-pole-pair)
    // acceleration response at the driving frequency, outside the poles.
    let responses: Vec<Vec<C64>> = freqs
        .iter()
        .map(|f| {
            modes
                .iter()
                .map(|m| {
                    let mut unit = *m;
                    unit.outputs[0] = (1., m.sigma / (TAU * m.freq_hz));
                    board::transfer(&[unit], r, 0, TAU * f, sr)
                        * board::mic_differentiator(TAU * f, sr)
                })
                .collect()
        })
        .collect();
    let mut outputs = vec![[[C64::ZERO; 2]; 6]; freqs.len()];
    // Independently verify this quadrature against sb.radiation() for
    // three modal shapes at their own frequencies (piston / low / high).
    let selected = [0, 1, modes.len() / 4, modes.len() - 1];
    let mut check = [[C64::ZERO; 2]; 4];
    for ix in 0..nx {
        for iy in 0..ny {
            let x = (ix as f64 + 0.5) * dx;
            let y = (iy as f64 + 0.5) * dy;
            let ds = dx * dy * sb.outline.as_ref().map_or(1., |o| o.coverage(x, y, dx, dy));
            if ds == 0. {
                continue;
            }
            let shapes: Vec<f64> = sb
                .modes
                .iter()
                .take(modes.len())
                .map(|m| sb.shape(m, x, y) * if m.m == 0 { p.piston_radiation } else { 1. })
                .collect();
            let paths = p.mics.map(|(mx, my, mz)| {
                let d2 = (mx - x).powi(2) + (my - y).powi(2);
                (
                    (d2 + mz * mz).sqrt(),
                    (d2 + (mz + p.back_depth).powi(2)).sqrt(),
                )
            });
            let kernel = |f: f64, ear: usize| {
                let (front, back) = paths[ear];
                let w = TAU * f;
                (C64::polar(p.case_gain(w) / front, -w * front / 343.)
                    - C64::polar(p.back_gain / back, -w * back / 343.))
                .scale(1.2 / TAU * ds)
            };
            for (fi, f) in freqs.iter().enumerate() {
                let mut surface = [C64::ZERO; 6];
                for ((m, phi), a) in modes.iter().zip(&shapes).zip(&responses[fi]) {
                    let b = if m.freq_hz < 1500. {
                        1
                    } else if m.freq_hz < 4000. {
                        2
                    } else if m.freq_hz < 8000. {
                        3
                    } else if m.freq_hz < 12000. {
                        4
                    } else {
                        5
                    };
                    surface[b] += a.scale(*phi);
                }
                surface[0] = surface[1..].iter().copied().fold(C64::ZERO, |a, b| a + b);
                for b in 0..6 {
                    for ear in 0..2 {
                        outputs[fi][b][ear] += surface[b] * kernel(*f, ear);
                    }
                }
            }
            for (j, i) in selected.iter().enumerate() {
                for ear in 0..2 {
                    check[j][ear] += kernel(modes[*i].freq_hz, ear)
                        * C64::new(0., TAU * modes[*i].freq_hz * shapes[*i]);
                }
            }
        }
    }
    let rad = sb.radiation();
    let mut worst: f64 = 0.;
    for (j, i) in selected.iter().enumerate() {
        for ear in 0..2 {
            worst =
                worst.max((check[j][ear] - rad[*i][ear]).norm() / rad[*i][ear].norm().max(1e-15));
        }
    }
    eprintln!("quadrature_relative_error={worst}");
    assert!(worst < 1e-7);
    println!("frequency,band,pl_re,pl_im,pr_re,pr_im,a_re,a_im");
    for (i, f) in freqs.iter().enumerate() {
        let acc = board::transfer(&modes, r, MICS + r, TAU * f, sr)
            * board::mic_differentiator(TAU * f, sr);
        for (b, label) in ["all", "low", "mid", "upper", "near", "high"]
            .iter()
            .enumerate()
        {
            let [l, rr] = outputs[i][b];
            println!(
                "{f},{label},{},{},{},{},{},{}",
                l.re, l.im, rr.re, rr.im, acc.re, acc.im
            );
        }
    }
    Ok(())
}
