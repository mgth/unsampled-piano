use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use piano_core::modal::{Isa, ModalBank, ModeSpec};
use std::hint::black_box;

fn bench(c: &mut Criterion) {
    let block = 128;
    let mut g = c.benchmark_group("modal_bank_block128");
    for n in [256usize, 4096] {
        // Throughput in mode·samples per block.
        g.throughput(Throughput::Elements((n * block) as u64));
        for isa in Isa::available() {
            let mut b = ModalBank::new(48_000.0, n, block);
            b.set_isa(isa);
            b.set_modes((0..n).map(|i| ModeSpec::driven(30.0 + 5.0 * i as f64, 0.5, 1e-4)));
            let mut out = vec![0.0f32; block];
            let _ftz = piano_core::denormal::DenormalGuard::new();
            g.bench_with_input(BenchmarkId::new(format!("{isa:?}"), n), &n, |bch, _| {
                bch.iter(|| b.process(None, black_box(&mut out)))
            });
        }
    }
    g.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
