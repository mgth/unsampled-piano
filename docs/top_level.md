# The level of the top: a flexible hammer shank

Status as of 2026-10-24.

## Listening report

In A/B against MAESTRO performances: "the high notes are more percussive in the real recordings".

## The register balance

The bench measured each register on its own; it now also compares them (`balance`): the attack's peak above 250 Hz, median over the register's keys, re the middle's at the same layer. Eight real grands against the model before:

| | bass ff | treble ff | top ff | top mf |
|---|---|---|---|---|
| real grands (mean) | −3.8 | +1.9 | −2.4 | −4.8 |
| model | −8.0 | −0.3 | −10.6 | −16.8 |

The top's deficit falls from C7 on: −11 to −17 dB at F#7–C8, against 0 to −4 on the real grands.

## Where the top loses it

The hammer gives the strings 58 % of its energy at C4 (ff), 8 % at C7, 1 % at C8. The 5.3 g hammer stays 0.53 ms on the C8 strings, two of their periods. The modal truncation is not the cause: the same energy at 48, 96 and 192 kHz.

The listening adds to it. The bench's close microphones stand 50 cm from the board's top corner, where the bridge ends; a microphone over that corner gains 11 dB at A7. In the default listening (a hall at 1.5 m) the top was at −5 to −8 dB.

What did not do:
- **Harder top felt alone** (`hardness_slope_top` 0.05–0.15): the top gains 0.4–2.3 dB and its attack gets too bright (bench 1.10 → 1.15–1.18).
- **Lighter top hammers** (×0.6 at C8): +2 dB, and the top's partials 2–4 fall against partial 1 by 3 z.
- **The strike point:** the energy swings by ±8 dB from key to key as the contact time crosses multiples of the period.

## The flexible shank

A hammer shank is a 13 cm wooden rod; the head sits on it as on a spring. A short blow (the treble, 0.5 ms) meets the head nearly alone, a long one (the bass, 3–4 ms) the whole hammer. The contact solver's hammer is now a head of mass (1 − `shank_share`)·m on a spring and a dashpot to the shank's effective mass `shank_share`·m, both thrown at the hammer speed. The mode of the head against the shank is at `shank_freq` (loss factor `shank_loss`). m is Stulov's effective mass, unchanged. The spring force on the head is explicit (central difference); the energy check includes the shank and the spring, and does not grow (test `flexible_shank_energy_never_grows`).

With 0.3 and 300 Hz the strings get +4.5 dB at C8 (ff) and −2 dB at C4. The gain goes to the top's fundamentals more than to its partials 2–4; the top hammers are voiced harder instead (`hardness_slope_top` 0.2: Q0 × e^{0.2·(n − 64)} above C6), as voicers do.

| | balance bass ff | treble ff | top ff | top mf | bench score |
|---|---|---|---|---|---|
| real grands | −3.8 | +1.9 | −2.4 | −4.8 | |
| before | −8.0 | −0.3 | −10.6 | −16.8 | 1.15 |
| shank 0.3 / 300 Hz, top slope 0.1 | −7.6 | +1.8 | −3.6 | −9.9 | 1.10 |
| shank 0.3 / 300 Hz, top slope 0.2 (default) | −7.6 | +2.1 | −1.9 | −5.9 | 1.09 |
| shank 0.3 / 400 Hz, top slope 0.2 | −8.4 | +1.1 | −2.4 | −6.4 | 1.10 |

The bench's score counts `balance` from now on (1.15 before for the same model). Other moves of 0.5 z or more:
- **Better:** the top's partials 2 and 4 at mf, its 2–8 kHz shares, the middle's 4 kHz attack band.
- **Worse:** the top's partial 3 at mf (+2.3 → +5.0 z), its 1 kHz bands, the treble's partials 2, 3 and 6 at ff (about 1 z each), the middle's 8 kHz bands.

## Still open

- **The bass** sits 4 dB under the real grands against the middle (z −1.9 at ff).
