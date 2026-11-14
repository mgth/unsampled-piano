# What still separates the model from the D

Status as of 2026-11-14. A follow-up list for the Concert grand preset: what in the model is not the Steinway D's physics yet, ranked by how much each is likely to move the sound. The figures are those of docs/steinway_d.md, docs/real_board.md, docs/release.md and docs/bass_gaps.md. The ranking is a judgment from those figures, not a measured sensitivity: the board above 3 kHz, ranked first on 2026-11-11, turned out neutral once done, and item 4's gaps were for the most part not the radiation's (docs/case_radiation.md).

The preset changed on 2026-11-14 (docs/bass_gaps.md): the felt four times harder (`hardness` 1.2) with the top's hardening taken down to keep the top keys where they were (`hardness_slope_top` 0.035). Bench of eight real grands 1.98 → 1.84 (bass 1.39, middle 1.31, treble 1.93, top 2.64); distance to the commercial D 9.46 → 9.53 (bass 8.71, middle 10.08, treble 10.34, top 8.88). The trials of the sections below were made on the preset before it (`hardness` 0.3, `hardness_slope_top` 0.15) and their tables give its figures.

It changed again the same day: the board as the D is drawn (17 ribs to the rim, the bridge in one piece, its real heights, the strings' hold on the bridge's top; docs/bass_gaps.md, "The bass bridge's coupling"). Bench 1.84 → 1.75 (bass 1.41, middle 1.38, treble 1.66, top 2.50); distance to the commercial D 9.53 → 8.60 (bass 8.38, middle 9.21, treble 9.04, top 7.67). Every figure below that depends on the board is the former board's. The list keeps its numbers: item 1 is done.

And a third time: the top hammers nearer the capo bar (`strike_ratio_top` 0.05; "The strike point in the top octave" below). Bench 1.75 → 1.82 (treble 1.61, top 2.75); distance to the commercial D 8.60 → 8.23 (treble 8.82, top 6.19).

- **Wrong**: the right physics is known and the model does not follow it.
- **Estimated**: the value is missing from the sources used.

## Open, ranked

