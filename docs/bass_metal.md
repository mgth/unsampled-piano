# The metallic bass

Status as of 2026-10-27.

## Listening report

Beethoven op. 31 no. 2, finale, against Liang's Disklavier recording: "the bass is really far from the original: it is more metallic".

## What it was

Liang's dataset also holds every key of the same Disklavier at velocity 96, isolated. The model played the same notes (close microphones, studio); octave bands of the sustain (0.4–1.4 s) re the partials' energy, model minus recording:

| | 1–2 kHz | 2–4 kHz | 4–8 kHz |
|---|---|---|---|
| A0–B1, attack (0–0.15 s) | +0.3 dB | +0.4 dB | −3.1 dB |
| A0–B1, sustain | +6.3 dB | +15.5 dB | +19.8 dB |
| C2–B2, sustain | −0.6 dB | +3.5 dB | +3.1 dB |

The attack is right; the low bass's high partials then ring on. Their decay over 0.1–1 s, 2–4 kHz:

| | A0 | C1 | E1 | G1 | B1 | E2 | A2 |
|---|---|---|---|---|---|---|---|
| recording | −47 | −35 | −27 | −23 | −24 | −16 | −10 dB/s |
| model | −6 | −8 | −17 | −11 | −18 | −15 | −12 dB/s |

Neither the phantom partials (nonlinearity off), the sympathy, the frame nor the rim: each switched off in turn left the table within 1.5 dB. It is the strings' own loss. The loss law σ_k = b1 + b2·f_k² (b2 fitted on Salamander, the same for every string) gives the low bass's partials at 3 kHz, its 50th–100th, hardly more loss than its fundamental.

## The winding

The extra loss these partials need, from 1 to 3 kHz, grows about as the frequency: a constant loss factor, ≈ 3–5·10⁻⁴. On every string it would cut the middle's decays (−12 dB/s more on C4's partials at 1 kHz). It belongs to the wound strings, whose copper winding rubs on the steel core as the string bends, and above all to the lowest ones, wound in two layers.

`losses.winding` (η) adds ½·η·ω_k to every partial, full at A0 and fading out linearly to `losses.winding_until` (E2).

A term on the share of each partial's energy in bending, B·k²/(1 + B·k²), was tried first. It fitted the 2–8 kHz band but left 1–2 kHz ringing (that share is 0.04–0.19 there), and was dropped.

| winding | A0–B1 sustain, 1–2 / 2–4 / 4–8 kHz (re Liang) | bench |
|---|---|---|
| 0 | +6.3 / +15.5 / +19.8 dB | 1.13 |
| 2·10⁻⁴ | +2.7 / +7.7 / +8.0 dB | 1.14 |
| 4·10⁻⁴ | −0.6 / −0.1 / −5.8 dB | 1.18 |

The eight grands of the bench have a brighter bass than Liang's Disklavier: at 4·10⁻⁴ the bass's sustain at 1–2 and 8 kHz fell 4–18 dB under them, and its envelopes decayed ≈ 2 dB faster. At 2·10⁻⁴ the bass's 4 kHz sustain lands on their mean (z +1.1 → +0.3). The user heard no difference between 2 and 4·10⁻⁴; 2·10⁻⁴ is the default.

## The 220 Hz line

Listening again to the same passage (24.7–28.2 s: A1, D2, E2, F2 in the left hand, D3–D4 above): "still metallic, and above all not precise". By third-octave band the model stood +7 dB over the recording at 200 Hz, all of it one line: 220 Hz (A), fed by D2's third partial (+20–24 dB against Liang's isolated D2), A1's fourth and A3. The recording plays this passage the same with and without the pedal (≤ 1 dB), and the model's line stays without the sympathy: it is in the notes.

D2's third partial (219.7 Hz) lies 5 Hz under the board mode (1,3) at 224.8 Hz and forms a strongly coupled pair with it (T60 4.6 s). More board loss (η 0.035), more back-face short circuit (`back_gain` 0.9) and `loss_low_corner` 300 leave it within 2 dB. The grand's outline (docs/outline.md) brings it to +4–10 dB but moves the bumps elsewhere: on the passage, 200 Hz goes from +6 to −5 dB and 400 Hz from +4 to +7.

Against the bench's six grands it is Liang's Disklavier that stands apart. Partials below 700 Hz of A0–C4 at the top layer, dB re each piano's median, by third octave:

