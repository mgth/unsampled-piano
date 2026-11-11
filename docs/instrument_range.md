# A range of instruments from one engine

State as of 2026-10-23.

## Aim

The aim is to see how far the current engine stretches across the piano family. Two sets of presets were made and kept outside the repository:

- **Instrument families** built from physical data: concert grand, grand, baby grand, upright, compact upright, 19th-century grand, Viennese fortepiano.
- **Presets fitted on measurements** of the ten acoustic models of a commercial modelled piano.

## Engine additions

- `hammer_mass_scale`: multiplies Stulov's hammer masses (light leather-covered fortepiano hammers, small upright hammers).
- `strike_ratio`: strike point as a fraction of the speaking length up to C6, 0.05 closer to the agraffe at C8 (default 0.12, as before).

The defaults leave the sound unchanged.

## Tools

- `tools/fit_preset.py <survey.csv>` fits the inharmonicity law e^{a_bass·m + b_bass} + e^{a_treble·m + b_treble} and the stretch law on a `piano-survey` CSV of a recorded instrument, and prints them as preset sections. The topmost keys are left out of the stretch fit. On Salamander it gives back the calibrated laws: B at A0 2.2e-4, stretch −15 / +27 cents at A0 / C8.
- `tools/set_metrics.py [--json] name=dir[:components.csv] …` gives, per register, the attack brightness at mf and ff, the dynamics ff − mf, the ff sustain at 1, 2 and 3 s and, given ESPRIT components, the prompt T60 and the unison spread.

## Method

Each recording is a sequence of every sixth key at velocities 60 and 100, held 3.5 s. The commercial piano was played through the graph with its reverb, effects, limiter and equaliser off and its pedals up.

For each model:

1. **Size class** (concert grand, grand, upright): string scale, board and frame sizes.
2. **Inharmonicity and stretch** fitted on its survey.
3. **Voicing** interpolated between three anchors (mellow, normal, bright: top felt hardness, core hardness and depth, hysteresis) to match the middle register's ff brightness, then the bass core depth for the bass.
4. **Velocity range**: ff velocity kept, range widened towards pp to match the dynamics ff − mf.

## Recording the commercial piano

The plugin runs in Carla, whose OSC remote control (TCP port 22752, on by default) gives every parameter by name. The reverb, delay, equalisers and limiter can be on in a running instance, as they were on 1 November 2026, and loading a program again does not change them: switch them off and check before each take, since nothing in a recording says how it was made. `$PLUGIN` is the plugin's name in Carla and its node in the graph, `$OUT` the recording without its extension.

```bash
tools/carla_control.py dump "$PLUGIN" > "$OUT.before.txt"
tools/carla_control.py program "$PLUGIN" "<program name>"
tools/carla_control.py dry "$PLUGIN"
tools/note_sequence_midi.py --every 1 seq.mid 4.0 3.5 60 100 > seq.txt
jack_rec -f "$OUT.wav" -d 709 -b 24 "$PLUGIN:output_1" "$PLUGIN:output_2" &
timeout 707 target/release/unsampled-piano --name piano-rec --file seq.mid --midi-out "$PLUGIN"
wait
tools/split_notes.py seq "$OUT.wav" seq.txt "$OUT" 4.0
```

`dry` prints the switches it set, read back: all must read 0. The microphones, volume and every other parameter stay the program's. `--midi-out` sends the file's notes to the plugin through the engine's MIDI mirror, so the engine sounds on its own output during the take; only the plugin's outputs are captured. A keyboard connected to the plugin must stay silent during the take.

Carla does not report a change to the client that made it, so the tool reads the values back through a new client, once the first has unregistered. It can take longer than 3 s to answer on a loaded machine. The values Carla reports are not always the plugin's: it gave `Hammer Noise` as 0 before the 88-key take, while the take sounds as with the program's 0.286 and a take set to 0 is 13 dB quieter on the top keys. Only a measurement on the audio settles what a setting did.

**Checking a take.** A take that matches the old one proves nothing if the switches had no effect, so one short take with the effects on is the control: on A0, A2, A4 and A6 at velocity 100 (1 November 2026), the effects add 2–7 dB under 500 Hz and 3–15 dB above 1 kHz in the body (0.2–1.2 s), and A0's release tail 0.3 s after the key rises from −37 to −26 dB. The take of 23 October follows the dry control, not this one: its reverb, equalisers and limiter were off. Against the new take (`out/catalog_clean/steinway_d/`, all 88 keys at 60 and 100) it is about 2 dB quieter at every frequency, for a reason not traced; single notes differ by a few dB from take to take.

`out/catalog_clean/steinway_d_no_hammer_noise/` is the same take with `Hammer Noise` at 0, for comparing the strings and the board without it (docs/action_noise.md). It differs from the first only below about 2 kHz in the upper keys: by nothing up to B3, 3–4 dB under 500 Hz at C4–B5, 6–12 dB under 1 kHz at C6–B6, 10–14 dB under 2 kHz at C7–C8.

## What the engine reproduces

| | Fitted presets vs measurements |
|---|---|
| inharmonicity, A0–C8 | fitted law, rms 0.04–0.2 in log B (0.6 for the upright) |
| stretch tuning | rms 0.4–6 cents |
| middle attack brightness (ff) | within 0.3 |
| middle dynamics ff − mf | within 0.8 dB |

The measured models span:

- A0 inharmonicity from 0.8e-4 to 2.7e-4;
- bass stretch from −4 to −17 cents at A0;
- middle ff brightness from 1.9 to 2.9 (centroid / f0);
- dynamics ff − mf from 6.4 to 9 dB.

## What it does not reproduce yet

The same gaps showed for every model. They are limits of the engine, not of the presets:

- **Bright bass**: the bass ff brightness tops out near 10 (centroid / f0) whatever the felt settings; the brightest models reach 12–13.
- **Treble dynamics**: ff − mf is 9–12 dB against 7–8. A faster hardness growth towards the treble (`hardness_slope` 0.07) took 2 dB off.
- **Middle sustain at 3 s**: 3 to 11 dB shorter than measured.
- **Bass prompt decay**: 16–19 s (ESPRIT, partials 1–2) against 3.5–10 s.

## Lessons from the families

- **Lower tension sustains longer.** A string at lower tension feeds the bridge less, since the bridge loss rate goes as T·Re(Y)/L. Historical instruments only sound short with a much thinner, more mobile board and lossier thin wire.
- **The core sets the voicing.** Once the hammer reaches the hard core, the top felt hardness barely changes the brightness: brighter or mellower voicings come from the core depth and hardness and from the hysteresis.
- **A small board gives a slightly longer middle sustain.** The baby grand sustains a little longer than the 2 m grand: a smaller board has fewer modes, hence less coupling.
