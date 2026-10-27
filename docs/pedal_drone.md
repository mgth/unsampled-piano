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

## Limits

- The load is the mean value. A real board mode is damped more or less depending on whether a free string's partial falls within its bandwidth; the model damps every mode as if it were average.
- The strings held by keys or by the sostenuto keep their share of the load too, as they already did.
- A two-way coupling, where each listening string takes its own energy from the board, would replace both the mean load and its approximation (docs/dynamics_pedal.md).
