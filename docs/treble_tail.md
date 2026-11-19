# The treble's tail

Status as of 2026-10-29.

## What was heard

"Notes sound like a string buzzing slightly, and the sound is a bit mid-rangey… On strong strikes, as if harmonics saturated… In fact the note's tail is far too loud: on A5, against the commercial piano, the tail is at the same volume while the attack is much louder."

A5 at velocity 100, the user's sound (concert microphones), level re the attack's peak:

| | 0.3 s | 0.6 s | 1 s | 1.5 s | 2 s |
|---|---|---|---|---|---|
| commercial modelled piano | −19 | −29 | −38 | −39 | −36 dB |
| Salamander (real) | −12 | −20 | −27 | −31 | −33 dB |
| model | −10 | −22 | −21 | −25 | −22 dB |

From 0.6 s on, the model's tail stayed at −21 to −25 dB, beating with a period of about a second.

## Where it came from

The model's renderer was built at each commit of the last two days, and the same notes were rendered with the user's sound. The tail appeared with 90bc8f9, the board's outline in the physics. The user's sound already held the grand's outline, so it became active then; it has been on by default since bf23502. With the rectangle, A5 fell to −31 dB at 1 s and −33 at 2 s.

The sympathetic strings, the frame, its coupling, the rim, the top's undamped strings, the strings' nonlinearity, the polarisation and the strike's evenness each change A5's tail by less than 2 dB.

**The outlined board takes less of the treble's fundamentals.** Bridge conductance, outline re rectangle (same ρh, D and slopes; `examples/frame_drain.rs`, `RECT=1`):

| key | C5 | E5 | G5 | A5 | B5 | D6 | F#6 |
|---|---|---|---|---|---|---|---|
| partial 1 | −8.0 | −3.0 | −5.1 | −5.2 | −5.1 | −3.1 | −3.5 dB |
| partial 2 | +2.6 | −1.3 | −0.3 | 0.0 | 0.0 | 0.0 | 0.0 dB |

With the board's own modes up to 3 kHz instead of 1.5, partial 2 loses 0–2 dB and partial 1 the same: the outlined plate is genuinely less mobile there below 1.5 kHz. The nearest edge of the treble bridge is the front one in both boards, 6–21 cm away.

Less conductance couples a unison's strings less to one another. Their mistuning, 1.8 cents at A5, then dominates: they beat instead of decaying together into a prompt sound and an aftersound. Tightening the unisons removes the plateau (`width_mid` 0.5, `width_treble` 2.5: A5 −29 dB at 1 s, −35 at 2 s).

## The change

The unisons' mistuning grew from `width_mid` (1 cent up to C4) to `width_treble` (5 cents at C8) as t², with t = (key − C4)/4 octaves. Lowering `width_treble` mends the treble, but the top (C7–C8) loses its attack's 2–4 kHz and its envelope at 0.3 s. The new `unison.width_curve` sets the exponent instead. With 4, A5 is at 1.3 cents (1.8 before), C7 at 2.3 (3.25), and C8 keeps its 5.

| | score | off | treble env ff 1 s / 2 s (z) | treble env mf / pp 1 s (z) | top attack 2 kHz band (z) |
|---|---|---|---|---|---|
| t² (before) | 1.11 | 12 | +1.56 / +1.56 | +0.91 / +0.85 | −2.69 |
| width_treble 3 | 1.12 | 10 | — / +0.45 | — | −4.15 |
| width_treble 2 | 1.14 | 12 | +0.35 / +0.66 | −0.21 / −0.06 | −4.79 |
| width_mid 0.5, width_treble 2.5 | 1.17 | 12 | | | |
| t³ | 1.11 | 10 | +0.97 / +0.36 | +0.31 / +0.31 | −3.35 |
| **t⁴** | **1.10** | **10** | **+0.67 / +0.36** | **+0.06 / +0.13** | −3.88 |
| t⁶ | 1.14 | 10 | | | |

