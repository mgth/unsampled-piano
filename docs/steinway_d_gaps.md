# What still separates the model from the D

Status as of 2026-11-11. A follow-up list for the Concert grand preset: what in the model is not the Steinway D's physics yet, ranked by how much each is likely to move the sound. The figures are those of docs/steinway_d.md and docs/real_board.md. The ranking is a judgment from those figures, not a measured sensitivity.

- **Wrong**: the right physics is known and the model does not follow it.
- **Estimated**: the value is missing from the sources used.

## Ranked

| # | Difference | Likely effect on the sound | To implement | Kind | State |
|---|---|---|---|---|---|
| 1 | The board above 3 kHz: a statistical plate set on `rho_h`, `dx`, `dy`, not the built structure | Strong in the treble, the register furthest from the D (8–12 dB); radiation hole near 2–2.5 kHz | Finer or second-order elements, at a cost in build time | Wrong (approximation); no data missing | started 2026-11-11 |
| 2 | The felt's law: exponent 3.7–5.0 against 2.3–3.5, brightness growing with velocity by 22.7–31 dB at A0–B1 against the commercial D's 35.7 | Strong, whole keyboard | The exponent alone is a quick trial; another form of law is real work | Wrong, though RT-425's values do not carry over as they are | open |
| 3 | The bass bridge's conductance at 70–140 Hz: two to four times too high | Strong at A0–E1: partials 2–4 die too fast | Cause located (modes at 77 and 95 Hz too broad under the resting strings' load), remedy not | Wrong, measured against a real grand | open |
| 4 | The last damper at G6 | Strong at the top: its 1 kHz band in the sustain falls from −42 to −110 dB, real grands −43 | A parameter | Estimated (the commercial D's setting) | open |
| 5 | The air in the case: a statistical small room, modes not coupled through the air | Medium: C6–B6's fundamentals still 2.8 dB short, hole at 1–1.7 kHz | Heavy: acoustics of the real case | Wrong (simplification) | open |
| 6 | Case height, lid size, prop angles | Medium: the openings set the case field's level | Parameters exist | Estimated | open |
| 7 | Crown and bearing: a foundation term set on four modes of another board | Medium in the bass: places the board's first modes | Heavy: a prestressed shell | Both | open |
| 8 | The strings' rear lengths (bridge to hitch pin) | Medium in the bass: the first resonance moves from 62 to 73 Hz within the estimate | A table, once measured | Estimated | open |
| 9 | Duplex resonances | Small to medium, in the treble | Medium | Absent; lengths missing | open |
| 10 | The iron frame's real shape | Unknown, probably small to medium | Medium, given a drawing | Estimated | open |
| 11 | The wound strings' core and winding | Small: mass and inharmonicity are already the D's, the winding's loss is left | Parameters | Estimated | open |
| 12 | Rim and frame giving nothing back to the board | Negligible: computed, 2 modes in 227 change | Possible, not useful | Wrong, without effect | closed |
| 13 | The 17th rib, the plate's braces, the bridges' pins and notches, the rib-plate glue line | Small | Easy to medium | Mixed: the 17th rib's place is missing, the rest simplified | open |

## Heard, cause unknown

- **The bell of the low bass** (A0–D#1): strong by ear, and it lies in the level of each partial of each note. No law to implement was found.
- **The real strike point at the top**: the value is RT-425's and it costs (D#7–F#7 9.5 dB short). Something else is wrong up there, probably item 1.

## About the felt (item 2)

RT-425's law was tried key by key, as a whole: exponent, stiffness, hysteresis time and mass, without a core. Distance to the commercial D 9.37 against 6.54, loudness key to key 6.95 dB against 3.80, the top keys 16 dB short, brightness growth 15.8 dB. Not tried: the real exponent alone, kept with the model's core and hysteresis.

## Data to find

All of it can be measured on a real D or read off dimensioned photographs, and none of it needs new code: rear lengths, case and rim heights, the lid's size and its prop's angles, the frame's shape, the 17th rib's place, the last damper.
