//! The three soundboard banks of a preset, built as the engine builds them,
//! timed alone on the shapes they run with, with a hash of every output bit.
//!
//!   cargo run --release -p piano-render --example board_shapes -- \
//!       [--preset p.toml] [--blocks 400] [--reps 3] [--ghz 5.1] [--isa all|generic|avx2|avx512]
//!       [--only name,name]
//!
//! Per bank (`board_low`: the low modes, one input per key; `board`: the
//! modes above, one input per region; `board_sym`: every mode, the regions'
//! sympathetic force, mics only) and case (keys or regions sounding): ns per
//! mode·sample, the share of the parts' kernels in it, their cycles per
//! item (one chunk's term on one output at one sample, recursion included,
//! at the clock given) and an FNV-1a hash over the bits of every output of
//! every block.
//! The hashes must agree across ISAs and across versions of the kernel.
use piano_core::board::{BoardBank, LowDrive, LOW_INPUTS, MICS, PROBES, REGIONS};
use piano_core::modal::{Isa, LANES};
use std::time::Instant;

const LEN: usize = 64;

struct Fnv(u64);

impl Fnv {
    fn eat(&mut self, xs: &[f32]) {
        for x in xs {
            for b in x.to_bits().to_le_bytes() {
                self.0 ^= b as u64;
                self.0 = self.0.wrapping_mul(0x100_0000_01b3);
            }
        }
    }
}

/// Deterministic pseudo-random forces in [−1, 1) (xorshift).
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        ((self.0 >> 40) as f32 / (1u64 << 24) as f32) * 2.0 - 1.0
    }
}

struct Run {
    hash: u64,
    /// Best ns per block: all of it, and the parts' kernels alone.
    ns: f64,
    kernel: f64,
}

/// Runs `blocks` sub-blocks of `bank` with inputs on `active`; the outputs
/// the engine asks for: bridge velocities and probes with `disp`, the rim's
/// with `extra`.
fn run<const R: usize>(mk: &dyn Fn() -> BoardBank<R>, isa: Isa, active: &[usize], disp: bool, blocks: usize, reps: usize) -> Run {
    let (mut best, mut kernel) = (f64::INFINITY, f64::INFINITY);
    let mut hash = 0;
    for _ in 0..reps {
        let mut bank = mk();
        bank.set_isa(isa);
        let n_ex = bank.extra_outputs();
        let mut inputs = vec![vec![0.0f32; LEN]; R];
        let mut mics = vec![vec![0.0f32; LEN]; MICS];
        let mut dv = vec![vec![0.0f32; LEN]; REGIONS];
        let mut ex = vec![vec![0.0f32; LEN]; n_ex];
        let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
        let mut h = Fnv(0xcbf2_9ce4_8422_2325);
        let (mut time, mut kt) = (0.0, 0.0);
        for _ in 0..blocks {
            for &r in active {
                for x in inputs[r].iter_mut() {
                    *x = rng.next() * 1e-3;
                }
            }
            for b in mics.iter_mut().chain(dv.iter_mut()).chain(ex.iter_mut()) {
                b.fill(0.0);
            }
            let mut m: Vec<&mut [f32]> = mics.iter_mut().map(|v| &mut v[..]).collect();
            // As `process` does, the parts timed apart.
            let t = Instant::now();
            bank.prepare(&inputs, LEN, disp, disp && n_ex > 0);
            let (job, parts) = bank.split();
            let tk = Instant::now();
            for part in parts {
                part.run(&job);
            }
            kt += tk.elapsed().as_secs_f64();
            bank.finish(&mut m, disp.then_some(&mut dv[..]), (disp && n_ex > 0).then_some(&mut ex[..]));
            time += t.elapsed().as_secs_f64();
            for b in mics.iter().chain(dv.iter().filter(|_| disp)).chain(ex.iter().filter(|_| disp)) {
                h.eat(b);
            }
            for p in bank.probes() {
                h.eat(p);
            }
        }
        best = best.min(time * 1e9 / blocks as f64);
        kernel = kernel.min(kt * 1e9 / blocks as f64);
        hash = h.0;
    }
    Run { hash, ns: best, kernel }
}

