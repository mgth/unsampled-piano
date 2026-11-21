# The bridge rocking

Status as of 2026-11-21. Off by default and in the Concert grand: measured, not heard.

## What it is

A string's sideways (horizontal) motion pulls the bridge across the strings at the height where they bear on its pins. Until now the engine summed everything a key's strings do into one vertical force at its bridge point: the vertical oscillators whole, the horizontal ones by `polarization.radiation`, the longitudinal force too. Every part of a partial then reached the microphones by the same path, and a partial kept the same left/right balance from its strike to its end.

On every real grand a strong partial wanders between the microphones while it sounds. With `soundboard.bridge_rocking` the sideways force also acts as a moment on the board: h·F, h = `bridge_height` (0.033 m, the strings above the board's mid-plane), which drives each mode of the built board by its slope across the bridge, h·∂φ/∂y (the strings run along x), where the vertical force drives it by φ. The vertical and the sideways parts of a partial then radiate through different modes, and as they beat their sum moves between the microphones.

It adds to the sideways motion's direct share (`polarization.radiation`, which stands for the bridge's asymmetries: side-draft, pins, bearing), and needs the built board (`structure`) with `exact_below`, where each key drives the low modes at its own bridge point.

## In the engine

- `ModalBank::process_outputs`: a third output (`out3_*`) from the same states in the same pass, on every ISA (test `third_output_matches_a_separate_bank_with_or_without_the_second`).
- `NoteModel::rock_input`: the coupled modes' third output carries the horizontal oscillators' force, whole (`wrock`), for the partials and the duplex front lengths.
- `Soundboard::bridge_slopes`: h·∂φ/∂y at the key's bridge point, a central difference over ±1 cm (the elements' size), for the built board's modes; 0 for the others.
- `board::LOW_INPUTS` = 2 × 88: the low bank's inputs are each key's force, then each key's moment (`LowDrive::rock_input`). Without the rocking the moments' inputs are zero and never active.
- `Voice::mix` adds the moment's force to its input.

Two simplifications, both small:

- **During the hammer's contact** (1–4 ms) the sideways motion is heard through its direct share only.
- **The strings' own sympathetic bank** cancels its key's vertical force at its bridge point, not the rocking's: a key's own moment reaches its own listening strings.

## What it changes

Off, the renders are those of before bit for bit (three notes at close microphones and a phrase with the room, against a build of the commit below). Tests `without_sideways_motion_the_rocking_changes_nothing` and `with_sideways_motion_the_rocking_is_heard`.

On, Concert grand, B0–B2, close microphones in the studio, velocity 100 (`docs/case_effect.md` for the measure):

| | before | rocking | six real grands |
|---|---|---|---|
| a strong partial's left/right wander, 0.1–2 s (dB std) | 0.1–0.3 | 0.7–2.8 | 1.1–3.3 |
| level | | +0.1 dB (−0.5 to +1.3) | |
| partials at 2–4 kHz, 0.8–1.6 s re their start | −16.5 dB | −14.2 dB | −9 to −15 |
| coherence between the microphones between the partials | 0.73–0.87 | 0.73–0.89 | |

The halo between the partials keeps its coherence: it does not come through the sideways motion.

**Cost:** Chopin's nocturne op. 9 no. 2 rendered on one core, alternated runs on a machine in use: off, as before (99.2 and 99.6 s, against 98.3 and 111.9 s for the build without it); on, 104.8, 104.9, 105.4 and 120.5 s, about +6 %. Every sounding key then drives one more input of the low bank.

## Heard

On a listening page of nine bass keys (B0–B2, ff and mf) and a bass phrase, against Splendid: "it does not come closer". The partials' wandering, which an offline transfer of Splendid's whole envelopes per microphone had carried, is not what made that transfer sound like it (docs/case_effect.md). Kept as a switch: it is physical, measured, and neutral when off.
