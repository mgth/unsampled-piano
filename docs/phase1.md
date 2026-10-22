# Phase 1 — string and hammer

Status as of 2026-10-21. One modal string per key, struck by a felt hammer through an explicit, passive physical contact. Unisons, bridge coupling and the soundboard come in phase 2.

## Model

**String** (`params.rs`, `engine.rs`).
- Transverse modes up to 0.45·fs: f_k = k·f0·√(1+B·k²).
- f0, B and stretched tuning follow the laws fitted on Salamander (Yamaha C5).
- Losses: σ_k = b1(note) + b2·f_k², the Salamander early-decay law. A single string cannot produce the double decay, hence phase 2.
- Linear density derived from a generic string scale (lengths from 5.2 cm at C8 to 1.45 m at A0, tension 750 N). This scale is not measured.
- Mode shape at the striking point averaged over the felt width (8 mm in the bass, 4 mm in the treble).
- Striking point at 0.12·L up to C6, then 0.07·L at C8.
- Output: bridge force T·∂y/∂x(0).

**Hammer** (`hammer.rs`).
- Mass and Stulov felt law F = Q0·[u^p + α·d(u^p)/dt], per key.
- The contact is integrated by energy quadratisation (Ducceschi et al. 2021, van Walstijn et al. 2023). Each mode uses the same exact recursion as the SIMD bank.
- Each step is solved with a single division, without iteration, at a cost of O(modes).
- Discrete energy proven non-increasing (proof at the top of `hammer.rs`). The two safeguards (no tensile force, ψ reset to zero out of contact) only remove energy.
- The solver takes over during contact, including on a string that is already vibrating (restrike), then hands the state back to the modal bank without loss.
- It also handles hammer re-contacts within 4 ms after separation.
- Passivity guard: the hysteresis is forced to α ≥ 0. Stulov's published formula (via [JOS PASP](https://ccrma.stanford.edu/~jos/pasp/Including_Hysteresis.html)) becomes negative beyond key ~85. It is capped at 30 µs.

**Local oversampling.** During contact only, the solver takes several substeps per sample: ×1 below C6, ×2 from C6 to B6, ×5 above at 48 kHz.

## Checks (tests)

| Test | Result |
|---|---|
| Lossless energy during contact | constant to within < 1e-10 |
| Energy with losses, 6 notes × 4 velocities | never increasing |
| Very heavy string ≈ rigid wall | contact duration = Stulov closed-form formula within 2 % |
| Increasing velocity | shorter contact, larger force |
| Whole keyboard × 4 velocities, restrikes during contact and on a vibrating string | finite, bounded output |
| Audio path | no allocation, bit-exact render |

**Contact convergence** (`cargo run --release -p piano-core --example contact_convergence`). This is the deviation in peak force relative to a 16 times finer step, without oversampling and then with the automatic setting. It is below 1.1 % up to C6 without oversampling. At C8 fortissimo, it reaches +13 % without and +0.2 % with. Contact lasts from 5.9 ms (A0, pp) to 0.47 ms (C8, ff).

**CPU cost.** Sweep of the whole keyboard at velocity 127, one note every 30 ms, pedal down, 64 simultaneous voices: rendered 62 times faster than real time on a single thread, i.e. about 1.6 % of a core. Default polyphony raised to 128 voices.

## Comparison with Salamander

`piano-eval` renders Salamander's 30 notes × 16 layers with the model and measures them with the same analysis tools. `tools/eval_compare.py` produces the figure and the tables (`out/salamander/eval_phase1.md`).

![phase 1](img/phase1_eval.png)

**Calibration** (`tools/grid_hammer.py`, search over 36 settings of the felt exponent, the stiffness and the maximum hammer velocity).
- Brightness: with Stulov's values, the attack brightness (centroid over 50 ms) rises too fast with velocity (×1.86 versus ×1.51). A less nonlinear felt, p reduced by 1 (≈ 2.7 to 4, Russell's dynamic range), with Q0 ×2, divides the error on this curve by 2.6. **For this piano, the measurements settle the Russell-p versus Stulov-p contradiction noted in the report.**
- Dynamics: a hammer velocity of 0.4 to 4 m/s (MIDI velocities 1 to 127, exponential law) reproduces the peak/velocity curve within 1.4 dB rms. The range is 18.9 dB for the model versus 21.4 dB for the reference.
- Result: relative centroid at layer 16 ×1.50 (reference ×1.51), at layer 1 ×0.69 (reference ×0.65).

**What does not fit yet**: the partial profile (relative levels, k ≤ 12).
- The median deviation is 8.5 dB: 3.5 to 12 dB from bass to middle, up to 22 dB at some treble notes (D#5 to A7).
- The grid search does not improve it (7.7 to 8.9 dB for all settings): it does not come from the hammer.
- It comes from what is missing:
  - the soundboard and radiation, which strongly filter the partials (partial 2 of C4 dominates in the recording);
  - the microphones;
  - the 2 or 3 strings of the unison;
  - the generic string scale (lengths, striking points), not measured.
- This is the measurable goal of phases 2 and 4.

## Remaining in phase 1

1. Longitudinal modes and phantom partials in the bass (Bank & Sujbert), and tension modulation (pitch glide in fortissimo).
2. Real string scale: lengths, striking points and tensions of a grand piano, to be digitised from Conklin or measured.
3. Blind comparative listening, note by note, against Salamander.

## Commands

```bash
cargo test --workspace
cargo run --release -p piano-core --example contact_convergence
cargo run --release -p piano-eval -- data/salamander/Samples --out out/salamander/eval_phase1.csv
python3 -I tools/eval_compare.py out/salamander/eval_phase1.csv
python3 -I tools/grid_hammer.py
cargo run --release -p piano-render -- sweep out/sweep.wav --velocity 100
```

> Note (phase 2): the profile metric has been corrected. The old one was sensitive to the partial tracker losing the strongest partial. With the robust metric, this phase 1 configuration gives 5.8 dB, not 8.5 dB. See [phase2.md](phase2.md).
