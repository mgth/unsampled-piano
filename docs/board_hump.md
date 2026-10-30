# The board's hump at 290–330 Hz (D4–E4)

Status as of 2026-10-30. The board's mobility is lowered by 1.25 at the same modal frequencies (`rho_h` 10.5, `dx` 5737.5, `dy` 2550). The hump itself is still there.

## What was heard and measured

D#4–F4 fizzed in the highs during the sustain. Listening tests traced it to high partials kept alive by the unison's 1-cent detuning. Real pianos have that too, but they cover it with a longer fundamental. The model's fundamentals of D4, D#4 and E4 fell fast. At mf (close microphones, studio) H1 stood −20, −14 and −19 dB under its own attack at 0.4 s. The real grands (Helgesson, Splendid, BiVib, Salamander) show −4 to −11 dB at D4, +1 to −13 at D#4 and −9 to −10 at E4.

The board's conductance at each key's fundamental (`examples/fundamental_conductance.rs`) is compared with the fit of the strings' measured decays (`tools/bridge_conductance.py`, ≈ 3e-4 s/kg). D4 stood 7.7× over it, D#4 3.8×, E4 5.3×, A3 4.2× and A4 4.5×. The real modal scatter about that fit is about ±2.5× (rms 0.91 in ln over 2103 partials of Salamander).

## Where it comes from

`examples/board_hump.rs` splits Re Y(f0) into the modes that make it:

