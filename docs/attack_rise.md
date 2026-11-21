# The attack's rise

Status as of 2026-11-21.

## The gap

The comparison tool's Rise time (10 % to 90 % of the peak) read 0–1 ms on the model and 4–12 ms on the real grands, and grew from 33 to 40 keys out of range once the body's halo stopped being struck at once (`body.rise`, docs/body.md). Per octave band, from −20 to −3 dB of the band's peak, mf, without the halo: B0–D2 at 1 / 2 kHz 2.5 / 1.5 ms against 8.9 / 7.6 ms on the real grands, A2–C4 1.8 / 1.5 against 8.7 / 3.7.

## Where it comes from

Plath's concert grand (anechoic, the bridge's acceleration at the hitch pins and a dummy head at the player's seat, forte) against the model rendered by `bridge_signal` at velocity 105, the player's microphones, no room:

- **At the bridge the model's strings are right.** From D2 up both rise in 1–2 ms in the 1–2 kHz bands. In the bass the dispersion is right too: at B0 each band reaches the bridge, after the strike, at 3.5 / 5.8 / 7.2 / 10.0 / 12.0 ms (8 kHz down to 500 Hz) in the model, 2.8 / 5.8 / 7.2 / 9.8 / 10.8 ms on Plath's grand. Neither the hammer nor the string is the cause above D2.
- **Below D2 the real bridge carries a precursor.** From the strike until the transverse wave arrives (5–10 ms at B0–A1), Plath's bridge holds a broadband plateau 15–25 dB under the note (1–8 kHz). The model's held nothing (−40 to −90 dB): its longitudinal modes started after the contact, faded in over 2 ms. The precursor is the longitudinal wave of the strike's own stretching; Plath's key reaches the keybed 1–3 ms before the strike, too late after it for the keybed's thump to be it, and the bridge's precursor follows the strike by about 0.7 ms.
- **After the bridge the real sound keeps rising.** At the dummy head the 1–2 kHz bands climb for 5–20 ms after the bridge; the model's reached their peak at once. On close-miked real grands (Headroom, Helgesson, Salamander, Splendid) the first 3 ms after the onset stand 6–8 dB under the next 10–40 ms from 250 Hz to 1 kHz in the bass; the model's stood 4–5 dB over them at 1–2 kHz.

Two things in the model arrived all at once:

- **The radiation had no travel time.** Each mode's weight at a microphone is the Rayleigh integral at the mode's own frequency: the time each part of the board takes to reach the microphone was only a phase, and the whole board was heard at the same instant. Split by distance into zones delayed by their own travel time, the bridge-to-ear response of A2–C5 at the player's seat rises like Plath's (C4 1 / 2 kHz: 3.5 / 4.2 ms against 4.3 / 2.6). With close microphones it changes little: the board under them dominates.
- **The case's field was there from the first sample.** The reverberant field of the box of the case and the lid (`case_field`) is 30–70 % of the energy at 1–4 kHz at the close microphones. A box fills over its time constant 4V/(cA): 4.6 ms on the concert grand with the lid at 35° (0.97 m³ under the lid, 2.45 m² of openings and absorption).

The halo's moment, the action's knock (at 0 ms instead of 8 ms after the strike: nothing measurable at its level) and the rise measure itself were ruled out. The measures tied to an onset are fragile: a variant that moves the held level by 1–2 dB moves the peak and the onset, and the −20 → −3 dB rise jumps; the level of the first 3 ms against 10–40 ms per band follows the ear better.

## The model

Three switches, off by default:

- `soundboard.case_field_builds`: the case's field into a mic slot of its own, heard through five Schroeder all-passes (gain 0.7, delays 0.07–0.47 τ, a little apart per microphone) whose energy arrives 16 / 43 / 78 / 98 % by 0.35 / 0.9 / 1.75 / 3.5 τ (e^(−t/τ): 30 / 58 / 83 / 97 %). Each partial keeps its mean level; against the direct sound a partial may move by a few dB, the case field's phase being random per mode either way.
- `soundboard.radiation_delays`: the Rayleigh integral split by path length into as many zones as the free mic slots allow (4 for a pair), each path's phase taken re its zone's delay, a whole number of samples; the zones fold into their microphone after their delays (`board::ZoneFold`, the board's own part too, which the room subtracts). At each mode's own frequency the sum is exactly the plain weight. About a quarter more CPU on the comparison tool's 264 notes.
- `nonlinear.strike_stretch`: during the contact, the string's speed under the hammer is v = F/(2Z) and the strain v²/c² leaves the strike point at ±c (d'Alembert, before the pulses reach the ends); the longitudinal mode n gets the bracket (8/(L·c))·cos(nπx_h/L)·∫v²(s)·cos(nπc(t − s)/L) ds of the modal drive, which takes over after the contact. `nonlinear.strike_scale` scales it.

`bridge_signal --zones` writes the zoned transfer next to the plain one.

## Listening

Close microphones in the studio, the Concert grand with `body.rise`, one change per column, equal level (out/ab_attack, out/ab_attack2):

- The case filling in brings the texture of the attack close to the Splendid Grand's. Two and three times slower filled no better: the box's own 4.6 ms is kept.
- The precursor as computed improved the attack from D2 up and added a harsh high ring below it (B0: +15 dB at 4 kHz over Plath's). At −12 dB (`strike_scale` 0.25) it meets Plath's at D#1 and A1, B0 but for 4–5 kHz, and the ring is no longer heard. Kept. The preset's phantom-partial `coupling` stands 10 dB over the default, which may account for the excess.
- The radiation's travel time was not singled out at the close microphones, where it changes little. It stays off.
