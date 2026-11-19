# Tuning against the comparison tool

State as of 2026-11-19.

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

A checkpoint of `tools/compare_search.py` now records the measure version, the band and the starting preset. If any of them differs, the search starts again from a freshly scored baseline.
