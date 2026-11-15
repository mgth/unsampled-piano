# The unisons of the bass

Status as of 2026-11-15. Item 5 of docs/steinway_d_gaps.md: the Concert grand's bichords and trichords of F1–B2 beat where a real concert grand's do not. Measured again on main after the board as drawn, the top's strike point, the case's field and the felt's law (449871a). The cause is found; no single width closes the gap, because the beats were hiding another one; a parameter is added. Played at the keyboard the same day, the narrow unison with half the strings' frequency loss up to C3 (trial C of section 5) was preferred and did not buzz: it is in the preset. The renders, variants, scripts and the literature notes are in this session's `out/` (not in the repository): `out/tools/`, `out/presets/`, `out/literature.md`, `out/m_*.txt`.

## What comes out

- **The gap is there on the rebuilt board, and it is firm.** At the bridge, 94 % of the model's partials of F1–B2 between 400 and 3500 Hz dip by more than 3 dB under their own line in the first 1.6 s (median dip 11 dB); on the real grand 34 %, and 45 % in its other state a year earlier, another tuning (median dips 1.9 and 2.6 dB). The real grand's single strings, A0–E1, do as much (2 to 45 % by key, 71 % on one): most of what the real unisons show is not a unison's.
- **The cause is the mistuning itself.** `unison.width_mid` puts the extreme strings 1 cent apart, the same at every partial, so partial k of a bichord beats at k·f1/1731 Hz. Above 300 Hz that is two to seven times what the bridge's coupling can hold: the strings do not lock, each keeps its frequency, and every partial passes through a null. Not the coupling (the bass bridge's mobility is the measured one), not the hammer's uneven blow (no effect), not the inharmonicity (the strings of a key share it in the model).
- **A real grand's bass unisons are within about 0.15 cent, or held together.** Its partials at 400–1200 Hz show no null in the five seconds of the recordings, in either tuning. The shallow dips of its first second and a half are those of the model at 0.25–0.4 cent, but at that width the model's partials go into a null between 2.5 and 4 s that the real ones do not have.
- **No width gives the real decay.** Narrowed, the model's unison decays at one rate, 5.7 to 7.5 dB/s at 400–1200 Hz, where the real ones lose 3.6 to 6.4 and slow down after a second. At 2.5–3 s that band is 5 dB under the real grand's at 0.1 cent and 9 to 11 dB under at 0.3 cent. The preset is level with the real grand there (within 3 dB), by the return of its beats.
- **What the beats were hiding is the strings' own loss**, item 6 of docs/steinway_d_gaps.md: at 1200–3500 Hz a narrowed unison loses 10.5 dB/s against 4.5 to 5. Items 5 and 6 are one knot.
- **In the code**: `unison.width_bass` and `unison.width_bass_until`, a width of their own for the low keys, 0 by default (every render unchanged, bit for bit). Three trials were put in the instrument's list and played.
- **In the preset, after the keyboard**: `unison.width_bass = 0.1` with `losses.b2 = 1.0e-7` up to C3, back to 2.0e-7 by C4 (`b2_top`, `b2_top_from = 60`). Found better than the preset, and the buzz that refused the lower loss twice did not come back: it had been heard with the unisons a cent wide. Bench 1.80 to 1.73 (bass and middle 1.41 and 1.40 to 1.21), distance to the commercial D 8.02 to 8.06.
- **The middle is narrower on the real grand too** (its partials of C3–B4 dip as the model's at 0.4–0.5 cent), but 0.5 cent at C4 is refused by the measures: the bench goes from 1.80 to 1.89 and the sustain of C4–B5 falls 5 to 10 dB under the real grand's.

## 1. The gap, measured again

Method. Plath's concert grand (bridge acceleration and dummy head at the player's seat, lid off, every key, one dynamic), take 2 of its two states, PROD08 and PROD07, a year apart: two tunings of one instrument. The model through piano-render's `bridge_signal` at velocity 105, as in docs/bass_gaps.md. `out/tools/uni.py` follows each partial between 400 and 3500 Hz that stands 8 dB over the floor, in windows of six periods of the key (16 hops per window), and reads the deepest dip of its envelope under its own straight line over 0.15–1.6 s, and its decay over 0.15–1 s and 1–2.5 s. The windows are half as long as those of docs/bass_gaps.md's `dips.py`: the real grand's dips come out deeper than its −0.8 to −2.3 dB.

