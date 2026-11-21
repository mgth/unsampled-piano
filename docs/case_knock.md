# The case's knock

Status as of 2026-11-21. Heard and kept: the Concert grand has `noise.case` 0.05.

## Listening report

After the body's halo settled in (docs/body.md) and the attack's rise (docs/attack_rise.md): the timbre is much closer, but the "box" of the Splendid Grand is still missing.

## What the real pianos have

Below the fundamental of the keys from C4 up there is no string partial. What sounds there on the real grands (Splendid, Plath's concert grand in its anechoic room, Helgesson), per third octave, medians over the keys, in dB re each note's own first 100 ms:

| C6–C8 | 80 Hz | 125 | 200 | 315 | 500 | 630 |
|---|---|---|---|---|---|---|
| Splendid, 0–20 ms | −31 | −29 | −24 | −22 | −24 | −25 |
| Plath, 0–20 ms | −32 | −30 | −30 | −27 | −25 | −23 |
| Model, 0–20 ms | −43 | −40 | −39 | −36 | −34 | −31 |
| Splendid, 60–150 ms | −33 | −39 | −36 | −36 | −41 | −43 |
| Model, 60–150 ms | −49 | −45 | −46 | −49 | −54 | −59 |
| Splendid, 150–400 ms | −47 | −49 | −47 | −50 | −57 | −60 |
| Model, 150–400 ms | −61 | −55 | −56 | −62 | −68 | −71 |

- **One event at the blow, the same under every key.** About the same level in every third octave from 80 to 630 Hz, and 16–19 dB less in the 63 Hz band than in the 80 Hz one on the three grands: nothing radiates below the structure's first resonance (77–93 Hz on six grands). In absolute level it is flat from B2 to C8 (Splendid's 80 Hz line within ±4 dB, Plath's 86 Hz line 10–15 dB over its neighbours on every key from C3 to C8): it does not follow the key's place along the keyboard, as a bending mode of the keybed struck under the key would (a prediction tested and refuted), nor the bridge's place.
- **A wooden decay.** T60 0.7–0.9 s at 80 Hz, 0.3 s at 500 Hz on Splendid and Plath: about (200/f)^0.5 from 0.55 s at 200 Hz, a loss factor of 3.5 % at 80 Hz falling to 1.4 % at 500 Hz.
- **A small source.** The two microphones agree at 80–160 Hz (correlation 0.9 to 0.7 on Splendid, 0.99 on Plath's dummy head) and part above 200 Hz, as two microphones at different distances from one source do.
- **Not the strings through the bridge.** The model with its knock and halo off holds −69 dB at 100 Hz in the first 20 ms at these keys, gone by 60 ms: the share of the hammer's momentum the string hands to the bridge (the striking ratio) is 40 dB short of what is heard.

The model's two sources there miss it each their own way: the action's knock (`noise.action`, 7.5 dB under the real grands' level by choice, docs/action_noise.md) dies in 0.3 s at 100 Hz and peaks at 1–1.5 kHz, a clack rather than a box; the body's halo (docs/body.md) is dense, comes in over 0.1 s and is heard with a random sign at each microphone. Together they are 11 dB under Splendid on average over 80–630 Hz and the first 0.4 s.

## The model

`crate::case`, heard directly like the action's knock:

- **The resonances:** 28, from `noise.case_low` (80 Hz) up, one every 34 Hz on average (thick wooden panels have few modes), level per resonance flat to 600 Hz and falling at 18 dB per octave above, where the action's knock takes over.
- **The decay:** T60 `noise.case_time` (0.55 s) at 200 Hz, as (200/f)^0.5.
- **The strikes:** each key strikes at the hammer's contact with a 1 ms Hann pulse, `noise.case` Pa at the microphones per m/s of hammer speed, with its own ±3 dB per resonance; the lowest resonances keep one sign for every key.
- **The microphones:** both sides hear the same knock, the second 0.3 m of path later.
- The `mechanism` control moves it with the other noises.

Fitted off the engine first (a sum of decaying sines added to the renders): the mean distance to Splendid over ten third octaves and four windows went from 7.7 to 1.9 dB.

## Level

Referred to the middle notes, as the action's knock was (against its own C4–B4, the model's C5–B5 are 5 dB weaker than Splendid's and its C7–C8 7 dB stronger, so a level set against each key's own note is off by as much): `noise.case` 0.05 puts C4–B4 within 1–3 dB of Splendid from 80 to 160 Hz. With it the keys C5–B6 stand 3–6 dB over Splendid re their own note, and C7–C8 6 dB under.

In a phrase the knock adds little energy: +0.3 dB at 70–180 Hz over 40 s of a Chopin nocturne, +0.6 dB over scales. It is heard under the notes whose strings leave that range empty.

## Listening

`out/ab_caisse_fable/index.html`: nine keys A1–C7 at mf and ff and two phrases, close microphones in the studio; the Concert grand as it is, with `noise.case` 0.05, with 0.1, what is added on its own (+12 dB), and Splendid.

Heard (2026-11-21): not the "box" the report was about, which is still to be found, but something the Splendid Grand has; well dosed on most keys, a little strong on C5 and G5 even at 0.05. Kept at 0.05.

## Key by key

The knock below the fundamental (72 Hz to half the fundamental, 630 Hz at most, first 150 ms) and the note's first 100 ms, each re the set's own C4–B4 notes, on the 40 keys of Splendid from C4 to C8:

- **The model's knock is even**: 0.6–1.9 dB of spread from key to key, as on Plath's grand (1.1 dB over its 49 keys). Splendid's wanders by 3.1 dB: 4–6 dB under its neighbours at C5 and from A6 to B6.
- **The model's notes are not**: E5 −9 dB, F5 −9, A5 −6, A#5 −7, G5 −4.5 against the middle, where Splendid's hold within 1.3 dB. At E5 a near-field dip of the close pair's right microphone at 650–665 Hz was found earlier; whether it is the cause on the other keys was not checked. There the knock stands 5–12 dB over Splendid's re the note.
- So C5 heard strong is Splendid's own low knock on that key; G5 is the model's weak note. The knock is left as it is.
- From C7 up the model's notes are 6–8 dB stronger than Splendid's against the middle, and the knock 5–12 dB more discreet re the note.

## The comparison tool

`--score`, the band without the pianos that buzz, the preset before and after: 376 → 372 measures off, mean gap 0.953 → 0.950. Rise time 40 → 37 off, early decay 50 → 49; no measure gains one. By register: bass 97 → 94, middle 97 → 98, treble 119 → 117, top 63.

## Not verified

- **What the radiator is.** The evidence says a small wooden structure struck alike by every key, with a first resonance near 80 Hz on grands of every size (the keybed and its frame are the only parts of one size on all of them); no measurement here tells the keybed from the case's bottom.
- **The second side's delay** is one figure for every set of microphones; it should come from where they stand.
- **The level of the notes from E5 to A#5** at the close microphones, which makes the knock stand out there.
