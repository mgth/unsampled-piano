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
| `repetition_time`, `repetition_floor` | 0, 0.5 | 0.06 s, 0.5 | slower repetition without double escapement (see below) |
| `soundboard.crown_freq` | 90 Hz | 60 Hz | weaker crown on the smaller board (see the bass balance below) |
| `soundboard.vertical` | false | true | the board stands vertically in the room |
| `soundboard.wall_distance` | 0 | 0 (option: 0.2 m) | reflecting wall behind the board |
| `soundboard.wall_reflection` | 0.9 | 0.9 | its pressure reflection coefficient |
| `soundboard.case_open`, `case_panel_mass` | 1, 6 kg/m² | 1 (option: 0.35) | the closed case in front of the board |
| `room.directivity` | 3 | 0 = from the piano | see below |

The pedal and repetition parameters are live. The others change the radiation weights or the room, so changing them rebuilds the instrument.

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

## Repetition

A grand's action has a double escapement (repetition lever): the hammer is caught halfway down, and a key that has risen only part of the way can throw it again. An upright's action lacks it, so the key must come back nearly all the way up for the jack to reset under the hammer. An upright repeats up to about 7 notes per second, against about 14 for a grand.

The engine only gets note-on and note-off, not the key's position. It used to re-strike at full speed at any time, like an ideal action.

New live parameters:
- `repetition_time` (s, 0 on a grand; 0.06 on the upright presets): the time after a key's release before its action has reset. A strike sooner gets the hammer speed × (floor + (1 − floor)·smoothstep(Δt/T)). Δt runs from the note-off to the next note-on of the same key, and is 0 when there was no note-off.
- `repetition_floor` (0.5): the share left at once. A note is never dropped, so a keyboard player's notes all sound.

A re-strike 10 ms after the release comes out 4.5 dB weaker; one 100 ms after is unchanged. In the MAESTRO performances of `data/midi`, 0.1–1.1 % of the notes are re-struck within 60 ms of their release, 2.8 % in La Campanella. The 0.06 s value is an estimate, with no measurement behind it.

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

## Listening set-ups

Listening report: "with the upright, concert microphones and the concert hall, the room sounds far more reverberant than with a concert grand in the same set-up."

The microphone presets placed their mics in a grand's board coordinates: the concert pair 1.6 m beyond the board's treble edge, in front of the lid's opening, and 2 m above the board. On a standing board the same coordinates put the pair one above the other, 0.5 and 1 m above the floor, off to the side of the upright. So low, the floor's reflection arrived almost with the direct sound and strengthened the room's bass.

Room energy relative to the direct sound, in the concert hall, on a mono mix of C2 to C6 by octaves at velocities 50 and 100, each held 1 s:

| | total | 125 Hz | 250 Hz | 500 Hz | 1 kHz |
|---|---|---|---|---|---|
| concert grand, concert set-up | −7.0 dB | −6.2 | −9.3 | −10.5 | −8.2 |
| upright, the grand's coordinates | −5.2 dB | +1.0 | −0.6 | −9.7 | −9.1 |
| upright, 2.6 m in front, 1.5 m high | −5.1 dB | −0.1 | −5.5 | −6.0 | −5.4 |
| **upright, 2 m in front, 1.5 m high** | **−6.8 dB** | **−7.4** | **−6.1** | **−5.7** | **−7.0** |

New `soundboard.mics_upright`: the mics used instead of `mics` on a vertical board, in the same coordinates (relative with `mics_relative`), with the left one on the bass side. Each listening preset now places them for both kinds of piano:

| Preset | Upright mics | Room re direct (grand / upright) | Direct level (grand / upright) |
|---|---|---|---|
| close | 40 cm in front, over the same parts of the bridge | −24.0 / −23.7 dB | −48.6 / −48.8 dB |
| concert | 2 m in front, 1.5 m high, 0.6 m apart | −7.0 / −6.8 dB | −48.8 / −48.0 dB |
| audience_lid_closed | 1.6 m in front, 1.3 m high | −6.3 / −7.4 dB | −52.0 / −51.8 dB |

At 2 m the upright is miked closer than the grand (about 3 m from the board's centre), as one would mic a piano that projects less. Its lower directivity (Q 1.5 against 3) thus no longer makes the room louder than the grand's.

Most of the room's energy at these distances is in its early reflections, the image sources up to the third order, the floor's among them: without them the room falls to −17 dB for both pianos. The bass of the room is therefore sensitive to the mics' height and distance through the floor's reflection. Moving the upright's pair from 2.6 to 2 m changed the 125 Hz band by 7 dB.

## What is still grand-only

- **The rim.** The model's case is a ring around the board, not an upright's back frame and posts.
- **Diffraction.** The case is a gain on the front face's paths, with no diffraction around its edges.
