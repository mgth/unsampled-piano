# Brightness of the attack and sustain

Status as of 2026-10-22.

## Observation

During the sustain (0.5–1 s), partials 2 to 12 of the middle register were 5 to 14 dB below the mean of the three references (Salamander, Iowa, a commercial modelled piano). The partial profile seemed to show an even larger deficit in the treble.

## Measurement

The per-partial profile is not enough in the treble. Its search window assumes an inharmonicity of 4e-4, whereas the treble exceeds 2e-3. Beyond partial 8, it falls between the partials, on the background noise of the recordings. Two tools, working in octave bands, replace this measurement:

- **`tools/band_decay.py`** gives the levels per octave band in 0.4 s windows. The time points are chosen with `TIMES`.
- **`tools/band_score.py`** gives, per register, the model's deviation from the mean of the references in the 1 to 8 kHz bands, in the attack (0.05 s) and during the sustain (0.6 s). It also gives the root-mean-square deviation. The treble's 8 kHz band is excluded, because it is the noise of the reference recordings.

Deviations of the starting model, in dB (1, 2, 4 and 8 kHz bands):

| Register | Attack | Sustain |
|---|---|---|
| bass | −4 / −9 / −3 / +8 | −8 / −6 / 0 / −4 |
| middle | −3 / −8 / −16 / −11 | −6 / −7 / −15 / −17 |
| treble | −4 / −2 / −2 / (−9) | 0 / −1 / −1 / (−20) |

Root-mean-square deviation: 7.8 dB.

Findings:

- **The treble is right**, except above 8 kHz.
- **The bass and middle lack high frequencies from the attack onwards.** Between the attack and the sustain, their bands decay like those of the references. The ESPRIT decays per frequency band are even a little long in the model.

So the sustain is dull because the attack is. Neither the losses nor the bridge coupling are at fault: spreading the force over the bridge changes nothing (`bridge_spread` 0 to 0.3), nor does the microphone position.

## Diagnosis

1. **Soundboard damping at high frequencies.** It was η = 0.02·(1 + f/700 Hz), i.e. 13 % at 4 kHz and 25 % at 8 kHz, tuned long ago to the attack brightness before the two-layer hammer.
   - The power the string gives to the soundboard does not depend on η. However, the share the soundboard radiates instead of dissipating drops as 1/η.
   - This damping therefore muffled the high frequencies without changing note duration.
   - Real soundboards measure 2 to 3 %, roughly constant.
2. **Hammer felt.** `examples/felt_compression.rs` measures the maximum felt compression:
   - in the middle, it goes from 0.49 mm at pp to 0.60 mm at ff, for a hard core located 0.58 mm deep: the core is barely reached at ff;
   - thinning the middle-register felt makes the core work from mf onwards. The mf 4–8 kHz band then became 18 dB too strong;
   - a faster strike engages the core at ff only.
3. **The hammer score** (`tools/hammer_score.py`) was dominated by the attack profile of C7–C8. Its partials 3 to 6 (8 to 25 kHz) are at the level of the references' noise. This criterion is now excluded, as the dynamics of this register already were. Starting score without it: 0.79.

## Changes

Chosen from two grids of 16 trials, taking into account the brightness deviation and the hammer score:

| Parameter | Before | After |
|---|---|---|
| `soundboard.loss_corner` | 700 Hz | 2500 Hz (η at 4 kHz: 13 % → 5 %) |
| `velocity.vmin` / `vmax` | 0.5 / 3.5 m/s | 0.6 / 4.2 m/s (velocity 77 ↔ 1.94 m/s, versus 2 m/s in Goebl & Bresin) |
| `core_depth_bass` | 0.7 mm | 0.8 mm |
| `core_depth_mid` (new, at C4) | — | 0.57 mm (as before) |
| output gain (`PRESSURE_GAIN`, `FORCE_GAIN`) | 1/22, 1/440 | 1/30, 1/600 (−2.7 dB) |

Explanations:

- **Core depth.** The felt is now defined at three points (A0, C4, C8) rather than two. The bass keeps a thick felt, even with a faster hammer.
- **Una corda.** The shift makes the strings strike felt that the strings have not grooved: softer, as before, and now also thicker above the core (+30 % at full shift). The C4 attack darkens by 1.2 dB instead of about 0.3.
- **Output gain.** The faster hammer played 2.8 dB louder. The gain compensates, and the Chopin keeps its mean level: −38.2 versus −38.3 dB RMS.

## Result

Deviations from the references, in dB (1, 2, 4 and 8 kHz bands):

| Register | Attack | Sustain |
|---|---|---|
| bass | −2 / −5 / 0 / +9 | −5 / −3 / +3 / −1 |
| middle | −2 / −5 / −9 / −4 | −5 / −4 / −8 / −9 |
| treble | −4 / 0 / +1 / (−2) | +2 / +1 / +1 / (−14) |

Root-mean-square deviation: 4.7 dB (7.8 before).

Middle sustain profile (0.5–1 s), partials 2, 3, 4 and 6 relative to partial 1:

| | references | before | after |
|---|---|---|---|
| dB | +0.3 to +4.6 / −7 to −4 / −10 to −4 / −16 to −11 | −2.6 / −9.9 / −14.2 / −20.6 | −0.6 / −6.5 / −10.6 / −18.5 |

Other measurements:
- **Hammer score**: 0.84 (0.79 before).
- **Sustain**: the end of the note (3 s) varies by less than 1.2 dB, within the references' range.
- **ff − pp dynamic difference**: unchanged, because the vmax/vmin ratio stays 7.
- **CPU**: unchanged (38 % of one core at worst). The tests pass.

## What remains

- **Middle at 4–8 kHz** (since addressed: [middle_highs.md](middle_highs.md)): was still 4 to 9 dB below the references, in the attack as well as during the sustain. Neither the felt (hardness, depth, core) nor the soundboard fixes it without brightening the mf or the other registers. Leads:
  - the shape of the contact pulse, with a surface felt whose stiffness grows faster with compression;
  - the duplex resonances.
- **Bass at 8 kHz**: 9 dB above the references in the attack.
- **Treble above 8 kHz**: below the references during the sustain. But the measurement there is dominated by the recordings' noise.