fn main() {
    let mut preset = String::from("presets/instruments/Concert grand/concert_grand.toml");
    let (mut blocks, mut reps, mut ghz) = (400usize, 3usize, 5.1f64);
    let mut isas = Isa::available();
    let mut only: Option<Vec<String>> = None;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut val = || args.next().expect("missing value");
        match a.as_str() {
            "--preset" => preset = val(),
            "--blocks" => blocks = val().parse().unwrap(),
            "--reps" => reps = val().parse().unwrap(),
            "--ghz" => ghz = val().parse().unwrap(),
            "--only" => only = Some(val().split(',').map(str::to_owned).collect()),
            "--isa" => {
                let v = val();
                if v != "all" {
                    isas.retain(|i| format!("{i:?}").eq_ignore_ascii_case(&v));
                }
            }
            _ => panic!("unknown argument {a}"),
        }
    }
    let sr = 48_000.0;
    let model = piano_host::load_preset(std::path::Path::new(&preset)).unwrap();
    let inst = piano_core::Quality::Full.instrument(&model, sr);
    let board = inst.board.as_ref().expect("a rendered soundboard");
    let rim_couple = inst.rim.as_ref().map_or(&[][..], |r| &r.couple[..]);
    let n_low = inst.low.as_ref().map_or(0, |l| l.n);
    let (low_couple, high_couple) = rim_couple.split_at(n_low.min(rim_couple.len()));
    let low_modes = inst.low.as_ref().map(|l: &LowDrive| l.modes(board));
    let zones = inst.zones;
    let mk_high = || {
        let mut b = BoardBank::with_extra(sr, &board[n_low..], high_couple, LEN);
        if let Some(z) = &zones {
            b.set_zones(z);
        }
        b
    };
    let mk_low = || {
        let mut b = BoardBank::with_extra(sr, low_modes.as_ref().unwrap(), low_couple, LEN);
        if let Some(z) = &zones {
            b.set_zones(z);
        }
        b
    };
    let mk_sym = || {
        let mut b = BoardBank::new(sr, board, LEN);
        if let Some(z) = &zones {
            b.set_zones(z);
        }
        b
    };
    // Mic slots the modes radiate to (the kernel's outputs before the probes).
    let n_mics = (0..MICS).rev().find(|&o| board.iter().any(|m| m.outputs[o] != (0.0, 0.0))).map_or(0, |o| o + 1);
    println!(
        "{preset}: {} board modes, low {n_low} ({} rim outputs), high {} ({} rim outputs), {n_mics} mic slots; {blocks} blocks of {LEN}, best of {reps}",
        board.len(),
        low_couple.first().map_or(0, |e| e.len()),
        board.len() - n_low,
        high_couple.first().map_or(0, |e| e.len()),
    );
    // Keys: the low bank's inputs are per key (A0 = 0); the high bank's are
    // the key's region and the two shared ones (as the engine feeds them).
    let keys10: Vec<usize> = (0..10).map(|i| 7 + 8 * i).collect();
    let keys88: Vec<usize> = (0..88).collect();
    let regions_of = |keys: &[usize]| {
        let mut r: Vec<usize> = keys.iter().map(|&k| piano_core::board::region_of(21 + k as u8)).chain([0, REGIONS - 1]).collect();
        r.sort();
        r.dedup();
        r
    };
    let cases: Vec<(&str, usize, Vec<usize>, bool)> = vec![
        ("board_low 1 key", 0, vec![39], true),
        ("board_low 10 keys", 0, keys10.clone(), true),
        ("board_low 88 keys", 0, keys88.clone(), true),
        ("board 1 key", 1, regions_of(&[39]), true),
        ("board 10 keys", 1, regions_of(&keys10), true),
        ("board 16 regions", 1, (0..REGIONS).collect(), true),
        ("board_sym 3 regions", 2, vec![4, 7, 11], false),
        ("board_sym 16 regions", 2, (0..REGIONS).collect(), false),
    ];
    let wanted = |n: &str| only.as_ref().is_none_or(|o| o.iter().any(|w| n.contains(w.as_str())));
    for (name, bank, active, disp) in cases.iter().filter(|c| wanted(c.0)) {
        let mut line = format!("{name:22} in {:2}", active.len());
        let mut hashes = Vec::new();
        for &isa in &isas {
            // Modes, and the extra outputs with the modes they take.
            let (r, modes, ex) = match bank {
                0 => (run::<LOW_INPUTS>(&mk_low, isa, active, *disp, blocks, reps), n_low, low_couple),
                1 => (run::<REGIONS>(&mk_high, isa, active, *disp, blocks, reps), board.len() - n_low, high_couple),
                _ => (run::<REGIONS>(&mk_sym, isa, active, *disp, blocks, reps), board.len(), &[][..]),
            };
            // Items: one chunk's term on one output at one sample.
            let outs = if *disp { n_mics + PROBES + REGIONS } else { n_mics };
            let n_ex = if *disp { ex.first().map_or(0, |e| e.len()) } else { 0 };
            let items = (modes.div_ceil(LANES) * outs + ex.len().min(modes).div_ceil(LANES) * n_ex) * LEN;
            let ns_ms = r.ns / (modes * LEN) as f64;
            line += &format!(
                "  {isa:?} {ns_ms:.3} ns/m·s ({:.0} % kernel) {:.2} cyc/item",
                100.0 * r.kernel / r.ns,
                r.kernel * ghz / items as f64
            );
            hashes.push(r.hash);
        }
        let same = hashes.windows(2).all(|w| w[0] == w[1]);
        line += &format!("  hash {:016x}{}", hashes[0], if same { "" } else { "  ISAs DIFFER" });
        if !same {
            line += &format!(" {hashes:x?}");
        }
        println!("{line}");
    }
}
