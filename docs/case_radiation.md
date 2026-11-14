# From the bridge to the listener: what the radiation's gaps were made of

Status as of 2026-11-14. Item 4 of docs/steinway_d_gaps.md listed four gaps of the Concert grand's radiation: 6–7 dB short at 630–1000 Hz against a real grand, a hole at 1–1.7 kHz and a step near 2–2.5 kHz, a missing hump at 100–250 Hz at the player's seat, and fundamentals below 60 Hz 9 to 29 dB short there. They were measured again on the preset as it is now (the board as the D is drawn, the top hammers nearer the capo bar), the measure itself was questioned, and each suspect was given a test before any code.

## What comes out

- **Most of the list does not stand as a gap of the radiation.** One was the former board's, one is the real recording's noise floor, and two are halves of a pair: the bridge's own motion is off one way, the transfer from the bridge to the ears the other way by about as much, and the sound at the ears is right within 4 dB from 100 Hz to 3.2 kHz.
- **One error was found in the radiation and corrected**: with the lid taken off, the case's field gave back to the listener nearly all the board's power a second time. It now gives back the share that meets the rim, 16 %. Lid on, nothing changes, bit for bit.
- **What is left at the ears against the real grand**, lid off, player's seat: +4 to +5 dB at 400 Hz, −8 at 4 kHz, +6 to +8 at 6.3–8 kHz, +11 at 80 Hz. None of them is in the list this work started from.
- **The real case's acoustics do not need to be built for these gaps.** Nothing measured here asks for the air's own modes, the frame's openings or the strings as radiators.

## The measure

Plath's concert grand (docs/stage_swap.md): bridge acceleration and a dummy head at the player's seat, lid off, 88 keys, two states of the piano a year apart (PROD07, PROD08). For each partial of each key, the ear's level over the bridge's, mean of the two ears in power. The model's side: `bridge_to_ear` (pressure per bridge acceleration at the key's bridge point, from the modes) and `bridge_signal` renders at velocity 105, the player's microphones, lid off, no room. Medians per third octave of the partial's frequency. The scripts are in this session's `out/tools/` (not versioned): `tfreal.py`, `tfreg.py`, `tfcmp.py`, `sig.py`, `stages.py`, `realwin2.py`, `lowfund.py`, `dbands.py`.

### What the real recording can and cannot say