At the bridge: median dip (dB), share of the partials dipping by more than 3 dB, decay of the partials at 400–1200 Hz early / late (dB/s).

| | F1–A1 (two strings) | A#1–E2 (three, wound) | F2–B2 (three, plain) |
|---|---|---|---|
| the real grand | −2.3, 36 %, 5.3 / 4.2 | −2.6, 42 %, 9.1 / 3.4 | −0.9, 22 %, 7.5 / 6.4 |
| the real grand a year earlier | −1.7, 34 %, 5.4 / 4.0 | −2.8, 47 %, 7.9 / 2.5 | −3.7, 54 %, 10.9 / 5.3 |
| the preset | −12.2, 98 %, 19.9 / −4.2 | −11.0, 91 %, 17.5 / −0.1 | −9.2, 94 %, 19.5 / −0.2 |

At the ears the real grand dips more (−6.3, −6.3, −2.8 dB; 70, 71, 46 %) and the model the same as at its bridge (−12.2, −11.1, −9.2; 98, 91, 93 %): the model's two stages carry one envelope, the real ones do not.

**How firm.** Another take of the same state gives the same figures to 0.1 dB. The two states differ key by key (A1: 20 % of its partials dip in one, 87 % in the other; C2: 12 and 82 %; G2: 7 and 90 %): each tuning has its own keys that beat, and neither comes near the model's 91–98 % on every key. The real single strings are the measure's floor: A0–E1 dip by 0.5 to 2.5 dB, 2 to 45 % of their partials by more than 3 dB (A0 in the earlier state 71 %), where the model's single strings read 0 to 8 %. One instrument, two tunings, one dynamic.

**The middle**, the same reading with `out/tools/eqw.py`: the width, the same on every key, at which the model's median dip is the real key's (partials at 250–9000 Hz), median per register for the two states.

| | F1–A1 | A#1–E2 | F2–B2 | C3–B3 | C4–B4 | C5–B5 | above |
|---|---|---|---|---|---|---|---|
| width that gives the real dips (cents) | 0.30, 0.25 | 0.31, 0.34 | 0.25, 0.41 | 0.53, 0.48 | 0.41, 0.54 | 0.33, 0.48 | not readable |

Flat in cents across F1–B2, whose fundamentals go from 44 to 123 Hz: the unit is the cent, not the hertz. Above C5 too few partials last long enough for the reading. This is an upper reading of the width, since the single strings dip too.

