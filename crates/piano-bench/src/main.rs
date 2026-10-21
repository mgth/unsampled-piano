//! Phase 0 experiments on the modal core:
//! - `throughput`: how many modes one core (or N threads) sustains in real time;
//! - `precision`: f32 frequency/decay error of the complex one-pole form
//!   against the classic real biquad, across the keyboard.

use clap::{Parser, Subcommand};
use piano_core::modal::{Isa, ModalBank, ModeSpec};
use std::hint::black_box;
use std::time::Instant;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Real-time capacity in modes per core.
    Throughput {
        #[arg(long, default_value_t = 48_000.0)]
        sr: f64,
        #[arg(long, default_value_t = 128)]
        block: usize,
        /// Seconds of audio rendered per measurement.
        #[arg(long, default_value_t = 2.0)]
        seconds: f64,
        /// Bank sizes to test.
        #[arg(long, value_delimiter = ',', default_values_t = [256usize, 2048, 16384])]
        modes: Vec<usize>,
        /// Also measure aggregate capacity with this many threads (best ISA, group 4).
        #[arg(long, value_delimiter = ',')]
        threads: Vec<usize>,
    },
    /// Coefficient-quantisation error of f32 resonators.
    Precision,
}

fn bank(sr: f64, n: usize, block: usize) -> ModalBank {
    let mut b = ModalBank::new(sr, n, block);
    // Log-spaced frequencies over the audio band, piano-like decay rates.
    b.set_modes((0..n).map(|i| {
        let f = 27.5 * (0.45 * sr / 27.5).powf(i as f64 / n as f64);
        ModeSpec::driven(f, 0.3 + 1e-6 * f * f, 1e-4)
    }));
    b
}

/// Returns real-time capacity: modes that fit in one core at this rate.
fn measure(b: &mut ModalBank, sr: f64, block: usize, seconds: f64, input: bool) -> f64 {
    let blocks = (seconds * sr / block as f64).ceil() as usize;
    let mut x = vec![0.0f32; block];
    x[0] = 1.0;
    let mut out = vec![0.0f32; block];
    let _ftz = piano_core::denormal::DenormalGuard::new();
    // Warm-up.
    for _ in 0..blocks / 10 + 1 {
        b.process(input.then_some(&x[..]), &mut out);
    }
    let t = Instant::now();
    for _ in 0..blocks {
        b.process(input.then_some(black_box(&x[..])), black_box(&mut out));
    }
    let wall = t.elapsed().as_secs_f64();
    let audio = (blocks * block) as f64 / sr;
    b.len() as f64 * audio / wall
}

fn throughput(sr: f64, block: usize, seconds: f64, sizes: &[usize], threads: &[usize]) {
    println!("sr {sr} Hz, block {block}; capacity = modes sustainable in real time on one core\n");
    println!("{:>8} {:>8} {:>6} {:>6} {:>14} {:>10}", "modes", "isa", "group", "input", "capacity", "ns/mode·s");
    for &n in sizes {
        for isa in Isa::available() {
            for g in [1, 2, 4] {
                for input in [false, true] {
                    let mut b = bank(sr, n, block);
                    b.set_isa(isa);
                    b.set_group(g);
                    let cap = measure(&mut b, sr, block, seconds, input);
                    println!(
                        "{n:>8} {:>8} {g:>6} {:>6} {:>14.0} {:>10.3}",
                        format!("{isa:?}"),
                        if input { "yes" } else { "no" },
                        cap,
                        1e9 / (cap * sr)
                    );
                }
            }
        }
        println!();
    }
    for &t in threads {
        let n = *sizes.last().unwrap();
        let caps: Vec<f64> = std::thread::scope(|s| {
            let hs: Vec<_> = (0..t)
                .map(|_| {
                    s.spawn(move || {
                        let mut b = bank(sr, n, block);
                        measure(&mut b, sr, block, seconds, false)
                    })
                })
                .collect();
            hs.into_iter().map(|h| h.join().unwrap()).collect()
        });
        let total: f64 = caps.iter().sum();
        println!("{t:>3} threads × {n} modes: aggregate {total:.0} modes, {:.0} per thread", total / t as f64);
    }
}

