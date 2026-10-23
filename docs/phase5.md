# Phase 5 — mechanical noises, presets, real time, calibration

Status as of 2026-10-21.

## Mechanical noises (`params::NoiseLaw`, `engine.rs`, `voice.rs`)

No samples are used. Each noise is an impulse injected into an already modelled structure, which filters and radiates it physically:

| Noise | Mechanism | What sets it |
|---|---|---|
| Key bottoming | 3 ms half-sine into the soundboard, at the key's bridge point (keyboard → rim → soundboard) | proportional to hammer velocity |
| Damper drop | Momentum impulse at the damper point: each mode k of each vertical string receives Δq̇ = φ_k·I/M, projected onto the coupled modes | MIDI release velocity; for the pedal, pedal speed |
| Damper rail | Half-sine into the soundboard, at the middle of the keyboard, when the pedal moves; on release, the drops of all the dampers are added | pedal speed |

The damper drop excites the strings, which then pass it on to the soundboard: the noise thus takes on the colour of the strings being stopped. A listening string receives the impulse in its sympathetic bank.

**Levels** (`cargo run --release -p piano-core --example noise_levels`), isolated by the difference between a render with noises and one without, relative to a mezzo-forte C4:

| Noise | Level |
|---|---|
| Key bottoming | −25 dB |
| Damper drop | −25 dB |
| Rail (pedal pressed) | −26 dB |
| Rail (pedal released, about 70 dampers falling back) | −17 dB |

- These levels are *tuned*, not measured: the literature places touch noises 20 to 30 dB below the note. A first version transmitted all of the felt's momentum to the string (+4 dB above the note), which is unrealistic since the energy is mostly dissipated in the soft felt.
- The finger–key contact, which makes a “struck touch” audible (Goebl 2014), is not modelled. The key does not exist as a body.

## Presets (`piano-host`)

- The whole model (`PianoModel`) serialises to TOML, thanks to the `serde` feature of `piano-core`: the core stays dependency-free by default.
- A preset contains only what it changes: absent sections and fields keep the value of the default instrument.
- The velocity curve (hammer velocity from 0.4 to 4 m/s) is now part of the model.

```bash
cargo run --release -p piano-render -- dump-preset presets/default.toml   # all parameters
cargo run --release -p piano-render -- --preset presets/instruments/Character/honky_tonk.toml note out/c4.wav --note 60
```

Examples provided:

| Preset | What changes |
|---|---|
| `presets/default.toml` | instrument matched to Salamander, 132 lines |
| `honky_tonk.toml` | unison of 12 to 18 cents, hard felt, weak dampers |
| `baroque_415.toml` | A = 415 Hz, no stretched tuning, softer felt |
| `dry_close.toml` | microphones 40 cm from the soundboard (+9 dB), reduced sympathy |

The exposed parameters correspond to the setting families of a commercial modelled piano: pitch and stretched tuning, inharmonicity, unison width, felt hardness and exponent (voicing), velocity curve, string losses, soundboard (dimensions, stiffnesses, losses, bridge series stiffness), dampers (position, force, half-pedal, timing), sympathy, polarisation, noises, microphone position.

## Real-time application (`unsampled-piano`, crate `piano-play`)

- Audio: cpal 0.18 with the **native PipeWire** host by default (JACK and ALSA available with `--host`) and real-time priority for the audio thread.
- MIDI: midir. The first real port is chosen automatically, otherwise `--midi <name>`.
- Lock-free event queue (rtrb): the MIDI callback pushes, the audio callback drains the queue at the start of each buffer.
- The instrument is built on the main thread (≈ 0.8 s), after which the audio callback does neither allocation nor locking.
- The CPU load is displayed every second.

```bash
cargo run --release -p piano-play -- --list
cargo run --release -p piano-play -- --midi "SL STUDIO" --buffer 128
cargo run --release -p piano-play -- --demo        # built-in sequence, no keyboard
```

Measurements on the development machine (Ryzen 9 9950X, PipeWire, 48 kHz), demo sequence with pedal and 88 voices, sound muted:

