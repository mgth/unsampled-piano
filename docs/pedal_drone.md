# The drone under the sustain pedal

Status as of 2026-10-27.

## Listening report

A/B of Chopin op. 10 no. 3 (Liang's Disklavier, played with the pedal), while testing the board's outline (docs/outline.md): "unbearable, a hum, a very present and very unpleasant resonance". The rectangular board was better, "but well below the real one". The recording, by contrast, is "very balanced, and its resonance is harmonious and pleasant".

## What it was

The first minute of the piece, by third-octave band, model minus recording (close microphones, both at −20 LUFS):

| | 100 Hz | 125 Hz | 200 Hz |
|---|---|---|---|
| played without the pedal | −1 dB | −1 dB | −2 dB |
| played with the pedal | **+9 dB** | −3 dB | +5 dB |

Without the pedal the model's balance matches the recording within ±2 dB from 100 to 500 Hz. On the Disklavier the pedal barely changes it: +0.8 dB in all. In the model it added 4.7 dB, nearly all at 108–110 Hz: the board's first mode, which follows no note of the piece. With the outline, that mode moves to 122 Hz, right on B2, the piece's dominant (+16 to +22 dB there).

Three suspects, each tried on the pedalled Chopin (model minus recording):

| | 100 Hz | 200 Hz |
|---|---|---|
| as it was | +9 dB | +5 dB |
| the damped strings' load kept with the pedal down | −2 dB | +3 dB |
| no sympathetic strings | +3 dB | 0 dB |
| more loss on the board's low modes (`loss_low_corner` 150 Hz) | −1 dB | +5 dB |

The damped strings load the board (`string_load`, docs/evenness.md): their characteristic impedance √(T·μ) damps its modes, η from 0.02 to 0.07–0.09 at 100–200 Hz. The pedal lifted that load with the dampers. With the pedal down the board's lowest modes were left with η ≈ 0.02, and rang.

## Why the load stays

A free string loads the bridge too. Its input impedance at the bridge is Z0·coth(γL), with γ = α + ik its lossy wavenumber. Averaged over frequency, Re coth(γL) is 1 for any α > 0 (coth z = 1 + 2·Σ e^(−2nz)). The frequency-averaged resistance is therefore √(T·μ), as for a damped string; it is only gathered at the string's partials instead of spread. In the bass a board mode's bandwidth (≈ 2 Hz at 110 Hz) holds one or two partials of the strings below it, about 0.7 per Hz.

The model's sympathy is one-way (Bank, Zambon and Fontana 2010, docs/phase3.md): the listening strings ring from the bridge's motion but take nothing back from the board. Keeping the load is the other half of that exchange: the energy the free strings take from the board is the energy they then sing back. Without it, the board kept its energy and the strings sang on top.

`soundboard.string_load_free` (0–1, 1 by default): the share of the damped strings' load the board keeps while the pedal lifts the dampers. 0 reproduces the earlier sound bit for bit; without the pedal nothing changes.

## Results

**Liang's isolated notes with and without the pedal** (`tools/pedal_bench.py`, velocity 80). The halo is the energy the pedal adds between a note's own partials, 0.2–0.9 s (pedal minus no pedal):

| register | recording | before | string_load_free 1 |
|---|---|---|---|
| bass | +4.6 dB | +13.3 dB | +2.2 dB |
| middle | +15.5 dB | +20.1 dB | +19.4 dB |
| treble | +14.0 dB | +20.7 dB | +21.5 dB |

(CC64 at 96; at 127 the same within 0.5 dB.) The bass's halo was the board's own drone; the other registers' halo is the strings', and barely moves. "Hold" and "after" change by < 1 dB, except the bass's "after" at full pedal (−8.8 → −7.2 dB).

**Pieces with the pedal** (model minus recording, third-octave bands):

| | Chopin 100 Hz | Chopin 125 Hz | Beethoven op. 31 no. 2 finale, 100 Hz |
|---|---|---|---|
| before | +9 dB | −3 dB | +6 dB |
| string_load_free 1 | −2 dB | −1 dB | −1 dB |
| same, with the grand's outline (impedance slope 0.01) | −6 dB | +2 dB | −5 dB |

With the outline, B2's drone in the Chopin falls from +7 to +2 dB at 125 Hz. Beethoven's +7 dB at 200 Hz stays without the outline; it is not the board's first mode.

**Bench** (`cargo xtask bench`, pedal up): unchanged, bit for bit (the A0–C8 sweep without the pedal renders with the same hash).

## The sympathy's strength

Listening to the fixed sound (2026-10-27): in the Chopin "a resonance too marked around B5". B5's line (988 Hz) lay 8 dB under the recording without the pedal and level with it with the pedal: the pedal raised it 7 dB more than on the Disklavier. All the B strings (B1–B5) share that frequency, and the piece is in E major.

The sympathy is one-way: each free string is driven by the bridge's motion as if the others were not there. A real string ringing at its own frequency brakes the bridge there. Its input impedance at resonance, ≈ √(T·μ)·Q/(nπ), reaches a few hundred kg/s, of the order of the board's at the bridge. With a note's three strings and their octaves, the bridge moves several dB less at those frequencies. The more strings share a frequency, the more the one-way coupling overshoots.

Gain on the bridge-to-string coupling (`sympathetic.gain`), Chopin lines (model minus recording) and the isolated notes' halo (CC64 96, model minus recording):

| gain | A3 | B4 | B5 | halo bass | halo middle | halo treble | bench |
|---|---|---|---|---|---|---|---|
| 1 | +12 dB | +6 dB | −1 dB | −2.4 dB | +3.9 dB | +7.6 dB | 1.07 |
| 0.6 | +9 dB | +4 dB | −5 dB | −2.8 dB | +0.7 dB | +9.6 dB | 1.13 |
| 0.4 | +7 dB | +3 dB | −7 dB | −3.3 dB | −2.1 dB | +9.7 dB | |

0.6 brings B5 back among its treble neighbours (D#6 −5 dB, F#6 −7 dB) and matches the middle's halo. The user preferred it by ear. The treble's halo does not follow the gain: unexplained so far. The bench, played without the pedal, loses through the top (C7–C8): these keys have no dampers and always listen, and their sound below the fundamental (1–2 kHz bands, already 10–20 dB under the real grands) comes from the sympathy.

A two-way coupling would brake the bridge where strings share a frequency and leave the rest: it would serve both the pedalled middle and the undamped top.

## The note on the board's first mode

With the grand's outline, "B2 sounds a bit like a barrel". The board's first mode (122 Hz with the outline, 109 Hz on the rectangle) sits on B2 (A2 on the rectangle). That note comes out ≈ 8 dB louder at the attack than its neighbours (velocity 80, close microphones) and loses it fast. Moving the mode (lower crown) moves the barrel to A2. More loss on the lowest board modes (`loss_low_corner` 200 Hz: η × 3.7 at 110 Hz, × 1.6 at 260 Hz) evens the attack (B2 −19 → −22 dB, its neighbours −23 and −24). On the outline with gain 0.6 it costs 1.14 → 1.16 on the bench. Kept as an option of the outline (out/ab_pedal2/E2_outline.toml), not in the default; 200 Hz became the default later, for the bass's articulation (docs/bass_metal.md).

## Limits

- The load is the mean value. A real board mode is damped more or less depending on whether a free string's partial falls within its bandwidth; the model damps every mode as if it were average.
- The strings held by keys or by the sostenuto keep their share of the load too, as they already did.
- A two-way coupling, where each listening string takes its own energy from the board, would replace both the mean load and its approximation (docs/dynamics_pedal.md).
