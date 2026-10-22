# Radiation of bass fundamentals

Status as of 2026-10-22.

## Observation

During the sustain (0.5–1 s), the fundamental of bass notes dominated: partials 2 to 6 ranged from 3 dB below it to 7 dB above, versus 10 to 15 dB above in all three references. This is the “barrel” sound.

`tools/third_octaves.py` measures the third-octave spectrum of keys A0–B2 (ff, 0.05–0.45 s). Levels are expressed relative to the energy between 100 Hz and 1 kHz:

| Band | Commercial | Iowa | Salamander | model before |
|---|---|---|---|---|
| 25–40 Hz | −34 to −41 | −43 to −47 | −27 to −31 | −10 to −13 |
| 50 Hz | −16 | −30 | −21 | +0.5 |
| 63 Hz | −5 | −25 | −21 | +5.6 |
| 79 Hz | −3 | −8 | +2 | +0.3 |
| 100 Hz | +3.5 | −4 | +8 | −7.4 |

Below 70 Hz, the model radiated 15 to 30 dB too much, and the 100 Hz band was a dip.

## Diagnosis

`examples/bass_radiation.rs` (crate piano-render) computes the transfer function between the force at the bass bridge and the pressure at the microphones, per third octave. It also shows the effect of several variants. Three causes stand out.

1. **Radiation computed from velocity.** Each mode's weight towards the microphones is computed once, at its resonance frequency (Rayleigh integral). It was applied to the modal velocity. But the radiated pressure follows the surface acceleration. Below a bass fundamental, the soundboard moves mostly through the quasi-static response of the modes from 100 Hz to 1 kHz. Each of them then radiated as if at its own resonance, i.e. ω_n/ω times too much: 10 to 30 dB too much below 50 Hz.
2. **The piston mode radiated.** The mode representing the rigid-body motion of the case ([bass.md](bass.md)) radiated with a weight of 0.3. But the air it pushes above the soundboard returns via the underside, which is open: this radiation is almost entirely short-circuited.
3. **The first soundboard mode was too low.** Mode (1,1), at 61 Hz, was the only one between 40 and 110 Hz, at the centre of the soundboard, just below the bass bridge. The references, however, only radiate well from 80 to 100 Hz upwards. On a real soundboard, the crown and the downbearing of the strings on the bridge make it a prestressed shell: its membrane stiffness mainly raises the lowest modes.

The short circuit between the two faces of the soundboard (`back_gain`) plays almost no part: going from 0.6 to 1, or making the cancellation total at low frequencies, changes only 1 to 2 dB.

## Changes

1. **Radiation from acceleration** (`board::to_acceleration_weights`). The microphone weights of all radiating banks (soundboard, frame, rim) are divided by the response of a differentiator at the mode frequency. The engine then applies this differentiator, y[n] = (x[n] − x[n−1])·fs, to the sum of the microphones.
   - At resonance, the result is unchanged; below it, it is proportional to ω.
   - The bank sleep criterion (`energy`) takes this factor into account.
   - `board::mic_transfer` gives the transfer as rendered.
2. **The piston mode barely radiates any more**: `piston_radiation` goes from 0.3 to 0.03.
3. **Crown**: `crown_freq` = 90 Hz, a distributed stiffness (elastic foundation) that adds (2π·90)² to the ω² of each soundboard mode. Mode (1,1) moves from 61 to 109 Hz; above 300 Hz, the modes barely move.
4. **Stiffer bass side of the soundboard**: `impedance_slope_bass` goes from 0 to −0.03, i.e. an impedance ×2.2 at A0.
   - The crown groups the first modes between 150 and 300 Hz. The bass bridge absorbed more there, and the end of bass notes lost 5 dB at 3 s.
   - Reducing the intrinsic losses of the wound strings instead (`b1_floor` 0.1) gave the right envelope. But the component-by-component decays became twice too long: 23 s versus 8 to 11 s.

## Result

Spectrum of bass notes (same measurement as above):

| Band | 25–40 Hz | 50 Hz | 63 Hz | 79 Hz | 100 Hz | 126 Hz | 200 Hz |
|---|---|---|---|---|---|---|---|
| references | −27 to −47 | −16 to −30 | −5 to −25 | −8 to +2 | −4 to +8 | −10 to +5 | −0.4 to +0.5 |
| before | −10 to −13 | +0.5 | +5.6 | +0.3 | −7.4 | −4.0 | +5.3 |
| after | −27 to −47 | −25 | −15 | −12 | +5.7 | −2.1 | +2.3 |

Bass sustain profile (0.5–1 s), partials 2, 3, 4 and 6 relative to partial 1:

| | Commercial | Iowa | Salamander | before | after |
|---|---|---|---|---|---|
| dB | +14 / +13 / +10 / +12 | +15 / +15 / +15 / +18 | +14 / +14 / +13 / +8 | +4 / +0.5 / −7 / −3 | +13 / +11 / +12 / +7 |

Other measurements:
- **Bass attack** (partials 2 and 3 relative to partial 1): +7 / +11 dB, versus +4 to +9 / +3 to +14 in the references (+5 / 0 before).
- **Hammer score**: 1.38 → 1.11.
- **Sustain**: unchanged in the bass (−19.6 dB at 3 s). The middle loses 1.4 dB at 3 s (−32.1), within the references' range (−27.5 to −32.7).
- **Level**: the bass stays 1.8 dB below the middle (references: from −3.3 to +1.2 dB). The Chopin rises by 1 dB (peak −17.8 dBFS).
- **CPU**: unchanged (37 % of one core at worst). The tests pass.

## What remains

- **Bass between 1 and 2.5 kHz**: 3 to 6 dB below the references during the attack, hence a slightly dark bass. This is the job of the sustain brightness work.
- **79 Hz and 317 Hz bands**: dips of 5 to 10 dB, due to the mode distribution of a uniform rectangular plate. A real soundboard (shape, ribs, variable thickness) would distribute its modes differently.
- **Decay of the “prompt” sound in the bass**: a little long (17 s for partials 1–2, versus 8 to 11 s), while the envelope is right.
