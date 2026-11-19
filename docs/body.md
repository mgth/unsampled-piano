# Between the partials: the body's halo

Status as of 2026-11-17.

## What the real pianos have

The comparison tool's "Between partials" measure put the model far below every real piano, on nearly every key. Taking each partial out of a note (tools/between_partials.py: every partial found, followed in time and subtracted, down to A0) leaves what a piano does around its partials. Heard on its own, it carries much of each piano's character. The target chosen by ear is the Splendid Grand, a Steinway, which a recorded commercial modelled piano sounds close to.

In dB re the note's level over its first 100 ms, medians over the keys, at ff:

| | 125 Hz | 250 Hz | 500 Hz | 1 kHz | 2 kHz | 4 kHz |
|---|---|---|---|---|---|---|
| Splendid, 60–150 ms | −34 | −38 | −45 | −53 | −57 | −57 |
| Model, 60–150 ms | −48 | −48 | −54 | −58 | −55 | −51 |
| Splendid, 150–400 ms | −48 | −52 | −62 | −70 | −71 | −66 |
| Model, 150–400 ms | −74 | −80 | −89 | −75 | −70 | −64 |

- **Where the model falls short:** 125 Hz to 1 kHz, from 60 ms on, by up to 30 dB at 0.15–0.4 s, in every register. Above 2 kHz it is within about 5 dB. In the first 20–60 ms it is close.
- **A halo, not lines,** in the middle and treble: 13–34 % of Splendid's residual lies in narrow lines (0.1–0.6 s), against 85–91 % of the model's. It starts with the attack and dies within 0.6–1 s at 125 Hz–1 kHz, as on Plath's concert grand in an anechoic room: it is the instrument, not the hall.
- **Lines in the bass,** half-way between the partials: phantom partials and longitudinal modes (A1: a group at 1.2–1.6 kHz). The model has them 10 dB weaker. Splendid's halo there grows faster than the note: +12 dB re the note at 150–400 ms from mf to ff, against nothing in the middle.
- **The undamped top strings:** lines at the fundamentals of D♯6–B6 under 10–17 of Splendid's 38 keys below C6 (Plath's the same), where the model had them from G♯6.
- **Fixed lines of each body:** Splendid 76–104, 180 and 219 Hz; Plath 190–617 Hz; the model 76, 141 and 180 Hz.

### Heard

- A shaped noise fitted to the real level ("tac" of a plugged jack at the attack, "a horrible plop" below C4): the onset was a step at the level of 0.1 s plus 6 dB, on top of an attack that already had the right level; and in the bass the real energy is lines, not noise.
- A real piano's residual grafted on the model's partials, C4–C7: "it brings a lot of realism".

## The engine: three changes

1. **The body bank** (`crate::body`, `[body]`). 1024 damped resonances, 40 Hz–10 kHz, density uniform below 2 kHz (one every 3.5 Hz at the bottom, 5 Hz at 1 kHz) and falling as 1/f above, so that at T60 ≤ 1 s they overlap into a halo rather than a chord. All keys share it; each key strikes it with its own pattern of amplitudes and signs, half common to every key (`shared`). The strike is a 0.25 ms Hann pulse, `delay` after the hammer meets the string, of peak `gain` Pa per m/s of hammer speed, `slope` dB per octave of key. Level and decay per octave band (`levels`, `t60`, 63 Hz–8 kHz). Each microphone hears each resonance with its own weight and sign. Heard directly, before the room, like the action's knock (`crate::action`), which stays as it is. Its cost is fixed, whatever is played. Off by default.
2. **The last damper at D6** in the Concert grand (`damper.highest_note` 86, from 91).
3. **The phantom partials at Splendid's level** in the Concert grand (`nonlinear.coupling` 1.0, from 0.3): A1 within 2–3 dB of Splendid from 20 ms to 0.4 s at 500 Hz–2 kHz.

## Calibration on Splendid

For each octave band and window, the body must add Splendid's residual minus the model's (power); the body alone is the difference of two renders with and without it (the engine is deterministic and the body does not reach the strings). 14 keys A1–C7, ff, close microphones and studio.

