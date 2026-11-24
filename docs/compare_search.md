# Tuning against the comparison tool

State as of 2026-11-24.

`tools/compare_search.py` tunes a preset one parameter at a time, and the comparison tool alone decides what is kept. It is not the bench of `docs/bench.md`.

## What judges a step

`unsampled-piano-compare --score out.json` opens no window. It renders the model's 264 notes (88 keys, pp, mf, ff) in memory and measures them as the window does, in about 35 s. It reports two numbers over all the tool's measures:

- **Measures off:** how many have |z| > 2, the red of the window.
- **Mean gap:** the mean |z|, each z clipped at 3, so that a reference reading its noise floor does not decide alone.

The band leaves out the pianos heard to buzz at D#4 ff (`--band-without bivib_grand,iowa,liang`). It holds Headroom, Helgesson, Plath, Salamander and Splendid. Splendid has no D#4 ff to judge it by.

A step is kept in either of two cases:

- at least 2 measures fewer are off, and the mean gap rises by at most 0.002;
- no more measures are off, and the mean gap falls by at least 0.003.

## The search

Each of the 66 parameters is tried up, then down, with up to three steps in a direction that keeps improving. Passes repeat until one keeps nothing. The steps are then halved, and the search stops when a pass at half steps keeps nothing.

The first run took 794 trials over six passes. They kept 8, 5, 1, 0, 1 and 0 steps.

| Parameter | Before | After |
|---|---|---|
| `hammer_mass_scale` | 1.053 | 1.003 |
| `hardness_slope_bass` | 0.0378 | 0.0318 |
| `polarization.slope_to` | 700 Hz | 532.5 Hz |
| `polarization.radiation_bass` | 0.5 | 0.6 |
| `stretch.k_t` | 0.1 | 0.0826 |
| `losses.b2_top` | 2.0e-7 | 3.9e-7 |
| `unison.width_curve` | 1.0 | 0.8 |
| `soundboard.loss_factor` | 0.0223 | 0.0245 |
| `body.levels` at 125 and 250 Hz | 0, −4 dB | +3, −1 dB |
| `body.speed_halo` at C5 | 0.3 | 0.4 |

The values are rounded after the search; the rounded preset was scored again.

## Result

Measures off, before and after:

| Measure | Band of 5, before | After | All 8 real pianos, before | After |
|---|---|---|---|---|
| Early decay | 49 | 34 | 44 | 38 |
| Partial 1 ripple | 53 | 47 | 57 | 56 |
| Late decay | 46 | 43 | 56 | 54 |
| Rise time | 41 | 33 | 35 | 31 |
| T60 of partial 1 | 28 | 22 | 30 | 29 |
| Spectral tilt | 24 | 18 | 23 | 18 |
| Between partials | 21 | 22 | 14 | 13 |
| Tuning | 23 | 11 | 9 | 5 |
| Inharmonicity B | 8 | 8 | 0 | 1 |
| **Total** | **293** | **238** | **268** | **245** |
| Mean gap | 0.861 | 0.810 | 0.811 | 0.775 |

The buzz measure, at ff (partials 17–24 re partials 9–16, then re the strongest partial, dB):

| Key | Before | After |
|---|---|---|
| C#4 | −21.3, −49.6 | −25.9, −56.3 |
| D4 | −21.9, −55.8 | −26.4, −62.4 |
| D#4 | −23.1, −51.4 | −29.1, −60.2 |
| E4 | −26.0, −57.9 | −31.7, −66.6 |
| F4 | −25.4, −61.2 | −32.3, −70.9 |

At D#4 the model leaves the group of pianos heard to buzz (about −23 dB).

## What this does not say

- Nothing here was listened to. The search undoes part of two choices made by ear: `width_curve` (1.0) and `slope_to` (700 Hz).
- A lower count on one tool's measures is not a better piano. It has to be checked at the keyboard.
- The mean gap of the late decay rises slightly (1.221 to 1.244 on the band of 5).

## Second run, with the sustain's overtones (2026-11-19)

The tool now counts partials 2–4 in the sustain, and its brightness floor is a factor of 1.2 instead of 3.2 (docs/compare_tool.md). From the preset above, measured again with them: 338 measures off on the band of 5. Partials 2–4 alone have 68 off of 212.

The search took 526 trials over four passes and kept three steps, all on the body's halo:

| Parameter | Before | After |
|---|---|---|
| `body.levels` at 1 kHz | −18 dB | −21 dB |
| `body.levels` at 4 kHz | −20 dB | −18.5 dB |

It read each partial at its strongest line, which made a unison's split partial look weaker. That reading was then corrected to the power summed over each partial's band (measure version 5). Everything below was scored again with the corrected measure:

| | Band of 5, before | After | All 8, before | After |
|---|---|---|---|---|
| Between partials | 22 | 19 | 13 | 9 |
| Partials 2–4, sustain | 68 | 68 | 59 | 59 |
| Rise time | 33 | 33 | 31 | 30 |
| **Total** | **338** | **335** | **321** | **316** |
| Mean gap | 0.956 | 0.955 | 0.907 | 0.909 |

