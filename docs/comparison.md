# Comparison with Salamander, the Iowa and a commercial modelled piano

Status as of 2026-10-22.

## Protocol

Each source plays the same series of notes: every third key starting from A0 (Salamander's 30 notes), at velocities 40, 60 and 100, filed like Salamander's layers 4, 8 and 13. The key is held for 4 s, and the recording is split into `<note>v<layer>.wav` files.

| Source | Instrument | Recording |
|---|---|---|
| Salamander | Yamaha C5, samples | sampler layers |
| Iowa MIS (`data/iowa/`) | Steinway model B, pp/mf/ff | microphones 20 cm above the strings |
| Commercial | commercial modelled piano, preset loaded, no reverb | played in real time by `unsampled-piano --midi-out "<port>"`, recorded by `jack_rec` |
| Model | offline render | no room |

The commercial piano only falls back to digital silence after the keys are released: there is therefore no reverb to subtract.

Tools, in order of use:
- `tools/note_sequence_midi.py` generates the sequence;
- `tools/split_notes.py` splits and converts;
- `tools/compare_sources.py` computes sustain, partial profile, brightness and dynamics;
- `piano-survey` measures inharmonicity and decays;
- `examples/hammer_stats.rs` gives the hammer contact.

## Findings

### 1. The hammer–string contact of the unisons was wrong (main cause)

The model's partial profiles barely moved when the felt hardness (×8.6), its exponent or its hysteresis were changed. Neither did the contact: 5 ms and 15 N for C4 ff, whatever the hardness. The literature (Askenfelt & Jansson, Chaigne & Askenfelt) gives about 2 ms and several tens of newtons.

The cause: the solver summed the displacements of the 2 or 3 strings of a unison, and the felt compressed this sum. The strings were therefore in series, whereas under a felt they are in parallel. The hammer saw strings N² times too compliant (×4 for two strings, ×9 for three). The calibration had compensated as best it could: hardness 0.35, exponent −0.5.

Fix (applied, see below): the felt presses on the mean displacement of the strings, each string receiving its share of the force. Resulting contact:

| Note | Duration | Force |
|---|---|---|
| C4 ff | 1.9 ms | 39 N |
| C4 pp | 2.7 ms | 8 N |
| A1 ff | 3.7 ms | 36 N |

The felt hardness takes effect again. Profile of the ff attack (partials 2 and 3 in dB relative to partial 1):

| Register | Salamander | Iowa | Commercial | Before | Fixed |
|---|---|---|---|---|---|
| bass | +6 / +3 | +9 / +14 | +4 / +3 | +2 / −6 | +10 / +8 |
| middle | −2 / −13 | +5 / −3 | −2 / −6 | −11 / −10 | −3 / −10 |
| treble | −19 / −26 | −22 / −34 | −17 / −26 | −13 / −20 | −20 / −26 |

The attack timbre now matches the references in all registers: the bass and middle get brighter, and the treble, previously too bright, softens.

Still to recalibrate:
- hardness, exponent and velocity map;
- level: +2 to +3 dB in the bass and middle, up to +8 dB at C6 ff;
- top treble at soft dynamics: C8 pp and mf lose 6 to 9 dB, and the ff − pp difference rises to +36 dB versus +13 to +25 dB in the references;
- una corda.

### 2. In the bass, the upper partials die out too quickly

Median T60 (s) at the start of the decay, ff, partials 1–4 / 5–12:

| Register | Salamander | Iowa | Commercial | Model |
|---|---|---|---|---|
| bass | 9.5 / 6.8 | 6.5 / 5.1 | 9.5 / 7.7 | **12.8 / 3.9** |
| middle | 2.6 / 2.3 | 2.0 / 1.3 | 2.9 / 2.6 | 2.1 / 1.3 |
| treble | 1.0 / 0.4 | 0.8 / 0.3 | 0.9 / 0.4 | 1.0 / 0.6 |

In the bass, the fundamentals last too long and partials 5 to 12 die twice as fast as they should. During the sustain (0.5–1 s), the fundamental therefore dominates by 10 to 20 dB: this is the “barrel-like” bass and the “soft” sound. Even once fixed, the contact does not make up this gap: the bass sustain profile stays between −13 and −25 dB on partials 4 to 12.

The frequency dependence of the bass string losses needs revisiting: the constant `b1_floor` is replaced by losses that increase with frequency for the wound strings. The bass bridge conductance between 150 Hz and 1.5 kHz also needs revisiting.

### 3. The timbre changes too little with dynamics

ff/pp attack brightness ratio:

| Register | Iowa | Commercial | Model (before or fixed) |
|---|---|---|---|
| bass | ×1.8 | ×2.1 | ×1.2 |
| middle | ×1.5 | ×1.8 | ×1.25 |

The playing sounds uniform. This is a matter for the felt exponent and hysteresis, to be recalibrated once the contact is fixed, against the Iowa and the commercial piano: Salamander itself has a flat ratio (×1.2).

### 4. What is already in place

- **Inharmonicity B**: 9e-5 in the bass, 2.5e-4 in the middle, 2e-3 in the treble, consistent with the three references.
- **Dynamics ff − mf**: 8 to 10 dB, between Salamander and Iowa, close to the commercial piano.
- **Bass and middle sustain** up to 1 s: within the range of the references.

### 5. Known gaps, not measured here

- End of treble notes between 1 and 3 s, shorter than Salamander and Iowa; the commercial piano is in between.
- Phantom partials and string non-linearities in loud playing.
- Duplex.
- Passive sympathetic coupling.

## Hammer recalibration (applied)

The parallel contact is in `hammer.rs`, without allocation: the mode iterator is traversed twice. The hammer was then recalibrated against all three references at once with `tools/hammer_grid.sh` and `tools/hammer_score.py`. The score is the normalised quadratic deviation from the mean of the references, with their spread as the tolerance, and at least 3 dB. It covers:

- the profile of partials 2, 3, 4 and 6 at the attack, in ff and in mf;
- the brightening of the attack from pp to ff;
- the dynamics ff − pp and ff − mf, compared with Salamander and the commercial piano only, since the Iowa is played in dynamic markings rather than velocities.

| | Score |
|---|---|
| before | 9.3 |
| contact fixed, old settings | 5.3 |
| chosen | 1.9 |

Chosen setting, about 80 combinations tried:

| Parameter | Before | Chosen |
|---|---|---|
| `hardness` | 0.35 | 0.7 |
| `hardness_slope` | 0.02 | 0.05 (harder hammers towards the treble) |
| `p_offset` | −0.5 | +0.5 (p from 4.2 to 5.5, within Stulov's range) |
| `hysteresis` | 1 | 0.25 |
| velocity | 0.3–5.5 m/s | 0.6–3.0 m/s |

An exponent of +1 or +2 gained 0.02 to 0.13 in score, but went outside the measured range.

Outputs brought down by −5.3 dB (`PRESSURE_GAIN` 1/22, `FORCE_GAIN` 1/440): the Chopin keeps its peak of −18.5 dBFS, the Campanella rises to −14 dBFS. Sustain unchanged (`envelope_compare.py`).

Resulting contact, as duration and peak force:

| Note | pp | ff |
|---|---|---|
| C4 | 2.4 ms, 11 N | 2.0 ms, 28 N |
| A1 ff | — | 4.1 ms, 25 N |
| C6 | 1.3 ms, 19 N | 1.1 ms, 45 N |
| A7 | 0.8 ms, 36 N | 0.5 ms, 95 N |

What remains after recalibration:

- **pp → ff brightening too weak**: ×1.14 in the bass and middle, versus ×1.2 to ×2.1 in the references (mean ×1.5). Tightening it would widen the dynamics further. Stulov's felt model alone does not give enough timbre change per decibel. Leads: two-layer felt (hard core, soft surface), or a stiffness law depending on compression.
- **Dynamics** slightly narrow in the bass and middle (6 to 7 dB for ff − pp, versus 10) and slightly wide in the top treble (16 versus 13).
- **Bass partial 4** 8 dB below the references.
- **Top treble in mf** too bright (partial 2 at −25 dB versus −35).

## Priorities for closing the gap

1. ~~Parallel unison contact and hammer recalibration~~: done.
2. ~~Bass string losses~~: largely done ([bass.md](bass.md)).
3. **Timbre change with dynamics**: the felt is done ([timbre_dynamics.md](timbre_dynamics.md)). The bass needs string non-linearity.
4. Only then, the physics additions: string non-linearities, duplex, end of treble notes, passive sympathy.
5. ~~Sustain after the first second~~: done ([long_sustain.md](long_sustain.md)).
6. ~~Radiation of bass fundamentals~~: done ([bass_radiation.md](bass_radiation.md)).
7. Sustain brightness: largely done ([brightness.md](brightness.md)); the middle at 4–8 kHz remains.