```
gain = 0.05, slope = -3.0
levels = [-7, 0, -4, -12, -23, -28, -40, -40]
t60 = [1.6, 1.15, 0.9, 0.8, 0.8, 0.5, 0.3, 0.2]
```

With it, the model's residual minus Splendid's, medians over the keys:

| window | 63 Hz | 125 | 250 | 500 | 1 k | 2 k | 4 k |
|---|---|---|---|---|---|---|---|
| 0–20 ms | −1 | −3 | 0 | 1 | −3 | −5 | −7 |
| 20–60 ms | −4 | −1 | −4 | −2 | 1 | 1 | −1 |
| 60–150 ms | −2 | −5 | −1 | −3 | −2 | −1 | −1 |
| 150–400 ms | −1 | −1 | −1 | 1 | 0 | 1 | 1 |
| 400–1000 ms | −2 | −4 | −3 | −4 | −7 | −2 | −6 |

Key by key, within ±5 dB mostly. What is left is not the body's: C3–A3 5–10 dB short; C5–A5 and A6 5–20 dB over at 1–2 kHz (already without the body: the undamped top strings ring too loud under E5); C7 10–15 dB short. Over the first 40 ms the body alone stays 8–10 dB under the model's own residual, with no peak at the onset.

## Bench and cost

`tools/bench.py run` (close microphones, studio), the Concert grand before and after the three changes: score 1.79 → 1.38, 63 → 66 % of the measures within the real pianos, 32 → 26 off; brightness 3.05 → 1.71, resonance 2.31 → 1.61, the others within 0.03. The bench's own between-partials measures all come closer (bass early −1.37 → 0.03 z, late −1.15 → 0.03; middle early −2.56 → −1.92; treble late −2.10 → −1.13; top early −3.27 → −1.85), as do the top's sustained 1 kHz band (−13.5 → −3.5 z) and the treble's (−4.5 → 0.1). The largest losses are 0.1–0.7 z (bass p8 sustain, top early T60).

A 91 s Chopin nocturne renders in 22.4 s instead of 21.2 s (+5.5 %, the three changes together, two alternated runs).

Heard on the listening page `out/ab_corps/` (Splendid, the model before and after, the commercial D; notes, their residuals and a phrase): "much better in the treble"; in the bass the gain is more discreet but there, and something is still missing.

## The bass: the strings' inharmonicity

Heard after the body: much better in the treble, more discreet in the bass. Measured on 21 keys B0–B3 against Splendid: A0–B1 within 5 dB per octave and window, but at B0–E1 the 1 kHz octave 17–43 dB short, and the shape of the residual far from Splendid's (profile correlation, sixth-octave bands 0.8–5 kHz, 0.1–0.6 s, the top strings' lines left out: 0.44 at A0–B1).

- **Not the longitudinal modes' frequency.** The model's first longitudinal mode lies at 850 Hz at B0 and 2.0 kHz at C2 (core from the inharmonicity, the whole string's mass); the bumps first read as misplaced were its second mode. A factor on their frequencies (0.7, 0.8, 0.9, 1.1, all keys) does worse than 1.0 on average (correlation 0.33–0.38 against 0.42).
- **The inharmonicity.** Splendid's bass is twice as inharmonic as the model's, which had RT-425's D (Plath's D the same): A0 ×2.0, A1 ×2.2, A2 ×1.3, the same from C3. The model's phantom partials then fall within a quarter of the spacing from the partials, Splendid's half-way between them. The law fitted on its 60 keys, the treble kept (`a_bass` −0.0730, `b_bass` −6.901): correlation 0.44 → 0.52 at A0–B1 (B0–D1 and A1 from 0.2–0.4 to 0.6–0.7), level −6.4 → −4.3 dB. With the phantoms 3.5 dB louder (`coupling` 1.5) the level comes within 1.2 dB, but a share above 1 is not physical and the ear preferred without.
- **Bench:** 1.38 → 1.42, 66 → 68 % within, 26 off either way; the bass inharmonicity comes inside the real pianos (z −1.00 → 0.60), the bass p8 sustain closer (−1.56 → −0.71); the bass p5–12 early T60 (2.81 → 3.38) and the treble ff balance (−2.99 → −3.60) further.

