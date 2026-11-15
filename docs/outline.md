# The soundboard's outline

Status as of 2026-10-28: **on by default** since that day (see the end).

## Why

The board was an orthotropic plate clamped on an a × b rectangle, with analytic modes (Warburton). A grand's board is not a rectangle: it follows the bentside and narrows towards the tail. docs/evenness.md ended on "a more realistic board (ribs, an irregular outline, two bridges)" as the next step against the key-to-key unevenness of partials 1–2 in the sustain. The outline is the first of these. It also lets the interface draw the board the model actually has.

## The model

`soundboard.outline`: a polygon of 10 points in normalised plate coordinates (as the bridges), inside the a × b rectangle. Unused points repeat the last one. A polygon without area, as the default's ten zeros, is the full rectangle.

`soundboard.outline_below` (Hz, 1500): below it the board has the outlined plate's own modes; above, the rectangle's.

- **Own modes** (`Soundboard::new`). Rayleigh–Ritz on the rectangle's modes below 4 × `outline_below` (306 modes for the 2 m grand), mass-normalised on the rectangle. A stiff elastic ground holds the board outside the outline: it adds ω_g²·ρh·∫_outside Φ_i·Φ_j to the rectangle's diagonal ω_i², with ω_g twice the basis's highest frequency. The integral runs over a grid of ≥ 8 points per wavelength of the basis, each cell weighted by its share outside the outline (4 × 4 samples). The lowest eigenvectors are the board inside the outline, clamped along it. The crown's foundation stays in the diagonal.
- **Above the corner** the rectangle's modes remain. They keep their drive at the bridge and radiate from the cells inside the outline only.
- **Every user of the shapes** goes through `Soundboard::shape`: the bridge's drive (`bridge_shapes`, the regions, `LowDrive`), the damped strings' load, the driving-point mobility. The radiation grid weights each cell by its share inside the outline. The edge loss takes the distance to the outline. The rim receives the own modes' edge forces in two ways. Along the rectangle's edges that the outline shares, it gets their basis modes' reactions. Where the outline leaves the rectangle, it gets the ground's reaction κ·Φ, placed on the rim's ring by its direction from the board's centre.
- **Solver.** `eigen::symmetric_ql`: Householder reduction and implicit QL (EISPACK tred2/tql2), one O(n³) pass. The cyclic Jacobi of `eigen::symmetric` would need several sweeps.

### Checks

- **No outline**: the A0–C8 sweep with and without pedal renders bit for bit as before (same hash as the untouched build). `an_outline_without_area_is_the_rectangle` checks frequencies, drive shapes, radiation and edge loss bit for bit for a polygon without area.
- **A shorter rectangle as the outline** (75 % of the length) gets the analytic modes of a board that short within 1.0 % rms below 1.5 kHz (`a_shorter_outline_has_the_shorter_boards_modes`). The basis and the ground were chosen this way:

| basis × corner | ground 0.5 | 1 | 2 | 4 |
|---|---|---|---|---|
| 2 | 5.7 % | 3.1 % | 1.6 % | 1.0 % |
| 3 | 4.0 % | 2.2 % | 1.1 % | 1.2 % |
| 4 | 3.3 % | 1.9 % | 1.0 % | 1.0 % |
| 5 | 2.8 % | 1.6 % | 1.0 % | 1.1 % |

  A softer ground lets the modes leak out of the outline; a much stiffer one makes the truncated basis lock. On the grand's outline, the twelve lowest modes move by < 1 % from a basis of 4× to 8×.
- **The rim**: the own modes' edge forces balance their inertia, Σ F = ω²·∫ρh·Φ, within 3 % of its bound (`outlined_edge_forces_balance_the_plate`). The ground alone gave 280 % off: the shared edges carry most of it.
- **The grand's board**: every bridge point lies inside the outline, ≥ 5 cm from its edge. The own modes keep < 1 % of their energy outside (cells more than 5 cm from the edge). The mean conductance at C4 stays near the infinite-plate value, as on the rectangle (`a_grand_outline_holds_its_board`).

