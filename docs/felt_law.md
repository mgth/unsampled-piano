# The hammer felt's law

Status as of 2026-11-15. Item 3 of [steinway_d_gaps.md](steinway_d_gaps.md): the Concert grand's felt has an exponent of 3.7 to 5.0 where "measured felts" are said to have 2.3 to 3.5, and the measured values, put in, made the sound worse. This document says what the literature gives, why the measured laws failed here, what was tried and what is left to hear. Rule of the session: the real pianos rule over the commercial D; its distance is reported and not optimised for.

Candidate B is in the preset since it was played at the keyboard (below, "In the preset"). The other figures of this document are those of the preset before it.

## What comes out

- **The present law is a measured law, of another kind of hammer.** It is Stulov's fit (new, unvoiced Abel hammers dropped at 0.5 to 0.85 m/s on a rigid sensor, up to 55 N), 1.2 times as stiff, 0.55 times at A0, with 0.6 of its hysteresis. The core term no longer does anything: since the felt was made four times harder the blows stop short of it (A0 at velocity 100: 59 N with the core, 55 without; every other key and velocity the same force).
- **Its exponent is not comparable with the 2.3–3.5 of the other sources.** Stulov's law has a rate term, F = Q0·[u^p + α·d(u^p)/dt], and while the felt is being compressed that term leads: the loading branch goes as p·α·V·Q0·u^(p−1). Read as a laboratory reads a hammer without hysteresis, the present law shows an exponent of 2.8 to 3.3 from A0 to C5 on its loading branch, 3.4 to 3.7 at C6, 3.8 to 4.7 in the top octave. Putting 2.3 into the same law with its rate term kept (the trial of 2026-11-12, `p_offset` −1.45) gives a loading exponent of 1.4 to 2.0 up to C5, softer than any measured felt: that trial counted the hysteresis twice.
- **A voiced Steinway D's hammers were measured, and they are not Stulov's.** Russell and Rossing threw a factory-voiced D's hammers, on their own shanks, at a rigid sensor at 0.6 to 6.2 m/s. Their pulses quicken with speed as an exponent of 2.1 to 2.7 from A0 to A4, 2.85 at A5, 3.0 at A6, and 4.3 to 4.6 at D7–F7. On that same test the present law is 20 to 40 % too quick at 1 m/s from A2 up and far too sensitive to speed (b = 0.70–0.73 against 0.36–0.50).
- **RT-425's law failed for two reasons of its own.** Its exponents and stiffness follow such measures in the middle (D4: 849 Hz and 0.43 on Russell and Rossing's test, measured 849 and 0.40), but its loss term is 3.4 ms at C1, longer than the contact, which makes the bass felt a damper (it gives nothing back: 100 % of the blow's energy lost on a rigid target), and its top hammers are half as quick as the measured ones (D7: 735 Hz against 1262), with contacts of 0.85 and 0.89 ms at C7 and C8 where a real grand has 0.55 and 0.45. Hence the top keys 16 dB short.
- **The measured D law does not carry over to the model as it is either.** Fitted key by key on Russell and Rossing's figures and put on the model's strings, with the model's hysteresis it makes the bass's and the middle's mf attack far too bright against the eight real grands (bass 4–8 kHz +27 dB where +14 is asked). Without hysteresis it is the best of the candidates in the bass and the worst in the middle, and equal to the preset overall (bench 1.82 and 1.82).
- **Against the real pianos the present felt has two faults, and neither is its exponent as a whole.** The bass's mf attack has nothing at 4–8 kHz (−75.8 dB re the attack, eight real grands −62.1, z −2.8), so its 4–8 kHz grows by 48.6 dB from mf to ff where a Disklavier grand's grows by 26.0. And the treble's mf attack is 7 to 9 dB too bright at 2–8 kHz. A lower exponent in the bass alone corrects the first exactly; a softer felt above C4 half of the second.
- **The commercial D asks for the opposite in the low bass**: its 2–4 kHz grows by 35.5 dB from mf to ff at A0–B1, a real Disklavier grand's by 23.3 at the same velocities, the preset's by 21.8. What the earlier felts were built to follow there is that instrument's own.

## The baseline (main at cd3904f)

Bench of eight real grands 1.82 (bass 1.41, middle 1.38, treble 1.61, top 2.75). Distance to the commercial D, 88 keys at velocities 60 and 100: 8.23 (bass 8.38, middle 9.20, treble 8.82, top 6.19); loudness key to key 4.56 dB rms. Both as docs/steinway_d_gaps.md gives them.

**Growth of the attack from mf to ff** (first 60 ms, octave bands, mean over every third key), against BiVib's Disklavier grand, the one real reference with known velocities (56 and 100, the bench's layers), and per dB of the whole attack's growth, which does not need the velocities to agree:

