# The bass against its two references

Status as of 2026-11-14. A0–B2 of the Concert grand preset, measured against the commercial D and against a real concert grand, each gap put on a stage of the model, and the cheap levers tried one line at a time. Sections 1 to 4 are the first pass, on the preset as it was on 2026-11-12; what was heard afterwards, the measure that follows the ear and the change the preset took are in the sections from "After listening" on; "At the keyboard" takes half of that change back; the last one, "The bass bridge's coupling", finds the cause of the first gap below in the board's ribs; the board as it is drawn is in the preset since 2026-11-14. The third gap, the unisons, is taken up in docs/bass_unisons.md (2026-11-15): the 0.2 cent of sections 2 and 3 was read on the first second and a half, and over five seconds no width gives the real decay; the preset took 0.1 cent up to B2 with half the strings' frequency loss up to C3, after the keyboard. The variants are in `out/presets/`, their renders in `out/v/`, a listening page in `out/ab_bass_gaps/` (none of it in the repository). Tool: `tools/bass_gaps.py`.

## What comes out

- **Three gaps are shared by the commercial D, the real grand and the bench**, and each has a measured cause:
  1. the bass bridge takes too much from the strings between 60 and 140 Hz (partials 2–4 of A0–E1, the fundamentals of A#1–B2);
  2. the strings' own loss above 1.2 kHz is two to three times too high over the whole bass (`losses.b2`);
  3. the strings of a bichord or trichord beat (F1–B2), where the real ones do not (`unison.width_mid`).
- **The largest term of the distance to the commercial D is not a fault.** The D's low fundamentals fade at 6–10 dB/s; the real grand's and the preset's hold (0.3–1.4 dB/s). After half a second the preset's fundamental stands 14 dB over the D's at A0–B1, and that alone is the −11 to −13 dB bias of the distance's sustain family. The real piano rules: not to be corrected.
- **The bass bridge at 60–140 Hz is still the main gap against the real grand**, but not as it was described. Fitted on the real grand's own partials, its resonances are about as broad as the model's with half the resting strings' load (loss factor 0.10 at 97 Hz, 0.07 at 139 Hz). What is wrong is how hard the bridge is coupled to them: seven times too much near 70–80 Hz, 2.3 times near 95 Hz. No cheap lever changes that. The cause is the board itself, built from a simplified mesh: with the D's 17 ribs carried to the rim, the bridge in one piece and the strings' grip on it, the board has the real grand's three resonances (78, 100, 137 Hz against 74, 98, 136) and the bass strings lose what the real ones lose (partials 2–4 of A0–E1: 4.3 dB/s against 4.3, 10.3 before). In the preset since 2026-11-14: see "The bass bridge's coupling".
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
- (2026-11-14) None of the three was it; the last line's check was: see "The bass bridge's coupling: the board as it is drawn".
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

## After listening (2026-11-13): what the ear asks for

The variants of section 3 were heard against the preset and the two references (`out/ab_bass_gaps/`): hardly any difference between them, all of them far from both references, nearer the real grand than the commercial D. The measures above did not say that. What follows is the chain of listening tests that found what they missed. Every file is at the player's seat, lid off, against Plath's grand.

**A distance as heard.** Third-octave bands by 5 ms frames over the first 2 s, in dB, each note level-matched on its first second, cells within 60 dB of the note's strongest; mean absolute difference over A0–B2:

| | A0–B0 | C1–B1 | C2–B2 | all |
|---|---|---|---|---|
| the real grand, another take of the same note | 0.2 | 0.5 | 0.6 | 0.5 |
| the real grand a year earlier | 2.7 | 3.2 | 3.6 | 3.3 |
| the preset | 5.6 | 7.0 | 8.2 | 7.4 |
| `b2` 1.0e-7 | 5.6 | 6.1 | 7.4 | 6.6 |
| `b2` 1.0e-7, `width_mid` 0.2, `string_load` 0.5 | 5.5 | 5.8 | 7.1 | 6.4 |
| `core_depth_bass` 0 | 5.7 | 6.6 | 8.0 | 7.1 |

The three lines of section 3 close a seventh of the gap, above 1.4 kHz after half a second, where the sound is already weak: what was heard. The gap is there from the attack in every band (4–8 dB in the first 50 ms).

**The chain**, each step answered by ear on A0, C1, E1, A1, D2, A2:

| Test | Heard |
|---|---|
| Stage swap with the preset (`tools/stage_swap.py`): the model's strings through the real board, the real strings through the model's | The real strings through the model's board are the furthest, on all six keys. The model's strings through the real board are nearer than the model on A0, C1, E1 and worse than the model on A1, D2, A2 |
| The model with the real transfer's broad colour, by third octaves, one curve for the bass or one per note | not nearer |
| The real grand with one ear in both, or the sum of its ears | still as good: it is not the dummy head |
| The model's partials with what the real grand has between its partials | not nearer |
| The model, each partial set to the level it has in the real grand, nothing else | **as the real grand** |
| The model, each partial given the real one's whole envelope in time | not needed |

Two notes on the method. The files that pass through an estimated board (the swap, and a first version of the level correction with a linear-phase filter) smear the attack over tens of milliseconds: an attack judged poor on them is the filter's. And the model's own attack was heard as right on A0, C1, E1 and a little soft on A1, D2, A2.

**Between the partials: a large gap that is not heard.** Each sound split in two complementary halves, the bands around its partials (±0.2·f1) and what lies between them. Level of the second re the first (dB), real / model:

| | 0–50 ms | 50–300 ms | 0.3–1 s | 1–2.5 s |
|---|---|---|---|---|
| A0–B0 | +1.6 / −19.2 | −5.5 / −22.0 | −6.3 / −24.9 | −8.5 / −28.6 |
| C1–B1 | −4.9 / −22.4 | −10.8 / −26.4 | −12.9 / −30.8 | −13.6 / −34.3 |
| C2–B2 | −10.4 / −29.8 | −23.4 / −36.7 | −30.1 / −46.9 | −25.3 / −47.0 |

The real grand has 15–20 dB more between its partials than the model, and giving the model that part did not bring it nearer. A gap in decibels is not a gap heard: this one is recorded, and nothing is to be built on it without another listening test.

**The measure that follows the ear: the level of each partial of each note.** Each sound cut in one band per partial (f_k ± f1/2, partials 1–48, first 1.5 s), the level of each band, model − real, each note about its own mean, over the partials within 50 dB of the note's strongest (dB rms):

| | at the ears: A0–B0 | C1–B1 | C2–B2 | A0–B2 | at the bridge: A0–B0 | C1–B1 | C2–B2 | A0–B2 |
|---|---|---|---|---|---|---|---|---|
| the real grand, another take | 0.4 | 0.5 | 0.7 | 0.6 | 0.2 | 0.4 | 0.5 | 0.4 |
| the real grand a year earlier | 4.3 | 4.6 | 5.4 | 4.9 | 2.5 | 2.8 | 3.4 | 3.0 |
| the preset | 8.4 | 8.3 | 9.2 | 8.7 | 5.0 | 8.4 | 11.0 | 9.2 |
| `strike_ratio` 0.110 | 10.8 | 10.9 | 10.6 | 10.8 | 10.2 | 12.1 | 12.3 | 12.0 |
| `strike_ratio` 0.115 | 9.6 | 9.5 | 10.0 | 9.7 | 7.8 | 10.5 | 11.8 | 10.8 |
| `strike_ratio` 0.1225 | 8.4 | 8.2 | 9.0 | 8.6 | 5.3 | 8.1 | 10.6 | 8.9 |
| `strike_ratio` 0.125 | 10.0 | 9.5 | 9.6 | 9.6 | 8.8 | 10.1 | 11.7 | 10.7 |
| `strike_ratio` 0.130 | 10.7 | 10.7 | 10.5 | 10.6 | 9.8 | 11.8 | 12.6 | 11.9 |

The same piano a year apart differs by 4.9 dB; the preset by 8.7. That is the room there is: under 5 dB the model would be another state of the same piano. The strike point is where it should be (0.12–0.1225; an eighth of the string is worse), and the measure moves by 2 dB for half a percent of the string's length: it is a sensitive one. The variants of section 3 read 8.9–9.1 with their own partial tables, against 9.7 for the preset read the same way: nothing.

**Where it comes from.** The gap at the ears is the sum of the bridge's and of the transfer's, which pull against each other (correlation −0.4 to −0.5 from partial to partial, the cancelling of section 2 seen one partial at a time):

| | at the ears | of which finer than 5 partials | at the bridge | from the bridge to the ears |
|---|---|---|---|---|
| A0–B0 | 8.7 | 6.6 | 5.2 | 9.4 |
| C1–B1 | 9.0 | 7.6 | 8.6 | 9.3 |
| C2–B2 | 10.6 | 7.1 | 11.6 | 9.1 |

Most of it is finer than five partials, which is why no correction by third octaves helps. Is it a law? Share of the gap's variance that the other keys predict, by a profile of the partial's number / by a curve of frequency a twelfth of an octave wide (each key left out in turn):

| | at the ears | at the bridge | from the bridge to the ears |
|---|---|---|---|
| A0–B0 | 16 / 29 % | 41 / 26 % | 12 / 33 % |
| C1–B1 | 18 / 10 % | 79 / 51 % | 19 / 19 % |
| C2–B2 | 49 / 38 % | 79 / 57 % | 17 / 11 % |
| A0–B2 | 28 / 23 % | 71 / 45 % | 16 / 19 % |

- **At the bridge it is a law**: 71 % of it is one profile by partial number, the same on every key. That profile is the slope of section 2 (the model's bridge 8–13 dB over on partials 1–8, 5–8 dB on 9–16, level beyond 20) with the strike point's notches on it. The hammer and the string carry it, and a law can be modelled.
- **From the bridge to the ears it is not**: neither the partial's number nor its frequency predicts more than a fifth of it from the neighbouring keys. It is the board's own fine response from each key's place on the bridge, as different from the real board's as two boards' would be; the same stage that the swap found furthest by ear.

**Which share the ear needs: both, together** (heard, 2026-11-13). The model corrected by the bridge's share alone, or by the transfer's alone, is "not good at all", worse than uncorrected: each stage hides the other's error, and correcting one uncovers the other's. This is also why the correction by third octaves failed above: it was the transfer's alone. The whole correction smoothed over nine partials, applied at the ears, "comes quite near" the partial-by-partial one.

**The smooth part is a law.** The correction of each key computed from its neighbours only (five keys on each side, the key itself left out), as a profile of the partial's number or as a curve of frequency, was heard as about as good as the key's own smooth correction. Gap left at the ears (dB rms):

| | as it is | the key's own correction, smoothed over 9 partials | the neighbours', by partial number | the neighbours', by frequency |
|---|---|---|---|---|
| A0–B0 | 8.7 | 6.8 | 7.9 | 8.0 |
| C1–B1 | 9.0 | 8.0 | 8.3 | 8.2 |
| C2–B2 | 10.6 | 7.4 | 8.0 | 7.9 |
| A0–B2 | 9.7 | 7.6 | 8.1 | 8.0 |

The smooth part is only 2 of the 9.7 dB, and it is what the ear takes first; the 7.6 dB left, finer than nine partials, was heard as a smaller step (from "quite near" to "as the real grand"). Within eleven keys the partial's number and its frequency are nearly the same thing, so the test does not say which of the two the law follows.

What the model has too much of (+) or too little (−) at the ears, smoothed, mean over the keys (dB):

| partials | 1–4 | 5–8 | 9–13 | 14–18 | 21–27 | 30–39 | 42–48 |
|---|---|---|---|---|---|---|---|
| A0–B0 | −1 | −1 | +1 | +7 | 0 | −4 | +2 |
| C1–B1 | +1 | +4 | +6 | +5 | −1 | −2 | −3 |
| C2–B2 | +8 | +7 | +7 | +7 | +2 | −7 | −12 |

From C2 to B2 it is a slope of 15–20 dB, too much of the low partials and too little of the high ones: the attack heard as a little soft on A1, D2 and A2. From C1 to B1 a hump of 4–6 dB on partials 5–18.

**No parameter reaches it.** The smooth gap (rms, dB) and C2–B2's slope (partials 1–16 / 33–48) for the variants already rendered at the player's seat:

| | A0–B0 | C1–B1 | C2–B2 | C2–B2: partials 1–16 / 33–48 |
|---|---|---|---|---|
| the preset | 4.7 | 4.2 | 6.8 | +5.6 / −7.1 |
| `losses.b2` 1.0e-7 | 4.7 | 3.8 | 5.8 | +4.2 / −6.0 |
| `losses.b2` 6.0e-8 | 4.7 | 3.6 | 5.2 | +3.3 / −5.2 |
| `b2` 1.0e-7, `width_mid` 0.2, `string_load` 0.5 | 4.8 | 3.8 | 5.5 | +3.4 / −5.6 |
| `hardness` × 4 | 4.8 | 4.3 | 5.8 | +4.2 / −6.0 |
| `hardness` × 2 | 4.7 | 4.4 | 6.2 | +4.9 / −6.6 |
| `core_depth_bass` 0 | 4.7 | 3.8 | 6.3 | +4.8 / −6.6 |
| `hardness_slope_bass` −0.03 | 4.8 | 4.3 | 6.0 | +4.6 / −6.3 |
| no case field | 4.9 | 4.5 | 7.9 | +7.2 / −8.5 |
| `core_hardness` 2, `core_depth_bass` 0.4, hammer a fifth lighter, `winding` 0, `string_load` 0, no back face | 4.7–5.0 | 4.0–4.9 | 6.5–6.8 | as the preset |

The string's loss and the felt's hardness each take a quarter of C2–B2's slope and nothing below C2. The law is in the model's physics, not behind a setting.

**What this changes in the ranking above.** The levers of section 3 stand as measures of decay and are right about the real grand, but they are not what is heard first. In the order the ear gave:

1. **The smooth level of the partials at the ears**, note by note: a law common to neighbouring keys, 2 dB rms of the gap and most of what is heard. To be found in the physics, on the sum of the two stages and never on one: a change that brings the bridge nearer the real one and leaves the transfer as it is makes the sound worse. First to explain: C2–B2's slope of 15–20 dB and C1–B1's hump on partials 5–18.
2. **The fine level of each partial**, 7.6 dB rms against 4.9 between two states of the same real piano: the last step by ear. Its share from the bridge to the ears follows no law of the neighbouring keys; it may have no remedy short of the real board's own response.
3. The decays of section 3.

The measure to judge the bass on is the per-partial level at the ears and its smooth part, with the real grand a year apart as the yardstick; the distance of docs/steinway_d.md and the bench's registers average both away.

The scripts of these tests are in `out/tools/` (`audist.py`, `listen_*.py`, `law_*.py`), not versioned.

## Looking for the law's cause (2026-11-13)

With the per-partial gap as the judge. Five things tried; one lever found, for C2–B2 only.

**The strike point is right.** The comb 20·log10(|sin(π·k·r)| + ε) fitted on the bridge's partials 1–48 of each key (first 0.4 s) gives the real grand 0.1205–0.1210 from A0 to F#1 and 0.1215–0.1225 from G1 to B2; the model reads 0.1200, as set. The comb takes half of the roughness of the bridge's spectrum on both (7.8 → 4.5 dB rms on the real A0, 6.6 → 3.2 on the model's). From F2 up the real notches are shallower than the model's (ε 0.25 against 0.06).

**Number or frequency?** Within five semitones the two cannot be told apart. From further away they can: share of each key's smooth gap that other keys predict, by partial number / by frequency:

| keys used | at the ears | at the bridge | from the bridge to the ears |
|---|---|---|---|
| 1–5 semitones away | 60 / 65 % | 88 / 89 % | 48 / 56 % |
| 8–14 semitones away | 12 / 21 % | 80 / 55 % | 36 / 32 % |
| 15–24 semitones away | 4 / −5 % | 57 / 7 % | 18 / 11 % |

- **At the ears the law is local.** It holds between neighbouring keys and is gone an octave away: it is neither a rule of the partial's number nor a curve of frequency for the instrument, but something that changes along the bridge. No single correction for the whole bass exists.
- **At the bridge it is a rule of the partial's number for the whole bass**, a staircase on the lobes of the strike comb. Model − real, each note about its mean (dB), partials 1–8 | 9–16 | 17–24 | 25–32 | 33–41 | 42–48:

| | at the bridge | at the ears |
|---|---|---|
| A0–B0 | +4.5, +1.9, +2.9, −1.1, −3.3, −5.0 | +1.3, +3.1, +3.3, −5.6, −4.5, +3.4 |
| C1–B1 | +10.8, +6.9, +0.6, −3.1, −5.3, −10.5 | +2.3, +5.4, +1.1, −3.0, −2.5, −3.3 |
| C2–B2 | +15.2, +8.1, −0.5, −6.1, −7.8, −12.0 | +6.5, +4.4, +3.3, −2.2, −5.7, −8.3 |

  A fall from lobe to lobe is what a longer or smoother hammer pulse gives. But it is not the string's force alone: taking the bridge's profile off the model's force and leaving the transfer as it is makes the ears' gap worse below C2 (8.4 → 10.6 at A0–B0, 8.3 → 9.0 at C1–B1) and better above (9.2 → 7.8 at C2–B2). Below C2 two thirds of the bridge's staircase is undone on the way to the ears. A bridge mobility would be a matter of frequency, not of number, so what undoes it is not understood; the comparison of the two bridge signals themselves (a real accelerometer at one spot, the model's board under the strings) is the first suspect.
- **A ceiling.** The ears' own profile by lobe, per sub-register, taken off the model, the best a hammer law could do, leaves 7.5, 7.6 and 7.6 dB (from 8.4, 8.3, 9.2), and a smooth gap of 3.5, 2.9, 4.2 (from 4.7, 4.2, 6.8).

**The board's low end: refuted.** The part of the ears' gap that an additive fit puts on frequency asks for 12–14 dB more at 100–126 Hz and 7–16 dB more under 50 Hz, and the model's force-to-ears curve (`force_to_ear`, piano-render's example) has its low maximum at 63–79 Hz, on its first two modes, where the real grand seems to have it at 100–126 Hz. A grid of 96 boards followed: rear lengths × 1, 0.5, 0.33, 0.25, `crown_freq` 40, 60, 80, 100, `string_load` 1 and 0.5, `back_gain` 0.6, 0.3 and 0. The best came from 6.4 to 3.5 dB of that curve (rear lengths halved, no back face), and on the per-partial gap at the ears every one of the six measured among the best is no better than the preset (A0–B0 8.2–9.8 against 8.4, C1–B1 8.3–8.7 against 8.3, C2–B2 8.9–9.2 against 9.2). A curve of frequency fitted across keys is not what the ear's law is made of (it is local), and a fit to it is not a judge.

**The felt's hardness with the default's string loss: C2–B2's slope.** The one place where the bridge and the ears agree is C2–B2, too much of the first lobes and too little of the last on both. Per-partial gap at the ears, smooth / whole (dB rms), and C2–B2's smooth gap on partials 1–16 / 33–48:

| | A0–B0 | C1–B1 | C2–B2 | C2–B2's slope |
|---|---|---|---|---|
| the preset | 4.7 / 8.4 | 4.2 / 8.3 | 6.8 / 9.2 | +5.6 / −7.1 |
| `losses.b2` 6.0e-8 (the default's) | 4.7 / 8.4 | 3.6 / 8.0 | 5.2 / 8.2 | +3.3 / −5.2 |
| `hardness` × 4 | 4.8 / 8.4 | 4.3 / 8.3 | 5.8 / 8.5 | +4.2 / −6.0 |
| `hardness` × 8 | 4.8 / 8.4 | 4.1 / 8.2 | 5.4 / 8.4 | +3.7 / −5.6 |
| `core_hardness` 16, 32 | 4.7 / 8.4 | 4.0 / 8.2 | 6.7 / 9.2 | +5.4 / −7.1 |
| `hardness` × 4 and `b2` 6.0e-8 | 4.7 / 8.4 | 3.7 / 8.1 | **4.3 / 7.7** | +1.7 / −3.3 |
| `hardness` × 8, `core_depth_bass` 0, `b2` 6.0e-8 | 4.7 / 8.4 | 3.5 / 8.0 | **4.1 / 7.6** | +1.1 / −2.7 |
| `hardness_slope_bass` −0.04 and `b2` 6.0e-8 | 4.7 / 8.4 | 3.6 / 8.0 | **4.4 / 7.7** | +2.0 / −3.7 |

A harder felt and the default's string loss together take C2–B2's slope from 12.7 dB to 5, and its gap from 9.2 to 7.7: the first setting the measure sees. `hardness_slope_bass` −0.04 does it without touching anything above C4 (the felt 4.2 times harder at C2 than with the default's +0.02, 2 times at C3); the string loss is the whole keyboard's, as in section 3. Nothing moves A0–B0, and the core's hardness does nothing anywhere. Whether a felt four times harder at C2 is a real hammer's is not checked against a measurement: RT-425's own law was further from the commercial D (docs/steinway_d.md), and the preset's soft felt was set for that D's growth from mf to ff. Listening files: `out/ab_bass_gaps/` section 11. Heard (2026-11-13): with the harder felt the attack from A1 to B2 comes nearer the real grand's. The user kept the felt four times harder: see "The candidate".

**Where this leaves the search.**

- C2–B2: a hammer too soft and a string loss too high, a lever that the measure sees; to hear.
- A0–B1: the smooth gap (4.2–4.7 dB) is local to the place on the bridge and no parameter tried reaches it. What is left to try is the board as seen from each key of the bass bridge, by a measure that does not pass through a fitted curve: the per-partial gap itself, key by key, on boards that differ in the bass bridge's own construction (section 4).
- Everywhere: 7.5 dB of fine structure against 4.9 between two states of the real piano.

## The candidate (2026-11-13)

Of the three hardnesses of section 11 the user kept the felt four times harder with the default's string loss. Both are settings for 88 keys, heard on the bass only, so they were measured up the keyboard before anything else. Per-partial gap at the ears against the real grand, whole / smooth (dB rms):

| | C2–B2 | C3–B3 | C4–B4 | C5–B5 | C6–B6 |
|---|---|---|---|---|---|
| the preset | 9.2 / 6.8 | 9.5 / 6.0 | 8.1 / 4.5 | 7.3 / 2.7 | 5.0 / 1.1 |
| `hardness` × 4 | 8.5 / 5.8 | 9.2 / 5.3 | 7.7 / 4.0 | 7.0 / 2.4 | 5.0 / 1.0 |
| `losses.b2` 6.0e-8 | 8.2 / 5.2 | 8.5 / 4.2 | 7.3 / 3.3 | 6.8 / 2.3 | 5.5 / 1.0 |
| both | 7.7 / 4.3 | 8.5 / 3.7 | 7.1 / 2.9 | 6.9 / 2.2 | 6.1 / 1.1 |

They help against the real grand up to B5, not in the bass alone, and only the string loss hurts, from C6 up. So the loss gets a value of its own at the top instead of one below the break: `losses.b2_top` from `losses.b2_top_from`, coming in over the octave below (0, the default: `b2` on every key, and every render as before, bit for bit on three keys at two velocities). And the top felt is softened so that four times the hardness does not reach it: on the bench the top register went from 2.81 to 3.03 with the harder felt alone.

The candidate, `out/presets/cand4t035.toml`, is the preset with four lines: `hardness` 1.2, `hardness_slope_top` 0.035 (0.15), `losses.b2` 6.0e-8 (2.0e-7), `losses.b2_top` 2.0e-7 from C6.

| | the preset | the candidate |
|---|---|---|
| ears against the real grand, whole / smooth: A0–G#1 | 8.3 / 4.2 | 8.2 / 4.0 |
| A1–G#2 | 9.2 / 6.6 | 7.8 / 4.3 |
| A2–G#3 | 9.1 / 6.1 | 8.0 / 3.7 |
| A3–G#4 | 8.8 / 5.3 | 7.7 / 3.2 |
| A4–G#5 | 8.0 / 2.9 | 7.4 / 2.6 |
| A5–G#6 | 4.6 / 1.2 | 4.4 / 1.0 |
| bench, eight real grands: overall | 1.98 | **1.75** |
| bass, middle, treble, top | 1.50, 1.70, 1.83, 2.81 | **1.09, 1.17**, 1.87, 2.64 |
| distance to the commercial D, 88 keys: overall | 9.46 | 9.83 |
| bass, middle, treble, top | 8.70, 9.86, 9.69, 9.54 | 8.67, 10.61, 10.97, 8.88 |
| growth mf → ff of 2–4 kHz, A0–B1 (the D 35.5) | 31.8 | 22.0 |

(The ears' and the bench's figures are those of the candidate before the top felt was softened for the first, which only changes keys above C6, and after it for the second.)

The two real references agree, and they are independent: one grand at the player's seat, eight grands through close microphones. The commercial D disagrees in the middle and the treble, and on the bass's growth from mf to ff, which is where the preset's soft felt came from. By the rule of this document the real pianos have it; it is the user's call, since the preset was built to follow that D. Heard so far on the bass only: `out/ab_bass_gaps/` section 12 has the candidate over the whole keyboard, against the real grand at the player's seat and against the D through the close microphones.

Not reached by it: A0–G#1 (8.3 → 8.2), the law local to the bridge, and the fine structure everywhere.

**In the preset (2026-11-13).** Heard over the whole keyboard against the real grand and the commercial D, the candidate was taken: `concert_grand.toml` has its four lines. Its renders are those of `out/presets/cand4t035.toml`, bit for bit. docs/steinway_d.md and docs/steinway_d_gaps.md still give the figures of the preset before it.

## At the keyboard (2026-11-14)

Played from a keyboard, the preset of 2026-11-13 was heard as much better at the attack and as strings that buzz, "a little like an Asian instrument". The single held notes of the listening page did not show it to the user: it was heard while playing only. Nine presets in the instrument's own list, switched while playing, each one line or two from the preset:

| felt | string loss | heard |
|---|---|---|
| × 4 | 6.0e-8 up to C5 (the preset of 2026-11-13) | buzzes |
| × 4 | 6.0e-8, 2.0e-7 coming in from C5, from F#4, from C4 | buzz, all three |
| the former felt | 6.0e-8 | buzzes |
| × 4 | 1.0e-7 | buzzes |
| × 4 | 2.0e-7 on every key | does not |
| × 2 | 2.0e-7 on every key | does not |
| the former felt | 2.0e-7 (the preset of 2026-11-12) | does not |

The buzz is the string loss alone, on every felt, and half of the way back (1.0e-7) is not enough. The harder felt is not in it, and its attack stays.

**In the preset.** `losses.b2` is back at 2.0e-7 on every key; `hardness` 1.2 and `hardness_slope_top` 0.035 stay. `losses.b2_top` is no longer used by the preset (the parameter stays, 0 by default). Its renders are those of `out/presets/fz_b220.toml`.

| | 2026-11-12 | 2026-11-13 | now |
|---|---|---|---|
| ears against the real grand, whole / smooth: A0–G#1 | 8.3 / 4.2 | 8.2 / 4.0 | 8.3 / 4.3 |
| A1–G#2 | 9.2 / 6.6 | 7.8 / 4.3 | 8.6 / 5.7 |
| A2–G#3 | 9.1 / 6.1 | 8.0 / 3.7 | 8.8 / 5.3 |
| A3–G#4 | 8.8 / 5.3 | 7.7 / 3.2 | 8.5 / 4.6 |
| A4–G#5 | 8.0 / 2.9 | 7.4 / 2.6 | 7.8 / 2.8 |
| A5–G#6 | 4.6 / 1.2 | 4.5 / 1.1 | 4.5 / 1.1 |
| bench, eight real grands: overall | 1.98 | 1.75 | 1.84 |
| bass, middle, treble, top | 1.50, 1.70, 1.83, 2.81 | 1.09, 1.17, 1.87, 2.64 | 1.39, 1.31, 1.93, 2.64 |
| distance to the commercial D, 88 keys: overall | 9.46 | 9.83 | 9.53 |
| bass, middle, treble, top | 8.70, 9.86, 9.69, 9.54 | 8.67, 10.61, 10.97, 8.88 | 8.71, 10.08, 10.34, 8.88 |
| growth mf → ff of 2–4 kHz, A0–B1 (the D 35.5) | 31.8 | 22.0 | 22.0 |

**What this says of the measures.** The per-partial gap at the ears and the bench both preferred the low string loss, and the ear at the keyboard refused it. Both read levels over the first second or so of single notes, where the attack weighs most. What the low loss changes is later: the share of the sound above 2.5 kHz between 1 and 2 s, model − real grand at the ears, mean over every third key:

| | A0–F#2 | A2–F#3 | A3–F#5 | A5–C7 |
|---|---|---|---|---|
| 2026-11-12 | −7.6 | −13.8 | −1.6 | −7.0 |
| 2026-11-13 | +2.0 | +1.3 | **+8.7** | −6.3 |
| now | −7.6 | −11.6 | −0.1 | −6.6 |

From A3 to F#5 the low loss holds 5 to 12 dB more highs than the real grand after a second (C4 +11, C5 +11, D#5 +12), and the preset's loss is right there. Below A3 this measure does not agree with the ear: it reads the low loss as right and 2.0e-7 as 8 to 14 dB short, while the three presets that kept the low loss below C4, F#4 or C5 only were all heard as buzzing. Not understood; single held notes at one velocity through a dummy head are not what was played (chords, repeated notes, the instrument's own listening set-up). A measure of the buzz that agrees with the keyboard is still to find, and until then a string-loss change is to be played before it is kept.

## The bass bridge's coupling: the board as it is drawn (2026-11-14)

Item 5 of the ranking, taken alone. The cause is found, and it is not in the bass bridge: the model's ribs were those of a simplified mesh, not the D's, and three more things about the bridge and its strings were missing. With them the board has the real grand's three resonances and the bass strings lose what the real ones lose. Measured as a candidate beside the preset of that morning, then taken into the preset the same day, after it was played at the keyboard ("In the preset", below). "The preset" in this section's tables is the one before it.

### The gap again, and how firm it is

`bass_loss` on the preset and the fit of section 2 give back section 2's table to the digit. The real side is firm: the decay of each partial of A0–B2 below 320 Hz (0.15–1 s, at the bridge), read on three recordings of the same grand:

| | 45–160 Hz (59 partials) | 160–320 Hz (81) |
|---|---|---|
| two takes of the same day | 0.7 dB rms | 0.2 dB rms |
| new, and after a year in a hall | 1.8 dB rms | 2.9 dB rms |

The three resonances stand where they stood a year before: D1's second partial (73 Hz) loses 10.2 and 8.2 dB/s, C1's third (97.6 Hz) 23.8 and 25.3, A0's fifth (137 Hz) 39.4 and 39.2. And they are the board's own: the same grand's bridge mobility, measured with a hammer at each stage of its building (Plath 2019, below), has its first three resonances at 74, 98 and 136 Hz once strung.

The factors 7 and 2.3 are less firm than the decays. They come from one resonance fitted on eight partials per band, and in the first band the fit gives the preset a loss factor of 0.32 that no mode has, so that its area spreads outside the band. Without a fit, the sum of the single strings' decays in each band (A0–E1, dB/s, the preset's being its budget, own loss and bridge):

| | 60–88 Hz | 88–120 Hz | 120–160 Hz | 60–160 Hz |
|---|---|---|---|---|
| the real grand | 45 | 60 | 102 | 207 |
| the preset | 120 | 114 | 76 | 310 |
| preset / real | 2.7 | 1.9 | 0.75 | 1.5 |

So: two to three times too much from 60 to 120 Hz, a quarter too little at 120–160 Hz, and 1.5 times in all. The same as a mass, which is what a string sees of a mode (1/Φ² at its bridge point, the mode being mass-normalised; from the fits' areas for the real grand, B0–D1): 15 kg at 71–73 Hz, 5 kg at 97 Hz and 4 kg at 139 Hz, where the preset shows C1 5 kg at 76 Hz, 3 kg at 94 Hz, 10 kg at 128 Hz, and three more modes in the same span. The fault is as much the number of modes the bass bridge sees, six below 140 Hz against three, as how hard each is coupled.

### What the sources say of this piano

The real grand is the one of N. Plath's dissertation (*From Workshop to Concert Hall: Acoustic Observations on a Grand Piano under Construction*, Hamburg 2019), measured at every stage of its building; its maker is not named there, the plate in the data set's photograph reads Steinway, and its board has the D's 17 ribs and 20 keys on the bass bridge.

| | first three resonances (Hz) | mean mobility at 15 bridge points, 50–1000 Hz (s/kg) |
|---|---|---|
| board glued in the rim, no plate, no strings | 56, 80, 114 | 0.8e-3 |
| plate and strings on | 74, 98, 136 | 0.7e-3 |
| regulated and tuned | 76, 106, 149 | 0.7e-3 |

(Read off its figures 6.8 and 6.9, to 2 Hz and 0.1e-3.) Its text: stringing raises the resonances by about 20 Hz up to 350 Hz and takes 10 dB off their peaks; the second resonance is a dipole whose nodal line crosses the long bridge at its key 34 (F#3); at the long bridge's bass end the first resonance hardly shows once strung. Its board: panel 7.9 kg, ribs 3.1 kg once feathered, bridge 3.15 kg in one piece, 13.7 kg glued in.

The D's board as built is drawn, with dimensions, in Boutillon, Ege and Paulello, "Comparison of the vibroacoustical characteristics of different pianos" (Acoustics 2012, arXiv 1210.3948, figure 2): 17 ribs from rim to rim, 17 to 27 mm high and 21 to 26 mm wide, 80 to 119 mm of clear gap between two, the bridge in one piece that turns round at the tail and comes back as the bass bridge, 29 to 37.5 mm high along the long bridge and 60 mm on the bass one, and a bar across the bass front corner. Plath's own drawing has the ribs to the rim and the bridge in one piece (its bend, about 20 cm, not glued to the board), and no bar; he measured the ribs' ends, feathered from full height to 4 mm as 19.55·exp(−0.024·d) + 4 mm.

The model's board came from RT-425's mesh (docs/real_board.md), which is a simplification: 16 ribs 114 to 146 mm apart that stop 10 to 20 cm short of the rim at both ends and leave the bass front corner bare, and bridge heights read off a colour map (55–70 mm on the long bridge, 52–60 on the bass one). The same mesh has the bridge in one piece; the model had cut it in two.

### The candidates, each with its test

| Candidate | Test | Result |
|---|---|---|
| The bridge's mobility against published measurements | mean of \|Y\| over 50–1000 Hz at Plath's fifteen points (`bass_shapes`) | the preset 0.93e-3 s/kg, 0.96e-3 unstrung, against 0.7e-3 and 0.8e-3 measured: a third too mobile on the whole, and 2.0e-3 on the bass bridge at 60–160 Hz. No strung bass-bridge curve of a grand is published; Conklin's (E2, a concert grand) has no stated reference |
| The modes' shapes and masses at the bass bridge | maps of the first modes (`bass_shapes`, `MAP=dir`) | **the cause.** The preset's first two modes, 68 and 76 Hz, have their bellies in the bass front corner, where the mesh has no rib (1.4–1.5 kg there), and its third, 94 Hz, in the tail's corner, on A0's bridge point (2.0 kg), beside the last rib's end and the free end of the bass bridge. None of the three is a real board's first mode |
| The ribs: to the rim, and the D's own | the mesh's ribs carried on to the rim along their lines; then the drawing's 17 | the mesh's, longer: three modes left under 140 Hz (77, 91, 120 Hz) but the bridge still sees 8, 2 and 3 kg. The drawing's, feathered: 75, 96, 130 Hz and 32, 4, 3 kg at A0; the sum of 60–120 Hz falls from 234 to 117 dB/s (real 105). **Kept** |
| The bridge in one piece | an arc from the long bridge's lowest key to A0, 10 cm off their chord | alone on the preset: the tail mode's coupling at 88–160 Hz down a fifth (190 → 152 dB/s), the first band hardly (120 → 96). With the drawing's ribs: 4 → 6 kg at 98 Hz. **Kept**: it is the D's, and the smaller of the two |
| The bridge's own height and mass | the drawing's heights; ±25 % of thickness on the preset (section 3) | the bass bridge's own section does little (section 3). The long bridge at 29–37.5 mm instead of 46–61 lowers the third resonance (130 → 122 Hz): on its own it moves away from the real 138. **Kept** as the D's dimension, with the next line |
| What the strings do to the board besides the spring T·(1/L + 1/l) | Plath's stages: stringing raises 56, 80, 114 Hz by 18, 18 and 22 Hz. A spring under the bridge raises each mode by the same ω², so less in hertz the higher the mode: the model gave +19, +16, +12 | **a term was missing.** A string pinned on the bridge is stretched when the bridge's top moves along it, that is when the board tilts under the bridge: E·A·(1/L + 1/l)·h² per radian, h the top's height over the stack's neutral plane. A stiffness on the slope, which grows with the mode as k², like the measured shifts. With it: +21, +21, +27 Hz, nothing fitted. **Kept** (`strings_grip`) |
| The bass bridge on a shelf | the sources | none on a D: glued straight to the board. Closed |
| The boundary at the rim | not redone | clamped, as before; the rim made two-way changed 2 modes in 227 (docs/real_board.md) |
| Crown and bearing | the board in its rim against Plath's unstrung stage | with the candidate's ribs and the 40 Hz of `crown_freq` fitted on another board: 57, 79, 110 Hz against 56, 80, 114. The foundation term is enough for these three modes; left as it is |
| The strings' rear lengths | the bass bridge's at 0.6 of the estimate, with and without the tenor's | first resonance +1 to +3 Hz, third +1 to +2, coupling of 60–88 Hz down a quarter. Not the cause; left at the estimate |
| A bar across the bass front corner | a bar 5 cm wide and 4 cm high where the drawing has it | +1 to +2 Hz, masses within a fifth. The two drawings disagree on it; left out |
| The two polarisations, the unison | not redone | measured before (docs/real_board.md): a real single string decays at one rate, and so it reads again here, early and late rates within 1 dB/s on most partials |
| Part of the loss being elsewhere | the budget | the excess was in the conductance itself, partial by partial; with the candidate the budget follows the real decay to 3.2 dB rms over 45–300 Hz (3.8 before, with a mean 2.1 dB too high) and no other sink is needed |

Step by step, the board's first modes with the mass each shows at A0, C1 and E1 (kg), and the sums of the first table (dB/s):

| | first modes: Hz (kg at A0, C1, E1) | 60–88 | 88–120 | 120–160 Hz |
|---|---|---|---|---|
| the real grand, from its strings | 71–73 (15), 97–98 (5), 138–139 (4) | 45 | 60 | 102 |
| the preset | 68 (45, 38, 31), 76 (6, 5, 5), 94 (2, 3, 5), 111, 128 (6, 10, 32), 135 (6, 21, –) | 120 | 114 | 76 |
| the mesh's ribs carried to the rim | 77 (8, 8, 8), 91 (2, 3, 5), 120 (3, 4, 10) | 105 | 123 | 86 |
| the bridge in one piece, alone | 68 (70, 50, 36), 77 (8, 7, 6), 95 (3, 3, 5), 112, 129 (8, 11, 37), 134 | 96 | 98 | 54 |
| the drawing's 17 ribs, feathered | 75 (32, 21, 14), 96 (4, 4, 5), 130 (3, 4, 11) | 43 | 74 | 86 |
| and the bridge in one piece | 75 (42, 24, 15), 98 (6, 5, 5), 130 (4, 5, 12) | 35 | 64 | 76 |
| and the drawing's bridge heights | 76 (54, 29, 17), 95 (7, 6, 6), 122 (4, 5, 12) | 34 | 71 | 66 |
| **and the strings' grip: the candidate** | 78 (23, 16, 12), 100 (7, 6, 7), 137 (4, 7, 18) | 37 | 53 | 78 |

The candidate's shapes are those the sources describe: one lobe at 78 Hz, weak at the long bridge's bass end (26 kg at F2); a dipole at 100 Hz whose nodal line crosses the long bridge at E3 (F#3 measured); three lobes along the board at 137 Hz. Its mean mobility at Plath's fifteen points is 0.76e-3 s/kg strung and 0.82e-3 in the rim (0.7e-3 and 0.8e-3 measured), its mass 12.7 kg on 2.09 m² (11.2 before; 13.7 for the real board with its margin on the rim).

### What changed in the code

`soundboard.structure`, every default as before, so that a preset without these lines renders bit for bit as it did (eight notes of the Concert grand compared):

- `rib_feather`, `rib_tip`: a rib keeps its middle thickness and is cut over its last `rib_feather` metres to `rib_tip` under the board, as measured; 0, the parabola.
- `bridge_ring`: the arc that joins the two bridges, by how far it stands from their chord; 0, two bridges.
- `strings_grip`: the strings' hold on the bridge's top, a stiffness on the board's slope along each key's strings, its steel section taken from the string's inharmonicity; off by default.
- `tools/steinway_d_ribs.py` places the drawing's ribs on the preset's outline; `bass_shapes` (piano-host's example) prints the first modes, the mass they show at the bridges, the mean mobility at Plath's points, and maps.

The candidate is the preset with these lines under `[soundboard.structure]` (`out/presets/cand.toml`; now the preset's own):

```toml
ribs = <tools/steinway_d_ribs.py>
rib_feather = 0.15
rib_width_treble = 0.0215
rib_width_bass = 0.0255
bridge_ring = 0.1
main_bridge_thickness = [[41.0, 0.0465], [49.0, 0.0455], [57.0, 0.0445], [65.0, 0.044], [77.0, 0.042], [89.0, 0.0395], [99.0, 0.0385], [108.0, 0.038]]
bass_bridge_thickness = [[21.0, 0.069], [32.0, 0.069], [40.0, 0.069]]
strings_grip = true
```

### Measured

**At the bridge against the real grand**, rendered, decay over 0.15–1 s (dB/s, median):

| | partials 2–4: A0–B0 | C1–B1 | A0–E1 | fundamentals of C2–B2 | A0–E1 by frequency: 45–70 | 70–100 | 100–140 Hz | F1–E2: 70–100 | 100–140 | F2–B2: 100–140 |
|---|---|---|---|---|---|---|---|---|---|---|
| the real grand | 3.2 | 5.7 | 4.3 | 9.6 | 3.0 | 6.7 | 3.9 | 8.5 | 5.9 | 4.7 |
| the preset | 11.6 | 8.8 | 10.3 | 18.4 | 4.5 | 20.7 | 10.3 | 24.3 | 18.8 | 14.4 |
| the candidate | 4.0 | 6.6 | 4.3 | 9.3 | 1.0 | 7.1 | 5.8 | 16.2 | 7.1 | 6.5 |

Partial by partial the budget follows the real resonances: 28.7 dB/s for A0's fifth (39.4 real, 21.8 before), 15.1 for C#1's fourth (22.9; 6.0), 14.5 for C1's third (23.8; 29.8), 4.7 for A#0's third (4.2; 22.4), 8.7 for D#1's second (4.5; 21.0). Fitted as in section 2: 78.5 Hz, η 0.21, strength 128; 99.4 Hz, η 0.13, 186; 136.7 Hz, η 0.07, 204 (real 74, 232, 295; the preset 544, 528, 248). The candidate is now a little under the real coupling at 97 and 139 Hz, and its first resonance 5 Hz too high: the second partials of A0–C#1 (55–69 Hz) decay at 0.7–2.9 dB/s against 1.5–8.4.

Drop of the sound at the ears between 0.2 and 2 s (dB): A0–B0 9.3 (real 11.5, the preset 14.2), C1–B1 11.0 (9.1; 12.6), C2–B2 15.2 (13.6; 16.4).

**Level at the bridge**, model − real, 0–0.3 s (dB): the excess of the low partials at the bridge, which the transfer to the ears undid (section 2), is mostly gone on the lowest keys.

| | partial 1: bridge / to the ears / ears | partials 2–4 | partials 5–8 |
|---|---|---|---|
| A0–B0, the preset | +10.7 / −23.7 / −13.1 | +8.7 / −8.4 / +0.3 | +7.0 / −6.2 / +0.8 |
| A0–B0, the candidate | +1.3 / −30.3 / −29.0 | +1.1 / +1.3 / +2.5 | +3.0 / −3.8 / −0.8 |
| C1–B1, the preset | +4.8 / −7.2 / −2.4 | +7.9 / −8.6 / −0.7 | +10.1 / −6.9 / +3.2 |
| C1–B1, the candidate | −0.4 / −8.9 / −9.2 | +5.7 / −4.7 / +0.9 | +4.5 / −2.3 / +2.2 |
| C2–B2, the preset | +5.9 / −2.8 / +3.1 | +3.9 / −4.5 / −0.5 | +2.9 / −2.2 / +0.7 |
| C2–B2, the candidate | +2.3 / +1.3 / +3.6 | +4.4 / −3.1 / +1.3 | +2.5 / +1.6 / +4.0 |

**The price: the lowest fundamentals at the ears.** The bridge now moves as the real one at the fundamentals of A0–B1, and what the transfer lacked below 90 Hz is no longer hidden: A0–B0's fundamental (27–31 Hz) is 29 dB under the real grand's at the ears (13 before), C1–B1's 9 dB under (2 before). That is the radiation towards the player at the bottom (the back face cancelling too much, section 2; item 4 of docs/steinway_d_gaps.md), not the board's modes; how far a chamber and a dummy head can be trusted at 30 Hz is not known either. To be judged by ear.

**The per-partial gap at the ears** against the real grand, whole / smooth (dB rms), every key:

| | A0–G#1 | A1–G#2 | A2–G#3 | A3–G#4 | A4–G#5 | A5–G#6 |
|---|---|---|---|---|---|---|
| the preset | 8.3 / 4.3 | 8.6 / 5.7 | 8.8 / 5.3 | 8.5 / 4.6 | 7.8 / 2.8 | 4.5 / 1.1 |
| the candidate | 7.5 / 4.1 | 9.0 / 6.0 | 8.2 / 4.6 | 8.4 / 4.8 | 8.3 / 3.0 | 5.2 / 1.5 |

The first move of A0–G#1 on this measure since it was set up (8.3 → 7.5, the yardstick being 4.9), most of it in the fine structure. Above, it goes both ways by what a redraw of the board gives (docs/real_board.md): the new board is another draw of every partial's level.

**The bench and the commercial D**, all 88 keys, the preset measured again beside it (1.84 and 9.53, as recorded):

| | overall | bass | middle | treble | top |
|---|---|---|---|---|---|
| bench, the preset | 1.84 | 1.39 | 1.31 | 1.93 | 2.64 |
| bench, the candidate | **1.75** | 1.41 | 1.38 | **1.66** | **2.50** |
| distance to the commercial D, the preset | 9.53 | 8.71 | 10.08 | 10.34 | 8.88 |
| distance to the commercial D, the candidate | **8.60** | 8.38 | **9.21** | **9.04** | **7.67** |

On the bench the decay family goes from 1.38 to 0.97 and the attack from 1.83 to 1.55; the sustain from 1.21 to 1.61, the resonance from 2.34 to 2.74 and the dynamics from 1.26 to 1.55. Nearer: the treble's envelope at 1–3 s (z −4.2 → −2.0 at mf), its 1 kHz band in the sustain (−5.4 → −3.6), the top's third partial at the attack (+4.4 → +1.6), the bass's balance. Further: the top's balance (−2.1 → −3.7), the noise between the treble's partials (−2.1 → −3.3), the bass's partials 4 and 6 against its fundamental in the sustain (−0.8 → −1.8, −1.5 → −2.5: the fundamental holds, as on the real grand, and these measures are ratios to it). The bass register does not move: it never read the 70–140 Hz decays (section 1). Against the commercial D every register is nearer, the treble and the top by more than a redraw.

### Listening

`out/ab_bass_bridge/index.html` (the `ecoute` entry of `.claude/launch.json`): every key from A0 to B2 at the player's seat, lid off, no room, the real grand, the preset and the candidate, one gain for the two models and the real grand at the same mean level; then eleven keys through the close microphones at velocities 100 and 60 with the commercial D. `out/keyboard/instruments/Bass bridge trial/` holds the two presets for the instrument's own list (`target/release/unsampled-piano --ui --presets out/keyboard`).

### In the preset (2026-11-14)

The two presets were put in the instrument's list and played from the keyboard, and the user had the candidate taken into the preset. `concert_grand.toml` has its eight lines; its renders are the measured candidate's, bit for bit (four notes compared). What was heard is not recorded here: no remark came with the decision, on the lowest fundamentals or on anything else. docs/steinway_d.md still gives the figures of the presets before it.

### Not verified, for this section

- **What was heard.** The candidate was played and kept, without a remark to record; the listening page's verdict key by key, and the lowest fundamentals at the ears, are not on record.
- **The outline.** Placed on the preset's outline with the 17th rib at its own length, ribs 1–13 come out 10 to 15 cm longer than the drawing's and ribs 14–17 within 6 cm: the model's board is wider at the bentside than the drawing's inner rim, by some 8 cm. RT-425's outline may follow the case rather than the inner rim. Not corrected: it moves every bridge point's distance to the rim.
- **The ribs' angle**, 50° from the spine (Plath: 40° from the keyboard's line), and the feather's length, 15 cm (the drawing's marks, 15–19 cm on the long ribs, 5–9 on the short), are read, not measured. One length for all ribs, a quarter of the rib at most.
- **The long bridge's heights by key** are the drawing's five figures placed along the keys by eye; its width stays 3 cm (3.5 cm changes nothing here).
- **The ring's shape**: an arc 10 cm off the chord, glued all along, where the real bend has 20 cm not glued to the board.
- **The strings' grip** takes the string as held on the bridge without slipping, its two lengths as springs (true below their own longitudinal resonances, some hundreds of hertz for the longest wound strings), the steel section from the inharmonicity (the winding adds a little to it), and the bass strings as running along the board's length. As a moment at a point it acts a little under its first-order value on a bare plate (a unit test has it at 0.6 to 1). Its measure is the three shifts of one piano.
- **The first resonance is 5 Hz too high** (78 against 71–73 as the strings see it, 74 as measured after stringing), and the tuned state's 76, 106, 149 Hz, read off a figure, are not what the strings of the same state show (73, 98, 138): not understood.
- **The strung bass bridge's mobility** was never measured directly on a grand in what was found; the real side is the strings' decay.
- **The bass's buzz** (section "At the keyboard") was not looked for: no string loss was changed.
- **The radiation below 90 Hz**, uncovered above.

## Not verified

- The levers of section 3 were heard as nearly alike; see "After listening".
- The real grand is one instrument, at one dynamic, in one state (PROD08, take 2): its unisons and its low resonances are its own. The bench's eight grands confirm the decays' direction, not the figures.
- The real resonances are fitted on eight or nine partials each, from eight keys along 30 cm of bridge: the loss factors are good to a third, the strengths to a factor of 1.5.
- Which of the bridge and the radiation is wrong above 1.5 kHz (section 2).
- The cost of `string_load` on the evenness from key to key, and of `unison.width_mid` on the middle by ear.
- The unison's width was read here on dips over 0.15–1.6 s with long windows: docs/bass_unisons.md reads the real grand's two tunings and its five seconds, and finds that a narrow unison uncovers a decay that is too fast.
- The dynamics against a real grand: the bench's Disklavier only.

## Tools

`tools/bass_gaps.py d <D's notes> name=dir …` and `tools/bass_gaps.py plath <Plath's data> <cache> name=preset.toml …` print every table of section 1. `bass_loss` (piano-host's example) gives the budget of section 2, with `CSV=1` for a file. `bass_shapes` (the same crate's) gives the first modes as the bridges see them, and `tools/steinway_d_ribs.py` the D's ribs from its drawing. The last section's own scripts are in this session's `out/tools/` (`realdec.py`, `cmp.py`, `run.sh`, `measure.sh`, `plotmodes.py`, `layout.py`), with the sources' figures in `out/lit/` and the literature notes in `out/literature.md`. The scripts that make the variants, fit the resonances and build the listening page are in `out/tools/` (not versioned): `mk.py`, `trial.sh`, `summ.py`, `resfit.py`, `lowsum.py`, `dips.py`, `tfpos.py`, `abpage.py`.
