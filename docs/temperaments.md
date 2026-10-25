# Temperaments

Status as of 2026-10-25.

A temperament sets how the twelve notes of the octave are tuned against each other. Equal temperament makes every key alike. The older ones keep some keys purer and leave others rougher, as music was played before the 19th century.

## Parameters

`[temperament]` in a preset (`TemperamentLaw`):

| | |
|---|---|
| `kind` | 0 equal, 1 Werckmeister III, 2 Kirnberger III, 3 Vallotti, 4 Young II, 5 quarter-comma meantone, 6 sixth-comma meantone, 7 Pythagorean, 8 custom; any other value: equal |
| `key` | the note class it is laid on, 0 = C … 11 = B (C for the usual tables) |
| `cents` | the custom temperament (kind 8): C, C♯ … B, cents from equal temperament |

A4 keeps its pitch (`a4`), whatever the temperament and the key. The stretch (`[stretch]`) adds on top: the octaves stay stretched as the strings' inharmonicity asks. Changing the temperament rebuilds the instrument.

## How each is set

Each temperament is set by how much each fifth of the circle is narrowed from pure (3:2, 701.955 cents). The twelve fifths add up to seven octaves, so one of them closes the circle with what is left:

| | narrowed fifths | closing fifth |
|---|---|---|
| Werckmeister III (1691) | C–G, G–D, D–A, B–F♯ by ¼ Pythagorean comma | — |
| Kirnberger III (1779) | C–G, G–D, D–A, A–E by ¼ syntonic comma | F♯–C♯ (the schisma) |
| Vallotti | F–C … E–B by ⅙ Pythagorean comma | — |
| Young II (1799) | C–G … B–F♯ by ⅙ Pythagorean comma | — |
| quarter-comma meantone | eleven fifths from E♭ to G♯ by ¼ syntonic comma | the wolf G♯–E♭, 35.7 cents wide of pure |
| sixth-comma meantone | the same by ⅙ syntonic comma | the wolf G♯–E♭ |
| Pythagorean | eleven pure fifths from E♭ to G♯ | the wolf G♯–E♭, 23.5 cents narrow |

The other fifths are pure. The note classes follow from the fifths' sizes. The published tables are matched to within 0.06 cent (test `temperaments_match_their_tables`).

## In the rendered sound

Measured on rendered notes (quarter-comma meantone against equal temperament, A4 fixed): C4 +10.9, C♯4 −14.6, E4 −3.5, G♯4 −16.6 and B♭4 +17.6 cents, against +10.3, −13.7, −3.4, −17.1 and +17.1 by the table. The strings' coupling to the board pulls each partial by up to 1 cent.

## Limits

- The temperament moves the notes' fundamentals. The strings' partials follow their own inharmonicity, as on a tuned piano.
- One temperament for the whole keyboard: no per-octave tables.