- **The ratio is not the same early and late on keys with three strings.** A linear path from one force would give the same ear-over-bridge ratio at any time. On the real grand it does for the single strings (A0–E1: −0.6 dB early against late, both states), and does not in the treble: the fundamentals of C5–B6 have 5 to 6 dB more at the ears per unit of bridge acceleration in the first 80 ms than between 0.1 and 1 s (partials of C4–B6: +2 to +4). Once the unison's strings have drifted apart, the accelerometer, at one string's end, sees a motion that does not radiate. So the transfer is read in the attack (0–80 ms in the treble, 0–0.3 s in the bass), where the strings still move together as the model's single force does. `tools/plath_tf.py` reads 0.1–1 s: its treble figures are 2 to 6 dB low.
- **Take to take**, the ratio repeats within 0.1 to 2 dB rms per partial. **State to state**, the medians per band agree within 1.7 dB up to 5 kHz (2 to 3.5 above), with 2 to 9 dB rms per partial: a band's median is firm to about 2 dB, a single partial is not.
- **Below 38 Hz the real ears hold noise.** The fundamentals of A0–D1 (27.5–37 Hz) stand −13 to +4 dB from the level of the same band before the key is struck. The "29 dB short" of A0–B0's fundamental was the model against that floor. D#1–G1 (39–49 Hz) come out of it by 12 to 20 dB and are 12 to 19 dB over the model per unit of bridge motion; the chamber is small (the photographs show its wedges half a metre from the rim) and cannot be anechoic at 45 Hz. Not a firm gap either.
- **The seat.** The dummy head stands some 25 cm in front of the keys, the fallboard and the desk taken off. The model's player is 1.19 m from the board's front edge on the D (the set-up's 0.95 m scaled with the board), some 0.3 m further than the photograph suggests: 2 dB of level, not a shape.
- **The head** would raise the real side at 2–4 kHz if its ears were not equalised; the model is over the real one there, so it does not explain the excess.
- **No second real view of the transfer was found.** No other corpus of the inventory holds a bridge signal; the ones with several microphone positions give the whole chain only.

### Against the real grand, on the preset as it is

Model − real (dB), early window, all 88 keys, both states; n is the number of partials:

| Hz | 80 | 100 | 126 | 160 | 200 | 250 | 315 | 400 | 500 | 630 | 800 | 1000 | 1260 | 1600 | 2000 | 2500 | 3200 | 4000 | 5000 | 6300 | 8000 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| n | 10 | 13 | 16 | 22 | 27 | 33 | 43 | 57 | 68 | 87 | 104 | 125 | 152 | 176 | 190 | 206 | 208 | 211 | 180 | 169 | 119 |
| bridge acceleration | +2.9 | +2.3 | +4.7 | +5.2 | +5.3 | +5.7 | +4.5 | +4.0 | +2.2 | +0.9 | −4.3 | −3.6 | −3.9 | −5.1 | −5.7 | −5.6 | −4.8 | −5.7 | −3.5 | +3.8 | +7.4 |
| bridge to ear | +6.5 | +0.1 | −6.9 | −6.0 | −6.7 | −6.2 | +0.5 | +3.0 | +0.5 | −1.5 | +1.8 | +0.5 | +2.1 | +5.3 | +9.6 | +7.9 | +3.2 | 0.0 | +4.9 | +6.7 | +8.9 |
| ears | +10.5 | +3.3 | −2.6 | −2.5 | −1.2 | −0.2 | +2.6 | +6.7 | +2.7 | −0.5 | −2.1 | −3.0 | −1.4 | +0.5 | +3.9 | +4.1 | −1.7 | −5.9 | +1.6 | +9.0 | +10.8 |

The blow is not known (docs/steinway_d_gaps.md), so the bridge's and the ears' rows carry one unknown gain; the transfer's does not.

### Against the commercial D

Through the close microphones, lid on, velocity 100, partial levels over the first 0.3 s, one gain: every band from 100 Hz to 3.2 kHz is within 4.3 dB of the D's (+6.5 at 250 Hz, +3.7 and +4.3 at 315 and 400 Hz; −1.7 to +1.2 from 500 Hz to 1.27 kHz; +2.1 to +3.2 from 1.6 to 3.2 kHz). No hole at 1–1.7 kHz shows at the microphones. The hole and the step were read off the model's own transfer from force to microphones (docs/real_board.md), not off a difference with the D.

## The four gaps, one by one

| Gap as listed | On the preset as it is | Verdict |
|---|---|---|
| 6–7 dB short at 630–1000 Hz against the real grand | The same rendered swap (C5–C8, attack): +5.6, −1.1, −1.9 dB at 630, 800, 1000 Hz. All keys: −1.5 to +1.8 | **Gone with the board as drawn.** It stood on 4 to 6 keys per band, and a redraw of the board redraws them (docs/real_board.md) |
| Hole at 1–1.7 kHz, step near 2–2.5 kHz | Transfer +5 to +10 dB over the real one at 1.6–2.5 kHz while the bridge is 5 to 6 dB under it; ears +4 at 2–2.5 kHz; +2 to +3 against the D | **A pair at the bridge, plus 4 dB at the ears**; see below |
| Missing hump at 100–250 Hz | Transfer −6 to −7 dB, bridge +5; ears −0.2 to −2.6 | **A pair again: not a gap at the ears** |
| Fundamentals below 60 Hz, 9 to 29 dB short | Real ears at their noise floor below 38 Hz; 39–49 Hz in a chamber that is not anechoic there | **Not measurable on this recording** |

## The suspects, each with its test

Each was switched off or changed alone in `bridge_to_ear` and read in its own band, model − real, all keys.

- **The back face** (`back_gain` 0, and `back_depth` 1.5 and 2.5 m to give its sound a longer way round). Without it the transfer gains 4 to 7 dB from 63 to 250 Hz: 126–250 Hz comes to −2 to −5, and 80–100 Hz goes to +5 to +11. A longer way round does not make a hump: +3 to +4 at 126–160 Hz, as plain removal. Since the ears are right at 126–250 Hz and 10 dB too strong at 80 Hz, taking the back face away would cost more than it gives. **Refuted as the cause; not changed.**
- **The modes seen from the bass bridge at 117–166 Hz.** The second partials of B0–E2 there reach the real ears at −21 to −25 dB re the bridge's acceleration, the model's at −40 to −44: the largest single figures of the list. The model's bridge moves 5 dB more than the real one in the same band. Whether the board's third and fourth modes radiate too little to the player or move too much at the bass bridge cannot be told from a ratio. **Open; it is a matter of the board's modes at the bass bridge, not of the case.**
- **The bridge points and the bridge's mobility.** From force to ear the model's transfer is level within 3 dB from 500 Hz to 2.5 kHz (three to five keys, `force_to_ear`), with +1.5 dB at 2 kHz at the player's seat, 2 m above the board and at the close microphones alike: no lobe towards the player, no hole. Its bridge mobility is level too, 4·10⁻⁴ s/kg from 1 to 8 kHz. The real transfer per unit of bridge acceleration falls 12 dB from 1.27 to 2.5 kHz where the model's falls 7. A real bridge that yields more above 1.5 kHz (published upright boards do, by 14 to 17 dB above 2.5 kHz; nothing is published for a grand) gives exactly the pair seen: too little motion at the model's bridge, too much sound per unit of it, the ears right. **The excess of the transfer at 1.6–2.5 kHz is mostly the bridge's mobility, which is item 2's open question, not radiation.** The same pair was seen in the bass's upper partials (docs/bass_gaps.md).
- **The statistical plate above 6 kHz and the case's field.** With `case_field` off, lid off, the transfer loses 1 to 3 dB from 1.27 to 5 kHz and 5 to 6 dB at 6.3–8 kHz. From force to ear at the player's seat the field is 3 to 7 dB of the level above 2.5 kHz. **The field carries a third of the excess at 2–2.5 kHz and more than half of it at 6.3–8 kHz**; see the next section.
- **The air in the case as modes, the frame, its openings, the strings.** Not tested by a build: nothing above is left for them to explain within the measure's 2 dB. The literature has no measured air mode of a grand's case and no figure for the plate's shadow (`out/literature.md`).

## The case's field with the lid off

`case_field` (docs/real_board.md) treats the case and the lid as a small room: what the board radiates is counted once as its direct sound and once more as a field without direction, p² = 4ρc·P/R inside and the openings' share over a half-sphere outside. With a lid that is what happens: nearly all the board's sound meets the lid or the rim before it leaves. With the lid taken off the code kept the same reasoning with the whole top as the opening: 87 % of the board's power was sent out a second time, in every direction, on top of the direct sound that already carries it. At the player's seat, where the board's direct sound is weak above its coincidence, that second count was most of the level above 2.5 kHz.

The change: with no lid, the field takes only the share of the board's sound that meets the rim before leaving. For a diffuse radiator that is one less the view factor between two parallel rectangles, the board and the opening `lid_hinge_height` above it: 16 % on the D (84 % goes straight out), 8 dB less field. R = A/(share · (1 − the rim's absorption)), and the openings give back that share. No rim, no field. `SoundboardParams::case_room_constant`, `facing_share`; test `without_a_lid_only_the_rim_scatters`. Nothing is fitted.

