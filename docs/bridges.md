# The bridges: where the bass bridge ends

2026-10-30. Until then the bass bridge carried 8 keys, A0–E1 (`main_bridge_from` 29, F1), a value set with the modal board in phase 4 and never checked. Real grands carry about three times as many.

## What real grands do

The break is where the strings leave the bass bridge for the long (main) bridge. Published values, keys numbered from A0 = 1 (MIDI = number + 20):

| Piano | Length | First key on the long bridge | Keys on the bass bridge | Source |
|---|---|---|---|---|
| Steinway D | 274 cm | F2 (21) | 20 | Overs, scaling charts: "F21 at 183 cm speaking length for the D" |
| Steinway C | 227 cm | F2 (21) | 20 | Overs: "the lowest note on the long bridge F21" |
| Overs 225 | 225 cm | G#2 (24) | 23 | Overs: "G#24 at 154 cm for the Overs 225" |
| Yamaha C5 | 200 cm | about A#2 (26) | about 25 | rec.music.makers.piano survey of Yamaha grands: the C5's break "at Bb 26" (which side of it is not said) |
| Steinway O, early | 180 cm | D3 (30) | 29 | PTG pianotech list: "the real break is at C#29/D30" |

The Steinway D went from a 17- to a 20-note bass section with its 1884 scale (Wikipedia, Steinway D-274). Steinway's published wire scales (steinwayreference.com) put the first plain-wire key at F2 on the B, C and D, so these three carry wound strings on their bass bridges only. On the A and O the plain wire starts at A#2–D3; the bridge break is not given there. Wound strings on the low end of the long bridge are common on medium grands (Piano World and PTG technicians' threads). Fandrich's string-scale notes (PTG Seattle, 2012): there is no ideal number; short scales have more single strings, long ones fewer.

The shorter the piano, the more keys on the bass bridge: 20 on concert grands, 23–26 at 2–2.25 m, about 29 at 1.8 m. The model's scale, `GRAND_2M` (A0 1.45 m speaking, close to a Steinway B's 1.51 m), belongs to the middle group: `main_bridge_from` is now 45 (A2), 24 keys on the bass bridge. It falls between the Overs 225 and the Yamaha C5, and on the first trichord (`trichord_from` 45). The wound strings (`wound_until` 48) go on for three keys, A2–B2, on the main bridge.

Sources:
- Overs Pianos, sample scaling charts: https://www.overspianos.com.au/chrts.html
- Steinway stringing scales: https://www.steinwayreference.com/steinway-stringing-scales/
- Steinway D-274: https://en.wikipedia.org/wiki/Steinway_D-274
- A new C6, and survey of Yamaha grands: https://groups.google.com/g/rec.music.makers.piano/c/-Atu7ntfwu8
- Sneaky Steinway (PTG): https://my.ptg.org/communities/community-home/digestviewer/viewthread?GroupId=1165&MID=500625
- D. Fandrich, Understanding piano string scales (PTG Seattle, 2012): https://seattleptg.org/wp-content/uploads/2024/06/String-Scales.pdf
- Tenor break (Piano World): https://forum.pianoworld.com/ubbthreads.php/topics/1126105/tenor-break.html

## The new geometry

