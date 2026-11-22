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

## A physical loss law for the strings (2026-11-21, option, off)

`losses.physical` (off by default; off, the engine is bit for bit what it was) replaces, above each key's fundamental, the frequency term `b2·f²` and its three zones (`b2` up to C3, `b2_top` from C4, `b2_high` from C7) by the losses of a steel string, computed from each string's own data.

### The law

Valette & Cuesta (*Mécanique de la corde vibrante*, Hermès 1993), in the form Issanchou, Bilbao, Le Carrou, Touzé & Doaré give it (*J. Sound Vib.* 393, 2017, eq. 12; read in the authors' preprint):

- Q⁻¹ = Q_air⁻¹ + Q_ve⁻¹ + Q_te⁻¹, with Q = π f / σ (σ the amplitude decay rate, 1/s).
- Air friction: Q_air⁻¹ = R / (2π μ f), R = 2π η + 2π d √(π η ρ f), so σ_air = R / (2μ). η = 1.8e-5 kg/(m s), ρ = 1.2 kg/m³. A 1 mm steel string at 4 kHz: 2.4 dB/s.
- Viscoelastic bending: Q_ve⁻¹ = 4π² μ E I δ_ve f² / T². That is δ_ve·B n² for partial n. The code uses δ_ve·x/(1 + x), the bending share of the partial's energy, with x(1 + x) = B f²/f0². To first order this is the same, and it stays bounded at high n.
- Constant loss factor (thermoelastic, dislocations): Q_te⁻¹, so σ_te = π f Q_te⁻¹.
- The literature's values for a 0.43 mm steel guitar string, measured isolated, are δ_ve = 4.5e-3 and Q_te⁻¹ = 2.03e-4. These are the defaults of `delta_ve` and `q_te`. `air_scale` scales the air term (default 1).
- The brief this was written from gave the air term as (ρ_air/ρ_s)(2√2 M + 1)/M² with M = (d/4)√(2π f/ν). Worked out, that expression is twice Issanchou's R-form, term by term. The 1993 book was not reached to settle which is right, so the code uses the published R-form.

What the engine knows per string (`StringGeometry::of`):

- **Core diameter.** From B, as `note_model.rs::longitudinal` does: d⁴ = 64 B T L² / (π³ E). This gives C4 at 1.01 mm. Its steel mass matches the tabulated μ there within 1 %.
- **Winding.** Any mass more than 10 % over the core's is taken as one layer of copper wire wound touching. That gives the outer diameter, 5.1 mm at A0 and 0.76 mm at C8.
- **Rear lengths.** Behind the bridge, the same wire takes its own f0 and B.

### How it combines with b1 and the winding

- **b1 keeps the fundamental.** `b1(note)` is the empirical per-key constant: it already holds what air and the string do at the fundamental, and it was set together with `b2·f0²`. So the law adds Σ(f) − Σ(f0), and the b2 term is frozen at the key's fundamental. Partial 1 keeps today's rate exactly.
- **A first trial was wrong.** It subtracted b2·f0² as well, and the treble's partial 1 then rang 40 % too long (T60 of partial 1, treble 7.3 s against 5.2).
- **The bridge is not part of the law.** Its loss stays where it is (`eps`, the board's admittance).
- **The winding term is kept as it is.** `winding`, ½ η ω on A0–E2, is a constant loss factor in the winding's friction against the core. That is a mechanism the steel's Q_te does not contain, with its own fade and its own calibration (docs/bass_metal.md).

### What the real grands do

Script: decay of each partial line, 0.3–2.2 s after the onset, slope over 0.2 s windows. The real pianos are Splendid, Plath, Helgesson, Headroom and Salamander, at forte, every third key from C3 to C7. Rates are in dB/s:

| register | 1.2–2.5 kHz | 2.5–4 kHz | 4–8 kHz |
|---|---|---|---|
| C3–B3, real (median of 5) | −8.9 | −11.0 | −15.1 |
| C3–B3, A today | −12.1 | −22.2 | −48.9 |
| C3–B3, F | −13.2 | −15.9 | −23.9 |
| C4–B4, real | −10.9 | −13.9 | −19.4 |
| C4–B4, A | −14.9 | −44.9 | −76.7 |
| C4–B4, F | −10.8 | −17.6 | −24.7 |
| C5–B5, real | −13.1 | −17.5 | −29.1 |
| C5–B5, A | −19.1 | −35.7 | −58.4 |
| C5–B5, F | −13.5 | −20.7 | −38.9 |
| C6–C7, real | −17.8 | −20.0 | −34.6 |
| C6–C7, A | −18.7 | −32.1 | −59.3 |
| C6–C7, F | −18.5 | −22.5 | −43.0 |

**What the late decays follow.** Pooled over the partials above 1 kHz, the decay rate correlates with:

| variable | r |
|---|---|
| f | 0.57 |
| key | 0.47 |
| B k² | 0.36 |
| rank | −0.01 |

- **Frequency, not rank.** Within a key the rate grows like f^0.5–0.8; the pooled fit is f^0.59 with e^(0.022·key).
- **Not B k² either.** The viscoelastic term grows like f³ within a key. Across keys it would make the bass lossier at a given frequency, because B/f0² falls from 6.4e-9 at C3 to 2.5e-9 at C6. The real grands do the opposite.
- **So δ_ve = 0 in the candidates.** The guitar string's 4.5e-3 alone would add about 30 dB/s at C4, 4 kHz.

**What the real decays leave for the string.** The engine's bridge and board losses were taken off with `crates/piano-host/examples/string_losses.rs`, which gives the slowest coupled mode per partial. The engine's own lines also decay 1.2–1.5 times faster than that slowest mode, and the real rates were divided by the same factor. What remains is nearly a constant loss factor: Q⁻¹ ≈ 0.9e-4 at C3, 1.1–1.2e-4 at C4 and 1.25–1.35e-4 at C5, from 1.2 to 8 kHz. That is air (0.1–0.25 s⁻¹ here) plus `q_te` = 1.0e-4.

### Candidates and scores

The presets are in out/presets/p3:

- **F:** the law (`air_scale` 1, `delta_ve` 0, `q_te` 1.0e-4) with today's damper.
- **G:** F with the grip damper of out/presets/p2/B_damper.toml.

Trials:

| q_te | total off | older 13 off | |
|---|---|---|---|
| 1.0e-4 | 709 | 443 | retained |
| 1.3e-4 | 707 | 430 | Lines bass and middle worse at 1.2–2.2 s |
| 0.7e-4 (with the grip damper) | 693 | 452 | |

Scores with `--score`, band without bivib_grand, iowa, liang:

| | A today | C (grip + b2_top 2e-7) | E (grip + b2_top 1e-7) | F | G |
|---|---|---|---|---|---|
| total off / mean | 760 / 1.156 | 669 / 1.116 | 682 / 1.121 | 709 / 1.127 | 693 / 1.118 |
| 13 older off / mean | 376 / 0.953 | 396 / 0.981 | 436 / 1.017 | 443 / 1.012 | 443 / 1.012 |
| Lines 4–7 kHz 0.6–1.2 s, middle (band −55.3) | −86.6 | −65.3 | −55.9 | −52.1 | −52.1 |
| Lines 2.5–4 kHz 1.2–2.2 s, middle (band −54.4) | −75.8 | −61.2 | −53.5 | −55.8 | −55.8 |
| Spectral tilt off | 19 | 27 | 45 | 64 | 64 |
| Brightness, sustain off | 25 | 38 | 44 | 37 | 37 |

- **The law repairs the Lines** in the middle and the treble, as well as `b2_top` 1e-7 does.
- **It costs more of "Spectral tilt" than any b2 setting.** The treble's tilt over 0.02–0.32 s is −18.7 against −24.7 for the band.
- **Partial 1 is unchanged.**

### What is not explained

- **The fast first stage of the treble.** Over 0.02–0.32 s the real grands' partials at 2.5–8 kHz fall fast: C5 −66 dB/s at 4–8 kHz, C6–C7 −74. Then they ring on slowly: C5 −29, C6–C7 −35. The steep `b2` zones gave the model the first stage (C5 −127) at the price of the second (−58). One rate per partial, as the physical law gives, cannot have both: F has C5 −28 early and −39 late. The real grands' first stage is not the string's own loss. It is most likely the unisons' and polarisations' coupling through the bridge at high frequency, which the model's coupled modes do not produce there. That is where "Spectral tilt" and "Early decay" in the treble go wrong with F and G.
- **The growth of Q⁻¹ with the key.** It goes from 0.9e-4 at C3 to about 1.3e-4 at C5 and higher in the top. Neither the air term nor δ_ve gives it.
- **The bass's 1.2–2.5 kHz.** At 1.2–2.2 s it falls further under the band with the law: −40.6 against −32.0, where today's preset is at −36.0. q_te 1e-4 takes more at 1–3 kHz than `b2` 1e-7 did. The bass was not measured below C3 here.
- **The late slopes are still a little fast.** C3–C4 at 4–8 kHz: −24 against −15 to −19 dB/s. A lower q_te (0.7e-4) did not score better.
- **Listening.** Nothing has been listened to. Every earlier lowering of the string losses was heard as strings that buzz while playing (docs/bass_gaps.md, docs/bass_unisons.md). The page out/ab_p3 is there for that.

## The treble's two stages (2026-11-21, options, off)

With the physical law (G, out/presets/p3) the treble's upper partials have one stage. On the real grands they have two: a fast fall over the first 0.3 s, then a slow ring. This section finds why the engine's coupled modes do not give the first stage at 2.5–8 kHz, and adds two options: `soundboard.bridge_yield` (a fitted law) and `unison.b_spread`. Off, the engine is bit for bit what it was.

### The two stages, measured

Every key from C4 to C7, ff (layer 13), the same scripts on the five real grands (Splendid, Plath, Helgesson, Headroom, Salamander) and on the model's renders (close microphones, studio, held 3.5 s). Per line: the median of each partial line's slope, early over 0.02–0.32 s (0.1 s windows every 25 ms), late over 0.3–2.2 s (0.2 s windows); the real value is the median of the five pianos' medians. dB/s, early / late:

| register, band | real | A today | C | G | H | H2 |
|---|---|---|---|---|---|---|
| C4–B4, 2.5–4 kHz | −52 / −15 | −68 / −46 | −49 / −26 | −42 / −18 | −37 / −17 | −49 / −15 |
| C4–B4, 4–8 kHz | −49 / −20 | −135 / −76 | −73 / −67 | −29 / −25 | −41 / −23 | −43 / −19 |
| C5–B5, 2.5–4 kHz | −76 / −18 | −65 / −36 | −44 / −27 | −36 / −21 | −48 / −16 | −51 / −15 |
| C5–B5, 4–8 kHz | −68 / −27 | −123 / −57 | −65 / −65 | −26 / −35 | −56 / −22 | −38 / −23 |
| C6–C7, 2.5–4 kHz | −56 / −20 | −58 / −32 | −47 / −25 | −44 / −23 | −79 / −19 | −63 / −17 |
| C6–C7, 4–8 kHz | −79 / −37 | −119 / −54 | −81 / −65 | −44 / −39 | −63 / −27 | −43 / −26 |

The same lines summed over the band (as the comparison tool's Lines do), C5–B5 at 4–7 kHz: real −61 / −23 dB/s and a level at 0.3 s of −16.5 dB re 0.02 s; G −18 / −28 and −10.7; H −52 / −19 and −15.2. The single strings of A today fall too fast in both stages (b2·f²); G has the late stage right and no early one.

### Why the coupled modes give no first stage up there

`crates/piano-host/examples/unison_stages.rs` prints, per partial, what the engine builds: the string's own rate σ_s, the bridge coupling ε_V = (T/L)·Y (Re: one string's bridge loss), the strings' frequency split, the six coupled modes (rate, offset, weight in the blow and in the output), and from them the line's envelope read in the measures' windows. Medians over the partials of a register, G, ff (rates in 1/s):

| register, band | σ_s | Re ε_V | Re Y (s/kg) | split (Hz) | \|ε\|/2π (Hz) | fastest mode | slowest | predicted early / late (dB/s) | sideways share at 0 / 0.3 s |
|---|---|---|---|---|---|---|---|---|---|
| C4–B4, 2.5–4 kHz | 1.33 | 0.62 | 4.2e-4 | 2.1 | 0.25 | 2.6 | 1.35 | −46 / −17 | −16 / −15 dB |
| C4–B4, 4–8 kHz | 2.26 | 0.67 | 4.8e-4 | 3.9 | 0.41 | 3.7 | 2.26 | −24 / −25 | −15 / −15 dB |
| C5–B5, 2.5–4 kHz | 1.58 | 1.25 | 4.7e-4 | 2.8 | 0.44 | 4.4 | 1.60 | −42 / −20 | −14 / −12 dB |
| C5–B5, 4–8 kHz | 2.62 | 1.29 | 5.1e-4 | 7.1 | 0.75 | 5.1 | 2.65 | −31 / −30 | −15 / −14 dB |
| C6–C7, 2.5–4 kHz | 2.00 | 1.72 | 3.8e-4 | 3.3 | 0.88 | 6.5 | 2.01 | −38 / −23 | −12 / −10 dB |
| C6–C7, 4–8 kHz | 3.08 | 2.83 | 5.6e-4 | 9.7 | 1.52 | 8.6 | 3.09 | −52 / −33 | −13 / −11 dB |

The prediction follows the renders' own measures within a few dB/s in most cells (C4–B4 2.5–4 kHz −46 against −42 measured, C5–B5 4–8 kHz −31 against −26): what the renders show at 0.02–0.32 s is the coupled modes, not the body's halo or the room.

- **The prompt sound needs strings closer than the coupling.** At the fundamentals the strings of a unison lie within |ε| of one another, and the in-phase mode takes 3·Re ε (the prompt sound) while the others keep σ_s (the aftersound). At 2.5–8 kHz they lie 2–10 Hz apart, 5–10 times |ε|/2π, as they must with one B (k times the fundamental's split): each string then decays on its own at σ_s + Re ε, a pair of near strings at about σ_s + 2·Re ε. The only slow motion left is the sideways one (σ_s + h·Re ε).
- **a. The bridge's conductance: confirmed.** Re Y at the keys' bridge points is 4–6e-4 s/kg from 2.5 to 8 kHz (the built board's level, docs/case_radiation.md). Re ε_V is then 0.6–2.8 s⁻¹ and the fastest mode 2.6–8.6 s⁻¹, −23 to −75 dB/s at best, diluted by the others in the line. The real first stage (C5–B5, 4–8 kHz: −68, then −27) asks the strings' vertical motion to lose about 8 s⁻¹ and the slow carrier 3 s⁻¹: Re ε_V ≈ 4–6 s⁻¹, Re Y ≈ 2e-3 s/kg at C5–B5. Uprights' bridges fall from 1–2·10³ to 200–300 kg/s above 2.5 kHz (Giordano 1998; Ege et al. 2013; docs/steinway_d_gaps.md, second-hand): 3–5e-3 s/kg. No grand's treble bridge is published above 3.2 kHz.
- **b. The sideways motion too present from the start: ruled out at 2.5–8 kHz.** It carries −12 to −16 dB of the output at the start and −10 to −15 dB at 0.3 s. It is the slow carrier, but it masks nothing: G's level at 0.3 s is −8 to −14 dB where the real grands' is −12 to −20, because its fast stage is too slow, not because the slow one is too strong. With the yield (below) the same sideways motion carries the late stage (−6 to −2 dB of the output at 0.3 s in C5–C7), which is what it should do. `polarization.radiation`, `slope` and `hammer_tilt` were left as they are.
- **c. One B for the three strings: real, not the cause.** With one B a unison's partials split exactly k times the fundamental's split. With each string's own B (`unison.b_spread`, below) the high partials split by amounts of their own, which mostly moves them further apart: it does not make a prompt sound. Its effect on the per-line early slopes is a reshuffling of the beats (±10–20 dB/s per register), and it scores better (below).
- **d. The measure.** The early slope over 0.3 s is sensitive to the beats: the five real pianos differ by about 25 dB/s (sd) at the median key, band-summed. The model's renders and the coupled-mode prediction agree, so no artefact of the measure was found. The early window does start at 20 ms, while the hammer's transient still fills the first window: both the renders and the recordings lose 8–16 dB of line power between 0–50 ms and 100–150 ms.

### The options

- **`soundboard.bridge_yield`** (bool, off), `yield_conductance` (s/kg, 1e-3 by default), `yield_corner` (Hz, 3000 by default). This is **a fitted law**, not a built part: it adds a conductance `yield_conductance`·x⁴/(1 + x⁴), x = f/`yield_corner`, to the admittance the strings see at the bridge. The sideways and cross couplings take their shares of it, as of the rest. It damps the strings and radiates nothing. The force reaching the board, and so the sound per unit of force, are left as they were: docs/case_radiation.md found the model's bridge too stiff above 1.5 kHz and its transfer too strong per unit of bridge motion, the ears right. A rise as x² (tried first) took too much at 1.2–2.5 kHz, where the treble's fundamentals and low partials already fall as fast as the real ones (C6–C7, 1.2–2.5 kHz: real −67 dB/s early, G −80). x⁴ leaves 1.5 kHz at 6 % of the top value.
- **`unison.b_spread`** (0 by default): the relative spread (rms) of B between the strings of a unison. Each string takes B·(1 + b_spread·u_j), u_j fixed per key and string with zero mean and unit rms (a hash of the key, like the detune pattern), the same at every build. Each string's first partial stays where the tuning puts it, as the tuner clears the low beats. A real unison's wires differ: once tuned, B ∝ d²/L⁴, so 0.5 % on the wire's diameter is 1 % on B. The bridge also crosses the strings obliquely, so the wires of one key are a few millimetres apart in length: 4·ΔL/L on B, 3–4 % at C5 and 10 % and more at C7. 2 % sits between. The front duplex's modes take each string's own B too.
- **`losses.q_te` 0.7e-4** (1.0e-4 in G): once the bridge takes the first stage, the late stage is carried by the sideways motion and by the strings' own loss. G's late rates at 4–8 kHz were 5–8 dB/s too fast (C5–B5 −34.5 against −26.5). In step 3, without the yield, 0.7e-4 had not scored better.

### Candidates and scores

The presets are in out/presets/p4:

- **H** (H_yield_bspread.toml): G + `bridge_yield` (2e-3 s/kg, 3 kHz) + `b_spread` 0.02 + `q_te` 0.7e-4.
- **H2** (H2_yield.toml): H without `b_spread`.

Trials, every key C4–C7 rendered, then `--score`:

| trial | 22 measures off / mean | what it showed |
|---|---|---|
| G + b_spread 0.02 | 685 / 1.110 | Spectral tilt 64 → 72: b_spread alone makes no first stage |
| G + yield x² 2e-3, 3 kHz | 745 / 1.160 | tilt 48, Brightness sustain 15, but Lines 1.2–2.5 kHz 80 off (bass −46.7 against −32) |
| the same + b_spread 0.02 | 737 / 1.150 | |
| G + yield x⁴ 2e-3, 3 kHz + b_spread 0.02 | 699 / 1.127 | tilt 36; late stage still 5–10 dB/s too fast |
| that + q_te 0.7e-4 (**H**) | 661 / 1.1025 | retained |
| H without b_spread (**H2**) | 692 / 1.118 | |

Scores (`--score`, band without bivib_grand, iowa, liang; measures version 9, with the two early-decay Lines measures of docs/compare_tool.md):

| | A today | C | G | H | H2 |
|---|---|---|---|---|---|
| 22 measures of version 8, off / mean | 760 / 1.156 | 669 / 1.116 | 693 / 1.118 | 661 / 1.103 | 692 / 1.118 |
| 13 older, off / mean | 376 / 0.953 | 396 / 0.981 | 443 / 1.012 | 395 / 0.977 | 403 / 0.976 |
| all 24, off / mean | 865 / 1.194 | 741 / 1.136 | 761 / 1.136 | 753 / 1.136 | 796 / 1.159 |
| Spectral tilt | 19 / 0.89 | 27 / 1.00 | 64 / 1.41 | 38 / 1.15 | 38 / 1.10 |
| Early decay | 50 / 1.11 | 51 / 1.14 | 53 / 1.10 | 45 / 1.08 | 48 / 1.08 |
| Brightness, sustain | 25 / 0.75 | 38 / 0.87 | 37 / 0.89 | 22 / 0.79 | 27 / 0.82 |
| Partial 1 ripple | 36 / 1.07 | 36 / 1.07 | 35 / 1.06 | 35 / 1.07 | 35 / 1.07 |
| T60 of partial 1 | 25 / 0.91 | 26 / 0.95 | 26 / 0.89 | 28 / 0.94 | 28 / 0.94 |
| Lines 2.5–4 kHz, early decay | 44 / 1.61 | 32 / 1.36 | 27 / 1.30 | 49 / 1.78 | 53 / 1.84 |
| Lines 4–7 kHz, early decay | 61 / 1.90 | 40 / 1.51 | 41 / 1.52 | 43 / 1.51 | 51 / 1.73 |
| Lines 4–7 kHz, early decay, treble median (band −57.9) | −87.5 | −56.0 | −25.5 | −51.3 | −43.5 |
| Lines 1.2–2.5 kHz, 1.2–2.2 s | 54 / 1.49 | 46 / 1.50 | 64 / 1.73 | 53 / 1.52 | 65 / 1.78 |
| Lines 2.5–4 kHz, 0.6–1.2 s | 70 / 1.83 | 34 / 1.33 | 41 / 1.40 | 47 / 1.51 | 58 / 1.62 |
| Lines 2.5–4 kHz, 1.2–2.2 s | 81 / 2.30 | 45 / 1.64 | 33 / 1.35 | 46 / 1.60 | 47 / 1.61 |
| Lines 4–7 kHz, 0.6–1.2 s | 55 / 2.38 | 35 / 1.92 | 24 / 1.40 | 24 / 1.50 | 28 / 1.58 |
| Lines 4–7 kHz, 1.2–2.2 s | 28 / 2.73 | 34 / 2.75 | 17 / 1.79 | 23 / 2.07 | 19 / 1.85 |

- **The yield gives the treble its first stage.** C5–B5 at 4–8 kHz goes from −26 to −56 dB/s early (real −68), and the late stage from −35 to −22 (real −27). "Spectral tilt" goes from 64 to 38 measures off, "Early decay" from 53 to 45, "Brightness, sustain" from 37 to 22.
- **b_spread** takes 31 measures off H2's 22, mostly in the Lines (1.2–2.5 kHz 65 → 53, 2.5–4 kHz at 0.6–1.2 s 58 → 47) and in the new early-decay measures (53 → 49, 51 → 43). "Partial 1 ripple" does not move (35 / 1.07 both), since partial 1 keeps its tuning.
- **On the 22 measures H is the best candidate so far** (661, against C 669 and G 693). On all 24, C leads (741 against H 753): the new 2.5–4 kHz early-decay measure finds H off at more keys than G (49 against 27). Its medians stay near the band (middle −29.9 against −31.3, treble −39.0 against −44.5), but they scatter more.
- **The top's fundamentals.** At C7–G7's fundamentals (2.1–3.1 kHz) the yield adds 4e-4 to 1.1e-3 s/kg, on a board whose conductance there was fitted on Plath's decays at the bridge (`edge_spring` 1500, 2026-11-17). At the microphones, partial 1's early slope of keys 96–103 (0.02–0.3 s) stays within the five real grands' range: H −53 to −118 dB/s, G −53 to −94, the real grands −39 to −132. The check at the bridge against Plath was not redone.

### What is not explained

- **The bass and the middle.** The yield acts on every key. There it lowers the late Lines: 2.5–4 kHz at 0.6–1.2 s, bass −51.9 against −49.4 for the band (G −47.6); 4–7 kHz at 1.2–2.2 s, middle −74.6 against −62.1 (G −66.9). C4–B4's early stage is still slower than the real grands' (−37 / −41 against −52 / −49). Whether a grand's bass and tenor bridge yields as its treble bridge does is not known. A yield that depends on the bridge point would be a further fit.
- **C5–B5 at 2.5–4 kHz.** The early stage there stays at −48 against −76: ranks 4–7, where the yield's x⁴ is still rising.
- **What the real yield is.** The law is a fit on the decays. The real grands' first stage is matched by a conductance about four times the built board's, half the uprights' published values, with nothing measured on a grand.
- **Listening.** Nothing has been listened to. The page out/ab_p4 has A, G, H and H2 on the rows of out/ab_p3, plus C5 and C6 held at mezzo and forte.
