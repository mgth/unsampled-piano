# Room level: piano directivity and microphone patterns

State as of 2026-10-23.

## Listening report

"The room presets sound rather caricatural, very reverberant."

## Measurement

The reverberation times match real treated rooms:

| Preset | Model T60 | Real rooms |
|---|---|---|
| studio | 0.3 s | recording booth 0.2–0.4 s |
| small_hall | 1.05 s | chamber music hall 0.9–1.3 s |
| concert_hall | 1.8 s | large hall 1.8–2.2 s |

The level of the room relative to the direct sound was too high. With the close microphones, the studio was only 9.5 dB below the direct sound, which is the level of an untreated room.

## Diagnosis

The room was fed as if the piano and the microphones were omnidirectional:

- **The piano is directional.** A grand radiates mostly upwards and towards the audience (directivity factor Q ≈ 3–4). The room receives the power of the source, so its level relative to the on-axis direct sound goes as 1/Q (about −5 dB).
- **Close microphones are usually cardioids.** A pattern a + (1 − a)·cos θ picks up the diffuse sound with a power efficiency of a² + (1 − a)²/3: 1/3 for a cardioid (−4.8 dB).

## Changes

| Parameter | Default | Role |
|---|---|---|
| `room.directivity` (new) | 3 | Q of the piano; the room input is divided by √Q |
| `soundboard.mic_pattern` (new) | 1 (omni) | a of the microphones; the room input is multiplied by √(a² + (1 − a)²/3) |

`mic_pattern` belongs to the listening set-up: the `close` preset uses cardioids (0.5), `concert` and `audience_lid_closed` an omni pair (1).

## Result

Room energy relative to the direct sound:

| Room | close mics before | close mics after | concert mics after |
|---|---|---|---|
| studio | −9.5 dB | −19 dB | +2 dB |
| small_hall | −7.8 dB | −17 dB | +2 dB |
| concert_hall | | −25 dB | −9 dB |

With the close microphones the room now sits 17–25 dB below the direct sound, as in a treated studio or hall.

The concert omni pair at about 2.6 m stays room-dominated in the small rooms. This is expected, since it lies beyond the critical distance of those rooms, and such a pair is not used there in practice.
