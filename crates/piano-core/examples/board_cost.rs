//! Where does the time go in a soundboard bank?
use piano_core::instrument::Instrument;
use piano_core::modal::{Isa, ModalBank, ModeSpec};
use piano_core::params::PianoModel;
use std::time::Instant;

fn time(label: &str, specs: &[ModeSpec], input: bool, isa: Option<Isa>) {
    let mut bank = ModalBank::new(48_000.0, specs.len(), 128);
    bank.set_modes(specs.iter().copied());
    if let Some(i) = isa {
        bank.set_isa(i);
    }
    let x: Vec<f32> = (0..128).map(|i| ((i * 7919) % 101) as f32 * 0.01 - 0.5).collect();
    let mut out = vec![0.0f32; 128];
    let _ftz = piano_core::denormal::DenormalGuard::new();
    let blocks = 48_000 / 128 * 2;
    let t = Instant::now();
    for _ in 0..blocks {
        bank.process(input.then_some(&x[..]), &mut out);
    }
    let el = t.elapsed().as_secs_f64();
    println!("{label:40} {:.3} ns/mode-sample", el * 1e9 / (blocks * 128 * specs.len()) as f64);
}

fn main() {
    let inst = Instrument::new(PianoModel::default(), 48_000.0);
    let specs = inst.board.as_ref().unwrap().banks[5].clone();
    time("board bank, input", &specs, true, None);
    time("board bank, no input", &specs, false, None);
    let unit: Vec<ModeSpec> = specs.iter().map(|s| ModeSpec { out_re: 1.0, out_im: 0.0, ..*s }).collect();
    time("unit output weights, input", &unit, true, None);
    let low: Vec<ModeSpec> = specs.iter().map(|s| ModeSpec { sigma: 1.0, ..*s }).collect();
    time("sigma = 1, input", &low, true, None);
    time("generic isa, input", &specs, true, Some(Isa::Generic));
}
