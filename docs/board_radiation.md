# The board's radiation towards the player

Status as of 2026-10-28. Branch `claude/piano-render-realism-5cf127`. Examples: `direct_field`, `radiation_probe` (piano-render); parameters `soundboard.scatter`, `scatter_corner`, `scatter_step`, `scatter_height`, `scatter_shadow`, `rib_spacing`, `rib_share` (all off by default).

## A correction first

`presets/default.toml` was a dump of an older default: the rectangular board with its old calibration (ρh 7, D 3825/1700, impedance slopes 0.02/0, no outline). Every tool that took it as "the default" ran the rectangle: the stage swap (docs/stage_swap.md, its listening files and the user's verdicts), `bridge_partials`, the bridge-to-ear tables of docs/realism_axes.md (axis 3) and the first measures below. The file is now regenerated from `PianoModel::default()` (`unsampled-piano-render dump-preset`), and the tables here are the outlined board's. The picture did not change: the outlined board, heard from the player's seat, puts the same few mid partials up and buries the same highs (M − A, docs/stage_swap.md: A2 rms 12.3 dB with partials 4, 7, 9 at +10, +14, +9; A1 13.8 dB with partials 8–20 at +8 to +27); its bridge-to-ear transfer against Plath's grand reads +5 to +7 dB at 400–800 Hz, −10 at 250 Hz, −9 to −13 at 3–5 kHz, within a decibel or two of the rectangle's.

## What was heard, what the board does

The user: the model's strings through the real board sound right; the model at the player's seat sounds like a bell; the real strings through the model's board worst of all. `radiation_probe`, the mean |pressure per unit modal velocity|² of the board's modes per third octave of their frequency, lid off:

| mode frequency | 315 | 630 | 1000 | 1587 | 2000 | 3175 | 4000 | 6350 | 8000 Hz |
|---|---|---|---|---|---|---|---|---|---|
| player's seat, 95 cm before the keyboard, 45 cm up | 29 | 36 | 33 | 28 | 18 | 16 | 13 | 7 | 5 dB |
| close pair, 40 cm above the board | 36 | 35 | 39 | 42 | 42 | 44 | 43 | 44 | 43 |

Towards the player the modes above 2 kHz radiate 20–30 dB less than those at 600–1000 Hz, where the plate's coincidence lies (800–1200 Hz for D 4590/2040, ρh 8.4); above the board they radiate flat.

**It is not a defect of the modal sum.** `direct_field` computes the pressure per unit force at the player from a point force at a key's bridge point, two ways: as the engine does (each mode's Rayleigh integral at its own frequency) and exactly (every mode's integral at the driving frequency, the sum interfering as the plate's velocity field does, the direct field of the point force included). The exact one is *lower* at the player: −13 to −39 dB below the engine's from 2 to 8 kHz on A2, −13 to −21 on C4 (±10 dB with no trend at the close pair). A plate in a baffle radiates its short waves upwards and nothing along its surface; the engine's approximation already gives more at grazing incidence than the plate would.

**Ribs do not change it.** Ribs as a discrete-minus-smeared stiffness in the outlined board's Rayleigh–Ritz basis (`rib_spacing` 0.13 m, `rib_share` 0.5–0.8, the corner at 6 kHz so that the ribbed modes cover the band): the probe towards the player moves by ≤ 2 dB. Periodic ribs make Bloch waves, extended, not the localised modes a real board's irregular ribs and thickness give; and even confined to a strip, a mode would still radiate towards the normal. The parameters stay, off, for the evenness work (docs/outline.md).

So the player's highs in a real grand are not the board's direct field. They are what lies over the board and scatters it: the strings, the cast-iron frame and its struts, the dampers, the bridges, the case's edges. Plath's dummy head, through the same strings, hears them.

## The scattering layer