| # | Difference | Likely effect on the sound | To implement | Kind |
|---|---|---|---|---|
| 2 | The top octave's fundamentals: 7 to 26 dB short at the bridge from E7 up against a real grand (17 on average), the second partial 19 dB too strong re the fundamental. Against the commercial D: fundamental −6.4 dB at ff and −13.9 at mf, partials 2–4 +15 dB | Strong from E7 up | **Cause found (2026-11-14): the strike point.** The preset keeps the hammers at 0.12 of the string up to C8 (RT-425's rule, not a measure of the top hammers); a real grand's C8 hammer is set between 1/12 and 1/17 (Conklin). `strike_ratio_top` 0.05 (1/14 at C8) halves the gap at the bridge and brings the top's attack to the eight real grands'. In the preset since it was played at the keyboard ("The strike point in the top octave" below). Left after it: 8 dB of fundamental at the bridge, not told apart from the bridge's mobility or the unknown blow | A parameter | Wrong |
| 3 | The felt's law. Taken up on 2026-11-15 (docs/felt_law.md): the model's law is Stulov's, measured on new unvoiced hammers at 0.5–0.85 m/s, and its exponent of 3.7–5.0 belongs to a law with a rate term (its loading branch reads 2.8–3.3 up to C5); the 2.3–3.5 are hysteresis-free readings of voiced hammers, and putting them into the model's law counted the hysteresis twice. A voiced D's own hammers (Russell and Rossing) are slower and far less sensitive to speed than the model's on a rigid target; their law, put on the model's strings, scores as the preset does (bench 1.82, bass 1.31, middle 1.55). Against the eight real grands two faults are left: the bass's mf attack has no 4–8 kHz (z −2.8), and the treble's mf attack is 7–9 dB too bright. The core is idle since the felt was hardened | Medium: bench 1.77 to 1.82 over seven laws | Three candidates of existing parameters wait to be played (a lower exponent in the bass; with it a softer felt above C4; the D's measured hammers without hysteresis): none is a clear gain by measure. What the present law stands in for in the middle and the treble (the pulse on a real string, the felt's depth, the shank) is not found | Wrong in the bass's exponent; elsewhere a measured law of other hammers |
| 4 | The radiation from the bridge to the listener. Measured again on the board as drawn (docs/case_radiation.md, 2026-11-14): the 6–7 dB lack at 630–1000 Hz is gone with that board; the fundamentals below 38 Hz are under the real recording's noise floor; at 100–250 Hz and at 1.6–2.5 kHz the bridge and the transfer err in opposite ways (+5 and −6 dB; −5 and +8 dB) and the ears are within 4 dB of a real grand's from 100 Hz to 3.2 kHz. Left at the ears, lid off: +5 dB at 400 Hz, −8 at 4 kHz, +6 to +8 at 6.3–8 kHz | Small as radiation. The pairs are the bridge's own motion: its mobility above 1.5 kHz (with item 2) and the board's modes at the bass bridge at 117–166 Hz | **One error corrected**: with the lid off the case's field sent the board's power out a second time; it now takes the share that meets the rim (16 %). Lid on, nothing changes. The real case's acoustics (air modes, frame, strings) are not called for by any measure here | Wrong (simplification), mostly not where it was looked for |
| 5 | The unisons of F1–B2 beat, a real grand's do not | Medium in the bass | `unison.width_mid` 0.2 does it in the bass and changes the middle and the treble too: a width per register, and to be heard | Estimated |
| 6 | The strings' own loss above 1.2 kHz in the bass: two to three times too high by measure (`losses.b2`). Lowered, the strings buzz at the keyboard, at 6.0e-8 and at 1.0e-7, on every felt: the measure and the ear disagree below A3 | Medium, C1–B2 | Not until a measure agrees with the keyboard; any change of it is to be played before it is kept | Wrong by measure, refused by ear |
| 7 | The last damper at G6 | Strong at the top: its 1 kHz band in the sustain falls from −42 to −110 dB, real grands −43 | A parameter | Estimated (the commercial D's setting) |
| 8 | The dampers' release: 6.1 dB rms from the commercial D's key-ups, 4.3 from a Disklavier grand's. On the pedal bench, with `damper.horizontal` 0.3: halo 3.6, hold 4.8, after 11.5 dB rms; at CC64 = 96 the treble's "after" is 15.7 dB over the recording, the middle's 2.0 short. The half pedal (CC64 = 64) damped 3 dB less than recorded in the middle and the treble before `horizontal` 0.3, not read again since | Medium, on every key-up and pedal change; the treble under the pedal most | Parameters and the felt's law of contact | Wrong in part, the felt's own values estimated |
| 9 | Case height, lid size, prop angles | Medium: the openings set the case field's level | Parameters exist | Estimated |
| 10 | Crown and bearing: a foundation term set on four modes of another board. With the D's drawn ribs it gives the reference grand's unstrung resonances (57, 79, 110 Hz against 56, 80, 114): less of a gap than it looked | Small to medium in the bass: places the board's first modes | Heavy: a prestressed shell | Both |
| 11 | The strings' rear lengths (bridge to hitch pin) | Small to medium in the bass: on the board as drawn the first resonance moves by 1 to 3 Hz for 0.6 of the bass's estimate (62 to 73 Hz on the former one) | A table, once measured | Estimated |
| 12 | Duplex resonances | Small to medium, in the treble | Medium | Absent; lengths missing |
| 12b | The board's outline: some 8 cm wider at the bentside than the D's drawn inner rim (found while placing the ribs, 2026-11-14) | Unknown; it moves every bridge point's distance to the rim | A redraw of the outline from the dimensioned drawing | Estimated |
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
| The bass bridge between 60 and 140 Hz: the strings lost two to three times too much from 60 to 120 Hz (2026-11-14, item 1) | The cause was the board, built from a simplified mesh (16 ribs short of the rim, the bass front corner bare, the bridge cut in two): six modes under 140 Hz at the bass bridge where a real board has three. Now the D's 17 drawn ribs, feathered, the bridge in one piece with its real heights, and the strings' hold on the bridge's top (`strings_grip`), kept after the keyboard (docs/bass_gaps.md) | First resonances 78, 100, 137 Hz against a real grand's 74, 98, 136; partials 2–4 of A0–E1 at 4.3 dB/s against 4.3 (10.3 before), the fundamentals of C2–B2 at 9.3 against 9.6 (18.4). Bench 1.84 → 1.75, distance to the commercial D 9.53 → 8.60. It uncovers item 4's lack below 90 Hz |
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

The trials of this section were read against the commercial D alone. docs/felt_law.md (2026-11-15) takes them up against the real pianos, with the literature and the reason each one failed: the real exponent alone counted the hysteresis twice, and RT-425's law has a loss time longer than the bass's contact and a top felt half as quick as a measured D's.

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

`out/ab_swap_top/` (worktree `branch-cleanup-f317cd`): the listening files of six keys (0, M, A, B), `swap_top.py`, `swap_bands.py`, `contact.py`.

## The strike point in the top octave (2026-11-14, later)

Item 2 taken up on the board as the D is drawn (main after the bass bridge's work). Scripts and renders in `out/` of the worktree `top-octave-fundamentals-7c83eb` (`tools/`, `cat88/`, `full_*.txt`, `literature.md`).

**The gap did not move with the board.** Stage swap against Plath's grand, velocity 105, attack 0–80 ms: from 1.2 kHz up the board still carries the bridge to the ears within 5 dB; at the bridge the fundamentals fall 10 to 24 dB short in the 3.2 and 4 kHz bands. Key by key from E7 to C8 the bridge's fundamental is 7 to 26 dB under the real one (17 on average) and its second partial 19 dB too strong re the fundamental. Against the commercial D, C7–C8: fundamental −6.4 dB at ff and −13.9 at mf, partials 2–4 +15.0 dB. Distance 8.60 (top 7.67), bench 1.75 (top 2.50).

**The measure, questioned.**

- Plath's accelerometer (PCB 352C23, 0.2 g, flat to 10 kHz) sits on the bridge at the played string's termination, normal to the board (ISMA 2019 paper); the model's signal is the bridge's normal velocity averaged along a few keys. At 2.6–4.2 kHz that is a fair pair.
- The blow is not known: the keys were struck by hand with an impact-hammer head, "forte"; velocity 105 was set on the bass's sustain. An absolute level in the top octave carries that doubt; the second partial re the fundamental does not.
- The real piano against itself: five takes agree within 1 to 3 dB, but its two states (new, and a year later) differ by up to 9 dB on a single top key's fundamental (E7 35.5 and 44.1 dB, G#7 36.3 and 43.6). The figures here are against the mean of the two states.
- A second real reference agrees on the shape and softens the level. The bench's eight grands through their close microphones, ff, E7–C8, attack: the second partial lies 21 to 34 dB under the fundamental (the commercial D 34, the preset 14), and the fundamental 6 to 21 dB under the set's own C4–B5 fundamentals (12 on average, the commercial D 10, the preset 17). So at microphones the preset's fundamental is some 5 dB short, not 17: part of the bridge's figure is the measure's. The excess of the second partial is in every reference.

**What the literature says** (`out/literature.md`; the JASA papers themselves could not be opened, so Hall, Askenfelt and Jansson, Chaigne and Askenfelt are second-hand there). Conklin (*Five lectures*, "Where should the hammer hit the string?"): the C8 hammer is placed for each piano at the factory for the loudest tone, usually between 1/12 and 1/17 of the speaking length. Kartofelev and Stulov take 1/16.5 at F#6. Giordano's and Ege and Boutillon's bridge impedances fall from 1–2·10³ kg/s to 200–300 kg/s above 2.5 kHz, on uprights; nothing is published on a concert grand's treble bridge above 3.2 kHz.

RT-425's strike point, which the preset follows (`strike_ratio_top` 0), is 0.12 of the length at every key up to B7, to the millimetre: a rule of that report, like its laws for the hammers, not a measure of the top hammers' line.

**Why the strike point decides.** At C8 the head (4.1 g, the shank's 30 % apart) weighs twenty times its three strings: the contact is a mass on the strings' static stiffness, T/(β(1−β)L) each, and its length follows √β. At β = 0.12 that half period is 0.35 ms, the 0.39 ms measured, and the force is close to a half sine whose spectrum has fallen 22 to 31 dB at the fundamental, with the second partial only 6 to 16 dB further down. Nearer the capo bar the strings are stiffer under the hammer, the pulse shorter, and the fundamental comes out of the fall; the mode's own weight sin(πβ) loses 3 dB at most.

**One variable at a time**, the bridge against the real grand, E7–C8 (six keys for the trials, eleven for the three strike points):

| | fundamental re the real bridge (dB) | second partial re the fundamental, re the real (dB) |
|---|---|---|
| the preset | −17.1 | +18.8 |
| `strike_ratio_top` 0.035 (1/12 at C8) | −11.5 | +13.0 |
| `strike_ratio_top` 0.05 (1/14) | −8.5 | +6.1 |
| `strike_ratio_top` 0.06 (1/17) | −7.0 | +3.2 |
| `contact_width_scale` 0.25 / 0.5 / 2 | −17.0 / −16.9 / −14.8 | +17.0 / +17.0 / +16.8 |
| `shank_share` 0 / 0.5 (0.3 in the preset) | −12.5 / −15.2 | +10.0 / +15.2 |

- **The contact's width does nothing** (2 to 8 mm at C8).
- **The shank moves the dip from key to key and closes nothing**: without it C7 and E7 lose 6 dB while F#7 gains 13.
- **The string's stiffness and its front termination were not tried**: the modes are those of a stiff string on simple supports (B 0.023 at C8), and a clamped end would need other mode shapes.
- **The bridge's mobility cannot be settled.** The model's own, at the top keys' bridge points (`bridge_mobility`, piano-render's example): 0.4 to 1.5·10⁻³ s/kg from 2.5 to 4 kHz, that is 700 to 2500 kg/s where the uprights of the literature give 200–300. If a concert grand's treble bridge yields as theirs, the 7 to 8 dB left after the strike point are there; no measure says so.

**The strike point, on every measure.** Only the keys above C6 change (the law starts there): bass and middle are the same renders.

| | the preset | 0.035 | 0.05 | 0.06 |
|---|---|---|---|---|
| strike point at C7 / C8 | 0.12 / 0.12 | 0.1025 / 0.085 | 0.095 / 0.07 | 0.09 / 0.06 |
| distance to the commercial D, overall / treble / top | 8.60 / 9.04 / 7.67 | 8.32 / 8.90 / 6.56 | 8.23 / 8.82 / 6.19 | 8.19 / 8.79 / 6.00 |
| C7–C8's fundamental re the commercial D's, ff / mf (dB) | −6.4 / −13.9 | −1.1 / −10.3 | +1.6 / −7.9 | +2.8 / −6.1 |
| C7–C8's partials 2–4 re the commercial D's, ff (dB) | +15.0 | +13.4 | +10.9 | +11.0 |
| close microphones, E7–C8, ff: second partial re the fundamental (real grands −21 to −34) | −13.8 | −18.7 | −25.0 | −27.6 |
| the same, fundamental re the set's C4–B5 (real grands −12, from −6 to −21) | −17.0 | −12.0 | −9.0 | −7.5 |
| bench, overall / treble / top | 1.75 / 1.66 / 2.50 | 1.77 | 1.82 / 1.61 / 2.75 | |

With 0.05 the contact stays a real grand's: 0.55 ms at C7 and 0.33 at C8 at velocity 105 (Askenfelt and Jansson 0.55 and 0.45, read to 0.05 ms), 0.71 and 0.52 at velocity 60 (0.65 and 0.5).

**The bench of real grands, top register, 0.05.** Everything the item is about comes to the real pianos': the attack's second partial −14.2 → −27.4 dB (theirs −27.0), its third −35.3 → −47.5 (−45.9), 4–8 kHz in the attack −10.8 → −20.3 (−19.6), the centroid 1.15 → 1.03 (1.03), the top's level against the keyboard −11.0 → −6.4 (−1.4). The register's score still rises, 2.50 → 2.75, on measures that were off before and go further: the 1 kHz band of the sustain (−112 → −122 dB, theirs −43: the last damper, item 7), the 2 kHz band (−32 → −39, theirs −29), the sustain's second partial after half a second (−45 → −55, theirs −37: it decays too fast, at either strike point), and ff − pp (17.0 → 23.1 dB, one source at 17.8). To be read with that: the attack is nearer, the sustain between and under the partials is not this item's and looks worse for it.

**Kept**: `strike_ratio_top` 0.05, the engine's default, 1/14 at C8, inside Conklin's range. No code changes. Played at the keyboard by the user and taken into the preset the same day, without a listening remark. The trial's files: `out/keyboard/instruments/Top strike trial/` (the preset and the trial), listening page `out/ab_top_strike/index.html` (C6–C8, the real grand at the player's seat and the commercial D through close microphones).

**Not verified.** The D's own strike line at the top (no measured source found; RT-425's is a rule). The law between C6 and C8 is a straight line in the ratio, the real one follows the hammers' line and the strings' lengths. What is left at the bridge after the change (8 dB of fundamental, G#7–B7 most) between the bridge's mobility at 2.6–4.2 kHz, the unknown blow and the real piano's own 9 dB from one state to the other. The mf fundamental stays 8 dB under the commercial D's. The front termination. The upper partials' excess against the commercial D (+11 dB on partials 2–4, at the real grand's level at the bridge) is still there.