**Baselines** (main, the renders of docs/felt_law.md's kept candidate, checked bit for bit on four notes against this build): bench of eight real grands 1.80 (bass 1.41, middle 1.40, treble 1.54, top 2.74); distance to the commercial D, 88 keys, 8.02 (bass 8.37, middle 9.13, treble 8.34, top 5.85).

## 2. What the literature gives

Notes with every statement marked read or second-hand, and the sources' addresses: `out/literature.md`. In short:

- **No published table of the mistuning of a tuned concert grand's unisons by register was found, and nothing at all on the wound bichords and trichords of the bass.**
- Kirk (JASA 1959; abstract only, the paper is behind a paywall): listeners prefer 1 to 2 cents of maximum deviation among the three strings, trained listeners less, in close agreement with what artist tuners leave. The registers he tested could not be checked.
- Tuners' own figures, second-hand: 0.1 to 0.5 cent; the examination tolerance of their guild about 1 cent, C3–B4 only.
- Weinreich (the KTH lecture and the Scientific American article, read; JASA 1977 not opened): for the middle range and a resistive bridge, no beats below about 0.3 Hz of mistuning (1.7 cents at D#4), one null and then a beatless aftersound whose level rises with the mistuning. That a tuner sets the mistuning to even out the aftersound is given as a hypothesis, never tested. No figure for the bass.
- From his figures the mistuning a bridge holds is about the single string's bridge decay in dB/s over 27.3, in hertz. Woodhouse (2021): the lowest register is where the board's modes are isolated, and two strings on one such mode repel and beat rather than lock.
- Cheng, Dixon and Mauch (2015): on the recorded pianos of a public database, beats "appear extensively" in the partials of G#1–F3; no depth, no rate.
- A single wound string beats on its own: Tanaka and others measured 0.07 Hz between the two polarisations of an E1 string. Mismatched wound pairs that beat are reported by tuners even on new concert grands; no figure on the inharmonicity of the two strings of a pair is published.

So a concert grand in tune has, as far as it is written: 0.5 to 2 cents in the middle by Kirk's listeners, 0.1 to 0.5 by tuners, and nothing measured in the bass. The real grand here is the only measure of the bass at hand.

## 3. The model's unison, and each cause with its test

**What the parameters set.** `unison.width(key)` is the distance in cents between the extreme strings: `width_mid` up to C4, then towards `width_treble` at C8. `detune(key)` places the strings in it by a fixed pattern of the key (a bichord 0.85 to 1.15 of the width apart, a trichord's middle string anywhere in the middle 60 %): fixed, not drawn again at each note. A string's mistuning multiplies its fundamental, so every partial is off by the same cents and by k times the hertz. The strings of a key share their inharmonicity, their loss, and their coupling to the bridge, ε = (T/L)·Y at the key's bridge point, at each partial; each has a sideways polarisation 0.3 cent off, coupled thirty times less. The coupled modes are solved once per key (`unison.rs`, after Weinreich). `hit_spread` and `height_spread` make the blow uneven from string to string.

**The mistuning against the coupling**, `unison_lock` (piano-host's example): F1, two strings 0.98 cent apart.

| partial | 5 | 10 | 16 | 22 | 30 | 40 | 50 | 64 |
|---|---|---|---|---|---|---|---|---|
| Hz | 218 | 436 | 701 | 970 | 1339 | 1820 | 2330 | 3102 |
| mistuning (Hz) | 0.12 | 0.25 | 0.40 | 0.55 | 0.76 | 1.03 | 1.31 | 1.75 |
| what the coupling holds, \|ε\|/π (Hz) | 0.09 | 0.12 | 0.07 | 0.11 | 0.13 | 0.18 | 0.21 | 0.28 |
| the string's own loss, one string's bridge loss (dB/s) | 0.7, 1.8 | 1.3, 3.2 | 2.2, 1.4 | 3.4, 2.3 | 5.5, 1.5 | 8.9, 2.5 | 13.4, 2.3 | 21.9, 2.3 |

From 300 Hz up the mistuning is two to seven times the coupling: the engine's two modes sit one mistuning apart (0.40 Hz at partial 16, 1.03 at partial 40) and decay at nearly one rate. In cents the coupling holds 0.15 to 0.5 cent between 400 and 3000 Hz.

**One test per cause**, each the full preset with one line changed, F1–B2 at the bridge: median dip (dB) and share of the partials dipping by more than 3 dB.

| cause | test | F1–A1 | A#1–E2 | F2–B2 | verdict |
|---|---|---|---|---|---|
| | the preset | −12.2, 98 % | −11.0, 91 % | −9.2, 94 % | |
| the mistuning | `width_mid` 0.5 | −6.9, 74 % | −4.2, 69 % | −5.0, 71 % | |
| | 0.3 | −2.0, 40 % | −1.6, 23 % | −1.6, 27 % | the real dips |
| | 0.2 | −1.0, 17 % | −0.8, 4 % | −0.8, 8 % | |
| | 0.1 | −0.4, 6 % | −0.3, 3 % | −0.4, 1 % | **the cause** |
| the hammer's uneven blow | `hit_spread` 0 | −10.4, 99 % | −11.3, 91 % | −10.3, 95 % | no |
| | `height_spread` 0 | −13.8, 98 % | −10.9, 92 % | −10.7, 95 % | no |
| too little coupling | none needed | | | | no: see below |
| the inharmonicity of a wound pair | none possible | | | | not in the model: its strings share one |

- **The coupling is not too weak.** To hold 1 cent the bridge would have to yield three to ten times more from 400 to 3500 Hz. Its mean mobility at Plath's fifteen points is 0.76·10⁻³ s/kg against 0.7 measured (docs/bass_gaps.md), and the real single strings bound it from the other side: a string cannot lose more to the bridge than it loses in all, 3 to 8 dB/s at 400–1200 Hz, which holds 0.5 cent at most at 800 Hz and 0.3 at 2 kHz. A real grand's bridge cannot lock strings 1 cent apart either.
- **The uneven blow** changes nothing at 1 cent, and nothing at 0.05 to 0.2 cent with `hit_spread` 0.6 or 1 (the band's level at 2.5 s within 0.7 dB).
- **The reactive part of the coupling** is as large as the resistive one above 700 Hz in the model. Weinreich's two coupled strings computed with the engine's own ε and losses (`out/tools/pair.py`) give the engine's envelopes, and the same within 2 dB with the reactive part removed or doubled.

## 4. Why a narrower unison is not enough

The band 400–1200 Hz at the bridge, by windows of half a second, re its level at 0.15–0.5 s (dB, mean over the keys; `out/tools/envt.py`). The real recordings last 5 s, the model's renders 3.2 s.

| from (s) | 0.5 | 1.0 | 1.5 | 2.0 | 2.5 | 3.0 | 4.0 |
|---|---|---|---|---|---|---|---|
| **A0–E1, one string**: the real grand | −1.4 | −4.0 | −6.1 | −8.1 | −10.1 | −11.8 | −15.4 |
| the preset | −0.9 | −3.5 | −6.2 | −8.6 | −10.9 | | |
| **F1–A1**: the real grand | −1.4 | −3.4 | −5.1 | −7.1 | −8.0 | −9.4 | −12.5 |
| a year earlier | −0.5 | −2.3 | −4.6 | −6.4 | −7.8 | −9.0 | −11.6 |
| the preset, 1 cent | −5.1 | −8.9 | −6.9 | −6.8 | −7.7 | | |
| 0.5 cent | −2.2 | −7.4 | −12.9 | −15.1 | −14.2 | | |
| 0.3 cent | −1.5 | −5.2 | −9.2 | −13.8 | −18.1 | | |
| 0.1 cent | −1.2 | −4.2 | −7.0 | −10.1 | −13.0 | | |
| 0.05 cent | −1.2 | −4.1 | −6.9 | −9.8 | −12.5 | | |
| **A#1–E2**: the real grand | −2.6 | −6.9 | −8.8 | −9.2 | −10.1 | −11.7 | −13.2 |
| a year earlier | −2.3 | −5.1 | −6.0 | −7.4 | −9.2 | −10.8 | −12.3 |
| the preset, 1 cent | −4.9 | −12.2 | −11.8 | −12.5 | −12.8 | | |
| 0.3 cent | −2.0 | −6.1 | −10.6 | −15.4 | −20.2 | | |
| 0.1 cent | −1.7 | −5.1 | −8.5 | −11.8 | −15.0 | | |
| **F2–B2**: the real grand | −1.9 | −5.6 | −8.9 | −12.0 | −14.4 | −16.7 | −20.6 |
| a year earlier | −2.7 | −8.1 | −10.5 | −11.9 | −13.1 | −13.8 | −16.0 |
| the preset, 1 cent | −4.9 | −12.0 | −12.8 | −12.6 | −12.6 | | |
| 0.3 cent | −2.3 | −6.8 | −11.7 | −16.7 | −21.0 | | |
| 0.1 cent | −2.1 | −5.9 | −9.8 | −13.4 | −17.1 | | |

- **The single strings are right**: the model's follow the real ones within 1 dB over 2.5 s.
- **The real unisons fall slowly and without a null**: the bichords at 3.6 dB/s, slower than the single strings below them (4.1); the wound trichords at 8 dB/s for a second, then at 2. Nothing in five seconds looks like the 0.3 cent of section 1: at that width the model's band is on its way into a null from 1.5 s on, 10 dB under the real one at 2.5 s. A null not yet come at 5 s on a partial at 1200 Hz means less than 0.15 cent, or strings that do not part.
- **The preset is right late and wrong early.** After 1.5 s its partials are coming back from their nulls and the band sits within 3 dB of the real one; between 0.5 and 1.5 s it is 3 to 6 dB under.
- **Narrowed, the model's unison falls at one rate, too fast**: 5.7 dB/s for the bichords against 3.6, 6.5 for the wound trichords against the real 8-then-2, 7.5 for F2–B2 against 6.4 then 4. No width between 0.02 and 0.5 cent gives a slow second part, uneven blow or not.

**Where the too-fast decay comes from.** In the model a string's own loss at 400–1200 Hz (1.3 to 4.4 dB/s) is as large as what the bridge takes or larger, so the modes of a pair decay at nearly one rate (4.8 and 4.1 dB/s for F1's partial 18) and there is no slow mode for an aftersound to live in; above 1.2 kHz the own loss is all of it. The same windows at 1200–3500 Hz:

| at 2.5–3 s (dB) | F1–A1 | A#1–E2 | F2–B2 |
|---|---|---|---|
| the real grand, its two states | −9.9, −9.6 | −10.8, −9.4 | −16.8, −14.3 |
| the preset | −15.8 | −16.7 | −14.7 |
| `width_bass` 0.1 | −24.0 | −24.8 | −25.5 |
| the same, `losses.b2` 1.0e-7 below C3 | −19.9 | −20.4 | −21.0 |
| the same, 6.0e-8 | −18.0 | −18.2 | −18.7 |

That is `losses.b2`, item 6: two to three times too high in the bass by measure, lowered once and refused at the keyboard as strings that buzz. Lowered with the narrow unison it closes a third to a half of what is missing (at 400–1200 Hz one decibel of five). The beats of a 1 cent unison were holding the late sound up where the strings' own loss takes it down: the two items compensate, as the bridge and the transfer do in docs/bass_gaps.md.

One thing follows that is not tested: the buzz was heard with the unisons at 1 cent, when a lower loss let partials at 2 to 4 kHz last while beating at 1 to 2 Hz. It was never played with the unisons narrowed.

## 5. The change, and what is not proposed

**In the code**: `unison.width_bass` (cents) and `unison.width_bass_until` (a key, B2 by default): the width of the low keys, coming back to `width_mid` at C4 key by key. 0, the default, leaves `width_mid` down to the lowest unison, and every preset renders as before, bit for bit (four notes of the Concert grand compared with the renders of the build before; a unit test holds the law). Help text, and the page of the other `unison.` fields.

**Candidates**, each the full preset with one or two lines, measured on everything of section 1.

| | the preset | A: `width_bass` 0.1 | B: `width_bass` 0.3 | `width_bass` 0.3, `width_mid` 0.5 | C: A and `losses.b2` 1.0e-7 below C3 |
|---|---|---|---|---|---|
| dips at the bridge, F1–A1 / A#1–E2 / F2–B2 (real −2.3, −2.6, −0.9 dB) | −12.2, −11.0, −9.2 | −0.4, −0.3, −0.4 | −2.0, −1.6, −1.6 | as B | −0.4, −0.3, −0.3 |
| share over 3 dB (real 36, 42, 22 %) | 98, 91, 94 | 6, 3, 1 | 40, 23, 27 | as B | 4, 3, 0 |
| decay at 400–1200 Hz, early / late: F1–A1 (real 5.3 / 4.2) | 19.9 / −4.2 | 6.3 / 6.3 | 7.3 / 10.2 | as B | 5.5 / 5.6 |
| A#1–E2 (real 9.1 / 3.4) | 17.5 / −0.1 | 6.9 / 6.8 | 8.0 / 10.1 | as B | 6.2 / 6.1 |
| F2–B2 (real 7.5 / 6.4) | 19.5 / −0.2 | 7.7 / 8.2 | 8.9 / 11.7 | as B | 6.9 / 7.5 |
| 400–1200 Hz at 2.5–3 s, bridge: F1–A1 (real −7.8, −6.0) | −7.4 | −13.2 | −18.5 | as B | −12.2 |
| A#1–E2 (real −9.8, −9.6) | −12.6 | −14.0 | −19.3 | as B | −12.5 |
| F2–B2 (real −14.2, −12.2) | −12.4 | −17.3 | −21.9 | as B | −16.2 |
| C3–B3 (real −20.0, −18.9) | −16.1 | −21.7 | −20.3 | −24.9 | −20.9 |
| C4–B4 (real −22.2, −20.7) | −21.9 | −21.9 | −21.9 | −27.2 | −21.9 |
| C5–B5 (real −24.4, −20.2) | −28.2 | −28.2 | −28.2 | −37.8 | −28.2 |
| the whole sound at the ears at 2 s: F1–A1 (real −6.2, −6.6) | −8.5 | −7.6 | −8.9 | as B | −7.3 |
| A#1–E2 (real −10.8, −9.3) | −12.2 | −9.6 | −10.9 | as B | −8.9 |
| F2–B2 (real −12.4, −12.7) | −12.5 | −10.6 | −11.8 | as B | −10.1 |
| C3–B3 (real −15.4, −16.3) | −17.3 | −20.2 | −20.1 | −20.9 | −19.7 |
| key to key, the whole sound at the bridge at 2.5–3 s, rms about the neighbours: A#1–E2 (real 1.4, 1.7) | 1.7 | 5.3 | 6.4 | as B | 4.7 |
| F2–B2 (real 0.9, 2.2) | 0.8 | 5.8 | 5.8 | as B | 5.3 |
| bench, overall | 1.80 | 1.80 | 1.80 | 1.89 | 1.73 |
| bass / middle / treble / top | 1.41 / 1.40 / 1.54 / 2.74 | 1.39 / 1.40 / 1.54 / 2.74 | 1.42 / 1.40 / 1.54 / 2.74 | decay family 0.94 to 1.24 | 1.21 / 1.21 / 1.54 / 2.74 |
| distance to the commercial D, overall | 8.02 | 8.06 | 8.06 | 8.21 | 8.06 |
| bass / middle / treble / top | 8.37 / 9.13 / 8.34 / 5.85 | 8.40 / 9.25 / 8.34 / 5.85 | 8.40 / 9.24 / 8.34 / 5.85 | 8.40 / 9.22 / 8.98 / 5.81 | 8.40 / 9.24 / 8.34 / 5.85 |

- **A, 0.1 cent**: the beats are gone, the first two seconds of the whole sound come nearer the real grand's (A#1–E2 at the ears at 2 s: −9.6 against −10.8 and −9.3; the preset −12.2), the bench and the distance do not move. The price: the band 400–1200 Hz 5 dB under the real one at 2.5–3 s (the preset level with it); the long decay less even from key to key on the trichords (5 to 6 dB rms about the neighbours against 1 to 2); and C3–B3, where the width comes back to 1 cent through the values that send a null to 2–4 s, 3 dB lower at 2 s than the preset and 4 to 5 under the real grand. It is cleaner than the real grand in the first second and a half (1 to 6 % of the partials dip, 22 to 54 on the real one): the real dips that single strings also show are not in the model.
- **B, 0.3 cent**: the real dips over 1.6 s, and the worst late sound of all (9 to 11 dB under the real grand at 2.5–3 s). The reading of section 1 taken at its word; refuted by the five seconds of section 4.
- **0.5 cent at C4** on top of it: what the real grand's dips ask for in the middle. Refuted by the measures: the sustain of C4–B5 falls 5 to 10 dB under the real grand's at 2.5–3 s, the bench's decay family goes from 0.94 to 1.24, the treble against the commercial D from 8.34 to 8.98. Not put in the list.
- **C** was a trial of section 4's last remark, not a proposal: `losses.b2` is not to be changed on a measure (item 6). The bench likes it (bass 1.41 to 1.21, middle 1.40 to 1.21), as it liked the low loss before the keyboard refused it.

A removes the fault the item names and uncovers another of the same size later in the note; which of the two is heard, and whether C buzzes, was for the keyboard.

**At the keyboard (2026-11-15).** The four presets were played from the instrument's list. C was found better, and the buzz did not come back. C is in the Concert grand preset: `unison.width_bass = 0.1`, `losses.b2 = 1.0e-7` up to C3 and 2.0e-7 from C4 up, as before (`b2_top = 2.0e-7`, `b2_top_from = 60`). From A3 to F#5, where the buzz had been measured as high partials held too long, the loss is within an eighth of what it was. What the keyboard refused at 6.0e-8 and at 1.0e-7 was then the lasting high partials of unisons a cent wide, beating at 1 to 2 Hz, and not their length.

## 6. Listening and playing

- `out/ab_bass_unisons/index.html` (the `ecoute` entry of `.claude/launch.json`, port 8774): eighteen keys from F1 to C5, the preset, A, B and C; through the close microphones without a room at velocities 100 and 60 with the commercial D, then at the player's seat, lid off, with the real grand, four seconds each. One gain for the model's columns. Each click restarts from the beginning and stops the other players.
- `out/keyboard/instruments/Bass unisons trial/`: the preset, A, B and C for the instrument's own list (`target/release/unsampled-piano --ui --presets out/keyboard`).

## Not verified

- **The listening page was not reported on**; the verdict is the keyboard's, in a few words (C better, no buzz), from a player who is not a pianist. Nothing was said of A and B, of C3–B3 where the width comes back to a cent, or of the evenness from key to key.
- **A loss lower still under narrow unisons** (6.0e-8, the value the bench liked best) was not played.
- **The real grand's own width.** Bounded from above only (no null in 5 s). Its strings were not recorded one at a time, so "within 0.15 cent" and "held by the bridge" are not told apart; the second would need a coupling the single strings' decay does not allow at these frequencies, unless the real bridge yields far more to a unison's strings than to a single one's.
- **Why the real unisons hold after a second.** The bichords decay more slowly than the single strings below them and the wound trichords in two stages; the model's narrowed unison does neither. The strings' own loss is one part (section 4); the other could be an aftersound the model lacks (the strings of a key meet the bridge at one point with one coupling in the model; on a real bridge each has its own pin and its own length), or the real single strings' own beats. Not separated.
- **The dips of real single strings** (2 to 45 % of their partials): the two polarisations, 0.07 Hz apart on a measured wound string, or the measure's noise. The model's single strings have none.
- **The inharmonicity of the two strings of a wound pair**: nothing published, nothing in the model, not read on the real grand.
- **The middle and the treble**: read by dips only, on one instrument; above C5 not readable by this method. The preset's 1 cent at C4 is inside Kirk's 1 to 2 cents and outside the tuners' 0.1 to 0.5.
- **The law's shape between B2 and C4**, a straight line to `width_mid`, passes through the widths that put a null at 2–4 s. Holding the narrow width up to B3 and stepping at C4 was not tried.
- **Kirk's paper, Weinreich's of 1977 and the report with the D's strings** could not be opened; what is said of them is from abstracts and from those who cite them.
- **Dynamics**: the real grand has one; the model was read at velocity 105 only.

## Tools

`unison_lock` (piano-host's example): `cargo run --release -p piano-host --example unison_lock -- preset.toml [first last [step]]`, the mistuning against the coupling per key and partial (`CSV=1` for a file). Not versioned, in this session's `out/tools/`: `uni.py` (dips and decays per key, bridge and ears), `eqw.py` (the width that gives the real dips), `envt.py` (a band by half seconds), `aft.py` (the long decay and its evenness), `pair.py` (two or three coupled strings with the engine's figures), `mkvar.py`, `abpage.py`, `run_cands.sh`, `run2.sh`.