The default is t⁴. On the user's sound A5 now reads −24 dB at 1 s, −28 at 1.5 s and −39 at 2 s, against −21, −25 and −22 before. The tail decays instead of holding.

The honky-tonk preset keeps the quadratic curve (its unisons are meant to be far apart all along).

## Also seen

At the concert microphones (the user's set-up), a strong strike's attack carries 10 dB more at 2–4 kHz re its fundamental than Salamander's at C4 (−4.5 against −14.7 dB). With the close microphones it is −14.2, Salamander's. The board's radiation at that elevation and distance colours it; neither the lid, the room, a cardioid pattern nor the scattering layer changes it (the scattering raises it). Not changed here.

## 2026-11-17: the aftersound too weak since the real board

**What the comparison tool showed.** From C5 to C#6, partial 1's ripple (its RMS distance from a straight decay over 0.1–3 s) was 1.5–3 dB above the real grands' at pp and mf. The cause was not beats. Partial 1 fell in a fast prompt sound (E5–C6: −50 to −70 dB/s for the model's fast mode, against about −30 to −40 on Steinway D, Salamander and the commercial grand) into an aftersound 15–20 dB under theirs:

| Key | Model | Real grands |
|---|---|---|
| F5 | −38.5 dB at 1 s | −19 to −27 |
| A5 | −40 dB at 1 s | −23 to −38.5 |

With the real board, the October trade-off (t⁴, `width_treble` 2) had gone too far the other way.

**Levers tried.** E5–D6 at mf. The table gives the median gap to the real grands of the overall level at 1 s and at 2 s (dB), then of partial 1's ripple (dB):

| | 1 s | 2 s | Ripple |
|---|---|---|---|
| As before | −7.4 | −11.7 | +0.7 |
| `width_mid` 1.5 | −2.5 | −4.6 | +0.1 |
| `width_mid` 2.0 | +0.7 | −9.8 (they beat) | −0.4 |
| `hit_spread` 0.6 | −8.2 | −10.1 | +1.4 |
| `hammer_tilt` 0.15 | −9.0 | −10.7 | +0.6 |
| polarization `radiation` 0.3 | −7.8 | −13.5 | +0.5 |
| **`width_curve` 1** | **−2.7** | **−5.7** | **+0.1** |

Only the unisons' spread gives the aftersound its level. `width_mid` would also change the middle, whose tail at 2 s it makes 2.5 dB too loud. The linear curve (C5 1.25 cents, C6 1.5, C7 1.75) leaves the middle alone: C4–B4 −1.4 / −1.1, as before. It also brings E6–D7 at 1 s from −5.3 to −2.4.

**Result in the tool.**

| Treble (C5–B6) | Before | After |
|---|---|---|
| Ripple, measures off | 19 | 11 |
| T60 of partial 1, measures off | 4 | 2 |
| Early decay, median z | −0.9 | +0.1 |
| Late decay (1–3 s), measures off | 23 | 33 |

The late decay rises against references whose tail is mostly their noise floor. Against the clean recordings, B5–E6 decay at −12 to −17 dB/s, against −8 to −13 for Steinway D and the commercial grand and −6 to −10 for Helgesson; before the change the model decayed at −4 to −10. In all, 298 measures are off, against 294.

**The middle, checked.** The comparison tool measures each microphone separately. On it, the middle's partial 1 was already in the real grands' range:

| Key | Model | Real grands |
|---|---|---|
| E4 | 3.5 dB | 5.3 |
| G4 | 4.4 dB | 4.1 |
| A4 | 3.7 dB | 4.2 |
| B4 | 3.2 dB | 4.5 |

Only C4 and C#4 were low, at 1.1 and 2.6 dB against 4.5–4.9. (A first reading of 0.2–0.4 dB averaged the two microphones, which cancel each other there.)

`width_mid` 1.5 (trial) brought C4 to 2.7 dB and cut the treble's ripple measures off from 11 to 4, since it also widens the treble. But through the bass ramp it widens C3–B3, where the ripple rose to 5–8 dB on A3–B3 against about 1 dB. Over the whole tool, the middle's ripple measures off went from 27 to 30 and the total from 298 to 300. Not kept.

## 2026-11-18: the fundamentals of C#5–G#5 under their upper partials

**What was wrong.** From C#5 to G#5, the sustained sound (0.5–1 s) was far too rich. Its spectral centroid at mf was 2.05 times the fundamental on D#5, 2.88 on E5 and 2.35 on F5, against 1.02–1.05 on the real grands. The comparison tool does not flag it: this measure's tolerance is too wide.

**Not the microphones.** The gap stays with the close, concert and player microphones, with and without the room.

**Two causes.** Levels are re partial 1 at 0.03–0.13 s, model minus the real grands' median, C#5–G#5 at mf, close microphones:

- Partial 1 is 8 to 11 dB too low from 0.33 to 2.4 s.
- Partials 3 and 4 are 10 and 12 dB too strong from the attack on. C5 and A5–B5 show the same excess.

At the bridge, against Plath's grand, the fundamentals of C#5, D#5, E5, F5 and G#5 decay at −29 to −42 dB/s, against −19 to −30. The board's conductance over C5–B5 is 4.1–9.7e-4 s/kg at the fundamentals, about 5e-4 on its third-octave mean, against 3.1e-4 for the fit of `tools/bridge_conductance.py`.

**Levers tried.** C#5–G#5 at mf, median over the keys. Partial 1 is model minus real (dB); the real centroid is 1.03.

| | Partial 1, 0.6–1 s | Partial 1, 1–1.6 s | Centroid, 0.5–1 s |
|---|---|---|---|
| As before | −11.4 | −9.2 | 2.06 |
| `radiation` 0.3 | −7.5 | −9.5 | 1.81 |
| **`radiation` 0.6** | **−3.9** | **−9.7** | **1.49** |
| `h_coupling` 0.12, `cross` at its limit | −13.4 | −8.5 | 1.96 |
| `detune_cents` 1 | −11.1 | −8.8 | 2.05 |
| `hammer_tilt` 0.2 | −8.3 | −10.1 | 1.85 |
| `hit_spread` 0.6 | −9.0 | −8.3 | 2.04 |
| `width_curve` 0.7 | −9.3 | −8.8 | 2.02 |
| `bridge_mass` 0.3 kg | −6.7 | −1.4 | 1.73 |

`bridge_mass` lowers the conductance, so it mends partial 1, but more and more with frequency. The upper partials then last longer too. Over the whole keyboard the centroid of C5–A#5 rose from 1.64 to 1.90, and the between-partials measures off went from 9 to 34. Not kept.

**Kept: `radiation` 0.6**, after listening. Sideways motion decays more slowly than vertical motion, so radiating more of it holds up the fundamental and leaves the upper partials alone.

| Whole keyboard, measures off | Before | After |
|---|---|---|
| Partial 1 ripple | 73 | 61 |
| Late decay | 68 | 59 |
| Early decay | 47 | 46 |
| Rise time | 39 | 38 |
| T60 of partial 1 | 30 | 26 |
| Spectral tilt | 21 | 23 |
| Tuning | 10 | 11 |
| Between partials | 9 | 9 |
| Inharmonicity | 1 | 0 |
| **Total** | **298** | **273** |

The sustained centroid at mf:

| Key | Before | After | Real grands |
|---|---|---|---|
| C#5 | 2.07 | 1.57 | 1.43 |
| D#5 | 2.05 | 1.84 | 1.02 |
| E5 | 2.88 | 2.66 | 1.05 |
| F5 | 2.35 | 1.35 | 1.03 |
| G#5 | 1.64 | 1.42 | 1.02 |
| A5 | 1.27 | 1.10 | 1.04 |

**Still open.**

- D#5 and E5 stay too rich: their partials 2 to 4 are too strong from the attack.
- D4, F4 and A4 get richer, and they already were. A4 at ff goes from 2.09 to 2.81, against 1.60.
- C5–A#5 now decay too slowly over 0.1–1 s: −21.3 dB/s, against −26.3.
- The late decay of B5–E6 eases from −14.8 to −11.5 dB/s, against −4.6.

## 2026-11-18: D#5 and E5, the sideways coupling up to 700 Hz

**What was left.** With `radiation` 0.6, D#5 and E5 stayed the richest keys: a sustained centroid at mf of 1.84 and 2.66, against 1.02 and 1.05. At the attack (0.03–0.13 s, re partial 1, mf), D#5's partials 2 and 4 stood at −6.3 and −16.0 dB, against −17.6 to −30.7 and −27.5 to −50.5 on the real grands. E5's partial 3 stood at −8.7 dB, against −30.5 to −36.1.

**The transfer.** Ear pressure per bridge acceleration at the player's seat, model minus Plath's grand, keys C4–B6, median per third octave (dB):

| Hz | 500 | 630 | 800 | 1000 | 1250 | 1600 | 2000 | 2500 | 3150 | 4000 |
|---|---|---|---|---|---|---|---|---|---|---|
| Preset | +3.2 | +1.8 | +1.5 | −1.7 | +2.0 | +5.1 | +7.7 | +10.7 | +0.9 | −2.8 |
| No case field | +2.0 | +3.0 | +0.9 | −3.4 | +1.8 | +3.0 | +6.6 | +6.3 | −1.6 | −6.4 |

The fundamentals of C5–B5 are within about 5 dB of Plath's, and D#5's and E5's within 2.4. The excess lies at 1.6–2.5 kHz, where those keys' partials 3 and 4 fall. It is not settled: at the bridge, the same partials are below Plath's, so the two stages may partly make up for each other (the transfer's method is in `docs/case_radiation.md`). Nothing here corrects the transfer.

