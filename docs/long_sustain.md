# Sustain after the first second

Status as of 2026-10-22.

## Observation

Compared with a commercial modelled piano, Iowa and Salamander, the model had a correct first second, then lost 6 to 17 dB too much between 2 and 3 s, in all registers. Notes “sang” for too short a time.

## Measurement

Protocol from [comparison.md](comparison.md): one key in three, velocities 40, 60 and 100, key held for 4 s, close miking (`presets/close.toml`), no room or noises. Each trial is rendered then cut with the tools from that comparison.

Tools added:
- `tools/partial_decay.py`: level of partials 1 to 12 at 0.1, 0.5, 1, 2 and 3 s;
- `tools/partial_t60.py`: effective decay of each partial between 0.3 and 3 s (beats averaged);
- `tools/band_decay.py`: levels per octave band in the attack, at 0.3 s and at 3 s;
- `tools/aftersound_compare.py`: “prompt” sound and aftersound from the ESPRIT components of `piano-survey`, i.e. their duration, level difference and frequency difference;
- `examples/aftersound.rs`: the same decomposition computed directly on the model's coupled modes.

## Diagnosis

The component-by-component decays (ESPRIT) were correct. What was missing comes down to three causes, one per register.

### Middle: aftersound too weak

In all three references, the aftersound starts about 15 dB below the “prompt” sound and lasts about 9 s. It takes over around 2 s. In the model, it was at −26 dB: the end of the note was just the tail of the “prompt” sound.

The frequency difference between these two components reveals the cause. Measured the same way everywhere, it is:

| | Commercial | Iowa | Salamander | model (0.35 cent) |
|---|---|---|---|---|
| bass | 1.50 | 1.28 | 1.54 | 0.66 |
| middle | 0.65 | 0.99 | 0.78 | 0.48 |

The unison detuning, reduced to 0.35 cent for [bass.md](bass.md), was too tight. If the detuning is small compared with the coupling through the bridge, the antiphase mode barely moves the bridge, so it does not radiate.

### Bass: a piston mode that absorbs too much

Level per octave band, keys A0–B2, loss between 0.3 and 3 s:

| Band | Commercial | Iowa | Salamander | model (piston 1.5 kg) | without piston |
|---|---|---|---|---|---|
| 63 Hz | 21 dB | 11 dB | 6 dB | **37 dB** | 19 dB |
| 125 Hz | 14 dB | 20 dB | 14 dB | 18 dB | 16 dB |
| 500 Hz | 12 dB | 11 dB | 11 dB | 15 dB | 14 dB |

The case's piston mode (40 Hz, heavily damped) had been tuned to the decay of the “prompt” sound alone. Yet in the references, bass fundamentals decay in two stages: a fast drop, then a slow tail. The piston absorbed both. With 6 kg instead of 1.5, it absorbs four times less.

The frame, the rim, the nonlinearity and sympathetic resonance play no part (trials on C2 and F#1).

### Treble: intrinsic string losses too high

From A♭5 to D#6, the slow component of the fundamental lasts 7 to 9 s in the references, versus 3 to 4.5 s in the model. It was capped by the intrinsic string losses: T60 of 5.3 s at C6 and 3.6 s at F#6.

### Leads with no effect

Horizontal polarisation (oblique strike ×3 or ×6, radiation ×2 to ×3, horizontal coupling ×3), more uneven strike (±40 %) and piston radiation: none of these settings changed the envelope at 3 s by more than 2 dB.

## Changes

1. `UnisonLaw::width_mid` goes from 0.35 to 1.0 cent. The prompt/aftersound difference becomes 1.3 cent in the bass and 1.0 in the middle, and the middle aftersound rises to −14 dB.
2. `SoundboardParams::piston_mass` goes from 1.5 to 6 kg.
3. `LossLaw`: b1 = exp(0.055·key − 5.1) instead of exp(0.059·key − 4.782), i.e. ×0.65 at C4 and ×0.5 at F#6. b2 and the wound-string floor are unchanged.

## Result

ff sustain, in dB relative to the peak, at 0.3 / 1 / 2 / 3 s:

| Register | Commercial | Iowa | Salamander | before | after |
|---|---|---|---|---|---|
| bass | −3 / −8 / −13 / −17 | −5 / −11 / −15 / −18 | −4 / −9 / −14 / −17 | −5 / −11 / −18 / **−24** | −4 / −10 / −15 / −20 |
| middle | −6 / −19 / −30 / −33 | −8 / −19 / −24 / −28 | −7 / −20 / −26 / −32 | −7 / −19 / −31 / **−39** | −7 / −18 / −25 / −31 |
| treble | −17 / −32 / −41 / −50 | −18 / −27 / −39 / −46 | −16 / −30 / −37 / −46 | −16 / −34 / −50 / **−62** | −15 / −29 / −36 / −48 |

mf follows in the same way.

Aftersound (ESPRIT, ff), as T60 and level relative to the “prompt” sound:

| Register | References | Before | After |
|---|---|---|---|
| bass | 20–38 s, −12 to −15 dB | 11 s, −22 dB | 18 s, −13 dB |
| middle | 9–10 s, −10 to −17 dB | 11 s, −26 dB | 10 s, −14 dB |
| treble | 3 s, −12 dB | 3.1 s, −15 dB | 3.2 s, −11 dB |

Other measurements:
- **Hammer score.** It goes from 1.69 to 1.38. The bass attack is less dominated by the fundamental: partial 2 at +4.9 dB, versus +1.7 dB before and +4 to +9 dB in the references.
- **CPU.** Unchanged: 38 % of one core at worst (`examples/stress_cost.rs`).
- **Tests.** They pass.

## What remains

- **End of bass notes**: still 2 to 3 dB below the references at 3 s.
- **Spectrum of the bass sustain**: the fundamental dominates more than before, by about 3 dB. Partial 2 at +4 dB relative to partial 1 between 0.5 and 1 s, versus +14 dB in the references. The cause is radiation: the 63 Hz band is the model's strongest in the attack, whereas it is 3 to 15 dB below the strongest in the references. Since addressed: [bass_radiation.md](bass_radiation.md).
- **Duration of the “prompt” sound**: a little long in the bass (13 s versus 8 to 11 s) and the middle (4.8 s versus 2.9 to 3.7 s), while the envelope is right.
