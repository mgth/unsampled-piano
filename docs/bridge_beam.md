# The bridges as beams

Status as of 2026-10-29. `soundboard.bridge_beam`, 0 (off) by default.

## Why

The bridges are hardwood beams glued on the board, about 25 × 30 mm (`bridge_stiffness` EI 470 N·m², `bridge_line_mass` 0.6 kg/m). Until now the model used these two values only to spread a string's force along the bridge (`bridge_spread`); the board's modes did not have the bridges in them. On a real board the bridge:

- makes neighbouring bridge points move together;
- adds its own impedance to the plate's at the strings' feet, so that the mobility there falls above 1–2 kHz (Giordano 1998, Ege et al. 2013).

[ribs.md](ribs.md) found the model's conductance flat at −31 dB re 1 s/kg from 1 to 8 kHz.

## The model

**Below `outline_below`.** The bridges' stiffness EI·∫Φ_i,ss·Φ_j,ss ds and mass μ·∫Φ_i·Φ_j ds are added along the main and the bass bridge's curves to the outlined board's Rayleigh–Ritz problem. Here s is the arc length and Φ_ss the second derivative along the tangent; the integrals use the midpoint rule on at least 8 points per wavelength of the basis. With the mass, the problem becomes generalised, K·v = λ·M·v: `eigen::generalized` (Cholesky, then the symmetric problem, then v = L⁻ᵀ·y, with vᵀ·M·v = 1).

**Above it.** The rectangle's modes do not have the beam, and the modes overlap there. The board is treated as an infinite plate under an infinite beam, whose impedances add:

Z = 8·√(D·ρh) + 2(1 + i)·(EI·μ³)^¼·√ω

Each of those modes' drive shapes is scaled by the root of its conductance against the plate's alone (`Soundboard::beam_drive`).

At 1 kHz the own modes with the beam give −34.0 dB, the formula −33.5: the two agree.

The board takes 0.15 s to build with the beam, against 0.02 s without.

## Measured

**Driving-point conductance** over every key's partials (`rib_mobility`, median, dB re 1 s/kg):

| | 315 Hz | 1000 | 2000 | 4000 | 8000 |
|---|---|---|---|---|---|
| no beam | −29.9 | −31.5 | −31.3 | −31.3 | −31.5 |
| beam, own modes only | −32.6 | −34.0 | −31.3 | −31.3 | −31.5 |
| beam, with the formula above the corner | −32.6 | −34.0 | −33.7 | −34.6 | −35.7 |

**Bridge to the player's ear** against Plath's grand (spread per third octave, real piano first): at 400 Hz 8.7 against 10.3 (no beam) and 8.2 (beam); at 1 kHz 6.9 against 11.3 and 9.2; at 500 Hz 4.4 against 7.4 and 9.4.

**Bench:**

| | score | off | attack | brightness | sustain | decay |
|---|---|---|---|---|---|---|
| no beam (default) | 1.11 | 12 | 1.19 | 1.40 | 1.45 | 0.81 |
| beam, own modes only | 1.19 | 16 | 1.31 | 1.48 | 1.38 | 1.01 |
| beam, with the formula | 1.24 | 21 | 1.19 | 1.55 | 1.27 | 1.28 |
| the same, plate × 0.6 (ρh, D: same modal density, mobility +2.2 dB) | 1.19 | 16 | 1.18 | 1.52 | 1.29 | 1.14 |
| the same, plate × 0.5 | 1.21 | 15 | 1.18 | 1.54 | 1.49 | 1.11 |

With the beam in the own modes only, the treble's fundamentals (0.5–2 kHz) lost conductance but not their upper partials above the corner: the ratios of the treble's attack partials jumped (+1 z). With the formula above the corner, those ratios come right: the treble's attack partial 6 goes from +1.46 to −0.02, and the ff balance of the treble and the top is mended. But the falling conductance slows the decay of the upper partials: treble and top partials 5–12, z +0.4/+0.9 → +2.2/+3.5. A more mobile plate brings the middle and the bass back, not the highs.

## The recalibration tried

Eleven bench runs with the beam on, compensating with the plate's mobility (ρh, D × s, the same modal density), the strings' high-frequency loss `losses.b2`, the top's extra loss `losses.top_slope`, the drive's spread `bridge_spread` (the beam's other model) and the beam at half strength:

| variant | score | off | sustain | decay | brightness |
|---|---|---|---|---|---|
| no beam (default) | **1.11** | 12 | 1.45 | 0.81 | 1.40 |
| s 0.75 | 1.22 | 20 | 1.26 | 1.23 | 1.54 |
| s 0.75, b2 8e-8 | 1.28 | 18 | 1.77 | 1.15 | 1.56 |
| s 0.75, b2 1e-7 | 1.27 | 19 | 1.74 | 1.08 | 1.65 |
| s 0.6, b2 8e-8 | 1.24 | 14 | 1.76 | 1.06 | 1.54 |
| s 0.6, b2 1e-7 | 1.25 | 15 | 1.74 | 1.02 | 1.63 |
| s 0.6, b2 1.3e-7 | 1.45 | 18 | 2.61 | 0.96 | 1.84 |
| s 0.75, spread 0 | 1.22 | 19 | 1.34 | 1.21 | 1.51 |
| beam 0.5, s 0.85 | 1.20 | 16 | 1.52 | 0.94 | 1.53 |
| beam 0.5, s 0.85, spread 0 | 1.19 | 16 | 1.56 | 0.93 | 1.50 |
| s 0.75, top_slope 0.02 | 1.23 | 20 | 1.20 | 1.25 | 1.57 |
| s 0.6, top_slope 0.02 | 1.20 | 17 | 1.27 | 1.16 | 1.55 |

None comes back to 1.11. The same two faults remain:

- **The treble's and the top's upper partials decay too slowly** (T60 of partials 5–12, z +2.4 / +3.9): the beam takes 3–4 dB of conductance off at 4–8 kHz.
- **The middle's pp and mf envelopes at 1–2 s fall too fast** (z −1.1 to −1.5), as soon as the plate is made more mobile to give the conductance back below 1 kHz.

The strings' own high-frequency loss mends the first and spoils the sustain's partial ratios; the top's extra loss changes little.

The real grands' upper partials decay as if their bridges' conductance at 4–8 kHz did not fall as an infinite beam on an infinite plate makes it fall. A tapered bridge (smaller towards the treble), the bridge's own resonances on a finite length, or the ribs' local modes may keep it up; the model has none of these.

## What it needs

The board's other settings were calibrated without the beam: its mobility level, the treble's impedance slopes, the board's loss law (`loss_corner`), and the strings' own high-frequency loss (b2), set when the board's conductance at 4–8 kHz was 3–4 dB higher. With the beam, the real pianos' fast high partials have to come from the strings' own loss instead. Kept off: the recalibration above did not find a set that scores as well as the board without it.
