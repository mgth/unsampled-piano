# What still separates the model from the D

Status as of 2026-11-14. A follow-up list for the Concert grand preset: what in the model is not the Steinway D's physics yet, ranked by how much each is likely to move the sound. The figures are those of docs/steinway_d.md, docs/real_board.md, docs/release.md and docs/bass_gaps.md. The ranking is a judgment from those figures, not a measured sensitivity: the board above 3 kHz, ranked first on 2026-11-11, turned out neutral once done.

The preset changed on 2026-11-14 (docs/bass_gaps.md): the felt four times harder (`hardness` 1.2) with the top's hardening taken down to keep the top keys where they were (`hardness_slope_top` 0.035). Bench of eight real grands 1.98 → 1.84 (bass 1.39, middle 1.31, treble 1.93, top 2.64); distance to the commercial D 9.46 → 9.53 (bass 8.71, middle 10.08, treble 10.34, top 8.88). The trials of the sections below were made on the preset before it (`hardness` 0.3, `hardness_slope_top` 0.15) and their tables give its figures.

- **Wrong**: the right physics is known and the model does not follow it.
- **Estimated**: the value is missing from the sources used.

## Open, ranked

| # | Difference | Likely effect on the sound | To implement | Kind |
|---|---|---|---|---|
| 1 | The bass bridge between 60 and 140 Hz: coupled to the strings seven times too hard near 70–80 Hz, 2.3 times near 95 Hz. Its resonances are about as broad as a real grand's once the resting strings' load is halved (`string_load` 0.5): the width is not the fault, the coupling is | Strong at A0–E1: partials 2–4 die too fast, and the fundamentals up to B2. The main gap against the real grand | Code; no parameter reaches it (docs/bass_gaps.md, sections 2 and 4) | Wrong, measured against a real grand |
| 2 | The top octave's fundamentals: 12 to 21 dB short at the bridge from E7 up against a real grand, from the first 20 ms, the second partial 21 to 26 dB too strong re the fundamental. Against the commercial D: fundamental −3.7 dB at ff and −11 at mf, partials 2–4 +21 dB | Strong from E7 up | Cause not found. Not the board, the felt's exponent, its hardness, the core or the contact's length (a real grand's). Left: the force's shape during the contact, the strike point, the bridge's mobility at 2.6–4.2 kHz | Unknown |
| 3 | The felt's law: exponent 3.7–5.0 against 2.3–3.5. The real exponent alone does not help. The two references pull apart: the harder felt of 2026-11-14 is nearer the real grands, and the bass's brightness growth from mf to ff fell from 31.8 to 22.0 dB (the commercial D 35.5). The felt over the core in the bass (`core_depth_bass` 0.3) follows the commercial D against the real grands too | Strong, whole keyboard | Another form of law is real work; which reference to follow is a choice first | Wrong, though RT-425's values do not carry over as they are |
| 4 | The radiation between 630 Hz and 2.5 kHz: 6–7 dB short at 630–1000 Hz against a real grand (C6–D6's fundamentals), a hole at 1–1.7 kHz from the bridge points, a step near 2–2.5 kHz. From 1.2 kHz up the board carries the bridge to the ears within 5 dB of the real one. What is left is the air in the case (a statistical small room, modes not coupled through the air) and what fills the hole on a real piano (strings, frame, the plate's openings). In the bass, a hump at 100–250 Hz is missing at the player's seat | Medium in the treble, less than was thought: most of the treble's gap is at the bridge (the stage swap below) | Heavy: acoustics of the real case; a first step is to find by measure which of the three carries it | Wrong (simplification) |
| 5 | The unisons of F1–B2 beat, a real grand's do not | Medium in the bass | `unison.width_mid` 0.2 does it in the bass and changes the middle and the treble too: a width per register, and to be heard | Estimated |
| 6 | The strings' own loss above 1.2 kHz in the bass: two to three times too high by measure (`losses.b2`). Lowered, the strings buzz at the keyboard, at 6.0e-8 and at 1.0e-7, on every felt: the measure and the ear disagree below A3 | Medium, C1–B2 | Not until a measure agrees with the keyboard; any change of it is to be played before it is kept | Wrong by measure, refused by ear |
| 7 | The last damper at G6 | Strong at the top: its 1 kHz band in the sustain falls from −42 to −110 dB, real grands −43 | A parameter | Estimated (the commercial D's setting) |
| 8 | The dampers' release: 6.1 dB rms from the commercial D's key-ups, 4.3 from a Disklavier grand's. On the pedal bench, with `damper.horizontal` 0.3: halo 3.6, hold 4.8, after 11.5 dB rms; at CC64 = 96 the treble's "after" is 15.7 dB over the recording, the middle's 2.0 short. The half pedal (CC64 = 64) damped 3 dB less than recorded in the middle and the treble before `horizontal` 0.3, not read again since | Medium, on every key-up and pedal change; the treble under the pedal most | Parameters and the felt's law of contact | Wrong in part, the felt's own values estimated |
| 9 | Case height, lid size, prop angles | Medium: the openings set the case field's level | Parameters exist | Estimated |
| 10 | Crown and bearing: a foundation term set on four modes of another board | Medium in the bass: places the board's first modes | Heavy: a prestressed shell | Both |
| 11 | The strings' rear lengths (bridge to hitch pin) | Medium in the bass: the first resonance moves from 62 to 73 Hz within the estimate | A table, once measured | Estimated |
| 12 | Duplex resonances | Small to medium, in the treble | Medium | Absent; lengths missing |
| 13 | The iron frame's real shape | Unknown, probably small to medium | Medium, given a drawing | Estimated |
| 14 | The wound strings' core and winding | Small: mass and inharmonicity are already the D's, the winding's loss is left | Parameters | Estimated |
| 15 | The 17th rib, the plate's braces, the bridges' pins and notches, the rib-plate glue line | Small | Easy to medium | Mixed: the 17th rib's place is missing, the rest simplified |
| 16 | The board above 6 kHz: still the statistical plate | Small: 8 kHz measured and sounded as 6 kHz | Above 8 kHz the 1 cm grid and the plate model give out | Wrong (approximation) |

## Not a gap

- **The low fundamentals' hold.** The commercial D's fade at 6–10 dB/s; the real grand's and the preset's hold (0.3–1.4 dB/s). After half a second the preset's fundamental stands 14 dB over the D's at A0–B1: the largest term of the bass's distance to the commercial D, and not to be corrected (docs/bass_gaps.md).
- **The contact's length in the top octave.** Longer than the string's period, as on a real grand (Askenfelt and Jansson, below).

## Done or closed

| Difference | What was done | Result |
|---|---|---|
| The felt's hardness (2026-11-14) | `hardness` 1.2, `hardness_slope_top` 0.035, kept after listening and at the keyboard (docs/bass_gaps.md) | Bench 1.98 → 1.84, the attack heard as much better; distance to the commercial D 9.46 → 9.53 |
| The felt's real exponent, the top felt's hardness, the core in the treble (2026-11-12) | Tried one at a time ("About the felt") | The exponent does not help; a softer top felt helps the soft blows only; the core does nothing above the bass |
| The hammers' contact in the top octave (2026-11-12) | Measured against the string's period and against Askenfelt and Jansson's grand | A real grand's at both dynamics: closed as a cause |
| The board above 3 kHz (2026-11-11) | The built board's own modes up to 6 kHz, with the elements' mass matrix, at no cost per mode (docs/real_board.md, "Above 3 kHz"). 8 kHz tried: the same by measure and by ear | Neutral: distance to the D 9.29 → 9.46 on all 88 keys, bench 1.97 → 1.98, the treble further (8.51 → 9.69), the top octave's level 2–3 dB nearer |
| The strings on the bridge, the piston mode, the sideways polarisation (2026-11-11) | `structure.strings`, `piston_freq` 0, the polarisation measured on a real grand | Distance to the D 9.08 → 7.32 (every sixth key), single strings decaying at one rate |
| The dampers' strength up the keyboard (2026-11-12) | `damper.strength_slope` 1: every key stops at about the same rate (docs/release.md) | Releases against the commercial D 9.4 → 6.5 dB rms, against a Disklavier grand 5.7 → 4.3 |
| The sideways tail after the key-up (2026-11-12) | `damper.horizontal` 0.3: the felt grips the sideways motion | Tail over C2–C5 −50 → −57 dB (the commercial D −60), eight keys above −45 dB down to one; releases 6.5 → 6.1 |
| Rim and frame giving nothing back to the board | Computed, not built: 2 modes in 227 change | Closed, without effect |

## Heard, cause unknown

- **The bell of the low bass** (A0–D#1): strong by ear, and it lies in the level of each partial of each note. No law to implement was found.
- **The buzz of a lower string loss** (2026-11-14): heard while playing only, not on single held notes, and read by no measure below A3 (item 6).

## About the felt (item 3)

RT-425's law was tried key by key, as a whole: exponent, stiffness, hysteresis time and mass, without a core. Distance to the commercial D 9.37 against 6.54, loudness key to key 6.95 dB against 3.80, the top keys 16 dB short, brightness growth 15.8 dB.

**The real exponent alone** (2026-11-12): `p_offset` −1.45, which takes the felt's exponent from 3.7–5.0 to 2.25–3.55, the model's core and hysteresis kept, nothing else changed. All 88 keys at velocities 60 and 100, close microphones, no room, against the D without hammer noise. A lower exponent makes the felt harder at small compressions for the same `hardness`, so two softer felts are given beside it. Growth: the attack's 2–4 kHz band (first 60 ms) from velocity 60 to 100, mean over the keys.

| | Overall | Bass | Middle | Treble | Top | Growth, A0–B1 (the D 35.5) | Growth, C4–B5 (the D 17.1) |
|---|---|---|---|---|---|---|---|
| the preset of 2026-11-12 (`hardness` 0.3) | 9.46 | 8.70 | 9.86 | 9.69 | 9.54 | 31.8 | 17.9 |
| real exponent | 9.75 | 8.71 | 10.00 | 10.52 | 9.69 | 25.5 | 15.0 |
| real exponent, `hardness` 0.1 | 9.58 | 8.71 | 9.88 | 9.74 | 9.94 | 24.1 | 18.1 |
| real exponent, `hardness` 0.03 | 9.17 | 8.69 | 10.44 | 9.22 | 8.18 | 17.6 | 21.9 |

The real exponent does not help. Alone it is further from the D, and whatever the hardness the low bass brightens less with velocity (25.5 to 17.6 dB against 31.8, the D 35.5), which is what the preset's felt was built for. The softest felt comes nearer overall, through the top and the treble only: their attack's partials 2–8 stand 4.5 and 9.5 dB over the D's in the preset, and any softer hammer brings them down (to −1.2 and +8.0). That is the top felt's hardness, not its exponent: `hardness_slope_top` with the model's exponent is the trial that follows from it. The renders are in `out/felt_exponent/`.

**The top felt's hardness** (2026-11-12): `hardness_slope_top`, the felt's hardening above C6 (0.15 by default, 36 times harder at C8), the model's exponent kept. Only the keys above C6 change. Bias: the attack's partials 2–8 re the D's, over C8's octave (the top register), at velocity 100 and 60.

| `hardness_slope_top` | Overall | Treble | Top | Bias at ff | Bias at mf | ff − mf re the D's |
|---|---|---|---|---|---|---|
| 0.15: the preset of 2026-11-12 | 9.46 | 9.69 | 9.54 | +9.5 | +8.4 | +4.3 |
| 0.10 | 9.20 | 9.58 | 8.62 | +8.9 | +6.0 | +4.7 |
| 0.05 | 9.15 | 9.49 | 8.48 | +8.1 | +4.6 | +5.0 |
| 0 | 9.13 | 9.38 | 8.52 | +8.0 | +4.2 | +5.1 |

A softer top felt comes nearer the D, by the soft blows only: the mf attack loses half of its excess, the ff attack hardly moves (+9.5 → +8.0) and the level's growth from mf to ff, already too large, grows. At ff the top's brightness is not set by the top felt. Not checked on the bench of real grands, where the default's 0.15 was set, and not heard: the preset is unchanged.

**The core in the treble** (2026-11-12): it is not the core either. `core_depth_treble` 0.5 and 1.0 (0 in the preset: the core from the first contact at C8) and `core_hardness` 4 and 2 (8 in the preset) all leave the distance at 9.45–9.46 and every register where it was; the two depths give the same renders to 0.01 dB. A treble hammer does not compress its felt far enough for a u⁷ term to count. `core_hardness` only acts in the low bass, where 2 raises the growth of the 2–4 kHz band from 31.8 to 33.6 dB (the D 35.5) and that of 4–8 kHz from 46.5 to 54.1 (the D 41.4).

**What the top's excess is made of.** Absolute levels of the attack's partials (first 60 ms), model − D, one gain per set taken on A2–A5's peaks:

| | partial 1, ff | partials 2–4, ff | partial 1, mf | partials 2–4, mf |
|---|---|---|---|---|
| C4–B5 | −0.5 | +0.2 | −1.5 | +1.1 |
| C6–B6 | −2.5 | +4.7 | −6.9 | +6.8 |
| C7–C8 | −3.7 | +21.0 | −11.0 | +15.6 |
| C7–C8, `hardness_slope_top` 0.05 | −4.8 | +17.7 | −14.3 | +1.8 |

Up to B5 the model is level with the D. Above, two things add up: the upper partials are far too strong (the D's top octave has next to nothing above its fundamental), and the fundamental is short, more at mf than at ff. A softer top felt takes the mf partials down and the fundamental with them. What sets the partials at ff, and the fundamental's level, is not found: neither the felt's exponent, nor its hardness, nor the core. Left to try, by a stage swap rather than by one more parameter (docs/stage_swap.md): the hammer's force, the string, or the radiation.

## What the board above 3 kHz taught

The treble's distance was put on the statistical plate and was not there. Before a heavy item is built, a cheap test should show that it carries the gap: a swap of one stage with the reference (docs/stage_swap.md), or the gap read in the band the item acts on.

## Data to find

All of it can be measured on a real D or read off dimensioned photographs, and none of it needs new code: rear lengths, case and rim heights, the lid's size and its prop's angles, the frame's shape, the 17th rib's place, the last damper.

## The stage swap in the top two octaves (2026-11-12)

The treble's gap split in two stages against a real concert grand, as docs/stage_swap.md does for the bass: Plath's Steinway (bridge acceleration and dummy head at the player's seat, lid off, every key) against the Concert grand's bridge acceleration and player's microphones, lid off, no room, velocity 105. Every key from C5 to C8, the partials of the attack (0–80 ms) up to 9 kHz, model − real, medians per third octave of the partial's frequency:

| Hz | 630 | 1000 | 1260 | 1600 | 2000 | 2500 | 3200 | 4000 | 5000 | 6300 | 8000 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| bridge acceleration (hammer, string, bridge) | +3.6 | +5.7 | +0.1 | −7.3 | −5.5 | −10.0 | −12.8 | −2.6 | −1.3 | +6.1 | +9.3 |
| bridge to ear (board and radiation) | −5.6 | −6.9 | −0.1 | +1.5 | +4.0 | +3.3 | +1.1 | +0.5 | +4.1 | +5.3 | +3.2 |
| ears | +0.2 | −0.9 | −0.6 | −4.9 | +1.5 | −6.8 | −12.2 | −3.4 | +0.9 | +9.3 | +14.3 |

- **The radiation is not the top's gap.** From 1.2 kHz up the board carries the bridge to the ears within 5 dB of the real one. It is short by 6–7 dB at 630–1000 Hz, which is C6–D6's fundamentals (−14 and −7.5 dB at those two keys): the hole of item 4, seen against a real piano.
- **The gap is at the bridge, and it follows the frequency**: 7 to 13 dB too little between 1.6 and 3.2 kHz, 6 to 9 dB too much above 6 kHz, whichever partial falls there.
- **The top keys' fundamentals are short on top of that**: at the bridge, 12 to 21 dB under the real ones from E7 up, from the first 20 ms (so a matter of excitation, not of decay), while their second partial stands 21 to 26 dB higher re the fundamental than on the real bridge. That is the top octave's excess of partials and its weak fundamental measured against the commercial D, found again against a real grand.
- **Not the board's corner.** With the built board ending at 3 or at 8 kHz the bridge's profile is the same within 2–4 dB (2.5 kHz: −10.5 and −9.4; 8 kHz: +10.7 and +9.3).

So what is left is before the board: the hammer's force on a short string, or the string's hold on the bridge.

**The hammer's contact against the string's period.** The force of the Concert grand's hammers on their strings (`strike_force_dump`, which now takes a preset), its length and its spectrum at the key's partials, re 0 Hz:

| | C6 | G#6 | C7 | E7 | F#7 | G#7 | A#7 | C8 |
|---|---|---|---|---|---|---|---|---|
| contact / period, velocity 105 | 1.08 | 1.15 | 1.23 | 1.33 | 1.40 | 1.45 | 1.50 | 1.64 |
| force at partial 1 (dB) | −7.8 | −12.5 | −16.3 | −22.1 | −26.2 | −30.6 | −30.7 | −28.9 |
| partial 2 re partial 1 (dB) | −27.0 | −28.7 | −17.6 | −10.3 | −7.1 | −5.5 | −9.0 | −14.3 |
| contact / period, velocity 60 | 1.34 | 1.44 | 1.58 | 1.76 | 1.82 | 1.99 | 2.10 | 2.31 |
| force at partial 1, velocity 60 (dB) | −13.5 | −19.1 | −28.2 | −37.0 | −39.8 | −37.5 | −39.1 | −55.3 |

Up the top octave the contact (0.59 ms at C7, 0.39 ms at C8) shortens less than the period, and the fundamental slides into the first dip of the force's spectrum: from E7 up the hammer gives it 14 to 23 dB less than at C6, and the second partial 17 to 22 dB more re the fundamental. These are the bridge's figures against the real grand (12 to 21 dB, 21 to 26 dB). A softer blow stays longer still, 1.8 to 2.3 periods: the fundamental short by 11 dB at mf against the commercial D. A softer felt lengthens the contact too (`hardness_slope_top` 0.05: 1.72 periods at C8), which is why it took the fundamental down with the partials.

**A shorter contact closes the top octave's gap: lighter hammers, as a test.** The contact's length goes as the hammer's mass against the felt's stiffness and the string's push back. `hammer_mass_slope` taken down so that the hammers above C5 are lighter, to 60 % and to 40 % of their mass at C8 (the keys below C5 kept as they are), nothing else changed:

| | the preset | 60 % at C8 | 40 % at C8 |
|---|---|---|---|
| contact / period at C8, velocity 105 | 1.64 | 1.35 | 1.24 |
| the hammer's force at C8's fundamental (dB re 0 Hz) | −28.9 | −20.2 | −13.0 |
| distance to the commercial D, overall / treble / top | 9.46 / 9.69 / 9.54 | 8.82 / 9.23 / 7.31 | 8.70 / 9.21 / 6.74 |
| C7–C8's fundamental re the commercial D's, ff / mf (dB) | −3.7 / −11.0 | +2.0 / −3.8 | +4.1 / −1.1 |
| C7–C8's partials 2–4 re the commercial D's, ff (dB) | +21.0 | +17.1 | +16.4 |
| C7–C8 at the real grand's bridge: partial 1 (dB) | −10.7 | | −4.6 |
| C7–C8 at the real grand's bridge: partial 2 re partial 1 (dB) | +13.1 | | −1.0 |

With a contact of 1.1 to 1.25 periods the top octave's fundamental and its second partial come to the real grand's at the bridge, and the top register makes the largest step of every trial here (9.54 → 6.74). The mechanism holds.

The mass is not the remedy: 40 % is 2.3 g at C8, where Stulov's law gives 5.3 g and RT-425's 5.7 g (Hall: an effective mass of 12 g in the bass to 6 g in the treble; Conklin: the smallest treble hammers 3.5 g).

**And the contact is not too long: it is a real grand's.** Askenfelt and Jansson measured the contact's length on a grand, one key per octave, from pp to ff (*Five lectures on the acoustics of the piano*, "String contact duration and dynamic level", figures 7 and 8, https://www.speech.kth.se/music/5_lectures/askenflt/stricont.html). Read off the two figures, to about 0.05 ms:

| | C6 | C7 | C8 |
|---|---|---|---|
| real grand, ff: ms / periods | 0.85 / 0.9 | 0.55 / 1.1 | 0.45 / 1.7 |
| real grand, mf: ms / periods | 1.1 / 1.2 | 0.65 / 1.25 | 0.5 / 2.15 |
| the Concert grand, velocity 105: ms / periods | 1.04 / 1.08 | 0.59 / 1.23 | 0.39 / 1.64 |
| the Concert grand, velocity 60: ms / periods | 1.28 / 1.34 | 0.76 / 1.58 | 0.55 / 2.31 |

The model's top hammers stay on their strings as long as a real grand's, at both dynamics; their text says as much of the real instrument: in the treble "even the fundamental period is much too short to be optimally excited". So a contact longer than the period is the piano's own, the lighter hammers closed the gap for a wrong reason, and what was written above of something keeping the hammers too long does not hold. Nothing goes into the preset.

What is then left unexplained: with the same contact length, the real grand's bridge carries 12 to 21 dB more of the top keys' fundamentals than the model's. Not the contact's length; perhaps the shape of the force during it (the reflections coming back under a hammer still on the string), the strike point, or how much the bridge yields at 2.6–4.2 kHz, which a bridge signal cannot tell from the force. One real grand on each side, neither of them the same piano, and Plath's accelerometer at its own place on the bridge.

Not explained by the contact: the partials 2–4 of the top octave still 16 dB over the commercial D's in absolute level (they are level with the real grand's at the bridge: that part may be the commercial D's own), the band from 1.6 to 3.2 kHz on the lower keys' partials, the excess above 6 kHz.

`out/ab_swap_top/`: the listening files of six keys (0, M, A, B), `swap_top.py`, `swap_bands.py`, `contact.py`.
