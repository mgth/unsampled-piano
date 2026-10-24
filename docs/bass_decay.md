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

## Still open

- **Partial 1 of the bass, and the whole low bass (A0–F#1).** They do not move with this loss: below 150 Hz the board has few modes where the partials fall, and the bridge takes almost nothing there. The model's C2 fundamental lasts T60 ≈ 20 s, of which the string's own loss is most. A real board's first modes (50–100 Hz, Conklin, Giordano) take more. The model's low board (crown, ribs, the lowest Warburton modes) is the next place to look.
- The low bass's fundamental is also too strong at the attack: A0 and C1 have partial 1 at −8 dB re the strongest partial, against −20 on the real grands.
