# Test bench: scoring the model against real pianos

Status as of 2026-10-24.

Until now each change was measured with the script written for it (`tools/band_score.py`, `tools/hammer_score.py`, `tools/set_metrics.py`…), and what it moved elsewhere was found later: the release-ring gain of the treble cost its mf sustain at 3 s. The bench measures everything at every change, with one fixed set of measures, and against real pianos only. The commercial modelled piano used so far as a reference is itself a model: its biases would become ours. It is still measured, as information, and never enters the targets.

## Running it

    cargo xtask bench                       # build, render, measure, score
    cargo xtask bench --preset some.toml    # another instrument
    cargo xtask bench --name trial --reuse  # measure out/bench/trial/notes again
    cargo xtask bench --save-baseline       # make this run the reference for the next ones

`cargo xtask bench` runs `tools/bench.py run`. It renders every third key from A0 at velocities 45, 56 and 100 (the layers 4, 8 and 13 of the references, pp/mf/ff: the velocities BiVib's Disklavier recorded), each note held 20 s, one note per render so that no other string rings, then measures the notes and `piano-survey`'s partial analysis. About 35 s on a 16-core machine.

It writes `out/bench/<name>/scorecard.md` (the summary, the changes against the baseline, then every measure) and `result.json`, and prints one line:

    score 1.15, 72 % within the real pianos, 20 off — attack 1.09, brightness 1.30, dynamics 1.06, sustain 1.11, decay 1.19, tuning 0.51

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
| dynamics | `brightening` | centroid ff / pp at the attack | ×1.16 |
| sustain | `sustain_ff` | partials 2, 3, 4, 6, 8, 12 re partial 1, 0.5–1 s | 4 dB |
| decay | `env_ff`, `env_mf`, `env_pp` | level above 0.7·f0 at 0.3, 1, 2, 3 s re the peak | 3 dB |
| decay | `t60_ff` | median T60 of partials 1–4 and 5–12 (early decay) and 1–4 (late decay), `piano-survey` | ×1.28 |
| tuning | `inharm`, `stretch` | median inharmonicity B (log10), tuning re equal temperament (cents) | 0.15, 5 c |

Levels below −60 dB re the peak count as −60 dB, and a level within 6 dB of a recording's own noise (its quietest frames after the peak) is not read. Partials above 12 kHz are left out of the partial measures. The references are cut to the model's 20.5 s before their decays are fitted.

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

## Results on main (e921cf5)

Against the eight real grands: score 1.15, 72 % of the measures within the real pianos, 20 off. With two pianos (Iowa and Salamander) it was 1.58 and 29 off: several gaps were one piano against the other.

What the real pianos agree on, and the model does not:

- **Top treble (C7–C8)**, most of the off measures:
  - below the fundamental, the real pianos have about 16 dB more energy at 1 kHz and 7–12 dB more at 2 kHz, at the attack and in the sustain (knock of the hammer, soundboard and frame). The commercial model has it (z ≈ 0);
  - the partials die out about twice as slowly as on the real pianos (T60 0.6 s against 0.2–0.4 s for partials 1–4; the late decay 2.5 s against 0.6–1.4 s);
  - ff − pp is 25 dB against 18 on BiVib's Disklavier, ff − mf 20 against 14;
  - at the attack, partial 3 is 14 dB too strong; in the sustain, partials 2–3 are 10–17 dB too strong.
- **Treble**: at mf, partials 3–6 of the attack are 10–12 dB too strong; partials 1–4 decay about twice as slowly at first (T60 1.3 s against 0.6–1.2 s).
- **Bass**: the 8 kHz band of the attack is 13 dB above the mean, the 1 kHz band of the sustain 9 dB below.
- **Dynamics, bass to treble**: ff − pp (velocity 45 to 100) is 7–12 dB on the model against 10–14 dB on the Disklavier, about 3 dB short.

The middle register and the decays of the bass are within the real pianos. The gap noted earlier against the commercial piano in the sustain partials 8–12 of the middle is within the real pianos' spread: it was a gap to the commercial piano, not to real pianos.

## Velocity curve

    tools/velocity_bench.py "<BiVib>/Grand/binaural samples/grand-open"

The model plays every third key at BiVib's ten velocities (12 to 111) and the peak level is compared, re velocity 100 (dB):

| Register | Source | 12 | 23 | 34 | 45 | 56 | 67 | 78 | 89 | 100 | 111 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| bass | Disklavier | −18.3 | −13.7 | −11.5 | −9.5 | −7.4 | −5.3 | −3.4 | −1.8 | 0 | +2.0 |
| bass | model | −11.3 | −9.8 | −8.4 | −6.9 | −5.5 | −4.1 | −2.8 | −1.4 | 0 | +1.4 |
| middle | Disklavier | −22.4 | −16.4 | −13.5 | −11.2 | −8.4 | −6.0 | −3.9 | −1.9 | 0 | +2.0 |
| middle | model | −13.1 | −11.4 | −9.8 | −8.1 | −6.5 | −4.8 | −3.2 | −1.6 | 0 | +1.6 |
| treble | Disklavier | −31.5 | −23.4 | −18.2 | −14.3 | −9.8 | −7.4 | −3.9 | −1.5 | 0 | +2.3 |
| treble | model | −20.0 | −16.9 | −14.1 | −11.5 | −9.1 | −6.8 | −4.5 | −2.2 | 0 | +2.0 |
| top | Disklavier | −30.2 | −26.8 | −21.4 | −17.3 | −13.0 | −10.0 | −5.5 | −2.9 | 0 | +2.7 |
| top | model | −35.8 | −32.5 | −28.9 | −24.8 | −20.3 | −15.6 | −10.0 | −4.2 | 0 | +3.3 |

From the bass to the treble, the model's curve is a straight line in dB, about 1.4–2.8 dB per 11 velocity steps. The Disklavier's is steeper at the soft end: from 34 down to 12 it loses another 7–13 dB where the model loses 3–6. Its pp is therefore 7–12 dB quieter than the model's. In the top treble the model is the steeper, 4–8 dB too quiet from velocity 23 to 78.

The attack also brightens more with velocity on the Disklavier: in the bass the centroid at velocity 12 is 52 % below that at 100, against 34 % on the model (treble and top at low velocities: the recordings' noise, not read).

The recorded performances agree ([pieces.md](pieces.md)): within a piece, the level rises 2 to 3 times as fast with velocity on a Bösendorfer SE and on a Disklavier as on the model.