- **D4: 93 % comes from one mode.** It is own mode 5 at 295.4 Hz, (2,3)-like, η 0.054 (0.033 its own, 0.022 the damped strings' load). Its Φ² at the bridge is 0.23. That is about 95 % of the largest Φ² any mode of this board can reach, 4/(ρh·A).
- **D#4 and E4: two more modes.** Two (4,1)-like modes at 315.6 and 328.8 Hz add to the same mode 5.
- **The three modes share one lobe.** It covers the main bridge from C#4 to F4 (|Φ| 0.44–0.50, `examples/mode_map.rs`).

That makes three strong modes within 35 Hz where the mean spacing is 27 Hz. The ±1/3-octave mean conductance there is 7–8e-4, against 4.3–5.5e-4 on either side.

This is the board's draw, not a modelling fault:

- **The contrast between peaks and mean is a real board's.** It goes as 1/(modal overlap). Ege et al. (2013, an upright in situ, strings muted) measure η 1–3 % (mean 2.3 %) and an overlap of 0.3 at 150 Hz and 0.7 at 550 Hz. The model reaches about 0.6 at 300 Hz with twice that loss and half that density. The model's rms of ln(Re Y(f0)/reference) over C3–C6 is 0.69, against Salamander's 0.91.
- **D4 is a 2.4σ outlier of the model's own spread.** Real grands have such keys too: BiVib's F#4 has H1 at −24.6 dB at 0.4 s, Salamander's D#4 −13.2 dB.
- **One part was a level offset.** The geometric mean of Re Y(f0)/reference over C3–C6 was 1.48.

## What was tried

Re Y(f0)/reference at D4/D#4/E4, the geometric mean and rms over C3–C6, the number of keys of 37 over 2.5×, and the bench (`tools/bench.py`, close microphones, studio):

| | D4 | D#4 | E4 | gmean | rms | > 2.5× | bench |
|---|---|---|---|---|---|---|---|
| before | 7.74 | 3.84 | 5.29 | 1.48 | ×2.00 | 10 | 1.14, 11 off |
| `bridge_beam` 1 | 5.29 | 3.13 | 1.09 | 1.08 | ×2.20 | 8 | 1.26, 18 off |
| `loss_factor` 0.05 | 4.82 | 4.00 | 3.64 | 1.76 | ×1.58 | 8 | |
| `crown_freq` 0 | 3.56 | 4.97 | 1.51 | 1.41 | ×2.06 | 8 | |
| main bridge with a real grand's agraffe offsets (4–13 cm towards the front) | 6.2 | 5.5 | 4.2 | 1.59 | ×1.86 | 9 | |
| ribs 13 cm, share 0.8 | 7.73 | 3.84 | 5.29 | 1.48 | ×2.00 | 10 | |
| ribs 13 cm, share 0.95, jitter 1 | 8.57 | 3.28 | 4.28 | 1.68 | ×2.05 | 13 | |
| own modes 2× as dense, same mobility (see below) | 2.19 | 2.05 | 1.88 | 1.87 | ×1.54 | 9 | 1.41–1.66, 18–37 off |
| mobility × 0.67 | 5.80 | 2.52 | 3.71 | 0.97 | ×2.09 | 6 | 1.14, 13 off |
| mobility × 0.75 | 6.32 | 2.85 | 4.12 | 1.09 | ×2.06 | 6 | 1.13, 12 off |
| **mobility × 0.8 (kept)** | 6.62 | 3.04 | 4.36 | 1.17 | ×2.05 | 6 | 1.13, 11 off |

**Levers that only move the peaks.** The bridge as a beam, the crown, the bridge's place and irregular ribs move the peaks to other keys: the rms stays at ×2. The beam also costs the bench as in [bridge_beam.md](bridge_beam.md). The loss would need η ≈ 0.16 at 300 Hz to bring D4 within 2.5×, seven times Ege's.

**Ribs at a real spacing (10–18 cm) change nothing at 300 Hz.** The bending wave is 0.5–0.7 m long there, four to five rib spacings. It sees their smeared stiffness, which `dx`/`dy` already hold ([ribs.md](ribs.md)). Only at an unreal 20–25 cm do irregular ribs free D4–E4, and 15–16 keys go over 2.5× instead.

**Modal density.** A 2 m grand's board should have a denser low range than the model's. Ege's upright (0.91 × 1.39 m) and Suzuki's baby grand have a mode every ≈ 22 Hz. The model's outlined board, 1.81 m², has one every 27 Hz between 120 and 480 Hz, and its lowest mode at 122 Hz. The own modes (below `outline_below`) were put on a plate twice as dense, with the bridge's drive scaled to the same mean mobility. Only +62 modes are needed; the rectangle's modes above were left alone.

The strings' side came out as hoped. The rms fell to ×1.54. With the drive at 0.6 of the mean mobility, D4–E4 stood at 1.1–1.3× and only 2 keys of 37 over 2.5×. H1 at 0.4 s came to −6.4/−5.9/−3.9 dB.

The radiation did not follow. A denser plate's coincidence frequency rises with it. At the same mass, the own modes radiated 2–15 dB less per newton between 200 Hz and 1.25 kHz (`examples/board_radiation_level.rs`). A lighter plate gives it back by the mass law, +3 dB per halving, but not the shape: a hole at 200–315 Hz and at 1–1.25 kHz stayed. Even at ρh 2.1 kg/m², lighter than a bare spruce panel, the bench stood at 1.41 with 18 off:

- the bass's partials 3–6 down 5–8 dB against p1;
- the 1 kHz bands down 4–20 dB in every register;
- the treble's envelopes at 2 s down 3 dB.

A board that dense needs its radiation rebuilt (ribs, bridges and a lighter panel together), not a recalibration. Not kept.

## What was kept

The board's surface mass and stiffnesses × 1.25, at the same D/ρh. The modal frequencies and shapes are unchanged; the mobility everywhere is × 0.8. This moves the level offset, not the hump: the geometric mean of Re Y(f0)/reference goes from 1.48 to 1.17.

Bench (`channel-power-v3`): 1.14 → 1.13, 11 off → 11; harmonic profiles 365 → 362 OFF.

- **Better.** The middle's and the treble's envelopes at 1–2 s, pp and mf (z −0.9 to −1.7 → −0.4 to −1.1), and the treble's 2 kHz attack band.
- **Worse.** The middle's early decay of partials 5–12 at ff, now off (z +1.4 → +2.3). The treble's level at 0.3 s at ff (z +0.2 → +0.9) and the top's lines (−4.4 → −4.8, already off).
- **`hf_even`.** 8–11 kHz rms +4.2 → +4.4 dB, largest gap +10.6 → +10.7; 4–8 kHz largest gap +11.9 → +12.1.
- **Level.** The board radiates ≈ 1.5–1.9 dB less for the same force (mass law: 20·log 1.25).

H1 at 0.4 s (mf, close, studio):

| | D4 | D#4 | E4 | F4 | A4 |
|---|---|---|---|---|---|
| before | −20.1 | −13.7 | −18.8 | −7.6 | −25.5 |
| after | −19.4 | −10.7 | −16.8 | −6.3 | −24.4 |
| real grands | −4 to −11 | +1 to −13 | −9 to −10 | −6 to −7 | −4 to −15 |

D#4 comes into the real grands' range. D4, E4 and A4 stay outside: their fundamentals still sit on this board's modes.

## Tools

- `examples/board_hump.rs` (piano-core). With no argument: Re Y at each fundamental from C3 to C6 against the reference, with the summary above. With keys: the modes that make it. `SB="name=value,…"` overrides board parameters (`mobility`, `density`, `bridge_beam`, ribs, `tb1`–`tb4` for the main bridge's points).
- `examples/mode_map.rs`: an ASCII map of an own mode, with the bridge.
- `out/table_bosse/` (not in the repository): the before/after listening page, D4–F4 at mf, the chromatic scale C4–G4 and Chopin's Nocturne op. 9 no. 2 (75 s), close microphones and studio, the same gain.
- `examples/board_radiation_level.rs`: the pressure at the first microphone per newton at a key's bridge, per third octave.
