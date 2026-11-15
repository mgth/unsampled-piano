# The strings in sections

Status as of 2026-11-15. Code: `SoundboardParams::string_sections`, `bridge_place`, `bridge_curve_at` (`soundboard.rs`); the Model page's `key_across` (`model/scene.rs`) and the plate's struts (`model/scene/plate.rs`). Analysis: `piano-render`'s examples `string_sections` and `bridge_mobility_keys`.

On a real grand the strings do not run evenly from A0 to C8: they stand in blocks, with a gap between two blocks where a bar of the iron frame passes, and the hammers and the dampers are grouped the same way. The model set only the bass apart (its own bridge). It can now place the strings in sections; on the Concert grand that is right for the Model page and **neutral for the sound, by measure**. The Concert grand's preset has them since 2026-11-15, at the user's word; the measures below compare the preset before ("preset") with them.

## The real layout

| Section | Keys | Notes | Count | How it is known |
|---|---|---|---|---|
| Bass, overstrung, wound, on its own bridge | 1–20 | A0–E2 | 20 | documented: the 1884 scale's 20-note bass; plain wire from F2 (docs/steinway_d.md) |
| Tenor | 21–35 | F2–G3 | 15 | counted on a photograph; follows from the others |
| | 36–53 | G♯3–C♯5 | 18 | counted on a photograph (dampers); its upper end documented |
| | 54–71 | D5–G6 | 18 | counted on a photograph (dampers); its lower end documented |
| Top | 72–88 | G♯6–C8 | 17 | follows from the others |

Sources and their weight:

