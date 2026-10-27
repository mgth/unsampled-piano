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

## Still open

- **C2–A2 lacks body.** Relative to the strongest partial, partials 4–6 lie 10–13 dB under the recording in the sustain (−1 to −5 at the attack): they decay at −14 dB/s against −9. The edge loss is not it (off: −12). The bench's eight grands agree: the bass's partials 4–12 are 4–8 dB weak in the sustain. The bridge's conductance at 300–800 Hz is next.
- Partials above the 20th of C2–A2 stand 5–20 dB over the recording.