fn precision() {
    println!("f32 pole quantisation: frequency error (cents) and decay-rate error (%)");
    println!("decay rates follow the placeholder law (T60 ≈ 28 s at A0)\n");
    println!(
        "{:>7} {:>10} {:>9} | {:>12} {:>10} | {:>12} {:>10}",
        "sr", "freq Hz", "T60 s", "cplx cents", "cplx σ %", "biquad cents", "biquad σ %"
    );
    for sr in [48_000.0f64, 96_000.0] {
        for (f, sigma) in [(27.5, 0.25), (55.0, 0.3), (110.0, 0.35), (261.6, 0.5), (1046.5, 2.0), (4186.0, 8.0), (12000.0, 120.0)]
        {
            let t = 1.0 / sr;
            let th = std::f64::consts::TAU * f * t;
            let r = (-sigma * t).exp();
            // Complex one-pole.
            let (pr, pi) = ((r * th.cos()) as f32 as f64, (r * th.sin()) as f32 as f64);
            let th_c = pi.atan2(pr);
            let sig_c = -(pr.hypot(pi)).ln() * sr;
            // Real biquad y = a1·y1 − a2·y2.
            let (a1, a2) = ((2.0 * r * th.cos()) as f32 as f64, (r * r) as f32 as f64);
            let rb = a2.sqrt();
            let th_b = (a1 / (2.0 * rb)).clamp(-1.0, 1.0).acos();
            let sig_b = -rb.ln() * sr;
            println!(
                "{sr:>7} {f:>10.1} {:>9.2} | {:>12.4} {:>10.2} | {:>12.4} {:>10.2}",
                6.9078 / sigma,
                1200.0 * (th_c / th).log2(),
                100.0 * (sig_c - sigma) / sigma,
                1200.0 * (th_b / th).log2(),
                100.0 * (sig_b - sigma) / sigma
            );
        }
        println!();
    }

    // Accumulated rounding over a long free decay, f32 recursion vs exact.
    println!("30 s free decay of the A0 fundamental in f32 (complex form) vs f64 exact:");
    for sr in [48_000.0f64, 96_000.0] {
        let (f, sigma) = (27.5, 0.25);
        let mut b = ModalBank::new(sr, 1, 256);
        b.set_modes([ModeSpec::driven(f, sigma, 1.0)]);
        let mut x = vec![0.0f32; 256];
        x[0] = 1.0;
        let mut out = vec![0.0f32; 256];
        let n_blocks = (30.0 * sr / 256.0) as usize;
        let mut last = Vec::new();
        for i in 0..n_blocks {
            out.fill(0.0);
            b.process((i == 0).then_some(&x[..]), &mut out);
            if i == n_blocks - 1 {
                last = out.clone();
            }
        }
        // Compare the last block with the exact response (pole rounding included).
        let t = std::f64::consts::TAU * f / sr;
        let r = (-sigma / sr).exp();
        let (pr, pi) = ((r * t.cos()) as f32 as f64, (r * t.sin()) as f32 as f64);
        let (th_q, r_q) = (pi.atan2(pr), pr.hypot(pi));
        let n0 = (n_blocks - 1) * 256;
        let (mut err, mut refp) = (0.0f64, 0.0f64);
        for (j, &y) in last.iter().enumerate() {
            let n = (n0 + j) as f64;
            let want = r_q.powf(n) * (th_q * n).sin();
            err += (y as f64 - want).powi(2);
            refp += want * want;
        }
        println!(
            "  sr {sr}: rounding-noise error at 30 s = {:.1} dB relative to the signal (pole quantisation excluded)",
            10.0 * (err / refp).log10()
        );
    }
}

fn main() {
    match Cli::parse().cmd {
        Cmd::Throughput { sr, block, seconds, modes, threads } => throughput(sr, block, seconds, &modes, &threads),
        Cmd::Precision => precision(),
    }
}
