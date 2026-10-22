# Note sustain: bridge, soundboard thickness, string losses

Status as of 2026-10-22.

## Listening feedback

“The sustain of the note seems a bit short. It lasts well over time, but loses energy very quickly at the start. It makes for a rather soft piano, not very powerful.”

## Measurement

[`tools/envelope_compare.py`](../tools/envelope_compare.py) compares the model's envelope with Salamander's. The protocol:

- every third key, layer 13 versus velocity 100, key held for 4 s;
- dry render: no room or noises;
- levels expressed relative to the peak.

A high-pass at 0.7·f0 removes the low floor of the recordings. In the treble, this floor dominates the end of the note: noise, room and low resonances. Without this filter, Salamander's treble notes seemed to hold at −40 dB for seconds, and the comparison was wrong beyond 1 s.

[`tools/t60_bins.py`](../tools/t60_bins.py) complements this view. It compares the duration (T60) of each partial, by register and by frequency band, using the ESPRIT components from `tools/calibrate.sh`.

### Mean deviation model − reference (dB), before

| Register | 0.1 s | 0.3 s | 0.6 s | 1 s | 2 s | 3 s |
|---|---|---|---|---|---|---|
| A0–B2 | −2.6 | −0.7 | +0.1 | +1.4 | +2.9 | +3.8 |
| C3–B4 | −2.9 | −5.0 | −5.2 | −3.9 | −4.0 | −3.7 |
| C5–B6 | −2.7 | −5.4 | −7.6 | −7.2 | −18.8 | −23.1 |
| C7–C8 | −2.3 | −6.9 | −10.6 | −10.3 | −11.6 | −8.3 |

The listening impression was right. From the middle to the treble, the note lost 5 to 8 dB too much in the first half-second.

## Diagnosis

The energy leak from a string mode into the bridge is σ = N·G·T/L, with N strings, G the bridge conductance, T the tension and L the length. For C6, the measured fast decay (T60 1.46 s) gives G ≈ 3.7e-4 s/kg. This is also what the global Salamander fit gives (`tools/bridge_conductance.py`): log G = −8.84 + 0.289·log f − 0.0146·key. The model had 7.3e-4 at the C6 bridge point: twice too much.

Since the treble strings are short (large T/L), they are the most sensitive to this excess.

[`examples/bridge_conductance_model.rs`](../crates/piano-core/examples/bridge_conductance_model.rs) fits the same law on the model. Before the fix, the conductance grew too fast with frequency (slope 0.465 instead of 0.289) and did not drop enough towards the treble (−0.0094 per key instead of −0.0146). [`examples/decay_budget.rs`](../crates/piano-core/examples/decay_budget.rs) details, note by note, the string's own losses and the T60 of the coupled modes.

## Changes (`soundboard.rs`, `params.rs`)

1. **The bridge spreads the force** (`bridge_stiffness` 470 N·m², `bridge_line_mass` 0.6 kg/m, `bridge_spread` 0.3). The bridge is a maple beam of about 3 × 2.5 cm glued onto the soundboard. A string's force does not drive the soundboard at a point: it is spread along the bridge.
   - It is spread over 0.3 times the bridge's bending wavelength, 2π·(EI/(μω²))^¼, capped at 0.5 m, with a Hann window along the bridge path.
   - This smoothing applies mode by mode: `Soundboard::bridge_shapes` gives the shape seen by each key.
   - It is used for the soundboard bank inputs, the bridge velocities (sympathy, reciprocity) and the admittance that couples the strings.
   - Effect: the small ripples of the soundboard, i.e. the high modes, are less excited. The conductance therefore rises more slowly with frequency.
2. **The soundboard is stiffer under the treble** (`impedance_slope` 0.035). Soundboards are thicker and more densely ribbed on the treble side, and the end of the treble bridge is close to the rim.
   - The impedance under the bridge varies as e^{0.035·(key − 60)}: ×4.6 at C8, ×0.4 at A0, relative to C4.
   - It is applied to the shapes, so the soundboard also responds less to the treble strings.
   - The modal plate stays uniform: this is an equivalent gradient, not a resolved variable thickness.
3. **High-frequency string losses**: b2 = 1e-7 instead of 3e-8.
   - The bridge spreading slowed down the upper partials too much. With this b2, they recover the measured decay rate.
   - It also removes most of the excess above 6 kHz.
   - b2 = 2e-7 (the value from the measured fast decay) aligns the upper partials even better, but shortens the treble fundamentals by 2 to 3 dB.
