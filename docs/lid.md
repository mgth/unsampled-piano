# The lid

Status as of 2026-10-30: the lid can be **shut** (`soundboard.lid_shut`). Until then an angle of 0 meant a lid taken away, and the "lid closed" listening preset was in fact a piano without its lid.

## The open lid

A reflecting rectangle hinged on the bass side (y = b), as wide as the board and 1.05·b long, at `lid_angle`. Each board cell gets an image path through it when the specular reflection lands on it; a microphone behind it gets `lid_transmission` of the direct path. No diffraction, no frequency dependence.

### Its shape does not matter

A real lid follows the case: the bentside takes away the corner on the treble side towards the tail. Testing the reflection point against the board's outline instead of the rectangle changes nothing at the listening presets' microphones, 0.0 dB in every third octave from 125 Hz to 8 kHz:

- concert pair (2 m up, 1.6 m in front): no reflection reaches it at all, it passes above the lid's edge;
- close pair (40 cm above the board): every reflection lands near the hinge;
- player's seat: the few that land off the outline are in the 5 cm margin at the keyboard end, not in the missing corner.

So the lid stays a rectangle in the radiation. The 3D view draws the shut lid along the case's outline.

## The shut lid

Shut, the lid closes a box over the board's top face, 25 cm high (`lid_hinge_height` h). Below the box's height resonance c/2h (≈ 690 Hz) the board, the air and the lid move together and the sound gets out round the rim and through the lid; above, the box rings and its felts and wood take a growing share, and only what leaks through the gaps and the opening behind the music desk gets out. The front face's paths (direct, and the open lid's image) get

  g(f) = 1 − s + s·(leak + (1 − leak)/√(1 + (f/fc)²)),  fc = c/2h,

with s = `lid_shut` and leak = `lid_leak` (0.25, −12 dB in the highs). At s = 1 there is no open lid, whatever `lid_angle` says, and the room's directivity is a closed grand's (Q = 2). The back face is untouched. Grands only: an upright has its own case (`case_open`, docs/upright.md). The weights are per mode, so the real-time cost is nil.

No measurement of one piano with its lid open and shut was found (the corpora of `/mnt/local/SSD_B-CT4000/Piano/` have none; the literature found is qualitative: "mostly the timbre, a small level change"). `lid_leak` and the c/2h corner are estimates.

## What it changes

`examples/lid_shut.rs`: mean |pressure per unit modal velocity|² of the board's modes per octave at the listening presets' microphones, shut minus open (or removed), dB:

| | 125 | 250 | 500 | 1000 | 2000 | 4000 | 8000 Hz |
|---|---|---|---|---|---|---|---|
| audience, 1 m (lid removed) | −0.1 | −0.4 | −1.2 | −3.3 | −2.0 | −3.5 | −4.0 |
| concert, 2 m (lid 35°) | −0.1 | −0.3 | −1.2 | −3.4 | −3.2 | −1.8 | −3.6 |
| close, 40 cm (lid 35°) | −3.1 | −1.1 | −2.3 | −3.8 | −6.4 | −7.2 | −7.0 |

The bass is kept, the highs lose 3–4 dB in front of the piano and 7 dB above it. The front face alone loses 8 dB at 4 kHz: in front of the piano the back face, heard round the case with a flat `back_gain`, then takes over. The real case shades the back face's highs, so the model's shut lid is probably too bright there. At the close pair the open lid's image, gone once shut, also took 3 dB of bass with it.

## Where it is

- `soundboard.lid_shut`, `soundboard.lid_leak` (listening: `lid_shut` travels with the microphones' presets).
- `presets/listening/Microphones/audience_lid_closed.toml` shuts it; the other microphone presets set `lid_shut = 0`.
- The `lid` macro moves the closure with the angle: towards shut at −1, none at +1.
- The microphones' page and the 3D view draw the shut lid.
