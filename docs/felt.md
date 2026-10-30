# A graded hammer felt

Status as of 2026-10-24.

## The two-layer hammer and its switch

Since [presence.md](presence.md) the hammer had two layers: Stulov's top felt, Q0·u^p, over a core 30 times as stiff, reached once the felt was compressed by 0.8 mm (bass), 0.45 mm (C4) or 0.4 mm (treble). A blow that reaches the core shortens the contact and brightens the attack.

After the velocity law was fitted on two Disklaviers ([bench.md](bench.md#velocity-curve)), mf is played with a slower hammer than before (velocity 56: 1.19 m/s instead of 1.40). In the bass the mf blow then stopped short of the core, and the attack's share at 4–8 kHz fell 30 dB below a Disklavier's at the same velocity (bench: z −4.5 against eight real grands).

Moving the core showed how abrupt the layered hammer is. Scored on the bass's mf attack (4–8 kHz, z):

| Core depth (bass) | 0.8 mm | 0.75 | 0.72 | 0.7 | 0.65 | 0.55 |
|---|---|---|---|---|---|---|
| z | −4.5 | −1.4 | +0.7 | +1.5 | +3.4 | +5.0 |

A tenth of a millimetre turns too dark into too bright: whether a dynamic sounds bright depends on whether it crosses the threshold. Hammer felt has no such layer. It is pressed onto the wooden molding under tension, and its density grows gradually towards the inside.

## A graded felt

The core term keeps its form, Q_core·(u − d)^p_core, but with no depth (d = 0) and an exponent above the top felt's (p ≈ 3.7–5):

F = Q0·u^p + Q_core·u^p_core

The force is a sum of two power laws: the top felt alone at small compressions, then a continuous stiffening as the second term takes over. No new parameter is needed: `core_depth_*` = 0, `core_p` and `core_hardness` set the stiffening.

Grid against the eight real grands (`cargo xtask bench`, close microphones). The score is the RMS of z, lower is better:

| | two layers | 0.72 mm core | graded p7, ×1.5 | p7, ×2 | p7, ×3 | p6, ×3 | p8, ×2 | p5, ×10 |
|---|---|---|---|---|---|---|---|---|
| score | 1.66 | 1.62 | 1.57 | 1.58 | 1.59 | 1.61 | 1.58 | 1.69 |
| bass | 0.92 | 0.66 | 0.69 | 0.66 | 0.65 | 0.66 | 0.66 | 0.82 |
| middle | 0.76 | 0.74 | 0.72 | 0.70 | 0.68 | 0.71 | 0.78 | 0.94 |
| treble | 1.54 | 1.54 | 1.47 | 1.46 | 1.46 | 1.49 | 1.48 | 1.62 |

The graded felts sit on a plateau (1.57–1.61 for p 6–8, hardness 1.5–3): no knife edge. The default is p_core = 7 and core_hardness = 2. Against the two-layer hammer:
- **Better** (ten measures by 0.5 z or more):
  - the mf attack's 4–8 kHz, bass (z −4.5 → −0.5) and middle (−2.2 → +0.1);
  - the bass's 8 kHz band at the attack;
  - the top's dynamics;
  - the treble's sustain partials 4–6.
- **Worse** (three measures):
  - the bass's 8 kHz band in the sustain (−1.0 → −2.0);
  - the top's partial 3 at mf;
  - the middle's mf centroid (0 → +0.6).

The velocity curve stays within the Disklaviers' (BiVib, velocity 12 re 100: bass −15.9 dB against −18.3, middle −19.4 against −22.4, treble −28.7 against −31.5). The attack now brightens less with velocity: the middle's centroid at velocity 56 is 17 % below that at 100, against 26 % on the Disklavier.

**Una corda** softened the top felt (Q0 × (1 − 0.6)) and pushed the core 30 % deeper. With no depth, the inner term is softened alike. C2/C4/C6 ff then lose 3.3/2.0/5.4 dB, centroid × 0.81/0.84/0.97, against 3.2/2.2/6.3 dB and × 0.77/0.77/1.00 with the two layers.

The two-layer hammer is still available (`core_depth_* > 0`, `core_p` 2.5, `core_hardness` 30): presets that set the depths keep it.

## The fizz of D#4–F#4: a stiffer shank in the middle

Status as of 2026-10-30.

D#4, E4 and F4 fizz at the attack (heard, and measured by the bench's `hf_even`). At mf their 8–11 kHz share in the first 60 ms stood 10–15 dB over their neighbours', and 8–14 dB over the real grands' at the same keys.

**Not the computation.** Rendered at 48, 96 and 192 kHz and measured alike, the gaps of D#4/E4/F4 stay +10/+11/+15, +11/+12/+18 and +10/+12/+18 dB. The contact is integrated at the sample rate below C6 (no oversampling), so halving the step would have moved a numerical artefact.

**Where it comes from: the end of the contact.** The hammer's force (`cargo run --release -p piano-render --example second_impact`, mf on the playback scale) carries the highs in its first episode; the re-contacts after it stay under 0.1 N. At F4 the force has fallen to 0.5 N at 2.65 ms when the bridge's wave comes back; it rises to 2.6 N at 3.0 ms and is gone at 3.2 ms. At C4 the wave comes back after the force has fallen to nothing; at G4–A4, while the felt is still well compressed, and the bump ends smoothly.

**The real grands are flat.** Their 8–11 kHz share at mf sits near −74 dB from C3 to B4 (Helgesson, Splendid, Salamander), within ±4 dB key to key. The model's ran from −112 dB (A3) to −60 dB (F4).

**What moves it.** The force's 8–11 kHz share key to key over C3–B4 (rms/largest gap, dB; mf):

| | control | `hysteresis` 4 | `contact_width_scale` 3 | `shank_loss` 0.8 | `shank_share` 0.15 | `shank_freq` 200 | 400 | 500 | 700 |
|---|---|---|---|---|---|---|---|---|---|
| force | 4.9/12.6 | 3.8/6.4 | 4.1/11.3 | 2.8/6.2 | 2.0/2.5 | 4.9/9.7 | 5.4/11.3 | 3.8/6.3 | 3.4/4.7 |

Most of these move the coincidence to other keys (rendered, `shank_loss` 0.5–1.2 and `shank_share` 0.1 or 0.2 left gaps of 17–30 dB elsewhere; the earlier trials in [bench.md](bench.md) did the same). Hysteresis brightens the whole middle (+20 dB at 8–11 kHz). A shank mode at 700 Hz, three octaves of velocity, rendered (`hf_even` 8–11 kHz, rms/largest gap, dB):

| velocity | 40 | 56 | 75 |
|---|---|---|---|
| 300 Hz | 9.4/28.8 | 8.6/17.0 | 5.7/17.5 |
| 700 Hz | 7.1/15.1 | 4.6/11.4 | 4.7/12.4 |

At mf D#4–F#4 come to −75, −76, −75, −76 dB (−67, −69, −60, −68 before), within the real grands' −71 to −79.

**But not in the top.** At 700 Hz everywhere the top loses the level the supple shank gave it ([top_level.md](top_level.md)): its mf balance falls from −6.6 to −10.6 dB (z −1.9), its mf partials 3–4 by 9 dB, and the treble's mf balance goes to z −2.0. The shank mode is now `shank_freq_mid` (700 Hz) up to C5, geometric to `shank_freq` (300 Hz) at C6.

Bench: 1.14 → 1.14, 11 off. Better: `hf_even` 8–11 kHz largest gap z +3.8 → +2.1, rms +1.9 → +0.7, 4–8 kHz largest gap +1.8 → +0.8; the treble's balance (ff +1.4 → −0.1, mf +1.1 → −0.4); the bass's ff partial 6 (−2.3 → −1.5 sustain, −1.3 → −0.5 attack). Worse: the top's mf balance (−0.7 → −1.6), the treble's mf partial 3 (+0.4 → +1.5) and its ff early decay of partials 5–12 (−0.1 → +1.3), the bass between the partials (−0.8/−0.5 → −1.5/−1.3).

Still open: below C4 the model's attack has 20–35 dB less at 8–11 kHz than the real grands (−98 to −109 dB at C3–A3), and the step up to C4 is now `hf_even`'s largest gap.
