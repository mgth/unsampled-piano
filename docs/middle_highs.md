# Middle register: the 4–8 kHz highs

State as of 2026-10-22.

## Finding

After [brightness.md](brightness.md), the middle register (C3–B4) stayed 4 to 9 dB below the mean of the references at 4–8 kHz, in the attack as well as during the sustain (`tools/band_score.py`). None of the soundboard or felt settings tried so far fixed it without brightening the mf or the other registers.

## Diagnosis

1. **Neither the soundboard nor the microphones.**
   - `examples/board_transfer.rs` (crate piano-render) gives the transfer from the bridge force to the microphone pressure, per octave band. From 1 to 8 kHz it falls by 6 to 11 dB in every register, with no dip specific to the middle.
   - Moving the microphones (concert position, or above the middle) changes nothing.
2. **The missing energy is in the partials.** In the 2.8–5.6 kHz band of C4 and A4, all the energy of the references sits on the partials: no noise, no longitudinal component between them.
3. **The bridge force itself lacks highs.** Rendering the sequence with `radiate = false` gives the force at the bridge. Relative to the 1 kHz band:

   | | 2 kHz | 4 kHz | 8 kHz |
   |---|---|---|---|
   | middle, bridge force (model) | −10.5 | −25 | −42 |
   | middle, radiated sound (model) | −9 | −27.5 | −48 |
   | middle, references (radiated sound) | −4 to −7 | −16 to −25 | −45 to −47 |
   | treble, bridge force (model) | +10 | +4 | −16 |

   In the treble, the force is very bright and the board attenuates it. In the middle, the hammer pulse lacks fast content.
4. **The felt behaves like a switch.** `examples/felt_compression.rs` gives the peak felt compression:
   - at G3 it went from 0.49 mm at pp to 0.60 mm at ff, with the core 0.58 mm deep;
   - the top felt was thus almost fully crushed from pp on, and all the dynamics went through the core;
   - at ff the hammer entered it by only 0.02 mm.

   One setting at a time (felt and core exponents, hysteresis, contact width, hardness alone) barely moved the middle's 4 kHz band.

## Changes

Chosen on a grid of 15 runs (top felt hardness, core depth at C4, hysteresis), weighing the brightness gap and the hammer score:

| Parameter | Before | After |
|---|---|---|
| `hardness` (top felt, × Stulov) | 0.2 | 0.3 |
| `core_depth_mid` (core at C4) | 0.57 mm | 0.45 mm |
| `hysteresis` | 0.6 | 0.3 |

At ff the hammer goes further into the core (0.05 mm at G3), with a steeper pulse. C4 ff: 44 N in 1.9 ms, against 32 N in 2.0 ms. At pp the top felt is still crushed down to the core.

A thicker bass felt (0.9 mm instead of 0.8) corrected the bass excess at 4–8 kHz, but it raised the hammer score from 0.51 to 1.10, so it was left out.

## Result

Gaps to the references, in dB (1, 2, 4 and 8 kHz bands):

| Register | Attack before | Attack after | Sustain before | Sustain after |
|---|---|---|---|---|
| bass | −2 / −5 / 0 / +9 | −1 / −4 / +3 / +9 | −5 / −3 / +3 / −1 | −4 / −1 / +6 / −2 |
| middle | −2 / −5 / −9 / −4 | −1 / −2 / −3 / +2 | −5 / −4 / −8 / −9 | −4 / 0 / −2 / −5 |
| treble | −4 / 0 / +1 / (−2) | −4 / +3 / −2 / (−1) | +2 / +1 / +1 / (−14) | +2 / +4 / −1 / (−13) |

RMS gap: 4.7 → 3.6 dB.

Other measurements:
- **Hammer score**: 0.84 → 0.51.
- **Middle brightening from pp to ff**: ×1.28 → ×1.39 (references: ×1.15 to ×1.79).
- **Middle sustain profile** (partials 2, 3, 4, 6, 8, 12): −0.3 / −5.6 / −9.2 / −17 / −29 / −34.5 dB, against −0.6 / −6.5 / −10.6 / −18.5 / −32 / −39 before. The reference mean is +2.8 / −5.2 / −7.7 / −13 / −21 / −30.
- **Sustain, dynamic range and CPU load**: unchanged. The Chopin is 0.5 dB louder. The tests pass.

## What remains

- **Bass at 4–8 kHz**: +3 to +9 dB in the attack, +6 dB at 4 kHz during the sustain. The bass reacts to the core depth like a switch, which makes it hard to tune.
- **Partials 8 and above of the middle during the sustain**: still 4 to 8 dB below the references.
- **The two-layer felt is still a threshold model.** A felt whose stiffness grows gradually with compression, like real felt compacted from the outside in, would make the dynamics less dependent on a single tipping point.
