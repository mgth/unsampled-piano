# The shape of the strike: a contact per string

Status as of 2026-10-27.

## What was heard

"Repeated C6s at around velocity 45–55 have an unpleasant plop at the attack; softer and louder they don't." The flight recording of the session held C6 struck some sixty times between velocities 1 and 90. Rendered again, the attack's partial 2 fell from −9 dB re partial 1 at velocity 30 to −23 at velocity 52, then rose again to −8 at 85; partial 4 vanished at the same velocities. With little more than its fundamental and partial 3, the attack sounded hollow.

The real grand does not do that. The 16 velocity layers of Salamander's C6 (a Yamaha C5) give partial 2 a steady rise, from −26 dB at the softest layers to −7 at the loudest.

## Why

The partials a strike leaves on a string are the spectrum of its force pulse at their frequencies. A smooth, symmetric pulse has zeros in its spectrum, and they move up as a harder blow shortens the pulse. When one lands on a partial, that partial is not excited. At C6 the pulse lasted 1.7 ms at velocity 30, 1.4 at 52 and 1.1 at 85, with its first zero passing partial 2 around velocity 50.

Over C5–C7, the largest fall of each of partials 2–4 below its level at softer blows (the dips) averaged 10.2 dB, and reached 15–25 dB at C6, F#6 and C7. Salamander's average is 4.9 dB.

The engine's felt pressed on the unison's mean displacement. A string lying lower than the highest one took no force until the hammer had travelled to it, then took its full share at once. The pulse had a kink there, and kept its zeros.

Changing the hysteresis, the strings' height spread or the shank's flexibility alone moved the dips without filling them (mean 15–20 dB).

## A contact per string

Each string of a unison now presses its own part of the felt (`hammer.rs`):

- string s takes σ_s of the hammer's stiffness, its share of the blow (the `hit` of its oscillators, uneven felt included);
- its compression is u_s = x − d_s − Σφq over its own modes, where d_s is how much lower it lies than the highest string (`unison.height_spread` × its height);
- each contact has its own quadratised felt energy ψ_s, and the hammer takes the sum of the forces.

The step stays explicit. The forces solve F_s·(1 + a_s·Λ_s) + a_s·μ·ΣF = b_s in closed form, and a string whose force would pull is left out and the rest solved again. The scheme stays passive.

Strings level with one another give the single felt exactly: they move as one string three times as heavy and as tense (test `level_strings_make_the_single_felt`). A lower string meets the felt later and is compressed less, so it takes a shorter and weaker pulse with no kink.

The CPU load is unchanged within the measurement noise (`perf_audit`, alternated runs).

## Calibration

The bench against the eight real grands, close microphones (score: RMS of z, lower is better):

| | single felt, hysteresis 0.3 | contacts, 0.3 | contacts, height spread 0.05 mm | contacts, hysteresis 0.6 |
|---|---|---|---|---|
| score | 1.07 | 1.11 | 1.07 | **1.03** |
| attack | 1.05 | 0.98 | 0.95 | 0.99 |
| sustain | 1.56 | 1.72 | 1.62 | 1.47 |
| decay | 0.97 | 1.01 | 0.96 | 0.91 |
| mean dip, C5–C7, partials 2–4 | 10.2 dB | — | 5.8 dB | 5.2 dB |

The default hysteresis is now 0.6, as it was before [middle_highs.md](middle_highs.md) chose 0.3 for the single felt. A larger hit spread (0.3–0.4) or height spread (0.15–0.3 mm) did not do better.

Against the single felt:
- **Better** (18 measures by 0.5 z or more):
  - the treble's and top's mf attack partials 3, 4 and 6 (top partial 4: z +1.8 → −0.2);
  - the early T60 of partials 5–12 (top +3.7 → +0.5, treble +1.6 → +0.4);
  - the middle's mf 2–8 kHz share;
  - the treble's sustain partial 8.
- **Worse** (15 measures):
  - the top's mf partial 2, now too strong (+1.3 → +3.1);
  - the treble's sustain partials 3–4 and 12 (+0.6 to +1.6);
  - the middle's and treble's late envelopes at pp and mf, 1–2 dB low (z −0.9, −1.9).

With the user's sound (concert microphones), C6's partial 2 now dips 4.6 dB around velocity 48 instead of 13.6, and partials 3–5 no longer dip at all (2 dB instead of 12–13).

## Tools

- `cargo run --release -p piano-host --example strike_sweep -- preset.toml <key> [velocities…]`: the contact over the velocities, or the force of each string, sample by sample, and its spectral zeros.
- `engine::strike_force` and `strike_forces`: the force of one strike, in total or string by string.
- Tests in `hammer.rs`: level strings make the single felt, energy never grows with the strings apart, and the dips at C6, F#6 and C7 stay under a bound.

## Still open

- C7 keeps 13 dB of dips (Salamander: 8).
- The C6 octave is still dull: partial 2 at mf is about 15 dB under Salamander's. This is the treble's spectral tilt (C5 too bright, C6 too dull), not the dips.
