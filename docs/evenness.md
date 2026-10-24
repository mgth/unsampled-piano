# Evenness from key to key

Status as of 2026-10-24.

## Listening report

At the end of the Chopin op. 10 no. 3 A/B (Liang's Disklavier): "a rather strong high resonance; the original has it too, but much more muffled".

## What it was

Not the sympathetic strings and not the action's knock: with either turned off, the same lines stand out. Over 55.5–57.5 s, three notes ring 7–12 dB louder than on the recording:
- A3's fundamental (220 Hz);
- C#4's partial 2 (554 Hz);
- F#4's partial 2 (740 Hz).

G#3's fundamental, on Liang's isolated notes, is 13 dB weaker than recorded. The model's soundboard favours some notes and starves others.

## The measure

`evenness` (tools/bench.py): for each bench key from C2 to C7, the level of partials 1 and 2 at 0.5–1.5 s (ff) minus the median of the keys within ±6 semitones; the rms of these residuals. One figure, filed under the middle.

| | partial 1 | partial 2 |
|---|---|---|
| real grands, mean | 6.4 dB | 4.7 dB |
| real grands, range | 4.6–8.4 dB | 3.0–5.9 dB |
| model | 9.4 dB | 7.0 dB |

Taken apart:
- **At the attack** (0.02–0.12 s), the model is as even as the real grands: partials 1/2 at 6.1/6.9 dB against 2.3–7.9/3.0–7.5.
- **The decays** (0.45 → 1.5 s) vary less from key to key than on the real grands: 5.4/4.4 dB against 4.0–8.9/4.1–6.0.
- **In the sustain** the two add up: 9.7 dB on partial 1, at the top of the real range (5.1–9.1; Liang's Disklavier 8.1).

## What was tried

| | evenness p1 / p2 | bench (1.10) |
|---|---|---|
| board loss factor 0.02 → 0.03 | 9.1 / 6.3 | 1.12 |
| bridge drive spread 0.3 → 0.6 / 1.0 | 8.4 / 6.6, 7.9 / 6.5 | 1.15, 1.28 |
| board loss corner 2.5 kHz → 1 kHz / 500 Hz | 9.6 / 6.6, 8.9 / 6.1 | 1.13, 1.23 |
| 1.5× / 2× the modal density at equal mean mobility | 7.5 / 7.4, 9.1 / 5.7 | 1.41, 1.75 |
| other microphone set-ups | 9.5–12.8 / 7.4–8.2 | 1.24–1.43 |

None was kept. A more realistic board (ribs, an irregular outline, two bridges) would be the next step.