| Host | Buffer | Underruns | Mean load | Max load |
|---|---|---|---|---|
| ALSA (via PipeWire) | 256 | many | 18 % | 36 % |
| ALSA (via PipeWire) | 512 | none | 17 % | 25 % |
| Native PipeWire + real-time priority | 256 | none | 17 % | 22 % |
| Native PipeWire + real-time priority | 128 (2.7 ms) | none | 17 % | 21 % |

The load is expressed as a fraction of the buffer duration. With the ALSA host, the underruns at 256 samples came from the ALSA → PipeWire bridge and the lack of real-time priority, not from the computation.

Current limitation: the events of a buffer are all applied at its start. The jitter is therefore at most one buffer (2.7 ms at 128 samples). midir's timestamps would allow each event to be placed sample-accurately.

## End-to-end calibration (`tools/calibrate.sh`)

The script takes any sample bank named `<note>v<layer>` and produces, in about 47 s for Salamander:
1. the survey of the reference: partials, B, decays, ESPRIT components, fitted laws, bridge conductance;
2. the model renders for the same notes and velocities, optionally with a preset;
3. the analysis of these renders with exactly the same tools;
4. the comparisons: dynamics, brightness, spectral slope, fluctuations, double decay.

```bash
tools/calibrate.sh data/salamander/Samples out/calib [presets/my_piano.toml]
```

The grid searches (`tools/grid_hammer.py`, `unison_grid.py`, `pol_grid.py`, `coupling_grid.py`, `board_grid.py`, `voicing_grid.py`) remain available to refit a group of parameters.

## What remains

- **Plugin** (CLAP, then VST3): the engine is already independent of any framework; it needs a wrapper with nice-plug or clack. For VST3, the Rust bindings impose GPLv3; CLAP can stay under a permissive licence.
- **Graphical interface** for the settings, and rebuilding the instrument off the audio thread when a parameter changes. For now, a new preset and a restart are needed.
- **Excess treble above 3 kHz** (phase 4, open).
- **Second reference piano** (Iowa MIS) to separate what comes from the model from what comes from the Salamander recording.
- **Blind comparative listening** (MUSHRA) against Salamander and the commercial piano.

## Listening feedback (2026-10-22): “very muffled, like a mute” and “release noise too bright”

First try at the keyboard (Novation SL Studio, `unsampled-piano --buffer 128`). The measurements found four causes, all fixed physically:

1. **Striking point on the wrong side.** The bridge is at x = 0 and the hammer struck at 0.12·L… *from the bridge*. The mode spectrum is identical (|sin(kπx0)| is symmetric), so no spectral calibration had noticed it. But during contact, the bridge received, almost statically, about 88 % of the hammer's push, instead of about 12 % when the strike is near the agraffe. The soundboard turned this into a low “boom”: 64 to 92 % of the attack energy fell between the partials, versus 1 to 6 % for Salamander. The striking point and the dampers are now measured from the agraffe.
2. **Infinite-baffle radiation.** A real soundboard radiates from both faces. The rear face must be added in antiphase (Rayleigh integral with an image at 0.6 m, gain 0.6, matched to the spectral slope): the acoustic short circuit attenuates the low modes. Slope error: from 4.7 to 3.8 dB.
3. **Soundboard losses constant with frequency.** Internal damping and radiation increase with frequency: η = 0.02·(1 + f/700 Hz). For heavily damped soundboard modes, the exact impulse response of the oscillator (factor 1 + iσ/ω) replaces the cosine approximation.
4. **Decays too short** (prompt ×0.7, aftersound ×0.67 in the middle; ×0.6 and ×0.5 in the treble). In phase 3, the passivity correction had tripled the horizontal coupling, and the intrinsic loss law from phase 2 was overestimated. Refit on the complete system (`tools/decay_grid.py`): horizontal coupling 0.03, cross coupling at the passivity limit (0.173), intrinsic losses ×0.45. Residual biases: middle ×1.03 and ×1.05; treble ×0.81 and ×0.74.

