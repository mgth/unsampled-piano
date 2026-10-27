# Evenness from key to key

Status as of 2026-10-25.

## Listening report

At the end of the Chopin op. 10 no. 3 A/B (Liang's Disklavier): "a rather strong high resonance; the original has it too, but much more muffled".

## What it was

Not the sympathetic strings and not the action's knock: with either turned off, the same lines stand out. Over 55.5–57.5 s, three notes ring 7–12 dB louder than on the recording:
- A3's fundamental (220 Hz);
- C#4's partial 2 (554 Hz);
- F#4's partial 2 (740 Hz).

G#3's fundamental, on Liang's isolated notes, is 13 dB weaker than recorded. The model's soundboard favours some notes and starves others.

## The measure

`evenness` (tools/bench.py): for each bench key from C2 to C7, the level of partials 1 and 2 at 0.5–1.5 s (ff) minus the median of the keys within ±6 semitones; the rms of these residuals. One figure, filed under the middle.

| | partial 1 | partial 2 |
|---|---|---|
| real grands, mean | 6.4 dB | 4.7 dB |
| real grands, range | 4.6–8.4 dB | 3.0–5.9 dB |
| model | 9.4 dB | 7.0 dB |

Taken apart:
- **At the attack** (0.02–0.12 s), the model is as even as the real grands: partials 1/2 at 6.1/6.9 dB against 2.3–7.9/3.0–7.5.
- **The decays** (0.45 → 1.5 s) vary less from key to key than on the real grands: 5.4/4.4 dB against 4.0–8.9/4.1–6.0.
- **In the sustain** the two add up: 9.7 dB on partial 1, at the top of the real range (5.1–9.1; Liang's Disklavier 8.1).

## What was tried

| | evenness p1 / p2 | bench (1.10) |
|---|---|---|
| board loss factor 0.02 → 0.03 | 9.1 / 6.3 | 1.12 |
| bridge drive spread 0.3 → 0.6 / 1.0 | 8.4 / 6.6, 7.9 / 6.5 | 1.15, 1.28 |
| board loss corner 2.5 kHz → 1 kHz / 500 Hz | 9.6 / 6.6, 8.9 / 6.1 | 1.13, 1.23 |
| 1.5× / 2× the modal density at equal mean mobility | 7.5 / 7.4, 9.1 / 5.7 | 1.41, 1.75 |
| other microphone set-ups | 9.5–12.8 / 7.4–8.2 | 1.24–1.43 |

None was kept. A more realistic board (ribs, an irregular outline, two bridges) would be the next step. The outline is now in the model, off by default: on every key it gains 0.7 dB on partial 1 and nothing on partial 2 (docs/outline.md).

## Key to key while playing

Listening report (2026-10-25): "brutal changes from one key to the next, especially between B and C in the middle of the keyboard".

### The measure

The attack level (0–100 ms, both channels' power) of every key from C3 to C6, and its jumps from one key to the next. The real grands heard where the pianist sits (Plath's dummy head in an anechoic room, BiVib's binaural recordings) show 3–4 jumps above 5 dB, the largest about 6 dB. Close microphones make any piano less even: the Helgesson set reaches 11 dB. Scratch tools: `examples/board_evenness.rs` (the board alone, no rendering) and `examples/board_basis.rs`.

### What it was

- **The bridge regions.** Each key drove the board at the centre of its region, one region per 5.5 keys. Region ends fall between B3 and C4 and between E4 and F4. Driving the board at each key's own point (an 88-region test build, 2.2× the CPU) brought the player's position to 3 jumps above 5 dB, the largest 6.2 dB, as on the real grands.
- **The board's resonances.** Below 400 Hz they are narrower than their spacing, so two neighbouring keys meet unrelated peaks and dips. Irregular mode shapes do not help: mixing neighbouring modes' shapes at random made it worse.

### What changed

- **The damped strings load the board** (`soundboard.string_load`, 1). Each string resting under its damper absorbs what reaches it from the bridge: averaged over frequency, a lossy string's input resistance is its characteristic impedance √(T·μ). Mode n gains a damping rate ½·Σ√(T·μ)·Φ_n² over the damped keys. Its loss factor goes from 0.021 to 0.069 at 100–200 Hz and from 0.022 to 0.043 at 200–400 Hz, and stays nearly the same above 800 Hz. The pedal lifts the dampers and the load with them: the board's poles follow the dampers' engagement (`BoardBank::set_load`). The board sounds ≈ 2 dB quieter without pedal, 1.3 dB with it.
- **Low band shared between two regions.** Below 400 Hz (`board::SHARE_HZ`), a key's forces go to the two regions around it with linear weights. Above, they go to its own region. Neighbouring regions' drive shapes correlate by 0.9 at 100–200 Hz, by 0.6–0.7 at 200–400 Hz and by 0.2 at 400–800 Hz in the middle (`examples/region_coherence.rs`). Sharing at all frequencies lost ≈ 3 dB of the treble's upper partials, because the two regions no longer add coherently.
- **Exact drive of the low modes, optional** (`soundboard.exact_below`, Hz; 0 by default). The board modes below it run in a bank of their own. The keys' drive shapes of the modes below 1 kHz span few dimensions: their 16 principal profiles keep 99.98 % of them. The low bank's 16 inputs are these profiles (`board::LowDrive`, Jacobi decomposition of their Gram matrix at build time), and each key feeds them with its own weights. Its shapes are then rebuilt within 1–1.5 %, against 19–49 % for its region's. The sympathetic forces keep the regions: they form a diffuse halo, and the exact drive there doubled the cost for nothing audible.

Attack jumps, keys C3–C6, velocity 64:

| | rms | largest | above 5 dB |
|---|---|---|---|
| player's position (`presets/listening/Microphones/player.toml`), before | 3.6 dB | 6.7 dB | 7 |
| same, exact drive below 1 kHz | 3.2 dB | 6.2 dB | 2 |
| engine's own set-up, before | 2.9 dB | 8.6 dB | 4 |
| same, exact drive | 2.7 dB | 6.8 dB | 4 |
| close microphones 40 cm above the board, string load and sharing | 3.2 dB | 8.7 dB | 5 |
| same, exact drive | 3.1 dB | 8.3 dB | 5 |

On a chromatic scale E3–D5 with close microphones, the string load and the sharing take the jumps from 4.2 to 3.3 dB rms, and the largest (E4 to F4) from 12.4 to 8.9 dB. With these microphones F4's fundamental still lies in a dip of the board's answer, even with the exact drive.

Bench, measured on the same dynamic range for the model and the references (docs/bench.md): score 1.02 → 1.05 with the string load and the sharing, 6 measures off instead of 8. The middle's evenness of partial 1 drops from 9.6 to 7.7 dB (real 6.4, range 4.6–8.4), and the bass envelopes come closer (−11 dB at 1 s against −13, real −9). The middle's partials 4–12 sustain 3–4 dB too long. With the exact drive the score is 1.08: partial 2's evenness drops from 6.5 to 5.8 dB (real 4.8), but the middle's attack loses ≈ 1 dB against the bass and the treble. With eight bench keys per register, part of that may be where the dips happen to fall.

Cost: about 5 % of the Chopin for the sharing (each key feeds two regions), and 4 % more for the exact drive. Building the instrument takes 0.07 s more.

### Limits

- The strings read the bridge's motion at their region (sympathy), even with the exact drive.
- The keys held down lift their own dampers, but their strings' load stays.
- With the pedal down, the board's modes lose the load, but the strings' coupling, computed at build time, keeps it.
