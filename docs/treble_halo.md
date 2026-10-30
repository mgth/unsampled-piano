# The treble's halo and the free strings' brake

Status as of 2026-10-30: the default since this change (`sympathetic.gain` 1, `sympathetic.brake` on; before, 0.6 without the brake).

The bench's `resonance` family (docs/bench.md) and the top's 1–2 kHz bands measure the same fault: under the top octave's notes the model is 10–20 dB cleaner than every real grand. That sound is the halo of the undamped strings (D#6 and above), rung by the struck note. This page finds where the model loses it, against Plath's concert grand, whose bridge acceleration was recorded with every note, and adds the coupling the strings were missing.

## Where the halo is lost

Tools: the model's bridge velocity at the key's region and its microphones, `cargo run --release -p piano-render --example bridge_signal -- presets/default.toml <prefix> --note N --velocity 100`; Plath's PROD07 take M0000 (bridge accelerometer at the hitch pins, dummy head at the player). Plath's piano is played at f; the model at velocity 100. The two bridge signals land within 1–2 dB of each other at the fundamental (C7: 40.4 against 39.9 dB re 1 m/s², first millisecond), so the levels compare directly.

**The blow at the bridge is right.** The 1–2 kHz band of the bridge acceleration, by millisecond after the onset (dB re 1 m/s²):

| | 0–1 ms | 1–2 | 2–4 | 4–8 | 8–16 |
|---|---|---|---|---|---|
| C7, Plath | 30.5 | 24.2 | 16.6 | 14.6 | 8.4 |
| C7, model | 30.3 | 21.3 | 16.0 | 17.0 | 11.1 |
| D#7, Plath | 23.8 | 16.1 | 8.2 | 9.4 | 3.3 |
| D#7, model | 25.7 | 16.1 | 9.6 | 9.3 | 5.3 |

The real pulse comes with the hammer's contact, 1–4 ms before the key reaches the keybed (Plath's force sensor), and the model's quasi-static push and string modes give the same. The capo bar is no path either: the model's frame is 32–34 dB less mobile than the board at 1.5–2 kHz (`frame_drain`, velocity per force), so even the 7 times larger force on the agraffes of the top (strike point at 0.12 of the length) adds nothing.

**The strings answer too weakly, then fade too fast.** The lines of A6, A#6 and B6 under C7 (dB re C7's fundamental at the ears):

| | 20–120 ms | 120–300 ms | 0.3–0.7 s | 0.7–1.5 s |
|---|---|---|---|---|
| Plath | −28.0 −28.8 −22.2 | −21.1 −21.1 −22.8 | −16.0 −16.9 −16.0 | −19.8 −23.7 −17.2 |
| model, `sympathetic.gain` 0.6 | −34.1 −33.9 −34.8 | −44.5 −37.8 −39.9 | −40.2 −34.8 −38.2 | −28.3 −27.8 −33.3 |
| model, gain 1 | −29.9 −29.7 −30.6 | −40.3 −33.7 −35.7 | −36.0 −30.5 −33.9 | −24.1 −23.6 −29.1 |

With the physical coupling (gain 1) the halo starts where Plath's does. Then the model's lines fade 4 times faster; so do its own struck top notes (A#6's fundamental loses 16 dB from the first window to the second, Plath's 8). That is the unisons' first, fast decay, and the eight bench grands spread it from −1 to −19 dB at these keys (beats), the model on the fast edge: a separate question, not the sympathy's.

Plath's undamped unisons are badly out of tune (A6's three strings at +20, +36 and +41 cents, each ringing apart); the eight grands, read the same way at 0.3–1.8 s, spread their C6–A6 unisons over 0–6 cents, the model's over 0–3. Widening the model's treble unisons (`unison.width_curve` 2) lifts the halo 5–10 dB after 0.3 s but costs the treble's own sustain (bench 1.114, a new red) and changes the tails the user had settled by ear (docs/treble_tail.md): not kept.

## Why the gain was 0.6

`sympathetic.gain` 1 is the one-way physical coupling. It was lowered to 0.6 (docs/pedal_drone.md) because with the pedal down the middle's halo came out 3.9 dB over Liang's Disklavier and B5 too marked in Chopin op. 10 no. 3: each free string rang as if it were alone, while real strings ringing at one frequency hold the bridge back there. The uniform 0.6 paid for it in the undamped top, where no other free string shares the struck note's frequencies.

