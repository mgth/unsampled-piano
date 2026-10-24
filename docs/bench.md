# Test bench: scoring the model against real pianos

Status as of 2026-10-24.

Until now each change was measured with the script written for it (`tools/band_score.py`, `tools/hammer_score.py`, `tools/set_metrics.py`…), and what it moved elsewhere was found later: the release-ring gain of the treble cost its mf sustain at 3 s. The bench measures everything at every change, with one fixed set of measures, and against real pianos only. The commercial modelled piano used so far as a reference is itself a model: its biases would become ours. It is still measured, as information, and never enters the targets.

## Running it

    cargo xtask bench                       # build, render, measure, score
    cargo xtask bench --preset some.toml    # another instrument
    cargo xtask bench --name trial --reuse  # measure out/bench/trial/notes again
    cargo xtask bench --save-baseline       # make this run the reference for the next ones

`cargo xtask bench` runs `tools/bench.py run`. It renders every third key from A0 at velocities 45, 56 and 100 (the layers 4, 8 and 13 of the references, pp/mf/ff: the velocities BiVib's Disklavier recorded), each note held 20 s, one note per render so that no other string rings, then measures the notes and `piano-survey`'s partial analysis. About 35 s on a 16-core machine.

The model is heard as the references were recorded: close microphones in a studio (`presets/listening/Microphones/close.toml` and `Room/studio.toml`; `--listening <file>` to choose others, `--listening none` for the engine's own). The engine's default listening, a concert hall heard from 1.5 m, put its reverberation in every decay and between the partials: the first runs, made with it, scored 1.15 where the close microphones give 1.66, and some of their findings were the hall's (see the results below).

It writes `out/bench/<name>/scorecard.md` (the summary, the changes against the baseline, then every measure) and `result.json`, and prints one line:

    score 1.60, 74 % within the real pianos, 23 off — attack 1.05, brightness 2.34, dynamics 0.56, sustain 2.38, decay 1.23, tuning 0.51

## Scoring

Each measure is averaged per register (bass A0–B2, middle C3–B4, treble C5–B6, top C7–C8) and compared with the mean of the real pianos. The gap is counted in units of their spread:

z = (model − target) / max(spread, floor)

A gap as large as the one between two real pianos is therefore not a fault. The floor (3 dB for most levels, 0.15 for log ratios) keeps a measure on which the references happen to agree from becoming impossible to meet. |z| ≤ 1 is within the real pianos, ≤ 2 is close, beyond is off. The score is the RMS of all z (lower is better), also given per family and per register.

The baseline, `bench/baseline.json`, holds the measures of the run saved with `--save-baseline`; they are scored against the current references, so that adding a reference does not break the comparison. Each run lists the measures whose |z| moved by 0.5 or more, worse first. A change is accepted when it improves what it targets without a list of new reds elsewhere; the baseline is saved again with the commit that keeps it.

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
| dynamics | `brightening` | centroid ff / pp at the attack | ×1.16 |
| sustain | `sustain_ff` | partials 2, 3, 4, 6, 8, 12 re partial 1, 0.5–1 s | 4 dB |
| decay | `env_ff`, `env_mf`, `env_pp` | level above 0.7·f0 at 0.3, 1, 2, 3 s re the peak | 3 dB |
| decay | `t60_ff` | median T60 of partials 1–4 and 5–12 (early decay) and 1–4 (late decay), `piano-survey` | ×1.28 |
| tuning | `inharm`, `stretch` | median inharmonicity B (log10), tuning re equal temperament (cents) | 0.15, 5 c |

Levels below −60 dB re the peak count as −60 dB, and a level within 6 dB of a recording's own noise (its quietest frames after the peak) is not read. Partials above 12 kHz are left out of the partial measures, and so is a partial standing less than 12 dB above the median of its search window (the recording's noise between the partials), when that window spans at least eight resolution cells: the real top notes' sustain (partials 3–8 at about −50 dB) and the treble's partial 8 sat on the recordings' noise, which the model was scored against (treble partial 8 of the sustain: z −4.2 → −1.0; score 1.10 → 1.04). The references are cut to the model's 20.5 s before their decays are fitted.

## The references

`bench/references.json` holds the measures of each reference (no audio). It is rebuilt from note sets laid out as `<note>v<layer>.{wav,flac}`, by one tool per kind of source:

| Tool | Source |
|---|---|
| `tools/split_notes.py` | Iowa, Salamander |
| `tools/sfz_set.py` | any multisampled piano described by an SFZ file: the regions it plays at velocities 45, 56 and 100, at each key it recorded, pedal up, first round robin |
| `tools/bivib_set.py` | BiVib: the samples recorded at velocities 45, 56 and 100 |
| `tools/maps_set.py` | MAPS isolated notes: M and F, pedal up |
| `tools/plath_set.py` | Plath's anechoic measurements: the ears of the dummy head, median take |

and the line that measures them all (11 sources, about 2 min):

    tools/bench.py refs iowa=out/compare/iowa:nodyn salamander=out/compare/salamander:nodyn \
        headroom=out/bench/sets/headroom:nodyn splendid=out/bench/sets/splendid:nodyn \
        helgesson=out/bench/sets/helgesson:nodyn liang=out/bench/sets/liang:nodyn \
        plath=out/bench/sets/plath:nodyn bivib_grand=out/bench/sets/bivib_grand \
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

The velocity an instrument records under the fingers is a measurement, on another scale than the one it plays back. A single law cannot follow both, and the default stays on the played-back scale.

`presets/touch/recorded_performances.toml` is a velocity law for MIDI recorded from pianists. `tools/velocity_fit_pieces.py` fits it on the performances, keeping velocity 60 at the default law's speed (1.29 m/s, the mf timbre unchanged): 0.205–5.6 m/s, curve 0.78. The level against the performances is 2.0 dB rms, 2.2 with the default law. It is applied over an instrument as a listening preset is (`unsampled-piano-render --listening presets/touch/recorded_performances.toml`), and was chosen after an A/B on MAESTRO.

