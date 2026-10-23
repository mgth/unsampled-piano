---
title: Command line
description: The options of the standalone player and of the offline renderer, for playing, testing and rendering MIDI files to WAV.
order: 5
icon: terminal
---

## `unsampled-piano` — the player

```bash
unsampled-piano [options]
```

| Option | What it does |
|---|---|
| `--ui` | open the interface window (every parameter, presets, MIDI learn) |
| `--preset <file>` | instrument preset (TOML) — watched and reloaded when edited |
| `--presets <folder>` | the interface's preset folder (default: the preset's, else `./presets`) |
| `--list` | list audio devices and MIDI ports, then exit |
| `--midi <name>` | MIDI input port: a substring of its name (default: the first hardware port) |
| `--host <backend>` | `node` (Linux default: a PipeWire/JACK graph node with MIDI in and out), or a plain audio host without MIDI ports: `pipewire`, `jack`, `alsa`, `wasapi` (Windows default), `asio`, `coreaudio` (macOS default) |
| `--device <name>` | audio output: a substring of the device, or of the playback ports (node) |
| `--buffer <frames>` | buffer size (node: requested graph period); default 256 |
| `--output-channel <n>` | first channel of the stereo pair on a multichannel interface (from 1) |
| `--gain-db <dB>` | output gain |
| `--name <name>` | name of the graph node (default `unsampled-piano`) |
| `--midi-out <name>` | connect the MIDI mirror output to inputs whose name contains this (node) |
| `--file <files…>` | play standard MIDI files one after the other |
| `--from <s>` | start the first file at this time |
| `--demo` | play a short built-in sequence instead of listening to MIDI |

Anything you always pass can go in [`play.toml`](/docs/getting-started/#personal-defaults).

```bash
# Linux: low latency, the interface, a preset you are voicing in an editor
unsampled-piano --buffer 128 --ui --preset my_grand.toml

# play a MIDI file through the concert microphones
unsampled-piano --preset presets/listening/Microphones/concert.toml --file etude.mid
```

## `unsampled-piano-render` — offline rendering

Deterministic MIDI → stereo WAV, faster than real time, bit-identical from run to run. Useful
to compare settings, render a piece, or check a change.

```bash
unsampled-piano-render [--preset <file>] [--sr 48000] [--block 128] [--tail 3] <command>
```

| Command | What it renders |
|---|---|
| `midi <in.mid> <out.wav>` | a standard MIDI file, all tracks and channels merged |
| `note <out.wav> --note 60 --velocity 100 --hold 4` | one note, held for `hold` seconds |
| `sweep <out.wav> --velocity 90 --step 0.5 [--sustain]` | every key from A0 to C8, optionally with the pedal down |
| `dump-preset <out.toml>` | not a sound: the full default preset, every parameter |

From the source tree, both are `cargo run --release -p piano-play --` and
`cargo run --release -p piano-render --`.