4. **Bridge path**, with 5 points instead of 3. The x follows each string's speaking length from the agraffes; the y follows the key's position. The treble end goes into the clamped corner of the soundboard, as on a real piano. On its own, this change alters the decays little.
5. **The hammer strikes the strings of a unison unevenly** (`UnisonLaw::hit_spread` 0.2, i.e. about ±20 % force from one string to the next, distributed irregularly but deterministically by key). The uneven part directly excites the slow anti-phase modes, i.e. the aftersound. In the treble (C5–D#6), the aftersound thus goes from −13.5 to −10.6 dB, versus −11.2 dB in the reference, without changing its duration or the profile deviation (6.0 dB).
6. `LossLaw::b1_floor`: loss floor for the wound strings. It is 0 by default, since the impedance gradient was enough to shorten the bass.

The settings were chosen on a grid. For each trial: full calibration (`tools/calibrate.sh`, one minute) and keyboard envelopes.

## Result

Mean deviation model − reference (dB), after:

| Register | 0.1 s | 0.3 s | 0.6 s | 1 s | 2 s | 3 s |
|---|---|---|---|---|---|---|
| A0–B2 | −2.1 | −1.2 | −1.2 | −0.0 | +1.6 | +1.9 |
| C3–B4 | −1.5 | −1.2 | −0.2 | +1.6 | −2.1 | −4.1 |
| C5–B6 | +1.2 | −0.6 | −3.6 | −3.7 | −11.9 | −20.0 |
| C7–C8 | +5.7 | +3.3 | −4.5 | −8.4 | −14.3 | −11.0 |

- **First second.** The first three registers are within 4 dB of the reference, versus 5 to 8 dB of excess loss before.
- **Partial durations (ESPRIT).** The middle and the upper partials are between −0.4 and +0.7 in log2, versus −0.5 to −0.8 before for the high partials.
- **Spectral profile deviation.** 6.0 dB instead of 7.0 dB.
- **6.4–20 kHz band.** −14.4 dB instead of −8.7 dB, versus −23.4 dB in the reference.
- **Cost.** CPU (28 % of a core at worst) and construction (1.06 s) are unchanged.

## What remains

- **End of note in the treble.** From C5 to B6, between 2 and 3 s, the envelope stays on average 11 to 18 dB below the reference. What has been checked:
  - **It is not the room.** With the small reverberant room, the gap is the same.
  - **It is not the horizontal polarisation.** Excitation ×3 and radiation ×2.5 change nothing.
  - **It is the fundamental.** At 1.8 s, it dominates the reference. The model has 2 dB less of it at F#5, 5 dB at A5 and 10 dB at C5.
  - **The gap varies with the note.** It follows the detuning of each unison, irregular from one key to the next as on a real piano.
  - **Tightening the detuning** (`width_treble` 2.5 or 1.5) brings the frequency difference between components closer to the measurement: 1.0–1.2 cents versus 0.5 in the reference. But the aftersound then weakens (−16 to −23 dB), because in the model it arises mainly from the detuning.
  - **Next lead.** The mechanism that, in a real piano, gives both a small frequency difference and a strong, long aftersound: the reactive part of the bridge admittance and the coupling between neighbouring unisons through the bridge.
- **Bass.** Partials from 300 to 700 Hz decay about 2 times too fast, those below 300 Hz about 2 times too slowly.
- **Una corda.** The aftersound gain is no longer reproduced on average: it varies from −7 to +7 dB depending on the note. The unison coupling has been weaker since these changes. The test only checks that the sustain is not lost.
- **Limits of the soundboard model.** The impedance gradient and the bridge spreading are equivalent models. A soundboard of variable thickness, with coupled ribs and beam bridge, would require numerically computed modes (finite elements) instead of analytical modes.

## Listening feedback: bass “like in a barrel” (2026-10-22)

The impedance gradient also applied on the bass side. It made the soundboard 2 to 2.5 times more mobile under the bass than under C4, and thus reinforced its low resonances. In the 100–200 Hz band, the model exceeded Salamander by 12.8 dB (26.8 versus 14.0), versus 9.7 dB before these changes.

It is now split in two (`impedance_slope_bass` = 0, `impedance_slope` = 0.035 on the treble side). The bass decays are taken over by the own losses of the wound strings: `LossLaw::b1_floor` = 0.2 s⁻¹ below `wound_until` = 48, fading out over the octave above. This is the order of magnitude of the ESPRIT rates of the bass aftersound in Salamander (0.15–0.18).

| | 0–100 Hz | 100–200 Hz | 200–400 Hz | 400–800 Hz | tilt error |
|---|---|---|---|---|---|
| Salamander | 11.8 | 14.0 | 5.3 | 1.0 | — |
| before | 13.0 | 26.8 | 8.3 | 4.1 | 5.3 dB |
| after | 11.9 | 17.3 | 6.6 | 1.8 | 3.5 dB |

Bass envelopes (A0–B2): between −0.9 and +0.8 dB from the reference from 0.1 to 3 s, versus +2.1 to +2.8 dB after 2 s before.
