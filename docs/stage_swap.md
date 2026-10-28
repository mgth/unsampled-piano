# Stage swap: the model's strings through a real board, the real strings through the model's

Status as of 2026-10-28. Tools: `tools/stage_swap.py`, `tools/bridge_partials.py`, piano-render's `bridge_signal` example, `Engine::bridge_force` / `bridge_velocity` / `pressure_scale`. Listening files in `out/ab_swap/` (not in the repository), their measures in `out/ab_swap/report.txt`.

## Why

The bass "tube" survived ten A/B rounds on parameters (docs/bass_metal.md). Every one of them changed the whole chain at once; none said whether the tube lives in the strings and the hammer or in the board and its radiation. Plath's dataset makes the two separable: for every key of a Steinway concert grand in an anechoic chamber, lid taken off, it holds the bridge acceleration (an accelerometer at the bridge, at the hitch pins of the string) and the pressure at a dummy head in the player's seat, recorded together. The model gives the same two signals: the bridge velocity at the key's region and the pressure at the player's microphones (`presets/listening/Microphones/player.toml`, lid off, no room). Each board is then estimated from its own pair, and the signals are crossed.

## Method

For a key (MIDI m, Plath's key m − 20, take M0002 of the piano's second state PROD08):

- **Real.** The bridge acceleration a_r and the two ears p_r, 50 kHz resampled to 48 kHz.
- **Model.** `bridge_signal` renders the note block by block and writes the region's bridge velocity (m/s), the microphones' pressure (Pa, the output scaling and the set-up's gain undone, no limiter) and, on request, the board's transfer from that region's acceleration to the microphones on an FFT grid. a_m is the backward difference of the velocity, as the engine makes it for the strings. The velocity is the one whose bridge acceleration (rms over 0.1–1 s) matches the real one: 101–110 on the four keys, Plath's dynamic was forte.
- **Each board**, from its own (a, p) pair: Welch cross-spectrum (16384-point windows, quarter hops, the whole note), the bins with coherence above 0.5 and energy within 40 dB of the maximum, between 20 Hz and 10 kHz (213–548 bins on the real piano, 300–770 on the model: the partials, and the attack's continuum where it is strong); the magnitude interpolated in log–log across the partials, nothing below 20 Hz, a roll-off above 10 kHz; a minimum-phase response (1.5 s) plus the ear's delay from the attack's cross-correlation. Everything that reaches the ears in step with the bridge counts as the board: rim, frame, radiation, the dummy head's colour on the real side. What does not, the action's own noise and the sympathetic strings, is left out of the swapped files. `--analytic` takes the model's board from its modes instead (`bridge_to_ear`'s transfer, exact at every frequency); both agree within 0.3–0.9 dB per third octave.
- **Files**, 48 kHz, 24-bit, aligned on the onset, the same loudness (EBU R128 integrated, −23 LUFS):

| file | what |
|---|---|
| `<note>_0_real.wav` | the real piano, p_r |
| `<note>_M_model.wav` | the model, p_m |
| `<note>_A_model_strings_real_board.wav` | a_m through the real board |
| `<note>_B_real_strings_model_board.wav` | a_r through the model's board |
| `check/<note>_real_via_estimate.wav` | a_r through the real board's estimate: how much the method itself changes the real piano |
| `check/<note>_model_via_estimate.wav` | the same for the model |

Keys: A0, A1, D2, A2.

## How to listen

- **0 against A**: the same real board, the model's strings instead of the real ones. If A carries the tube, the tube is in the strings and the hammer.
- **M against B**: the same model board, the real strings instead of the model's. If B still has the tube, the tube is in the board and its radiation; if B loses it, the board is innocent.
- **0 against check/real_via_estimate** first: that is the cost of the method on the real piano (0.6–2.6 dB rms per third octave in the sustain, see below). What the swaps change must be heard above that.

The real files were heard by a dummy head; the model's by two bare omnidirectional microphones at the same place. A and B each carry one side's colour.

## What the measures say (report.txt)

Third-octave levels, both channels, the bands within 40 dB of the reference's strongest, model minus real, for the two stages themselves:

| | A0 | A1 | D2 | A2 |
|---|---|---|---|---|
| bridge acceleration, attack 0–50 ms, 200–635 Hz | +10 at 200 | +9 to +15 | +5 to +11 | +4 to +8 (+10 to +15 at 100–159) |
| bridge acceleration, attack, 5–6.4 kHz | +11 / +10 | +13 / +20 | +14 / +10 | +7 / +10 |
| bridge acceleration, sustain 0.1–1 s, 1.3–2 kHz | −3 to −5 | −8 to −10 | −4 to −7 | −4 |
| ears, attack, 2–5 kHz | −6 to −24 | −3 to −8 | −7 to −18 | −5 to −15 |
| ears, sustain, 400–800 Hz | +4 to +13 | +5 to +15 | +3 to +8 | +10 to +20 |

Two things stand out before any listening:

- **At the attack the model's bridge carries more than the real one** at partials 3–12 (200–635 Hz, up to +15 dB) and at 5–6.4 kHz (+10 to +20 dB on every key), the latter at matched sustain level. The dispersive chirps docs/bass_metal.md saw in both pianos' bridges are far stronger in the model's.
- **The model's board and radiation then take 10–20 dB off 2–5 kHz at the player's ears** where the bridge has as much or more: the ears' attack sits 6–24 dB under the real one at 2–5 kHz while the bridge sits above. Axis 3 of docs/realism_axes.md measured the same board in the sustain, −11 to −13 dB per unit of bridge acceleration at 3–5 kHz.

So the two stages err in opposite directions above 2 kHz, and the full model, M, lands near the real piano there while neither stage is right. The swaps undo that compensation: A (the model's strings on the real board) has +4 to +13 dB more than the real piano at 2.5–5 kHz, B (the real strings on the model's board) 4–15 dB less than the model at 5 kHz. Which of the two the ear calls the tube is the point of the listening.

The lid: at the player's position the model's lid at 35° changes the radiation by less than 0.1 dB up to 4 kHz (none of the board's cells is shadowed, few reflect towards the ears); the files were rendered without it, as Plath's grand was.

## Limits

- The real board is sampled at the note's partials and interpolated between them: the fine structure finer than a partial's spacing (55 Hz on A1) is lost, and the attack's continuum passes through an interpolated response. The check files say what that costs: 0.6–2.6 dB rms per third octave on the real piano's sustain.
- The accelerometer's exact spot on the bridge, the dummy head's equalisation and the piano's state are Plath's; the model's bridge signal is the region's drive, a Hann average along a few keys of bridge, not a point.
- One velocity per key, chosen on the sustain's bridge level; the model's velocity law and the real forte do not have to agree at the attack.
- Isolated notes only. A passage would need the engine to write the bridge force of every key apart, and a real board response per key: the same machinery, more plumbing.

## Heard (2026-10-28)

The user, on the four keys: the real piano is the closest to the truth in every case, **B is always the worst** (the real strings through the model's board sound less like a piano than the model itself), A in between. In A there are often "whistles, like in overtone singing": the real piano has them too; in A2 they are much too strong, in D2 light, in A0 absent, in A1 weaker than in the real piano.

What the whistles are, partial by partial (0.3–1 s, both files through the same real board, so the difference is the model's bridge signal against the real one): on A2, partial 13 (1444 Hz) stands 14 dB above the real piano's and is the strongest line of the file; on A1, partials 20–30 (1.2–1.8 kHz), which the real Steinway holds at −1 to −9 dB under its strongest partial for a second, lie 18 to 40 dB lower. The real board rings a few partials out, and the model's strings give it the wrong ones.

### The strings' stage, measured at the bridge

`tools/bridge_partials.py` compares the model's bridge acceleration with Plath's, partial by partial: the level early (0.05–0.2 s) and in the sustain (0.3–1 s), and the decay over 0.1–1 s, no board or ear involved. Median decay of the partials above 1 kHz, dB/s:

| | A0 | A1 | D2 | A2 |
|---|---|---|---|---|
| real | −14.5 | −5.5 | −7.5 | −9.9 |
| model | −15.5 | −15.6 | −6.7 | −16.4 |

Partial by partial on A1, D2 and A2 the model's decays are not uniform where the real piano's are (−4 to −12 dB/s from 1 to 2.2 kHz): **−15 to −29 dB/s between 1 and 1.5 kHz, 0 to −4 dB/s between 1.7 and 2.2 kHz** (A1 partials 31–35, D2 partials 22–28). The decay of a partial through the bridge is the board's conductance there: the model's bass bridge has too much of it just below 1.5 kHz and almost none just above. That is where `outline_below` (1.5 kHz) hands the board from the outlined plate's own modes to the rectangle's, whose modal density jumps at the corner (docs/outline.md). The real board, ribbed, has no such step. Verify with this tool once the corner is moved (`outline_below` 3 kHz, which docs/outline.md found neutral on the bench) or the board above it made continuous with the one below.

The early levels differ by ±10 dB from partial to partial (A2: +9 at partial 12, +6 at 13 and 16, −8 at 11 and 14), the hammer's and the bridge's share of the whistles; and the real bridge has deep antiresonances the model's has not (A1's partial 18 at 1015 Hz: −43 dB early on the real bridge, −6 on the model's): the fine-structure spread axis 3 of docs/realism_axes.md measured.

So the two verdicts meet: the board and its radiation are the weaker stage to the ear (B worst), and the strings' stage errs where the board couples them (the decays), which the same corner of the board model explains.

### The corner tried; the unisons found

The outline's corner moved (`outline_below` 3 kHz, 6 kHz; the board builds in 0.5 and 2.3 s instead of 0.2) and the rectangle (presets/default.toml) leave the bridge decays where they were (A1 −15.6 to −15.9 dB/s whatever the corner, A2 −16.4 to −22.8), and the bench worsens (1.15, 1.17; the rectangle 1.09). The corner is not it.

The envelopes are. At the bridge, the model's A1 partials near 1.2 kHz fall 22–24 dB by 0.6 s and come back to −8 dB by 1 s; those near 2 kHz fall 18–22 dB by 0.35 s and come back to −6 dB: beat nulls, one period per partial, the period shrinking as the partial number grows. The real Steinway's partials fall 4–8 dB over the second with wobbles of a decibel or two. A bichord detuned by `unison.width_mid` = 1 cent beats at k·f₁/1731 Hz: 0.74 Hz at A1's partial 22, 1.2 Hz at partial 35, nulls at 0.67 and 0.42 s, as measured. The real pair's strings are tuned closer than that, or locked by the bridge; either way its bridge shows no null. `tools/bridge_partials.py` now reports the deepest dip of each partial's envelope below its exponential trend; medians over the partials above 1 kHz:

| | A1 real / model | D2 real / model | A2 real / model |
|---|---|---|---|
| deepest dip, 1 cent (default) | −1.1 / **−13.8** dB | −0.8 / **−8.9** | −0.3 / **−7.9** |
| deepest dip, 0.2 cent | −1.1 / −0.3 | −0.8 / −0.4 | −0.3 / −0.3 |
| decay, 1 cent | −5.5 / −15.6 dB/s | −7.5 / −6.7 | −9.9 / −16.4 |
| decay, 0.2 cent | −5.5 / −10.4 | −7.5 / −10.2 | −9.9 / −12.7 |
| decay, 0 cent | −5.5 / −8.6 | −7.5 / −8.8 | −9.9 / −11.0 |

The "decay two to three times too fast below 1.5 kHz, none above" of the section before was the nulls' first halves and their recoveries read as slopes. With the pair at 0.2 cent the bridge decays sit within 3–5 dB/s of the real piano's and nothing dips. What remains on A1 is excitation: partials 20–22 (1.17–1.28 kHz), the real bridge's strongest lines, lie 19–24 dB lower on the model's bridge from the first 0.2 s, and partials 17–19 and 31 are 10–30 dB stronger: the real board's own resonances and antiresonances at the bass bridge, which the model's bridge has not.

Bench: 0.2 cent scores 1.19 and 0 cent 1.20 against 1.11 (23 measures worse, 12 better): the middle's early decays become too slow (z −0.2 → +1.2/+1.6) and its envelopes at 2–3 s too low, the treble's sustain partials 3, 8 and 12 too weak. The beats were part of the calibration of the losses and the coupling: the bench cannot judge the width on its own, and the losses would have to be set again at the narrower width. docs/bass_metal.md had seen the nulls sweep ("a flanger") and tried an irregular detune, which keeps them; a narrower pair was never heard.

Listening files: `out/ab_swap_ob3000/` (the corner at 3 kHz: 0 and B, as asked) and `out/ab_swap_u02/` (the pair at 0.2 cent: A and M against the default's).