## The brake

`sympathetic.brake` (on by default) adds that braking to first order. With the strings' reaction on the bridge, the bridge's acceleration at a frequency is the one-way acceleration divided by 1 − Σ_k A·H_k: H_k a free key's sympathetic force per bridge acceleration (its own listening bank's transfer, so the same coupling the engine runs), A the board's acceleration at the listening key's region per force at key k's region (the transfer the engine uses for the key's own contribution). Per key and listening mode, the sum takes the other free keys' modes within 2 % of its frequency (the farther ones answer off resonance, and their mean load is already in the board, `string_load`), the free strings taken to move with the bridge at the listening key's region. The brake multiplies the mode's two input gains (the bridge and the cancellation of its own contribution), capped at 1.

Two sets per key: the undamped keys alone (dampers down) and every key (dampers lifted); the voices switch when the pedal crosses the listening strings' threshold, keeping their motion. Built once with the instrument (0.15 s for the 88 keys). `cargo run --release -p piano-core --example sym_brake -- [key …]` prints them; with the pedal up the halo's strings, D#6–B6, are not braked at all (an undamped D7 partial is, by 1.9 dB, by another undamped string's); with the pedal down A2's partials 1–4 lose 3.5–4.5 dB, E4's partials 3–4 3.2 dB, the top's fundamentals 0.6–2 dB.

## Results

**Bench** (pedal up, the same renders' protocol as the baseline):

| | score | off | bass | middle | treble | top |
|---|---|---|---|---|---|---|
| gain 0.6 (baseline) | 1.143 | 11 | 0.85 | 0.94 | 0.92 | 1.76 |
| gain 1 | 1.069 | 11 | 0.84 | 0.95 | 0.90 | 1.54 |
| gain 1, brake | 1.066 | 11 | 0.84 | 0.94 | 0.90 | 1.53 |

Every top measure of the halo moves 0.6–1 z: bands at the attack 1k −4.52 → −3.94 and 2k −4.49 → −3.78, sustain 1k −3.55 → −2.80 and 2k −2.79 → −2.09, `between` −4.12 → −3.14, `lines` −4.35 → −3.39; treble `between` −2.58 → −2.35. The largest other move: the top's late T60 of partials 1–4, +0.53 → +1.42. The halo stays 3 z short: the strings' first decay above is the rest of it.

**Pedal** (`tools/pedal_bench.py`, Liang's isolated notes with and without the pedal; halo the pedal adds, CC64 96, dB):

| register | Liang | gain 0.6 | gain 1 | gain 1, brake |
|---|---|---|---|---|
| bass | +5.3 | +3.5 | +3.9 | +1.4 |
| middle | +14.5 | +16.7 | +17.9 | **+14.7** |
| treble | +12.1 | +22.2 | +20.1 | +20.4 |
| top | −2.7 | +6.5 | +5.5 | +5.3 |

The brake brings the middle's pedalled halo onto the Disklavier's (it was 2.3 dB over at 0.6, 3.4 at 1 without it), and costs the bass 2.5 dB of halo. The treble's and the top's pedalled halo stay 7–10 dB over whatever the gain or the brake: unexplained since docs/pedal_drone.md, and not the coupling's strength. The middle's "hold" (partials 1–4 held by the pedal, 1.3–1.8 s) goes from −2.1 to −3.9 dB against Liang.

## Listening

Blind A/B in `out/ab_halo/` (not in the repository): the top notes C6–G#7 at velocity 96 without the pedal, one clip per note, and the first 40 s of Chopin op. 10 no. 3 with the pedal in five 8 s phrases (Liang's Disklavier; the model with close microphones in a studio; all at −20 LUFS), each clip as the recording, then the two settings in a random order: before (`gain` 0.6, no brake) and after (`gain` 1, brake), nothing else changed. A first version played the whole passages; too long to compare by ear, it was cut note by note. The user preferred "after". The bench's baseline is saved on it (1.066, 11 off, 344 OFF / 58 WATCH profiles).

## Still open

- The top strings' first decay: the model's top notes lose their fundamental faster in the first 0.3 s than Plath's (and than most of the eight grands), and so does their halo; the halo stays 3 z short on the bench.
- The treble's and the top's pedalled halo, 7–10 dB over the Disklavier whatever the coupling.