None of the 66 parameters moves the overtones of the sustain. Their excess sits on single keys (against the band's median: D#3 +18 dB, D#5 +19, D6 +19, G1 −24, D7 −27). It takes a lever that acts key by key, such as the board's modes at each bridge point, not the instrument-wide values searched here.

A checkpoint of `tools/compare_search.py` now records the measure version, the band, and the starting preset's path and contents (SHA-256, since a pull may change the file in place). If any of them differs, the search starts again from a freshly scored baseline.

## Third run, on the treble, after the top bridge points moved (2026-11-24)

The top strings' bridge points moved across the board (docs/string_sections.md, "The top strings square to the hammers' line"). Scored on all 88 keys and three layers (measures version 9, band of 5), the move alone is nearly neutral: 866 measures off for 856 with the old points, mean gap 1.203 for 1.193; bass 197 for 203, middle 359 for 361, treble (C5–B6) 240 for 223, top 70 for 69. In the treble the rise time (14 off for 7), the late decay (23 for 19) and partials 2–4 (28 for 25) lose, the lines' early decays gain (2.5–4 kHz 8 for 14, 4–7 kHz 22 for 26). The bass and the middle move too, as the bridge is one piece of the board's structure and its modes are dealt again.

The search then ran from the new points on 27 of the 66 parameters: those that act on the treble and are fits, not facts of the D (no masses, lengths, tensions, tuning, inharmonicity or bridge points). `losses.b2_top` was held at 2.0e-7 or more, under which the middle was heard to buzz. 383 trials over seven passes kept 32 steps (20, 7, 1, 0, then 2, 2, 0 at half steps):

| Parameter | Before | After |
|---|---|---|
| `strike_ratio_top` | 0.05 | 0.08 |
| `shank_freq_mid` | 700 Hz | 460 Hz |
| `hardness_slope` | −0.05 | −0.07 |
| `polarization.detune_cents` | 0.3 | 0.03 |
| `polarization.radiation` | 0.8 | 1.0 |
| `polarization.hammer_tilt` | 0.05 | 0.06 |
| `polarization.slope` | 1.5 | 1.625 |
| `polarization.slope_to` | 532.5 Hz | 767 Hz |
| `losses.b2_top` (not kept, below) | 3.9e-7 | 2.2e-7 |
| `unison.hit_spread` | 0.3 | 0.2 |
| `unison.width_curve` | 0.8 | 0.64 |
| `soundboard.loss_corner` | 3500 Hz | 4375 Hz |
| `body.t60` at 2 kHz | 0.5 s | 0.65 s |
| `body.speed_halo` at C5 | 0.4 | 0.5 |

The search ended on `polarization.detune_cents` 0, outside the parameter's range (0.03–3): at 0.03, with the other values rounded, 665 measures are off (655 at 0).

| | Old points | New points | New points, tuned |
|---|---|---|---|
| Measures off | 856 | 866 | 665 |
| Mean gap | 1.193 | 1.203 | 1.093 |
| Bass, off | 203 | 197 | 165 |
| Middle | 361 | 359 | 265 |
| Treble | 223 | 240 | 184 |
| Top | 69 | 70 | 51 |
| Bench of the real grands (docs/bench.md) | 1.47 | 1.59 | 1.30 |

Most of the gain is not the move's: the partial lines' measures (versions 8 and 9) had never been searched, and `b2_top` back near 2.0e-7 takes most of them (Lines 1.2–2.5 kHz late 52 off to 19, 2.5–4 kHz at 0.6–1.2 s 76 to 36, at 1.2–2.2 s 80 to 46, 4–7 kHz at 0.6–1.2 s 54 to 35). The top's spectral tilt comes within range with the hammers nearer 1/12 of the string (21 off to 8). Worse: the sustain's brightness (20 off to 27), the release drop at 150–600 Hz (13 to 17), Lines 4–7 kHz at 1.2–2.2 s (28 to 33); the treble's rise time stays at 14 off.

### At the keyboard: the buzz is `b2_top`

Played by the user the same day, the tuned preset buzzed: "the tone leans towards a harpsichord's". Each of the 14 values was then scored alone on the committed preset, and taken out alone from the tuned one (C4–C6, three layers, median change of the model's values):

| Added alone | Lines 2.5–4 kHz, 0.6–1.2 s | 1.2–2.2 s | Lines 4–7 kHz, 0.6–1.2 s |
|---|---|---|---|
| `b2_top` 2.2e-7 | +9.8 dB | +16.3 dB | +21.0 dB |
| the five of `polarization` | +1.1 | +2.9 | +2.1 |
| the four of the hammer | −1.3 | −0.4 | −0.5 |
| each of the others | under 0.4 | under 0.4 | under 0.4 |
| all 14 | +9.6 | +19.3 | +22.6 |

By ear, on presets built from this: the tuned preset with `b2_top` back at 3.9e-7 no longer buzzes, and the committed preset with `b2_top` 2.2e-7 alone does. So `b2_top` stays at 3.9e-7, and the 13 other values are kept: 772 measures off (665 with it), against 866; bench of the real grands 1.51 (1.30 with it, 1.59 before the search, 1.47 with the old bridge points).

The partial lines' measures therefore reward what the ear refuses. Clean string lines held at the real grands' level at 2.5–7 kHz are the buzz; the real grands reach that level without it (docs/sustain.md). `tools/compare_search.py` no longer tries `b2_top` under 3.9e-7. The 13 values kept were heard together in that one preset, not one by one.

Before that listening, the note on the search read: it goes back on values chosen or kept by ear: `b2_top` (3.9e-7 came from the first search; 2.0e-7 was the lowest heard without a buzz), `hardness_slope` (−0.05 kept at the keyboard), `polarization.radiation` (0.8), `slope_to` (700 by ear, here 767) and `width_curve` (1.0 by ear).