| 1–2 / 2–4 / 4–8 kHz (dB) | A0–B1 | C2–B2 | C3–B4 | C5–B6 | C7–C8 |
|---|---|---|---|---|---|
| Disklavier grand | 14.3 / 23.3 / 26.0 | 14.5 / 25.1 / 24.4 | 12.4 / 20.7 / 30.9 | 15.7 / 23.1 / 27.9 | 22.9 / 27.6 / 29.1 |
| the preset | 14.2 / 21.8 / 48.6 | 13.1 / 18.9 / 34.1 | 11.1 / 16.0 / 27.2 | 11.4 / 14.1 / 20.9 | 7.8 / 17.6 / 22.8 |
| per dB of level: Disklavier grand | 1.74 / 2.84 / 3.17 | 1.75 / 3.04 / 2.95 | 1.43 / 2.38 / 3.55 | 1.28 / 1.89 / 2.28 | 0.97 / 1.17 / 1.24 |
| Salamander (Yamaha C5) | 2.19 / 4.22 / 5.57 | 1.90 / 3.31 / 3.75 | 1.47 / 2.37 / 4.18 | 1.17 / 1.85 / 2.72 | 0.73 / 1.12 / 1.75 |
| Iowa (Steinway B) | 1.76 / 2.79 / 2.44 | 1.46 / 2.61 / 2.75 | 1.28 / 2.02 / 3.00 | 1.19 / 1.74 / 2.12 | 0.82 / 1.18 / 1.93 |
| Headroom (Yamaha C3) | 2.83 / 4.05 / 2.03 | 2.39 / 3.64 / 2.21 | 1.72 / 2.80 / 3.19 | 1.28 / 2.21 / 2.61 | 0.69 / 1.07 / 1.64 |
| the preset | 1.89 / 2.89 / 6.45 | 1.68 / 2.43 / 4.39 | 1.37 / 1.98 / 3.37 | 1.06 / 1.32 / 1.94 | 0.45 / 1.01 / 1.31 |
| the commercial D (60 → 100) | 2.20 / 3.93 / 4.59 | 1.90 / 3.45 / 5.05 | 1.68 / 2.47 / 4.15 | 1.28 / 1.86 / 2.96 | 0.66 / 0.94 / 1.65 |

In the low bass the preset's 2–4 kHz grows as the real grands' does, and its 4–8 kHz twice too much, because there is none of it at mf. From C2 to B6 the preset brightens too little with level, most in the treble (2–4 kHz: 14.1 dB against 23.1), where its mf is too bright and its ff right.

**The attack's spectrum** (the bench's shares of the attack, dB; target: mean of the real grands):

| | bass, mf | bass, ff | middle, mf | middle, ff | treble, mf | treble, ff | top, mf | top, ff |
|---|---|---|---|---|---|---|---|---|
| 2–4 kHz: the preset / real | −32.9 / −39.2 | −20.0 / −20.1 | −28.1 / −28.9 | −20.2 / −16.7 | −22.3 / −29.3 | −18.9 / −18.6 | −5.1 / −4.2 | −4.8 / −3.0 |
| 4–8 kHz: the preset / real | **−75.8 / −62.1** | −40.8 / −41.3 | −50.3 / −54.6 | −31.2 / −32.0 | **−38.7 / −47.8** | −28.6 / −30.4 | −25.7 / −28.3 | −20.3 / −19.6 |

The top's dynamics are too wide: ff − pp 23.1 dB against 17.8, ff − mf 17.4 against 13.7.

