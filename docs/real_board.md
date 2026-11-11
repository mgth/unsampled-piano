# The soundboard as it is built

Status as of 2026-11-11 (its own modes up to 6 kHz since that day: [Above 3 kHz](#above-3-khz)). `soundboard.structure.enabled`, off by default: with it off, the renders are bit for bit those of the commit before (three keys and presets compared, the per-key profiles' path included).

## Why

The model's board was a plate of uniform surface mass and stiffness, fitted: `rho_h` 14.7 kg/m² against about 5.4 for the real thing, axes along a rectangle, and a slope on the drive standing in for what the real structure does from bass to treble. The real board has a thin spruce plate, ribs across its grain and a heavy bridge, and these are not a small correction: spread over their spacing, the ribs carry about 50 times the plate's bending stiffness across the grain.

## Sources

- J. Chabassier's thesis (2012, https://pastel.hal.science/pastel-00690351/file/These.pdf), §I.3 and §II.3: the IRCAM Steinway D as a Reissner–Mindlin plate of varying thickness; measured ribs ("17 ribs, 2 to 2.6 cm wide from the treble to the bass, 1.9 to 2.6 cm thick, closer towards the treble"); zones and angles (Figure I.45); its modes against a 3D elastodynamic computation of the same board (Figure II.12); Conklin's Chladni figures of a real 2.74 m board (Figure I.42).
- Chabassier & Duruflé, RT-425 (https://hal.inria.fr/hal-00688679v2/file/RT-425.pdf), §2: both spruces and the beech; Figure 2, the maps of thickness in metres.

**Read off RT-425's Figure 2** by inverting its colour bars (axes in metres; its bridge passes within 0.5 cm of the table's bridge points):

| | Value |
|---|---|
| Ribs | 16 in the mesh (the thesis counted 17 on the piano; the 17th is not placed), at 35.5° to the RT-425 frame's x axis, 12–15 cm apart, closer towards the treble |
| Rib thickness, plate included | 15–34 mm at the middle, 13–25 mm at the ends, parabolic between |
| Plate | 9 mm away from the rim, about 7 mm at it, over about 10 cm (the figure's scale goes to 6) |
| Main bridge, plate included | 55 mm at F2, 70 mm near F3, 64 mm at C#4, 42 mm at A5, 29 mm at C8 |
| Bass bridge, plate included | 52 to 60 mm |

Bridge width is the least sure: 3 cm, from the figure's 2.5 cm and what the thesis's photo shows.

The thesis's angles (grain −40°, ribs 54.4°) are from the frame's y axis: the ribs' 54.4° then gives the 35.6° read on the figure, and the grain falls square to the ribs. In the board's axes the grain is at 40° from its length.

## The model

`crates/piano-core/src/fem.rs`.

- **Section.** Every point is a stack: plate; rib under it, fibres along the rib; bridge over it, fibres along its curve. It reduces to a bending stiffness about its own neutral plane (D* = D − B·A⁻¹·B), a transverse shear stiffness, a surface mass and a rotary inertia. The thesis instead gives a whole thickness one material, and notes that this overstiffens the bridge.
- **Plate.** Reissner–Mindlin, as the thesis's. Shear is not optional here: across the grain spruce's transverse shear modulus is 0.039 GPa, 300 times below its modulus along the grain, and at 3 kHz a cross-grain bending wave on 9 mm moves as fast as its shear wave. A thin plate (Hermite elements, tried first) had half the modes at 6 kHz.
- **Elements.** MITC4 rectangles on a 1 cm grid, with assumed shear so that they do not lock, 4 × 4 integration points each. Their mass matrix is 1.5 × the lumped one − 0.5 × the consistent one, which keeps their waves at the plate's own frequencies up to 8 kHz and more ([Above 3 kHz](#above-3-khz)). A node along the outline is clamped, or only its displacement held (`clamped` off).
- **Solver.** Spectrum slicing, 400 Hz a slice, halved until none holds more than 64 modes: the count of each from the inertia of K − λ·M (LDLᵀ of the band matrix), then shift-invert Lanczos with full reorthogonalisation at its middle, until it has them. No slice can lose a mode unnoticed. As many slices side by side as fit in 6 GB.
- **Shapes.** Each is normalised on the lumped mass, Σ m·φ² = 1 over the nodes, and given a sign of its own (upwards at the first node that moves by half its largest movement): the solver leaves the sign to chance, and the case's field, which gives each mode a phase, changed with it.
- **Cost.** In the preset, 1104 modes below 6 kHz in 41 s on 16 threads, the first time, within 4.9 GB. 49 MB on disk (`~/.cache/unsampled-piano/boards/`, by a hash of the structure, outline and bridges; shapes in 16 bits), the air's 18 kB beside it, 0.9 s to start again. Up to 3 kHz: 409 modes, 17 s, 18 MB. The crown does not enter the hash: it moves the frequencies afterwards.

### Into the engine

- **The drive.** A mode is driven at the key's bridge point, with no `bridge_spread` (the bridge is in the structure) and no `impedance_slope`.
- **Per key.** With `exact_below`, the modes get a force per key in a bank of 88 inputs (`BoardBank<KEYS>`). The 16 profiles of the fitted plate do not fit: they made levels jump by ±12 dB from key to key (B3 +12, D3 −12), which is the truncation and goes away. It costs nothing more when few keys sound. Thirty keys with the pedal pumping, on one thread: 3.6× real time with the built modes up to 3 kHz, as on the fitted plate, 3.3× up to 6 kHz.
- **Radiation.** A built mode radiates from its own nodes when the radiation grid (`grid_step`, 1.5 cm) is coarser than the elements: its waves across the grain are 4.3 cm long at 6 kHz. Below 3 kHz the two differ by 0.1 dB.
- **Above the corner.** The rectangle's modes above the structure's last (`structure.below`, 6 kHz in the preset, 3 kHz until 2026-11-11) stay, set on `rho_h`, `dx`, `dy`. Each key's drive is scaled so that the conductance carries on across the corner (the octave below against the octave above, smoothed over ±3 keys), and their radiation by √(ρh·A/mass), since their mass is not the board's.
- **The rim** gets the reactions at the held nodes, F = −K_cf·φ, which balance the plate's inertia (test).

### Damping

The thesis's law, fve = 2·10⁻⁵·f² + 0.07·f, with f in Hz (III.1.1; RT-425 gives only its coefficients, and eq. I.3.19 writes it as a function of λ). The thesis says it is chosen to match η = 2 % at low frequencies, which holds if fve is the decay constant α = η·π·f of its Figure I.43, and not the coefficient of q̇, which would make it half. Read that way: η = 0.0223·(1 + f/3500). The model's fitted law was 0.02·(1 + f/2500), nearly the same; the change moves the distance to the D from 8.16 to 8.18.

### Air

`structure.air`, on with the structure. The wood's law above is its own: the thesis leaves the losses by radiation to its air, and the model had none. Each mode now radiates into the air it moves: Rayleigh's integral of the mode on itself, on both faces, the back face in antiphase through the same `back_gain` and `back_depth` as the microphones hear it,

Z = 2·(iωρ₀/2π)·∫∫Φ·(e^{−ikR}/R − β·e^{−ikR'}/R')·Φ' dS dS' = r + iω·m,

by FFT on a 2 cm grid below 3 kHz and on the elements' nodes above (`fem::air_on`; 4 s on 16 threads for 1104 modes, kept on disk beside the modes). So (1 + m)·q̈ + r·q̇ + ω²·q = F: the frequencies fall by √(1 + m), every shape too, and r/((1 + m)·ω) joins the mode's loss factor. Above the corner the plate radiates as a whole: η = 2ρ₀c/(ω·ρh). The coupling between modes through the air is left out.

Checked against a baffled piston (mass 8ρa³/3, resistance ρc·A·(ka)²/2) within 3 %, and Wallace's efficiency of a plate's (1, 1) mode within 5 %. On the D's flat board:

| Mode | 1 | 2 | 3 | 4 | 11 | 41 | 121 | 201 | 351 |
|---|---|---|---|---|---|---|---|---|---|
| Hz | 33 | 52 | 78 | 89 | 210 | 538 | 1235 | 1828 | 2798 |
| Added mass | 13 % | 10 % | 10 % | 8 % | 7 % | 4 % | 3 % | 2 % | 1 % |
| η of radiation | 1.6 % | 0.2 % | 0.3 % | 0.3 % | 1.8 % | 0.5 % | 0.9 % | 1.4 % | 1.4 % |

Radiation is then a fair share of a mode's losses beside the wood's 2.2–4 %, and it varies by a factor of nine from mode to mode where the wood's law is smooth. Efficient radiators are damped more than their neighbours.

## Checks

**Against the thesis, same board.** 9 mm plate, ribs of 25 mm under it, bridge of 20 mm over it, simply supported, as it computes them (curves read off its Figure II.12; `examples/board_fem.rs thesis 0.01 6200`):

| Mode | 1 | 2 | 3 | 4 | 5 | 10 | 15 | 20 | 200 | 400 | 600 | 800 | 1000 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Here | 29.8 | 44.4 | 67.5 | 69.0 | 98.3 | 160 | 229 | 294 | 1889 | 3124 | 4125 | 5022 | 5845 |
| Here, until 2026-11-11 | 29.8 | 44.4 | 67.6 | 69.0 | 98.4 | 160 | 230 | 295 | 1933 | 3265 | 4410 | 5486 | |
| Thesis, 3D | 29 | 43 | 64 | 67 | 94 | 163 | 229 | 280 | 1750 | 2700 | 3350 | 4050 | 4750 |
| Thesis, plate | 29 | 42 | 62 | 67 | 91 | 164 | 226 | 284 | 1900 | 3100 | 4000 | 4750 | 5500 |

3.1 % rms over the first 20 against 3D. Within 1 % of its plate at modes 200 and 400, 3 % at 600, 6 % at 800 and 1000; with the consistent mass used until 2026-11-11 it was 5 % at 400, 10 % at 600 and 15 % at 800, which had kept the corner at 3 kHz. What is left is not the elements': against a 5 mm grid of the same board they are within 1.5 % up to 8.8 kHz ([Above 3 kHz](#above-3-khz)). The thesis's bridges and outline are not exactly these (its mesh could not be fetched), and its curve is read off a figure.

**Against a real board.** The D's structure, clamped, flat: 33, 52, 78, 89 Hz. Conklin's real 2.74 m board, without strings or bridge: 49, 67, 89, 112 Hz. Simply supported is far lower (the thesis's 29, 43, 64, 67), so a real board is nearer clamped. The difference looks like what the crown does. With the air, the flat board is at 31, 49, 74, 85 Hz, and 40 Hz of crown gives 49, 62, 83, 94 Hz. That is the value set: it is fitted on these four modes, not measured, on a board whose bridge is missing and which has no strings on it.

**Unit tests.** A clamped square against Leissa within 0.5 %; a grid of five elements to the wavelength against a fine one; slices that miss nothing; shapes mass-normalised, their sign the same however far the spectrum is solved; reactions balancing the inertia; the cache reading back what it wrote; a low mode radiating the same from its nodes and from the grid's cells.

Mass: 11.2 kg on 2.09 m² (5.4 kg/m²). Mean conductance at the bridge: −33 to −36 dB re 1 s/kg between 800 Hz and 8 kHz in the middle.

## Against the commercial D and the real grands

| | Overall | Bass | Middle | Treble | Top | Bench | Raw level (D −20.9 dB) |
|---|---|---|---|---|---|---|---|
| Last commit, fitted plate | 7.20 | 5.83 | 5.81 | 8.92 | 7.76 | 1.83 | −29.9 |
| Built board, regions, crown 55 | 8.17 | 7.39 | 5.79 | 11.76 | 6.35 | | −25.7 |
| Built board, per key, crown 36, damping | 8.31 | 8.20 | 6.16 | 10.80 | 7.40 | 1.99 | −25.4 |

| + air, crown 40: the preset | 8.85 | 7.86 | 7.29 | 12.15 | 7.14 | 2.02 | −26.7 |
| same, without `loss_low_corner` and `edge_loss` | 8.98 | 8.10 | 8.04 | 11.89 | 7.14 | 2.12 | −26.2 |

The bench's 1.83 is the fitted plate with the last damper at G6. That step alone took it from 1.60: with no sympathetic ringing left from D#6–G6, the top's 1 kHz band in the sustain falls from −42 to −110 dB, against −43 on the real grands.

The structure is further from the recorded D, as expected with its fitted terms gone. What it gets right and wrong:

- **Level:** 4.5 dB closer in raw level, with no gain touched.
- **Bass decay:** A0–B1 lose 7.7–8.1 dB between 0.2 and 1 s, the D 3.2–4.4. This is the main distance to the D, but the real grands lie between the two: the bass's envelope at 1 s is −11.3 dB here, −7.7 on the fitted plate, −10 on the real grands (z −0.4 against +0.7), its early T60 5.8 s, 11.2 s and 8.3 s. The physics for a wound string, σ = (T/L)·Re Y at −30 dB, gives 7.6 dB/s.
- **Better on the bench:** the treble's 4 kHz band, partial 4 (z −1.7 → +0.2), the middle's partial 6, its high bands' unevenness (z +5.1 → +2.1), key-to-key evenness.
- **Worse:** the treble's fundamentals, C6–B6 by 6.7 dB against the middle, its balance z −1.1 → −3.2. Their absolute level hardly changes; the middle rose. Part of it is physical and unresolved: below the coincidence frequency, about 2.5 kHz on a plate this light, the board radiates poorly, and level from force to microphones jumps by up to 10 dB between 2 and 2.5 kHz at some keys. The real pianos don't show that hole; what fills it (strings, frame, the plate's openings: `soundboard.scatter` is off) is not settled.

What no longer acts in this preset: `impedance_slope_bass`, `rib_spacing`, `rib_share`, `rib_jitter`, and `rho_h`, `dx`, `dy` below 3 kHz. They still reached the iron frame's own plate, which inherits the board's settings: taking them out of the preset changes the sound a little (distance 8.18 → 8.31, all of it in the top register's mf sustain, not traced further).

**The air** moves the D's distance from 8.31 to 8.85 and the bench from 1.99 to 2.02: nearly neutral on the real grands, with gains and losses that balance. Better: the noise between the bass's partials (`between` z −2.2 → −1.1), the treble's balance (z −3.2 → −2.7), the middle's 4 kHz. Worse: the middle's envelope at 1 s (z 0.0 → +0.8), the attack's upper partials at the top (`attack_mf` p2 z +5.7 → +6.4). The bass's decay hardly moves: 13.5 dB between 0.2 and 2 s against 13.8, the D's 6.7.

`loss_low_corner` and `edge_loss` stand for what the rim and the case take from the low modes, which the rigid edge cannot. Without them, the bass's envelope falls on the real grands' (at 1 s −9.9 dB against their −10.0, late T60 14.2 s against 13.5 s). But the score goes to 2.12, and the 8–11 kHz unevenness of the middle jumps (z +2.9 → +7.3). That jump is one key: F#3's 8–11 kHz share of its first 60 ms goes from −96 to −75 dB, a level nobody hears, and the measure's largest gap takes it.

### What the rim and the case could take

Neither the rim nor the case moving on its legs accounts for these two losses (2026-11-11).

- **The rim, two ways.** The board's edge forces on each mode of the ring (`RimModel`) give each board mode the pole it has with the rim attached, Craig–Bampton's coupling through L = F/ω²: s² + 2σs + ω² − Σ L²s⁴/(s² + 2σ_r·s + ω_r²) = 0. Of the 227 modes below 2 kHz, two change: 127 Hz, which falls on the ring's 129 Hz mode (11.8 s⁻¹ added to its 10.5), and 890 Hz (12.7 s⁻¹ added to its 102). The rest gain 0.06 s⁻¹ (median), and frequencies move by under 0.3 Hz. The first mode's loss is 5.1 s⁻¹ from the wood and the air, and `loss_low_corner` 200 Hz makes it 63. With 40 orders instead of 8 the answer is the same. The ring's modes are few and lightly damped (η 2 %), and the board drives them off their resonances. The edge's moments, which the elements' reactions leave out, and the 160–180 kg of iron bolted on the rim are not counted; the second would make the rim move less still. The coupling was not put into the engine.
- **The case on its legs.** The board's net force, Σ F = ω²·∫ρhΦ, on 480 kg on a support tuned anywhere from 10 to 50 Hz with η 0.3–0.5, adds at most 1.6 s⁻¹ to the first mode, under 0.1 to the others. The `piston_*` mode (6 kg, 40 Hz, η 0.7) is a fit, not that case. No published stiffness or frequency of a grand's support was found.
- **Measured low modes.** Corradi et al. (Applied Acoustics 2017, a Fazioli F278's board glued in its case, unstrung): modal damping 0.8–4 % of critical up to 348 Hz, so η 1.6–8 %, not given mode by mode. Ege, Boutillon and Rébillat (JSV 2013, an upright, strings muted): η 1–3 % except the first four resonances, about 4.7 % at 81 Hz and 4–5 % at 115–160 Hz, where they put part of it in the rim. Suzuki's first mode (1986, as plotted there): about 6 %. Here, with the wood and the air: 3.3 % at 49 Hz, 2.6 % at 83 Hz. With `loss_low_corner` 200 Hz: 41 % at 49 Hz, 15 % at 83 Hz, 7 % at 146 Hz. A corner near 80 Hz gives values of the measured order: 9.3 % at 49 Hz, 4.7 % at 83 Hz and 3.7 % at 146 Hz.

| | distance | bass | middle | treble | top | bench | bass drop 0.2–2 s, A0–B0 / C1 / C2 |
|---|---|---|---|---|---|---|---|
| the case's field (`loss_low_corner` 200 Hz, `edge_loss` 1e-3) | 9.00 | 8.48 | 7.62 | 12.40 | 6.36 | 1.89 | 13.6 / 13.0 / 18.0 dB |
| `edge_loss` 0 | 8.98 | 8.46 | 7.86 | 12.21 | 6.34 | 1.88 | 13.0 / 12.6 / 17.2 dB |
| **`edge_loss` 0, `loss_low_corner` 80 Hz: the preset** | 9.08 | 8.60 | 8.39 | 12.05 | 6.33 | 1.92 | 13.0 / 12.6 / 16.7 dB |
| neither | 9.11 | 8.65 | 8.51 | 12.01 | 6.33 | 1.93 | 12.9 / 12.6 / 16.5 dB |
| neither, no piston mode | 9.29 | 9.05 | 9.02 | 11.91 | 6.31 | 1.94 | 12.7 / 12.3 / 16.3 dB |
| the commercial D | | | | | | | 6.7 / 9.1 / 14.8 dB |

`edge_loss` does nothing a measure sees. Without `loss_low_corner`, the bass's partials 2–3 at the attack fall further under the real grands' (z −1.8 → −2.5) and the middle moves away from the D. And the bass's fast decay is not theirs: without either, A0–B0 still lose 12.9 dB between 0.2 and 2 s, the D 6.7.

The preset takes the measured boards' corner, 80 Hz, and no edge loss: 0.08 further from the D and 0.03 on the bench, where the fitted 200 Hz had no measurement behind it. Heard key by key against the preset before, the difference was not audible.

**The case's field** answers what the air left open: C6–B6 at −11.9 dB against A2–A5, the D at −2.8. Only the fundamentals were short, by 12 dB, partials 2 and up being right. Two causes add up. From the keys' bridge points, the direct field from force to microphones has a hole at 1–1.7 kHz (46–58 dB, against 61–72 at 2–3 kHz), below the plate's coincidence, whereas the board's total radiation there is healthy: η_rad 0.9–1.4 %, a radiation efficiency of 0.5–1. And these fundamentals lose 65 dB/s in their first 300 ms, 3·(T/L)·Re Y with Re Y ≈ 6e-4 to 1e-3 s/kg. The real grands' treble does as much (early T60 0.8 s, ours 0.9), so that part stays.

So the sound is radiated, but not straight at a microphone 40 cm up: a plate near its coincidence sends it along its surface. There it meets the rim, the frame and the lid. `case_field` treats the box of the case and the lid as a small room, from each mode's own radiation loss:

- **Power:** half of what a mode radiates goes up, P = (air·ω)·v²/4 for a peak modal velocity v.
- **Inside the box:** p² = 4ρc·P/R in rms, R = A/(1 − A/S) its room constant. A is the openings plus 10 % of its other surfaces. With the lid at 35° the gap along the rim is 2.1 m², A 2.65 m², R 4.15 m². Outside the box, what leaves spreads over a half-sphere from the board's centre.
- **Below the Schroeder frequency,** 2000·√(0.161/A) = 490 Hz here, the box's modes stand apart: the power is scaled by f²/(f² + f_S²).
- **The microphone's pattern:** a cardioid hears a third of a diffuse field's energy.
- **Phases:** each mode at each microphone has its own fixed phase, the field adding in power.

It adds no damping: that energy is already the mode's radiation loss. It takes no fitted value, the 10 % hardly counting beside the openings. It has no build-up either: the box's T60 is about 80 ms, and the field arrives with the direct sound.

| | distance | bass | middle | treble | top | bench | raw | C6–B6 | C7–B7 |
|---|---|---|---|---|---|---|---|---|---|
| no field | 8.85 | 7.86 | 7.29 | 12.15 | 7.14 | 2.02 | −26.7 | −11.9 | −18.5 |
| `scatter` 2 (for comparison) | 8.67 | 8.23 | 6.81 | 12.38 | 5.78 | 1.88 | −25.9 | −6.4 | −12.8 |
| field at all frequencies | 9.08 | 8.53 | 7.68 | 12.52 | 6.45 | 1.93 | −24.2 | −6.6 | −17.5 |
| **field above Schroeder** (the preset until 2026-11-11) | 9.00 | 8.48 | 7.62 | 12.40 | 6.36 | 1.89 | −25.4 | −5.6 | −16.7 |
| the commercial D | | | | | | | −20.9 | −2.8 | −10.6 |

The fundamentals of C6–F6 now start 1.2 dB under the D's (9.8 dB before). On the bench: the treble's balance (z −2.7 → −0.7), the top's attack partials 2–4 and its 2 kHz band get closer. Worse: the noise between the bass's partials (`between` late z −1.1 → −2.3) and the middle's 8–11 kHz unevenness (z +2.9 → +3.9). The distance to the D rises by 0.15, most of it in the bass and the middle's attack, though their mean bands hardly move, not traced further.

`scatter` 2 scores about the same with a free constant. Its layer radiates more power than the board can (an efficiency near 5 above its corner) and takes it from nowhere; at 1, with a bench of 1.96, it is already at 1.2. The two are not to be added: the field stands for what the layer did.

## The bass against a real concert grand

Status as of 2026-11-11. The commercial D's bass holds twice as long as the preset's (A0–B0 lose 6.7 dB between 0.2 and 2 s, the preset 13.0). Before closing that gap, what a real one does: Plath's concert grand (a new Steinway, anechoic, every key; docs/steinway_d.md), its dummy head and the accelerometer on its bridge. Drop of the sound between 0.2 and 2 s, then between 0.2 and 1 s, velocity 100 for the two models (dB):

| | A0–B0 | C1–B1 | C2–B2 | C3–B3 |
|---|---|---|---|---|
| the commercial D | 6.7 / 3.2 | 9.1 / 4.4 | 14.8 / 7.1 | 23.6 / 12.0 |
| the real grand, at the ears | 11.5 / 6.1 | 9.5 / 5.6 | 14.2 / 8.2 | 18.7 / 10.7 |
| the preset | 13.0 / 7.1 | 12.6 / 7.3 | 16.7 / 9.2 | 17.7 / 9.0 |
| strings on the bridge, no piston mode | 12.8 / 7.2 | 12.6 / 7.5 | 16.1 / 9.1 | 19.3 / 9.7 |
| the same, measured polarisation | 13.9 / 7.3 | 13.0 / 7.5 | 17.1 / 9.2 | 20.2 / 10.6 |

The real grand's three lowest keys fall as the preset's, not as the commercial D's: no real measure here says that A0–B0 should hold as long as that D. From C1 to B2 the real grand follows the commercial D and the preset falls 2–3 dB too fast; at C3–B3 the preset holds 1 dB longer than the real grand and 6 dB longer than the D.

**Partial by partial.** Decay rate of each partial over 0.15–1 s and over 1–2.5 s (dB/s, median over the keys and the partials; the real grand at its bridge):

| A0–E1, one string | partial 1 | 2–4 | 5–8 | 9–16 |
|---|---|---|---|---|
| the commercial D | 7.3 / 6.3 | 4.2 / 4.1 | 3.0 / 2.7 | 2.7 / 2.5 |
| the real grand | 0.7 / 0.5 | 4.9 / 4.4 | 4.1 / 3.5 | 4.6 / 4.6 |
| the preset | 6.5 / 10.8 | 11.5 / 4.0 | 7.3 / 4.6 | 4.4 / 4.8 |
| strings on the bridge, no piston mode | 0.9 / 2.2 | 10.9 / 3.1 | 7.2 / 4.4 | 4.4 / 4.4 |
| the same, measured polarisation | 0.4 / 0.6 | 10.1 / 7.9 | 7.1 / 5.7 | 4.4 / 4.7 |

A real single string decays at one rate, 4–5 dB/s from its second partial up, and its fundamental hardly at all. The preset's partials 2–8 fall two to three times faster at first, then slow down; its fundamental falls ten times too fast.

**A real bridge's conductance, from its strings.** A string alone on its key loses 8.69·(T/L)·Re Y dB/s to the bridge, so the decay of the real grand's eight single strings, partial by partial, bounds its bass bridge's conductance from above at 128 frequencies (`examples/bass_loss.rs` gives the model's side: the string's own loss, the board's conductance at the partial, the modes that carry it, the engine's coupled modes). Medians per band, dB/s:

| Hz | 25–45 | 45–70 | 70–100 | 100–140 | 140–200 | 200–280 | 280–400 | 400–700 |
|---|---|---|---|---|---|---|---|---|
| the real grand's single strings | 0.7 | 3.1 | 5.4 | 5.7 | 3.1 | 4.8 | 4.6 | 4.4 |
| the preset: the board alone | 4.6 | 19.6 | 18.6 | 7.7 | 3.0 | 5.0 | 3.1 | 3.3 |
| the preset: the engine's fast mode | 6.9 | 28.7 | 27.4 | 11.7 | 4.5 | 7.0 | 4.5 | 5.0 |
| strings on the bridge, no piston mode: the board alone | 0.3 | 4.1 | 20.6 | 9.7 | 2.7 | 5.4 | 3.4 | 2.8 |
| the same, measured polarisation: the fast mode | 0.6 | 4.5 | 21.7 | 10.5 | 3.4 | 6.3 | 4.5 | 4.6 |

Above 140 Hz the built board's conductance is the real bridge's, with nothing fitted. Three things stood between it and the real decay:

- **The board had no strings.** Its lowest modes were set on Conklin's board, measured without strings or bridge: 49, 62, 83, 94, 112, 127, 146 Hz. Under each key's fundamental the real bridge's acceleration holds no partial, and what rings there after the blow is the board (`plath_modes.py` in the session's tools): 66–72 Hz, weakly, then 109, 114, 118, 137–140, 149, 170, 177, 190, 207 Hz, and nothing clear under 60 Hz. The real single strings lose most at 92–103 Hz (12, 25 and 10 dB/s) and at 137–138 Hz (39 and 22 dB/s). With the strings' hold ([The strings on the bridge](#the-strings-on-the-bridge)): 68, 77, 95, 111, 128, 135, 156, 170, 183, 201, 209 Hz.
- **The piston mode** (`piston_freq` 40 Hz, 6 kg, loss 0.7, a fit standing for the case on its legs, which can add 1.6 s⁻¹ at most): it carried 50–80 % of the conductance at the fundamentals of A0–E1, 3–7 dB/s where the real ones lose 0.7.
- **The sideways polarisation.** `polarization` gives the bass a sideways coupling of 0.6 times the vertical one and a cross term (0.3 at C4, growing as 1/f down to C3, set for the middle's fundamentals by ear): a single string's fast mode then loses 1.47 times what the board takes, and a slow one stands 5 dB under it. The real single strings show neither: one rate, early and late. The law measured on a real grand's partials before that fit (0.03, cross at its passive limit, no slope) gives one rate again, the board's.

What is left: at 70–140 Hz the model's bass bridge still conducts two to four times too much (modes at 77 and 95 Hz, broad under the resting strings' load, η 0.13–0.17, where the real bridge shows one narrower resonance near 97 Hz), which is where partials 2–4 of A0–E1 lie.

### The strings on the bridge

`structure.strings`, off by default, on in the preset. A string under tension T crosses the bridge between its speaking length L and its rear length l, which ends at the hitch pin. Moving the bridge by w across them turns both parts, and the string pulls back with T·w·(1/L + 1/l): a spring under each key's bridge point, added to the stiffness the modes are computed with (it enters the cache's hash). `PianoModel::board_params` fills each key's hold from the scale: 8 kN/m at A0, 25 kN/m at C4, 60 kN/m at C7, 0.25 MN/m along the bass bridge and 2.6 MN/m along the main one, against some 0.3 MN/m of modal stiffness for the bare board's first mode. The rear lengths do most of it, being five to ten times shorter than the speaking lengths.

`structure.back_length` is an estimate, not a measurement: 4 cm plus a tenth of the speaking length, 24 cm at A0, 10.5 cm at C4, 4.5 cm at C8. No table of the D's rear lengths was found; one owner's photograph puts the lowest string's two ends 24 cm beyond its speaking length, tuning-pin side included. The static spring is right for a mode below the string's own fundamental, and for the rear length up to its own resonance, some kHz. A mode above the string's fundamental sees its waves instead, which `string_load` stands for; the speaking length's share of the spring, a tenth, is then wrong, and left.

Halving these lengths puts the first modes at 73, 87, 102 and 117 Hz, doubling them at 62, 72, 90 and 107 Hz, against 68, 77, 95 and 111 Hz as set and 49, 62, 83 and 94 Hz without strings: the real grand's first resonance, 66–72 Hz, lies within what the estimate allows and far from the bare board's.

It takes the place of nothing fitted, and `crown_freq` stays where Conklin's unstrung board put it.

| | distance to the D | bass | middle | treble | top | bench | bench, bass | bass T60, partials 1–4, early (real grands 8.3 s) |
|---|---|---|---|---|---|---|---|---|
| the preset until then | 9.08 | 8.60 | 8.39 | 12.05 | 6.33 | 1.92 | 1.41 | 6.0 s |
| + `structure.strings`, `piston_freq` 0 | 7.22 | 6.91 | 6.46 | 8.78 | 6.47 | 1.95 | 1.34 | 7.0 s |
| **+ the measured polarisation (0.03, cross 0.173, no slope): the preset** | 7.32 | 7.72 | 6.59 | 8.35 | 6.47 | 1.97 | 1.43 | 8.1 s |

With the strings and without the piston mode the built board comes back to the fitted plate's distance to the D (7.20) with nothing fitted in it, most of it in the treble and the middle, whose keys the strings hold hardest: their bridge moves less and their partials at the attack and in the sustain come 2.5–3 dB closer. The bench hardly moves: its sustain (1.58 → 1.37) and decay families (1.40 → 1.30) and its bass get closer, its attack (1.80 → 1.97) and its top register (2.79 → 2.95) further. The bass's broadband decay does not change, since its fundamentals carry little of the sound: what changes there is each partial's own decay.

The preset takes all three (2026-11-11), chosen after hearing both variants against it and the D (`out/ab_grave_cordes`): the strings, no piston mode, the measured polarisation.

## Above 3 kHz

Status as of 2026-11-11. The built board's own modes run up to 6 kHz in the preset, 1104 of them (409 below 3 kHz); the statistical rectangle keeps what is above. The elements were the obstacle and no longer are. On the measures, the result is neutral: neither the distance to the commercial D nor the bench moves by more than what a redraw of the same board moves them.

### Where the elements stopped being right

Two measures, neither needing the thesis's mesh.

**A wave on an endless grid.** A plane wave on a uniform 9 mm spruce plate, meshed with these elements, has a frequency that one node's 3 × 3 problem gives exactly (Bloch; `tools/mitc4_dispersion.py`). Against the plate's own, error in frequency at the true frequency, mean over the directions and, in brackets, the worst one, across the grain:

| 1 cm grid | 1 kHz | 2 kHz | 3 kHz | 4 kHz | 6 kHz | 8 kHz | 10 kHz |
|---|---|---|---|---|---|---|---|
| consistent mass, as until now | +1.1 (+2.2) % | +2.5 (+5.6) | +4.3 (+10.5) | +6.4 (+17) | +11.5 (+35) | +17.7 (+60) | +24.8 (+92) |
| lumped mass | +0.2 (+0.8) | +0.7 (+2.3) | +1.3 (+4.3) | +2.0 (+7.2) | +3.5 (+14) | +4.8 (+21) | +5.6 (+26) |
| 1.5 × lumped − 0.5 × consistent | −0.1 (−0.5) | −0.1 (+0.9) | 0.0 (+1.9) | +0.1 (+3.2) | +0.3 (+6.9) | +0.3 (+10) | −0.3 (+12) |
| wavelength along / across the grain, cm | 29.5 / 14.6 | 20.5 / 9.6 | 16.5 / 7.3 | 14.1 / 5.9 | 11.2 / 4.3 | 9.4 / 3.4 | 8.2 / 2.7 |

The elements are too stiff, and the consistent mass adds to it. With that mass a grid of 5 mm still errs by +2.9 % at 6 kHz and +4.4 % at 8 kHz: halving the step only gives 8 kHz what 1 cm had at 3 kHz.

**The D's board against a 5 mm grid.** The structure itself, clamped, flat, without strings: frequency of the n-th mode on each grid, re the 5 mm grid's (`board_fem d 0.005 10000 converge 22`):

| Mode | 50 | 100 | 200 | 300 | 400 | 600 | 800 | 1000 | 1200 | 1500 | 2000 | modes below 10 kHz |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Hz, 5 mm grid | 612 | 1031 | 1763 | 2374 | 2914 | 3871 | 4723 | 5526 | 6250 | 7265 | 8761 | 2458 |
| 1 cm, consistent mass (until now) | +1.9 % | +2.5 | +3.5 | +4.7 | +5.5 | +7.9 | +10.5 | +12.5 | +15.1 | +18.0 | | 1820 |
| 1 cm, lumped | +1.4 | +1.7 | +2.0 | +2.4 | +2.6 | +3.5 | +4.2 | +4.9 | +5.5 | +6.2 | +7.5 | 2187 |
| **1 cm, 1.5 × lumped − 0.5 × consistent** | +1.2 | +1.3 | +1.1 | +1.3 | +1.2 | +1.4 | +1.5 | +1.4 | +1.5 | +1.4 | +1.4 | 2415 |
| 2 cm, the same mass | +3.4 | +3.1 | +2.5 | +2.7 | +2.7 | +2.8 | +2.8 | +2.2 | +1.6 | +0.4 | −2.2 | 2914 |
| 2 cm, consistent | +6.4 | +8.4 | +11.7 | +15.9 | +19.7 | +27.9 | +35.7 | +42.2 | +48.9 | | | 1297 |

So the elements as they were had the board's modes 5.5 % too high at 2.9 kHz already, and a quarter of them missing below 10 kHz. With the new mass the error no longer grows with frequency: 1.2–1.5 % from 600 Hz to 8.8 kHz, presumably what a staircase of 1 cm makes of the outline, the ribs and the bridge (2.5–3.4 % at 2 cm).

### What the options cost

| | accuracy at 8 kHz | a factorisation | memory | cache per mode |
|---|---|---|---|---|
| 5 mm grid, consistent mass | +4.4 % (Bloch) | × 16 | × 8: 1.8 GB a slice | × 4 |
| second-order elements on the same nodes (not written) | about a third of the first-order error at equal nodes, by the textbook's estimate | × 4 (twice the band) | × 2 | × 1 |
| **the mass matrix** | within 0.3 % (Bloch), +1.4 % on the board | × 1 | × 1 | × 1 |

The 5 mm reference above, up to 10 kHz, took 16 minutes on 7 threads within 22 GB. The mass matrix is the one kept. It is a remedy for these elements on this wood, not a better element: across its grain spruce is so soft in shear that the plate's short waves are shear waves, and the blend is set on them. On an isotropic plate it overshoots: at five elements to the wavelength the modes come out up to 3 % low where the consistent mass has them up to 12 % high and the lumped one up to 5 % high.

Two things follow from that mass. The solver's shapes are orthonormal for it, not for the plate's mass: left so, the nodes would move 0.3 dB too little at 3 kHz and 1 dB at 8 kHz across the grain, hence the lumped normalisation. And a mode's sign: a board solved up to 6 kHz gave some of its low modes the other sign than the same board solved up to 3 kHz, and the case's field, which adds to each mode a pressure with a phase of its own, then redrew every partial above 300 Hz, by 3–7 dB rms, between two builds of the same board. A mode's sign is now its own.

### How far

| corner | built modes | modes in the bank | first build, 16 threads | peak memory | on disk | starting again | 30 keys, pedal pumping, one thread | whole keyboard ff |
|---|---|---|---|---|---|---|---|---|
| before, 3 kHz | 385 | 1958 | 15 s | | 32 MB | 1.1 s | 27.8 % of a core, 3.6× real time | 39.7 % |
| 3 kHz | 409 | 1982 | 17 s | 3.0 GB | 18 MB | 0.8 s | 28.0 %, 3.6× | 39.9 % |
| 5 kHz | | | 32 s | 4.9 GB | 37 MB | 0.85 s | | |
| **6 kHz** | 1104 | 2403 | 41 s | 4.9 GB | 49 MB | 0.9 s | 30.6 %, 3.3× | 43.9 % |
| 8 kHz | 1700 | 2815 | 64 s | 5.7 GB | 75 MB | 1.1 s | 33.7 %, 3.0× | 47.4 % |
| 10 kHz | 2415 | 3346 | 90 s | 6.3 GB | 106 MB | 1.2 s | 36.4 %, 2.7× | 51.6 % |

48 kHz, buffers of 128 frames, `perf_audit`, runs alternated on a machine at rest. The first build is paid once per structure, then read from the disk. Without the air kept on disk, starting again would take 5 s at 6 kHz and 8 s at 8 kHz.

How far the model can go, 8 kHz, and where the preset stops, 6 kHz:

- **The grid.** At 8 kHz the board's shortest waves, across the grain between the ribs, are 3.4 cm long: 3.4 nodes. At 10 kHz, 2.7 cm. The count of modes still holds there (2415 against the 5 mm grid's 2458), their shapes no longer. 8 kHz is as far as this grid goes.
- **The plate.** The thesis's 3D computation and its plate are already 15–19 % apart in frequency from 3 to 6 kHz (same figure, above), and a wave of 3.4 cm is hardly longer than a rib is wide. More accuracy in solving the plate buys nothing beyond.
- **The cost.** 6 kHz is a tenth more of a core than 3 kHz, 8 kHz a fifth, 10 kHz a third, with a 106 MB file and a minute and a half to build.
- **The sound.** Nothing measured separates 6, 8 and 10 kHz (below), and by ear neither: 8 kHz was the preset first, and heard key by key against the board before, the new elements at 3 kHz and the commercial D (`out/ab_table_8khz`, 2026-11-11), no difference was heard. The preset takes 6 kHz, which measures the same for half the added cost.

### Against the commercial D and the real grands

Same close microphones, no room, each set under its own level, all 88 keys at velocities 60 and 100; the bench as in docs/bench.md. One change at a time: the elements at the old corner, then the corner.

| | D, 88 keys | bass | middle | treble | top | D, the 15 keys | bench | bass | middle | treble | top |
|---|---|---|---|---|---|---|---|---|---|---|---|
| before | 9.29 | 8.63 | 10.09 | 8.51 | 9.83 | 7.32 | 1.97 | 1.43 | 1.68 | 1.68 | 2.94 |
| the new elements, corner at 3 kHz | 9.48 | 8.68 | 9.78 | 9.48 | 9.92 | 8.94 | 2.00 | 1.49 | 1.66 | 1.82 | 2.91 |
| **corner at 6 kHz: the preset** | 9.46 | 8.70 | 9.86 | 9.69 | 9.54 | 9.04 | 1.98 | 1.50 | 1.70 | 1.83 | 2.81 |
| corner at 8 kHz | 9.49 | 8.70 | 9.86 | 9.98 | 9.35 | 9.05 | 1.99 | 1.51 | 1.76 | 1.88 | 2.77 |
| corner at 10 kHz | 9.61 | 8.70 | 9.88 | 9.99 | 9.80 | 9.25 | | | | | |

**What these figures can tell.** Two things were found on the way, and both limit what can be read here.

- *The 15 keys.* The distance of docs/steinway_d.md takes every sixth key from A0, the keys the preset was matched on. On the five other sets of every sixth key the board before scores 9.11, 10.32, 10.63, 9.24 and 8.32, not 7.32; at 8 kHz the new board scores 9.05 on that set and 8.31, 10.93, 10.28, 8.86 and 9.13 on the others. The 7.32 was those 15 keys' own; on all 88 the step is 9.29 → 9.46.
- *A redraw.* Moving the board's modes by a few percent, which is what the new elements do below 3 kHz, moves each partial of each note by 6–8 dB rms in the middle and the treble, at the attack as in the sustain: force to microphones is a sum of many modes, and its fine structure does not survive such a shift. Two builds of the new elements at 3 kHz that differed only by the signs above scored 9.52 and 9.48, the treble 9.04 and 9.48, the top 8.62 and 9.92. A step under about 0.5 in a register, 1.3 at the top, says nothing about the board.

Read with that:

- **Overall: neutral.** 9.29 → 9.46 against the D, 1.97 → 1.98 on the bench.
- **The treble is further from the D**, 8.51 → 9.69 (9.98 at 8 kHz), more than a redraw: 1.0 with the new elements, 0.2 more with the corner at 6 kHz and 0.5 at 8 kHz (the same 0.5 in both draws). Its partials 2–8 stand 4.5 dB over the D's at the attack, 3.0 before, and its level 3.7 dB under the keyboard's median, 2.3 before. On the bench its register goes from 1.68 to 1.83: partial 4 of the attack (z +1.0 → +2.3 at ff) and the balance of its fundamentals (z −1.2 → −3.2, there from the new elements on).
- **The top is nearer**, 9.83 → 9.54 against the D (within a redraw) and 2.94 → 2.81 on the bench: partials 2 and 4 of its attack (z +5.9 → +5.3, +2.5 → +1.6), its 4 kHz band (z +2.9 → +1.5), partial 2 of its sustain (z −2.4 → −0.4).
- **The top octave's level** comes 2–3 dB nearer the D's, in both draws: C7–C8 at −14.7 dB re A2–A5 (−15.7 in the other draw) against −17.9 before, the D −10.6. It comes with the new elements, not with the corner.

Level of the fundamental, re each set's A2–A5 (dB, velocity 100, first 0.3 s):

| | C5–B5 | C6–B6 | C7–D7 | D#7–F#7 | G7–C8 |
|---|---|---|---|---|---|
| the commercial D | 3.4 | −2.7 | −9.4 | −13.0 | −16.7 |
| before | −0.4 | −8.0 | −23.5 | −21.5 | −25.5 |
| the new elements, corner at 3 kHz | −2.1 | −8.6 | −16.5 | −18.9 | −22.2 |
| corner at 6 kHz | −2.1 | −8.8 | −14.9 | −20.2 | −21.0 |
| corner at 8 kHz | −2.2 | −8.7 | −14.5 | −20.4 | −20.9 |

Raw level: −25.9 dB against −24.9 before, the D −20.9.

### What it did not change

- **The step near 2–2.5 kHz.** Force to microphones, direct field, mean over the keys, the structure without the strings' hold (dB; `board_tf`):

| | 1.6 kHz | 2 kHz | 2.5 kHz | 3.15 kHz | 4 kHz | 5 kHz | 6.3 kHz | 8 kHz |
|---|---|---|---|---|---|---|---|---|
| C6–B6, the fitted plate | 61.3 | 63.4 | 65.9 | 64.4 | 64.8 | 65.2 | 65.9 | 64.4 |
| C6–B6, before | 61.9 | 64.3 | 68.2 | 71.1 | 69.5 | 70.4 | 71.6 | 69.6 |
| C6–B6, corner at 6 kHz | 61.3 | 64.5 | 67.9 | 65.7 | 67.1 | 68.8 | 70.8 | 71.0 |
| C6–B6, corner at 8 kHz | 61.5 | 64.6 | 68.0 | 66.0 | 68.1 | 69.9 | 72.5 | 72.5 |
| C7–C8, before | 64.8 | 63.5 | 67.4 | 68.4 | 70.9 | 70.7 | 73.4 | 73.7 |
| C7–C8, corner at 6 kHz | 65.2 | 64.0 | 67.8 | 67.5 | 69.3 | 69.8 | 72.0 | 72.8 |
| C7–C8, corner at 8 kHz | 65.0 | 63.8 | 67.6 | 67.4 | 69.2 | 70.6 | 71.9 | 73.8 |

  The 4 dB between 2 and 2.5 kHz are as they were: they lie below the old corner, in modes that were the structure's already, and correcting these by 4–5 % does not fill them. What the built modes change is above: 5 dB less at 3.15 kHz for C6–B6 than the rectangle matched at the corner gave, 1–2 dB less elsewhere at 3–4 kHz.
- **The fundamentals of C6–B6**, 6 dB under the D's as before (−8.8 against −2.7): 1–2 kHz, below the old corner too.
- **The real strike point at the top.** With `strike_ratio_top` 0, RT-425's, against the default's 0.05, level of D#7–F#7 against the D's under one gain: −7.6 against −1.4 before, −4.7 against +1.2 with the new elements at 3 kHz, −5.6 against +0.1 with the corner at 8 kHz (not rendered at 6 kHz). The real point costs 6 dB with the built board as with the statistical plate: the plate was not the cause. Their second partial then stands 12 dB over the D's with the real point, 5.5 with the default's.

## Still not in it

- **Crown and bearing**: a foundation term set on four modes, not the shell under the strings' 200 kN.
- **Coupling**: the rim and the frame receive the board's forces and do not give back. The clamped edge is rigid. Made two-way, the ring would change two modes in 227 ([What the rim and the case could take](#what-the-rim-and-the-case-could-take)).
- **Air between modes**: each mode loads only itself. The case's own volume, the lid and the room do not act back on the board. The case's field is statistical: no individual reflection, no strut, the frame's real shape being unknown to the sources used here.
- **The strings' rear lengths** as measured (an estimate holds the bridge), their duplex resonances, the 17th rib, the plate's real braces, the bridges' pins and notches.
- **The modes above 6 kHz**: still a statistical plate, at the structure's level; the elements would carry them to 8 kHz, at a cost nothing heard or measured repays ([Above 3 kHz](#above-3-khz)).
- **A plate, not a solid**: the thesis's 3D computation of the same board has its 400th mode at 2.7 kHz where its plate, and this one, have it at 3.1 kHz, and its 1000th at 4.75 kHz against 5.5. The ribs, 2–2.6 cm wide and up to 3.4 cm deep, and the bridge, 3 cm wide and up to 7 cm deep, are layers of a plate here.
- **The rib-plate glue line** and in-plane relaxation under a narrow rib: the section takes the stack as free to find its neutral plane, which softens a little what a 3D model keeps stiff.

## Tools

`examples/board_fem.rs` (modes against the thesis, conductance per key; `board_fem d 0.005 10000 converge 22` for the elements against a 5 mm grid within 22 GB), `examples/board_tf.rs` (string force to microphones per partial and per third of an octave, built against fitted, with the corner as its second argument), `piano-render`'s `examples/build_parts.rs` (a preset's build time, part by part), `tools/mitc4_dispersion.py` (the elements' waves against the plate's, per grid step and mass matrix).
