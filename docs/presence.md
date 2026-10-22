# Presence and attack: two-layer hammer, microphone placement

Status as of 2026-10-22.

## Listening feedback

“The version from the commercial modelled piano is more percussive, and ours sounds like playing with the lid closed.”

## Measurement

`tools/presence.py` compares two things on the comparison sequence:
- **percussiveness**: peak over 5 ms minus the level between 150 and 300 ms;
- **the energy share per band** in the attack (0–60 ms) and in the sustain.

The model's percussiveness was normal: 6 to 7 dB in the bass and middle, like the references. What was missing was the high frequencies in the attack of the bass and middle. Bass ff attack, in dB:

| | 1–2 kHz | 2–4 kHz | 4–8 kHz |
|---|---|---|---|
| Commercial | −10 | −17 | −29 |
| Iowa | −8 | −20 | −45 |
| Salamander | −17 | −27 | −57 |
| Model | −25 | −43 | −78 |

## Causes

1. **The felt calibrated for brightening with dynamics** ([timbre_dynamics.md](timbre_dynamics.md)) was very soft and very nonlinear (hardness 0.1, p + 2). It cut the high frequencies: in the bass, −63 dB at 4–8 kHz versus −44 with the nominal felt. The calibration only looked up to partial 12, i.e. about 3 kHz in the middle.
   - The hammer force on the bridge already lacked high frequencies.
   - Soundboard radiation then removes another 5 to 10 dB.
   - The other changes (piston mode, unisons, nonlinearity) play no part.
2. **The microphone placement.** The concert microphones at 1 m height sat below the lid reflection, which removed about 10 dB at 2–4 kHz in the middle. The microphones above the soundboard, at 1.5 m, are also darker than close microphones.

## Changes

**Score.** `tools/hammer_score.py` now measures attack presence, i.e. the energy share at 2–4 kHz and at 4–8 kHz, at ff and mf, per register.

**Two-layer hammer** (`hammer.rs`, `felt()`): F(u) = Q·u^p + Q_c·(u − d)₊^p_c. This is a surface felt over a hard core. The quadratised scheme stays passive (g = F/√(2V), hysteresis α·dF/dt), and the energy conservation and decay tests pass with the core. The measured compression ranges from 0.35 to 1.2 mm (`examples/contact_stats.rs`); the core is placed between pp and ff.

Selected parameters (grids `tools/core_grid.sh`):

| Parameter | Value |
|---|---|
| `hardness` | 0.2 (surface) |
| `p_offset` | 0 (Stulov's p, 3.7 to 5) |
| `hysteresis` | 0.6 |
| `core_hardness` | 30 (× the key's Stulov Q0) |
| `core_p` | 2.5 |
| `core_depth_bass` / `core_depth_treble` | 0.7 / 0.4 mm |
| `hardness_slope` / `hardness_slope_bass` | 0.02 / 0.02 |
| `p_bass` | 0 |
| velocity | 0.5–3.5 m/s |

**Presets.**
- `concert.toml`: microphones raised to 2 m.
- `close.toml` (new): microphones 40 cm above the soundboard, lid at 35°. This is the closest to the references' balance.
- `output_gain_db` (new parameter) equalises the loudness between microphone setups: +3.4 dB for `concert`, −9.6 dB for `close`. The Chopin peak stays at −18.8 dBFS in both.

## Result

Hammer score, with presence: 3.23 → 1.54. ff attack, energy share (dB):

| | Bass 2–4 kHz | Bass 4–8 kHz | Middle 2–4 kHz | Middle 4–8 kHz |
|---|---|---|---|---|
| references (mean) | −21.5 | −43.8 | −19.1 | −36.6 |
| model before | −36.5 | −68.5 | −24.3 | −42.0 |
| model after (microphones above the soundboard) | −24.6 | −36.4 | −23.9 | −37.8 |
| model after, `close` preset | −23.8 | −38.4 | −25.9 | −42.2 |

- **pp → ff brightening** (slope, target +6.1 in the bass, +17.4 in the middle): bass +5.5 → +4.0 dB, middle +15.5 → +13.3 dB. This is the price of presence. The two-layer hammer gives the ff its brilliance, but also gives some to the pp.
- **ff − pp dynamics**: 7.7 dB in the bass, 9.0 in the middle, 13.6 in the treble.
- **CPU cost**: unchanged, 37 % of one core at worst.

## What remains

- **The commercial piano remains brighter than the two real pianos** above 4 kHz (−29 dB in the bass versus −45 and −57). The model aims at the mean of the three.
- **Brightening of the bass and middle** still 2 to 4 dB below the references.
- **Treble** slightly too contrasted between pp and ff (13.6 versus 12.2 dB).
