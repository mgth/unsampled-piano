# The bass against its two references

Status as of 2026-11-12. A0–B2 of the Concert grand preset, measured against the commercial D and against a real concert grand, each gap put on a stage of the model, and the cheap levers tried one line at a time. The preset is not changed: the variants are in `out/presets/`, their renders in `out/v/`, a listening page in `out/ab_bass_gaps/` (none of it in the repository). Tool: `tools/bass_gaps.py`.

## What comes out

- **Three gaps are shared by the commercial D, the real grand and the bench**, and each has a measured cause:
  1. the bass bridge takes too much from the strings between 60 and 140 Hz (partials 2–4 of A0–E1, the fundamentals of A#1–B2);
  2. the strings' own loss above 1.2 kHz is two to three times too high over the whole bass (`losses.b2`);
  3. the strings of a bichord or trichord beat (F1–B2), where the real ones do not (`unison.width_mid`).
- **The largest term of the distance to the commercial D is not a fault.** The D's low fundamentals fade at 6–10 dB/s; the real grand's and the preset's hold (0.3–1.4 dB/s). After half a second the preset's fundamental stands 14 dB over the D's at A0–B1, and that alone is the −11 to −13 dB bias of the distance's sustain family. The real piano rules: not to be corrected.
- **The bass bridge at 60–140 Hz is still the main gap against the real grand**, but not as it was described. Fitted on the real grand's own partials, its resonances are about as broad as the model's with half the resting strings' load (loss factor 0.10 at 97 Hz, 0.07 at 139 Hz). What is wrong is how hard the bridge is coupled to them: seven times too much near 70–80 Hz, 2.3 times near 95 Hz. No cheap lever changes that.
- **Two one-line changes close gaps 2 and 3**, `losses.b2` 1.0e-7 and `unison.width_mid` 0.2: the bench's bass register goes from 1.50 to 1.22 with `string_load` 0.5 added. They act on the whole keyboard, where they are not yet judged by ear.
- **The felt over the core in the bass (`core_depth_bass` 0.3) follows the commercial D against the real grands**: at mf the bass attack has 19 dB too little at 4–8 kHz on the bench, and none with the core at the surface.

## Method

Two references, never mixed:

- **The commercial D**, recorded dry without its hammer noise (`out/catalog_clean/steinway_d_no_hammer_noise/`, velocities 60 and 100, held 3.5 s). The preset plays the same notes through the close microphones without a room (`archive/presets/listening/Microphones/close.toml`, `Room/dry.toml`). One gain for the set, taken on the peaks of A2–A5 (−5.8 dB).
- **Plath's concert grand** (anechoic, lid off, every key at one dynamic, take 2 of PROD08): the bridge's acceleration and the dummy head at the player's seat. The preset gives the same two signals through piano-render's `bridge_signal` (bridge velocity at the key, pressure at `player.toml`'s microphones, lid off, no room), at velocity 105. Both sides are in physical units, so that levels are compared as they are, with no gain.

`tools/bass_gaps.py` follows each partial in each file on its own (f_k = k·f1·√(1 + B·k²), f1 and B refitted as the partials are found, up to partial 64 or 9 kHz) and reads:

- its **level** in 0–0.3 s and in 0.5–1.5 s (a partial less than 10 dB over the median of the spectrum around it is not read);
- its **decay rate** over 0.15–1 s and 1–2.5 s (341 ms windows; not read where the partial comes within 8 dB of the floor between the partials);
- third-octave and octave **bands**, the attack's growth from velocity 60 to 100, the envelope's drop.

Against Plath every figure comes three times: at the bridge (hammer, string and bridge), from the bridge to the ears (board and radiation: the ears' level minus the bridge's), and at the ears. The decay of a partial at the bridge does not depend on any level or calibration: it is the surest reading here.

Limits of the comparison:

- Plath's grand was played at one dynamic, called forte. The preset's bridge and ears are within 4 dB of it in mean level at velocity 105 (bridge +0.3, −2.1, −3.6 dB for A0–B0, C1–B1, C2–B2; ears −2.6, −0.8, −1.2). A level common to all partials is therefore free; the shape of a spectrum is not.
- The real ears are a dummy head's, the model's two bare microphones. The real accelerometer sits at one spot of the bridge; the model's bridge signal is the board under the key's strings.
- The commercial D's microphones are not the player's seat. Where the D and the real grand differ in the level of a band, it can be the position.
- The distance to the D of docs/steinway_d.md (`dscore.py`) reads partials 2–8 in 60 ms: at A0–B1 the partials are 27–62 Hz apart and such a window does not separate them. It is given below as it is, and the partials are read again in longer windows.

## 1. The gaps

### Against the commercial D

The distance of docs/steinway_d.md on the 27 keys, rms and (mean) per family: 8.70 over the bass (9.15, 8.31, 8.62 by sub-register), 9.46 on all 88 keys.

| | A0–B0 | C1–B1 | C2–B2 |
|---|---|---|---|
| attack's partials 2–8, ff | 9.6 (+9.2) | 9.3 (−0.1) | 11.7 (+0.7) |
| attack's partials 2–8, mf | 6.6 (+4.7) | 8.9 (+0.3) | 11.8 (+0.8) |
| sustain's partials 2–8, ff | 16.0 (−12.9) | 14.6 (−11.0) | 12.7 (+3.4) |
| sustain's partials 2–8, mf | 15.9 (−12.7) | 14.3 (−10.1) | 12.7 (+4.5) |
| envelope, ff | 7.0 (−5.6) | 3.3 (−2.2) | 4.7 (−1.4) |
| envelope, mf | 7.1 (−5.8) | 3.8 (−2.6) | 5.8 (−3.2) |
| brightness | 2.5 (−1.4) | 1.9 (+0.3) | 2.2 (+0.1) |
| level | 1.3 (+0.4) | 2.7 (+0.8) | 3.0 (−1.1) |
| dynamics | 0.8 (−0.8) | 1.1 (−0.6) | 0.7 (+0.4) |

The sustain family is the largest, and at A0–B1 it is a bias: partials 2–8 stand 11–13 dB lower against partial 1 than on the D. Read partial by partial, that is the fundamental, not the partials above it.

**Level of the partials**, preset − D, velocity 100, mean over the keys and the partials of each group (dB; the spread of single partials about these means is 4–11 dB rms):

| | partial 1 | 2–4 | 5–8 | 9–16 | 17–32 | 33–64 |
|---|---|---|---|---|---|---|
| A0–B0, 0–0.3 s | not read | +1.7 | +6.6 | +3.1 | +1.4 | +3.1 |
| C1–B1, 0–0.3 s | +4.3 | +0.6 | +6.4 | +2.1 | +2.7 | +6.2 |
| C2–B2, 0–0.3 s | +0.6 | +3.0 | −3.8 | −1.9 | 0.0 | −2.5 |
| A0–B0, 0.5–1.5 s | **+14.3** | −3.5 | −0.8 | +0.9 | −1.4 | +6.3 |
| C1–B1, 0.5–1.5 s | **+13.8** | −4.7 | +2.1 | −1.4 | −0.3 | −0.4 |
| C2–B2, 0.5–1.5 s | −6.8 | +0.1 | −8.2 | −5.8 | −6.6 | **−17.8** |

At velocity 60 the same, with more at the top of the spectrum (partials 33–64 in the sustain: +14.7, +3.6, −12.4).

**Decay of the partials** over 0.15–1 s, then over 1–2.5 s (dB/s, median over the keys and the partials), with the real grand's at its bridge beside the D's:

| | partial 1 | 2–4 | 5–8 | 9–16 | 17–32 | 33–64 |
|---|---|---|---|---|---|---|
| A0–B0, the D | . / 6.3 | 4.8 / 4.8 | 3.0 / 2.6 | 2.3 / 2.4 | 5.5 / 5.5 | 16.1 / 16.1 |
| A0–B0, the real grand | −0.3 / 0.4 | 3.2 / 3.2 | 4.7 / 3.9 | 6.4 / 5.2 | 7.8 / 5.7 | 11.0 / 8.2 |
| A0–B0, the preset | 0.3 / 0.5 | **11.4 / 9.0** | **10.7 / 10.3** | 6.4 / 6.5 | 8.1 / 7.9 | 12.3 / 12.1 |
| C1–B1, the D | 9.8 / 7.5 | 4.2 / 4.6 | 4.6 / 4.5 | 4.2 / 2.7 | 5.7 / 4.0 | 8.7 / 9.6 |
| C1–B1, the real grand | 1.2 / 0.8 | 5.7 / 4.3 | 4.6 / 3.8 | 4.3 / 4.1 | 6.0 / 3.9 | 8.2 / 7.5 |
| C1–B1, the preset | 1.4 / 1.6 | **8.9 / 10.2** | **9.5** / 5.2 | **10.7** / 2.6 | 8.9 / 6.9 | **14.7 / 14.3** |
| C2–B2, the D | 10.4 / 7.8 | 8.6 / 7.5 | 8.7 / 9.2 | 13.0 / 3.8 | 10.9 / 7.0 | 15.5 / 13.4 |
| C2–B2, the real grand | 9.6 / 5.8 | 7.6 / 7.9 | 8.1 / 5.8 | 8.4 / 5.5 | 8.6 / 5.9 | 15.1 / 10.7 |
| C2–B2, the preset | **18.3 / 14.6** | 8.8 / 8.6 | **12.9 / 0.5** | **20.5 / 1.1** | **15.8** / 10.1 | **26.5** / 15.8 |

**Drop of the whole sound** between 0.2 and 2 s, then between 0.2 and 1 s (dB):

| | A0–B0 | C1–B1 | C2–B2 |
|---|---|---|---|
| the commercial D | 6.5 / 3.2 | 9.0 / 4.3 | 14.6 / 7.1 |
| the real grand, at the ears | 11.5 / 6.2 | 9.1 / 5.0 | 13.6 / 7.2 |
| the preset, close microphones | 13.6 / 7.1 | 12.7 / 7.3 | 17.1 / 9.1 |
| the preset, at the player's seat | 14.4 / 7.2 | 12.7 / 7.7 | 16.5 / 9.9 |

**From mf to ff**, growth of the attack's bands (first 60 ms) from velocity 60 to 100, 1–2 / 2–4 / 4–8 kHz / peak (dB):

| | A0–B0 | C1–B1 | C2–B2 |
|---|---|---|---|
| the commercial D | 22.7 / 37.7 / 41.8 / 8.7 | 19.0 / 34.9 / 41.4 / 8.8 | 15.2 / 27.5 / 40.4 / 7.9 |
| the preset | 14.9 / 24.8 / 39.8 / 7.9 | 17.5 / 33.6 / 48.4 / 8.2 | 14.8 / 24.2 / 42.9 / 8.3 |

The 31.8 dB against 35.5 of docs/steinway_d_gaps.md (2–4 kHz, A0–B1) is three keys: A0–B0 grow by 24.8 dB against 37.7, C1–B1 by 33.6 against 34.9. It is the mf blow that differs: octave bands of the first 60 ms, preset − D, at velocity 60 then 100:

| | 2 kHz | 4 kHz | 8 kHz |
|---|---|---|---|
| A0–B0 | +11.1, then +1.0 | +16.6, then +2.8 | −21.8, then +3.9 |
| C1–B1 | +4.5, then +3.0 | +5.6, then +2.1 | −19.0, then −1.5 |
| C2–B2 | +1.1, then −0.2 | +2.9, then −1.9 | −20.4, then −8.3 |

### Against the real concert grand

**At the bridge, the decay** of the partials over 0.15–1 s by their frequency (dB/s, median; the keys grouped by their strings):

| Hz | 45–70 | 70–100 | 100–140 | 140–200 | 200–280 | 280–400 | 400–700 | 700–1200 | 1200–2000 | 2000–3500 |
|---|---|---|---|---|---|---|---|---|---|---|
| A0–E1 (one string), real | 3.0 | 6.7 | 3.9 | 3.7 | 4.6 | 4.6 | 5.6 | 6.5 | 7.3 | 11.9 |
| A0–E1, the preset | 4.5 | **20.7** | **10.3** | 3.4 | 6.7 | 4.7 | 5.4 | 7.2 | 10.9 | 16.0 |
| F1–E2 (two and three, bass bridge), real | 2.6 | 8.5 | 5.9 | 6.9 | 2.9 | 5.8 | 7.0 | 8.2 | 6.2 | 7.2 |
| F1–E2, the preset | 4.1 | **24.2** | **18.7** | 6.1 | 7.7 | **13.1** | **13.7** | **24.8** | **13.4** | **16.0** |
| F2–B2 (three, main bridge), real | . | 18.8 | 4.7 | 21.3 | 5.8 | 7.7 | 6.5 | 8.0 | 9.6 | 12.1 |
| F2–B2, the preset | . | 18.2 | **14.4** | 6.3 | 6.8 | 9.1 | **10.9** | **18.7** | **14.8** | 16.0 |

Above 140 Hz a single string decays as the real one, up to 1.2 kHz. Three things are off: 70–140 Hz everywhere, everything above 280 Hz on the keys with two or three strings, and 1.2–3.5 kHz on all of them.

**Level of the partials**, preset − real, 0–0.3 s: at the bridge / from the bridge to the ears / at the ears (dB, mean):

| | partial 1 | 2–4 | 5–8 | 9–16 | 17–32 | 33–64 |
|---|---|---|---|---|---|---|
| A0–B0 | +10.5 / −23.7 / −13.2 | +8.7 / −8.5 / +0.2 | +7.0 / −6.2 / +0.7 | +2.0 / −3.9 / −1.9 | +1.9 / −6.3 / −4.4 | −5.0 / +3.0 / −2.0 |
| C1–B1 | +4.8 / −7.2 / −2.3 | +7.9 / −8.6 / −0.7 | +10.0 / −6.9 / +3.1 | +4.0 / −3.2 / +0.8 | −3.4 / +0.7 / −2.6 | −8.6 / +10.2 / +1.7 |
| C2–B2 | +6.0 / −2.8 / +3.2 | +3.9 / −4.5 / −0.6 | +2.7 / −2.2 / +0.5 | −2.4 / +0.2 / −2.2 | −11.7 / +8.4 / −3.4 | −17.4 / +7.2 / −10.2 |

The same over 0.5–1.5 s:

| | partial 1 | 2–4 | 5–8 | 9–16 | 17–32 | 33–64 |
|---|---|---|---|---|---|---|
| A0–B0 | +9.2 / −13.1 / −3.9 | +1.2 / −5.8 / −4.5 | +5.5 / −7.4 / −1.9 | +2.9 / −3.5 / −0.6 | +2.1 / −7.4 / −5.3 | −5.4 / +3.5 / −1.8 |
| C1–B1 | +5.0 / −4.9 / 0.0 | +4.4 / −8.7 / −4.3 | +6.1 / −6.5 / −0.4 | −0.3 / −2.0 / −2.3 | −7.9 / +1.3 / −6.6 | −15.7 / +11.7 / −4.0 |
| C2–B2 | +0.1 / −2.7 / −2.7 | +1.9 / −4.1 / −2.2 | −1.6 / −1.3 / −3.0 | −9.3 / +2.9 / −6.4 | −21.5 / +11.6 / −10.0 | −36.4 / +9.0 / −27.4 |

Single partials spread about these means by 4–10 dB rms at the bridge and 6–10 dB from the bridge to the ears: the level of each partial of each note, which is where the bell of the low bass lies (docs/steinway_d.md), is as far from the real grand as from the D, with no pattern found.

Two things stand out.

- **At the ears the attack is right** within 3 dB up to partial 32, and so is the fundamental from C1 up. A0–B0's fundamental is 13 dB short at the attack and 4 dB after half a second.
- **The two stages err in opposite directions and cancel.** The preset's bridge carries 4–10 dB more of the low partials than the real one and 5–17 dB less of the high ones; from the bridge to the ears it is the reverse. No dynamic gives the real bridge's spectrum: at velocity 125, where the model's high partials are at their strongest, partials 33–64 still stand 12–14 dB lower against partials 2–8 than on the real bridge (13–21 dB at 105, 16–33 at 85).

**From the bridge to the ears by frequency** (third octaves, median over the partials of A0–B2 in 0.5–1.5 s): each transfer re its own mean over 400–1000 Hz, then the preset − real in absolute terms:

| Hz | 50 | 63 | 79 | 100 | 126 | 159 | 200 | 252 | 317 | 400 | 635 | 1008 | 1270 | 1600 | 2016 | 2540 | 3200 | 4032 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| real, player's seat | −0.2 | 0.0 | +0.6 | +5.3 | +11.2 | +4.8 | +6.8 | +9.3 | +2.2 | −0.8 | +2.1 | −1.8 | −4.8 | −10.3 | −17.2 | −21.0 | −19.9 | −20.4 |
| the preset, player's seat | −2.0 | +0.9 | +4.9 | −3.8 | −1.6 | +0.6 | +3.4 | 0.0 | +0.2 | +1.3 | −0.1 | −0.6 | −3.4 | −1.2 | −2.4 | −6.2 | −6.8 | −11.9 |
| the preset, close pair | +11.4 | +10.9 | +9.3 | +6.3 | +6.1 | +5.9 | +9.0 | +4.9 | +5.4 | +1.3 | +0.1 | −1.8 | −2.3 | −2.6 | −5.9 | −3.9 | −8.4 | −9.1 |
| preset − real, player's seat | −2.4 | +1.5 | +2.4 | −11.8 | −16.2 | −5.2 | −6.0 | −10.5 | −5.3 | 0.0 | −2.9 | −1.4 | +0.8 | +7.6 | +12.9 | +14.2 | +11.9 | +7.7 |

Per unit of bridge movement the real grand gives its player 5–11 dB more at 100–250 Hz than at 400–1000 Hz, and 10–21 dB less above 1.6 kHz. The model's transfer is flat where the real one has that hump, and falls half as much at the top.

**At the ears in the sustain** (0.5–1.5 s), preset − real by frequency of the partial (dB, median):

| Hz | 45–140 | 140–400 | 400–1200 | 1200–2500 | 2500–5000 |
|---|---|---|---|---|---|
| A0–E1 | −2.9 | −2.3 | −3.4 | −0.2 | +3.6 |
| F1–E2 | −5.9 | −1.0 | −4.3 | −2.5 | −13.1 |
| F2–B2 | −3.2 | +0.5 | −7.5 | −4.4 | −23.4 |

### The bench's bass register

`tools/bench.py`, eight real grands, close microphones in a studio: 1.98 overall, **1.50 for the bass** (52 measures). The envelope is within the real pianos at every velocity (at 1 s −10.0 dB against −10.0; z within ±0.6). What is off or nearly:

| measure | the preset | real grands | z |
|---|---|---|---|
| sustain's 8 kHz band | −88.8 | −55.5 | −5.7 |
| attack at mf, share of 4–8 kHz | −81.5 | −62.1 | −4.0 |
| attack's partial 3 re partial 1, mf / ff | 3.4 / 3.8 | 11.0 / 11.4 | −2.3 / −2.1 |
| sustain's 1 kHz band | −8.8 | −1.3 | −2.2 |
| attack's partial 2 re partial 1, mf / ff | 6.3 / 6.6 | 12.1 / 12.4 | −1.9 / −1.9 |
| sustain's partial 12 re partial 1 | −1.5 | 6.7 | −1.9 |
| between the partials, early / late | −27.3 / −29.8 | −22.2 / −24.3 | −1.7 / −1.8 |
| sustain's partials 2, 3, 6 re partial 1 | 7.2, 9.0, 6.2 | 14.1, 16.0, 12.0 | −1.7, −1.7, −1.5 |
| early T60 of partials 5–12 | 1.5 s | 1.9 s | −1.4 |
| late T60 of partials 1–4 | 3.7 s | 2.6 s | +1.3 |

### Where the references agree, and where not

| What the preset does | the commercial D | the real grand | the bench | To correct |
|---|---|---|---|---|
| Partials at 60–140 Hz decay two to three times too fast (partials 2–4 of A0–E1, partial 1 of A#1–B2) | yes: 4.2–4.8 dB/s against 8.9–11.4; C2–B2's fundamentals 10.4 against 18.3 | yes: 4–7 dB/s against 10–24 | yes: sustain's partials 2, 3 re 1 are 7 dB low | yes |
| Partials above 1.2 kHz decay too fast from C1 up | yes: 5.7–15.5 dB/s against 8.9–26.5 | yes: 6–12 against 13–16 | yes: sustain's 1 kHz band 7.5 dB low, partial 12 by 8 dB, 8 kHz band | yes |
| Partials 5–16 of the keys with two or three strings fall fast then stop falling (C2–B2: 12.9–20.5 dB/s, then 0.5–1.1) | yes: 8.7–13.0, then 9.2–3.8 | yes: 8.1–8.4, then 5.8–5.5 | early T60 of partials 5–12 short, late T60 of partials 1–4 long | yes |
| The whole sound of C1–B2 falls 3 dB too fast in 2 s | yes (12.7, 17.1 against 9.0, 14.6) | yes (12.7, 16.5 against 9.1, 13.6) | no: envelope within the real pianos | yes, through the three above |
| A0–B0 fall by 13.6 dB in 2 s | no: 6.5 | nearly: 11.5 | within | 2 dB at most; the D's 6.5 is its own |
| The fundamental of A0–B1 holds (0.3–1.4 dB/s), 14 dB over the D's after 0.5 s | no: the D's fades at 6–10 dB/s | **the preset is right**: 0.3–1.2 dB/s | | no |
| A0–B0's highest partials (33–64) decay at 12.3 dB/s | no: 16.1 | the preset is right: 11.0 | | no |
| The bass's mf attack: 2–4 kHz 11–17 dB over the D's at A0–B0, the octave above 20 dB under | too bright at 2–4 kHz, too dark above | 2–4 kHz right (z −0.5), 4–8 kHz 19 dB too dark | (the bench is the real reference here: Plath has one dynamic) | the top only: 4–8 kHz at mf. Not the D's darker 2–4 kHz |
| The fundamental against partials 2–3 at the attack | 4 dB over at C1–B1 | not seen at the player's seat (short at A0–B0) | 6–8 dB over | open: the position of the microphones differs |
| The level of each partial of each note | 4–11 dB rms | 4–10 dB rms at each stage | | no law found |

## 2. Each gap on its stage

### The bass bridge between 60 and 140 Hz: the bridge

Read on the decay of single strings at the bridge, where no level enters. A string alone loses 8.69·(T/L)·Re Y dB/s to the bridge; `bass_loss` gives the model's side for each partial. The partials of A0–E1 between 60 and 140 Hz, taken together (20 of them): real median 5.1 dB/s and mean 8.9, the preset 13.5 and 14.8.

A resonance fitted on those decays in each band (frequency, loss factor, peak; the same fit on the real decays and on the model's budget at the same partials; strength = frequency × loss factor × peak, the area under the resonance):

| | near 70–80 Hz | near 95 Hz | near 135 Hz |
|---|---|---|---|
| the real grand | 70.7 Hz, η 0.12, 9 dB/s, strength 74 | 97.3 Hz, η 0.10, 24 dB/s, 232 | 139.0 Hz, η 0.07, 32 dB/s, 295 |
| the preset | 80.5 Hz, η 0.32, 21 dB/s, 544 | 93.2 Hz, η 0.17, 34 dB/s, 528 | 131.9 Hz, η 0.10, 19 dB/s, 248 |
| `string_load` 0.5 | 78.0 Hz, η 0.22, 24 dB/s, 414 | 93.8 Hz, η 0.11, 49 dB/s, 498 | 132.3 Hz, η 0.07, 25 dB/s, 237 |
| `string_load` 0 | 76.6 Hz, η 0.09, 42 dB/s, 285 | 94.2 Hz, η 0.05, 88 dB/s, 438 | 133.2 Hz, η 0.02, 109 dB/s, 335 |

(The preset's first band holds two modes, 68 and 76.5 Hz, which one resonance fits badly: its η there is not a mode's.)

- **Width.** The real resonances are not much narrower than the preset's: 0.10 against 0.17 at 95 Hz, 0.07 against 0.10 at 135 Hz. Half the resting strings' load gives the real widths. docs/real_board.md's "one narrower resonance near 97 Hz" was read on medians per band; resonance by resonance it does not hold.
- **Strength.** The real bridge is coupled 2.3 times less to its 97 Hz resonance, seven times less to what it has near 71 Hz, and as much to the one near 139 Hz. The model's second mode, 76.5 Hz, takes 18–21 dB/s from D1 and D#1 where the real strings lose 4.5–10: the real grand has nothing that strong between 75 and 90 Hz.
- **The whole bass bridge.** On F1–E2 the preset reads 24.2 and 18.7 dB/s at 70–140 Hz against 8.5 and 5.9, and one string's budget there is 8.4–8.9 dB/s where three real strings together lose 4.5–7.

So the bridge is the stage, and its low modes' shapes at the bass bridge are the fault, more than their damping. It is still the main gap against the real grand: it sets the decay of the strongest partials of A0–E1 and of the fundamentals up to B2, and with them the 3 dB too many of C1–B2's fall.

### Partials above 1.2 kHz: the string's own loss

The model's budget against the real total (dB/s, median; the bridge's share is one string's):

| | 1.2–2 kHz: own, bridge, real total | 2–3.5 kHz | 3.5–6 kHz |
|---|---|---|---|
| A0–E1 | 8.0, 2.6, **7.3** | 13.9, 2.2, **11.9** | |
| F1–A1 | 6.8, 2.1, **4.7** | 16.0, 2.2, **8.4** | 27.2, 1.6, **12.3** |
| A#1–E2 | 5.6, 1.7, **7.0** | 14.5, 1.4, **6.4** | 32.6, 1.6, **10.7** |
| F2–B2 | 4.9, 1.5, **9.6** | 13.5, 1.4, **12.1** | 38.9, 1.5, **17.8** |

Above 2 kHz the string's own loss alone is larger than everything the real string loses, by a factor of up to three. It is `losses.b2`·f², 10.9 dB/s at 2.5 kHz with the preset's 2.0e-7; the real totals ask for 0.6 to 1.0e-7 once the bridge's share is taken off. The preset's value was raised to 3.3 times the default's for the commercial D's treble, poorer in partials than any real grand (docs/steinway_d.md); the bass paid for it. The stage is the string.

### Keys with two or three strings: the unison

The engine's own modes for these keys decay at 4–8 dB/s between 400 and 1200 Hz, yet the rendered partials read 14–25 dB/s over the first second, then 0.4 dB/s. That is not a decay: the strings of a unison are 1 cent apart (`unison.width_mid`) and beat, each partial k passing through a null after 1731/(2·k·f1) seconds. Deepest dip of a partial's envelope under its own exponential trend, at the bridge, 400–3500 Hz, 0.15–1.6 s (dB, median):

| | A0–E1 (one string) | F1–A1 (two) | A#1–E2 (three) | F2–B2 (three) |
|---|---|---|---|---|
| the real grand | −1.1 | −1.7 | −2.3 | −0.8 |
| the preset, 1 cent | −0.1 | −8.7 | −8.7 | −7.2 |
| 0.5 cent | −0.1 | −6.4 | −5.0 | −4.7 |
| 0.2 cent | −0.1 | −0.8 | −0.7 | −0.6 |
| 0.1 cent | −0.1 | −0.3 | −0.2 | −0.2 |

The real unisons of this grand are within 0.2–0.3 cent, or held together by the bridge. docs/stage_swap.md found the same on three keys of the old default. The stage is the strings.

### The bridge's spectrum and the transfer to the ears: not separable here

The preset's bridge has 13–21 dB less of partials 33–64 against partials 2–8 than the real one at velocity 105 (12–14 at 125), and its transfer to the ears has 10–17 dB more. None of the hammer's parameters moves the bridge's side by more than 3 dB at A0–B1 (`hardness` × 2 and × 4, `hardness_slope_bass` 0 and −0.03, `core_hardness` 2, `core_depth_bass` 0, 0.2 and 0.4, the hammer a fifth lighter, the strike point at 0.11 and 0.13), nor does any reach the low partials' excess. At C2–B2 a felt four times harder brings partials 33–64 from −17.4 to −10.7 dB.

A bridge that moved too much below 600 Hz for the same force, and too little above 1.5 kHz, would give exactly this: too much at the bridge, as much too little from the bridge to the ears, and the right sound. Two readings remain and the data cannot choose:

- the real accelerometer, on the bridge at the string, sees a local movement above 1.5 kHz that does not radiate (its transfer falls by 10–21 dB there), which the model's bridge, a point of the board, does not have;
- the model's board really gives 8–14 dB too much at 1.6–3.2 kHz towards the player, the 2–2.5 kHz step of docs/real_board.md, and its strings too little.

What is sure is the sum, at the ears: the attack right within 3 dB up to partial 32, and partials 33–64 of C2–B2 10 dB short.

Neither the back face nor the case's field carries the transfer's shape: without the back face (`back_gain` 0) the transfer below 90 Hz rises by 5–8 dB (the fundamental of A0–B0 from −23.7 to −15.9 dB, C1–B1's from −7.2 to +0.5) but the missing hump stays (−6 and −12 dB at 100 and 126 Hz, −10 at 252 Hz); without the case's field the top loses 1–3 dB (+12.7 at 2 kHz still).

### From mf to ff: the hammer, against the D only

`core_depth_bass` and `core_hardness` set the growth of A0–B0's attack (2–4 kHz: 15.8 dB without top felt, 20.0 at 0.2 mm, 24.8 at 0.3, 33.9 at 0.4, 33.5 with `core_hardness` 2; the D 37.7). The real grands of the bench say the opposite of the D about the mf blow: see the trial below.

## 3. Cheap levers, one line at a time

Each variant is the whole preset with one line changed (`out/presets/<name>.toml`), A0–B2 rendered at 60 and 100 through the close microphones and at 105 at the bridge and the player's seat.

**Against the real grand**: decay at the bridge over 0.15–1 s (dB/s) at 70–140 Hz | 400–1200 Hz | 1200–3500 Hz, the late decay (1–2.5 s) of 400–1200 Hz, and the ears' level in the sustain at 400–1200 | 1200–2500 | 2500–5000 Hz re the real grand's (dB).

| | A0–E1: decay | A0–E1: ears | F1–E2: decay, late | F1–E2: ears | F2–B2: decay, late | F2–B2: ears |
|---|---|---|---|---|---|---|
| the real grand | 4.6, 6.3, 9.8 | | 6.5, 7.6, 6.9; 3.8 | | 6.1, 7.6, 10.9; 6.4 | |
| the preset | 14.8, 6.8, 12.8 | −3.4, −0.2, +3.6 | 21.2, 20.9, 15.7; 0.4 | −4.3, −2.5, −13.1 | 14.9, 17.4, 15.4; 0.4 | −7.5, −4.4, −23.4 |
| `losses.b2` 1.0e-7 | 14.8, 6.1, 9.9 | −2.8, +2.3, +8.9 | 21.2, 20.2, 9.8; 0.0 | −3.6, +0.8, −4.9 | 14.9, 16.7, 10.4; −0.1 | −6.7, −0.9, −12.1 |
| `losses.b2` 6.0e-8 | 14.8, 5.8, 8.7 | −2.6, +3.1, +11.2 | 21.2, 20.0, 7.5; −0.2 | −3.5, +1.8, −0.9 | 14.9, 16.4, 8.7; −0.5 | −6.5, +0.4, −7.0 |
| `unison.width_mid` 0.5 | no change | | 20.9, 10.8, 25.7; 7.1 | +0.4, −6.7, −14.9 | 14.7, 8.9, 22.8; 9.1 | −3.8, −6.8, −22.8 |
| `unison.width_mid` 0.2 | no change | | 20.8, 7.8, 19.6; 9.1 | +2.7, −1.3, −13.2 | 14.6, 7.2, 17.4; 7.8 | −1.2, −1.4, −23.1 |
| `unison.width_mid` 0 | no change | | 20.8, 7.4, 16.5; 7.0 | +2.8, +0.1, −9.8 | 14.6, 6.9, 14.0; 6.4 | −0.6, +0.5, −18.9 |
| `string_load` 0.5 | 13.8, 6.7, 12.8 | −3.2, −0.2, +3.6 | 19.7, 20.9, 15.7; 0.4 | −4.0, −2.5, −13.1 | 13.4, 17.4, 15.4; 0.5 | −7.4, −4.3, −23.4 |
| `string_load` 0 | 8.5, 6.7, 12.8 | −3.0, −0.2, +3.6 | 16.6, 20.8, 15.7; 0.3 | −3.9, −2.5, −13.1 | 10.1, 17.4, 15.4; 0.5 | −7.4, −4.3, −23.4 |
| `losses.winding` 0 | 14.6, 5.0, 8.9 | −1.6, +3.8, +8.2 | 21.1, 19.7, 14.2; 0.0 | −3.8, −0.9, −11.2 | no change | |
| `losses.winding` 2e-4 | 14.9, 8.7, 16.8 | −5.1, −4.0, −1.0 | 21.3, 22.1, 16.9; 0.9 | −5.0, −4.0, −15.5 | no change | |
| `loss_low_corner` 0 | 14.7, 6.8, 12.8 | −3.3, −0.2, +3.6 | 21.4, 20.8, 15.7; 0.4 | −4.2, −2.5, −13.1 | 14.3, 17.4, 15.4; 0.4 | −7.5, −4.4, −23.4 |
| `b2` 1.0e-7 and `width_mid` 0.2 | 14.8, 6.1, 9.9 | −2.8, +2.3, +8.9 | 20.8, 7.2, 15.3; 8.5 | +2.9, +1.6, −5.3 | 14.6, 6.5, 12.9; 7.4 | −0.4, +1.2, −11.8 |
| the same and `string_load` 0.5 | 13.8, 6.1, 9.9 | −2.7, +2.3, +8.9 | 19.3, 7.1, 15.3; 8.5 | +3.1, +1.6, −5.3 | 13.1, 6.4, 12.9; 7.4 | −0.2, +1.2, −12.0 |

**Against the commercial D**, velocity 100: decay over 0.15–1 s of partials 2–4 | 5–8 | 9–16 | 17–32 | 33–64 (dB/s) and the drop of the whole sound in 2 s (dB).

| | A0–B0 | drop | C1–B1 | drop | C2–B2 | drop |
|---|---|---|---|---|---|---|
| the commercial D | 4.8, 3.0, 2.3, 5.5, 16.1 | 6.5 | 4.2, 4.6, 4.2, 5.7, 8.7 | 9.0 | 8.6, 8.7, 13.0, 10.9, 15.5 | 14.6 |
| the preset | 11.4, 10.7, 6.4, 8.1, 12.3 | 13.6 | 8.9, 9.5, 10.7, 8.9, 14.7 | 12.7 | 8.8, 12.9, 20.5, 15.8, 26.5 | 17.1 |
| `losses.b2` 1.0e-7 | 11.4, 10.6, 6.3, 7.5, 9.9 | 13.4 | 8.8, 9.5, 10.4, 7.7, 9.8 | 12.2 | 8.8, 12.5, 19.5, 10.7, 20.4 | 16.4 |
| `unison.width_mid` 0.2 | as the preset | 13.6 | 8.4, 8.5, 5.6, 8.3, 18.5 | 12.2 | 7.3, 7.1, 9.3, 17.6, 39.5 | 15.2 |
| `string_load` 0.5 | 9.4, 9.5, 6.3, 7.9, 12.2 | 12.7 | 8.9, 9.5, 10.6, 8.8, 14.7 | 12.3 | 8.7, 12.7, 20.5, 15.8, 26.5 | 16.3 |
| `string_load` 0 | 6.5, 7.7, 6.0, 7.5, 12.2 | 11.2 | 6.5, 9.1, 10.6, 8.8, 14.7 | 11.6 | 8.1, 12.5, 20.4, 15.9, 26.9 | 15.1 |
| `b2` 1.0e-7, `width_mid` 0.2, `string_load` 0.5 | 9.4, 9.5, 6.1, 7.5, 9.9 | 12.6 | 8.4, 8.3, 5.4, 7.2, 14.0 | 11.6 | 6.6, 6.5, 7.7, 13.3, 28.6 | 14.2 |

With these three lines the sustain of C2–B2 (0.5–1.5 s) comes from −8.2, −5.8, −6.6, −17.8 dB against the D's on partials 5–8, 9–16, 17–32 and 33–64 to −3.3, +2.7, +0.2, −4.3.

The distance to the D does not see any of this: it stays between 8.58 and 8.97 on the bass for every variant here (the preset 8.70, the three lines together 8.92), because its sustain family is the D's fading fundamental. It is not a measure to tune the bass on.

**On the bench** (bass register, 52 measures, real grands):

| | bass | overall | mf attack's 4–8 kHz (real −62.1) | sustain's 1 kHz band (−1.3) | sustain's partial 12 (6.7) | early T60 of partials 5–12 (1.9 s) | late T60 of partials 1–4 (2.6 s) |
|---|---|---|---|---|---|---|---|
| the preset | 1.50 | 1.98 | −81.5 | −8.8 | −1.5 | 1.5 | 3.7 |
| `b2` 1.0e-7, `width_mid` 0.2, `string_load` 0.5 | **1.22** | 1.94 | −81.9 | −3.1 | 4.6 | 2.3 | 2.2 |
| `core_depth_bass` 0.2 | 1.39 | 1.96 | −75.0 | −8.4 | −1.3 | 1.5 | 3.7 |
| `core_depth_bass` 0 | **1.28** | 1.94 | −62.6 | −7.8 | −1.3 | 1.5 | 3.7 |

The three lines change the whole keyboard: overall the attack family goes from 1.98 to 2.13 and the decay family from 1.37 to 1.69, the sustain from 1.26 to 1.11 and the brightness from 3.15 to 2.70. Nothing here moves the attack's partials 2 and 3 against partial 1 (z −1.9 to −2.3 in every variant).

**What the three lines cost outside the bass**, against the commercial D on all 88 keys:

| | overall | bass | middle | treble | top | what moves |
|---|---|---|---|---|---|---|
| the preset | 9.46 | 8.70 | 9.86 | 9.69 | 9.54 | |
| `losses.b2` 1.0e-7 | 10.10 | 8.67 | 10.11 | 10.96 | 10.51 | the sustain's partials 2–8 of the treble and the top, 6–9 dB richer than the D's (3 before) |
| `unison.width_mid` 0.2 | 9.67 | 8.80 | 9.66 | 10.55 | 9.61 | the treble's envelope, 8–11 dB under the D's (3–7 before): without their beats its unisons fall faster |
| `string_load` 0.5 | 9.50 | 8.89 | 9.87 | 9.67 | 9.56 | nothing outside the bass |
| all three | 10.29 | 8.96 | 9.75 | 11.51 | 10.74 | both of the above |

`b2` and the unison's width each cost about one point in the treble against the D, as global values. That is the D's treble, which docs/steinway_d.md already found poorer in partials than any real grand's; on the bench the three lines leave the overall score where it was (1.98 → 1.94). Either way, both ask for a value of their own below the break rather than one for 88 keys.

**The board's low modes**, without a render: `bass_loss` on each variant, the three resonances fitted as above. Sum of the strengths near 70–80 and 95 Hz: the real grand 306, the preset 1072.

| | first modes (Hz) | strength near 70–80 Hz | near 95 Hz |
|---|---|---|---|
| the real grand | 71, 97, 139 as seen from the strings | 74 | 232 |
| the preset | 68, 77, 95, 111, 128, 135 | 544 | 528 |
| rear lengths halved, bass bridge only | 69, 82, 99, 113, 129, 137 | 313 | 663 |
| rear lengths halved, every string | 73, 87, 102, 117, 131, 146 | 282 | 857 |
| rear lengths doubled, every string | 62, 72, 90, 107, 124, 130 | 600 | 304 |
| `crown_freq` 50 | 74, 82, 99, 115, 131, 138 | 419 | 711 |
| `crown_freq` 30 | 64, 72, 91, 108, 126, 133 | 1112 | 432 |
| bass bridge 25 % thicker | 68, 76, 94, 110, 128, 134 | 583 | 475 |
| bass bridge 22 % thinner | 68, 77, 95, 112, 128, 136 | 515 | 558 |
| no strings on the bridge | 49, 62, 83, 94, 112, 127 | 289 | 838 |
| `loss_low_corner` 0 | 68, 77, 95, 111, 128, 135 | 489 | 519 |

Each of them moves the modes by a few hertz and passes strength from one band to the next; none brings the sum under 900. Decay by decay, the rms of 10·log10(model/real) over the 31 partials of A0–E1 between 45 and 300 Hz is 3.8 dB for the preset and between 3.6 and 5.0 for every variant, the resting strings' load taken off or not.

### What each lever is worth

| Lever | Measured effect | Plausible value | Cost elsewhere |
|---|---|---|---|
| `losses.b2` 2.0e-7 → 1.0e-7 | 1.2–3.5 kHz at the bridge: 12.8, 15.7, 15.4 dB/s → 9.9, 9.8, 10.4 (real 9.8, 6.9, 10.9). Ears' sustain at 2.5–5 kHz: F1–E2 −13 → −5 dB, F2–B2 −23 → −12. C1–B2's highest partials against the D: 14.7, 26.5 dB/s → 9.8, 20.4 (8.7, 15.5) | Yes: it is what the real strings lose, and half way back to the default's 6.0e-8. 6.0e-8 fits F1–E2 better and overshoots at A0–E1 | A0–E1's sustain above 2.5 kHz goes from +3.6 to +8.9 dB over the real grand's at the ears. The treble against the D: see the 88 keys above. The value was set for the D's treble |
| `unison.width_mid` 1 → 0.2 cent | Dips −7 to −9 dB → −0.6 to −0.8 (real −0.8 to −2.3). 400–1200 Hz at the bridge: 20.9, 17.4 dB/s → 7.8, 7.2 (real 7.6, 7.6), and its late decay 0.4 → 7.8–9.1 (3.8, 6.4). C2–B2 against the D: partials 5–16 12.9, 20.5 → 7.1, 9.3 (8.7, 13.0) | Yes: 0.2–0.3 cent is what this grand's partials show. A freshly tuned concert grand; a piano in use drifts wider | It moves the null of each partial later, onto the higher partials (C2–B2's 33–64: 26.5 → 39.5 dB/s), which `b2` then has to carry. It holds up to C4: the middle changes too, where the default's losses were set with the beats (docs/stage_swap.md: bench 1.11 → 1.19 at 0.2 cent on the old default) |
| `string_load` 1 → 0.5 | Widths of the 95 and 135 Hz resonances as the real ones (0.11, 0.07 against 0.10, 0.07). 70–140 Hz at the bridge 14.8 → 13.8 dB/s (real 4.6); A0–B0's partials 2–4 against the D 11.4 → 9.4 (4.8) | Plausible, on two resonances. A string below its own fundamental is a spring, already in `structure.strings`, not a resistance: most strings should not load the modes under 100 Hz | Small: the bass against the D 8.70 → 8.87 through C2–B2's sustain. Its effect from key to key in the middle (docs/evenness.md) is not measured here |
| `string_load` 0 | 70–140 Hz 14.8 → 8.5, 21.2 → 16.6, 14.9 → 10.1 dB/s (real 4.6, 6.5, 6.1); A0–B0's drop 13.6 → 11.2 dB (real 11.5) | No: resonances two to three times narrower than the real ones (η 0.05, 0.02). It halves the gap by hiding the coupling's excess between the resonances | Peaks of 88 and 109 dB/s on the partials that fall on a mode |
| `core_depth_bass` 0.3 → 0 | Bench: the mf attack's 4–8 kHz −81.5 → −62.6 (real −62.1), bass 1.50 → 1.28 | As plausible as 0.3: a choice of voicing. The real grands side with it | Against the D, A0–B0's growth at 2–4 kHz falls from 24.8 to 15.8 dB (the D 37.7). The two references pull apart: the real ones rule |
| `core_depth_bass` 0.4, or `core_hardness` 2 | A0–B0's growth at 2–4 kHz 24.8 → 33.9, 33.5 dB (the D 37.7) | A choice of voicing | 4–8 kHz grows by 49–55 dB at C1–B1 (the D 41.4); darker still at mf, against the real grands |
| `losses.winding` 1e-4 → 0 | A0–E1 at 1.2–3.5 kHz 12.8 → 8.9 dB/s (real 9.8), drop 12.0 → 10.1 dB at the ears (9.7) | No: set on real grands' metallic low bass (docs/bass_metal.md). `b2` does the same from the right cause | A0–E1's sustain above 1.2 kHz 4–8 dB over the real grand's |
| `back_gain` 0.6 → 0 | The ears' level under 140 Hz −2.9, −5.9, −3.2 dB → +1.6, +0.5, +2.2 | Not as a value (a back face that radiates nothing); it says the back face cancels 5–8 dB too much at the player's seat | The fundamental against the D, already 14 dB over: 16.6 |
| rear lengths, `crown_freq`, bass bridge's thickness, `loss_low_corner` | None reduces the bridge's coupling at 60–140 Hz | | |
| `hardness`, `hardness_slope_bass`, hammer mass, strike point | Under 3 dB on the bridge's spectrum at A0–B1 | | |

## 4. Levers that need code, and the test that would justify each

**The bass bridge's coupling to the board's first modes** (the main gap; nothing cheap reaches it). The real bridge sees resonances near 71, 97 and 139 Hz with strengths 74, 232 and 295; the model's bass bridge sees four modes under 115 Hz with 1072 in all. Candidates, none verified:

- *Crown and bearing as a shell under the strings' load* (item 7 of docs/steinway_d_gaps.md): the foundation term raises every mode alike and cannot change which part of the board a mode moves. Test before building: in `board_fem`, a stiffness under the bass bridge alone, on top of the strings' hold, scanned until the 77 Hz mode's share at D1–D#1 is halved; if the value is of the order of a bearing's (the strings' down-force over the bridge's deflection), the shell is worth building.
- *The bass bridge's real construction*: on a D it stands on an apron, the strings bearing in front of its foot; here it is a layer of the plate at the strings' points. Test: the same scan with the bridge's foot moved a few centimetres towards the tail, the strings left where they are (`bass_bridge` in a preset moves both).
- *The sideways polarisation of a single wound string*: if a real string's movement turns between the two planes faster than it decays, it loses half of what the board takes from the vertical plane, at one rate, which is what is seen. That would halve every figure of the first table without touching the board. Test: Plath's accelerometer has one axis; a recording of a single bass string's two planes would settle it, and none is in the corpus.
- A check that costs nothing: `mode_map` of the first six modes along the bass bridge against which real keys see which resonance (the real 97 Hz one peaks on B0–C#1, the 139 Hz one on A0 and C#1, the 71 Hz one on C#1–D1).

**The string's loss by register.** `losses.b2` is one figure for 88 keys; the bass wants 0.6–1.0e-7 by the real grand and by the D, the D's treble 2–3e-7. A second value below the break, or a slope by key. The test is done: the bass rendered with 1.0e-7 is that lever, since the keys above were not played.

**The unison's width by register.** `unison.width_mid` holds from A0 to C4 and sets the start of the curve above. The bass wants 0.2–0.3 cent; the middle and the treble were calibrated with their beats. A width below the break is a few lines. The test is done the same way.

**The resting strings' load by frequency.** A string loads a mode with √(T·μ) only above its own fundamental; below, it is the spring `structure.strings` already holds. Weighting each string's load by f⁴/(f⁴ + f1⁴) is a few lines in `Soundboard::load_strings`. The test is done as far as a scale can do it: 0.5 gives the real widths at 95 and 135 Hz. It will not close the gap (13.8 dB/s against 4.6).

**The radiation towards the player between 100 and 250 Hz and above 1.6 kHz.** The real transfer has a hump of 5–11 dB at 100–250 Hz that the model lacks and falls 10 dB more above 1.6 kHz. Before any work: the same transfer for the eight keys of the main bridge above B2 (is the hump the bass bridge's or the instrument's?), and Plath's other state of the piano (PROD07) to see what is the piano and what the chamber, whose floor and low-frequency limit are not known here.

## Ranking

| # | Lever | Gap it closes | Measured | Plausible | Cost |
|---|---|---|---|---|---|
| 1 | `losses.b2` 1.0e-7 in the bass (a line today, a value per register to keep the treble) | partials above 1.2 kHz die too fast, C1–B2 | the real decay within 3 dB/s on all three groups of keys; the ears' top of the sustain 8–11 dB nearer | yes | one line changes the treble too; A0–E1 a little bright in the sustain |
| 2 | `unison.width_mid` 0.2 in the bass (a line today, a width per register to keep the rest) | beats on F1–B2 | the real dips and the real decay at 400–1200 Hz | yes, for a tuned concert grand | one line changes the middle and the treble too (the treble's envelope against the D); to hear |
| 3 | `core_depth_bass` 0 (or 0.2) | the mf attack's top on the real grands | bench's bass 1.50 → 1.28 | a voicing | further from the D's growth: a choice between the references |
| 4 | `string_load` 0.5, then by frequency | the low resonances' width | widths as the real ones; 1 dB/s of the 10 too many | yes | small |
| 5 | the bass bridge's coupling at 60–140 Hz | the main gap: partials 2–4 of A0–E1, the fundamentals to B2, 3 dB of C1–B2's fall | not reached | | code, after one of the tests above |
| 6 | the radiation towards the player | the missing hump at 100–250 Hz | not reached | | heavy; first the two checks above |
| — | the fundamental's hold, A0–B0's long fall on the D, the D's darker mf at 2–4 kHz | | the real grand sides with the preset | | not to be done |

## Listening

`out/ab_bass_gaps/index.html`: eleven keys from A0 to B2. First through the close microphones at velocities 100 and 60 with the commercial D, then at the player's seat with the real grand: the preset, `b2` 1.0e-7, the same with `width_mid` 0.2, the same with `string_load` 0.5, and `core_depth_bass` 0 alone. One gain for the preset and its variants.

## Not verified

- Nothing was heard. Every lever here is a measure.
- The real grand is one instrument, at one dynamic, in one state (PROD08, take 2): its unisons and its low resonances are its own. The bench's eight grands confirm the decays' direction, not the figures.
- The real resonances are fitted on eight or nine partials each, from eight keys along 30 cm of bridge: the loss factors are good to a third, the strengths to a factor of 1.5.
- Which of the bridge and the radiation is wrong above 1.5 kHz (section 2).
- The cost of `string_load` on the evenness from key to key, and of `unison.width_mid` on the middle by ear.
- The dynamics against a real grand: the bench's Disklavier only.

## Tools

`tools/bass_gaps.py d <D's notes> name=dir …` and `tools/bass_gaps.py plath <Plath's data> <cache> name=preset.toml …` print every table of section 1. `bass_loss` (piano-host's example) gives the budget of section 2, with `CSV=1` for a file. The scripts that make the variants, fit the resonances and build the listening page are in `out/tools/` (not versioned): `mk.py`, `trial.sh`, `summ.py`, `resfit.py`, `lowsum.py`, `dips.py`, `tfpos.py`, `abpage.py`.