`soundboard.scatter` adds to the board's radiation a layer of secondary sources on a grid of `scatter_step` m (≈ 180 on the 2 m grand, inside the outline), at heights spread about `scatter_height` m, each with a phase of its own (a fixed pseudo-random sequence, the same at every build: obstacles of different shapes and heights add in power, not in phase). Each re-radiates the board's velocity under it with the efficiency of a small obstacle, (f/fc)²/(1 + (f/fc)²) × `scatter` (Rayleigh scattering below `scatter_corner`, geometric above; the scattered pressure follows the obstacle's incident pressure, so the Rayleigh prefactor's ω is taken out). `scatter_shadow` takes the intercepted share from the direct sound of a point above the board (√(1 − share·f²/(fc² + f²)) on the front face and its lid image); it changes the close pair by less than a decibel and stays at 0. The build costs 0.01 s; the real-time cost is nil (the weights are per mode).

Ear pressure per bridge acceleration at the player's seat, model minus Plath's grand, median per third octave over every key's partials (`bridge_to_ear`, `tools/tf_compare.py`; the real piano's own spread is 4–15 dB):

| | 250 | 400 | 630 | 1000 | 1260 | 2000 | 2520 | 3175 | 4000 | 5040 | 6350 | 8000 Hz | shape rms 200 Hz–4 kHz |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| no scatter (the default) | −10 | +7 | +5 | +1 | −5 | −3 | −3 | −9 | −13 | −12 | −7 | −3 | 6.2 dB |
| scatter 1, corner 1.5 kHz | −9 | +7 | +6 | +1 | −4 | −1 | 0 | −5 | −8 | −8 | −1 | +2 | 5.1 |
| scatter 2, corner 1.5 kHz | −8 | +6 | +6 | +0 | −2 | +3 | +4 | +1 | −5 | −3 | +3 | +5 | 4.3 |
| scatter 3, corner 1.5 kHz | −8 | +7 | +7 | +1 | −1 | +6 | +7 | +4 | −2 | 0 | +6 | +9 | 4.2 |

The 2–8 kHz region comes within the tolerance at scatter 2 (−5 to +5 dB); the +5 to +7 dB at 400–800 Hz and the hole at 250 Hz are the board's own coincidence band and its lowest modes, untouched by the layer, and the user's A1 bell lives there (M − A at 440–1100 Hz unchanged at scatter 2, A2's 1.3–2.2 kHz deficit −10 to −17 → 0 to −5).

What it costs where the bench listens (the close pair above the board), `cargo xtask bench`:

| | baseline | scatter 1 | scatter 2 | scatter 2, corner 2.5 kHz | scatter 3 |
|---|---|---|---|---|---|
| score | 1.11 | **1.10** | 1.18 | 1.22 | 1.26 |

At 1 the bench does not move (11 off instead of 12). At 2 and above the top octave's balance (its attack re the middle's, already +1.2 z) runs to +2.5/+4.4 and the middle's 2 kHz sustain bands up: the layer adds 3–5 dB to the close pair above 2 kHz, where the real pianos' close recordings do not have it. The scatter at the close pair is likely too strong for what the model takes from the direct sound: a layer that scatters should shade what passes through it, and `scatter_shadow` as written does too little (the layer's incoherent sum outweighs the direct even shaded by 7 dB). An energy-consistent normalisation of the layer (scattered power bounded by the power intercepted) is the next step of the model; the bench and the ear decide between 1 and 2 meanwhile.

## For the ear

`tools/stage_swap.py` rendered again on the true default (`out/ab_swap_outline/`), with scatter 1 (`out/ab_swap_s1/`) and scatter 2 (`out/ab_swap_s2/`): the model M and the real strings through the model's board B, against the real piano 0, same loudness, player's seat, no lid. A listener at the keyboard, or the `player` and `concert` listening presets, are where the layer matters; the close pair hears mostly the direct field.

## Still open

- The board's coincidence band at the player: +5 to +7 dB at 400–800 Hz from every bridge point, and the first modes' hole at 250 Hz. Both are the plate's own radiation at grazing incidence; the real board's ribs, bridges and thickness spread its coincidence, and the frame shades the keyboard end.
- The layer's normalisation (above), and its law above 5 kHz, where Plath's grand falls off 14 dB from 5 to 8 kHz and the layer stays flat.
- `crates/piano-core/tests/realtime.rs` does not compile on this machine (`assert_no_alloc::AllocDisabler` is behind a feature of assert_no_alloc 1.1.2); unrelated to this work, left as found.
