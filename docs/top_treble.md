# Top treble: the knock of the truncated string

Status as of 2026-10-24.

## What the real pianos have

Against eight real grands ([bench.md](bench.md)), the top treble (C7–C8) was the worst register. Below the fundamental, a real top note has nearly as much energy at the attack as in its partial 1. Energy between 100 Hz and 0.9·f0, re partial 1, first 80 ms:

| | C6 | C7 | F#7 | C8 |
|---|---|---|---|---|
| Plath (anechoic) | −22.7 | −10.9 | −0.4 | −4.6 |
| Helgesson | −21.0 | −20.7 | −10.8 | +0.3 |
| Liang (Disklavier) | −27.1 | −11.0 | −13.6 | −3.3 |
| BiVib (Disklavier) | −25.4 | −2.8 | −9.8 | −6.5 |
| model | −25.2 | −27.9 | −24.2 | −14.5 |
| model, residual correction | −25.1 | −26.9 | −20.7 | −5.0 |

Plath's recording is anechoic, so the energy belongs to the instrument. Its peaks lie at 110–350 Hz and around 1 kHz. The keybed knock is not the source: ten times its force gains only 4–6 dB.

## The cause: a modal string with four modes

During the contact the hammer pushes the string down. For an ideal string pushed at x_h = (1 − β)·L from the bridge, the static deflection sends β of the hammer's force to the bridge and 1 − β to the agraffe. The hammer meanwhile sees a compliance of β(1 − β)·L/T.

The engine represents the string by its modes below 20 kHz. At C8 that is four. The modal sums of the end forces converge as 1/k, and with β = 0.07 the first four terms nearly cancel:
- **the bridge** gets 15 % of its quasi-static push (−16 dB): the knock of the top treble was lost;
- **the agraffe** gets 57 % of its push;
- **the hammer** sees the string twice as stiff as it is.

In the middle register, with 70 modes or more, the truncation costs nothing.

## The residual correction

The modes left out answer much faster than the contact lasts: they follow the force statically, q_k = φ_k·F/(m·ω_k²). `ContactSolver::strike` sums their static response from k_max + 1 to k_max + 400, felt width included:
- their compliance c is added to the string under the hammer;
- their share of the force goes to the bridge and to the agraffe, r_b·F and r_a·F.

A voice keeps its key, so the sums are cached and redone only when the instrument changes.

The solve lags this deflection by one step (ρ[n] = c·F[n−1]). The work done on it is the change of c·(F[n]² + F[n−1]²)/4 plus c·(F[n] − F[n−2])²/4 ≥ 0, so the scheme stays passive. It dissipates a few 10⁻⁵ of the energy over a contact.

## Results

| Scope | Before | After |
|---|---|---|
| bench score | 1.58 | 1.53 |
| bands 1 kHz (attack), z | −5.9 | −5.3 |
| bands 2 kHz (attack), z | −1.8 | −1.2 |
| bands 2 kHz (sustain), z | −3.3 | −2.7 |
| partial 3 at mf, z | −3.8 | −2.7 |
| sustain partial 2, z | −2.8 | −2.2 |

The bands and partial rows are measures of the top treble.

Four measures of the top move the other way by 0.5–1.3 z: partials 2–3 of the attack, the 4–8 kHz share at mf and the brightening. The velocity curve of the top gets closer to the Disklavier's: −33.4 dB at velocity 12 re 100, against −30.2 (−35.3 before). The CPU cost is unchanged within the audit's noise.

## Still open

- **The 1 kHz band of the sustain** (0.6–1 s): the real pianos keep 34–54 dB below the attack's strongest band, the model nothing (−101 dB). Something rings there in a real piano that the model lacks. Candidates: the frame, which is driven but radiates little (η 1 %), the duplex segments, or the action.
- **The decay of partials 1–4** is twice as long as on the real pianos: T60 0.6 s against 0.2–0.4 s, late decay 3.1 s against 0.6–1.4 s.
  - Raising the top strings' losses (`losses.top_slope` 0.05 to 0.14, from C6 or F#6) shortens the early decay (z +3.0 → +0.8). But it lowers the level at 0.3–1 s, already short (env 1 s z −2.1 → −2.7), and leaves the score at 1.51–1.55.
  - The late decay does not move: it is the slow coupled components of the unison, not the losses.
