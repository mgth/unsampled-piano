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
- **The strikes:** each key strikes the bank with its own fixed amplitudes (±4 dB per resonance, random signs). It does so `noise.action_delay` (8 ms) after the hammer reaches the string, with a peak pressure of `noise.action` per m/s of hammer speed at the microphones: 0.05 Pa matches the real grands (the default is 7.5 dB below it, below).
- **The spectrum:** flat to 500 Hz, +6 dB at 1–1.5 kHz, falling at 36 dB per octave above.
- **The decay:** T60 `noise.action_time` (0.2 s) at 500 Hz, shorter above as (500/f)^0.3.
- **The microphones:** each key's knock is louder on the side where its strings are heard: the treble on the left, the bass on the right. The signal joins the board's at the microphones, before the room.
- **The onset:** a strike is spread over a 0.6 ms Hann pulse; a one-sample impulse clicked.

Below the top keys' fundamentals, at ff, the model now has −25/−25/−27/−20/−21 dB in the 125/250/500/1000/2000 Hz octaves. The real median is −26/−26/−26/−19/−20 (before: −56/−52/−44/−38/−42).

## Bench

The score goes from 1.06 to 1.10, and the dynamics improve (z rms 0.43 → 0.32). What gets worse is the top's attack: partials 2–3 at mf, its 2–8 kHz shares, its level at 0.3 s against its peak. The top's strings are 10–20 dB too weak against the real ones, so the knock, at the real level, dominates its attack. It gave a percussive edge to the treble in the A/B on the MAESTRO performances.

The first spectrum tried (up to 6 kHz, +7 dB at 1–2.5 kHz) raised the 2–8 kHz shares of every register (bench 1.17). The real knocks stop around 2.5 kHz.

## Default level

Played live, the mechanical noises stood out too much (2026-10-27): the knock's level matched the real grands' recordings, close microphones in a studio, more than what the player hears. The defaults of every noise the musician's control `mechanism` moves (`noise.action`, `keybed`, `damper_mass`, `pedal`) are now 7.5 dB below these levels (× 0.42). The control spans × 8 each way (±18 dB): +0.42 brings back the real grands' level, +1 is 10.5 dB above it, −1 25.5 dB below. A first try at −12 dB left the control's lower half nearly inaudible.

## Against the commercial piano (2026-11-01)

The commercial piano has a `Hammer Noise` setting, 0.286 in its D. On the top keys, below 0.75·f0, where no string partial sounds, the knock can be read on its own. In dB against each set's own C4–A4 ff attack, mean of the top keys at ff, by time from the onset:

| | 0 s | 20 ms | 50 ms | 100 ms | 200 ms | 300 ms | 500 ms |
|---|---|---|---|---|---|---|---|
| Six real grands (iowa, salamander, headroom, splendid, helgesson, bivib_grand), range | −13…−8 | −18…−14 | −25…−18 | −33…−26 | −42…−35 | −48…−42 | −55…−48 |
| Commercial, its D | −8 | −12 | −17 | −26 | −38 | −45 | −54 |
| Commercial, `Hammer Noise` 0 | −20 | −24 | −29 | −36 | −47 | −57 | −67 |
| Commercial, `Hammer Noise` 1 | +1 | −3 | −8 | −17 | −28 | −35 | −45 |
| Model, default | −18 | −26 | −36 | −48 | −63 | −68 | −74 |

The real grands' readings at 500 ms and beyond may hold some recording noise.