- **Two photographs from above** (a D with its lid off; a grand's action out of the piano, its hammers in five groups), found by the user, who counted 20, 15, 18, 18 and 17. Measured again on the first: the dampers' heads stand at a pitch of 12.8 to 13.1 pixels, and the two blocks between the bars span 235 and 236 pixels: 18 and 18 keys. The tenor's block is half hidden by the diagonal bar (15 or 16 by the same count). The photographs are not in the repository.
- **Steinway's service manual** (Worldwide Technical Reference Guide, chap. 5, https://indyptg.org/PTG/SteinwayServiceManuals/Chp5.pdf): the sostenuto tab at "note #54" must clear the plate's bar: the break between keys 53 and 54. The strongest written source found.
- A technicians' forum describes the earlier Centennial D (a 17-note bass) as 17-18-18-18-17; three keys moved from the tenor to the bass give 20-15-18-18-17. Secondhand, and the step is an inference.
- No hammer maker's or manufacturer's table giving the five counts was found.

What else the sources say, and what they do not:

- **The gaps' width** is published nowhere. Read off the photograph, at the dampers' line: the space between the last head of a block and the first of the next is 12 to 15 pixels wider than between two neighbours, at 1.1 mm a pixel (the keyboard's 88 keys over 1114 pixels): **13 to 16 mm, ± 4 mm**, at each of the four breaks, the bass's included. The model takes 14 mm for all four. The same gap at the agraffes and at the bridge is an assumption (a bar of constant width along the strings); the photograph is too coarse to measure it there.
- **The bars.** Seen from the keyboard: one each side of the bass strings, a short one that stops at the front bar between keys 35 and 36 (on the 1884 D it does not run to the back of the plate), a full one between 53 and 54, another between 71 and 72, and the removable diagonal bar across the tenor.
- **The bridge under the bars.** The D's dimensioned drawing (Boutillon, Ege and Paulello 2012, arXiv 1210.3948, fig. 2) shows the long bridge with two small steps in its line in the treble, where the two full bars cross it, and none in the tenor. Their size is not dimensioned. Not modelled: the bridge's line is unchanged.
- **Agraffes and capo bar.** Agraffes up to key 53, the capo bar on the two top sections (secondary sources). The model does not tell them apart.
- **Dampers.** One source gives 71 dampers on a D (the first four sections); docs/release.md reads a real concert grand ringing free from F6. Not settled here, and not touched.
- **Hammers.** Bored at an angle in the bass and the low tenor, straight in the treble (Steinway's manual, chap. 4). The model's hammers have no angle.
- **The strings' directions** per section: not found. RT-425's bridge points and string lengths (docs/steinway_d.md) are rounded to the centimetre and smooth: they do not show the gaps (the step from key to key along the bridge falls evenly from 92 mm at F♯2 to 10 mm at the top). Plath's fifteen measured bridge points (± 2.5 mm) are too few to show them.
- **Measured tone across the bars**: nothing published was found. The technicians' lore is that the audible breaks sit at the bars because the scale changes there (bridge, wound to plain wire, string count), not because of the bars.

## What the model had

Every string's place comes from one function, `SoundboardParams::bridge_point`: the key's point on its bridge. The long bridge is a smooth curve through five points (RT-425's on the Concert grand, the strings' real ends, no slant added), its keys at equal steps of the curve's parameter from the lowest (F2) to C8; the bass bridge an arc, its keys at equal steps. That point feeds:

- the board: where the key's strings drive its modes (each key at its own point up to 6 kHz on the Concert grand), where they hold the bridge and grip its top (`soundboard.structure.strings`, `strings_grip`), and the sympathetic strings' regions;
- the frame: each key's agraffe faces its bridge point across (`key_across`);
- the Model page and the plan sketches: strings from the bridge point through the hammers' line to the agraffe; hammers at an even pitch across, the bass's beyond; dampers on the strings; the plate's struts either side of the bass, beside the tenor's lowest string, and two among the treble's strings at a third and two thirds of the way, over a unison's gap.

The microphones see the board, not the strings: the strings' places reach them only through the bridge points. The hammers' strike points are a share of each string's length and do not depend on the layout.

## What was changed

`soundboard.string_sections`: up to four pairs of a section's lowest key (MIDI) and the gap below it, in metres across the strings. The Concert grand's are

    string_sections = [[41.0, 0.014], [56.0, 0.014], [74.0, 0.014], [92.0, 0.014]]

- **On the long bridge** the gaps come out of the keys' spacing (`bridge_place`): the two end keys stay where the measured layout has them, each gap counts as its width over the keys' spacing across the strings there (the bridge runs at an angle to the strings: 14 mm across is 33 mm along it at G♯3, 19 mm at D5, 15 mm at G♯6), and each section's keys draw 3 to 4 % closer. The bridge itself does not move: its line, its beam in the board's modes and its heights are built along `bridge_curve_at`, the curve at equal steps.
- **The break from the bass** (the pair at `main_bridge_from`) parts the hammers only: the two bridges are apart already.
- **The frame's agraffes** follow the keys.
- **The Model page**: the hammers' line takes the same gaps out of its pitch (C8 and F2 stay in line with their strings), the bass's block stands its gap further; keys, action and dampers follow their hammers and strings; the plate's struts among the main strings stand in the gaps instead of at the thirds. Static, live, 3D and editor draw the same scene.

No sections (the default, and any preset saved before): every key where it was, bit for bit (test `the_sections_part_the_strings_along_the_bridge`; the 88-key renders of the preset reproduce the distance read on main, 8.06).

Where the keys go on the Concert grand (`cargo run --release -p piano-render --example string_sections -- <preset>`): the 20 bass keys, F2 and C8 do not move; the others by 7 mm on average along the bridge, the most just under the first break:

| Key | G3 (35) | G♯3 (36) | C♯5 (53) | D5 (54) | G6 (71) | G♯6 (72) |
|---|---|---|---|---|---|---|
| Moved along the bridge (mm) | 28.0 | 4.8 | 11.8 | 6.4 | 5.4 | 9.0 |
| Across the strings (mm, + towards the bass) | +10.9 | −2.0 | +8.4 | −4.5 | +4.8 | −8.1 |
| Step from the key below, along the bridge (mm; before) | 56.0 (56.7) | 86.4 (53.6) | 22.2 (23.0) | 41.4 (23.1) | 15.7 (16.3) | 30.4 (16.1) |

![The Model page's scene from above, before (top) and with the sections](img/string_sections.png)

## Does a real grand show the breaks?

Plath's Steinway concert grand (bridge acceleration and dummy head at the player's seat, anechoic, lid off, every key, one dynamic; the median take of five). Per key: the level of the first 0.2 s and of 0.5–1.5 s, the fundamental's level and partials 2–6 against it (0.02–0.52 s), the attack's spectral centroid (semitones over the fundamental), and the decay rate. Then the step from each key to the next; each break's step is given with the share of the 83 other steps that are smaller.

At the bridge:

| | attack (dB) | 0.5–1.5 s (dB) | partial 1 (dB) | partials 2–6 re 1 (dB) | centroid (semitones) | decay (dB/s) |
|---|---|---|---|---|---|---|
| The 83 other steps, rms | 2.1 | 4.2 | 5.2 | 6.2 | 3.1 | 12.2 |
| The three breaks of the long bridge, rms | 2.1 | 2.4 | 1.0 | 3.9 | 0.9 | 3.7 |
| E2→F2 (20→21, the bass's break) | +2.6 (78 %) | −1.2 (23 %) | +10.9 (96 %) | −8.2 (88 %) | −0.3 (8 %) | +1.1 (45 %) |
| G3→G♯3 (35→36) | +0.7 (27 %) | +1.2 (23 %) | −1.5 (19 %) | +1.8 (25 %) | +1.6 (39 %) | −1.0 (42 %) |
| C♯5→D5 (53→54) | +2.3 (76 %) | +3.9 (65 %) | +0.6 (12 %) | +4.0 (54 %) | +0.2 (7 %) | −2.9 (61 %) |
| G6→G♯6 (71→72) | −2.6 (80 %) | −1.1 (23 %) | −0.5 (11 %) | −5.1 (58 %) | −0.2 (6 %) | +5.5 (69 %) |

At the bridge the three breaks under the bars are ordinary steps, if anything smaller than the others. The bass's break is the one that shows: the fundamental 11 dB up from E2 to F2, the change of bridge and from wound to plain wire, which the model already has. At the ears the same holds for the first two (G3→G♯3: no step above the 53rd percentile; C♯5→D5: attack +4.7 dB, 88 %), the bass's break shows in the centroid (+12 semitones, the largest step of the keyboard), and G6→G♯6 shows a step in the sustain (−13 dB at 0.5–1.5 s, 99 %; decay +23 dB/s, 89 %) that the bridge does not have: something between the bridge and the ears or at the key's release, where the dampers end, not the strings' place. The decay columns are noisy (a line fitted on the first 25 dB) and carry little.

So on this real grand **the bars leave no mark on the tone from key to key**; what marks a break is what the scale changes there.

## The cheap test

Before the Model page was touched: the remap alone, and the board's driving-point mobility at every key's bridge point in octave bands (`bridge_mobility_keys`), with and without the sections.

| Octave band (Hz) | 125 | 250 | 500 | 1000 | 2000 | 4000 | 8000 |
|---|---|---|---|---|---|---|---|
| Change with the sections, 68 keys of the long bridge, rms (dB) | 0.16 | 0.15 | 0.13 | 0.17 | 0.32 | 0.68 | 0.15 |
| Step from key to key, the 64 ordinary ones, rms: before | 0.56 | 0.53 | 0.49 | 0.71 | 1.27 | 2.33 | 0.40 |
| the same with the sections | 0.56 | 0.53 | 0.47 | 0.69 | 1.25 | 2.39 | 0.41 |
| Step at the three breaks, rms: before | 0.42 | 0.37 | 0.62 | 0.21 | 0.77 | 3.28 | 0.53 |
| the same with the sections | 0.63 | 0.66 | 1.03 | 0.39 | 0.66 | 3.89 | 0.57 |

The mobility a key sees moves by a tenth to a third of the ordinary step to its neighbour (0.7 dB at 4 kHz, where the board's wavelength nears the moves). At the breaks the step grows by 0.2 to 0.4 dB below 1 kHz, to the size of an ordinary one. The bass bridge's keys move by 0.05 dB at most: the strings' hold on the long bridge sits elsewhere, which shifts the board's lowest modes by next to nothing.

## Measured beside the preset

Both from the full preset, one line apart; 88 keys at velocities 60 and 100, close microphones, no room, held 3.5 s.

**Key by key** (velocity 100; the same measures as on the real grand):

| With the sections − before, rms over the keys | n | moved (mm) | attack | 0.5–1.5 s | partial 1 | partials 2–6 re 1 | centroid | decay |
|---|---|---|---|---|---|---|---|---|
| Bass bridge | 20 | 0 | 0.08 | 0.11 | 0.08 | 0.23 | 0.30 | 0.04 |
| Long bridge, moved under 3 mm | 22 | 1.5 | 0.37 | 0.54 | 0.57 | 3.20 | 0.31 | 0.72 |
| 3–8 mm | 27 | 5.2 | 1.13 | 1.23 | 1.32 | 1.34 | 0.26 | 3.14 |
| 8–15 mm | 11 | 10.3 | 0.55 | 1.51 | 0.81 | 1.51 | 0.49 | 1.84 |
| over 15 mm | 8 | 23.4 | 0.54 | 0.73 | 0.46 | 0.55 | 0.52 | 1.27 |
| The six keys beside the three breaks | 6 | 10.9 | 0.83 | 2.00 | 0.97 | 1.88 | 0.23 | 2.81 |
| Mean change, long bridge (signed) | 68 | 7.0 | +0.03 | +0.09 | −0.03 | −0.14 | −0.03 | −0.28 |

A key's level changes by about 1 dB rms, its partials by 1 to 3 dB, with no mean shift in any register (under 0.4 dB) and no relation to how far the key moved: the keys that moved most (the low tenor, where the board's waves are long) changed least. That is the redraw of each partial's level by the board, as any change of the bridge points gives (docs/steinway_d.md), not an effect of the sections.

**Evenness** (each key against the median of the three keys either side, rms; velocity 100):

| | attack (dB) | 0.5–1.5 s (dB) | partial 1 (dB) | partials 2–6 re 1 (dB) | centroid (semitones) |
|---|---|---|---|---|---|
| Preset, 88 keys | 2.54 | 3.23 | 5.77 | 4.29 | 2.22 |
| With the sections, 88 keys | 2.51 | 3.26 | 5.80 | 5.46 | 2.30 |
| Preset, the 8 keys beside the breaks | 2.96 | 3.16 | 5.64 | 5.31 | 3.54 |
| With the sections, the same 8 | 3.37 | 4.16 | 6.03 | 5.11 | 3.70 |
| Real grand at the ears, 88 keys | 2.56 | 3.21 | 4.52 | 5.00 | 3.63 |
| Real grand at the ears, the 8 beside the breaks | 2.67 | 3.85 | 3.97 | 3.63 | 5.40 |
| Real grand at the bridge, 88 keys | 1.74 | 3.23 | 4.48 | 5.12 | 2.37 |
| Real grand at the bridge, the 8 beside the breaks | 1.63 | 2.02 | 3.95 | 4.04 | 2.34 |

Over the keyboard the levels do not move; partials 2–6 against the fundamental are 1.2 dB less even (4.29 to 5.46, the real grand's 5.0), from a few keys that moved under 3 mm: a redraw again. The eight keys beside the breaks are 0.4 dB less even at the attack and 1 dB in the sustain with the sections; on eight keys that is one step's redraw (G6 to G♯6 in the sustain, 5.5 dB), and the real grand's are not less even there than elsewhere. The model's steps at the breaks, before and with the sections: E2→F2 the same within 0.2 dB; G3→G♯3 attack +2.8 → +3.8 dB; C♯5→D5 the same within 1.2 dB; G6→G♯6 sustain +2.2 → +7.7 dB.

A step of the model's own, there before the sections and not theirs: from C♯5 to D5 the fundamental falls by 12 dB (larger than 98 % of the other steps) and the attack by 5.6 dB, where the real grand has +0.6 and +2.3 at the bridge. Not looked into here.

**Distance to the commercial D** (88 keys, `KEYSTEP=1`):

| | Overall | Bass | Middle | Treble | Top |
|---|---|---|---|---|---|
| Preset | 8.06 | 8.40 | 9.21 | 8.37 | 5.86 |
| With the sections | 8.01 | 8.25 | 9.39 | 8.14 | 5.87 |

Within ± 0.25 per register, with both signs: under what a redraw gives (docs/steinway_d.md: less than 0.5 in a register says nothing).

**Bench of the real grands**:

| | Score | Bass | Middle | Treble | Top | attack | brightness | dynamics | sustain | resonance | decay | tuning |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Preset | 1.44 | 1.21 | 1.21 | 1.44 | 1.91 | 1.23 | 1.73 | 1.66 | 1.68 | 2.70 | 1.00 | 0.70 |
| With the sections | 1.44 | 1.19 | 1.26 | 1.41 | 1.88 | 1.25 | 1.75 | 1.61 | 1.62 | 2.65 | 1.00 | 0.71 |

58 and 59 % of the measures within the real pianos, 25 and 26 off.

**Bass tables** (`tools/bass_gaps.py d`, A0–B2 against the commercial D): the partials' decay rates agree within 0.1 dB/s in every group but two (C2–B2, partials 5–8 and 9–16: 0.3 and 0.2), the levels within 0.5 dB once the set's gain is taken out (it is set on the peaks of A2–A5 and moved by 0.4 dB).

## Verdict

The sections are the instrument's real geometry, and the Model page is truer with them. For the sound they are neutral by measure: the mobility at the bridge, the levels and decays key by key, the evenness over 88 keys, the distance, the bench and the bass tables all stay within what moving the bridge points by a centimetre redraws, and the real grand shows no mark of the bars either.

## Not verified

- **Not heard and not played.** Listening page: `out/ab_sections/index.html` (the keys either side of each break and six keys in a row across it; preset, candidate, commercial D, real grand). Keyboard: `out/keyboard/instruments/String sections trial/`, with this branch's player (another build does not know the parameter). The sections went into the preset without a listening remark.
- The counts 15 and 17 of the tenor and the top section rest on the photograph and on the three others; the breaks 35|36 and 71|72 have no written source.
- The gap's width, 14 mm, is read off one photograph at the dampers' line, ± 4 mm; taken the same at the bridge and the agraffes, and the same at all four breaks.
- The GPU's 3D view was not opened in a window: the scene is the same one, checked through the software drawing (`PIANO_SHOTS`).
- Not modelled: the bridge's steps under the two full bars, agraffes against capo bar, the hammers' boring angle, the strings' direction per section (the main strings still run from the hammers' line to the bridge as before), the bars themselves in the frame's plate (a plate of uniform mass and stiffness, docs/frame_rim.md), and the sympathetic strings' regions, which still follow the bridge.
- The plan sketch of the parameter pages draws the bridge points, which follow; it has no struts.
