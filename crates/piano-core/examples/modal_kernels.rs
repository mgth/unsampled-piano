//! Cost of the string bank's kernels per mode and sample, on every ISA and
//! group size: free (struck strings), one output plus the agraffe's, and
//! two inputs (sympathetic strings).
use piano_core::modal::{Isa, ModalBank, ModeSpec};
use std::time::Instant;

fn main() {
    let (n, len) = (768, 64);
    let specs: Vec<ModeSpec> = (0..n)
        .map(|i| ModeSpec { gain_re: 1e-3, gain_im: 2e-4, gain2_re: -3e-4, gain2_im: 1e-4, out2_re: 0.5, ..ModeSpec::driven(30.0 + 25.0 * i as f64, 0.5, 1e-4) })
        .collect();
    let x: Vec<f32> = (0..len).map(|t| ((t * 13) % 7) as f32 - 3.0).collect();
    let _ftz = piano_core::denormal::DenormalGuard::new();
    for isa in Isa::available() {
        for g in [1, 2, 4] {
            let mut cells = Vec::new();
            for kind in 0..3 {
                let mut b = ModalBank::new(48_000.0, n, len);
                b.set_isa(isa);
                b.set_group(g);
                b.set_modes(specs.iter().copied());
                let (mut o, mut o2) = (vec![0.0f32; len], vec![0.0f32; len]);
                let reps = 3000;
                let t = Instant::now();
                for _ in 0..reps {
                    match kind {
                        0 => b.process(None, &mut o),
                        1 => b.process2(None, &mut o, &mut o2),
                        _ => b.process_two_inputs(&x, &x, &mut o),
                    }
                }
                cells.push(t.elapsed().as_secs_f64() * 1e9 / (reps * n * len) as f64);
            }
            println!("{isa:?} group {g}: free {:.3}  two outputs {:.3}  two inputs {:.3} ns/mode·sample", cells[0], cells[1], cells[2]);
        }
    }
}