- **What it is.** One event at the strike, about 20 dB per setting's full range, flat from 40 Hz to 2 kHz, the same at every top key, 5 dB softer at velocity 60 than at 100. There is no second event: no later hammer fall appears on these keys. It stands out in the high treble because the strings there are weak and leave the low band to it.
- **Fixed, then through the strings.** Below 0.75·f0 at C7, G7 and C8, the first 100 ms are one fixed waveform per key: correlation 0.999 from take to take, 0.996–0.998 between velocities 60 and 100 (only the level moves), 0.33–0.78 between keys, 0.98–0.996 with the tuning changed or the sympathetic and duplex resonances off. From 200 ms on the tail is strings: with those resonances off it falls 7 dB at 200 ms, 20 dB at 500 ms, and its lines move with the tuning (1272 → 1323 Hz). Its lowest line, near 1270 Hz, suggests the undamped top strings; sustain pedal 1 set through Carla changed nothing, so whether the pedal took effect is not known.
- **Noises off on both sides.** Setting 0 leaves a knock of the same waveform family (0.88 at C8), which no other setting tried removes. The model at its default knock is its nearest match, not the model with `noise.action` 0, which is 12–20 dB lower from 20 to 100 ms (`noise.keybed` 0 changes nothing here). Below 0.75·f0, in dB as above, at 0 / 20 / 50 / 100 / 200 / 300 ms: setting 0, −20 −24 −29 −36 −47 −57; model default, −18 −26 −35 −48 −63 −68; `noise.action` 0, −19 −38 −49 −57 −65 −68.
- **Against the real grands.** The commercial D's noise sits within them. With its setting at 0 it falls 10 dB below them. The model's knock starts 5–10 dB below them, which includes the default's deliberate 7.5 dB (above), and then falls too fast: 15–25 dB short from 100 ms on. The level is a choice; the decay is not, and `noise.action_time` (0.2 s) is the first thing to try.
- **The key's release.** `Key Release Noise` makes two events, at the key-up and 0.25 s later, the same at both velocities; set to 0, they vanish. At the top keys they lie 59–70 dB under the note's attack (−84 dBFS), the model's window 85–113 dB under: audible only with the note gone and the level high. Real release samples (VCSL) are still to be compared.

## The key's return (2026-11-24)

Listening report: the commercial piano plays the hammers falling back, plain when playing very softly and releasing late; the model had nothing there.

**The commercial D** (dry, velocities 15 and 40, keys C2–C8 held 8 s, then holds of 1–15 s and release velocities 1 and 127 on C4). Each key-up makes two events, on every key, with or without a damper:

- at the key-up, some 100 ms of noise around 500 Hz–1 kHz, −88 dBFS (10 ms rms);
- 0.25 s later a thud, strongest at 63–125 Hz, −14 dB at 1 kHz and −26 dB at 2 kHz, −74 to −80 dBFS, with a tail of 0.3 s.

Both are the same, to the dB, whatever the note's velocity, the hold and the release velocity. That is why it is heard after soft notes: at velocity 15 the C4's attack peaks at −40 dBFS and its strings are down to −83 dBFS after 8 s, under the thud; at velocity 100 the thud is 64 dB under the attack.

**Real grands.**

- Liang's Disklavier grand (velocity 96, held 2 s, microphones 50 cm above the strings), C5–C8, below 0.6·f0: a thud at the key-up, 20–25 dB under the same note's knock at the strike and 20 dB above the floor, peaking 10–100 ms after the key-up and gone in 0.25 s. A solenoid lets the key go; the key and the hammer fall back by themselves.
- VCSL's release samples, played by hand (Steinway B, Kawai): the strings' remainder dies in 50 ms, then a thud at 30–600 Hz comes 40–120 ms after the release starts and dies in 0.1–0.2 s, with nothing of it above 600 Hz.

So the event is real: one thud as the key lands back on its rail and the hammer on its rest, 40–120 ms after the key-up. The commercial piano's 0.25 s is longer than any of these, and its level sits some 15 dB lower against the strike's knock than the Disklavier's.

**The model.** `noise.key_return` (m/s; 0, the default: none, and every preset without it renders bit for bit as before) is the hammer speed whose knock the return equals: `noise.key_return_delay` (0.08 s) after the key-up the key strikes the action's bank, 12 dB per octave duller above 250 Hz, and the case's (`noise.case`), at that speed. Both are at release velocity 64: a release twice as fast lands in half the time, twice as hard (release velocities from 32 to 127 are told apart). The level does not follow the note's velocity. It follows `noise.action` and `noise.case`, and with them the musician's control `mechanism`. A key struck again before its return has landed makes no thud. The return lands at its own sample among the events, so a strike one sample earlier stops it and one sample later does not; and an engine replaced by a rebuild, which no longer gets the strikes, is told that the key went down (`Event::KeyDown`, piano-control's slot). The model's only event at a key-up used to be the damper's felt landing on the strings (`noise.damper_mass`), none on the keys without a damper.

At 0.2 m/s the thud is as far under the model's knock as the Disklavier's. The Concert grand takes 0.08: on C4, −74 dBFS against the commercial D's −77, its octaves from 63 Hz to 1 kHz at −4, −2, 0, −9 and −17 dB (the commercial D: −4, 0, −11, −7, −14). To be judged at the keyboard. Not modelled: the commercial piano's first event at the key-up on the keys without a damper, and a return that depends on how far the key was let up.
