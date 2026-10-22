# Bass string losses

Status as of 2026-10-22.

## Finding

According to the comparison with Salamander, the Iowa and a commercial modelled piano ([comparison.md](comparison.md)), the bass sustain was unbalanced. The fundamental dominated by 10 to 20 dB, and partials 7 to 12 died out twice as fast as they should. The result sounded “barrel-like”.

Median T60 of the partials, in ff, keys A0–B2 (tool `tools/bass_trial.sh`):

| Partials | References | Model before |
|---|---|---|
| 1–2 | 7.4 s | 17.7 s |
| 3–6 | 7.4 s | 8.1 s |
| 7–12 | 6.8 s | 3.5 s |

## Diagnosis

Three leads were ruled out.

- **The strings' own losses.** They give a T60 of 20 to 30 s, so they limit nothing (`examples/bass_budget.rs`).
- **Bridge conductance.** It is close to the estimate derived from Salamander: +1 to +2 dB on average (`examples/bass_conductance.rs`).
- **Bridge mass, sympathy, horizontal polarisation and radiation.** None of them affects these decays.

`examples/partial_modes.rs` gives the real cause for C2, partial 8. The two main modes last 8 s, but they have the same weight and are 0.3 Hz apart, because the two strings are detuned by about 1 cent. Their beating hollows out the envelope around 1.6 s, which gives an apparent decay of 3.2 s. The references measure a frequency difference between components (`after_cents`) of about 0.4 cent in the bass and middle, versus 0.9 cent in the model.

The bass fundamentals (30–120 Hz) fall below the first mode of the clamped soundboard, which is around 60 Hz. There, a soundboard alone absorbs almost nothing. On a piano, the soundboard, the rim and the case also move as a block, which makes a broad, damped mode that absorbs these fundamentals.

## Changes

1. **Unison detuning up to C4**: `UnisonLaw::width_mid` goes from 1.0 to 0.35 cent, the measured difference.
2. **Piston mode** (`SoundboardParams::piston_*`). It adds a uniformly shaped mode to the soundboard modes, tuned on the decays of the bass partials of the three references:
   - frequency 40 Hz, loss factor 0.7, effective mass 1.5 kg;
   - its radiation is reduced to 0.3, because when the whole case moves, the air pushed above largely comes back underneath. With 1.0, the 0–100 Hz band exceeded the reference by 4.6 dB;
   - it is an equivalent mode: the 1.5 kg mass is not a measurement.
3. **Loss floor of the wound strings** kept at 0.2 s⁻¹. With 0.08, partials 3 to 6 lasted too long.

## Result

Median T60 of the partials:

| Register | Partials | References | Before | After |
|---|---|---|---|---|
| bass (A0–B2) | 1–2 | 7.4 s | 17.7 s | 9.6 s |
| bass (A0–B2) | 3–6 | 7.4 s | 8.1 s | 10.0 s |
| bass (A0–B2) | 7–12 | 6.8 s | 3.5 s | 5.4 s |
| middle (C3–B4) | 1–2 | 2.9 s | 3.3 s | 3.0 s |
| middle (C3–B4) | 3–6 | 2.4 s | 1.8 s | 2.2 s |
| middle (C3–B4) | 7–12 | 2.2 s | 1.2 s | 1.8 s |

Long-term spectrum relative to Salamander (`tools/calibrate.sh`, `spectral_stats.txt`):

| | 0–100 Hz | 100–200 Hz | 200–400 Hz | 6.4–20 kHz | tilt error | profile deviation |
|---|---|---|---|---|---|---|
| Salamander | 11.8 | 14.0 | 5.3 | −23.4 | — | — |
| before | 11.9 | 17.3 | 6.6 | −14.1 | 3.5 dB | 6.3 dB |
| after | 12.2 | 11.5 | 6.5 | −22.2 | 2.5 dB | 4.5 dB |

- **Envelopes** (`envelope_compare.py`). The bass and middle stay within 2.5 dB of the reference up to 1 s. After that, they lose 3 to 6 dB too much at 2–3 s.
- **Hammer score.** It goes from 1.9 to 2.2.
- **Cost and tests.** Instrument construction: 1.09 s. The tests pass.

## What remains

- **Bass sustain profile.** Partials 3 to 6 stay 10 to 15 dB below the references relative to the fundamental between 0.5 and 1 s. In the references, the bass fundamental drops by about 8 dB in the very first half-second, a fast initial decay that the model lacks. The bass fundamentals are, however, the most scattered measurements in the references: their T60 ranges from 4.5 to 39 s depending on the source and the note.
- **End of middle notes at 2–3 s**, slightly too short since the tighter detuning.

Update: the detuning (1 cent), the piston mass (6 kg) and the strings' own losses were revisited on the sustain after 1 s ([long_sustain.md](long_sustain.md)).
