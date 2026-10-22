# Phase 0 — tooling and measurements

Status as of 2026-10-21. Measurement machine: AMD Ryzen 9 9950X (Zen 5, 16 cores / 32 threads, AVX-512), Rust 1.99 stable, 48 kHz, blocks of 128.

## What exists

| Crate | Role |
|---|---|
| `piano-core` | DSP engine with no framework dependency: SIMD modal bank (`modal`), polyphonic engine (`engine`), parameters (`params`), FTZ/DAZ guard (`denormal`). |
| `piano-render` | Deterministic offline MIDI→WAV rendering (`midi`, `note`, `sweep`), prints a hash of the render. |
| `piano-analysis` | Partial tracking, (f0, B) estimation, per-partial envelopes, single and double T60, MR-STFT distance, centroid. `piano-analyze` CLI. |
| `piano-bench` | Experiments: `throughput` (modal bank throughput) and `precision` (f32 error). |

The phase 0 voice is **one physical modal string** (modal projection of the force at the striking point, bridge force `4·f0·Σ sin(kπx0)/√(1+Bk²)·e^{-σt}·sin(ω_k t)`, B from Rigaud 2013) excited by an **imposed force** (half-sine whose duration follows Stulov's contact law on a rigid surface and whose impulse equals 2·m·V). This is not yet the coupled hammer–string contact (phase 1). The losses (b1, b2) and the damper are provisional, unfitted values.

## Result 1 — the CPU is not the limiting factor

Capacity = number of damped modes sustained in real time by one core (`piano-bench throughput`).

| ISA | group | no input | with input |
|---|---|---|---|
| generic (SSE2) | 1 | 177,000 | 114,000 |
| AVX2 | 2 | 420,000 | 287,000 |
| AVX-512 | 4 | **842,000** | **580,000** |

- Multi-threaded (AVX-512): 8 threads → 6.5 M modes; 16 threads → 9.7 M; 32 threads (SMT) → 12.5 M.
- The report assumed 2×10⁴ to 10⁵ modes per core: reality is **~10× higher**. A whole piano (88 notes × 3 strings × ~200 modes ≈ 53,000 modes, all strings undamped) would cost only about 6 % of a Zen 5 core for the linear bank alone, or ~20 % of an AVX2 core.
- Grouping several blocks of 16 modes in the same time loop hides the latency of the recursion. In AVX2, the group of 4 overflows the 16 ymm registers and loses; the default group is therefore chosen according to the ISA (`Isa::best_group`).
- This figure covers only the linear bank. The hammer contact, the bridge coupling and the longitudinal products have less vectorisable parts; they remain to be measured in phases 1–2.
- Consequence: we can afford more modes, richer coupling, even local oversampling, without leaving the budget of one core.

## Result 2 — f32 is enough at 48 kHz with the complex form, not with a biquad

`piano-bench precision`: error due to rounding the coefficients to f32.

| fs | frequency | complex form (cents / σ) | real biquad (cents / σ) |
|---|---|---|---|
| 48 kHz | 27.5 Hz (A0) | 0.0001 / 0.1 % | **1.6** / 0.1 % |
| 96 kHz | 27.5 Hz (A0) | 0.0000 / 0.3 % | **10.3** / 0.4 % |
| 96 kHz | 55 Hz | 0.0000 / 0.3 % | 1.6 / 0.1 % |

- The complex form (rotation) is exact in frequency; the direct biquad drifts by several cents in the bass, beyond the perception threshold for beats. Choice confirmed: complex form.
- Accumulated rounding error over 30 s of free decay of A0: −74 dB at 48 kHz, −58 dB at 96 kHz. Acceptable at 48 kHz; at 96 kHz and above, plan for f64 state (or renormalisation) for very lightly damped bass modes. This settles the report's open question.

## Result 3 — the extractors recover the injected parameters

- Synthetic signals (unit tests): B within 3 % and f0 within 0.2 cent from A0 to C7; single T60 within 1 %; double decay (σ1, σ2) within 5 % and component ratio within 1 dB.
- Synthesis → analysis loop on our own renders: A0: B = 5.094e-4 (injected 5.094e-4), f0 within 0.02 cent; T60 of the first 4 partials within 0.5 %; C4: B = 3.270e-4 (injected 3.269e-4), T60 identical to the hundredth.
- The render is bit-identical from one run to the next and regardless of ISA and group (no FMA contraction, fixed summation order); test `all_isas_and_groups_are_bit_identical`.
- The audio path does not allocate (test `realtime.rs` with `assert_no_alloc`, 1,500 blocks of notes, velocities, releases and pedal).

## Remaining to close phase 0

1. ~~Salamander data~~: done, see [salamander.md](salamander.md). Remaining: Iowa MIS (second piano) and digitising the Conklin and Giordano curves.
2. ~~ESPRIT~~: done, see [salamander.md](salamander.md#high-resolution-analysis-sub-band-esprit).
3. Python bindings (pyo3) if we want to fit with scipy; FAD with a declared embedding.
4. `rtsan-standalone` in CI, and a measurement on an ARM machine.

## Commands

```bash
cargo test --workspace
cargo run --release -p piano-bench -- throughput --modes 256,2048,16384 --threads 1,8,16
cargo run --release -p piano-bench -- precision
cargo run --release -p piano-render -- note out/c4.wav --note 60 --velocity 100 --hold 6
cargo run --release -p piano-render -- midi piece.mid out/piece.wav
cargo run --release -p piano-analysis --bin piano-analyze -- out/c4.wav --note 60 --until 5.9
```
