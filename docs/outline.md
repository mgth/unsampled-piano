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
- The lid stays a rectangle as wide as the board.
- The frame follows the case since 2026-10-28, with its own modes up to 3 kHz ([frame_rim.md](frame_rim.md#the-outline)); it had inherited the board's polygon on its own rectangle by accident.
- Uprights keep the rectangle.

## 2026-10-28: on by default

What settled it was not the evenness but the stereo attack of the bass ([bass_metal.md](bass_metal.md), "The attack's coherence, band by band"). With the rectangle, the modes at 260–335 Hz reach the two close microphones in antiphase: a bass attack arrives at them 3.7 ms apart with no correlation between the channels (real pairs 0.2–1.7 ms, +0.2 to +0.5), which the user heard as part of the bass's "tube". With the outline and the same microphones the attack is coherent in every band from 250 Hz to 8 kHz. The user chose the outline with the board recalibrated: `rho_h`, `dx`, `dy` × 1.2 (the modes above `outline_below` keep a fifth of their mass outside the outline), `impedance_slope` 0.0024 and `impedance_slope_bass` 0.0076 instead of 0.02 and 0. Bench 1.11 against the rectangle's 1.09 (17 measures worse, 19 better); the new baseline. Still to do: ribs and the two bridges; the sustain's 315 Hz still reaches the pair in antiphase (−0.84).
