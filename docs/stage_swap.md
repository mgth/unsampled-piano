# Stage swap: the model's strings through a real board, the real strings through the model's

Status as of 2026-10-28. Tools: `tools/stage_swap.py`, piano-render's `bridge_signal` example, `Engine::bridge_force` / `bridge_velocity` / `pressure_scale`. Listening files in `out/ab_swap/` (not in the repository), their measures in `out/ab_swap/report.txt`.

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
