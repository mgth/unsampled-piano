# The lid

Status as of 2026-10-31: the lid has the case's shape, and since 2026-10-30 it can be **shut** (`soundboard.lid_shut`). Until then an angle of 0 meant a lid taken away, and the "lid closed" listening preset was in fact a piano without its lid.

## The open lid

A reflecting plate of the case's shape (`SoundboardParams::lid_outline`) hinged on the bass side (y = b) at `lid_angle`: the board's outline (or the rectangle when none is set) grown by `LID_OVERHANG` (7.5 cm, the rim's thickness), its front flap folded back on it. Each board cell gets an image path through it when the specular reflection lands on it (the point, folded back down about the hinge, inside that outline); a microphone behind it gets `lid_transmission` of the direct path. No diffraction, no frequency dependence.

### Its shape

Until 2026-10-31 the lid was a rectangle, as wide as the board plus 5 cm and 1.05·b long, so a lid over the bentside's missing corner. It now follows the case: the bentside takes that corner away towards the tail. `examples/lid_shape.rs`, lid at 35°, case's shape minus rectangle, dB per third octave from 125 Hz to 8 kHz:

- concert pair (2 m up, 1.6 m in front), close pair (40 cm above the board): 0.0 everywhere. The concert pair is above the lid's edge, the close pair's reflections land near the hinge;
- player's seat: ±0.1, 0.2 at 8 kHz (the reflections in the margin at the keyboard end);
- off the presets, where the corner matters: a pair 60 cm high in front of the treble side −1 to +3 dB below 400 Hz, −0.7 to −1 dB above 3 kHz; a pair over the tail −0.5 to −2.7 dB.

The paths are computed once per build (the radiation weights, ≈ 50 ms for a pair), the same time as with the rectangle; nothing changes in the real-time loop. The 3D view and the microphones' page take the same outline.

## The shut lid

Shut, the lid closes a box over the board's top face, 16 cm high (`lid_hinge_height` h, the rim's top: see [The case's heights](#the-cases-heights)). Below the box's height resonance c/2h (≈ 1070 Hz) the board, the air and the lid move together and the sound gets out round the rim and through the lid; above, the box rings and its felts and wood take a growing share, and only what leaks through the gaps and the opening behind the music desk gets out. The front face's paths (direct, and the open lid's image) get

  g(f) = 1 − s + s·(leak + (1 − leak)/√(1 + (f/fc)²)),  fc = c/2h,

with s = `lid_shut` and leak = `lid_leak` (0.25, −12 dB in the highs). As s grows the open lid's image and shadow fade (reflection × (1 − s), shadowed paths from `lid_transmission` towards 1): at s = 1 there is no open lid, whatever `lid_angle` says, and the room's directivity goes from an open grand's Q = 3 to a closed one's 2. Shutting the lid or opening it from shut is so continuous (test `shutting_the_lid_is_continuous`). The back face is untouched. Grands only: an upright has its own case (`case_open`, docs/upright.md). The weights are per mode, so the real-time cost is nil.

No measurement of one piano with its lid open and shut was found (the corpora of `/mnt/local/SSD_B-CT4000/Piano/` have none; the literature found is qualitative: "mostly the timbre, a small level change"). `lid_leak` and the c/2h corner are estimates.

## What it changes

`examples/lid_shut.rs`: mean |pressure per unit modal velocity|² of the board's modes per octave at the listening presets' microphones, shut minus open (or removed), dB:

| | 125 | 250 | 500 | 1000 | 2000 | 4000 | 8000 Hz |
|---|---|---|---|---|---|---|---|
| audience, 1 m (lid removed) | −0.1 | −0.2 | −0.6 | −2.1 | −1.5 | −3.1 | −3.8 |
| concert, 2 m (lid 35°) | −0.0 | −0.1 | −0.6 | −2.2 | −2.5 | −1.7 | −3.4 |
| close, 40 cm (lid 35°) | −4.0 | −2.1 | −3.0 | −3.1 | −4.8 | −6.9 | −6.6 |

(With the 25 cm box before 2026-10-31 the highs lost about 1 dB more in front of the piano.) The bass is kept, the highs lose 2–4 dB in front of the piano and 7 dB above it. The front face alone loses 8 dB at 4 kHz: in front of the piano the back face, heard round the case with a flat `back_gain`, then takes over. The real case shades the back face's highs, so the model's shut lid is probably too bright there. At the close pair the open lid's image, gone once shut, also took 3 dB of bass with it.

## The case's heights

Until 2026-10-31 the lid's hinge stood 25 cm above the board and the rim's top 5 cm (`rim.top`, a 30 cm wall): the 3D view showed the lid floating 20 cm over the case. No drawing of a grand's vertical section was found; from what was (key tops at 71.5–73 cm from the floor, a lid shut at 99–103 cm, a case about 16 in deep), with the board ≈ 10 cm above the keys (the action's height, estimated):

- the lid's hinge and the rim's top 16 cm above the board (`lid_hinge_height`, `rim.top`), the lid resting on the rim;
- the rim 40 cm high (`rim.height`), its bottom where it was (−24 cm).

`examples/case_heights.rs` renders C2, C4, C6 mf and a ff chord (room on) at the old heights, which it keeps as its reference (`OLD`: 25, 5 and 30 cm), then with each change alone, then both; new minus old, dB:

| microphones | hinge alone | rim alone | both |
|---|---|---|---|
| default (lid removed, 1.5 m) | 0.0 | ≤ +0.2 (2 kHz) | ≤ +0.2 |
| close, lid 35° | +1.0 overall, +2.6 at 1 kHz | 0.0 | +1.0 overall, +2.6 at 1 kHz |
| concert, lid 35° | 0.0 | −1.0 at 1 kHz | −0.1 overall |
| player's seat | ≤ +0.3 | ≤ −0.1 | ≤ +0.3 |
| audience, lid shut | +0.4 overall, +1.1 at 1 kHz | +0.6 at 1 kHz | +0.4 overall, +1.6 at 1 kHz |

The lower lid sends a little more to the close pair; shut, the box's corner moves from 690 to 1070 Hz and lets more of the highs out. The rim radiates 25–48 dB under the note wherever its top is. Presets saved in full before then keep 25 cm and 5 cm; the 3D view draws a grand's case up to the hinge whatever `rim.top` says, so that the lid rests on it.

## Where it is

- `soundboard.lid_shut`, `soundboard.lid_leak` (listening: `lid_shut` travels with the microphones' presets).
- `presets/listening/Microphones/audience_lid_closed.toml` shuts it; the other microphone presets set `lid_shut = 0`.
- The `lid` macro moves the closure with the angle: towards shut at −1, none at +1.
- The microphones' page and the 3D view draw the shut lid.