### Cost

- Build: 0.010–0.025 s for the board's modes. The whole instrument goes from 0.15 to 0.19 s.
- Real time: the outlined board has fewer modes below the corner (53 instead of 67 below 1.5 kHz, area 82 %): 1070 modes in all instead of 1084.

`examples/outline_modes.rs` prints these checks, the conductance by register, the lowest modes and the build times.

## The prototype outline

`SoundboardParams::GRAND_OUTLINE`, for the 2 m board: straight front edge (x = 0) and spine (the bass side, y = b). The bentside leaves the treble cheek 20 cm behind the front edge and curves in to 40 % of the width at the tail, which keeps the bass side's 60 %. That is 82 % of the rectangle. The bass bridge's end nearest it (E1) lies 15 cm inside it; A0's end, furthest back and nearest the spine, 24 cm from the tail. (The measures below use that bass bridge, with 8 keys. Since 2026-10-30 it carries 24 keys, placed as on a Steinway D: its G#2 end lies 42 cm inside the rim, A0's 23 cm from the tail; docs/bridges.md.)

What changes on the board:

- The lowest mode rises from 109 to 122 Hz (smaller board, same crown).
- The low modes gather on the bass side. At A2/B2's bridge points the first mode's drive goes from 0.35 to 0.47 and the second's from 0.26 to 0.51 (mass-normalised Φ). Its conductance there is ≈ 5 dB higher.
- Bridge conductance, outline against rectangle (smoothed ±1/6 octave, damped strings' load), partials 1–4:

| | p1 | p2 | p3 | p4 |
|---|---|---|---|---|
| bass A0–B2 | +0.0 | −0.1 | −0.7 | −1.2 dB |
| middle C3–B4 | +0.8 | −0.0 | −0.3 | −0.0 dB |
| treble C5–B6 | −2.0 | −0.4 | +0.0 | +0.0 dB |
| top C7–C8 | 0 | 0 | 0 | 0 dB |

## Results

### Bench

`cargo xtask bench`, the current model (the saved baseline is older: 1.04):

| | score | off | evenness p1 / p2 (middle) |
|---|---|---|---|
| real grands | | | 6.4 / 4.8 dB (4.6–8.4 / 3.0–6.1) |
| rectangle | 1.07 | 7 | 7.7 / 6.5 dB |
| outline, corner 1.5 kHz | 1.18 | 12 | 6.1 / 5.5 dB |
| outline, corner 3 kHz | 1.18 | 13 | 6.0 / 5.6 dB |
| outline, corner 700 Hz | 1.26 | 17 | 8.7 / 7.2 dB |
| outline, treble impedance slope 0.02 → 0.01 | 1.12 | 13 | 6.0 / 5.4 dB |
| outline, slope 0 | 1.13 | 14 | 6.1 / 5.4 dB |

The corner must lie above the partials measured: at 700 Hz, the middle's partials 1–2 straddle the jump from the outlined board's modal density to the rectangle's.

With the outline (corner 1.5 kHz), besides the evenness:
- **Better**: the middle's attack and mf highs (partials 4–6, 2–8 kHz: z +1.0–1.7 → +0.1–0.7), the treble's partial 2 in the sustain (z +1.8 → +0.4), the bass's envelopes at 1–3 s (z −0.7 → 0.0).
- **Worse**: the middle decays faster, 2.5–4 dB lower at 0.3–1 s at every dynamic (z −0.6 → −1.6). Its partial 1 conductance is +0.8 dB, so +20 % on the decay rate. The treble decays slower, +5 dB at 0.3 s (z −0.3 → +1.5), from its −2 dB conductance. The middle's 1–4 kHz octave bands fall by 4–5 dB (z −0.9 → −2.4 at 1 kHz in the sustain), and the bass's ff balance by 1.6 dB.
- The treble's impedance slope (calibrated on the rectangle) at 0.01 gives back most of the treble (1.12). The middle's faster decay remains: a global mobility lever would also move the bass and the treble.

### Every key, not every third

The bench renders every third key. The partials 1–2 at 0.5–1.5 s (ff, close microphones), on every key from C2 to C7, about the median of ±6 keys:

| | p1, all keys | p1, the bench's keys | p2, all keys | p2, the bench's keys |
|---|---|---|---|---|
| rectangle | 8.6 dB | 8.8 dB | 6.5 dB | 6.2 dB |
| outline | 7.9 dB | 7.6 dB | 6.8 dB | 6.3 dB |
| outline, slope 0.01 | 7.8 dB | 7.6 dB | 6.7 dB | 6.2 dB |

(The bench's own measure files the middle and differs in detail; this table compares the boards on the same footing.) Partial 1 is about 0.7 dB more even; partial 2 not. The outline moves the board's peaks and dips to other keys more than it smooths them. Key by key from C2 to D4, the levels change by up to ±20 dB, in both directions.

### Attack jumps while playing

Chromatic scale E3–D5, velocity 64, attack level (0–100 ms) from key to key:

| | rms | largest | above 5 dB | B3 → C4 |
|---|---|---|---|---|
| close microphones, rectangle | 2.6 dB | 7.6 dB (E4 → F4) | 1 | −1.5 dB |
| close microphones, outline | 3.4 dB | 7.5 dB (E4 → F4) | 3 | −3.4 dB |
| pianist's ears (`player.toml`), rectangle | 3.7 dB | 8.2 dB (E4 → F4) | 4 | −1.4 dB |
| pianist's ears, outline | 3.3 dB | 6.3 dB (D4 → D#4) | 3 | +2.8 dB |

The E4 → F4 jump is the bridge regions' (docs/evenness.md), which the outline does not touch.

### A boom on B2 with the pedal

Chopin op. 10 no. 3 (Liang's MIDI, close microphones, studio): with the outline the first minute is 8 dB louder (−19.3 against −27.4 LUFS; 3 dB over the whole piece). Almost all of it is the board's first mode at 122 Hz, right on B2 (123.5 Hz), the dominant of E major: +16 to +22 dB at 120–125 Hz, +14 dB at 246 Hz. With the pedal down the dampers lift and the strings' load leaves the board: its low modes keep only η ≈ 0.02, and B2 feeds the first mode ≈ 5 dB more than on the rectangle. Without the pedal the same coupling drains B2 instead: its fundamental at 0.5–1.5 s lies 13 dB below its neighbours'. On the rectangle the first mode (109 Hz) sits on A2, which this piece hardly plays.

The model's lowest board mode, lightly damped once the pedal is down, makes one note boom wherever it falls. The outline did not create this, but it made that mode couple more strongly. The bench (pedal up, every third key) does not see it.

## Conclusion

- An outline in the physics is feasible and cheap: 0.04 s more to build, slightly fewer modes, nothing changed when it is off.
- The prototype grand outline does not settle the key-to-key unevenness. On every key, partial 1 gains 0.7 dB of evenness and partial 2 nothing; the bench's larger gain (7.7 → 6.1 dB) owes much to the keys it samples. It also unbalances calibrations made on the rectangle (the middle's decay, the treble's) and puts a boom on B2 with the pedal.
- The outline stayed off in the default until 2026-10-28 (below).

## Next

- **The lowest modes with the pedal down**: fixed by keeping the free strings' load on the board (docs/pedal_drone.md). With it, B2's drone with the outline falls from +7 to +2 dB at 125 Hz.
- **Ribs and two bridges**, with the same Rayleigh–Ritz machinery. Ribs as stiffness strips (Dx along them) added to the stiffness matrix; the bridges as a beam's stiffness and mass along their line. Ribs break the board's symmetry in a way an outline does not. The randomly mixed shapes tried earlier (docs/evenness.md) did not have this structure.
- **The bench's evenness on every key**, or every key around the middle: every third key gave a misleading 1.6 dB gain here.
- With the outline on, recalibrate what was set on the rectangle (crown, impedance slopes, edge loss) on the bench, then A/B.

## Limits

- Above `outline_below` the rectangle's modes remain: their mass, and so their mobility, includes the part outside the outline (18 % for the grand). The modal density jumps at the corner.
- The rim stays the rectangle's ring (its perimeter, radiation and modes); only the forces it receives follow the outline.
- The lid follows the outline, 7.5 cm wider over the rim, since 2026-10-31; at the listening presets' microphones it sounds as the rectangle did ([lid.md](lid.md#its-shape)).
- The frame follows the case since 2026-10-28, with its own modes up to 3 kHz ([frame_rim.md](frame_rim.md#the-outline)); it had inherited the board's polygon on its own rectangle by accident.
- Uprights keep the rectangle.

## 2026-10-28: on by default

What settled it was not the evenness but the stereo attack of the bass ([bass_metal.md](bass_metal.md), "The attack's coherence, band by band"). With the rectangle, the modes at 260–335 Hz reach the two close microphones in antiphase: a bass attack arrives at them 3.7 ms apart with no correlation between the channels (real pairs 0.2–1.7 ms, +0.2 to +0.5), which the user heard as part of the bass's "tube". With the outline and the same microphones the attack is coherent in every band from 250 Hz to 8 kHz. The user chose the outline with the board recalibrated: `rho_h`, `dx`, `dy` × 1.2 (the modes above `outline_below` keep a fifth of their mass outside the outline), `impedance_slope` 0.0024 and `impedance_slope_bass` 0.0076 instead of 0.02 and 0. Bench 1.11 against the rectangle's 1.09 (17 measures worse, 19 better); the new baseline. Still to do: ribs and the two bridges; the sustain's 315 Hz still reaches the pair in antiphase (−0.84).

## 2026-11-15: the Concert grand's outline against the D's drawing

Item 12b of [steinway_d_gaps.md](steinway_d_gaps.md): placed on the preset's outline, the D's drawn ribs came out 10 to 15 cm too long, which read as a board "some 8 cm too wide at the bentside". It is not an error of the bentside. The preset's outline is the edge of the panel, where it meets the outer rim; the drawing's ribs stop at the inner rim's face, which is where the board stops being free to move. The two lines are 3 to 5 cm apart all round the board.

### The drawing and its scale

The dimensioned drawing "STEINWAY D (2,74 m)" of Boutillon, Ege & Paulello (Acoustics 2012, arXiv 1210.3948, figure 2), an image of 1794 × 2484 pixels. It has two closed lines: an outer one, and an inner one at which every rib ends. Its dimensions are the ribs' lengths, widths, heights and clear gaps.

- **Scale.** The ribs' long edges, found on the image across their direction, against the drawing's own figures: 0.8888 mm a pixel over the 1.85 m from the third rib to the seventeenth, each of the fourteen gaps and fifteen widths giving 0.86 to 0.91.
- **With that scale** the outer line is 1.455 m wide and 2.10 m long, the inner one 1.37 m and 2.05 m. The margin between them, measured on the drawing: 3.8 cm along the spine, 3.7 at the tail, 4.4 to 4.7 along the bentside, 4.9 at the treble cheek, 2.8 along the belly rail.
- **The case.** A D is 157 cm wide: 5.75 cm of outer rim on each side of the outer line, 9.8 cm of rim in all with the margin. The preset's 1.46 m with its rim of 8.3 cm (`rim.thickness`, counted outwards from the outline, as the Model page draws it) makes a case 1.63 m wide; on the inner line it is 1.54 m.
- **Plath's Figure 4.1**, a factory-style drawing put to scale by his own table of fifteen bridge points (cm from the case's outer corner; the figure's dots fit the table within 13 mm rms): its outline is 1.455 m wide at the treble cheek and 2.12 m long. It is the panel's edge too.
- **Not checked**: the keyboard's width, which the drawing does not show, and the D's length, which no line of it spans.

### The preset's outline is the panel's edge

`soundboard.outline` was traced off RT-425's figure 1, ten points. Fitted on it by a rotation and a shift, the scale held, the drawing's outer line lies within 3 mm of it (median; 13 mm rms, the rounded corners being cut by straight sides). The inner line does not fit it (41 mm rms). With the outer line laid on the preset's outline, the preset's edge lies outside the drawing's inner line by:

| | treble cheek | bentside, front | bentside, curve | bentside, long part | tail | spine | belly rail |
|---|---|---|---|---|---|---|---|
| cm | 5.4 | 3.5 | 5.5 to 6.2 | 4.6 | 4.5 | 3.4 | 2.9 |

Area 2.09 m² against 1.85 m² inside the inner line (the published 2.07 m² is the panel's). A rib at 50° to the spine crosses both margins: 10 to 15 cm of length, the "8 cm" of the first reading.

Everything else in the model already takes the outline for the rim's inner face: the built board is held along it (docs/real_board.md), the rim's wall and the lid stand outside it, and the Model page draws the inner rim under a board that goes on over it. So the line to draw is the inner one.

### Bridges, strings' points, ribs

- **Bridge points.** RT-425's 84 points (the model's bridges) against Plath's table, key for key on the thirteen keys they share, by a rotation and a shift: 31 mm rms, 9 to 30 mm along the long bridge, 52 and 74 mm at two keys of the bass bridge, along it. With them RT-425's frame has its origin 5.4 cm inside the case's outer face at the spine (5.75 expected for the panel's edge). The two tables agree; nothing to move.
- **The drawn bridge.** The drawing's own bridge line follows both tables from the bass up to about C5, then runs up to 6 cm nearer the belly rail and ends in the board's treble corner. Plath's figure and both tables keep the top of the bridge further back. The drawing's bridge is a sketch there; its line was not used.
- **The ribs.** Measured on the drawing, they lie at 51.2° to the spine (50° in `tools/steinway_d_ribs.py`, from Plath's "40° to the keyboard"; ± 0.5° for the drawing's place in the model's frame). On the preset's outline the tool had put them 4.3 to 6.0 cm from where the drawing has them, across their direction, since it places the seventeenth rib by its length in the tail's corner. On the inner line and at 51.2°:

| rib | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 | 13 | 14 | 15 | 16 | 17 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| drawn length, mm | 241 | 370 | 488 | 553 | 655 | 776 | 936 | 1123 | 1130 | 1106 | 1080 | 1054 | 1029 | 1002 | 956 | 802 | 577 |
| on the preset's outline, 50° | +148 | +114 | +101 | +138 | +148 | +140 | +154 | +150 | +107 | +99 | +103 | +105 | +106 | +59 | −15 | +27 | 0 |
| on the inner line, 51.2° | −25 | −23 | −33 | +5 | +11 | 0 | −2 | −4 | 0 | −4 | −2 | −2 | −4 | −3 | +4 | 0 | 0 |

  Each rib's centre line then lies within 2 mm of the drawn one (ribs 3 to 17, those found on the image). Ribs 6 to 17 have their drawn length within 4 mm, with nothing fitted but the seventeenth's place: the inner line, the scale and the angle check each other. Ribs 1 to 3, in the treble corner, are 2 to 3 cm short: ten points cut that corner.

### The outline on the inner line

Ten points fitted on the drawing's inner line, in the preset's frame (`a` and `b` unchanged, so no bridge point moves): within 5 mm rms of the line, 34 mm at worst (a rounded corner), area 1.85 m².

```toml
outline = [[0.0187, 0.0469], [0.1106, 0.0435], [0.2860, 0.2972], [0.3684, 0.3560], [0.5206, 0.3865], [0.9223, 0.4540], [0.9735, 0.5841], [0.9808, 0.8799], [0.9423, 0.9727], [0.0687, 0.9796]]
```

Every bridge point comes 3 to 5.5 cm nearer the rim: A0 from 27 to 23 cm, F2 from 23 to 19, C4 from 37 to 31, C6 from 19 to 16. The top of the long bridge ends close to the treble cheek: B7 at 1.3 cm of the inner line, C8 at less than a centimetre, where the preset has them at 6.8 and 5.7 cm. That place is known to ± 1.5 cm only: it hangs on RT-425's bridge points against its own figure, and from Plath's table and a case of 157 cm with equal rims C8 would lie 3 cm inside the line.

### What it is worth: the board's resonances

The cheap test before anything else: the preset with that outline and the ribs placed again on it (`out/presets/cand.toml`), its first modes as the bridges see them (`bass_shapes`), against the D that Plath measured at each stage of its making.

| | in the rim, no strings (Hz) | strung (Hz) | mean mobility at Plath's fifteen points, strung / in the rim (s/kg) | board |
|---|---|---|---|---|
| the real D (Plath) | 56, 80, 114 | 74, 98, 136 | 0.7e-3 / 0.8e-3 | 13.7 kg with its margin |
| the preset: the panel's edge, clamped | 57, 79, 110 | 78, 100, 137 | 0.76e-3 / 0.76e-3 | 12.7 kg, 2.09 m² |
| the inner line, clamped, the ribs as they were | | 85, 112, 154 | 0.63e-3 | 11.8 kg, 1.85 m² |
| **the inner line, clamped, the ribs on it: the candidate** | 63, 87, 125 | 83, 108, 148 | 0.63e-3 / 0.63e-3 | 11.5 kg |
| the same, `crown_freq` 28 Hz instead of 40 | 57, 83, 122 | 79, 104, 146 | 0.63e-3 | |
| the same, simply supported (`clamped` off) | 55, 75, 105 | 75, 94, 129 | 0.63e-3 | |
| the panel's edge, simply supported | 51, 68, 93 | | 0.79e-3 | |

The outline as drawn, clamped, puts the three resonances 9 to 12 % above the real board's, where the preset has them within 4 %; simply supported on the same line, 2 to 8 % under. A real edge is between the two, and the preset's wider board, clamped, happens to stand where the real one does. What the 10 % may be is not settled: a rim's glued ledge that lets the edge turn a little, the spruce's moduli (RT-425's 11 GPa along the grain, on which a frequency goes as the square root), the feathered ends, or `crown_freq`, a term fitted on another board with the wider outline. Refitted on Plath's first resonance it is 28 Hz, and the second and third stay 4 and 7 % high: one term cannot do it.

Mobility at each key's bridge point, the candidate against the preset, by third of an octave from 500 Hz to 5 kHz (`bridge_mobility`): within 1 dB on average in most bands (3 to 4 at 500 Hz above A4, both ways) and 2 to 4 dB rms from key to key up to G#6, which is a redraw. From A6 the points are within 9 cm of the rim and lose mobility below their own fundamentals: A6–G#7 −11 dB at 500 Hz, −6 at 630, +5 at 1.6–2 kHz, nothing above; A7–C8 −34 dB at 500 Hz, −14 at 2 kHz, −5 at 3.15 kHz, −3 at 4 kHz, +1 at 5 kHz. At their partials the top four keys lose 3 to 5 dB.

So it can move three things: the partials of A0–B2 that fall between 60 and 200 Hz, through the resonances; the top four keys, through their distance to the rim; and, as any change of the board does, the level of every partial of every key.

### Measured beside the preset

All 88 keys at velocities 60 and 100 through the close microphones, no room (the distance to the commercial D, `dscore.py`); the bench of eight real grands; the bass against Plath's grand (`tools/bass_gaps.py plath`) and the per-partial gap at the ears. The preset is main at 88e4f13, measured again the same day: after the felt, the unisons, the string loss and the dampers it reads 1.44 on the bench and 8.06 against the commercial D.

| | bench | bass | middle | treble | top | D, 88 keys | bass | middle | treble | top |
|---|---|---|---|---|---|---|---|---|---|---|
| the preset | 1.44 | 1.21 | 1.21 | 1.44 | 1.91 | 8.06 | 8.40 | 9.21 | 8.37 | 5.86 |
| the candidate | 1.45 | 1.10 | 1.21 | 1.60 | 1.88 | 8.40 | 7.22 | 9.28 | 10.07 | 6.55 |
| the candidate, `crown_freq` 28 | 1.46 | 1.09 | 1.23 | 1.60 | 1.89 | 8.46 | 7.42 | 9.29 | 10.10 | 6.54 |
| the inner line, simply supported | 1.54 | 1.28 | 1.19 | 1.52 | 2.14 | 8.84 | 8.39 | 10.11 | 10.10 | 6.15 |

Decay of the partials at the bridge by their frequency, 0.15–1 s (dB/s, median):

| | 45–70 | 70–100 | 100–140 | 140–200 | 200–280 Hz |
|---|---|---|---|---|---|
| A0–E1, one string: the real grand | 3.0 | 6.7 | 3.9 | 3.7 | 4.6 |
| the preset | 1.0 | 7.1 | 5.8 | 3.8 | 2.9 |
| the candidate | 0.6 | 3.9 | 5.6 | 6.1 | 3.7 |
| the candidate, `crown_freq` 28 | 0.8 | 4.0 | 5.8 | 5.7 | 3.6 |
| F1–E2, the bass bridge: the real grand | 2.6 | 8.5 | 5.9 | 6.9 | 2.9 |
| the preset | 1.8 | 16.0 | 6.8 | 8.5 | 4.5 |
| the candidate | 1.2 | 11.7 | 5.8 | 9.9 | 4.7 |
| F2–B2, the main bridge: the real grand | | 18.8 | 4.7 | 21.3 | 5.8 |
| the preset | | 10.2 | 6.3 | 6.5 | 6.1 |
| the candidate | | 4.9 | 14.7 | 11.0 | 6.7 |

Per-partial gap at the ears against the real grand, whole / smooth (dB rms):

| | A0–G#1 | A1–G#2 | A2–G#3 | A3–G#4 | A4–G#5 | A5–G#6 |
|---|---|---|---|---|---|---|
| the preset | 7.8 / 4.5 | 8.8 / 5.7 | 8.0 / 4.2 | 8.8 / 5.4 | 7.9 / 3.0 | 6.3 / 2.3 |
| the candidate | 7.7 / 4.2 | 7.8 / 4.3 | 9.1 / 5.8 | 8.9 / 5.6 | 8.5 / 3.1 | 5.2 / 1.9 |

Drop of the sound at the ears between 0.2 and 2 s (dB), A0–B0, C1–B1, C2–B2: the real grand 11.5, 9.1, 13.6; the preset 9.1, 9.9, 12.6; the candidate 9.8, 10.2, 12.9.

**What is cause and what is the redraw.** A step under 0.5 in a register (1.3 at the top) says nothing about a board (docs/real_board.md).

- *Cause.* The resonances: the single strings' loss moves from 70–100 Hz, where the real grand and the preset have it (6.7 and 7.1 dB/s), to 140–200 Hz (3.9 and 6.1 where the real one reads 6.7 and 3.7). On the bass bridge's upper keys the preset's excess at 70–100 Hz comes down (16.0 to 11.7 against 8.5); on the main bridge's lowest keys the candidate takes at 100–140 Hz what the real board takes at 70–100. On the whole the low partials' decays are further from the real grand's.
- *Beyond a redraw, cause not traced.* The bass is 1.2 nearer the commercial D (its attack's partials, 10.3 to 8.3 dB rms at ff) and 0.11 better on the bench. The treble is 1.7 further from the D and 0.16 worse on the bench: C5–B6 are 1.3 dB lower against the keyboard's median and their partials 2 to 8 stand higher over the D's (attack +5.0 to +7.2 dB, sustain −2.2 to +4.5). Their bridge points are 2 to 3 cm nearer the rim on a board a ninth smaller.
- *Redraw.* The middle, the top, the gap at the ears (one octave nearer by 1.0, the next further by 1.1), the bench's total (1.44, 1.45, 1.46).

By measure the outline as drawn is neutral on the real grands and 0.3 further from the commercial D, and it takes the board's resonances away from the measured D's. Nothing in these figures asks for it in the preset; it is the drawing's line, and the ear and the keyboard were to say the rest.

### Listening

`out/ab_outline/index.html` (the `ecoute-contour` entry of `.claude/launch.json`): 23 keys from A0 to C8 through the close microphones at velocities 100 and 60 with the commercial D, and 20 keys up to C7 at the player's seat, lid off, with the real grand; the preset, the candidate and the candidate with `crown_freq` 28, one gain for the three. `out/keyboard/instruments/Board outline trial/` holds the three presets for the instrument's own list (`target/release/unsampled-piano --ui --presets out/keyboard`). The Model page draws whatever outline the preset has, with the inner rim under the board's margin outside it: no change of the interface was needed. The workspace's tests pass with the candidate in the place of the preset (316).

### In the preset (2026-11-15)

The three presets were put in the instrument's list and the user had the outline of the drawing taken into the preset, as it is drawn, `crown_freq` left at 40 Hz. `concert_grand.toml` has the ten points on the inner line and the ribs placed on it at 51.2° (`tools/steinway_d_ribs.py`, whose default angle that now is and which gives the preset's ribs back); its renders are the measured candidate's, bit for bit (four notes compared). The former outline is kept in a comment beside it; a preset that carries its own outline renders as before, and the examples `board_fem` and `board_tf` keep the former one, the board of docs/real_board.md's figures. What was heard is not recorded here: no remark came with the decision. The preset now reads 1.45 on the bench and 8.40 against the commercial D, and its board's first resonances are 63, 87, 125 Hz in the rim and 83, 108, 148 Hz strung, 9 to 12 % over the measured D's: the edge is the open question, below.

### Not verified

- **What was heard.** The outline was taken into the preset without a remark to record; whether the keyboard or the listening page decided, and on which keys, is not on record.
- **Which edge the real board has.** The inner rim's face is where the drawing ends its ribs; how far the glued margin lets the edge turn is not measured, and the 10 % on the resonances is not explained.
- **The drawing's place in the model's frame**, through the fit of its outer line on an outline that was itself traced by hand off RT-425's figure: ± 1 cm, ± 0.5°. The top keys' distance to the treble cheek hangs on it (± 1.5 cm on 1 cm).
- **The scale** on the ribs' figures only; no outer dimension of the piano is on the drawing. Plath's figure and the case's 157 cm agree with it within a centimetre or two.
- **Ten points**: the rounded corners are cut (34 mm at worst), ribs 1 to 3 are 2 to 3 cm short.
- **The case's plan.** With the outline on the inner line the rim's wall, the lid (7.5 cm outside the outline) and the frame's plate all come in by the margin: a case 1.54 m wide where the D has 1.57 and the preset 1.63. `rim.thickness` would be 9.8 cm; not changed, not measured for its effect.
- **The treble's 1.7** against the commercial D: read on the figures, not traced to a cause.
- **RT-425's figure 1** was not looked at again (not stored); that the preset's outline is its trace is the earlier record's word.

### Tools

`tools/steinway_d_ribs.py [angle] [short] [preset]` now reads the outline from a preset. The scripts of this section are in the session's `out/tools/` (not versioned): `extract.py` (the drawing's lines by colour), `scale.py` (its scale from the ribs), `shapefit.py` (the drawing's lines on the model's outline), `plathfig.py` (Plath's figure by his table, RT-425 against Plath), `polyfit.py` (the ten points), `ribcheck.py`, `mkpresets.py`, `measure.sh`, `summ.py`, `abpage.py`; the figures in `out/lit/`.
