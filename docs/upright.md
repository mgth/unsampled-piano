# Uprights

State as of 2026-10-23.

## What was grand-only

The strings, hammers, soundboard and frame take an upright's dimensions from a preset: shorter bass strings, a smaller board, lighter hammers. What was grand-only:

- **Pedals.** The soft pedal always shifted the action (una corda), and the middle pedal was always a sostenuto. On an upright, the soft pedal moves the hammers closer to the strings (half blow). The middle pedal is often a practice mute: a felt strip lowered between hammers and strings.
- **The wall.** An upright stands against a wall, which reflects the sound of the back of its board.
- **The board's bass radiation.** The crown setting of the grand put the small board's first mode at 135 Hz, against 109 Hz on the grand.
- **The room.** It laid every board flat at the height of a grand's.
- **The piano's directivity.** The room used a fixed Q of 3, the value for a grand with its lid open.
- **The case.** The front of an upright's board radiated as if its panels were off.

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
| `soundboard.crown_freq` | 90 Hz | 60 Hz | weaker crown on the smaller board (see the bass balance below) |
| `soundboard.vertical` | false | true | the board stands vertically in the room |
| `soundboard.wall_distance` | 0 | 0 (option: 0.2 m) | reflecting wall behind the board |
| `soundboard.wall_reflection` | 0.9 | 0.9 | its pressure reflection coefficient |
| `soundboard.case_open`, `case_panel_mass` | 1, 6 kg/m² | 1 (option: 0.35) | the closed case in front of the board |
| `room.directivity` | 3 | 0 = from the piano | see below |

The three pedal parameters are live. The others change the radiation weights or the room, so changing them rebuilds the instrument.

**Practice mute.** While CC 66 is down, new strikes go through a strip of t mm of loose felt in series with the hammer felt:

- the hammer's Q0 is multiplied by min(1, 0.09/t), which gives 0.03 for 3 mm;
- its hysteresis is multiplied by 3;
- its core is reached t mm later.

At C4 v100 the attack is 1.9 dB quieter and its brightness 3.5 dB lower. The strip does not touch the ringing strings.

**Wall.** The back face's sound is reflected once more towards the mics. Its path runs from the back-face image (depth `back_depth`) plus 2·`wall_distance`, with gain −β·r. With the mics 1.5 m in front, it lowers the bass keys' brightness (centroid / f0 at ff) from 9.3 to 5.3. That drives the model away from the commercial upright, which was probably modelled without a wall. The wall is therefore an option, off in the presets.

**Case.** The front face's paths (direct, and their lid image) get a gain open + (1 − open)·τ(f). Here `open` is the share that leaves through the openings, and τ = 1/√(1 + (ω·m/2ρc)²) is the panels' mass-law transmission: −13 dB at 100 Hz for 6 kg/m². With `case_open` 0.35, the middle register's ff brightness falls from 2.2 to 1.9 and the bass's from 5.4 to 4.0. The commercial upright is as bright as the model without a case (2.1 and 9.1), so the presets keep the case open. The option gives a closed upright's darker, boxed-in tone.

**Placement in the room.** A vertical board stands at the room's `piano` x, its top edge 0.25 m + its length above the floor, its string side and the mics towards the room. With a wall it stands against the room's wall y = 0, at `wall_distance`. The room's first reflection off that wall is then dropped, since the board's radiation already has it. The point the room takes as the source and the mics follow the same placement.

**Directivity.** With `room.directivity` 0, Q comes from the piano and its set-up:

| | Q |
|---|---|
| grand, lid open | 3 |
| grand, lid closed or removed | 2 |
| upright against a wall | 2 |
| upright standing free | 1.5 |

The grand with its lid open, the close and concert set-ups, keeps the earlier 3. The audience set-up with the lid closed gets 2, so its room is 1.8 dB louder. The uprights get 1.5 (3 dB more room) or 2.

Computing Q from the board's own radiation was tried first, with the Rayleigh integral over 96 directions and clusters of points around each mic, against the power radiated in every direction. Per octave from 125 Hz to 2 kHz it gave 0.1 to 8: with only a few modes per band, their interference at the mics dominates. Its mean, about 1, would also have raised the room by about 5 dB over the calibrated level (docs/room_level.md), so the rule was kept.

## Bass balance

The table compares the bass keys (A0–B2, ff, 0.1–1.1 s) with the commercial upright, with the mics and room of the preset fit (default mics, no room). The gap is the RMS over the third-octave bands from 40 Hz to 2 kHz. The commercial upright was recorded every sixth key, so the bands are coarse.

| Model | Band gap rms (mean) | Brightness, bass / middle (ff) |
|---|---|---|
| commercial upright | | 9.1 / 2.1 |
| before (crown 90 Hz) | 7.5 dB (−3.4) | 9.1 / 2.0 |
| crown 75 Hz | 5.1 dB (−1.7) | 9.2 / 2.2 |
| **crown 60 Hz** | **4.8 dB (−1.3)** | **9.3 / 2.2** |
| crown 60 Hz + wall 0.2 m | 6.4 dB (−2.9) | 5.3 / 2.2 |

A first fit had used the difference between the commercial upright and grand with the close microphones, and gave crown 50 Hz with the wall. Compared directly with the upright, that setting put 7 dB too much in the 100 Hz band and 8 dB too little at 500–800 Hz. The back face's depth and gain (`back_depth`, `back_gain`) moved the bass by only about 3 dB.

## What is still grand-only

- **The rim.** The model's case is a ring around the board, not an upright's back frame and posts.
- **Diffraction.** The case is a gain on the front face's paths, with no diffraction around its edges.
