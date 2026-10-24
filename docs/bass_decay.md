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

## Every key, and the middle

The figures above sample every third key (the bench's), and with the board's low modes 20–40 Hz apart, a key's decay depends on where its partials fall between them. Over every key of A0–B2 and C3–D#4 (decay over 0.1–1.5 s, dB/s, median over the keys; real grands: the median of those with the key):

| | bass p1 | bass p2 | bass p3 | middle p1 | middle p2 | middle p3 |
|---|---|---|---|---|---|---|
| real grands | −5.4 | −7.2 | −7.0 | −12.2 | −15.6 | −14.7 |
| `loss_low_corner` 250 Hz, piston 60 Hz / 3 kg | −5.6 | −5.0 | −6.7 | −21.2 | −13.6 | −14.2 |
| no extra loss, piston 60 Hz / 3 kg | −5.7 | −4.5 | −5.5 | −12.8 | −11.1 | −13.9 |
| edge loss, piston 40 Hz / 6 kg (default) | −6.5 | −6.2 | −7.5 | −12.3 | −11.5 | −13.5 |

The loss on the modes sped up the middle's fundamentals as much as the bass's partials 2–3 at the same frequencies: −21 dB/s against −12. A mode's loss is the mode's, whichever bridge drives it. But the strings do not see the rim alike:
- the bass bridge runs 16–45 cm from the board's edge in the model's geometry;
- the main bridge crosses the board's centre in the middle (41–55 cm);
- the treble end comes back to the edge, but at frequencies the rim loss does not reach.

## The edge loss

`soundboard.edge_loss` adds to the bridge mobility a conductance seen by each key's strings, e^(−d/edge_depth) with d the bridge point's distance to the nearest edge, falling above `edge_corner` as 1/(1 + (f/edge_corner)²). It damps the strings and radiates nothing: the energy goes into the rim and the case, which the model does not take back from the board. Defaults: 1e-3 s/kg, 15 cm, 400 Hz. The low-frequency mode loss is back to 0 (`loss_low_corner`, kept as a parameter).

With it, the piston mode returns to 40 Hz, 0.7 and 6 kg: at 60 Hz and 3 kg on top of the edge loss, the bass's partial 1 decayed at −7 dB/s against −5.4.

Results:
- **Error over the six columns**: 8.0 dB/s summed, against 14.2 with the mode loss (table above).
- **Bench**: 1.52 as before; the middle improves (RMS z 0.83 → 0.77) and the bass gets a little worse (0.70 → 0.74). The bass's broadband level at 2–3 s, mf and pp, falls 1 z lower, its 2 kHz attack band too, and its mf centroid improves.
- **Liang's pieces**: the middle's partials 1–2 decay at −10.0 dB/s (recording −12.0; −15.1 with the mode loss), the bass's at −5.9 (−7.2; −6.9 with the 60 Hz piston).

## The bass's level

The register balance (`balance`, docs/top_level.md) put the bass's attack 7.6 dB under the middle's at ff, against 3.8 on eight real grands; in the lowest octave the deficit lay at 250 Hz–2 kHz. The bass side of the board was stiffer (`impedance_slope_bass` −0.03, ×2.2 at A0), so the bass strings drove it less.

With `impedance_slope_bass` 0 the bass comes to −4.7 dB, but its strings lose more into the board: the bass's broadband envelopes at 1–3 s fell up to 2 z too fast (decay family 1.02 → 1.27). The board now takes the wound strings' extra loss: `losses.b1_floor` 0.2 → 0. The edge loss stays; without it partial 2 decayed at −3.5 dB/s against −7.2.

| | balance bass ff | bass p1 / p2 / p3 (dB/s) | bench |
|---|---|---|---|
| real grands | −3.8 | −5.4 / −7.2 / −7.0 | |
| before | −7.6 | −6.0 / −6.2 / −7.6 | 1.10 |
| impedance_slope_bass 0, b1_floor 0 | −4.7 | −6.5 / −5.5 / −7.2 | 1.10 |

Besides the balance (z −1.9 → −0.4), the bass's envelopes at 1 s move by −0.8 z (−0.3 to −0.5 → −1.1 to −1.3). Chosen after an A/B listen.

## Still open

- **Partial 2 of the middle** decays at −11.5 dB/s against −15.6. The mode loss helped it, the edge loss cannot.
- **The low bass's fundamental in the sustain.** Relative to the strongest partial, the fundamentals of C1–A#1 sit 4–7 dB over the real grands' at 0.3–0.7 s (+1.5 dB at the attack, six references; noisier ones excluded). Two causes add up:
  - the plate modes still radiate these fundamentals below their resonances;
  - the strongest partials (4–6, 130–280 Hz) decay faster than on the real grands (−8 to −9 dB/s against −5.5 to −6.4), so partial 1 gains on them.

  What was tried:
  - a stronger crown (120 Hz) brings the fundamental in line (−1.1 / +0.8 dB), but it dulls the middle (bench 1.52 → 1.60, 24 measures worse);
  - the back face (`back_gain`, `back_depth`) moves it 1–2 dB;
  - lowering the edge loss's corner (150–250 Hz) slows partials 4–6 by under 1 dB/s.

  The real fundamentals stand only 4–20 dB above the recordings' noise at C1: the measure is a bound, not a target.

## The low bass's fundamental at the attack

Partial 1 re the strongest of partials 1–8, first 0.1 s, ff. The real grands put the fundamentals of A0–F#1 at −32 to −47 dB (median of eight), the model C1–F#1 at about −25. These fundamentals (32–46 Hz) fall on the piston mode (40 Hz), which radiated 3 % of a bare board. When the whole case moves, the air pushed above returns below, so that radiation should be nearly nil.

| piston_radiation | 0.03 | 0.01 | 0.003 | 0 |
|---|---|---|---|---|
| gap at the attack, A0–D#2, median (dB) | +5.5 | +3.0 | +2.0 | +1.5 |
| C1–F#1 | +6.7 | +4.3 | +3.3 | +2.9 |
| gap at 0.3–0.7 s, C1–F#1 | +10.2 | +7.7 | +6.7 | +6.2 |

The default is 0.003. The bench stays at 1.52, its bass at 0.70 (0.74). The mode still absorbs as before: its radiation does not touch the strings' decay.
