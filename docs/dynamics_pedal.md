# Dynamics, pedal and sympathetic resonance

Status as of 2026-10-22.

## Listening feedback

First listening to real performances (MAESTRO, Disklavier): “It plays a bit like a lumberjack, the dynamic curve isn't right. It resonates for a very long time and loudly: is it the pedal not working, or is it playing too loud?”

Three causes were found; none came from the timbre.

## 1. MIDI velocity → hammer speed

The old law (0.4 → 4 m/s, exponential) had been fitted on Salamander's layers. But the mapping between Salamander's layers and velocities is that of a sampler, not a measurement.

Reproducing pianos follow a logarithmic scale: velocity = 52 + 25·log₂(v), with v the hammer speed in m/s. This is the Bösendorfer SE map, and the Disklavier places 2 m/s around 77 ([Goebl & Bresin, JASA 2003](https://iwk.mdw.ac.at/goebl/papers/Goebl-Bresin_JASA2003_reproAccuracy.pdf)). This map runs from 0.24 to 8 m/s. The old one squashed both ends: at velocity 20, 0.57 m/s instead of 0.41; at 110, 3.0 m/s instead of 5.0. Soft dynamics were too loud and loud ones not loud enough, which made the playing uniform.

- First default adopted: `VelocityLaw { vmin: 0.243, vmax: 8.0 }`. After comparative listening, it was replaced by an intermediate one (see below).
- The old law remains available as `VelocityLaw::SALAMANDER_LAYERS`.
- At 127, a lone A0 peaks at −6.7 dBFS. Across the six pieces, the maximum peak is −6.5 dBFS (Campanella).

## 2. Pedal dead travel

Pianists rest their foot on the pedal. In the MAESTRO Bach, CC64 never drops below 41, and 49 % of the time it is between 64 and 90.

The old model lifted the dampers fully from 0.4 of travel (CC 51), and halfway from CC 13. The whole Bach was therefore played with the pedal down.

On a real piano, the pedal has some dead travel before it touches the damper rail (`DamperLaw::pedal_free`).

First setting (0.35 → 0.8: half-pedal around CC 64, dampers lifted around CC 102): the resonance was under control, but the notes seemed “dryly damped, disconnected”. The Bach spends half its time between CC 64 and 90, a zone this setting treated as half-pedal. Yet the recorded files follow the MIDI convention “pedal down from 64”, which the MAESTRO authors apply themselves.

Chosen setting, 0.3 → 0.55:

| CC64 | 30 | 40 | 45 | 50 | 55 | 60 | 64 | 70 |
|---|---|---|---|---|---|---|---|---|
| damper contact | 1.00 | 0.91 | 0.69 | 0.49 | 0.32 | 0.17 | 0.08 | 0 |

These values remain estimates, as Yamaha does not publish the mapping.

**Felt non-linearity** (`DamperLaw::contact_exponent` = 2.5). The felt was a friction proportional to contact, set to maximum absorption. A half-resting damper therefore already damped at 89 % of the maximum: there was no real half-pedal, and each note stopped dead as soon as the damper brushed the string.

Friction now follows contact to the power 2.5, like Stulov's hammer felt. The damping rate of a C4 is 49 % of the maximum at half contact and 7 % at 0.2 contact. With the damper resting, nothing changes: T60 of 0.11 s for C4, 0.47 s for E1.

## 3. Sympathetic resonance level

`examples/sympathy_level.rs` measures, with the pedal down, what the listening strings add to the sound. With the “physical” coupling (gain 1):

- a lone C4 gained +8 dB between 1.5 and 3 s;
- a C2 gained +11 dB, and the C1 string ended up louder than the played note;
- a C–E–G–C chord gained +6 dB from the attack, and +10 dB afterwards.

In the Chopin, sympathetic resonance added 3.6 dB to the whole.

The cause is the one-way coupling (phase 3): the listening strings take their excitation from the soundboard without taking energy away from it. When partials coincide, which happens at octaves, they grow well beyond what a passive system allows. Two-way coupling, delayed, was unstable.

Pending a passive coupling, `SympatheticLaw::gain` goes to 0.25. This is not a measurement, it is a compensation. With this gain:

- a chord with the pedal down gains +1.3, +2.5 then +1.4 dB;
- a single note stays within ±2 dB;
- the whole Chopin gains 0.1 dB.

## Touch: comparative listening

Salamander's velocities come from a sampler, MAESTRO's from a Disklavier. Measurements alone do not tell which scale suits these files. Compared with Salamander, the Disklavier scale widens the dynamics: 30 dB from softest to loudest instead of 21. It also brightens the attack of loud dynamics: centroid ×1.4–1.7 instead of ×1.2–1.5. The three settings `out/ab/*.toml` were decided between by listening (A: Disklavier, B: Salamander, C: intermediate). The choice fell on **C**, which became the default setting: `VelocityLaw { vmin: 0.3, vmax: 5.5 }`, with 2 m/s around velocity 84.

After the hammer–string contact fix ([comparison.md](comparison.md)), the law was recalibrated together with the hammer, on the dynamics of Salamander and a commercial modelled piano at the same velocities: 0.6–3.0 m/s. This choice made by listening therefore needs redoing.

## To do

- **Two-way passive sympathetic coupling.** This would be the real fix: the listening string takes energy back from the soundboard. For example, an instantaneous coupling matrix restricted to pairs of nearby partials, with no delay.
- **Measure the pedal travel of a Disklavier.** Recording it would replace the two estimates above.
- **Calibration tests on Salamander.** They still use the sampler's layer velocities (`piano-eval --velocities`). To compare dynamics, `--vmin 0.4 --vmax 4` must now be passed.