## The bass: the first partial's decay

In the comparison tool (median z per register, 8 real grands, three layers), the bass first partial held too long (T60 z +1.0): 2–2.5 times the real grands' mean from F#1 to C#2 (A1 63 s against 31, C#2 15 against 6). `losses.b1_floor`, a loss on the wound strings fading out over the octave above them, at 0.1 puts F#1–A1 on that mean (42/39/37/33 s against 51/38/37/31); B1–C#2 stay long (21/17/13 s against 14/14/6); 0.2 falls under it at F#1–A1. It shortens every partial of those strings. Bench 1.42 either way: the bass's early T60 closer (p5–12 3.38 → 2.57 z, p1–4 0.52 → 0.04), its level at 2 s too (0.93 → 0.38 z at ff, about 0 at mf and pp); its level at 3 s at mf and pp goes under the mean (+0.3 → −0.5 z). Taken after listening.

Left: the first partial's ripple (z −1.3): its envelope is a straight line (0.1–0.3 dB from it, against 1–3 dB on the real grands, single strings included); no loss changes it.

## The bass: the first partial's double decay

On the real grands (Splendid, Plath, Helgesson, Salamander, BiVib; Liang's notes are released at 2 s) the ripple is mostly a double decay: the bass's first partial falls fast for about a second, then slowly (curvature of its envelope +1 to +5 dB/s² at A0–G1, +14 at Splendid's D2), single strings included; some add beats at 1–3 Hz. The vertical motion gives its energy to the board, the horizontal keeps it. In the model the horizontal motion was not heard: the cross term at its passive limit (0.1732 = √h_coupling) leaves the slow mode pushing nothing on the bridge, its radiation was 0.1 and the hammer's tilt 0.05. More loss through the rim (`edge_loss`) alone, or more tilt alone, only steepen the one straight line.

These three settings act on the whole keyboard, and changing them there moves the middle too (its fundamentals and its late decay), so `polarization` now has the bass's own values: `bass`, `bass_until`, `bass_fade`, `hammer_tilt_bass`, `radiation_bass`, `cross_bass` (`PolarizationLaw::at`). The Concert grand takes them to E2, blended into the rest by C3: tilt 0.6, radiation 1.0, cross 0.02, with `edge_loss` 1e-3 for the vertical motion's fast early decay and `b1_floor` back to 0. On A0–E2 against those five grands (rms of the envelope about a line, curvature, slope; mean distance to their median): 2.3 → 1.3 dB, 2.1 → 1.1 dB/s², slope as before; B0 still falls in one line, F1–A1 a little fast, D2 slow. The treble does not move; C4 loses 2 dB at 1 s, nearer the real grands.

Bench 1.42 → 1.50, 68 → 62 % within, 27 off: the bass's sustained partials 2–12 come closer (p4 −1.22 → −0.21 z), but its partials 1–4 decay too slowly early (T60 p1–4 early 0.04 → 4.97 z, late 0.15 → 1.64) and its level at 2 s stands over the real grands' (+1.2 to +1.5 z): the horizontal motion, now heard, holds too long without the wound strings' loss. Part of that loss given back, then (bench, all with the bass's own sideways values):

| `b1_floor` | `radiation_bass` | score | within | off | bass T60 p1–4 early (z) | bass level at 2 s, ff (z) |
|---|---|---|---|---|---|---|
| 0 | 1.0 | 1.50 | 62 % | 27 | +4.97 | +1.54 |
| 0.05 | 1.0 | 1.47 | 64 % | 27 | +3.82 | +1.26 |
| 0.1 | 1.0 | 1.45 | 64 % | 26 | +2.47 | +0.99 |
| 0.05 | 0.5 | 1.43 | 66 % | 26 | +1.21 | +0.64 |
| **0.1** | **0.5** | **1.43** | **67 %** | **25** | **+0.78** | **+0.36** |
| 0.05 | 0.7 | 1.45 | 66 % | 27 | +2.06 | +0.92 |

Kept: `b1_floor` 0.1, `radiation_bass` 0.5, chosen over 0.05/0.7, which follows the real ripple better (D2 4.2 dB against 4.0, E2 1.9 against 1.3) but scores 1.45. With it the ripple of B1–E2 overshoots the real grands' (the comparison tool's measure: B1 3.2 dB against 0.6, D2 6.0 against 4.0, E2 2.3 against 1.3): with less horizontal radiation the dip between the fast and the slow decay is deeper.

