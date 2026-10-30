# Test bench: scoring the model against real pianos

Bench updated 2026-10-30: the register measures follow the `channel-power-v3` protocol (channel power, below, as the harmonic profiles already did; v3 adds the partial floor, the two-source rule and the `resonance` family, see "Weak partials, single-source targets and the resonance measures"). Historical results below are dated separately and predate it unless stated.

The register measures now take the mean **power** of the channels, in the Python measures and in the Rust partial tracker and decay fits (`piano-survey --channel-power`). They no longer sum the left and right pressures first: a partial in antiphase between the two microphones is not lost. Plath is imported with one gain per pascal for the whole piano; the old per-note normalisation is refused. Reference and baseline files carry `analysis_version`: references of another version are refused, and a baseline of another version is not scored against the new measures. Codex's controlled before/after comparison, on the model of 2026-10-28: [bench_bias_correction.md](bench_bias_correction.md).

Until now each change was measured with the script written for it (`tools/band_score.py`, `tools/hammer_score.py`, `tools/set_metrics.py`…), and what it moved elsewhere was found later: the release-ring gain of the treble cost its mf sustain at 3 s. The bench measures everything at every change, with one fixed set of measures, and against real pianos only. The commercial modelled piano used so far as a reference is itself a model: its biases would become ours. It is still measured, as information, and never enters the targets.

## Running it

    cargo xtask bench                       # build, render, measure, score
    cargo xtask bench --preset some.toml    # another instrument
    cargo xtask bench --name trial --reuse  # measure out/bench/trial/notes again
    cargo xtask bench --save-baseline       # make this run the reference for the next ones