| | 112 Hz | 141 | 178 | 224 | 283 | 356 | 449 | 566 | 713 |
|---|---|---|---|---|---|---|---|---|---|
| six grands, attack | +6.9 ± 2.7 | +7.1 ± 2.3 | +4.3 ± 3.0 | **+6.0 ± 2.6** | +1.3 ± 2.4 | −1.3 | −0.3 | −3.2 | −4.6 |
| Liang, attack | +11.1 | +9.5 | +6.9 | **+1.9** | −1.2 | +0.4 | +1.5 | −1.5 | −1.0 |
| model, attack | **+16.5** | +7.5 | +6.3 | +7.8 | −4.7 | −0.3 | −2.9 | −3.5 | −7.0 |
| six grands, sustain | +8.6 ± 2.3 | +5.4 ± 2.9 | +5.7 ± 2.8 | **+5.3 ± 2.5** | −0.2 ± 3.1 | −1.8 | −1.5 | −4.5 | −5.5 ± 3.0 |
| Liang, sustain | +11.9 | +10.3 | +5.9 | **+0.3** | −6.2 | −2.5 | −0.8 | −4.6 | −3.0 |
| model, sustain | **+19.4** | +5.7 | +8.5 | +7.3 | −4.9 | +0.1 | −3.0 | −4.5 | **−13.4** |

At 224 Hz the model lies within the grands (+7.8 against +6.0 ± 2.6); Liang's Disklavier is hollow there, 4–5 dB under all the others. Lowering the model's 220 Hz would fit that one piano and leave the rest. What the model does have wrong in this register: 112 Hz, the board's first mode, 10 dB over the grands' mean (`loss_low_corner` 300 takes 3–4 dB of it), and the bass's partials near 700 Hz in the sustain (below).

For listening, out/ab_200 holds the passage (22–32 s) from the recording, the model, and the model with a 7 dB hollow at 222 Hz (an output equaliser, not a model change), to tell whether that line is what is heard.

The bass's partials 8, 16 and 24 (A0–A1) also stand 20–38 dB over Liang's: the recording has the strike point's notches there, the model hardly.

## "Not precise"

The user heard no difference with the 220 Hz hollow: the blur is elsewhere. Leads measured on the same passage:

- **Radiation damping.** The loss law gives every mode the same η, its radiation included. The Rayleigh self-impedance of the rectangle's modes, with the model's back face as an antiphase image (β 0.6, 0.6 m), gives η_rad 0.021 for (1,1) and < 0.006 for every other mode below 400 Hz. Only the first mode would gain it; the lingering below needs several times more.
- **Articulation**: how far each band falls between successive left-hand notes (the lowest level in the 30 ms before a note, re the peak of the one before). On 24.7–28.2 s without the pedal, 90–140 Hz falls 9.6 dB in the model against 14.5 dB in the recording, 40–90 Hz 11.9 against 19.1. `loss_low_corner` 300 brings 90–140 Hz to −17.0; firmer dampers (`damper.strength` 0.3) to −14.1. Neither touches 40–90 Hz. Over 22–32 s with the pedal, though, the medians agree (90–140 Hz: −16.5 model, −15.1 recording): the lingering is local to that figure (A1's second partial lies on the board's first mode, 108.8 Hz).
- **No late boom**: each band's peak after the 1–4 kHz onset comes no later in the model (median 32–41 ms) than in the recording (42–59 ms).
- **The attack's pulse train** (1–6 kHz envelope of the isolated notes A1–A2): the string's returns every period are as deep in both (12–21 dB), but the recording's first pulses are weak and grow over 2–3 periods, while the model's first is already among the strongest.

Bench: `loss_low_corner` 200 scores 1.14, 300 1.15 (baseline 1.14). With 300 the early T60 of partials 1–4 shortens, bass z −0.8 → −1.4 and middle −0.5 → −1.0. A/B in out/ab_flou (Beethoven 22–32 s with the pedal: recording, now, 200, 300), awaiting the user's ear.

## Still open

- **C2–A2 lacks body.** Relative to the strongest partial, partials 4–6 lie 10–13 dB under the recording in the sustain (−1 to −5 at the attack): they decay at −14 dB/s against −9. The edge loss is not it (off: −12). The bench's eight grands agree: the bass's partials 4–12 are 4–8 dB weak in the sustain. The bridge's conductance at 300–800 Hz is next.
- Partials above the 20th of C2–A2 stand 5–20 dB over the recording.
