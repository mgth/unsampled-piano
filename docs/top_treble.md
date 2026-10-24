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

## The decay

The bench's T60 suggested that the top partials last twice as long as on the real pianos. Their envelopes, filtered ±1.5 % around partial 1, tell another story. At C7:

| | 0 to 0.4 s | 0.4 to 2 s |
|---|---|---|
| real grands (Plath, Helgesson, Liang, Iowa) | −60 to −100 dB/s | −11 to −21 dB/s |
| model | −48 dB/s | −30 dB/s |

A real top note has two decays: a fast prompt sound, then a slow aftersound. The model had one decay, in between. Two settings made the aftersound short:
- **The top strings' extra losses** (`losses.top_slope` 0.05 from C6, ×1.8 at C7) had been set against a commercial modelled piano's release ring.
- **The bridge impedance** grew by ×4.6 towards C8 (`soundboard.impedance_slope` 0.035, from Salamander's envelopes alone). This held back the prompt sound, which the top strings lose into the bridge.

Envelopes of partials 1–2 at F#5, C6, C7, F#7 and C8, ff, against the median of eight real grands, 0.1–2 s (RMS gap, levels below −70 dB left out):

| top_slope / impedance_slope | 0.05 / 0.035 | 0.025 / 0.035 | 0 / 0.035 | 0.025 / 0.02 | 0 / 0.02 | 0 / 0 |
|---|---|---|---|---|---|---|
| RMS gap (dB) | 17.7 | 14.0 | 13.0 | 15.0 | 13.8 | 15.7 |
| bench score | 1.53 | — | 1.55 | — | 1.51 | — |

The default is now top_slope 0 and impedance_slope 0.02 (×2.4 at C8). On the bench: 77 % of the measures within the real pianos (70 %), 16 off (21). Fifteen measures improve by 0.5 z or more, three get worse.
- **Better**:
  - the top's 2–4 kHz bands in the sustain (z −2.7 → −0.9, −2.2 → −0.1);
  - its level at 1 s (−2.1 → −0.9) and its partial 2;
  - the treble's level at 0.3 s and its early T60 (+2.0 → +1.0).
- **Worse**:
  - the top's partial 4 of the sustain (−5.4 → −6.5);
  - its level at 0.3 s (+0.1 → +0.7);
  - the late T60 of the bench's survey (+4.1 → +4.8).

The survey fits its early decay on the first 15 dB. In the real top notes that drop includes the knock, which is as strong as partial 1 and dies in a few tens of milliseconds, so the measure says little about the strings there.

Presets that set neither key keep the new values. The local presets made before were frozen with the old ones.

## The upper partials of the sustain

In the top's sustain, partials 3–4 sat 20–40 dB below the real grands' (z −3.3 and −6.2), and the treble's partials 6–12 15–30 dB (z down to −6.2). The strings' frequency term b2·f² of σ (1e-7, set against Salamander) takes 6.4 s⁻¹ at 8 kHz, −56 dB/s.

| b2 | 1e-7 | 7e-8 | 6e-8 | 5e-8 | 4e-8 | 2.5e-8 |
|---|---|---|---|---|---|---|
| bench score | 1.52 | 1.42 | 1.31 | 1.40 | 1.42 | 1.47 |
| without the top's 1 kHz sustain band | 1.32 | 1.22 | 1.16 | 1.18 | 1.21 | — |
| within the real pianos | 71 % | 76 % | 76 % | 73 % | 73 % | — |

The top's 1 kHz sustain band (z −8 to −10, the model has nothing there) swings the score by itself, so the second row is the one to read. The default is 6e-8, which improves every register:

| register | bass | middle | treble |
|---|---|---|---|
| 1e-7 | 0.70 | 0.77 | 1.44 |
| 6e-8 | 0.64 | 0.62 | 1.27 |

- **Better**:
  - the top's sustain partials 3–4;
  - the treble's sustain partials 6–8;
  - the middle's and bass's 4–8 kHz sustain bands;
  - the middle's T60 of partials 5–12.
- **Worse**:
  - the treble's and top's early T60 of partials 5–12 (z up to +2.0 and +3.1), which now last too long;
  - the top's prompt sound, partial 1, which decays at −39 dB/s against −45 (−42 before).

## Still open

- **The 1 kHz band of the sustain** (0.6–1 s): the real pianos keep 34–54 dB below the attack's strongest band, the model nothing (−101 dB). Something rings there in a real piano that the model lacks. Candidates: the frame, which is driven but radiates little (η 1 %), the duplex segments, or the action.
- **The prompt sound.** The model's first 0.2 s still lose 5–10 dB less than the real ones at C7–C8.