A softer felt does not reach these keys. With `hardness_slope` −0.12, D#5's partials 2 and 4 go from +18.3 and +20.6 dB over the real grands' median to +16.9 and +18.6, and E5's partial 3 from +22.8 to +21.9.

**What was done instead: more fundamental in the sideways motion.** The whole keyboard, three steps:

| | Measures off | D#5 | E5 | Why not, or why |
|---|---|---|---|---|
| `radiation` 0.6 | 273 | 1.84 | 2.66 | the start |
| `radiation` 1, `h_coupling` 0.12, `cross` at its limit | 312 | 1.38 | 2.04 | T60 of partial 1 off 7 → 35 in the middle: C3–B3 at 13.8 s, against 6.3 |
| `radiation` 0.8, `slope` 1.5 from C4 | 273 | 1.49 | 1.86 | F7 out of tune, A#6–B6's early decay 7 z too slow |
| **The same, `slope_to` 700 Hz** | **277** | **1.54** | **2.27** | **kept after listening** |

`polarization.slope_to` is new: above it, the sideways coupling keeps its value there (0: no limit). The coupling goes from 0.03 at C4 to 0.13 at 700 Hz.

| Measures off | Before | After |
|---|---|---|
| Partial 1 ripple | 61 | 57 |
| Late decay | 59 | 58 |
| Early decay | 46 | 47 |
| Rise time | 38 | 38 |
| T60 of partial 1 | 26 | 31 |
| Spectral tilt | 23 | 26 |
| Between partials | 9 | 11 |
| Tuning | 11 | 9 |

**Still open.**

- E5 at 2.27 and D#5 at 1.54 remain far from 1.05 and 1.02.
- C5–A#5 decay too slowly over 0.1–1 s: −17.5 dB/s, against −21.3 before and −26.3 on the real grands.
- C3–B3's partial 1 lasts 8.5 s, against 7.3 before and 6.3 on the real grands.
- F4 gets richer: 1.92 at mf, against 1.77 before and 1.10.
