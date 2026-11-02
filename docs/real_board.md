# The soundboard as it is built

Status as of 2026-11-02. `soundboard.structure.enabled`, off by default: with it off, the renders are bit for bit those of the commit before (three keys and presets compared, the per-key profiles' path included).

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
- **Elements.** MITC4 rectangles on a 1 cm grid, with assumed shear so that they do not lock, 4 × 4 integration points each. A node along the outline is clamped, or only its displacement held (`clamped` off).
- **Solver.** Spectrum slicing, 400 Hz a slice: the count of each from the inertia of K − λ·M (LDLᵀ of the band matrix), then shift-invert Lanczos with full reorthogonalisation at its middle, until it has them. No slice can lose a mode unnoticed.
- **Cost.** 385 modes below 3 kHz in 15 s on 32 threads, the first time. 32 MB on disk (`~/.cache/unsampled-piano/boards/`, by a hash of the structure, outline and bridges), 1 s to read back. The crown does not enter the hash: it moves the frequencies afterwards.

### Into the engine

- **The drive.** A mode is driven at the key's bridge point, with no `bridge_spread` (the bridge is in the structure) and no `impedance_slope`.
- **Per key.** With `exact_below`, the modes get a force per key in a bank of 88 inputs (`BoardBank<KEYS>`). The 16 profiles of the fitted plate do not fit: they made levels jump by ±12 dB from key to key (B3 +12, D3 −12), which is the truncation and goes away. It costs nothing more when few keys sound, and a 30-note pedalled chord on one thread runs at 2.7× real time, as on the fitted plate.
- **Above the corner.** The rectangle's modes above 3 kHz stay, set on `rho_h`, `dx`, `dy`. Each key's drive is scaled so that the conductance carries on across the corner (the octave below against the octave above, smoothed over ±3 keys), and their radiation by √(ρh·A/mass), since their mass is not the board's. Level from string force to microphones then runs across 3 kHz without a jump (2.5, 3.15, 4 kHz at C4: 67.6, 63.6, 66.8 dB).
- **The rim** gets the reactions at the held nodes, F = −K_cf·φ, which balance the plate's inertia (test).

### Damping

The thesis's law, fve = 2·10⁻⁵·f² + 0.07·f, with f in Hz (III.1.1; RT-425 gives only its coefficients, and eq. I.3.19 writes it as a function of λ). The thesis says it is chosen to match η = 2 % at low frequencies, which holds if fve is the decay constant α = η·π·f of its Figure I.43, and not the coefficient of q̇, which would make it half. Read that way: η = 0.0223·(1 + f/3500). The model's fitted law was 0.02·(1 + f/2500), nearly the same; the change moves the distance to the D from 8.16 to 8.18.

### Air

`structure.air`, on with the structure. The wood's law above is its own: the thesis leaves the losses by radiation to its air, and the model had none. Each mode now radiates into the air it moves: Rayleigh's integral of the mode on itself, on both faces, the back face in antiphase through the same `back_gain` and `back_depth` as the microphones hear it,

Z = 2·(iωρ₀/2π)·∫∫Φ·(e^{−ikR}/R − β·e^{−ikR'}/R')·Φ' dS dS' = r + iω·m,

by FFT on a 2 cm grid (`fem::air_on`, 0.4 s on 32 threads). So (1 + m)·q̈ + r·q̇ + ω²·q = F: the frequencies fall by √(1 + m), every shape too, and r/((1 + m)·ω) joins the mode's loss factor. Above the corner the plate radiates as a whole: η = 2ρ₀c/(ω·ρh). The coupling between modes through the air is left out.

Checked against a baffled piston (mass 8ρa³/3, resistance ρc·A·(ka)²/2) within 3 %, and Wallace's efficiency of a plate's (1, 1) mode within 5 %. On the D's flat board:

| Mode | 1 | 2 | 3 | 4 | 11 | 41 | 121 | 201 | 351 |
|---|---|---|---|---|---|---|---|---|---|
| Hz | 33 | 52 | 78 | 89 | 210 | 538 | 1235 | 1828 | 2798 |
| Added mass | 13 % | 10 % | 10 % | 8 % | 7 % | 4 % | 3 % | 2 % | 1 % |
| η of radiation | 1.6 % | 0.2 % | 0.3 % | 0.3 % | 1.8 % | 0.5 % | 0.9 % | 1.4 % | 1.4 % |

Radiation is then a fair share of a mode's losses beside the wood's 2.2–4 %, and it varies by a factor of nine from mode to mode where the wood's law is smooth. Efficient radiators are damped more than their neighbours.

## Checks

**Against the thesis, same board.** 9 mm plate, ribs of 25 mm under it, bridge of 20 mm over it, simply supported, as it computes them (curves read off its Figure II.12; `examples/board_fem.rs thesis 0.01 6200`):

| Mode | 1 | 2 | 3 | 4 | 5 | 10 | 15 | 20 | 200 | 400 | 600 | 800 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Here | 29.8 | 44.4 | 67.6 | 69.0 | 98.4 | 160 | 230 | 295 | 1933 | 3265 | 4410 | 5486 |
| Thesis, 3D | 29 | 43 | 64 | 67 | 94 | 163 | 229 | 280 | 1750 | 2700 | 3350 | 4050 |
| Thesis, plate | 29 | 42 | 62 | 67 | 91 | 164 | 226 | 284 | 1900 | 3100 | 4000 | 4750 |

3.2 % rms over the first 20 against 3D. Within 2 % of its plate at mode 200 and 5 % at 400; the first-order elements fall behind above that (10 % at 600), which is why the corner stays at 3 kHz. The thesis's bridges and outline are not exactly these: its mesh could not be fetched.

**Against a real board.** The D's structure, clamped, flat: 33, 52, 78, 89 Hz. Conklin's real 2.74 m board, without strings or bridge: 49, 67, 89, 112 Hz. Simply supported is far lower (the thesis's 29, 43, 64, 67), so a real board is nearer clamped. The difference looks like what the crown does. With the air, the flat board is at 31, 49, 74, 85 Hz, and 40 Hz of crown gives 49, 62, 83, 94 Hz. That is the value set: it is fitted on these four modes, not measured, on a board whose bridge is missing and which has no strings on it.

**Unit tests.** A clamped square against Leissa within 0.5 %; slices that miss nothing; shapes mass-normalised; reactions balancing the inertia; the cache reading back what it wrote.

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

`loss_low_corner` and `edge_loss` stand for what the rim and the case take from the low modes, which the rigid edge cannot. Without them, the bass's envelope falls on the real grands' (at 1 s −9.9 dB against their −10.0, late T60 14.2 s against 13.5 s). But the score goes to 2.12, and the 8–11 kHz unevenness of the middle jumps (z +2.9 → +7.3), for a reason not traced. They stay until the rim gives back.

## Still not in it

- **Crown and bearing**: a foundation term set on four modes, not the shell under the strings' 200 kN.
- **Coupling**: the rim and the frame receive the board's forces and do not give back. The clamped edge is rigid.
- **Air between modes**: each mode loads only itself. The case's own volume, the lid and the room do not act back on the board.
- **Duplex lengths**, the 17th rib, the plate's real braces, the bridges' pins and notches.
- **The modes above 3 kHz**: still a statistical plate, at the structure's level. Finer elements or second-order ones would extend the corner.
- **The rib-plate glue line** and in-plane relaxation under a narrow rib: the section takes the stack as free to find its neutral plane, which softens a little what a 3D model keeps stiff.

## Tools

`examples/board_fem.rs` (modes against the thesis, conductance per key), `examples/board_tf.rs` (string force to microphones per partial and per third of an octave, built against fitted).