In the comparison tool the ripple of A#1–E2 comes up to the real grands' (C#2 0.17 → 3.0 dB, real 1.4), but not at A0–E1 (0.03–0.25 against 1.6–3.4) nor much at F1–A1. There the real first partial is no double decay: it lies 40–80 dB under the note, hardly decays, and wanders by ±3–7 dB (Helgesson's A0 at −70 dB jumps by 7 dB): recording noise, the room and the difference tones of neighbouring partials move a component that weak, and the measure says more about the recording than the piano. Letting the difference tones reach the board (`nonlinear.coupling_corner` 30 Hz instead of 1 kHz) gives 1–1.7 dB of ripple, but as one slow bump: not kept. The gap underneath is the first partial's level against the strongest: 7–12 dB too strong from A0 to C2 (A1 −17 dB against the real grands' −28, Splendid's −34), 8–12 dB too weak from F#2 to F#3 (C3 −9 against 0).

## The top: the strings' decay

In the comparison tool, from D6 up: the early decay (0.1–1 s) falls twice as fast as on the real grands from F7 (F#7 −47 dB/s against −29, A7 −59 against −33, C8 −58 against −27), the late decay (1–3 s) too, and the between-partials measure is 10–22 dB low from D#6 to D7.

- **The late decay of the top is mostly the recordings' noise.** By 1 s the real C8s lie within 0–10 dB of their noise floor; their flat line (0 to −5 dB/s) is that floor, and only 1–3 of the 8 grands keep a value from G#7 up. The model's own fault is earlier: its C7 is at −61 dB at 1 s against −29 to −44 on the real grands, whose fundamental still sounds there.
- **Two causes.** The strings of a key at C7, 2.3 cents apart (`unison.width_treble` 5 at C8), beat with a 25 dB null between 0.6 and 1 s; and the frequency term of the strings' loss is doubled from C4 up (`losses.b2_top` 2e-7, set against the commercial D, and heard in the middle as strings that buzz with less).
- **A third zone for that loss** (`losses.b2_high`, `b2_high_from`, coming in over the octave below): the Concert grand takes 1e-7 from C7, the middle keeps 2e-7; `unison.width_treble` 2. F#7 −35 dB/s, A7 −38, C8 −47; C7 at 1 s −47 dB. Bench 1.43 → 1.36, 25 → 21 off, sustain family 1.75 → 1.03 (top p2 sustain −6.69 → −0.84 z, p1–4 late T60 −2.02 → 0.07, 4 kHz sustained band −1.86 → 0.05, level at 1 s −2.5 → −0.8); further: the top's stretch (−0.01 → +1.44 z, the narrower unisons), C5–B6's sustained p3–p4 (+1.3 → +2.3, +0.0 → +0.8, where the loss blends in), the top's 2 kHz bands (−4.0 → −4.8).
- **Left: a continuous floor at 2.5–5 kHz** over the first 0.3 s, 10–20 dB over the model's from D#6 up (C7, D#6, Plath included). The body 10 dB louder at 1–4 kHz brings back 3–5 dB of the between measure only, sympathy ×3 next to nothing. The body's spectrum is the same for every key; the small hard hammers of the top likely knock brighter: a body halo that brightens towards the treble, or the hammer's contact noise there.

## The middle: the halo above 1 kHz

After the top, the comparison tool's between measure was still 12–15 dB low from C3 to C5 (C4 −62 dB against −49). Gap by gap at C4: below 1.2 kHz the model sits on the real grands; above, its halo in the middle of the gaps is 7–20 dB under theirs, more the higher, and its partials 6–11 are 4–7 dB strong. The body had been fitted on the residual's energy per octave, where those partials' skirts filled the octaves; the halo between them was too weak. Its spacing above 2 kHz is not the cause.

The body's levels at 1, 2, 4 and 8 kHz raised by 5, 15, 20 and 8 dB (−18, −13, −20, −32): C3–C5 within 4–6 dB of the real grands, E5–C6 on them; A6–C7 still 9–16 dB low. A step further (+7, +19, +25, +12) brought C3–C5 within 1–3 dB but E5–C6 2–4 dB over; the ear took the first. Bench 1.36 either way (22 off, 21 before); further: the bass's mf attack at 2–8 kHz (4–8 kHz z −0.2 → +2.6): the body's spectrum does not depend on the blow, and in a soft bass note the strings' high partials are weak. A knock that brightens with the hammer's speed would answer it.

## The blow: a soft attack's knock

**Listening report (2026-11-17).** A "chtac" at the attack, far too sharp at pp, about as strong as at ff, on every key since the halo was added.

**Measurement.** The residual between the partials was measured in the first 20 and 60 ms, relative to the note's level over its first 100 ms. The references are the commercial grand's layers (pp, mf, ff) and Splendid's (mf, ff; its pp files are its mf ones). The playback scale gives 0.94, 1.19 and 2.76 m/s. From ff to pp, the real grands' attack residual falls in the bass by 11, 25 and 28 dB at 1, 2 and 4 kHz, and in the middle by 7, 13 and 13 dB. The model's fell by at most 4 dB, and in the middle at 1 kHz it even rose. Two sources caused this:

- **The action's knock** ([`crate::action`]) dominated the pp attack at 1–2 kHz in the middle. On C4 at pp, the residual was −33 / −37 dB at 1 / 2 kHz with the knock, −37 / −44 without it.
- **The halo's pulse** carried the bass at 2–4 kHz.

Both scaled linearly with the hammer's speed, with a spectrum that did not depend on the speed.

**Change: one law for the blow.** Each resonance at f takes a further (speed/speed_ref)^γ, with γ = speed_low + speed_high·x/(1 + x) and x = (f/speed_corner)², interpolated over the keys between A0, C3, C5 and C7.

- **Reference speed:** speed_ref = 2.76 m/s (ff), so ff does not move.
- **Fit:** the law's values are fitted on the exponent the real grands ask for, which they give consistently. Below the corner, γ is 0 in the bass and middle, about 0.45 at C5 and 1 from C6 up. Above it, γ adds about 2.5 in the bass, 1.5 in the middle and 1 at C5.
- **Concert grand values:** speed_corner 1300 Hz, speed_low [0, 0, 0.45, 1.05], speed_high [2.5, 1.5, 1.0, −0.4].
- **Knock:** the action's knock takes the whole law.
- **Halo:** it takes a share, `speed_halo`. With the whole law, the tool's between-partials measure (0.02–0.32 s, relative to the partials) fell in the middle at pp and mf by 9 and 7 dB. The real grands instead keep it at their ff level or above at pp: −46 / −46 / −51 dB on Splendid and −44 / −48 / −50 on Salamander, pp / mf / ff. Only the bass's halo darkens with the attack. The Concert grand uses speed_halo [1, 0.25, 0.3, 0.3].

**Result, pp attack (0–60 ms).** The remaining exponent the real grands ask for, against the commercial grand's pp:

| | 1 kHz | 2 kHz | 4 kHz |
|---|---|---|---|
| C4–B4 | 0.57 → 0.14 | 1.10 → 0.72 | 0.89 → 0.58 |
| C5–B5 | 0.48 → 0.16 | | |
| A0–B1 | | 2.35 → 1.52 | 2.44 → 1.60 |

The bass stays the furthest: its halo would need more than the between-partials measure allows.

| Between partials, pp / mf / ff | Middle | Treble |
|---|---|---|
| Model before the law | −43.4 / −46.9 / −53.6 dB | −48.0 / −51.0 / −56.1 dB |
| Model with the law | −46.0 / −48.9 / −53.6 dB | −50.5 / −53.3 / −56.1 dB |

In the comparison tool, 294 measures are off against 288 before. The new ones are threshold crossings: between partials +2, rise time +3, top early decay +1.

## Making other pianos

The same steps fit any recorded piano: `tools/between_partials.py table` on its note set against the model's, the body law fitted per octave as above, the last damper read from the lines at the top strings' fundamentals, the phantom coupling from the bass lines between the partials.
