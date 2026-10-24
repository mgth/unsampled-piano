# Bass decay: the low modes lose more

Status as of 2026-10-24.

## The gap

In recorded performances, the bass partials 1–2 decay at −7 to −10 dB/s while a key is held, against about −4 dB/s on the model, with close microphones too ([pieces.md](pieces.md#decays-in-context)). The broadband envelope of the note-set bench showed no gap, because it is dominated by partials 3–8.

The isolated notes show it partial by partial. Envelope ±1.5 % around each partial, ff, median of eight real grands, decay rate over 0.1–1.5 s at A1, C2, D#2, F#2 and A2 (dB/s):

| | partial 1 | partial 2 | partial 3 | partial 4 |
|---|---|---|---|---|
| real grands | −6.7 | −11.7 | −10.9 | −9.2 |
| model | −4.5 | −6.9 | −7.6 | −9.1 |
| model, `loss_low_corner` 250 Hz | −4.5 | −10.9 | −10.3 | −10.6 |

## What was tried

- **A softer bridge in the bass** (`impedance_slope_bass` −0.03 → 0):
  - partials 1–2 come to −6.2 and −9.7 dB/s;
  - but the impedance acts at every frequency: partials 3–8 then fall too fast, and the bench's broadband envelopes at 1–3 s with them (bench 1.51 → 1.56, nine bass measures worse).
- **More loss in the wound strings** (`losses.b1_floor` 0.2 → 0.35–1.2):
  - a string's own loss is the same at every partial;
  - partial 1 comes closer (−5.8 to −8.0) only once the upper partials overshoot.
- **The cabinet's piston mode** (40–70 Hz, loss 0.7–1.5): no effect on the partials' decay.

## A loss growing towards the low frequencies

A low mode of the board works its edge, and the rim and the case it drives take energy back. The model's rim is driven one way and takes none. The board's loss factor gets a term growing below a corner:

η(f) = η·(1 + f/loss_corner)·(1 + (loss_low_corner/f)²)

With `soundboard.loss_low_corner` = 250 Hz (default; 0 turns it off), the board's modes are twice as lossy at 250 Hz and three times at 180 Hz. The bridge conductance between their resonances grows with them. Partials 2–4 of A1–A2 then decay as on the real grands (table above). Envelopes of partials 1–4 at A1–A2 come within 4.9 dB rms of the real median, against 6.6.

The bench score stays at 1.52 (1.51). It does not measure these decays directly: its T60 fits over 20 s, its envelopes are broadband. Four measures move by 0.5 z or more each way.
- **Better**: the bass's 2 kHz attack band, its 1 and 8 kHz sustain bands, its pp level at 3 s.
- **Worse**:
  - the bass's mf centroid (z +0.6 → +1.4);
  - its sustain partials 2 and 4 re partial 1 (−0.6 → −1.2, −0.3 → −0.9), since partial 1 still decays too slowly;
  - the middle's pp level at 2 s.

In Liang's recorded pieces (Disklavier grand), partials 1–2 of the bass decay at −5.1 dB/s (−3.9 before, recording −7.2), partials 3–6 at −7.3 (−6.1, recording −10.6).

## Partial 1: the cabinet's piston mode

Below the first clamped mode (109 Hz, raised there by the crown so that the bass fundamentals do not radiate too much, [bass_radiation.md](bass_radiation.md)), the bridge took almost nothing: the C2 fundamental lasted T60 ≈ 20 s, mostly on the string's own loss. A real piano's fundamentals lose energy there without radiating much. The board, rim and case move as a block on the case's compliance. The model has that path, a uniform "piston" mode that radiates 3 % of a bare board. But it sat at 40 Hz with 6 kg, too heavy to take much.

Decay rates over 0.1–1.5 s at A0–A2 (median of eight keys, dB/s):

| piston (Hz / loss / kg) | partial 1 | partial 2 | partial 3 | envelopes rms |
|---|---|---|---|---|
| real grands | −4.8 | −7.4 | −8.3 | — |
| 40 / 0.7 / 6 (before) | −4.1 | −4.2 | −5.8 | 4.9 dB |
| 40 / 0.7 / 3 | −5.4 | −4.5 | −5.9 | 4.7 |
| 60 / 1.0 / 3 | −5.8 | −5.3 | −6.2 | 4.5 |
| 60 / 1.0 / 1.5 | −9.0 | −6.7 | −6.8 | 5.5 |

The default is 60 Hz, loss 1, 3 kg. The bench stays at 1.52, and the bass gets a little closer (RMS z 0.72 → 0.70). In Liang's pieces the bass's partials 1–2 decay at −6.9 dB/s, against −7.2 on the recording (−5.1 with the old piston). The 63–100 Hz bands lose 11–12 dB between 0.3 and 3 s at A0–A1, against 3–16 on the references. A mass of 1.5 kg at 40 Hz once took 37 dB there ([long_sustain.md](long_sustain.md)).

## Still open

- **Partials 2–3 of the low bass (A0–F#1)** still decay at −5 to −6 dB/s against −7 to −8.
- **Partial 1 of the middle (C3–D#4)** decays too fast on the isolated notes: −22 dB/s with the low-frequency loss, against −12 on the real grands. It was −15 without it, already too fast. Liang's pieces put the middle's partials 1–2 at −15.1 dB/s against −12.0 on the recording. The loss acts on the board's low modes whichever bridge drives them. On a real board, the bass bridge sits near the rim and the main bridge in the board's middle, so a loss tied to the edge would spare the middle.
- **The low bass's fundamental is too strong at the attack.** A0 and C1 have partial 1 at −8 dB re the strongest partial, against −20 on the real grands.
