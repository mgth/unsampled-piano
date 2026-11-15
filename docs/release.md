# Releases and soft blows against a Disklavier grand

Status as of 2026-11-12.

## Listening report

In A/B against real recordings (tools/ab_real.py, Chopin on Liang's Disklavier and on the Vienna Bösendorfer): "sharp cuts where the background resonance is sucked away; that does not exist in the original."

## The release of a key

Liang's isolated notes (no pedal, velocity 96, keys held 2 s) give the drop of partials 1–2 after the key-up, ±1.5 % around each partial (dB, median over the keys):

| | 30 ms | 60 ms | 120 ms | 250 ms | 500 ms |
|---|---|---|---|---|---|
| recording, bass | 0.6 | 0.8 | 2.6 | 10.8 | 25.7 |
| recording, middle | 2.7 | 7.1 | 15.8 | 28.2 | 39.7 |
| recording, treble | 2.7 | 5.6 | 17.6 | 24.9 | 38.8 |
| model before, bass | 1.7 | 3.9 | 8.5 | 20.0 | 39.8 |
| model before, middle | 2.1 | 7.1 | 19.1 | 43.5 | 82.7 |
| model before, treble | 4.3 | 18.4 | 32.7 | 55.5 | 80.3 |
| model after, bass | 1.1 | 2.6 | 5.1 | 13.5 | 27.7 |
| model after, middle | 0.5 | 2.7 | 14.2 | 27.3 | 32.9 |
| model after, treble | 2.2 | 8.9 | 18.5 | 27.7 | 49.9 |

A real note decays in two stages: fast at first, then a slow tail 25–40 dB down, still 15–20 dB above the room's noise a second later. The model's dampers took everything down at one rate, two to four times too fast after 60–120 ms.

Two changes:
- **A softer felt.** `damper.strength` 1 → 0.1 (c/(2·Z0): the impedance-matched felt was the most absorbing one), `contact_exponent` 2.5 → 1.5, `fall_time` 12 → 15 ms.
- **The sideways motion.** The felt rests on top of the strings: it stops their vertical motion and only rubs against the horizontal one. New `damper.horizontal` (0.03): the share of the rate the horizontal polarisation gets. Each coupled mode of a unison gets the rate weighted by Σ_j w_j·V_jm² (first order, Vᵀ·V = I), w = 1 on the vertical oscillators and 0.03 on the horizontal ones.

RMS gap over every key and the five times, drops capped at 50 dB: 10.7 → 6.0 dB.

The half pedal then damped less. `pedal_curve` 1.5 → 1.2 brings it back (tools/pedal_bench.py, CC64 = 64, partials 1–4 at 1.3–1.8 s, pedal minus no pedal): middle −11.5 dB against −12.1 recorded (−14.4 before), bass −8.0 (−8.8), treble −7.7 against −15.2 (−9.8). RMS gaps over the pedal bench's rows: halo 4.6 (4.6), hold 4.5 (4.3), after 9.5 (10.8).

## In the pieces

Quick pedal changes in Liang's four pedalled pieces (218 changes, pedal from ≥ 96 to below 40 and back within 0.4 s): dip of the band's envelope during the change, median (dB):

| band | recording | before | after |
|---|---|---|---|
| 80–300 Hz | 8.2 | 11.7 | 10.4 |
| 150 Hz–3 kHz | 7.5 | — | 8.1 |
| 400 Hz–1.5 kHz | 8.9 | 9.1 | 8.6 |
| 1.5–6 kHz | 6.8 | 8.4 | 8.2 |

The pedal releases themselves (38, drops at 30–500 ms) did not change.

## The dampers' line (2026-10-31)

On a grand the damper wires stand in a straight line behind the hammers', so the felt rests a fixed distance behind the strike point, not at a fixed share of the length: much further along the short treble strings. New `damper.behind_strike` (m along the strings, 0: `position` everywhere, as presets saved before it read). In the Model page's action the wires must stand behind the backchecks: 6 cm, which puts A0's pad at 0.16·L (0.14 before), C4's at 0.22, C5's at 0.3 and D6's against the 0.45 the law stops at.

**The release bench read the recordings' noise.** Liang's isolated notes reach their noise floor (−42 to −60 dB re the peak) within a second of the key-up, sooner in the treble: F#5 can drop only 6 dB before it. The treble's "slow tail" at −25 dB was that floor, and the model, which went on decaying, looked far too fast. Both the recording and the model are now read down to 3 dB above the recording's own noise (the louder of before the onset and its last 0.3 s), every damped key A0–D6, velocity 96, held 2 s. On that reading the treble was right (3.6 dB rms), and the bass and the middle released too slowly at 120–500 ms.

Same presets otherwise (a partial preset reads `shank_freq_mid` 0: the comparisons set it to 700 Hz). RMS gap to the recording over the keys, 60 ms–1 s, mean (dB):

| | bass | middle | treble | all |
|---|---|---|---|---|
| before (0.14·L, strength 0.1) | 9.0 | 7.2 | 3.6 | 7.6 |
| 3.5 cm, 0.1 | 8.7 | 6.8 | 3.7 | 7.4 |
| 6 cm, 0.1 | 8.7 | 6.3 | 3.7 | 7.2 |
| 6 cm, 0.15 | 7.8 | 5.7 | 3.9 | 6.6 |
| **6 cm, 0.2** | 7.5 | 5.3 | 4.0 | 6.3 |

With 3.5 cm and strength 0.2 the pedal bench's "after" (0.3–0.6 s after the dampers fall) lost 0.5 dB in the bass; with 6 cm it did not. Pedal bench (tools/pedal_bench.py, RMS of model − recording over its rows): halo 6.2 → 5.9, hold 4.5 → 3.7, after 9.2 → 9.2. In Liang's four pedalled pieces (tools/piece_bench.py), partials 1–2 0.3 s after the damper falls, model − recording: bass +3.8 → +2.7 dB, middle +4.0 → +2.5, treble +7.3 → +3.7 (the decays while held unchanged). Note bench (tools/bench.py): 1.04 → 1.04.

So `behind_strike` 0.06 and `strength` 0.1 → 0.2. The treble's first 120 ms now drop 3–4 dB faster than the recording: the rate grows as f0 (c ∝ Z0), the next thing to look at.

## The felt's strength up the keyboard (2026-11-12)

With one `strength` for every key, c/(2·Z0) is the same everywhere and the rate, c/(μL), grows as the key's frequency: the bass lingered and the middle stopped dead. On the Concert grand preset, keys held 3.5 s at velocity 100 took from 0.04 s to more than 0.42 s to lose 20 dB after the key-up, where the commercial D takes 0.14–0.21 s on every damped key. The commercial D's median drops (2.9, 8.3, 18.9 and 29.0 dB at 30–60, 60–120, 120–250 and 250–400 ms) are close to the Disklavier's middle above.

New `damper.strength_slope`: `strength` is A1's (55 Hz) and falls as (55 Hz / f0)^slope. 0, the default, changes nothing: the default instrument renders bit for bit as before. The Concert grand takes 1, `strength` staying at 0.2.

Drops after the key-up re the last 0.2 s held, broadband, close microphones, no room, each key read down to 3 dB above the reference's own floor, RMS of model − reference over the keys and the windows (dB):

| | bass | middle | treble | all | key-to-key spread of the 120–250 ms drop |
|---|---|---|---|---|---|
| commercial D (A0–F#6, velocity 100, held 3.5 s), slope 0 | 6.6 | 8.6 | 13.0 | 9.4 | 7.0 (the D: 3.5) |
| slope 1 | 5.9 | 5.6 | 8.2 | 6.5 | 4.8 |
| slope 1.3 | 5.9 | 6.3 | 8.2 | 6.7 | 3.9 |
| Liang's Disklavier (A0–D6, velocity 96, held 2 s), slope 0 | 6.1 | 5.6 | 5.2 | 5.7 | 6.1 (the recording: 4.5) |
| slope 1 | 5.7 | 2.7 | 3.5 | 4.3 | 4.9 |

A uniform change does not do it: `strength` 0.05 with `horizontal` 0.2 brings the middle to 5.3 against the commercial D and the bass to 11.1 (every third key). Slope 1.3 with `strength` 0.25 or 1.5 with 0.3 measure as slope 1 (6.5 overall). In the treble the commercial D's take reaches its own floor 11–25 dB below the held level, so little of its release is read there.

Pedal bench on the Concert grand (tools/pedal_bench.py, velocity 80, RMS of model − recording over its rows): halo 3.6 → 3.6, hold 4.9 → 4.5, after 11.5 → 10.0. The half pedal (CC64 = 64) now damps less than recorded in the middle and the treble (hold +3.2 and +3.0 dB, −6.1 and −2.9 before), and the middle's full-pedal "after" goes from −4.7 to −9.7 dB. The distance to the commercial D on its 88 held keys does not move (9.49).

## The sideways tail (2026-11-12)

With the slope, some keys still rang on after the key-up: C#3, C4, G2, D#3, F2, F3, C3. Over C2–C5 the level 0.25–0.4 s after the key-up, re the attack's peak (velocity 100, held 3.5 s), had a median of −50 dB and reached −34 dB on C#3, eight keys above −45 dB; the commercial D's median is −60 dB, its loudest key −49.

It is the slow mode the bridge's cross term leaves. The Concert grand's `polarization.cross` is at its passive limit (docs/real_board.md): the bridge then takes one tilted polarisation and leaves the other, mostly sideways, alone. The felt braked that one with `horizontal`, 3 % of its rate. One change at a time on thirteen keys:

| | tail re the attack's peak, C#3 / C4 / G2 / D#3 (dB) |
|---|---|
| the preset | −34 / −42 / −40 / −43 |
| `polarization.cross` 0 | −55 / −63 / −52 / −54 |
| `polarization.hammer_tilt` 0 | −34 / −42 / −40 / −42 |
| `polarization.radiation` 0 | −34 / −42 / −39 / −41 |
| `damper.horizontal` 0.1 | −40 / −46 / −45 / −48 |
| `damper.horizontal` 0.3 | −52 / −55 / −52 / −56 |
| commercial D | −60 / −62 / −57 / −61 |

The cross term is the one measured on a real grand's partials and sets the held note, so it stays; the felt's grip is the lever. `horizontal` 0.03 had been set on a slow tail 25–40 dB down in Liang's recordings, which turned out to be their noise floor (above). The Concert grand takes 0.3; the default instrument keeps 0.03.

| `damper.horizontal` | 0.03 | 0.1 | 0.3 | 1 |
|---|---|---|---|---|
| tail, C2–C5: median / loudest key (dB) | −50 / −34 | −54 / −40 | −57 / −43 | −60 / −43 |
| keys above −45 dB | 8 | 4 | 1 | 1 |
| drops against the commercial D, all keys (dB rms) | 6.5 | 6.2 | 6.1 | 6.5 |
| middle | 5.6 | 4.7 | 4.2 | 5.0 |
| drops against Liang's Disklavier (dB rms) | 4.3 | 4.3 | 4.3 | 4.3 |

The drops' benches hardly see it: both read down to the reference's floor, which hides a tail 35 dB down. The held notes do not change (distance to the commercial D 9.49 → 9.49). Pedal bench, 0.03 → 0.3: halo 3.6 → 3.6, hold 4.5 → 4.8, after 10.0 → 11.5. The middle's "after" comes closer (−9.7 → −2.0 dB at CC64 = 96), the treble's goes back to where it stood before the slope (+5.7 → +15.7): once a key released without the pedal falls deeper, what the other strings add under the pedal, already 5–6 dB over in the treble's halo, shows whole.

## The dampers against a real grand's (2026-11-15)

Items 7 and 8 of docs/steinway_d_gaps.md, measured again on the preset after the board, the top strike point, the felt and the bass unisons changed, then split into parts. The scripts and renders are in `out/` of the worktree `dampers-release-34d47e` (`tools/`, `base/`, `cand/`, `pieces/`, `ab_dampers/`); the literature notes with their sources in `out/literature.md`.

### Where the dampers end

**The literature**: 71 dampers on a D, the last at G6, from a pianist's blog, a forum snippet and a technician's post giving 71 for the model C; no document of the maker's was found. Steinway's B, M and L have 67 (D#6), a Hamburg B 68, Yamaha's grands 69 (F6).

**The real recordings.** A string without a damper answers any blow at its own fundamental. In the sustain (0.3–1 s) of the thirteen keys C7–C8, the line at each key's own measured fundamental, in dB over the floor around it (median over the played keys):

| | C6 | C#6 | D6 | D#6 | E6 | F6 | F#6 | G6 | G#6 | A6 |
|---|---|---|---|---|---|---|---|---|---|---|
| a real concert grand (Plath, anechoic) | 8 | 8 | 9 | 10 | 6 | 19 | 18 | 30 | 19 | 24 |
| Yamaha CF-3 (Helgesson), five keys, 0.5–2 s | 6 | 4 | 3 | 2 | 10 | 6 | 5 | 6 | 25 | 28 |
| Disklavier grand (Liang), five keys, 0.5–2 s | 6 | 7 | 6 | 7 | 7 | 9 | 17 | 20 | 29 | 28 |

The real concert grand's strings ring free from F6: its last damper is E6, 68 dampers, which is also the count read by the user on a photograph of a D from above. The CF-3's last damper is G6, the Disklavier's F6. The candidate takes E6 (`highest_note` 88); G6 was the commercial D's setting.

**The −110 dB against −43 is not the last damper's place.** The bench's measure is the 707–1414 Hz octave, 0.6–1 s after the blow, on C7–C8: no partial of the played key lies there. In the real grands it is made of lines (75 to 98 % of the band's energy in six lines on the sets with a low floor), 12 to 25 dB over their own noise. On the real concert grand most of these lines are not free strings' fundamentals: 1105–1108, 1167–1172, 1238–1256, 1365 and 1388 Hz, some 20 cents under the damped keys C#6 to F6, and others between the keys. With the last damper at G6 the model's first free string (G#6, 1661 Hz) is outside the band and nothing is in it: −121 dB. At E6, F6 enters it: −62 dB (the real grands −34 to −53, mean −43). The 20 dB left are what else rings in a real piano there: the strings under their felts and the lengths beyond the bridge and the capo bar (item 12), none of which the model has.

### The release, split into parts

**The bench of the drops** (out/tools/relbench.py: broadband drop after the key-up re the last 0.2 s held, each key read down to 3 dB above the reference's floor; it gives 8.6 and 5.3 dB rms on the renders of 2026-11-12 where the tables above give 9.4 and 6.1). On the preset of today: 5.3 dB rms against the commercial D (bass 4.9, middle 4.2, treble 6.8), 4.8 against the Disklavier grand (bass 5.5, middle 4.3, treble 3.2).

**The damper's rate per partial** (out/tools/relpart.py): time each partial takes to lose 20 dB after the key-up, median over the keys (ms), read only where the partial stands 23 dB above the file's floor.

| | partial 1 | 2 | 3–4 | 5–8 |
|---|---|---|---|---|
| A0–B1, Disklavier grand | 188 | 201 | 226 | 333 |
| A0–B1, commercial D | 191 | 209 | 187 | 198 |
| A0–B1, the preset | 265 | 145 | 103 | 325 |
| A0–B1, wedges | 208 | 225 | 214 | 208 |
| C2–B2, Disklavier grand | 177 | 206 | 103 | 197 |
| C2–B2, the preset | 204 | 103 | 100 | 201 |
| C2–B2, wedges | 185 | 133 | 134 | 185 |
| C3–B3, Disklavier grand | 208 | 120 | 63 | 64 |
| C3–B3, the preset | 163 | 107 | 94 | 131 |
| C4–B4, Disklavier grand | 83 | 80 | 90 | 108 |
| C4–B4, the preset | 134 | 75 | 161 | 103 |
| C5–B5, Disklavier grand | 81 | 112 | 128 | 103 |
| C5–B5, the preset | 92 | 95 | 144 | 92 |

From C3 up the preset stops its partials as the real grand does, within the scatter of twelve keys (the Disklavier's own lag between the note-off and the damper is not known). In the low bass it does not: the real grand and the commercial D stop every partial at one rate, the model's law (a rate following the partial's motion under the pad) stops partials 2 to 4 twice too fast and the fundamental too slowly.

**A strong felt, refuted.** The exact modes of a string with a point dashpot (out/tools/dashpot.py) decay as the model's law only for a weak felt; from c = 4·Z0 up every mode decays at one rate, the string cut in two. But a viscous felt that strong passes through the matched impedance as it lands: with `wedge_strength` 5 at A0 the bass lost 10 dB within 40 ms, where the real string has lost 0.6 dB after 30 ms. And one strength law does not carry the exact rates from the bass (uniform) to C3–B3 (the fundamental three times slower than partials 3–4). Not built.

**Wedges, kept as a candidate.** The bass dampers are wedges astride one string or between two (Lehtonen, Askenfelt and Välimäki 2009, on a Steinway C: single wedge at C1, bichord wedge at A1, double wedges at G2 and G3, flat pads from A4; a felt of 98 mm at C1 and 71 mm at G3, starting at the strike line), and their model of the damper limits the string's displacement over the felt's span (Stulov, Välimäki and Lehtonen 2012), which acts on the wave and not mode by mode. New `damper.wedges` (a switch, off by default: every other preset renders bit for bit as before), `wedge_until` (B1, the wedge fading over the octave above) and `wedge_strength` (0.06: c/(2·Z0) at A1, every partial at the full rate). A uniform rate on every key was tried first (`node_floor` 1): 4.8 against the commercial D but 7.2 against the Disklavier grand, the bass too fast, and it loses the real grand's pattern from C3 up.

**The timing of the fall.** The real grands start slowly (Disklavier bass: 0.6 dB at 30 ms, 2.6 at 120 ms; Askenfelt and Jansson: the damper meets the string some 80 ms into a staccato note and bounces). The model's broadband drop is 2 to 3 dB ahead of both references at 30–120 ms. `fall_time` was not changed: the Disklavier's lag is unknown and the commercial D's is its own.

**The tail and the pedal's "after": the measure's.** In Liang's recordings the level 0.3–0.6 s after the dampers fall is 0 to 5 dB over the level before the note in every register and at every pedal depth (9 dB in the bass without the pedal): the bench's "after" reads the recording's noise, and the model, which has none, falls 20 to 50 dB deeper. With the recording's own noise put under the model (out/tools/pedfl.py) the "after" rows go from 9.5 to 2.9 dB rms. The treble's "after", 15.7 dB over the recording on 2026-11-12, is 4.2 as read on today's preset and nothing over the floor. What the recording does say: after the dampers fall the residue is at least 16 dB down in the treble, 25 in the middle and 29 to 37 in the bass. The "hold" rows are sound: its partials stand 35 dB or more over the noise.

**The half pedal.** Lehtonen and others: the part-pedal range is narrow, about 0.5 mm of damper lift, the string free at 0.9 mm in the treble and 1.35 mm in the bass; the tail's decay time grows from under 0.5 s to 10 s through it; Brauss puts it at 30 to 50 % of the pedal's travel, which is `pedal_free` 0.3 and `pedal_contact` 0.6. At CC64 = 64 (hold, pedal minus no pedal, model − recording): bass −9.3 dB, middle +2.6, treble +6.0. The bass's excess is the same fault as its key-up: partials 2 to 4 stopped too fast.

### The change, beside the preset before it

`wedges` on up to B1 with `wedge_strength` 0.06, `highest_note` 88 (E6). Everything else as the preset.

| | before | in the preset |
|---|---|---|
| drops against the Disklavier grand, all / bass (dB rms) | 4.8 / 5.5 | 5.2 / 6.1 |
| drops against the commercial D, all / bass | 5.3 / 4.9 | 4.7 / 3.1 |
| pedal bench: halo / hold / after as read / after over the floor (dB rms) | 5.5 / 4.7 / 9.5 / 2.9 | 5.1 / 4.2 / 9.6 / 2.7 |
| half pedal, hold: bass / middle / treble (model − recording, dB) | −9.3 / +2.6 / +6.0 | −5.9 / +4.4 / +5.3 |
| top keys' sustain, 1 kHz band (real grands −43) | −121 | −62 |
| distance to the commercial D, 88 keys | 8.06 | 8.06 |
| bench of eight real grands, overall / treble / top | 1.73 / 1.54 / 2.74 | 1.44 / 1.44 / 1.91 |
| the treble's sustain, 1 kHz band (real grands −26.4) | −40.7 | −26.4 |

The bench moves by the last damper alone: its 1 kHz band in the sustain, where F6 to G6 now ring, comes to the real grands' in the treble and nearer in the top (z −12.9 to −3.4); every other family of measures is the same to the figure (attack 1.23, dynamics 1.66, sustain 1.68, decay 1.00), as is the distance on the 88 held keys.

Against the real grand the per-partial times come right in the low bass while the broadband drops get 0.6 dB worse there: the model's bass already fell 3 dB deeper than the recording at 120–500 ms, and falls 4.8 deeper once the fundamental stops in time. What the recording keeps at that depth (other strings, the case, its room) is not told apart.

Listening page: `out/ab_dampers/index.html` (key-ups against the Disklavier grand and against the commercial D, pedal at 127, 96 and 64 against the Disklavier grand). Keyboard presets: `out/keyboard/instruments/Dampers trial/` (the preset, the candidate, the wedges alone with the last damper left at G6). Taken into the Concert grand preset on 2026-11-15, on the user's decision, once the keyboard presets were up.

### Not verified

- **The D's own damper count**: 68 from one real concert grand's free strings and one photograph, against 71 in informal sources. No maker's document.
- **Where a D's wedges end**, the felts' lengths on a D, and the wound trichords' felts: Lehtonen's shapes are a Steinway C's; `wedge_until` B1 is where the Disklavier's partials stop at one rate.
- **The Disklavier's lag** from note-off to damper, hence any absolute time under some 30 ms, and `fall_time`.
- **The commercial D under the pedal**: no take was made; its key-ups only.
- **The halo** (bass and middle 5 dB short, treble 7 to 9 over) was not split: the recording's floor lies in its no-pedal reference too (treble −38 dB against the model's −50).
- **The treble's hold at every depth** (+4 to +5 dB) and the top's.
- No listening remark was given with the decision.

## Soft blows: the unison struck unevenly

The most audible cut left came 0.3 s after a soft chord, the pedal changing just after it. Rendered with the pedal held throughout, the model still lost its 1.5–6 kHz band within 0.3 s; the recording kept it for 0.7 s. Over the first 0.3 s, eight real grands (tools/bench.py's references) decay the middle's upper partials at about −20 dB/s at pp and −30 at ff. The model had the same decay at every velocity: it is linear once the hammer leaves.

A soft blow meets the strings with the surface of the felt only, and a unison's strings are never quite level: the lighter the blow, the more unequal the force each string takes. The uneven part excites the slow out-of-phase modes, the aftersound. The spread of the force (`unison.hit_spread`, 0.2) now grows as the hammer slows:

spread(v) = hit_spread·(soft_speed/v)^soft_exponent, capped at 1

with `soft_speed` 2.75 m/s (velocity 100: unchanged at ff) and `soft_exponent` 1 (0.58 at velocity 45).

| soft_exponent | 0 | 0.5 | 1 | 2 |
|---|---|---|---|---|
| bench score | 1.13 | 1.09 | 1.06 | 1.17 |
| within the real pianos | 75 % | 76 % | 80 % | 77 % |

With 1, the middle's pp and mf sustain (2–3 s) come within the real pianos (z −1.0 → 0.0), and the treble's from −1.4 to −0.7; the top's attack partials 3–4 at mf improve. Exponent 2 overshoots in the treble (sustain at 1 s z +1.4, attack partial 2 z +4.3). The dampers' changes do not move the bench (1.13 either way): it holds every note 20 s.

## Strings at different heights

Over the first 0.3 s at pp, the middle's partials 5–12 still decayed at −29 to −45 dB/s against about −20 on the real grands. The comparison is made piano by piano and key by key, then the median taken: a median of the pianos' envelopes smooths their beats away and made the real decays look slower than they are. Partials 6 and 8 fared better only because they meet the undamped top strings, which hold them up sympathetically.

The real envelopes at pp hold these partials at −5 to −15 dB for up to a second. The model's dropped to −30 dB within 0.5 s, the in-phase mode dying into a beat trough before the slow modes took over.

A unison's strings never lie exactly level, so a slow hammer reaches them one after the other: 0.1 mm at 0.5 m/s is 0.2 ms, a lag of radians at 2–3 kHz. `unison.height_spread` (0.1 mm) gives string j of a key the delay h_j·height_spread/v (h_j between 0 and 1, irregular per key).

The first version rotated each string's partials by that lag when the contact ended, as if each string had its own hammer, struck that much later:

| height_spread, rotation | 0 | 0.05 mm | 0.1 mm | 0.2 mm |
|---|---|---|---|---|
| bench score | 1.06 | 1.03 | 1.02 | 1.05 |
| attack (z rms) | 1.04 | 0.93 | 0.85 | 0.87 |

Early decay 0.1–0.3 s at pp, median over the middle's keys (dB/s):

| partial | 5 | 7 | 9 | 10 | 12 |
|---|---|---|---|---|---|
| real grands | −19 | −19 | −19 | −20 | −22 |
| before | −36 | −33 | −29 | −34 | −45 |
| rotation, 0.1 mm | −19 | −21 | −23 | −28 | −29 |

The rotation made the bridge force jump the moment the contact ended: a click at every note, loudest at pp ("glitches" in the A/B). The strings now take part in the contact itself: string j takes no hammer force until its delay. At rest it joins without moving the felt, so the scheme stays continuous and passive. But one hammer drives every string with nearly the same force, and 0.2 ms is short against a 2–3 ms contact: the strings' partials barely part, and the early decays and the bench (1.06) are back to what they were without it. The rotation's gain came from independent, shifted force pulses, which a single felt does not give. A felt compressed by each string separately, each with its own force history, would be the physical model.

Two other levers were tried:
- **A tighter unison** (0.4–0.6 cent): it helps partials 5–8 but thins the slow modes, so the 2–3 s sustain falls by 0.5–1 z.
- **A wider or heavier bridge drive**: it slows every partial, the low ones too, and the bench gets worse (1.08–1.22).

## Still open

- **The treble's level after a pedal release** read 16–19 dB above a plain key-up's where the recording has −2 to −6 (pedal bench, "after"): the recording's noise floor, not the model ("The dampers against a real grand's").
- **The default instrument's felt** still has one strength for every key; the slope is set on the Concert grand only.
- **The middle's upper partials at pp**: partials 5–12 still decay at −29 to −45 dB/s over 0.1–0.3 s against about −20 on the real grands.
- **The cut after the soft chord** (Liang, Chopin op. 10 no. 3, 22.3 s): the model's 1.5–6 kHz band still drops 16 dB within 0.35 s, the recording 5. The chord's E4 dominates it, and isolated it sits within the real grands at pp, on their fast side.
- **The bass's quick pedal changes**: 80–300 Hz still dips 2 dB deeper than recorded.
