# What rings on after the release

State as of 2026-10-22.

## Listening report

"The small high resonance that lasts after the release sounds very realistic, so it must stay, but it is too present."

## Measurement

The sample references cannot answer: the Iowa and Salamander recordings are single notes held until they die, with no release in context. The commercial modelled piano was therefore recorded on a dedicated test, played to it in real time through the graph:

- `tools/release_test_midi.py` writes staccato runs and chords held 1 s, in the bass, middle and treble, at velocities 60 and 100, each followed by 2.5 s of silence and without pedal;
- `tools/release_tail.py` gives the octave-band levels 0.3–1.3 s and 1.3–2.3 s after the last key-up, in dB re the loudest band while playing;
- `tools/release_ring.py` does the same on the per-note reference sequences (key held 4 s).

Caveats:

- The commercial piano's current preset keeps energy at 250 Hz–1 kHz after the release, at the level of our hall: it probably includes some reverberation, so it is not a dry reference.
- Note by note (`tools/release_ring.py`), the model is quieter than the commercial piano after the release. The ring only shows in context.

2 kHz octave (1.4–2.8 kHz), dB re the playing level, early (0.3–1.3 s) / late (1.3–2.3 s), velocity 100:

| Segment | Commercial | Model before | Model after |
|---|---|---|---|
| middle run | −53 / −81 | −59 / −74 | −64 / −83 |
| middle chord | −61 / −89 | −66 / −81 | −70 / −89 |
| bass chord | −70 / −102 | −64 / −79 | −69 / −88 |
| treble chord | −68 / −99 | −58 / −73 | −62 / −81 |

Mean gap to the commercial piano over the 12 segments: 11.5 dB before, 5.3 dB after.

## Diagnosis

1. **The early level is right, the late one is not.** Just after the release the ring sits within a few dB of the commercial piano's. After 1.3–2.3 s it stayed 7 to 30 dB above.
2. **It is the sympathetic ring of the undamped top strings.** With the sympathetic resonance off, the 2 kHz tail of the middle drops to digital silence.
3. **These strings decay too slowly.**
   - The intrinsic losses had been halved above C6 to lengthen the sustain from A♭5 to D#6 ([long_sustain.md](long_sustain.md)).
   - In ESPRIT, the aftersound of the top treble (C7–C8) lasted 1.6 s, against 0.9–1.0 s in the three references.
4. **The commercial piano damps up to F#6.** One note at a time from D6 to C#7, released after 0.3 s: from D6 to F#6 the fundamental drops by 13–20 dB within 100 ms of the key-up; from G6 up, nothing of the kind. The model stopped at E6. On grands, the top 17–18 keys have no damper.
5. **The model's dampers are not the cause.** They stop a note faster and deeper than the commercial piano's (−53 dB within 50 ms of the key-up for E6).

## Changes

| Parameter | Before | After |
|---|---|---|
| `damper.highest_note` | 88 (E6) | 90 (F#6) |
| `losses.top_slope`, `losses.top_from` (new) | — | 0.05 from C6 (84) |

`LossLaw::sigma` multiplies b1 by e^{top_slope·(key − top_from)} above `top_from`: ×1.5 at G6, ×1.8 at C7, ×3.3 at C8.

Starting the increase at F#6 alone, only on the undamped strings, barely shortened the ring (9.7 dB of mean gap). Strings from C6 to F#6 also ring sympathetically while the pedal or other keys hold their dampers off. A slope of 0.06 shortened it further (3.9 dB) but made the held top treble decay 8 dB faster than the commercial piano at 1–2 s.

## Result

- **Ring after the release**: mean gap to the commercial piano 11.5 → 5.3 dB. The middle's late level is now that of the commercial piano.
- **Held top treble (C7–C8)**, dB re peak at 0.3 / 1 / 2 s: −27 / −65 / −87, against −31 / −61 / −80 for the commercial piano (−20 / −47 / −75 before).
- **Unchanged**: treble sustain (C5–B6), hammer score (0.51), attack and sustain brightness. The 1 kHz sustain band of the treble only drops for F#6 and A6, where it lies below the fundamental and holds nothing but sympathetic ring.
- **Tests and CPU**: unchanged.

## Bass: the partials with a node under the damper

Listening report: "the resonance that trails after the bass is released really drags on, for several seconds; the commercial piano has it too, but it stops within a second. It happens up to G3, not above."

Partial by partial, 0.3–1.3 s after the release of A0–C3, the model's partials 7, 14 and 21 (and partly 8, 15 and 22) were only 0–16 dB down, while all the others were 50–110 dB down. The commercial piano damps all of its partials alike, 25–39 dB down for the strongest.

The damper sits at 0.14·L, almost exactly on a node of partial 7 (1/7 ≈ 0.143), and on nodes of partials 14 and 21:

- ⟨sin²⟩ over the 2 cm pad is 0.012 for partial 7 of C2, against about 0.5 for the others;
- that partial thus gets 2 % of the full damping, and its T60 after the release is 8–9 s instead of 0.2 s;
- the narrower the pad relative to the string, the worse: hence the bass up to C3. From D#3 on, nothing escaped.

Widening the pad enough would take 9 cm, which is not realistic. On a real piano no partial escapes:

- the felt is crushed over a length of string;
- it presses on 2–3 strings whose relative contact points differ;
- it also brakes the sideways motion.

New `damper.node_floor`: every partial gets at least this share of a fully exposed partial's rate. The damper shape becomes floor + (1 − floor)·⟨sin²⟩.

| node_floor | A0 | C1 | D#1 | F#1 | C2 | F#2 | C3 |
|---|---|---|---|---|---|---|---|
| 0 (before) | 0 | −1 | −5 | −1 | −13 | −5 | −13 |
| 0.1 | −16 | −21 | −28 | −29 | −59 | −65 | −69 |
| **0.15** | −23 | −31 | −40 | −45 | −68 | −65 | −68 |
| 0.2 | −31 | −41 | −50 | −54 | −67 | −65 | −67 |
| commercial piano | −39 | −33 | −25 | −31 | −31 | −26 | −38 |

The table gives the strongest surviving partial 0.3–1.3 s after the release, in dB re its level just before.

With 0.15, the 350–700 Hz band of the bass runs and chords on the release test falls from −35/−46 dB (0.3–1.3 s / 1.3–2.3 s after the release) to below −100 dB. The commercial piano is at −53/−75 to −60/−96, its reverberation included. The pedal tests, half pedal included, still pass.

The fundamental of A2 (110 Hz) still keeps −20 dB for about a second: it sits on the board's first mode (109 Hz), which rings on after the strings stop (T60 ≈ 1 s).

## What remains

- **Staccato top notes** (G6, C7): just after the release they are still 15 to 20 dB above the commercial piano's. Their prompt decay (ESPRIT 0.5 s) is slower than in the references (0.3–0.5 s).
- **The sympathetic gain** (`sympathetic.gain` 0.25) was not touched: the early level matches.
