# Phase 3 — dampers, pedals, sympathetic resonance

Status as of 2026-10-21. The dampers, the three pedals and sympathetic resonance are physically modelled. Routing through the soundboard required a major redesign.

## Dampers (`params::DamperLaw`, `voice.rs`)

A felt is a damper with coefficient c resting on the string at x_d = 0.14·L, over a width of 2 cm. It adds to partial k the decay rate:

```text
σ_k = c·⟨sin²(kπx/L)⟩_felt/(μL) · (2Z0/(c + 2Z0))²,   c = e·2Z0,   Z0 = √(Tμ)
```

- The last factor expresses the impedance mismatch: a felt that is too hard reflects instead of absorbing. Absorption is maximal for c = 2Z0.
- Averaging over the felt width prevents partials that have a node at the contact point from ringing indefinitely. This is a defect found by the sostenuto test.
- e ∈ [0, 1] is the felt contact. It follows its target with a time constant of 12 ms when lowering and 5 ms when lifting.
- Only keys up to E6 have a damper.
- Computationally, the modal bank recomputes only the decay of each mode: one exponential per mode, no trigonometry (`ModalBank::set_damping`).

Results (tests `tests/pedals.rs`, initial T60 of the string energy after release):

| Note | Initial T60 | Comment |
|---|---|---|
| E1 | 0.45 s | the bass dampers are less effective, without tuning |
| C4 | 0.12 s | |
| E7 | 0.86 s | no damper: does not change on release |

After this rapid drop, a slower tail remains: that of the partials barely touched by the felt, as on a real piano.

## Pedals (`engine.rs`)

- **Sustain (continuous CC64)**: the pedal travel sets the contact of all the dampers whose key is up, e = ((0.4 − travel)/0.4)^1.5. Half-pedalling is therefore physical. On C4, the T60 after release is 0.12 s with the pedal up, 0.22 s at half travel and 5.9 s with the pedal down.
- **Sostenuto (CC66)**: it memorises the keys held down at the moment it is pressed, and only those stay free.
- **Una corda (continuous CC67)**: the action shifts. The hammer leaves the first string, and the strike lands only on the others (`hit` parameter of the contact solver). It also strikes a softer area of felt: Q0 × (1 − 0.35). The unstruck string is excited only by the phase 2 bridge coupling. Result on C4: −2.8 dB.

## Sympathetic resonance

### Architecture: one-way, unconditionally stable

Since the strings are linear, their motion is the sum of two responses:
1. the response to the hammer (*primary* bank, whose own bridge coupling is already in the coupled modes);
2. the response to the bridge motion caused by the *other* keys (*secondary* bank, same modes).

Only the primary forces move the bridge for the strings. There is therefore no feedback loop: the system is stable for any gain, as in Bank, Zambon and Fontana (2010). Both responses radiate. Second-order resonance, where a sympathetically excited string excites a third one, is neglected.

**Single multi-input / multi-output board** (`board.rs`):
- A single bank of 1,083 modes with 16 inputs (forces per bridge zone) and 18 outputs (2 microphones, plus the bridge velocity at the 16 zone points).
- It replaces the 16 banks of phase 4.
- A second bank receives the sympathetic forces and feeds only the microphones.

**Excitation of a string through its moving end.** The wave equation with y(0, t) = u(t) gives the exact modal force −(μL/kπ)·ü. Per string and for a real acceleration input, the gain is i·2·dt/(kπ·ω_k).

This acceleration form is essential. The w_k·u form, proportional to displacement, is correct only at resonance. Off resonance, it makes the modes follow the slow motion of the soundboard, that is, its static deflection, and the sum over k produces an enormous spurious force.

**Self-subtraction.** The input of a key contains its own contribution, already counted in its coupled modes. It is removed by a second input, its own force multiplied by the rendered transfer H of the soundboard bank, evaluated at the exact frequency of each coupled mode. In the treble, this self loop is strong: without subtraction, the sympathetic response of E7 would exceed its direct vibration by 13 dB. With it, it stays 27 dB below.

**Delay and phase.** The soundboard is processed after the strings, so their input arrives with a delay. The engine works in sub-blocks of 64 samples, with circular buffers that make this delay *exactly* constant (64.5 samples, counting the finite difference that gives the acceleration). Each input gain is advanced by e^{iω·D}: at the mode frequency, the coupling keeps its physical phase.

**Listening strings.**
- When the pedal lifts the dampers, each key receives a “listening” string. These strings are created 8 per sub-block, to smooth the cost.
- They run only their partials below 5 kHz.
- Keys without a damper listen permanently.

Result (pedal down, short C3): sympathetic energy 3.0 for the octave C4, 1.7·10⁻² for the twelfth G4, 1.5·10⁻⁴ for the semitone C#4. In the “halo” demo, we measure −45 dB after the chord without pedal, versus −28 dB with.

### Mistakes found along the way

1. **Delayed closed loop.** The first version (the strings feed back into the soundboard that excites them) diverged with any two listening strings. A one-block delay rotates the phase of the coupling, and dissipation becomes amplification. Hence the one-way design.
2. **Passivity constraint.** The cross-polarisation coupling tuned in phase 2 (0.5·ε with a horizontal coupling of 0.1·ε) violated ε_X² ≤ ε_V·ε_H: the bridge was no longer passive. With the modal soundboard, some modes then had *negative* damping. Fix: horizontal coupling 0.3, cross coupling 0.5 (second-best setting from phase 2, nearly identical score), with automatic clipping (`passive_cross`). A test checks that all modes of the complete instrument decay.
3. **Soundboard discretisation.** Impulse invariance counts in full, at the first sample, the velocity jump that an impact imposes on each mode. This added a spurious constant per mode, and over 1,000 modes the rendered conductance at 80 Hz was 20 times too high. A direct correction term is added, in the rendering as in the transfer H. A test now checks that the rendered mobility stays within 8 % of the continuous mobility.
4. **Performance.** The multi-output kernel was about 30 times too slow. Splitting it into two passes (recursion with history in L1, then streamed outputs) was not enough: the AVX-512 code generated by LLVM for this kernel remains about 10 times slower than the AVX2 code on Zen 5. The soundboard therefore runs in AVX2.

## Cost

Worst case: whole keyboard at velocity 127, one note every 30 ms, pedal down, 88 primary and secondary voices, two soundboard banks. We measure **4.65× real time, i.e. 22 % of a core**, of which about 9 % is sympathy. Building the instrument takes 0.8 s, including the self transfer computed mode by mode.

## What remains open

- The absolute level of sympathetic resonance is not matched to measurements. Salamander's “release resonance” samples mix dampers and sympathy. The one-way design neglects the back-reaction: it slightly overestimates the response of very strongly coupled strings, those of the treble.
- No damper or pedal noises: that is phase 5, mimetic.
- The half-pedal law (contact point 0.4, exponent 1.5) is generic.

## Demos and commands

```bash
python3 -I tools/make_demo_midi.py out/demos
cargo run --release -p piano-render -- midi out/demos/pedal_halo.mid out/demos/pedal_halo.wav
cargo test -p piano-core --test pedals -- --nocapture
cargo run --release -p piano-core --example stress_cost
cargo run --release -p piano-core --example sympathy_debug
```

Rendered demos: `pedal_halo` (same chord without then with pedal), `half_pedal` (releases with pedal up, at half travel, down), `una_corda` (normal chord then una corda), `sostenuto` (C2 held under a damped melody).
