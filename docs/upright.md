# Uprights

State as of 2026-10-23.

## What was grand-only

The strings, hammers, soundboard and frame take an upright's dimensions from a preset: shorter bass strings, a smaller board, lighter hammers. What was grand-only:

- **Pedals.** The soft pedal always shifted the action (una corda), and the middle pedal was always a sostenuto. On an upright, the soft pedal moves the hammers closer to the strings (half blow). The middle pedal is often a practice mute: a felt strip lowered between hammers and strings.
- **The wall.** An upright stands against a wall, which reflects the sound of the back of its board.
- **The board's bass radiation.** The crown setting of the grand put the small board's first mode at 135 Hz, against 109 Hz on the grand.

## Measurement

A commercial modelled piano's upright and two of its grands were played through the graph with effects, reverb, limiter and equaliser off. The test played C2 to C6 by octaves at velocities 50 and 100, held 1 s, first without pedal, then with the soft pedal down.

Soft pedal, attack (0–0.3 s), mean over the 10 notes:

| | level | brightness (centroid / f0) |
|---|---|---|
| commercial upright | −1.0 dB (−3.1 to +0.1) | ×0.98 |
| commercial grands | −1.7 dB | ×0.89 |
| model, grand una corda (before, on the upright presets) | −3.5 dB (−0.8 to −7.4) | ×0.74 |
| model, `half_blow` 0.1 | −1.2 dB | ×0.96 |

Release, level 0.3–1.3 s after the key-up relative to just before:

| | commercial upright | commercial grand |
|---|---|---|
| range over the 10 notes | −37 to −51 dB | −25 to −39 dB |

The upright damps at least as well as the grand. The model's dampers were left as they are.

The commercial piano's felt-strip pedal had no measurable effect on these models, so the practice mute is an estimate.

## Changes

| Parameter | Default (grand) | Upright presets | Role |
|---|---|---|---|
| `una_corda_shift` | 1 | 0 | share of the sideways shift at full soft pedal; the felt softening follows it |
| `half_blow` | 0 | 0.1 | hammer speed lost at full soft pedal |
| `mute_felt` | 0 | 3 mm | felt strip of the practice mute on the middle pedal (0: sostenuto) |
| `soundboard.wall_distance` | 0 | 0.2 m | reflecting wall behind the board |
| `soundboard.wall_reflection` | 0.9 | 0.9 | its pressure reflection coefficient |
| `soundboard.crown_freq` | 90 Hz | 50 Hz | first board mode ≈ 113 Hz, near the grand's |

The three pedal parameters are live. The wall changes the radiation weights, so changing it rebuilds the instrument.

**Practice mute.** While CC 66 is down, new strikes go through a strip of t mm of loose felt in series with the hammer felt:

- the hammer's Q0 is multiplied by min(1, 0.09/t), which gives 0.03 for 3 mm;
- its hysteresis is multiplied by 3;
- its core is reached t mm later.

At C4 v100 the attack is 1.9 dB quieter and its brightness 3.5 dB lower. The strip does not touch the ringing strings.

**Wall.** The back face's sound is reflected once more towards the mics. Its path runs from the back-face image (depth `back_depth`) plus 2·`wall_distance`, with gain −β·r. At 63 Hz it adds about 4 dB to the bass keys, and the 1–2 kHz band goes down about 3 dB relative to the rest.

## Bass balance

The table gives the third-octave balance of the bass keys (A0–B2, ff, 0.1–1.1 s), as the difference between the upright and the grand of the same maker, in dB:

| | 50 Hz | 63 Hz | 79 Hz | 100 Hz | 126 Hz | 159 Hz | 200 Hz |
|---|---|---|---|---|---|---|---|
| commercial upright − grand | −8 | −8 | −6 | −3 | +4 | +0 | +1 |
| model before (crown 90 Hz) | −6 | −16 | −11 | −13 | +9 | −1 | −6 |
| model, crown 50 Hz + wall 0.2 m | −6 | −6 | −4 | −2 | +5 | −8 | −3 |

The commercial upright was recorded every sixth key and the model every key, so the table is coarse. The back face's depth and gain (`back_depth`, `back_gain`) moved the bass by only about 3 dB.

## What is still grand-only

- **The room.** It places the board horizontally: the early reflections of a vertical board are approximated.
- **Directivity.** The room presets keep `room.directivity` 3, the value for a grand with its lid open.
- **The case.** The front of the board, behind the action and the panels, radiates to the mics as freely as the back. The model's case is a ring (the rim), not an upright's back frame.