`cargo xtask bench` runs `tools/bench.py run`. It renders every third key from A0 at velocities 45, 56 and 100 (the layers 4, 8 and 13 of the references, pp/mf/ff: the velocities BiVib's Disklavier recorded), each note held 20 s, one note per render, then measures the notes and `piano-survey`'s partial analysis. It also renders the intervening keys, held 1.5 s, in `harmonic_notes/`: the local harmonic comparison covers all 88 keys at all three velocities. The historical register score continues to use only `notes/`, so the additional keys do not change its sampling or its baseline.

The model is heard as the references were recorded: close microphones in a studio (`presets/listening/Microphones/close.toml` and `Room/studio.toml`; `--listening <file>` to choose others, `--listening none` for the engine's own). The engine's default listening, a concert hall heard from 1.5 m, put its reverberation in every decay and between the partials: the first runs, made with it, scored 1.15 where the close microphones give 1.66, and some of their findings were the hall's (see the results below).

It writes `out/bench/<name>/scorecard.md` (the summary, local harmonic alerts, changes against the baseline, then every register measure), `result.json`, and `harmonic_profiles.csv`. It prints the historical score followed by the local alert and coverage counts. Historical score example:

    score 1.60, 74 % within the real pianos, 23 off — attack 1.05, brightness 2.34, dynamics 0.56, sustain 2.38, decay 1.23, tuning 0.51

## Scoring

Each measure is averaged per register (bass A0–B2, middle C3–B4, treble C5–B6, top C7–C8) and compared with the mean of the real pianos. The gap is counted in units of their spread:

z = (model − target) / max(spread, floor)

A gap as large as the one between two real pianos is therefore not a fault. The floor (3 dB for most levels, 0.15 for log ratios) keeps a measure on which the references happen to agree from becoming impossible to meet. |z| ≤ 1 is within the real pianos, ≤ 2 is close, beyond is off. A target needs at least two real pianos (one alone has no spread, and the floor would judge the model against that recording's quirks), except `dyn`, which only BiVib's measured velocities give. The score is the RMS of all z (lower is better), also given per family and per register.

The baseline, `bench/baseline.json`, holds the measures of the run saved with `--save-baseline`; they are scored against the current references, so that adding a reference does not break the comparison. Each run lists the measures whose |z| moved by 0.5 or more, worse first. A change is accepted when it improves what it targets without a list of new reds elsewhere; the baseline is saved again with the commit that keeps it.

## Harmonic profiles, note by note

The local comparison retains each key, velocity layer and time window separately. It measures partials **2–8 relative to partial 1**, in dB, at the attack (0–60 ms) and in the sustain (0.5–1 s). Since specification v2, the channels' FFT powers are averaged rather than summing their waveforms; the onset uses their RMS amplitude too. Both operations preserve a note whose channels oppose in phase. The same search windows and 12 kHz/noise exclusions apply to the model and recordings. Missing recordings and unreadable partials remain missing, never zero. A weak fundamental can raise all the ratios: the warning identifies a spectral imbalance, not its physical cause or a perceptual diagnosis of “nasality”. Since 2026-10-30 the historical register metrics average the channels' powers too (`channel-power-v2`).

For each component, the targets are real pianos with a recording of **that exact key and layer**. No neighbouring-key interpolation is used, and sources marked `info` never enter the targets. At least two usable reference values are required. The target is their mean; z uses their standard deviation with a 4 dB minimum tolerance. Layers 4/8/13 correspond to the model's MIDI velocities 45/56/100, but only BiVib's velocities are calibrated to that exact scale; the other pianos supply corresponding soft/medium/loud layers. Their own variation remains in the tolerance.

Specification **v3** adds eightfold FFT zero padding: the short attack's narrow
H1 search band no longer falls between FFT bins on A0, A♯0, B0, D♯1, E1 and
G♯1. This samples the existing spectrum more finely; it does not improve the
physical resolution of a 60 ms window or guarantee that weak bass H1 is
separable from H2. All eight harmonic reference caches were regenerated.
Profiles are measured with four workers. Full recordings are retained because
the onset threshold depends on their global peak and attacks may start after
1.2 s; a fixed prefix trim would change the measurement.

After a specification change, partial `refs NAME=directory` updates are refused
before either reference file is modified. First run `refs --harmonics-only`
with all target corpora available. If that full measurement fails, the old
harmonic cache is preserved. Once the cache matches, partial updates keep the
other targets.

Each note/layer/window is classified independently:

- **OFF**: at least one usable partial has |z| > 2.
- **WATCH**: at least two partials deviate in the same direction by both |z| > 1 and at least 6 dB. This catches a coherent spectral tilt that no single component would flag strongly.
- **ok**: neither warning rule fires and at least two partials can be compared. This means no alert under these rules, not that every partial is within one spread.
- **insufficient**: fewer than two partials can be compared and no OFF component is present.

These thresholds apply to the entire keyboard, not specifically to D#4. The scorecard lists all 88 keys with six cells each; alerts cannot cancel against the opposite error on another key. Each cell shows the worst |z| component and its signed deviation in dB. `harmonic_profiles.csv` includes every component, its value, target, deviation, z, reference count and named reference values. `result.json` also preserves raw profiles, scores, coverage and alert counts. The historical RMS score is unchanged; local alert counts are an additional diagnostic printed on every run.

The scorecard also checks each real piano against the other real pianos, excluding it from its own targets. These held-out checks trigger many alerts too: individual-note spectra vary much more than register averages, and some keys/layers have only two reference pianos. The thresholds therefore surface local discrepancies; they are not a calibrated pass/fail test of audible realism. The report makes the reference alert counts and comparable coverage visible alongside the model's results.

`bench/harmonic_references.json` stores the measured reference profiles, without audio, with a versioned measurement specification. `tools/bench.py refs ...` updates the selected sources' profiles along with their historical measures. To rebuild only the profiles of all registered target sources from their original note sets:

    python3 tools/bench.py refs --harmonics-only

A normal run needs this checked-in JSON, not the reference recordings. Missing or outdated reference specifications stop the run with the refresh command. `--reuse` reads the existing long and short note folders without rendering; on an older run that has only every third key, the missing keys are explicitly reported as insufficient. Run again without `--reuse` to obtain full coverage.

`--save-baseline` saves the raw harmonic profiles too, but refuses to overwrite
the baseline if any recording is missing (including an old `--reuse` directory
without the intermediate keys). Re-render without `--reuse` first. Unreadable
partials in present recordings remain explicitly insufficient. Subsequent runs
rescore profiles against current references, report the comparable coverage
and missing baseline recordings, and count newly alerted and cleared profiles
only within that coverage. Older baselines or different harmonic specifications
display “no harmonic profiles saved yet”.

Tests of cancellation, velocity separation, sparse references, missing/noisy values, gain invariance, time windows and target filtering:

    python3 -m unittest discover -s tools -p 'test_bench.py'

### Weak partials, single-source targets and the resonance measures (2026-10-30)

Three of the 13 measures left off by the tuning below were the bench's, not the model's, and the ear's "rounder resonance" was not measured at all. Protocol `channel-power-v3`; all eleven sources measured again (the harmonic references did not change). Every figure below measures the same renders (the default at `ccfd57d`, identical to `b93b27a`), so only the bench differs.

**Weak partials.** The register means average dB over the keys, and a partial lost in the noise was left out. The recordings read 4 of their ~3100 partial levels below −80 dB re partial 1, none below −95; the model's renders, which have no noise, read 12 of ~510 between −80 and −120 dB (spectral notches of the strike), and lost a few others. One key's partial read at −110 or left out moved the register's mean by 10 dB or more: the top's mf attack partial 4 (C7, D#7, F#7) went from z −1.1 to −2.5 on a change that did not touch F#7. Partials below −80 dB now count at −80 dB, and so does a partial lost in a noise that lies below −80 dB (it is at most there); one lost in a noise above the floor stays unread, as before. Applied alike to the recordings and the model.

**Single-source targets.** A target held by one real piano has no spread, so the floor alone judged the model against that recording: the top's 3 s envelope (mf and pp) was read on one source only (the model at the −60 dB floor against its −53). A target now needs two real pianos, except `dyn` (only BiVib's velocities are measured).

| same renders | score | within | off | bass | middle | treble | top |
|---|---|---|---|---|---|---|---|
| v2 (before) | 1.094 | 73.3 % | 13 | 0.85 | 0.92 | 0.79 | 1.68 |
| v3 floor + two sources | 1.052 | 74.1 % | 8 | 0.85 | 0.92 | 0.84 | 1.54 |
| v3 with the resonance measures | 1.143 | 72.6 % | 11 | 0.85 | 0.94 | 0.92 | 1.76 |

What moved with the floor and the two-source rule (model, target, z):

| measure | v2 | v3 |
|---|---|---|
| top attack_mf p4 | −73.9 / −66.3, −2.48 | −62.9 / −68.9, +1.27 |
| top attack_ff p3 | −34.5 / −45.1, +2.23 | −34.5 / −45.9, +1.75 |
| top attack_ff p4 | −49.2 / −59.8, +1.56 | −59.5 / −62.5, +0.33 |
| top sustain_ff p4 | −72.4 / −60.8, −2.06 | −75.0 / −71.5, −0.34 |
| treble sustain_ff p12 | −67.1 / −66.3, −0.13 | −65.8 / −72.9, +1.75 |
| treble attack_mf p6 | −57.4 / −56.5, −0.20 | −54.5 / −61.3, +1.06 |
| top env_mf 3 s, env_pp 3 s | −60 / −53 (one source), −2.33 | not scored |

The targets moved as much as the model: the clean recordings' lost top partials now count at the floor too (top sustain p4: −60.8 → −71.5). The treble's p12 and p6 now read the model 1–2 z too strong, which v2 hid.

**Resonance.** The two axes piloted in `docs/realism_axes.md` enter the score as the `resonance` family: `between` (axis 1, every register) and `lines` (axis 2, filed under the top). Ported to channel power, and with one correction: a partial was taken as found when it stood 8 dB above the median of its own ±1.5 % window; a partial a few bins wide (beating unisons) raised that median, was missed, and its energy counted as between the partials (the model's F#4 read +12 dB, D#3 +8; the pilot, which took medians over the keys, hid it). Prominence is now judged against a ±5 % window (at least ten bins). The pilot also subtracted a noise spectrum, from before the onset or else from the quietest 0.3 s after it; only Liang's notes have the 0.2 s of silence it needs, and the quietest stretch after the onset still holds the note, so a model note cut to 1.6 s read 7 dB cleaner in its late window than the same note held 20 s (review of PR #24). Nothing is subtracted now, on the model or the recordings: the attack window reads the same on every set (±0.1 dB) and the sustain window within 1.3 dB, except the top's, which sits on the recordings' noise (up to +5 dB) and is left out. The real pianos read cleaner with it too: up to 4 dB in the bass and up to 12 dB in the middle (Salamander, BiVib, Helgesson).

| measure | model | real pianos (8) | z |
|---|---|---|---|
| bass between early / late | −24.6 / −25.8 | −19.9 to −23.6 / −21.8 to −26.3 | −0.81 / −0.50 |
| middle between early / late | −32.1 / −29.8 | −22.9 to −29.5 / −23.0 to −34.4 | −1.79 / −0.19 |
| treble between early / late | −33.4 / −34.1 | −20.1 to −30.4 / −23.3 to −31.9 | **−2.58** / −1.60 |
| top between early | −20.2 | −4.4 to −11.9 | **−4.12** |
| top lines | −43.3 | −21.3 to −29.3 | **−4.35** |

The model is as clean as the cleanest real piano in the bass and middle, cleaner than any in the treble's attack, and 12 dB cleaner in the top, whose fixed lines sit at the detection limit. This is the same fault as the top's 1–2 kHz bands (the undamped strings' halo, "Tuning the default" below), now measured where the ear hears it in every register; the score rises from 1.05 to 1.14 because the bench sees more of it, not because the model changed. Off after v3: the top's 1–2 kHz bands (4), `between` top and treble early, `lines`, the bass sustain p6/p12, `hf_even` 8–11k max and the top's early T60 of partials 5–12.

A fresh render of `b93b27a` gives every measure the same z to the digit; it is saved as the baseline (1.14, 11 off, 343 OFF / 61 WATCH profiles).

Tests: `python3 -m unittest discover -s tools -p 'test_*.py'` (the floor, a partial lost above and below it, the two-source rule, a halo raising `between`, `between` with 0–220 ms of silence before the onset and cut right after its windows, a line shared by three keys).

### Tuning the default on the off measures (2026-10-30)

Control: the default at `e08fb06` scores 1.17, 74 % within, 16 off, 342 OFF / 65 WATCH profiles, the saved baseline to the digit. Every trial below is a full `bench.py run` with a copy of `presets/default.toml` differing in the named keys only (same close microphones, studio and velocities); an unchanged copy reproduces the control exactly. z are against the control's `result.json`.

**Kept** (one trial each, then together):

| Trial | Score | Off | What it targets, z before → after | Moved ≥ 0.5 elsewhere |
|---|---|---|---|---|
| `losses.winding` 2e-4 → 1e-4 | 1.163 | 15 | bass sustain bands 1k −2.16 → −1.90, 2k −0.52 → −0.04, 8k −1.82 → −0.97 | none |
| `hardness_slope_top` 0.2 → 0.15 | 1.151 | 14 | top mf attack p2 +3.24 → +1.25, p3 +2.26 → +1.26, 4–8 kHz share +2.00 → +1.08; ff p3 +2.48 → +2.20 | top mf attack p4 −1.12 → +3.14 (artefact, below); ff p4 −0.06 → +1.62; late T60 p1–4 +0.60 → +1.42 |
| `unison.hit_spread` 0.2 → 0.3 | 1.143 | 15 | `hf_even` max 4–8k +2.35 → +1.78, 8–11k +4.83 → +3.77; middle 1.03 → 0.92, treble 0.85 → 0.80 | top mf p2/p3 +0.3/+0.6 |
| the three together (default) | **1.094** | **13** | bass 0.91 → 0.85, middle 1.03 → 0.92, treble 0.85 → 0.79, top 1.78 → 1.68; profiles 343 OFF / 61 WATCH | top mf attack p4 −1.12 → −2.48 (artefact) |

- **The winding.** 2e-4 was set against Liang's Disklavier alone, whose bass is darker than the other grands' (docs/bass_metal.md: at 4e-4 the bass fell under them). 1e-4 keeps half of the copper's rubbing loss that stopped the metallic ring and lets the bass's sustain above 1 kHz back towards the eight grands' mean; 5e-5 goes a little further (bands 1k −1.76, 8k −0.51, score 1.159) and is left as an option, nearer 0 than what the user heard.
- **The top hammers.** The top's mf attack had partials 2–3 about 10 dB over the real grands' and its 4–8 kHz share 8 dB: the felt is too hard for the speed (axis 4 of docs/realism_axes.md: the real top's partials 3–5 grow slower with velocity than the fundamental). 0.15 voices C7–C8 a little softer; the top's mf balance moves +0.03 → −0.70 z. 0.17 does half as much (p2 +2.09).
- **The unison's evenness.** A felt never meets the three strings of a key equally: ±30 % of force from one string to the next at velocity 100 instead of ±20 %, growing at soft blows as before (docs/release.md). Each string's contact then lasts its own time, and the wave returning from the agraffe meets them at different moments: the second, short impact that makes D#4–F4 fizz is less coherent. The same unevenness feeds the aftersound: the treble's and the top's pp/mf envelopes at 1–2 s come up 0.3–0.7 z. 0.4 goes further on `hf_even` (4–8k max +0.70, 8–11k max +3.25, middle 0.84, treble 0.76) but puts the top's mf attack p2/p3 at +4.0/+3.6 alone and +2.5/+2.2 with the softer top hammers (score 1.120, 14 off, 346 OFF profiles).

**The top's mf attack p4 is not a measure of the model.** It averages partial 4 over C7, D#7 and F#7 (A7's and C8's lie above 12 kHz). The model's F#7 has nothing at 11.8 kHz (−94 to −113 dB re p1), and whether that is read or falls under the 12 dB noise margin decides the average: −69.7 dB in the control with it, −56.5 without (`hardness_slope_top` 0.15 alone), −74 with −113 (the kept default). C7 and D#7 stay at −48 to −59 dB against a target of −66 in every run: the model's top p4 has been too strong all along, and the control's −1.12 hid it. The flips of ±3 z on this one component follow F#7, not the instrument.

**Rejected:**

| Trial | Score | Off | Why |
|---|---|---|---|
| `unison.width_treble` 5 → 7 cents | 1.174 | 17 | top 2 kHz attack band +0.9 z, but its 2 kHz sustain band −0.4, ff attack p4 +2.0, mf p2 +0.3 |
| `hardness_slope_bass` 0.02 → 0.01 | 1.220 | 17 | the slope runs to C4: `hf_even` 8–11k max +4.83 → +6.47 and rms new off; bass sustain unmoved |
| `p_bass` 0.3, 0.5 (with the kept three) | 1.094, 1.099 | 13 | bass sustain p6/p12 unmoved (±0.02 z) |
| `unison.height_spread` 0.1 → 0.3 mm | 1.320 | 24 | `hf_even` max 8–11k +3.32, 4–8k +0.59, top 1–2 kHz bands +0.7 z; eleven new reds in the treble and top (attack p2–p4, sustain p2–p3, balance) |

`hf_even` alone, rendered C3–B4 at mf (rms/max 4–8k, rms/max 8–11k; control 5.4/16.7, 7.9/18.8): `contact_width_scale` 1.5 5.4/16.9, 8.1/20.0; `hysteresis` 0 5.7/15.1, 7.5/20.7, 0.4 5.8/16.5, 7.1/17.1, 0.8 4.5/11.2, 8.6/18.7; `p_offset` +0.5 4.8/13.0, 7.4/20.1; `shank_loss` 0.3 4.4/9.3, 7.8/24.3; `height_spread` 0.2/0.5 mm 6.2/11.6, 6.5/14.3 and 6.1/15.6, 9.5/26.0. None removes it: they move the coincidence of contact end and returning wave from key to key. The hammer's own force says where the highs come from: its 8–11 kHz share is −83 dB of its spectrum at C4, −73 at D4, −64 at D#4, −62 at F4, −66 at F#4 (mf, `strike_force_dump`). At D#4–F4 the contact (3.2–3.4 ms) outlasts the period (2.9–3.2 ms) by little: the force has fallen to 0.2–0.5 N when the wave comes back, rises again to 1–2.5 N within 0.3 ms and stops within 0.15 ms. A stiffer shank in the middle evens it ([felt.md](felt.md#the-fizz-of-d4f4-a-stiffer-shank-in-the-middle)).

**Not moved: the top's sound below its fundamental** (bands 1 and 2 kHz, z −4.5 to −2.8). In the sustain it is the undamped strings' halo and nothing else: with the sympathy off, the 1 kHz sustain band falls from −65 to −127 dB (2 kHz: −41 → −80); without the frame, the rim or the mechanical noises it moves by less than 0.2 dB. At C7 the halo is D#6's, E6's and F6's fundamentals at −82, −83 and −66 dB re the attack's strongest band (F6 is driven at its partial 3 by C7's partial 2), where the real grands' 1 kHz band totals −43. Rendered C7–C8 (bands at the attack/in the sustain, 1k/2k, dB; control −48.6/−28.7, −65.3/−41.0):

| | attack 1k | attack 2k | sustain 1k | sustain 2k |
|---|---|---|---|---|
| `sympathetic.gain` 1.0 | −45.6 | −26.1 | −60.9 | −38.2 |
| `damper.highest_note` 84 (24 keys free) | −47.9 | −28.7 | −65.1 | −40.9 |
| Overkill (damped strings listen) | −46.1 | −28.0 | −64.5 | −40.9 |
| `impedance_slope` 0 | −48.2 | −28.7 | −64.8 | −40.5 |
| `noise.keybed` × 10, 3 or 1 ms | −48.5 | −28.7 | −65.4 | −40.9 |
| `noise.keybed` × 100, 0.5 ms (C7, F#7, C8 only) | +11 dB | +5 dB | +12 dB | +4 dB |
| real grands | −24.2 | −12.0 | −43.4 | −29.4 |

What rings the halo is the knock at the bridge: the hammer's quasi-static push, β·F (12 % of it at C7, docs/top_treble.md), and the keybed. A hundred times the keybed's force, 30 N per m/s of hammer speed in 0.5 ms into the board at the key's place, gains only 10 dB. Through the board as modelled the knock would need forces no key or hammer gives. The halo of a real grand is rung some other way, which the model has not: the 88 % of the hammer's push that goes into the agraffe or capo bar, then the frame, which is bolted to the rim that holds the board (the frame drives nothing in the model); the duplex segments behind the bridge. That is a change to the engine's structure, not to the preset.

Also left: the top's pp/mf envelope at 3 s (−60 dB, the floor, against −53 on a single readable source, Codex's note in bench_bias_correction.md), the top's sustain p4 (−72 against −61), the bass's sustain p6/p12 (−9 and −12 dB re p1 against the grands, already 6–20 dB short at the attack on D#1–A#1 and F2–B2: docs/bridges.md, the bass bridge's place).

### The fizz of D#4–F4 (`hf_even`, 2026-10-30)

The user heard D#4, E4 and F4 "fizz" (*friser*), and no other key (2026-10-29). The register measures could not see it: they play every third key (C4, D#4, F#4), and none of them compared a key with its neighbours at the attack.

The cause is the hammer. The wave it launches comes back from the agraffe one period later and strikes it again. On D#4–F4 the main contact lasts about one period (2.7–3.1 ms): the hammer has just left the string when the wave returns, and it takes a second, short and hard impact. Below, it is already away; from F#4 up, it is still pressing the string when the wave arrives. The short impact puts the high partials into the string. The examples `strike_shape_scan` (contacts per key) and `strike_force_dump` (the force against time) show it.

`hf_even` reads every key from C3 to B4 at mf: the references' own keys, the model's from `notes/` and `harmonic_notes/` (the harmonic profiles' renders, held 1.5 s). On a recording, a band within 6 dB of its noise (before the onset, else its last second) is not read. The model's renders have none, and what follows their attack depends on how long the key is held (20 s in `notes/`, 1.5 s in `harmonic_notes/`): they are read whole, whatever the hold. Below C#4 the model's attack has almost nothing at 8–11 kHz (−105 dB of its energy). The real grands that can be read there are Helgesson and Splendid in both bands and BiVib below 8 kHz (its 8–11 kHz is noise). They keep each key within 4–5 dB of its neighbours at 8–11 kHz (rms 1.7–2.7 dB), within 9–10 dB at 4–8 kHz. The model, at `03ddb1f`:

| | model | real grands | z |
|---|---|---|---|
| 4–8 kHz rms | 5.4 dB | 4.5–5.4 | +0.2 |
| 8–11 kHz rms | 8.0 dB | 1.7–2.7 | +1.9 |
| 4–8 kHz largest gap | 16.7 dB (F#4 −16.7, F4 +10.1) | 9.4–9.9 | +2.4 |
| 8–11 kHz largest gap | 18.8 dB (F4 +18.8, C4 −17.5, A#3 +12.0, E4 +11.4) | 3.9–4.7 | +4.8 |

F4 stood 24 dB above its neighbours on 2026-10-29, under the mono protocol and before the treble hammers changed (048955e); it stands 19 dB above them now. The second impact is still there (above: D4 and D#4 have two contacts at velocity 70). With `hf_even` the score is 1.18 (1.12 without), 15 measures off; no other reference measure moved. First written on 2026-10-29 (`823d60a`, not merged then) with its own `notes_even/` renders and the mono protocol; carried onto the channel-power protocol here.

### Channel power for the register measures (2026-10-30)

All eleven sources were measured again with `channel-power-v2`, Plath from its fixed-gain import (`out/bench/sets/plath-calibrated`). Of the harmonic references only Plath's changed, by its import. The default model at `6951313` renders byte for byte as the run that saved the previous baseline; its scores under the two protocols:

| Protocol | Score | Off | Bass | Middle | Treble | Top | Harmonic OFF |
|---|---|---|---|---|---|---|---|
| mono sum (previous baseline) | 1.10 | 12 | 0.95 | 0.72 | 0.97 | 1.67 | 348 |
| channel power (this baseline) | 1.12 | 13 | 0.92 | 0.77 | 0.84 | 1.77 | 348 |

The measures that move most are those the mono sum hid. The bass ff attack's p2 goes from 6.3 to 10.5 dB against p1 (target 12.4): the model's bass partials partly cancelled between the two close microphones. The top's mf attack p2 goes from −29.2 to −25.3 dB (target −35.0), and its 4–8 kHz share and sustain p2 rise too: what the sum took off the model's top was too much already. Alerts: treble ff balance, treble ff sustain p3 and the top's early T60 of p5–12 go; the bass 1 kHz sustain band, the top's mf attack p2 and its pp/mf envelopes at 3 s come. The 3 s envelopes rest on a single source, as Codex noted (bench_bias_correction.md).

The bass bridge variants of docs/bridges.md, measured again from their renders:

| Bass bridge | Score | Bass | Middle | Treble | Top | Harmonic OFF |
|---|---|---|---|---|---|---|
| 8 keys, F1 (before) | 1.10 | 0.80 | 0.94 | 0.78 | 1.70 | 359 |
| placed by string length | 1.14 | 0.98 | 0.79 | 0.84 | 1.77 | 353 |
| Steinway D proportions (default) | 1.12 | 0.92 | 0.77 | 0.84 | 1.77 | 348 |
| old ends bowed +5 cm | 1.09 | 0.72 | 0.78 | 0.87 | 1.77 | 334 |

The ranking stands; the default no longer gains on the historical score (1.10 → 1.12): the middle's gain is offset by the bass, the treble and the top.

### Review validation, stereo profiles v3 (2026-10-29)

All eight reference collections and the 528 profiles of the existing `4a00f00`
render were remeasured after the FFT fix. No recordings are missing; 19
profiles have insufficient usable comparisons. The six formerly empty bass
attacks now compare all seven upper partials at all three velocities. The
summary is 359 OFF, 63 WATCH and 87 ok: these are spectral warnings, not
counts of audible defects. D♯4 remains OFF in the sustain at all velocities.

### First complete run, superseded mono profiles (2026-10-29)

The numbers in this subsection describe specification v1, retained to document the initial finding. Do not compare them with the current v3 local profiles: mono summing suppressed E4's H2 by about 29 dB in the sustain. The original graph based on those profiles is likewise superseded. Rebuilding references and rerunning `--reuse` corrects this without rendering again; older harmonic baselines must be measured again before comparison.

On the default model at commit `4a00f00`, the historical score remains **1.11**. The 528 local profiles (88 × 3 × 2) have no missing recordings; 36 have insufficient usable partial comparisons, split equally between the lowest and highest octaves. D#4 (MIDI 63) is flagged in the sustain at all three velocities:

| MIDI velocity | status | p2 deviation | p4 deviation | p6 deviation |
|---|---|---|---|---|
| 45 | OFF | +16.1 dB | +24.2 dB | +18.3 dB |
| 56 | WATCH | +16.8 dB | +23.0 dB | +16.6 dB |
| 100 | WATCH | +14.8 dB | +19.1 dB | +13.1 dB |

Values are deviations from the exact-key reference mean, each partial relative to p1, at 0.5–1 s. D#4 has five reference pianos at 45/56 and seven at 100. This exposes the imbalance that the middle register's average hid, without asserting that all notes above D#4 share the same cause. The complete local report flags many other keys as well (348 OFF profiles, 54 WATCH, on 87 keys); use the per-note evidence and the held-out reference checks, not this count as a count of audible defects.

### Separating overtone shape from decay

`tools/harmonic_diagnostic.py --run out/bench/harmonic_local --keys 60 61 62 63 64 66 67 68 69` compares C4 through E4 and F#4 through A4 at all three velocities and writes `harmonic_diagnostic.json`. These nine keys are the default; C4/C#4 provide nearby listening controls. It uses the stereo-power extractor for both model and real recordings. Its complementary views are:

- **Overtone shape:** each harmonic peak relative to the sum of the peak powers H2–H6 in the same window. H1 does not enter the denominator. All five anchor partials must be measurable, or the profile is unavailable. H7/H8 and H1 can be inspected against that same anchor. This is a distribution of spectral peak powers, not an integrated full-band energy or loudness measure.
- **Decay:** each partial relative to the strongest H1–H8 peak in the first 60 ms, a single fixed anchor for the whole recording. A falling fundamental cannot lift all the later measurements. Recording gain cancels.
- **Own-partial decay (`decay_self`):** each partial relative to its own peak in the first 60 ms. Every usable curve begins at 0 dB, isolating its loss over time from the initial spectral balance. If that partial is unreadable at the attack, its whole curve is unavailable. This is the default view for the fundamental hypothesis; the earlier common attack anchor remains selectable.

Add `--prototype-run out/bench/experimental_b3b2808` to include an independently rendered experimental engine. Both run directories provide `result.json` provenance and `notes/` or `harmonic_notes/` WAV files in the bench naming convention. Missing requested model notes are errors. The output separates `models` from real recordings; neither model may enter the real-piano mean or min–max band.

The 2026-10-29 comparison uses main `4a00f00` and the `b3b2808` Rayleigh integral prototype from `codex/piano-main-revalidation`, with the retained per-key contacts and verified `upstream-4a00f00/close` bundle. Their base presets match: close microphones, studio room and Disklavier playback velocity law, 48 kHz, full quality, 128-frame blocks. The prototype renderer removes its known 128-frame latency. Main was rebuilt independently: all 27 WAVs match the earlier graphed files byte for byte. The prototype covers nine keys × three velocities, held 1.5 seconds; the plotted windows end at 1.2 seconds. Render commands, binary and bundle hashes are retained under `out/bench/` in `render_comparison.py` and each run's `result.json`. No application settings or production defaults were changed for this comparison.

That experimental study is now kept on a separate archive branch; see the
[results summary and immutable archive links](fundamental_sustain.md). Its
historical measurements use the earlier spectral specification, while new
diagnostic exports identify v3 and FFT padding ×8 in their metadata. The
autonomous diagnostic can analyze externally supplied prototype WAVs; it
does not require or ship the experimental renderer.

Windows cover 0–60 ms, 60–200 ms, then 200 ms steps through 1.2 s; a separate 0.5–1 s window matches the bench's sustain. The diagnostic retains individual reference pianos, so means and min–max ranges can be computed after normalising each recording separately. It does not rank nasality. A useful perceptual detector still requires listening labels on additional notes and velocities, with separate examples for validation; this small group alone is not enough to fit and validate it.

At velocity 100, H1's loss from its own attack to the 0.6–0.8 s window is 5.1 dB on C4, 11.8 on C#4, 46.0 on D4, 25.4 on D#4 and 39.1 on E4. The corresponding real-piano means are 15.0, 15.0, 10.7, 13.2 and 16.5 dB. The sharp change starts at D4, whose fundamental drops more than those of the two notes described as nasal: fast fundamental loss is a measurable lead, but cannot alone explain the reported perceptual ordering. These windowed levels are not monotonic decay fits or estimates of T60. D4–E4's fundamentals sit on a cluster of three board modes ([board_hump.md](board_hump.md)).

## The measures

| Family | Measure | What | Floor |
|---|---|---|---|
| attack | `attack_ff`, `attack_mf` | partials 2, 3, 4, 6 re partial 1, first 60 ms | 3 dB |
| attack | `hf_ff`, `hf_mf` | share of the energy at 2–4 and 4–8 kHz, first 60 ms | 4 dB |
| brightness | `bright_ff`, `bright_mf` | spectral centroid / f0, first 50 ms | ×1.16 |
| brightness | `bands_att`, `bands_sus` | octave bands 1–8 kHz re the strongest band at the attack, ff, 0.4 s windows at 0.05 and 0.6 s (8 kHz left out from C5: recording noise) | 3 dB |
| dynamics | `dyn` | peak ff − pp and ff − mf | 3 dB |
| dynamics | `balance` | the register's attack peak above 250 Hz re the middle's, ff and mf (median over the keys) | 2 dB |
| sustain | `evenness` | key to key: rms of the levels of partials 1 and 2 (0.5–1.5 s, ff) about their trend (median of the keys within ±6), C2–C7, filed under the middle | 1.5 dB |
| attack | `hf_even` | key to key: share of 4–8 and 8–11.3 kHz in the first 60 ms (mf) about its trend (median of the other keys within ±6), rms and largest gap over every key from C3 to B4, filed under the middle; a band within 6 dB of a recording's noise is not read (the model's renders have no recording noise and are read whole); the model's keys between the bench's come from `harmonic_notes/` | 3 dB |
| dynamics | `brightening` | centroid ff / pp at the attack | ×1.16 |
| sustain | `sustain_ff` | partials 2, 3, 4, 6, 8, 12 re partial 1, 0.5–1 s | 4 dB |
| decay | `env_ff`, `env_mf`, `env_pp` | level above 0.7·f0 at 0.3, 1, 2, 3 s re the peak | 3 dB |
| decay | `t60_ff` | median T60 of partials 1–4 and 5–12 (early decay) and 1–4 (late decay), `piano-survey` | ×1.28 |
| tuning | `inharm`, `stretch` | median inharmonicity B (log10), tuning re equal temperament (cents) | 0.15, 5 c |
| resonance | `between` | energy between the partials re the energy on them, 300 Hz–5 kHz, ff, 0.08–0.6 s and 0.6–1.5 s, no noise subtracted (the top's 0.6–1.5 s, on the recordings' noise, is left out; `docs/realism_axes.md`, axis 1) | 3 dB |
| resonance | `lines` | the instrument's fixed lines: median level (dB re the strongest partial) of the three strongest off-partial peaks (0.1–0.5 s, 0.8–6 kHz) shared by at least three of the C6–C8 bench keys, a missing one at the −45 dB detection limit; filed under the top (axis 2) | 4 dB |

Levels below −60 dB re the peak count as −60 dB, partials below −80 dB re partial 1 count as −80 dB (and so does a partial lost in a noise below that floor), and a level within 6 dB of a recording's own noise (its quietest frames after the peak) is not read. Partials above 12 kHz are left out of the partial measures, and so is a partial standing less than 12 dB above the median of its search window (the recording's noise between the partials), when that window spans at least eight resolution cells: the real top notes' sustain (partials 3–8 at about −50 dB) and the treble's partial 8 sat on the recordings' noise, which the model was scored against (treble partial 8 of the sustain: z −4.2 → −1.0; score 1.10 → 1.04). The references are cut to the model's 20.5 s before their decays are fitted. The decays are fitted down to 70 dB below each partial's maximum at most (`piano-survey --range-db`), the recordings' usable range (58–73 dB, median per set): a render has no noise floor, and its partials fell to digital silence, where the late fit took the voice's end. On the top notes, without a damper, this made the late T60 swing from 1 s to 4 s on renders 0.1 dB apart (fit errors near 100 dB).

## The references

`bench/references.json` holds the measures of each reference (no audio). It is rebuilt from note sets laid out as `<note>v<layer>.{wav,flac}`, by one tool per kind of source:

| Tool | Source |
|---|---|
| `tools/split_notes.py` | Iowa, Salamander |
| `tools/sfz_set.py` | any multisampled piano described by an SFZ file: the regions it plays at velocities 45, 56 and 100, at each key it recorded, pedal up, first round robin |
| `tools/bivib_set.py` | BiVib: the samples recorded at velocities 45, 56 and 100 |
| `tools/maps_set.py` | MAPS isolated notes: M and F, pedal up |
| `tools/plath_set.py` | Plath's anechoic measurements: the ears of the dummy head, median take |

First import Plath with a common calibrated gain (float WAV, median take,
manifest retaining the source pressure and selected take):

    python3 tools/plath_set.py <Plath CSV directory> out/bench/sets/plath-calibrated

Then measure all sources (11 sets):

    tools/bench.py refs iowa=out/compare/iowa:nodyn salamander=out/compare/salamander:nodyn \
        headroom=out/bench/sets/headroom:nodyn splendid=out/bench/sets/splendid:nodyn \
        helgesson=out/bench/sets/helgesson:nodyn liang=out/bench/sets/liang:nodyn \
        plath=out/bench/sets/plath-calibrated:nodyn bivib_grand=out/bench/sets/bivib_grand \
        bivib_upright=out/bench/sets/bivib_upright:info maps_close=out/bench/sets/maps_close:info:nodyn \
        commercial=out/compare/commercial:info

| Source | Role | Instrument | Recording | Licence |
|---|---|---|---|---|
| `iowa` | target | Steinway model B, pp/mf/ff | microphones 20 cm above the strings | free use |
| `salamander` | target | Yamaha C5, layers 4/8/13 | close stereo pair | CC BY 3.0 |
| `headroom` | target | Yamaha C3 (Headroom Piano, Bengt Nilsson) | close pair | CC BY 4.0 |
| `splendid` | target | Steinway (Splendid Grand Piano, AKAI) | — | public domain |
| `helgesson` | target | Yamaha CF-3 (Maestro Concert Grand, Mats Helgesson), every key | KM84 X/Y pair | free use |
| `liang` | target | Yamaha Disklavier grand, every key at velocity 96 (ff only) | spaced pair | CC BY 4.0 |
| `plath` | target | concert grand (Plath, Hamburg), every key at one dynamic (ff only) | anechoic chamber, dummy head at the player | CC BY 4.0 |
| `bivib_grand` | target, **the dynamics** | Yamaha Disklavier grand DC3 M4, lid open, recorded at velocities 45/56/100 | binaural at the player | CC BY-NC-SA 4.0 |
| `bivib_upright` | info | Yamaha Disklavier upright DU1A | binaural at the player | CC BY-NC-SA 4.0 |
| `maps_close` | info | Yamaha Disklavier upright (MAPS ENSTDkCl), M/F at MAPS's own velocities | close microphones | CC BY-NC-SA 2.0 FR |
| `commercial` | info | a commercial modelled piano | played note by note in a sequence, at velocities 40/60/100 | — |

The two uprights are measured but do not enter the targets of a grand.

**Dynamics** (`dyn`, ff − pp and ff − mf) are compared only with BiVib's grand, the one source whose layers are measured MIDI velocities (`nodyn` on all the others). Splendid and Helgesson are normalised per layer (ff − pp ≈ 0 dB), Iowa's pp/mf/ff are the player's, and the sample libraries map their layers to velocities as their authors chose. With a single source the tolerance is the floor, 3 dB. `tools/velocity_bench.py` compares the whole velocity curve, below.

The Osiris Piano (Yamaha C2) is recorded with the una corda only: it is kept for a later una corda measure.

## Results

With close microphones, against the eight real grands, after the pedal changes ([pieces.md](pieces.md)): score 1.60, 74 % of the measures within the real pianos, 23 off. By register the RMS z is 0.64 (bass), 0.70 (middle), 1.57 (treble) and 2.79 (top): the bass and the middle are within the real pianos almost everywhere, the dynamics too (velocity 45 to 100, 2–3 dB short of the Disklavier).

What the real pianos agree on, and the model does not:

- **Top treble (C7–C8)**:
  - below the fundamental, the real pianos' 1 kHz band sits 17–33 dB below the attack's strongest band at the attack and 34–54 dB in the sustain; the model has almost nothing there (−55 and −105 dB), and 7–13 dB too little at 2 kHz. The knock of the hammer, soundboard and frame;
  - partials 1–4 die out about twice as slowly as on the real pianos (T60 0.6 s against 0.2–0.4 s; the late decay 3.0 s against 0.6–1.4 s), while partials 2–4 of the sustain are 11–35 dB weaker re partial 1.
- **Treble**: at mf, partials 2–4 of the attack are 7–10 dB too strong; in the sustain, partials 6–12 are 15–29 dB too weak.
- **Middle**: the 8 kHz band of the sustain 9 dB low.

With the hall, the bass's 8 kHz attack band and 1 kHz sustain band were off, and the top's upper partials looked too strong in the sustain: those were the hall's reverberation. With close microphones they are within the real pianos, or the other way round.

## Velocity curve

    tools/velocity_bench.py "<BiVib>/Grand/binaural samples/grand-open"

The model plays every third key at BiVib's ten velocities (12 to 111), heard with close microphones, and the peak level is compared, re velocity 100 (dB):

| Register | Source | 12 | 23 | 34 | 45 | 56 | 67 | 78 | 89 | 100 | 111 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| bass | Disklavier | −18.3 | −13.7 | −11.5 | −9.5 | −7.4 | −5.3 | −3.4 | −1.8 | 0 | +2.0 |
| bass | model | −16.2 | −13.4 | −11.0 | −8.8 | −6.8 | −5.0 | −3.2 | −1.6 | 0 | +1.6 |
| middle | Disklavier | −22.4 | −16.4 | −13.5 | −11.2 | −8.4 | −6.0 | −3.9 | −1.9 | 0 | +2.0 |
| middle | model | −20.0 | −16.4 | −13.4 | −10.8 | −8.4 | −6.1 | −4.0 | −1.9 | 0 | +1.8 |
| treble | Disklavier | −31.5 | −23.4 | −18.2 | −14.3 | −9.8 | −7.4 | −3.9 | −1.5 | 0 | +2.3 |
| treble | model | −30.6 | −24.5 | −19.6 | −15.4 | −11.9 | −8.7 | −5.6 | −2.7 | 0 | +2.3 |
| top | Disklavier | −30.2 | −26.8 | −21.4 | −17.3 | −13.0 | −10.0 | −5.5 | −2.9 | 0 | +2.7 |
| top | model | −37.4 | −32.4 | −28.0 | −23.4 | −18.8 | −14.3 | −10.1 | −4.6 | 0 | +3.5 |

The model is shown with the velocity law fitted on these measurements (below). With the earlier law, an exponential from 0.6 to 4.2 m/s, its curve was a straight line in dB, 1.4–2.8 dB per 11 velocity steps, and its pp at velocity 12 was 7–11 dB louder than the Disklavier's: the Disklavier's curve falls faster at the soft end.

**The fit.** `tools/velocity_fit.py` renders every third key at hammer speeds from 0.1 to 10 m/s, which gives the model's level against the speed, then searches the law v = vmin·(vmax/vmin)^(t^curve), t = (velocity − 1)/126, whose level re velocity 100 follows two Disklavier grands: BiVib's (ten velocities) and Liang's (isolated notes at 49, 80 and 96). The speed at velocity 100 is kept (2.8 m/s), where the felt and the brightness were set. The result is 0.26–4.4 m/s with curve 0.75 (`velocity.curve`, new): 2.4 dB rms over both pianos against 3.5 for the earlier law, and 2.4 for the best exponential. From the bass to the treble the curve above is within 1–2.5 dB of BiVib's. The two Disklaviers differ by 20–25 % in their mid-range slope (velocity 49 to 96, Liang: 9.0/11.5/15.8 dB, bass/middle/treble; BiVib 45 to 100: 9.4/10.7/14.6 dB), and the fit sits between them. Within performances the level now rises 1.1/2.0/2.7 dB per 10 velocity steps on the model against 1.8/3.0/3.2 on MAESTRO's Disklaviers (it was 0.9/1.6/2.2).

The attack's brightening with velocity follows without being fitted: at velocity 12 the bass centroid is 51 % below that at 100 (Disklavier 52 %, earlier law 45 %), the middle 43 % (42 %).

**What it costs.** At mf the hammer is slower than before (velocity 56: 1.19 m/s instead of 1.40), and the attack darker. In the bass the share of 4–8 kHz at the attack was already 15 dB below BiVib's at the same velocity, it is now 30 dB below: the felt is too soft at low speeds, the next item.

The top treble is steeper than the Disklavier at low velocities (−37 dB at velocity 12 against −30): its hammer, not the map.

The recorded performances agree ([pieces.md](pieces.md)): within a piece, the level rises 2 to 3 times as fast with velocity on a Bösendorfer SE and on a Disklavier as on the model.

### Recorded performances: a second touch

On the notes a Disklavier plays back (BiVib, Liang's isolated notes) the model follows the level partial by partial (middle, re velocity 100: partial 1 −13.8 dB at velocity 23 against −13.9, −9.2 at 45 against −9.6). Within the performances a pianist recorded on Disklaviers (Liang's pieces, 30 MAESTRO performances), the onset level of partial 1 gains 1.4–1.6 times as much per velocity step as the model's on the same notes. Middle, re each performance's notes at velocity 50–70:

| velocity | 30–44 | 75–89 | 90–104 |
|---|---|---|---|
| Liang, recording / model | −6.4 / −4.6 | +6.3 / +2.8 | +11.2 / +2.4 |
| MAESTRO, recording / model | −7.7 / −5.1 | +5.5 / +3.7 | +9.5 / +5.6 |

The velocity an instrument records under the fingers is a measurement, on another scale than the one it plays back. A single law cannot follow both. The default now follows the fingers (a keyboard played live, recorded performances); the played-back scale is `presets/touch/disklavier_playback.toml`, which the bench, `tools/velocity_bench.py` and `tools/pedal_bench.py` apply, their references being notes a Disklavier played back.

`presets/touch/recorded_performances.toml` is that law, the default since 2026-10-24 (chosen after a second A/B on MAESTRO and Liang's pieces; within Schubert's performance, middle notes at velocity 30–45 sit 1.2 dB lower, loud treble notes 0.4 dB higher). `tools/velocity_fit_pieces.py` fits it on the performances, keeping velocity 60 at the default law's speed (1.29 m/s, the mf timbre unchanged): 0.205–5.6 m/s, curve 0.78. The level against the performances is 2.0 dB rms, 2.2 with the default law. It is applied over an instrument as a listening preset is (`unsampled-piano-render --listening presets/touch/recorded_performances.toml`), and was chosen after an A/B on MAESTRO.


## Channel-power protocol details

`tools/bench.py` calls `load(..., stereo=True)` and measures
`(|FFT(L)|² + |FFT(R)|²)/2`. Mono recordings are duplicated without changing
their energy. Onsets use the RMS across channels; time-domain levels and
filtered envelopes average the channel energies. Partial ratios use the square
root of the pooled power to retain the existing amplitude-dB and SNR conventions.
The Rust survey tracks peaks on pooled spectra and combines heterodyned channel
energies **before** fitting each decay. It does not average independently fitted
T60 values. ESPRIT is phase-sensitive and cannot be combined with
`--channel-power`; its existing mono diagnostic remains available.

Legacy callers of `bench.load()` retain their mono default. This migration is
for the isolated-note bench; the separate piece/pedal scripts and older scratch
analyses have not been silently migrated. References are trimmed to 20.5 s as
float WAV to preserve the fixed-gain import. Survey diagnostics are retained
beside each CSV in `.csv.log` files.

Validation:

    python3 -m unittest discover -s tools -p 'test_*.py'
    cargo test -p piano-analysis
