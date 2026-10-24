# The action's knock

Status as of 2026-10-24.

## Listening report

In A/B against MAESTRO performances (Rachmaninov's Lilacs, Schubert): "the high notes are more percussive in the real recordings".

## What the real pianos have

Below the fundamental of the top keys there is no string partial, only what the rest of the instrument does at the attack. On eight real grands (tools/bench.py's references), the first 60 ms below 0.75·f0 hold a knock:

- **Level:** at about the same level at every key from C4 to C8: 22–30 dB under a middle note's attack (C4–A4, all bands) in the 63 and 125 Hz octaves. From C5 to C8 the median is −23 to −27 dB at 125–500 Hz.
- **Spectrum:** −19 dB around 1 kHz, about −28 dB at 2–2.8 kHz, and next to nothing at 4–8 kHz: the real mf attacks have a 2–4 kHz share of −40 dB in the bass and −29 in the middle and treble.
- **Envelope:** it peaks 6–18 ms after the note's onset and loses 10–15 dB within 50 ms.
- **Velocity:** relative to a middle note at the same layer, about the same at pp, mf and ff.

The model had it 20 dB lower: its only knock was the key bottoming on the keybed (`noise.keybed`), as a force into the board. To reach the real level that way would take some 130 N, not a physical figure. The knock is mostly heard directly: the key on the keybed, the hammer shank and the wippen, the keyframe, radiating into the air near the microphones.

## The model

`crate::action`:
- **The resonances:** a bank of 24 damped resonances from 80 Hz to 2.6 kHz, shared by all keys: the keyframe and the action's wooden parts.
- **The strikes:** each key strikes the bank with its own fixed amplitudes (±4 dB per resonance, random signs). It does so `noise.action_delay` (8 ms) after the hammer reaches the string, with a peak pressure of `noise.action` (0.05 Pa) per m/s of hammer speed at the microphones.
- **The spectrum:** flat to 500 Hz, +6 dB at 1–1.5 kHz, falling at 36 dB per octave above.
- **The decay:** T60 `noise.action_time` (0.2 s) at 500 Hz, shorter above as (500/f)^0.3.
- **The microphones:** the left one hears the bass keys louder, the right one the treble keys. The signal joins the board's at the microphones, before the room.
- **The onset:** a strike is spread over a 0.6 ms Hann pulse; a one-sample impulse clicked.

Below the top keys' fundamentals, at ff, the model now has −25/−25/−27/−20/−21 dB in the 125/250/500/1000/2000 Hz octaves. The real median is −26/−26/−26/−19/−20 (before: −56/−52/−44/−38/−42).

## Bench

The score goes from 1.06 to 1.10, and the dynamics improve (z rms 0.43 → 0.32). What gets worse is the top's attack: partials 2–3 at mf, its 2–8 kHz shares, its level at 0.3 s against its peak. The top's strings are 10–20 dB too weak against the real ones, so the knock, at the real level, dominates its attack. It gave a percussive edge to the treble in the A/B on the MAESTRO performances.

The first spectrum tried (up to 6 kHz, +7 dB at 1–2.5 kHz) raised the 2–8 kHz shares of every register (bench 1.17). The real knocks stop around 2.5 kHz.
