# The treble's tail

Status as of 2026-10-29.

## What was heard

"Notes sound like a string buzzing slightly, and the sound is a bit mid-rangey… On strong strikes, as if harmonics saturated… In fact the note's tail is far too loud: on A5, against the commercial piano, the tail is at the same volume while the attack is much louder."

A5 at velocity 100, the user's sound (concert microphones), level re the attack's peak:

| | 0.3 s | 0.6 s | 1 s | 1.5 s | 2 s |
|---|---|---|---|---|---|
| commercial modelled piano | −19 | −29 | −38 | −39 | −36 dB |
| Salamander (real) | −12 | −20 | −27 | −31 | −33 dB |
| model | −10 | −22 | −21 | −25 | −22 dB |

From 0.6 s on, the model's tail stayed at −21 to −25 dB, beating with a period of about a second.

## Where it came from

The model's renderer was built at each commit of the last two days, and the same notes were rendered with the user's sound. The tail appeared with 90bc8f9, the board's outline in the physics. The user's sound already held the grand's outline, so it became active then; it has been on by default since bf23502. With the rectangle, A5 fell to −31 dB at 1 s and −33 at 2 s.

The sympathetic strings, the frame, its coupling, the rim, the top's undamped strings, the strings' nonlinearity, the polarisation and the strike's evenness each change A5's tail by less than 2 dB.

**The outlined board takes less of the treble's fundamentals.** Bridge conductance, outline re rectangle (same ρh, D and slopes; `examples/frame_drain.rs`, `RECT=1`):

| key | C5 | E5 | G5 | A5 | B5 | D6 | F#6 |
|---|---|---|---|---|---|---|---|
| partial 1 | −8.0 | −3.0 | −5.1 | −5.2 | −5.1 | −3.1 | −3.5 dB |
| partial 2 | +2.6 | −1.3 | −0.3 | 0.0 | 0.0 | 0.0 | 0.0 dB |

With the board's own modes up to 3 kHz instead of 1.5, partial 2 loses 0–2 dB and partial 1 the same: the outlined plate is genuinely less mobile there below 1.5 kHz. The nearest edge of the treble bridge is the front one in both boards, 6–21 cm away.

Less conductance couples a unison's strings less to one another. Their mistuning, 1.8 cents at A5, then dominates: they beat instead of decaying together into a prompt sound and an aftersound. Tightening the unisons removes the plateau (`width_mid` 0.5, `width_treble` 2.5: A5 −29 dB at 1 s, −35 at 2 s).

## The change

The unisons' mistuning grew from `width_mid` (1 cent up to C4) to `width_treble` (5 cents at C8) as t², with t = (key − C4)/4 octaves. Lowering `width_treble` mends the treble, but the top (C7–C8) loses its attack's 2–4 kHz and its envelope at 0.3 s. The new `unison.width_curve` sets the exponent instead. With 4, A5 is at 1.3 cents (1.8 before), C7 at 2.3 (3.25), and C8 keeps its 5.

| | score | off | treble env ff 1 s / 2 s (z) | treble env mf / pp 1 s (z) | top attack 2 kHz band (z) |
|---|---|---|---|---|---|
| t² (before) | 1.11 | 12 | +1.56 / +1.56 | +0.91 / +0.85 | −2.69 |
| width_treble 3 | 1.12 | 10 | — / +0.45 | — | −4.15 |
| width_treble 2 | 1.14 | 12 | +0.35 / +0.66 | −0.21 / −0.06 | −4.79 |
| width_mid 0.5, width_treble 2.5 | 1.17 | 12 | | | |
| t³ | 1.11 | 10 | +0.97 / +0.36 | +0.31 / +0.31 | −3.35 |
| **t⁴** | **1.10** | **10** | **+0.67 / +0.36** | **+0.06 / +0.13** | −3.88 |
| t⁶ | 1.14 | 10 | | | |

The default is t⁴. On the user's sound A5 now reads −24 dB at 1 s, −28 at 1.5 s and −39 at 2 s, against −21, −25 and −22 before. The tail decays instead of holding.

The honky-tonk preset keeps the quadratic curve (its unisons are meant to be far apart all along).

## Also seen

At the concert microphones (the user's set-up), a strong strike's attack carries 10 dB more at 2–4 kHz re its fundamental than Salamander's at C4 (−4.5 against −14.7 dB). With the close microphones it is −14.2, Salamander's. The board's radiation at that elevation and distance colours it; neither the lid, the room, a cardioid pattern nor the scattering layer changes it (the scattering raises it). Not changed here.
