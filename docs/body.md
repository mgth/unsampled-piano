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

## Making other pianos

The same steps fit any recorded piano: `tools/between_partials.py table` on its note set against the model's, the body law fitted per octave as above, the last damper read from the lines at the top strings' fundamentals, the phantom coupling from the bass lines between the partials.
