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
