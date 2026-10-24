# Releases and soft blows against a Disklavier grand

Status as of 2026-10-24.

## Listening report

In A/B against real recordings (tools/ab_real.py, Chopin on Liang's Disklavier and on the Vienna Bösendorfer): "sharp cuts where the background resonance is sucked away; that does not exist in the original."

## The release of a key

Liang's isolated notes (no pedal, velocity 96, keys held 2 s) give the drop of partials 1–2 after the key-up, ±1.5 % around each partial (dB, median over the keys):

| | 30 ms | 60 ms | 120 ms | 250 ms | 500 ms |
|---|---|---|---|---|---|
| recording, bass | 0.6 | 0.8 | 2.6 | 10.8 | 25.7 |
| recording, middle | 2.7 | 7.1 | 15.8 | 28.2 | 39.7 |
| recording, treble | 2.7 | 5.6 | 17.6 | 24.9 | 38.8 |
| model before, bass | 1.7 | 3.9 | 8.5 | 20.0 | 39.8 |
| model before, middle | 2.1 | 7.1 | 19.1 | 43.5 | 82.7 |
| model before, treble | 4.3 | 18.4 | 32.7 | 55.5 | 80.3 |
| model after, bass | 1.1 | 2.6 | 5.1 | 13.5 | 27.7 |
| model after, middle | 0.5 | 2.7 | 14.2 | 27.3 | 32.9 |
| model after, treble | 2.2 | 8.9 | 18.5 | 27.7 | 49.9 |

A real note decays in two stages: fast at first, then a slow tail 25–40 dB down, still 15–20 dB above the room's noise a second later. The model's dampers took everything down at one rate, two to four times too fast after 60–120 ms.

Two changes:
- **A softer felt.** `damper.strength` 1 → 0.1 (c/(2·Z0): the impedance-matched felt was the most absorbing one), `contact_exponent` 2.5 → 1.5, `fall_time` 12 → 15 ms.
- **The sideways motion.** The felt rests on top of the strings: it stops their vertical motion and only rubs against the horizontal one. New `damper.horizontal` (0.03): the share of the rate the horizontal polarisation gets. Each coupled mode of a unison gets the rate weighted by Σ_j w_j·V_jm² (first order, Vᵀ·V = I), w = 1 on the vertical oscillators and 0.03 on the horizontal ones.

RMS gap over every key and the five times, drops capped at 50 dB: 10.7 → 6.0 dB.

The half pedal then damped less. `pedal_curve` 1.5 → 1.2 brings it back (tools/pedal_bench.py, CC64 = 64, partials 1–4 at 1.3–1.8 s, pedal minus no pedal): middle −11.5 dB against −12.1 recorded (−14.4 before), bass −8.0 (−8.8), treble −7.7 against −15.2 (−9.8). RMS gaps over the pedal bench's rows: halo 4.6 (4.6), hold 4.5 (4.3), after 9.5 (10.8).

## In the pieces

Quick pedal changes in Liang's four pedalled pieces (218 changes, pedal from ≥ 96 to below 40 and back within 0.4 s): dip of the band's envelope during the change, median (dB):

| band | recording | before | after |
|---|---|---|---|
| 80–300 Hz | 8.2 | 11.7 | 10.4 |
| 150 Hz–3 kHz | 7.5 | — | 8.1 |
| 400 Hz–1.5 kHz | 8.9 | 9.1 | 8.6 |
| 1.5–6 kHz | 6.8 | 8.4 | 8.2 |

The pedal releases themselves (38, drops at 30–500 ms) did not change.

## Soft blows: the unison struck unevenly

The most audible cut left came 0.3 s after a soft chord, the pedal changing just after it. Rendered with the pedal held throughout, the model still lost its 1.5–6 kHz band within 0.3 s; the recording kept it for 0.7 s. Over the first 0.3 s, eight real grands (tools/bench.py's references) decay the middle's upper partials at about −20 dB/s at pp and −30 at ff. The model had the same decay at every velocity: it is linear once the hammer leaves.

A soft blow meets the strings with the surface of the felt only, and a unison's strings are never quite level: the lighter the blow, the more unequal the force each string takes. The uneven part excites the slow out-of-phase modes, the aftersound. The spread of the force (`unison.hit_spread`, 0.2) now grows as the hammer slows:

spread(v) = hit_spread·(soft_speed/v)^soft_exponent, capped at 1

with `soft_speed` 2.75 m/s (velocity 100: unchanged at ff) and `soft_exponent` 1 (0.58 at velocity 45).

| soft_exponent | 0 | 0.5 | 1 | 2 |
|---|---|---|---|---|
| bench score | 1.13 | 1.09 | 1.06 | 1.17 |
| within the real pianos | 75 % | 76 % | 80 % | 77 % |

With 1, the middle's pp and mf sustain (2–3 s) come within the real pianos (z −1.0 → 0.0), and the treble's from −1.4 to −0.7; the top's attack partials 3–4 at mf improve. Exponent 2 overshoots in the treble (sustain at 1 s z +1.4, attack partial 2 z +4.3). The dampers' changes do not move the bench (1.13 either way): it holds every note 20 s.

## Still open

- **The middle's partials 5–7** (1.3–2.3 kHz) still decay at −40 to −47 dB/s over 0.1–0.3 s at every velocity, against −20 to −30 on the real grands; this is most of the cut after the soft chord. Partials 6 and 8 fare better only because they meet the undamped top strings, which hold them up sympathetically (without sympathy they fall to −44 dB/s as well).
- **The bass's quick pedal changes**: 80–300 Hz still dips 2 dB deeper than recorded.