Only a preset with `case_field` and the lid removed changes: the Concert grand played without its lid. With a lid, at any angle, the renders are bit for bit as before (four notes compared through the close microphones), so the bench (1.82) and the distance to the commercial D (8.23), both taken lid on, are unchanged by construction; neither was run again.

Lid off, player's seat, model − real, all keys:

| Hz | 126 | 160 | 200 | 250 | 315 | 400 | 500–1000 | 1260 | 1600 | 2000 | 2500 | 3200 | 4000 | 5000 | 6300 | 8000 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| bridge to ear, before | −6.9 | −6.0 | −6.7 | −6.2 | +0.5 | +3.0 | −1.5 to +1.8 | +2.1 | +5.3 | +9.6 | +7.9 | +3.2 | 0.0 | +4.9 | +6.7 | +8.9 |
| bridge to ear, after | −6.2 | −6.8 | −7.8 | −8.5 | −2.3 | +1.1 | −2.3 to +2.3 | +0.6 | +3.5 | +8.4 | +5.9 | +1.3 | −2.7 | +2.1 | +4.1 | +4.2 |
| ears, before | −2.6 | −2.5 | −1.2 | −0.2 | +2.6 | +6.7 | −3.0 to +2.7 | −1.4 | +0.5 | +3.9 | +4.1 | −1.7 | −5.9 | +1.6 | +9.0 | +10.8 |
| ears, after | −1.9 | −1.9 | −2.7 | −1.7 | +0.9 | +5.0 | −3.2 to +2.3 | −3.2 | −0.8 | +2.4 | +1.2 | −3.9 | −8.3 | −1.5 | +6.0 | +7.6 |

Over the twenty bands from 100 Hz to 8 kHz the ears' difference goes from 4.3 to 3.8 dB rms. Nearer above 2 kHz but for 3.2 and 4 kHz, where the model was already short and loses 2 dB more; 1 to 2.5 dB further at 200–315 Hz, where the field, fading below the case's Schroeder frequency, still gave a little. A small gain by measure: the reason to keep it is that the former count was wrong, not the half decibel.

## Listening

`out/ab_case_field/index.html` (the `ecoute` entry of `.claude/launch.json`): nineteen keys from A0 to C8 at the player's seat, lid off, no room, the real grand, the preset and the change, one gain for the two models and the real grand at the preset's mean level; then the same keys through the close microphones, lid on, with the commercial D, where the change does not act.

At the keyboard the change is only heard with the lid removed (the Lid switch of the listening page), on the build of this branch against the one of main.

## Not verified

- **Not heard and not played.** No verdict of the ear is on record for the change.
- **One real piano, one seat, one head.** No second measure of a grand's transfer from bridge to listener exists in the corpora or was found in print.
- **The bridge's mobility on a concert grand above 1.5 kHz.** The reading of the 1.6–2.5 kHz pair rests on uprights' published impedances and on the pair's own arithmetic. A force at the real bridge was not measured.
- **The board's modes at the bass bridge between 117 and 166 Hz**: too mobile there, or too weak towards the player.
- **The view factor takes the board as a diffuse radiator.** Above its coincidence a plate sends more of its sound at low angles, where the rim is: the share met may be larger than 16 % there.
- **With a lid**, the field still counts the whole of the board's power beside the direct sound and its image in the lid. It was set by ear and against the commercial D that way and is left alone.
- **The lowest octave at the ears.** Nothing here can check the model's fundamentals below 50 Hz at the player's seat.
- **What is left at the ears**: 400 Hz (+5), 4 kHz (−8, the bridge's share), 6.3–8 kHz (+6 to +8, where the board is still the statistical plate), 80 Hz (+11, ten partials).
- **The player's distance** on the D, 0.3 m too far by the photograph.
