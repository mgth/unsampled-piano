# Test bench: scoring the model against real pianos

Status as of 2026-10-23.

Until now each change was measured with the script written for it (`tools/band_score.py`, `tools/hammer_score.py`, `tools/set_metrics.py`…), and what it moved elsewhere was found later: the release-ring gain of the treble cost its mf sustain at 3 s. The bench measures everything at every change, with one fixed set of measures, and against real pianos only. The commercial modelled piano used so far as a reference is itself a model: its biases would become ours. It is still measured, as information, and never enters the targets.

## Running it

    cargo xtask bench                       # build, render, measure, score
    cargo xtask bench --preset some.toml    # another instrument
    cargo xtask bench --name trial --reuse  # measure out/bench/trial/notes again
    cargo xtask bench --save-baseline       # make this run the reference for the next ones

`cargo xtask bench` runs `tools/bench.py run`. It renders every third key from A0 at velocities 40, 60 and 100 (the layers 4, 8 and 13 of the references, pp/mf/ff), each note held 20 s, one note per render so that no other string rings, then measures the notes and `piano-survey`'s partial analysis. About 35 s on a 16-core machine.

It writes `out/bench/<name>/scorecard.md` (the summary, the changes against the baseline, then every measure) and `result.json`, and prints one line:

    score 1.58, 61 % within the real pianos, 29 off — attack 1.63, brightness 1.93, dynamics 1.12, sustain 1.86, decay 1.36, tuning 0.66

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

`bench/references.json` holds the measures of each reference (no audio). It is rebuilt from note sets laid out as `<note>v<layer>.{wav,flac}`: `tools/split_notes.py` for the Iowa and Salamander sets, `tools/sfz_set.py` for any multisampled piano described by an SFZ file (the regions it plays at velocities 40, 60 and 100, at each key it recorded, pedal up, first round robin):

    tools/split_notes.py iowa data/iowa out/compare/iowa
    tools/split_notes.py salamander data/salamander/<samples> out/compare/salamander
    tools/sfz_set.py "<library>/Headroom Piano (NoFX).sfz" out/bench/sets/headroom --prefer close
    tools/sfz_set.py "<library>/Splendid Grand Piano.sfz" out/bench/sets/splendid
    tools/sfz_set.py "<library>/Maestro Concert Grand.sfz" out/bench/sets/helgesson
    tools/bench.py refs iowa=out/compare/iowa:nodyn salamander=out/compare/salamander \
        headroom=out/bench/sets/headroom splendid=out/bench/sets/splendid:nodyn \
        helgesson=out/bench/sets/helgesson:nodyn commercial=out/compare/commercial:info

| Source | Role | Instrument | Recording | Licence |
|---|---|---|---|---|
| `iowa` | target, no dynamics | Steinway model B, pp/mf/ff | microphones 20 cm above the strings | free use |
| `salamander` | target | Yamaha C5, layers 4/8/13 | close stereo pair | CC BY 3.0 |
| `headroom` | target | Yamaha C3 (Headroom Piano, Bengt Nilsson), 3 layers used | close pair | CC BY 4.0 |
| `splendid` | target, no dynamics | Steinway (Splendid Grand Piano, AKAI) | — | public domain |
| `helgesson` | target, no dynamics | Yamaha CF-3 (Maestro Concert Grand, Mats Helgesson), every key | KM84 X/Y pair | free use |
| `commercial` | info | a commercial modelled piano | played note by note in a sequence | — |

`nodyn` leaves a set out of the ff − pp measure: Splendid and Helgesson are normalised per layer (ff − pp ≈ 0 dB), and Iowa's pp/mf/ff are the player's, not MIDI velocities (ff − pp 22–27 dB). The measure then rests on Salamander and Headroom, whose layers are mapped to velocities by their authors; Disklavier recordings, whose MIDI velocities are measured, will replace them. The Osiris Piano (Yamaha C2) is recorded with the una corda only: it is kept for a later una corda measure.

## First run

On main at e921cf5, against the five real pianos: score 1.34, 68 % of the measures within the real pianos, 18 off. With two pianos (Iowa and Salamander) it was 1.58 and 29 off: several gaps were one piano against the other.

What the five pianos agree on, and the model does not:

- **Top treble (C7–C8)**, most of the off measures:
  - below the fundamental, the real pianos have 18–20 dB more energy at 1 kHz and 8–13 dB more at 2 kHz, at the attack and in the sustain (knock of the hammer, soundboard and frame). The commercial model has it (z ≈ 0);
  - the partials die out about twice as slowly as on the real pianos (T60 0.6 s against 0.2–0.4 s for partials 1–4; the late decay 2.5 s against 0.6–1.4 s);
  - ff − pp is 27 dB against 11–14, ff − mf 18.5 against 6–8;
  - at the attack, partial 3 is 12 dB too strong and partial 4 at mf 16 dB too weak; in the sustain, partials 2–3 are 10–19 dB too strong.
- **Bass**: the 8 kHz band of the attack is 16 dB above the mean (within the spread at 4 kHz), and the 1 kHz band of the sustain 8 dB below.
- **Treble**: the upper partials (5–12) decay a little too slowly (T60 0.6 s against 0.3–0.5).

The middle register and the decays of the bass and treble are within the real pianos. The gap noted earlier against the commercial piano in the sustain partials 8–12 of the middle is within the real pianos' spread: it was a gap to the commercial piano, not to real pianos.
