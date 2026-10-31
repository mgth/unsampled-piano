//! Key-to-key evenness of the soundboard alone: for keys C3–C6, the
//! pressure at the two microphones per unit bridge force at partials 1 and
//! 2, from the board's modes (no rendering), and the spacing statistics of
//! the modes below 1 kHz.
//!
//!   cargo run --release -p piano-core --example board_evenness [close|default|player]
use piano_core::params::PianoModel;
use piano_core::soundboard::{Soundboard, SoundboardParams};
use std::f64::consts::TAU;

fn main() {
    let set = std::env::args().nth(1).unwrap_or_else(|| "close".into());
    let model = PianoModel::default();
    let mut p = model.soundboard.unwrap_or(SoundboardParams::GRAND_2M);
    match set.as_str() {
        "close" => {
            p.mics_relative = true;
            p.mics = [(0.294118, 0.346154, 0.4), (0.705882, 0.653846, 0.4)];
            (p.lid_angle, p.lid_removed) = (35.0, false);
        }
        "player" => {
            p.mics_relative = true;
            p.mics = [(-0.38, 0.55, 0.5), (-0.38, 0.45, 0.5)];
            (p.lid_angle, p.lid_removed) = (35.0, false);
        }
        _ => {}
    }
    // LOSS=η, SPREAD=s: board loss factor and bridge spread overrides.
    if let Some(v) = std::env::var("LOSS").ok().and_then(|v| v.parse().ok()) {
        p.loss_factor = v;
    }
    if let Some(v) = std::env::var("SPREAD").ok().and_then(|v| v.parse().ok()) {
        p.bridge_spread = v;
    }
    if let Some(v) = std::env::var("LOAD").ok().and_then(|v| v.parse().ok()) {
        p.string_load = v;
    }
    // RIB_SPACING=m, RIB_SHARE=s, BELOW=Hz: the ribs and the outlined
    // board's own modes' corner.
    let env = |k: &str| std::env::var(k).ok().and_then(|v| v.parse::<f64>().ok());
    if let Some(v) = env("RIB_SPACING") {
        p.rib_spacing = v;
    }
    if let Some(v) = env("RIB_SHARE") {
        p.rib_share = v;
    }
    if let Some(v) = env("BELOW") {
        p.outline_below = v;
    }
    let mut sb = Soundboard::new(p.resolved());
    sb.load_strings(&model.damped_strings());
    // Effective loss factor of the modes, per octave band.
    for (lo, hi) in [(50.0, 100.0), (100.0, 200.0), (200.0, 400.0), (400.0, 800.0), (800.0, 1600.0), (1600.0, 3200.0)] {
        let e: Vec<f64> = sb.modes.iter().filter(|m| m.m > 0 && m.omega / TAU >= lo && m.omega / TAU < hi).map(|m| sb.mode_eta(m)).collect();
        if !e.is_empty() {
            let mean = e.iter().sum::<f64>() / e.len() as f64;
            let max = e.iter().fold(0.0f64, |a, &b| a.max(b));
            eprintln!("  {lo:5.0}–{hi:5.0} Hz: η mean {mean:.3}, max {max:.3} ({} modes)", e.len());
        }
    }
    let mut rad = sb.radiation();
    // MIX=g, SEED=s: mix each mode's shape with its frequency neighbours by a
    // random rotation of angle ~g (emulates an irregular board).
    let mix: f64 = std::env::var("MIX").ok().and_then(|v| v.parse().ok()).unwrap_or(0.0);
    let mut seed: u64 = std::env::var("SEED").ok().and_then(|v| v.parse().ok()).unwrap_or(1);
    let mut rnd = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed >> 11) as f64 / (1u64 << 53) as f64
    };
    let nm = sb.modes.len();
    let mut rot: Vec<(usize, usize, f64)> = Vec::new();
    if mix > 0.0 {
        for _ in 0..3 {
            for i in 0..nm - 1 {
                if sb.modes[i].m == 0 || sb.modes[i + 1].m == 0 {
                    continue;
                }
                let a = mix * (2.0 * rnd() - 1.0) * std::f64::consts::PI;
                rot.push((i, i + 1, a));
            }
        }
        for &(i, j, a) in &rot {
            let (c, s) = (a.cos(), a.sin());
            let (ri, rj) = (rad[i], rad[j]);
            rad[i] = [0, 1].map(|k| ri[k].scale(c) - rj[k].scale(s));
            rad[j] = [0, 1].map(|k| ri[k].scale(s) + rj[k].scale(c));
        }
    }
    let rotate = |mut sh: Vec<f64>| {
        for &(i, j, a) in &rot {
            let (c, s) = (a.cos(), a.sin());
            let (x, y) = (sh[i], sh[j]);
            sh[i] = c * x - s * y;
            sh[j] = s * x + c * y;
        }
        sh
    };

    // Spacing statistics below 1 kHz, in units of the local mean spacing.
    let f: Vec<f64> = sb.modes.iter().map(|m| m.omega / TAU).filter(|&f| f > 50.0 && f < 1000.0).collect();
    let s: Vec<f64> = f.windows(2).map(|w| w[1] - w[0]).collect();
    let mean = s.iter().sum::<f64>() / s.len() as f64;
    let small = s.iter().filter(|&&d| d < 0.25 * mean).count() as f64 / s.len() as f64;
    let large = s.iter().filter(|&&d| d > 2.0 * mean).count() as f64 / s.len() as f64;
    eprintln!(
        "{} modes 50 Hz–1 kHz, mean spacing {mean:.1} Hz; spacings < 0.25×mean: {:.0} % (Poisson 22 %, irregular plate ≈ 5 %), > 2×mean: {:.0} % (Poisson 14 %, irregular ≈ 1 %)",
        f.len(),
        100.0 * small,
        100.0 * large
    );

    println!("key,f0,p1_db,p2_db,total_db,ild1_db");
    let mut prev: Option<f64> = None;
    let mut jumps = Vec::new();
    // FROM=key, TO=key: the keys measured (C3–C6 by default).
    let (from, to) = (env("FROM").map_or(48, |v| v as u8), env("TO").map_or(84, |v| v as u8));
    for note in from..=to {
        let np = model.note(note);
        // DRIVE=key (exact), region (nearest region's centre, as rendered),
        // split (two regions, amplitude weights), cross:<Hz> (split below the
        // crossover, nearest region above).
        let drive = std::env::var("DRIVE").unwrap_or_else(|_| "key".into());
        let region_shapes = |r: usize| rotate(sb.bridge_shapes(piano_core::board::region_center_note(r)));
        let x = ((note.clamp(21, 108) - 21) as f64 + 0.5) * piano_core::board::REGIONS as f64 / 88.0 - 0.5;
        let r0 = (x.floor().max(0.0) as usize).min(piano_core::board::REGIONS - 1);
        let r1 = (r0 + 1).min(piano_core::board::REGIONS - 1);
        let w1 = (x - r0 as f64).clamp(0.0, 1.0);
        let near = piano_core::board::region_of(note);
        let cross: Option<f64> = drive.strip_prefix("cross:").and_then(|v| v.parse().ok());
        let (s0, s1, sn) = (region_shapes(r0), region_shapes(r1), region_shapes(near));
        let shapes_at = |w: f64| -> Vec<f64> {
            match drive.as_str() {
                "region" => sn.clone(),
                "split" => s0.iter().zip(&s1).map(|(a, b)| (1.0 - w1) * a + w1 * b).collect(),
                _ if cross.is_some() => {
                    let lp = 1.0 / (1.0 + (w / (TAU * cross.unwrap())).powi(2)).sqrt();
                    s0.iter().zip(&s1).zip(&sn).map(|((a, b), n)| lp * ((1.0 - w1) * a + w1 * b) + (1.0 - lp) * n).collect()
                }
                _ => rotate(sb.bridge_shapes(note)),
            }
        };
        let mut lv = [0.0; 2];
        let mut ild = 0.0;
        for (h, l) in lv.iter_mut().enumerate() {
            let k = (h + 1) as f64;
            let w = TAU * np.f0 * k * (1.0 + np.b * k * k).sqrt();
            let shapes = shapes_at(w);
            let mut pm = [piano_core::complex::C64::ZERO; 2];
            for ((md, phi), r) in sb.modes.iter().zip(&shapes).zip(&rad) {
                let den = piano_core::complex::C64::new(md.omega * md.omega - w * w, sb.mode_eta(md) * md.omega * w);
                let v = piano_core::complex::C64::new(0.0, w * phi) / den;
                for c in 0..2 {
                    pm[c] += r[c] * v.scale(w / md.omega);
                }
            }
            let (pl, pr) = (pm[0].norm_sqr(), pm[1].norm_sqr());
            *l = 10.0 * (0.5 * (pl + pr)).log10();
            if h == 0 {
                ild = 10.0 * (pl / pr).log10();
            }
        }
        let total = 10.0 * (0.5 * (10f64.powf(lv[0] / 10.0) + 10f64.powf(lv[1] / 10.0))).log10();
        if let Some(q) = prev {
            jumps.push(total - q);
        }
        prev = Some(total);
        println!("{note},{:.1},{:.1},{:.1},{:.1},{:.1}", np.f0, lv[0], lv[1], total, ild);
    }
    let rms = (jumps.iter().map(|j| j * j).sum::<f64>() / jumps.len() as f64).sqrt();
    let max = jumps.iter().fold(0.0f64, |m, j| m.max(j.abs()));
    let big = jumps.iter().filter(|j| j.abs() > 5.0).count();
    eprintln!("{set}: total jump rms {rms:.1} dB, max {max:.1} dB, {big} jumps > 5 dB");
}