The hammer was then recalibrated on the *absolute* attack brightness; until then only ratios between velocities had been matched (`tools/attack_grid.py`): stiffness 0.35, exponent p − 0.5, voicing slope 0.02 per key. Attack brightness error (rms log of the model/Salamander ratio): from 0.62 to 0.31. Before, the bass was 2 times too bright and the treble 2 to 3 times too dull; now almost the whole keyboard falls between 0.7 and 1.3. Only the top octave remains dull (A7 at 0.4).

**Release noise too bright.** The damper drop was an instantaneous impulse, hence broadband. The felt now lands over 4 ms: the impulse spectrum is that of a half-sine, and the noise becomes a dull “thump”, 23 dB below the note.

**Una corda.** It does give a darker timbre and, in this version, a longer aftersound, as Weinreich predicts. Since the sustain recalibration ([sustain.md](sustain.md)), the aftersound gain depends on the note and is zero on average; the test is now called `una_corda_is_darker_and_keeps_its_sustain`. But the model does *not* reproduce the level drop of a real piano: striking 2 strings instead of 3 does not reduce the energy transmitted by the hammer in our model.

After these changes: output gain +6 dB (worst case at −3.2 dBFS); noises at −22 / −23 dB below the note; pedal release at −14 dB.

## Room and lid (2026-10-22)

Listening feedback: “the sound is still a bit dry”. This was expected: the model stopped at the two microphones, as in a perfectly absorbing room.

**Room** (`room.rs`, `[room]` section of the preset).
- A rectangular room. By default, a concert hall of 40 × 28 × 16 m (≈ 18,000 m³, T60 ≈ 2.0 s in the bass and 1.6 s in the treble, critical distance ≈ 5.7 m). At 1.5 m, the reverberation arrives about 12 dB below the direct sound, and the first echoes after 30 to 60 ms. Presets: `small_hall.toml` (12 × 8 × 5 m, the former default, perceived when listening as a “small, very reverberant room”) and `studio.toml` (treated room).
- The source is the radiated pressure referred to 1 m, estimated from the microphones.
- **Early reflections**: image sources up to order 3. Each reflection has its exact delay, an attenuation of √(1 − α) per wall and an absorption low-pass that strengthens with the number of reflections.
- **Reverberation tail**: network of 16 delay lines (Householder matrix), lengths spread around the mean free path 4V/S.
  - Two-band attenuation giving the Sabine T60: 1.1 s in the bass, 0.8 s in the treble (air absorption included).
  - The level is set physically: reverberant energy = (r/r_c)² × direct energy, with the critical distance r_c = 0.057·√(V/T60) ≈ 1.3 m.
  - With the microphones at 1.5 m, the reverberation arrives about 2 dB below the direct sound.
- Test: the rendered T60 falls between the two Sabine T60s.
- Cost: about 1 % of a core.
- The room is disabled in `piano-eval`, because the calibrations compare the instrument alone with the recordings; `--room` re-enables it.
- Preset `presets/listening/Room/dry.toml`: no room.

**Lid** (`lid_*` in `[soundboard]`).
- Reflecting rectangle hinged on the long right-hand side, open towards the audience.
- For each soundboard element, the image source is its mirror image across the lid plane. It counts if the reflection point falls on the lid and if the microphone is on the soundboard side.
- A microphone placed behind the lid is in its shadow (transmission 0.1, no diffraction).
- By default, the lid is removed: the microphones matched to Salamander hang above the soundboard.
- `presets/listening/Microphones/concert.toml`: lid at 35°, microphones at 1.6 m in front of the opening. For these microphones, the lid adds 2 dB on average (up to +4.7 dB in the bass), with direct/reflected interference dips specific to each note. Without diffraction at the edge of the lid, these dips are deeper than in reality.

**Not yet modelled: the rest of the piano body.**
- The rim (the curved side) and the bottom of the case: mainly a bass contribution. The soundboard is assumed to be clamped in a rigid rim.
- The cast-iron frame, excited by the string forces at the agraffe and at the hitch pins: faint metallic resonance.
- The air cavity under the soundboard: bass modes.

These three elements fall under the same method: modes, excited by the forces already computed and radiating to the microphones.
