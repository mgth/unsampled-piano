# What still separates the model from the D

Status as of 2026-11-12. A follow-up list for the Concert grand preset: what in the model is not the Steinway D's physics yet, ranked by how much each is likely to move the sound. The figures are those of docs/steinway_d.md, docs/real_board.md and docs/release.md. The ranking is a judgment from those figures, not a measured sensitivity: the board above 3 kHz, ranked first on 2026-11-11, turned out neutral once done.

- **Wrong**: the right physics is known and the model does not follow it.
- **Estimated**: the value is missing from the sources used.

## Open, ranked

| # | Difference | Likely effect on the sound | To implement | Kind |
|---|---|---|---|---|
| 1 | The felt's law: exponent 3.7–5.0 against 2.3–3.5, brightness growing with velocity by 22.7–31 dB at A0–B1 against the commercial D's 35.7. The real strike point at the top costs 6 dB on D#7–F#7 whatever the board: that too is in the hammer and the string | Strong, whole keyboard, and the top | The exponent alone is a quick trial; another form of law is real work | Wrong, though RT-425's values do not carry over as they are |
| 2 | The bass bridge's conductance at 70–140 Hz: two to four times too high | Strong at A0–E1: partials 2–4 die too fast | Cause located (modes at 77 and 95 Hz too broad under the resting strings' load), remedy not | Wrong, measured against a real grand |
| 3 | The radiation between 1 and 2.5 kHz: a hole at 1–1.7 kHz from the bridge points, a step near 2–2.5 kHz, C6–B6's fundamentals 2.8 dB short. All of it lies below 3 kHz and did not move with the board's high modes: what is left is the air in the case (a statistical small room, modes not coupled through the air) and what fills the hole on a real piano (strings, frame, the plate's openings) | Medium to strong in the treble: 9.69 from the D with the corner at 6 kHz (bass 8.70, middle 9.86, top 9.54), and the one register the built board moved away, from 8.51 | Heavy: acoustics of the real case; a first step is to find by measure which of the three carries it | Wrong (simplification) |
| 4 | The last damper at G6 | Strong at the top: its 1 kHz band in the sustain falls from −42 to −110 dB, real grands −43 | A parameter | Estimated (the commercial D's setting) |
| 5 | The dampers' release: 6.1 dB rms from the commercial D's key-ups, 4.3 from a Disklavier grand's. On the pedal bench, with `damper.horizontal` 0.3: halo 3.6, hold 4.8, after 11.5 dB rms; at CC64 = 96 the treble's "after" is 15.7 dB over the recording, the middle's 2.0 short. The half pedal (CC64 = 64) damped 3 dB less than recorded in the middle and the treble before `horizontal` 0.3, not read again since | Medium, on every key-up and pedal change; the treble under the pedal most | Parameters and the felt's law of contact | Wrong in part, the felt's own values estimated |
| 6 | Case height, lid size, prop angles | Medium: the openings set the case field's level | Parameters exist | Estimated |
| 7 | Crown and bearing: a foundation term set on four modes of another board | Medium in the bass: places the board's first modes | Heavy: a prestressed shell | Both |
| 8 | The strings' rear lengths (bridge to hitch pin) | Medium in the bass: the first resonance moves from 62 to 73 Hz within the estimate | A table, once measured | Estimated |
| 9 | Duplex resonances | Small to medium, in the treble | Medium | Absent; lengths missing |
| 10 | The iron frame's real shape | Unknown, probably small to medium | Medium, given a drawing | Estimated |
| 11 | The wound strings' core and winding | Small: mass and inharmonicity are already the D's, the winding's loss is left | Parameters | Estimated |
| 12 | The 17th rib, the plate's braces, the bridges' pins and notches, the rib-plate glue line | Small | Easy to medium | Mixed: the 17th rib's place is missing, the rest simplified |
| 13 | The board above 6 kHz: still the statistical plate | Small: 8 kHz measured and sounded as 6 kHz | Above 8 kHz the 1 cm grid and the plate model give out | Wrong (approximation) |

## Done or closed

| Difference | What was done | Result |
|---|---|---|
| The board above 3 kHz (2026-11-11) | The built board's own modes up to 6 kHz, with the elements' mass matrix, at no cost per mode (docs/real_board.md, "Above 3 kHz"). 8 kHz tried: the same by measure and by ear | Neutral: distance to the D 9.29 → 9.46 on all 88 keys, bench 1.97 → 1.98, the treble further (8.51 → 9.69), the top octave's level 2–3 dB nearer |
| The strings on the bridge, the piston mode, the sideways polarisation (2026-11-11) | `structure.strings`, `piston_freq` 0, the polarisation measured on a real grand | Distance to the D 9.08 → 7.32 (every sixth key), single strings decaying at one rate |
| The dampers' strength up the keyboard (2026-11-12) | `damper.strength_slope` 1: every key stops at about the same rate (docs/release.md) | Releases against the commercial D 9.4 → 6.5 dB rms, against a Disklavier grand 5.7 → 4.3 |
| The sideways tail after the key-up (2026-11-12) | `damper.horizontal` 0.3: the felt grips the sideways motion | Tail over C2–C5 −50 → −57 dB (the commercial D −60), eight keys above −45 dB down to one; releases 6.5 → 6.1 |
| Rim and frame giving nothing back to the board | Computed, not built: 2 modes in 227 change | Closed, without effect |

## Heard, cause unknown

- **The bell of the low bass** (A0–D#1): strong by ear, and it lies in the level of each partial of each note. No law to implement was found.

## About the felt (item 1)

RT-425's law was tried key by key, as a whole: exponent, stiffness, hysteresis time and mass, without a core. Distance to the commercial D 9.37 against 6.54, loudness key to key 6.95 dB against 3.80, the top keys 16 dB short, brightness growth 15.8 dB. Not tried: the real exponent alone, kept with the model's core and hysteresis.

## What the board above 3 kHz taught

The treble's distance was put on the statistical plate and was not there. Before a heavy item is built, a cheap test should show that it carries the gap: a swap of one stage with the reference (docs/stage_swap.md), or the gap read in the band the item acts on.

## Data to find

All of it can be measured on a real D or read off dimensioned photographs, and none of it needs new code: rear lengths, case and rim heights, the lid's size and its prop's angles, the frame's shape, the 17th rib's place, the last damper.