**Contact lengths** (the engine's force pulse, ms, and peak force, N):

| velocity | A0 | C1 | C2 | C3 | C4 | C5 | C6 | C7 | C8 |
|---|---|---|---|---|---|---|---|---|---|
| 40 | 2.46 (9) | 2.57 (9) | 3.41 (8) | 2.96 (7) | 2.41 (8) | 2.11 (9) | 1.44 (10) | 0.90 (14) | 0.69 (19) |
| 60 | 2.02 (17) | 2.10 (16) | 3.20 (15) | 2.63 (12) | 2.05 (15) | 1.92 (17) | 1.19 (20) | 0.71 (28) | 0.52 (39) |
| 100 | 1.59 (59) | 4.95 (50) | 3.01 (47) | 2.34 (36) | 1.70 (41) | 1.43 (48) | 1.02 (56) | 0.56 (85) | 0.34 (129) |

Askenfelt and Jansson's grand: about 4 ms in the bass to under 1 ms at the top, ±20 % from p to ff around mf at C4 (the model: +18 % and −17 % from velocity 60 to 40 and 100); mf / ff 1.1 / 0.85 ms at C6, 0.65 / 0.55 at C7, 0.5 / 0.45 at C8. The model's top contacts shorten more with level than theirs (0.79 and 0.65 against 0.85 and 0.9 at C7 and C8, read off a figure to 0.05 ms). In the low bass the first contact ends at 2 ms, before the pulse comes back from the agraffe (4.4 ms at A0); their bass figure is about 4 ms, from a figure only.

## What the literature gives

The reading notes, with a source and a mark (read, second-hand, from memory, derived) on every figure, are in `out/literature.md`. The JASA papers of Hall and Askenfelt, Boutillon, Suzuki, Stulov (1995), Conklin and Askenfelt and Jansson could not be opened: their figures are second-hand, from the authors' Stockholm lectures and from Russell and Rossing's review.

**Two families of measures, which do not measure the same hammer nor the same thing.**

| | Stulov (2000–2009) | Hall and Askenfelt (1988), Russell and Rossing (1998) |
|---|---|---|
| hammers | new, unvoiced (Abel, Renner, Imadegawa), from a factory | voiced and played: a 2 m Grotrian, a Steinway D voiced at the factory |
| blow | dropped on a rigid tube, 0.3–1.5 m/s, up to 55 N and 0.8 mm | by the key and action, on the hammer's own shank, 0.6–6.2 m/s, 5–310 N |
| what is fitted | the whole loop F(u), with a rate term | how the pulse shortens with speed, no hysteresis |
| exponent | 3.7 (A0) to 5.0 (C8) | 2.1–2.7 up to A4, 2.85 at A5, 3.0 at A6, 4.3–4.6 at D7–F7 (the D); 2.3 / 2.5 / 3.0 at C2 / C4 / C7 (the Grotrian: Chaigne and Askenfelt's values) |

- **Stulov's own parameters are not unique**: his first device gave p = 2.4–2.7 for A1 hammers, his second 3.7–4.3 for the same kind, and he notes that different sets give very similar curves. Needling lowers both the stiffness and the exponent.
- **The rate term reconciles half of it** (above): 3.7 with a rate term loads as 2.7. Not all of it: on a rigid target the contact of a p = 3.72 felt shortens as V^−0.58, a voiced D's bass hammers as V^−0.36 to −0.41. The felt hardens less steeply at high force than Stulov's law extrapolates from 55 N.
- **Russell and Rossing's table for the D** (peak frequency of the pulse's residual shock spectrum, fmax = a·v^b): a = 640–760 Hz from A0 to A3, 830–850 at A4, 993 at A5, 1147 at A6, 1262–1307 at D7–F7; b = 0.36–0.46 up to A4, 0.48–0.50 at A5–A6, 0.62–0.64 at D7–F7.
- **RT-425's hammer laws** (the D's, in Chabassier's simulations) are its authors' proposals, not measures: exponent 2.28–2.42 up to C4, 3.5 at C8; stiffness 10^(0.0531·i + 7.64) N/m^p; a loss term R_H whose time R_H/K_H is 3.4 ms at C1, 91 µs at C4, 2.4 µs at C7.
- **Stiffness in N/m^p cannot be compared or interpolated across exponents**: the unit changes with p. The force at a reference compression can.
- **Masses** agree within 1 to 2 g: 11–12 g at A0, 8–9 at C4, 5.3–5.7 at C8. Chaigne and Askenfelt's 4.9 / 2.97 / 2.2 g are a 2 m grand's effective values.
- **Layers.** Conklin: the surface softer than the inner felt, often two layers, no gradient gives poor tone; no hardness profile in numbers was found. Stulov's bass felt is 15–17 mm thick with a wave speed of 25–125 m/s: 0.1 to 0.7 ms to cross it, not small against the contact.
- **On a real string** no published force pulse at several dynamics was found for a grand. Giordano's group reports (second-hand) that a Stulov-type law does not fit string-contact data and that the loop is narrow there. Chabassier: the bass strings pass an amplitude of one diameter from mf up, so part of a real bass's brightening is the string's nonlinearity, not the hammer.
- **Stulov's hysteresis time has a cubic term** the model's formula lacks: α = 248 + 1.83·N − 0.055·N² + 0.00085·N³ µs, 250 µs at A0 rising to 562 at C8. `hammer_stulov` stops at the square term and floors it at 30 µs: 140 µs at C6, 69 at C7, 30 at C8. Not changed (below).

## The laws on a rigid target and on the string

`out/tools/hsim.py` is the engine's contact outside it (stiff modal string, head on its shank, any felt law; within 5 % of `strike_force_dump` on peak force and length at five keys and three velocities). It gives the compression, which the engine's dump does not.

**Each law as Russell and Rossing measured the D's hammers** (rigid hammer of the whole mass; a (Hz), b):

| | A0 | A1 | A2 | A3 | A4 | A5 | A6 | D7 | F7 |
|---|---|---|---|---|---|---|---|---|---|
| measured, voiced D | 763, 0.41 | 640, 0.41 | 723, 0.40 | 756, 0.46 | 827, 0.46 | 993, 0.48 | 1147, 0.50 | 1262, 0.64 | 1307, 0.62 |
| the present law | 589, 1.02 | 705, 0.78 | 866, 0.70 | 1032, 0.72 | 1167, 0.73 | 1285, 0.73 | 1490, 0.72 | 1623, 0.71 | 1738, 0.70 |
| the same, exponent −1.45 | 501, 0.93 | 717, 0.50 | 922, 0.53 | 1153, 0.57 | 1335, 0.59 | 1487, 0.60 | 1763, 0.59 | 1914, 0.57 | 2028, 0.56 |
| RT-425's law | 512, 0.58 | 608, 0.58 | 701, 0.52 | 803, 0.45 | 898, 0.43 | 897, 0.46 | 795, 0.51 | 735, 0.52 | 698, 0.53 |

(A0's 1.02 is the core, reached on a rigid target at 4 m/s; without it 0.62.) With the head on the model's shank the pulse breaks up and the peak jumps from lobe to lobe: not usable, and the shank's 30 % share and 300–700 Hz are themselves estimates.

**On the strings** (velocities 40 / 60 / 100):

| | A0 | D#2 | C4 | C6 | C8 |
|---|---|---|---|---|---|
| present: peak force (N) | 9 / 17 / 56 | 7 / 13 / 39 | 8 / 14 / 40 | 11 / 20 / 55 | 22 / 42 / 130 |
| present: largest compression (mm) | 0.51 / 0.59 / 0.75 | 0.38 / 0.43 / 0.55 | 0.30 / 0.34 / 0.43 | 0.28 / 0.32 / 0.39 | 0.23 / 0.27 / 0.33 |
| Stulov as published: peak force | 9 / 17 / 55 | 7 / 13 / 39 | 8 / 14 / 40 | 11 / 19 / 54 | 18 / 36 / 115 |
| RT-425: peak force | 6 / 11 / 39 | 6 / 11 / 33 | 8 / 14 / 38 | 11 / 19 / 53 | 9 / 19 / 73 |
| RT-425: first contact (ms) | 3.62 / 2.99 / 1.69 | 2.96 / 2.83 / 2.74 | 2.23 / 2.10 / 1.92 | 1.62 / 1.39 / 1.17 | 3.09 / 1.69 / 0.89 |

**What each parameter does to the string's partials** (change of the 2–4 and 4–8 kHz shares at velocities 56 and 100, register means, simulator), against what the eight real grands ask of the preset:

| | mf 2–4 k | mf 4–8 k | ff 2–4 k | ff 4–8 k |
|---|---|---|---|---|
| **bass, asked** | −6.3 | +13.7 | −0.1 | −0.5 |
| exponent −1 | +2.4 | +14.5 | −0.6 | +0.7 |
| exponent −1.4, felt half as hard | −1.0 | +14.2 | −3.3 | −2.1 |
| the core at the surface | +6.6 | +15.1 | +3.3 | +7.4 |
| no core | +0.1 | +0.2 | −0.3 | +0.1 |
| **treble, asked** | −7.0 | −9.1 | +0.3 | −1.8 |
| felt a quarter as hard | −1.7 | −7.9 | −2.7 | −2.8 |
| exponent −1 | +0.9 | +5.2 | +0.8 | +1.1 |
| hysteresis × 2 | +2.3 | +6.2 | +0.7 | +2.7 |

The exponent is what sets the bass's mf attack at 4–8 kHz, and little else there. No felt setting takes the treble's 2–4 kHz down at mf by more than 2 dB: that excess is not the felt's. In the middle nothing moves the four figures by more than their own error (2.8 dB rms left, 1.7 at best). The top's dynamics narrow with a lower exponent at equal hardness (−1.5: ff − mf −3.5 dB, asked −3.7) or a harder felt.

## Candidates

All made of existing parameters, so the engine is untouched. `out/presets/`, the bench in `out/bench/<name>/`, 88 keys in `out/cat88/<name>/`.

- **A**, `ca`: `p_bass` −1.0 and `hardness_slope_bass` 0.0378. Only the keys below C4 change: the exponent is 2.7 at A0, back to Stulov's at C4, the felt softer by half at A0. It is the correction the real grands' attack asks for, not a law that reads as the D's hammers: its loading branch has an exponent of 2.2 to 2.6 below A2, and on Russell and Rossing's test it is slower and more sensitive to speed than theirs (A1: 593 Hz and 0.87, measured 640 and 0.41).
- **B**, `c2`: A and a softer felt above C4, `hardness_slope` −0.05 with `hardness_slope_top` 0.085: 0.55 of the present hardness at C5, 0.30 from C6 to C8.
- **C**, `rn5`: the voiced D's hammers as Russell and Rossing measured them, without hysteresis: `p_offset` −1.9 with `p_bass` 0.55 (exponent 2.35 from A0 to C4, 2.7 at A5, 2.9 at A6, 3.1 at C8), `hysteresis` 0, no core, `hardness` 0.2 with `hardness_slope_bass` −0.045 and `hardness_slope` −0.0055 (some 42 N at 0.5 mm from A0 to A3, 55 at A4, 77 at A5, as fitted on their table), and `hardness_slope_top` 0.07, set on the top's level (206 N at A6 where the fit has 110). On their test it reads 713 to 765 Hz and 0.41 from A0 to A3 (measured 640–763, 0.36–0.46), 862 and 0.43 at A4 (827, 0.46), 994 and 0.46 at A5 (993, 0.48), 1337 and 0.49 at A6 (1147, 0.50), 1536 and 0.50 at D7 (1262, 0.64): with the measured stiffness carried up to C8 at this exponent (−0.02) the top keys came 7 dB too quiet (`rn1`), the measured laws' failure once more. Their own top hammers have an exponent of 4.3 to 4.6, which no existing parameter gives above C6 only.
- For reference: `stulov`, Stulov's law as published (hardness 1, no slopes, no core, whole hysteresis, the model's α without its cubic term), and `c1` (`p_bass` −1.4, treble slope −0.03).

| | bench | bass | middle | treble | top | distance to the commercial D | bass / middle / treble / top | loudness key to key (dB rms) |
|---|---|---|---|---|---|---|---|---|
| the preset | 1.82 | 1.41 | 1.38 | 1.61 | 2.75 | 8.23 | 8.38 / 9.20 / 8.82 / 6.19 | 4.56 |
| A | 1.82 | 1.41 | 1.39 | 1.61 | 2.75 | 8.23 | 8.37 / 9.19 / 8.82 / 6.20 | 4.58 |
| B | 1.80 | 1.41 | 1.40 | 1.54 | 2.74 | 8.02 | 8.37 / 9.13 / 8.34 / 5.85 | 4.60 |
| C | 1.82 | 1.31 | 1.55 | 1.60 | 2.71 | 8.41 | 8.42 / 9.49 / 8.93 / 6.50 | 4.30 |
| C with the measured top (`rn1`) | 1.77 | 1.31 | 1.55 | 1.59 | 2.56 | 8.59 | 8.42 / 9.50 / 8.82 / 7.47 | 4.33 |
| `c1` | 1.81 | 1.42 | 1.39 | 1.57 | 2.74 | 8.10 | 8.37 / 9.15 / 8.55 / 5.99 | 4.55 |
| Stulov as published | 1.79 | 1.36 | 1.34 | 1.73 | 2.62 | 8.54 | 8.46 / 9.22 / 9.37 / 6.89 | 4.25 |

What each one does where it was aimed:

| | the preset | A | B | C | real grands |
|---|---|---|---|---|---|
| bass, mf attack at 4–8 kHz (dB; bench) | −75.8 | −66.5 | −66.5 | −64.1 | −62.1 |
| bass, mf attack at 2–4 kHz | −32.9 | −36.0 | −36.0 | −29.5 | −39.2 |
| A0–B1, growth mf → ff, 2–4 / 4–8 kHz | 21.8 / 48.6 | 23.0 / 30.0 | 23.0 / 30.0 | 19.4 / 22.7 | 23.3 / 26.0 (Disklavier grand) |
| treble, mf attack at 2–4 / 4–8 kHz | −22.3 / −38.7 | the preset's | −23.7 / −43.2 | −21.7 / −39.3 | −29.3 / −47.8 |
| C5–B6, growth mf → ff, 2–4 / 4–8 kHz | 14.1 / 20.9 | the preset's | 15.0 / 24.5 | 16.1 / 22.3 | 23.1 / 27.9 |
| top, ff − pp (dB) | 23.1 | 23.1 | 23.6 | 22.0 (17.1 with the measured top) | 17.8 |
| top keys' level re the keyboard, ff (bench) | −6.4 | −6.4 | −7.0 | −7.5 (−15.0) | −1.4 |

(The bass rows of A are B's: the keys below C4 are the same renders.)

- **A does what it was made for and nothing else**: the bass's mf attack at 4–8 kHz comes to the real grands' (z −2.8 → −0.9), its growth from mf to ff to the Disklavier's (30.0 against 26.0, 48.6 before). The register's score does not move (1.41): other bass measures give back what this one gains (the ff attack at 2–4 kHz −20.0 → −22.3, the 4 kHz band of the attack and of the sustain).
- **B gains a little on both references**: bench 1.82 → 1.80 (treble 1.61 → 1.54), distance to the commercial D 8.23 → 8.02 (treble 8.82 → 8.34, top 6.19 → 5.85). Half of the treble's mf excess at 4–8 kHz goes; the 2–4 kHz stays.
- **C trades the middle for the bass**: bass 1.41 → 1.31, middle 1.38 → 1.55, the same overall, further from the commercial D (8.41). Its bass comes at a mf attack 10 dB too bright at 2–4 kHz (z +1.4); in the middle the attack and sustain lose their 8 kHz band (−43.8 → −54.3 at the attack, real grands −41.6) and the treble's second partial at mf comes up 7 dB.
- **Stulov's law as published** scores as the preset does (1.79), better in the bass, the middle and the top, worse in the treble (1.73): the preset's voicing of it is worth what it costs, no more.

**None of them beats the present law on the real pianos by more than the bench can tell** (1.77 to 1.82 across all seven; the changes that brought 1.98 to 1.84 and 1.84 to 1.75 were heard). A and B correct two measured faults at no measured cost and are the ones worth hearing. Nothing goes into the preset before the keyboard.

## In the preset (2026-11-15)

The four presets were played at the keyboard by the user and B was kept, without a listening remark: `concert_grand.toml` has `p_bass` −1.0, `hardness_slope_bass` 0.0378, `hardness_slope` −0.05 and `hardness_slope_top` 0.085 (0.035 before). Its renders are those of `out/presets/c2.toml`, bit for bit on five keys. Bench 1.82 → 1.80 (bass 1.41, middle 1.40, treble 1.54, top 2.74); distance to the commercial D 8.23 → 8.02 (bass 8.37, middle 9.13, treble 8.34, top 5.85). `p_bass`'s range in the interface now goes down to −2 (it stopped at 0). The core's lines stay in the preset, idle.

## What the present law stands in for

- For a voiced D's hammers, in the middle and the treble, it stands in for something the rigid-target measures do not hold: put on the model's strings, the measured law is worse there, with or without hysteresis. The candidates for that something, none of them tested here: the pulse on a real string is not the rigid target's (Giordano); the felt's own transit time and its graded hardness; the shank, which both Giordano and Birkett name as a first-order effect and which is an estimate in the model; the strings' nonlinearity in the bass's brightening.
- In the bass its exponent is too high for the real grands' mf attack, and that part can be corrected (A).
- The core stood in for a layered felt while the felt was soft. It is idle now and can go from the preset without a change in the sound (not removed here: the renders would differ by the 4 N at A0).
- Hardness, mass and exponent do compensate one another: Chaigne and Askenfelt's light hammers (2.2 g at C7) go with their soft top felt; the same stiffness on the D's 6.5 g gives contacts twice a real grand's.

## Listening and playing

- Page: `out/ab_felt_law/index.html` (launch entry `ecoute`, port 8773): the preset, A, B and C on 14 keys from A0 to C7; through close microphones without a room at velocities 100 and 60 beside the commercial D; and at the player's ears, lid off, at velocity 105 beside Plath's real concert grand (one dynamic). One gain for the model's columns; each click restarts from the beginning and stops the other players.
- Keyboard: `target/release/unsampled-piano --ui --presets out/keyboard` from the worktree, instrument folder "Felt law trial": `1_current_preset`, `2_bass_lower_exponent` (A), `3_bass_lower_exponent_softer_treble` (B), `4_measured_d_hammers_no_hysteresis` (C).

## Not verified

- B was played and kept; A and C were played beside it and not kept, and no remark on what was heard is recorded. The measures are on single held notes.
- The simulator against the engine: 5 % on force and length; its predictions for the bass's mf attack at 4–8 kHz were met in sign and size by the bench (`c1`: +9.8 predicted, +14.0 measured; B: +4.9 and +9.3), nothing else of it was checked in the engine.
- Russell and Rossing's test on the model was made with a rigid hammer; theirs were on their shanks. The fit of C on their table carries that.
- The real references for the growth from mf to ff are one Disklavier grand with known velocities and three sample sets whose velocities are not known (per dB of level only). A binaural pair at the player's seat against the model's close microphones.
- Contact lengths in the bass and the middle against a real grand: Askenfelt and Jansson's figures could only be read from the lecture's text ("about 4 ms"), not key by key.
- Not tried: Stulov's hysteresis with its cubic term (it would take α from 0.6 × 30–140 µs to 360–560 µs above C6; on the simulator twice the hysteresis in the treble goes the wrong way at mf, +2 and +6 dB, and fitted on the D's hammers it asks for exponents under 2.1 up to A6); a top-only exponent (a new parameter) for C; a rate-independent hysteresis (Boutillon's two branches, whose exponents were not found); a felt with a transit time; any change of the shank.
- The top's dynamics (5 dB too wide) and the treble's mf at 2–4 kHz (7 dB too bright) are not reached by any felt setting tried.

## Tools

Unversioned, in `out/tools/` of the worktree `felt-law-c62f38` (`/mnt/local/SSD_B-CT4000/Piano/out/worktrees/felt-law-c62f38/`): `hsim.py` (the contact simulator), `laws.py` and `laws2.py` (pulses of the laws), `anvil.py` and `rr.py` (rigid-target readings, the fit on the D's hammers), `grid.py`, `grid2.py`, `gridtop.py`, `pred.py`, `rrlaw.py` (what a law does to the bench's attack measures), `growth.py` (growth from mf to ff per register), `contact.py`, `mkvar.py`, `cat.sh`, `bench.sh`, `sc.sh`, `abpage.py`. Results beside them: `literature.md`, `laws_first.txt`, `anvil_first.txt`, `rr_laws.txt`, `rr_fit.txt`, `grid.txt`, `grid2.txt`, `gridtop.txt`, `rrlaw.txt`, `growth_*.txt`, `d_all.txt`.
