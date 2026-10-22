# Timbre change with dynamics

Status as of 2026-10-22.

## The phenomenon

On a real piano, a note played loudly is not just louder: it is brighter. The hammer felt stiffens much faster than it compresses.

- **Soft playing**: the hammer stays in the soft layer of the felt. The contact is long (about 4 ms for C4) and excites only the lower spectrum.
- **Loud playing**: it reaches the hard part. The contact is short (1.5 to 2 ms) and excites all the harmonics.

In the bass, a second cause adds to this: the nonlinearity of the string itself. A large amplitude changes the tension, which creates “phantom” partials and the rumble of fortissimo.

## Measurement

`tools/hammer_score.py` has a new measure, the slope (`slope`). It takes the ff − pp gain of partials 6 to 12, minus that of partials 1 and 2, in the attack (0–100 ms). The measure covers only the bass and middle: higher up, partials 6 and above exceed 6 kHz and are lost in the recordings' noise.

| Note | Salamander | Iowa | Commercial | Model before |
|---|---|---|---|---|
| C2 | +0.7 | +8.0 | +5.8 | +2.5 |
| C3 | +5.0 | +23.7 | +14.4 | +4.8 |
| C4 | +14.4 | +27.3 | +24.6 | +8.8 |
| C5 | +17.2 | +21.5 | +20.0 | +16.2 |

The model's C4 contact shortened by only 30 % from pp to ff (2.8 → 2.0 ms): the felt was too hard at small compressions.

## Change: a softer felt at first, more nonlinear

Grid `tools/hammer_grid.sh` (about 50 combinations):

| Parameter | Before | After |
|---|---|---|
| `hardness` | 0.7 | 0.1 |
| `hardness_slope` | 0.05 | 0.02 |
| `p_offset` | +0.5 | +2.0 (p ≈ 5.7–7) |
| `hysteresis` | 0.25 | 0.6 |

The exponent exceeds the measurements for a homogeneous felt (2 to 5). It stands in for a multilayer hammer, with a soft surface felt over a hard underlayer: its stiffness grows faster than a single power law. An explicit two-layer model would be more accurate.

## Result

| | Before | After | References |
|---|---|---|---|
| C4 contact, pp → ff | 2.8 → 2.0 ms | 3.7 → 2.3 ms | ≈ 4 → 1.5–2 ms |
| slope, middle | +8.8 dB | +12.8 dB | +17.4 dB |
| slope, bass | +1.9 dB | +2.5 dB | +6.1 dB |
| ff − pp dynamics, treble | 9.1 dB | 11.4 dB | 12.2 dB |
| hammer score | 2.24 | 1.73 | — |

Loudness unchanged: La Campanella peaks at −15 dBFS, the Chopin at −18.7 dBFS.

## What remains

- **Bass**: the felt changes almost nothing there (+2.5 dB versus +6), because the heavy string sets the contact duration. The geometric nonlinearity of the string is needed: tension modulation, longitudinal modes, phantom partials.
- **Middle**: still 4 to 5 dB less brightening than the references.
- **Attack centroid**: its ff/pp ratio moves little (×1.14 versus ×1.4–1.5). It is dominated by the first partials and by the soundboard filtering.

## Bass brightening (2026-10-22)

The phantom partials ([nonlinearity.md](nonlinearity.md)), at their measured level, contributed only +0.1 dB of brightening. The main cause lies elsewhere.

The bass hammer strikes a heavy string: about 0.03 kg/m, two strings for C2. With the middle register's felt hardness, it compresses only about 1 mm, even at ff. The felt then filters only above 2 kHz. Between 150 Hz and 1.5 kHz, where partials 6 to 12 of the bass live, the contact is a linear collision between the hammer mass and the string: its shape does not depend on velocity. Real bass hammers are large and soft, and their felt filters lower, less so the harder the strike.

Changes:
- `hardness_slope_bass` = 0.10, a hardness slope specific to the bass (Q0 × e^{0.10·(key − 40)} below C4);
- velocity 0.5–3.5 m/s.

The contact durations stay physical: 11 ms for A0 pp, versus 9.4 ms before, dominated by the round trip to the agraffe; 7 ms for C2.

| | Before | After | References |
|---|---|---|---|
| slope, bass | +2.6 dB | +5.5 dB | +6.1 (from +0.7 to +8) |
| slope, middle | +12.9 dB | +15.5 dB | +17.4 |
| ff − pp dynamics, bass | 6.6 dB | 8.1 dB | 10.0 (Salamander and the commercial modelled piano) |
| ff − pp dynamics, middle | 7.5 dB | 9.1 dB | 10.1 |
| ff − pp dynamics, treble | 11.4 dB | 13.6 dB | 12.2 |
| hammer score | 1.73 | 1.79 | — |

The middle-register attack is slightly darker (partials 2 to 6 by −1 to −2 dB). The score now excludes the top-treble dynamics (C7–C8): these undamped strings still ring from previous notes in a recorded sequence, and their pp peaks are not their own. A wider velocity range (0.4–4.0) reaches the target in the bass and middle, but widens the treble too much (16.8 dB versus 12.2). Hardening the treble hammers does not fix it.

Remaining:
- **Bass fundamental in the attack**, still too strong compared with partials 2 to 6. The references put those 10 to 30 dB above it; this is what pulls down the mean of partial 4 (−7.7 versus +3 dB).
- **ff/pp centroid**: ×1.12 in the bass, versus ×1.2 to ×2.1 in the references.

## Revision: two-layer hammer (see [presence.md](presence.md))

The very soft, very nonlinear felt of this page cut the high frequencies of the attack (−63 dB at 4–8 kHz in the bass versus −44). It is replaced by a soft surface (hardness 0.2, Stulov's p) over a hard core. `hardness_slope_bass` returns to 0.02 and `p_bass` to 0. The brightening becomes +4.0 dB in the bass and +13.3 in the middle, versus +5.5 and +15.5 with this page's felt, and the attack gets its high frequencies back.