Main bridge: the same rule as before, at equal steps of its range (C8, ≈ G#6, ≈ E5, ≈ C#4, A2). x is the speaking length of the 2 m scale from agraffes 3 cm in front of the board's edge, y the key's place across the string band. Its low end now stops at A2, 19 cm in front of where F1 was.

Bass bridge (default since 2026-10-30): where the IRCAM Steinway D's lies, in proportion to the board (see "Is the geometry a 2 m grand's?" below). Its ends: G#2 at (0.553, 0.562), 55 % of the length and 44 % of the width from the spine, 42 cm from the rim; A0 at (0.867, 0.753), 87 % and 25 %, 23 cm from the tail. It is bowed 3.4 cm towards the tail, as the D's (4 cm on a 0.69 m chord), 2.6 cm per key along the arc. The strings cross the tenor at 12° (A0) to 15° (G#2), the D's being about 10–12°. The old bridge's ends were 15 cm (E1) and 24 cm (A0) from the rim. Its A0 key was also short of its end: it stopped at 7/8 of the segment. Now the highest bass key and A0 fall on the two ends.

A first layout placed the bass bridge by the strings' lengths instead: each string reached its point on the bridge from its key's place at the agraffes with its speaking length to within 2 mm. The bridge then ran straight from (0.635, 0.464) to (0.812, 0.745), with the strings crossing at 13–19°, 21 mm apart. With agraffes only 3 cm in front of the board that layout is no more real than the D's (below), and it sounds worse on the bench.

On the Model page the keyboard now keeps its real width (1.23 m): C8 and the main bridge's lowest key line up with their bridge points, and the bass keys go on beyond.

`bass_bridge_bow` (m, 0.034 by default, 0 for a straight bridge) bends the bass bridge into an arc of a circle through its two ends, with the keys at equal steps along it; positive values bow it towards the tail.

## On the bench

`cargo xtask bench`, close microphones, studio, every run from the same build. The historical score is the RMS of all z; "bass" is the RMS z of the A0–B2 measures. The harmonic counts cover 528 profiles (the OFF/WATCH alerts of docs/bench.md).

| Run | Score | Off | Bass | Middle | Harmonic OFF | A0–E1 OFF | F1–G#2 OFF |
|---|---|---|---|---|---|---|---|
| before: 8 keys, F1 | 1.11 | 11 | 0.81 | 0.91 | 359 | 30 | 74 |
| A2, bass bridge placed by length | 1.12 | 12 | 0.98 | 0.74 | 353 | 36 | 71 |
| same, bow +5 cm | 1.13 | 13 | 1.04 | 0.75 | 352 | 34 | 71 |
| same, bow −5 cm | 1.12 | 13 | 1.00 | 0.74 | 353 | 36 | 70 |
| A2, bass bridge 26→19 cm from the bentside | 1.12 | 12 | 1.00 | 0.76 | 355 | 36 | 69 |
| A2, bass bridge at its old ends | 1.07 | 10 | 0.74 | 0.73 | 346 | 27 | 75 |
| same, bow +5 cm | 1.06 | 10 | 0.65 | 0.73 | 334 | 25 | 64 |
| same, bow −5 cm | 1.10 | 12 | 0.90 | 0.74 | 350 | 33 | 73 |

- **The break helps the middle.** It does so wherever the bass bridge lies: from 0.91 to 0.73–0.76, mostly through the evenness of p1 (z +1.71 → +0.28) and the ff sustain of p3/p4. A2–B3 go from 64 to 57–60 OFF profiles, C4–B5 from 111 to 102–107.
- **The bass depends on how far back the bass bridge lies.** With the bridge placed by the strings' lengths, A0–G#2 lose 2–4 dB in partials 2–8 against p1, at the attack and in the sustain. A0–E1 lose this too, though they did not change bridge. Moving that bridge towards the bentside changes nothing. Its old ends, 8–15 cm further back, give the best bass of all the runs. Bowing them a further 5 cm towards the tail improves it again; bowing them forwards undoes it. With the bridge placed by length, the bow moves nothing.
- **The bow on its own is not the lever.** What it does here is move the bridge's middle back or forth.

The old ends do not fit the strings: from their keys' places at the agraffes, G#2 would reach them over 1.34 m and A0 over 1.57 m, against speaking lengths of 1.18 and 1.45 m. Either the bench rewards a bass bridge further back than the 2 m scale allows, or the scale, the agraffes or the board's size do not fit together. The next section looks at a real grand.

## Is the geometry a 2 m grand's?

Only one real grand's geometry was found in print: the IRCAM Steinway D (274 cm) of Chabassier's thesis (2012, https://pastel.hal.science/pastel-00690351/file/These.pdf). It gives the bridge pins of six keys (Table I.7), their speaking lengths (Table I.2) and the agraffe-to-strike distances (Table I.5), in a frame drawn on a photo of the board (Figure 6, metres, origin at the spine's end of the belly rail). Read off that figure:

| | Steinway D | Model |
|---|---|---|
| Board, length at the spine × width | ≈ 1.99 × 1.46 m | 1.70 × 1.30 m |
| Agraffes in front of the board's front edge | ≈ 30 cm at D♯1, 16 cm at F3, 7 cm at C♯5 | 3 cm for the bridges' layout; 15 cm for the frame (`offset_x` + `agraffe_x`) |
| A0 speaking length | (D♯1: 1.945 m) | 1.45 m |
| Bass bridge, A0 end | ≈ 27 cm from the tail (87 % of the length), 25 % of the width from the spine | placed by length: 32 cm (81 %), 25 % |
| Bass bridge, upper end (E2) | at 55 % of the length, 44 % of the width from the spine | placed by length: at 64 %, 54 % |
| Bass bridge, length and shape | ≈ 0.69 m for 20 keys, 3.4 cm per key, bowed ≈ 4 cm towards the tail | placed by length: 0.49 m for 24 keys, 2.1 cm per key, straight |

A Steinway B (211 cm) has its longest string at 151 cm, agraffe to bridge (M. Steinert & Sons, https://msteinert.com/steinway-model-b): the model's 1.45 m A0 is a 2 m grand's. No board size or bridge position of a 2 m grand was found. If the keyboard, action and rim take the same 75 cm in front of and around the board as on the D, a 211 cm grand's board would be about 1.35–1.4 m long. That is an inference, not a measure. The model's 1.70 m board is then a ≈ 2.4 m grand's, with a 2 m grand's strings. Its main bridge was laid out with agraffes 3 cm in front of the board. A real grand's are 7 cm (treble) to 30 cm (bass) in front, so the model puts the tenor end of its main bridge 10–25 cm too far back for its strings.

The bass bridge at the D's place, scaled to the model's board: ends (0.553, 0.562) and (0.867, 0.753), bow 3.4 cm (the D's 4 cm on a 0.59 m chord):

| Run | Score | Off | Bass | Middle | Harmonic OFF | A0–E1 OFF | F1–G#2 OFF |
|---|---|---|---|---|---|---|---|
| A2, the D's proportions, bowed | 1.10 | 12 | 0.95 | 0.72 | 348 | 33 | 70 |

Its A0 end, as far back as the old one, recovers part of A0–E1. Its upper end, as far forward as on the D, leaves F1–G#2 as dark as with the bridge placed by length. The best bass on the bench, the old ends bowed towards the tail, puts the whole bridge further back than the D has it. The bench's preference is not the real geometry's: something else in the board drives it, which these runs do not identify.

Seen from above (Model page, 3D), the bench's best puts the bass bridge in the back of the board towards the bentside: its G#2 end at 60 % of the width from the spine, the bass strings crossing the whole tenor at 19–21°, and 12–16 cm longer than the scale's. Nothing in it is impossible, but no real grand found looks like it, and a piano built that way would have other bass strings. The D's proportions look like the D, with strings at 12–15°.

**The default is the D's proportions, bowed** (1.10 against 1.11 before the break was moved): the realistic geometry, not the bench's best. What is left open:
- the bass on the bench (0.95 against 0.81 before): A0–G#2 lack 2–4 dB of partials 2–8 against p1, and the bridge's place only hides it;
- the board, 1.70 m long, a ≈ 2.4 m grand's under a 2 m grand's strings;
- the agraffes, 3 cm in front of the board for the bridges' layout and 15 cm for the frame, where a real grand's are 7–30 cm.
