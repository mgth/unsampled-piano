# Unsampled Piano

<img src="assets/icon.svg" alt="" width="96" align="right">

Physically modelled virtual piano, real-time from MIDI, in Rust. No samples are used to produce the sound (recordings are used only for analysis and calibration).

**Website: [unsampled.mgth.fr](https://unsampled.mgth.fr)** ([en français](https://unsampled.mgth.fr/fr/))

- [Listen](https://unsampled.mgth.fr/#listen): pieces and comparisons rendered by the engine, not recorded
- [Download](https://unsampled.mgth.fr/download/) the standalone application and the CLAP/VST3 plugin for Windows, macOS and Linux
- [Guides](https://unsampled.mgth.fr/docs/): getting started, the interface, presets, MIDI, the command line, how it works
- [Parameter reference](https://unsampled.mgth.fr/docs/parameters/): every parameter, what it is on a real piano and how the model does it, with interactive diagrams (felt, strike point, inharmonicity, soundboard modes, lid, room…)

![The interface (`unsampled-piano --ui`), Strings page: the C4 string, its partials, and per-key losses, string length and unison spread over the keyboard](docs/img/interface.png)

Physical chain: felt hammer (passive quadratised contact) → 1 to 3 stiff strings per key, two polarisations, coupled at the bridge (Weinreich modes) → modal soundboard (clamped orthotropic plate) → Rayleigh radiation to two microphones (optional lid) → room (image-source reflections, reverberation). The cast-iron frame (excited at the agraffes) and the rim (excited by the soundboard edges) also radiate. On top of this come the dampers, the pedals (half-pedal, sostenuto, una corda), sympathetic resonance through the bridge and impulse-driven mechanical noises.

## Usage

```bash
# play: the piano becomes a node in the PipeWire graph (JACK API) with stereo
# output, MIDI input (sample-accurate timing) and a MIDI output mirroring what it
# plays; it connects to the default output and to the first keyboard
# (--midi / --device to choose, --host pipewire for the old cpal mode;
# personal defaults in ~/.config/unsampled-piano/play.toml, e.g.
# device = "Stereo DJ X-Fade:In B")
cargo run --release -p piano-play -- --list
cargo run --release -p piano-play -- --buffer 128 [--preset presets/listening/Microphones/close.toml | …/concert.toml]
# presets: presets/instruments/<family>/*.toml (an instrument) and
# presets/listening/{Microphones,Room}/*.toml (microphones and lid, room);
# --preset (or `preset` in play.toml) lays a listening or touch preset over
# the last sound, and starts with any other instead of it;
# in the interface (--ui), loading a listening preset keeps the instrument
# and loading an instrument keeps the listening set-up
# the preset is watched: when edited during play, the instrument is
# rebuilt in the background (≈ 0.1 s) without cutting held notes
# play MIDI files in sequence (sample-accurate timing; --from skips the beginning)
cargo run --release -p piano-play -- --buffer 128 --file data/midi/*.mid
# in the interface (--ui), the bar above the status bar opens a MIDI file
# (or takes one dropped on the window): play, pause, stop, and a slider to
# move through it; pausing or moving lets the keys and pedals go

# offline rendering (stereo WAV)
cargo run --release -p piano-render -- midi piece.mid out/piece.wav
cargo run --release -p piano-render -- note out/c4.wav --note 60 --velocity 90
cargo run --release -p piano-render -- dump-preset presets/my_piano.toml

# calibration against a <note>v<layer> sample bank
tools/calibrate.sh data/salamander/Samples out/calib

cargo test --workspace
```

In the interface (`--ui`), the **Audio and MIDI** page chooses the backend, the output, the channel pair, the buffer size, the MIDI input, its channel (`--midi-channel`, `midi_channel`), a sustain pedal working the other way round (`--invert-sustain`, `invert_sustain = true`) and (graph node) the MIDI mirror while the piano plays (the channel and the pedal apply to the keyboard, not to MIDI files): the node reconnects its ports, another backend or device restarts the audio with the instrument rebuilt at its sample rate (≈ 0.1 s), and a failed choice falls back to the previous set-up. The same page sets the **performance**: the threads computing the voices (`--threads`, `threads` in `play.toml`; the sound is the same bit for bit, two threads take about a third off the load) and the CPU quality (`--quality full|balanced|light`, `quality`), the three levels proposed in [docs/performance.md](docs/performance.md) for smaller machines, which leave parts of the model out without touching the presets; `overkill` drops every shortcut, even inaudible ones, as the reference the others are measured against ([docs/performance.md](docs/performance.md#overkill-the-reference-without-shortcuts)). An output above 48 kHz (88.2 to 384 kHz) gets the piano computed at 44.1 or 48 kHz and upsampled, for half the load or less and the same sound up to 20 kHz (`--full-rate`, `full_rate = true`, to compute it at the output's rate; see [docs/performance.md](docs/performance.md#outputs-above-48-khz)). Under **Application** it sets the interface's theme (`theme = "dark"` or `"light"`; by default as the system), starts the MCP server (below) and chooses whether the player starts with the last sound (`last.toml` in the configuration directory, saved once the sound has stayed unchanged for 2 s; `restore = false` to start with the default instrument). The choices are saved to `play.toml`; options given on the command line still take precedence at the next start.

The **Model** page draws the instrument as the model builds it, to scale: static with its numbers, live with the engine's state (keys, hammers, dampers, strings' energy, the soundboard's motion), or as an editor of its geometry (board, bridges, agraffes, frame, microphones) on the plan; in 3D on the GPU in the standalone player, plainly or in a realistic look (materials, a studio's lights, soft shadows, reflections) ([docs/model_page.md](docs/model_page.md)).

On Linux, `tools/install-app.sh` builds the player and installs it for the current user, with a menu entry (`installer/linux/unsampled-piano.desktop`, which opens the interface), its icon and the shipped presets, under `~/.local` (`PREFIX=/usr/local` for every user, as root). Saved presets go to `~/.config/unsampled-piano/presets`.

## Windows, macOS and Linux packages

The standalone player for each comes from the `Standalone` workflow of GitHub Actions (run it from the Actions tab, or push a `v*` tag). Opened from the Explorer or the Finder, it shows the interface and plays through the default output, with the first MIDI keyboard; from a terminal it takes the same options as on Linux, e.g. `--device`, `--output-channel` for a pair other than 1–2 on a multichannel interface.

- **Windows**: an installer, `unsampled-piano-setup-windows-x86_64.exe`, that offers the standalone application, the VST3 plugin and the CLAP plugin, each one optional, and the presets they share, for all users (`Program Files`, `Common Files\VST3` and `\CLAP`, `ProgramData\Unsampled Piano\presets`) or for the current user only, with a Start menu shortcut and an uninstaller (script: `installer/windows/unsampled-piano.iss`); or a zip with `unsampled-piano.exe`, which needs nothing else installed, and the presets beside it. Output through WASAPI, or `--host asio` (the buffer size is then the one set in the driver's control panel). Personal defaults go in `%APPDATA%\unsampled-piano\play.toml` (also `host = "asio"`).
- **macOS**: `Unsampled Piano.app`, universal (Intel and Apple silicon), output through CoreAudio, presets inside the bundle. It is not notarised: the first time, open it with a right click → Open (or `xattr -dr com.apple.quarantine "Unsampled Piano.app"`). Personal defaults go in `~/Library/Application Support/unsampled-piano/play.toml`; from a terminal, run `"Unsampled Piano.app/Contents/MacOS/unsampled-piano"`.
- **Linux**: an archive, `unsampled-piano-<version>-linux-x86_64.tar.gz` (built on Ubuntu 24.04: PipeWire and glibc 2.39 or later), with the player, the CLAP and VST3 plugins, the presets, the menu entry and `install.sh`: `./install.sh` for the current user (`~/.local`, `~/.clap`, `~/.vst3`), `sudo ./install.sh --system` for all users (`/usr/local`, `/usr/lib/clap`, `/usr/lib/vst3`), `--uninstall` to remove them. Arch Linux and derivatives: the AUR packages `unsampled-piano` (a release) and `unsampled-piano-git` (the latest commit), whose PKGBUILDs are in `installer/aur/`; `installer/linux/make-archive.sh` builds the archive locally.

Presets: the interface (standalone and plugins) lists the presets saved by the user, in `presets` of the configuration directory (where saving writes), then the shipped ones, read-only: beside the program or in the bundle, and where an installer puts them, `%PROGRAMDATA%` or `%LOCALAPPDATA%\Unsampled Piano\presets` on Windows, `/Library/Application Support/Unsampled Piano/presets` (or the user's own) on macOS, `/usr/share/unsampled-piano/presets` (`$XDG_DATA_DIRS`) on Linux. A saved preset of the same name hides the shipped one. Without the Windows installer, copying `presets` to one of those folders gives them to the plugins.

Building them locally: `cargo build --profile dist -p piano-play` on Windows (cross-compiling from Linux works with `--target x86_64-pc-windows-gnu`), `tools/macos-app.sh` on a Mac (the bundle, in `target/macos`). ASIO is the `asio` feature: it needs the Steinberg ASIO SDK (dual-licensed, GPLv3; `CPAL_ASIO_DIR`, else downloaded), libclang and MSVC, so cross-compiling it with MinGW does not work.

## Plugin

CLAP and VST3 instrument, "Unsampled Piano". The live parameters (output gain, pedal and damper behaviour, velocity curve, mechanical noises) are automatable host parameters; the rest of the instrument and the MIDI-learnt controllers are saved with the project, and editing them rebuilds the instrument in the background without cutting the notes.

Builds for Linux, Windows and macOS (universal) come from the `Plugin` workflow of GitHub Actions: run it from the Actions tab, or push a `v*` tag to attach the bundles to a release. Locally:

```bash
cargo xtask bundle piano-plugin --release   # target/bundled/Unsampled Piano.{clap,vst3}
tools/install-plugin.sh                      # Linux: into ~/.clap and ~/.vst3
cargo build -p piano-plugin && cargo test -p piano-plugin   # loads the .clap as a host would
```

## AI assistants (MCP)

`unsampled-piano --mcp` serves the playing instrument to AI assistants through the [Model Context Protocol](https://modelcontextprotocol.io) (HTTP, `http://127.0.0.1:7426/mcp`; `--mcp ADDR` for another address, `mcp = true` in `play.toml` to have it always on). The assistant changes the sound while you play, through the same controller as the interface: live parameters apply at once, the others rebuild the instrument in the background without cutting the notes. Tools:

| Tool | Does |
|---|---|
| `list_params` | parameters with their value, default, unit, range, live or rebuild (filter by word or group) |
| `get` | one parameter, with what it changes on the piano and how the model does it |
| `set` | changes one parameter; for a rebuild, answers once the new instrument plays |
| `load_preset` | loads an instrument (keeps the listening set-up) or a listening preset (keeps the instrument) from the preset folder, or a file |
| `status` | rebuild pending, last build time, voices, CPU load, last error |

The server listens on the loopback interface only by default and has no authentication: any program on the machine can reach it while it runs. With Claude Code:

```bash
claude mcp add --transport http unsampled-piano http://127.0.0.1:7426/mcp
```

## Reference performances

The files in `data/midi/` come from [MAESTRO v3](https://magenta.tensorflow.org/datasets/maestro) (Google Magenta, CC BY-NC-SA 4.0): Piano-e-Competition pianists recorded on a Disklavier, with the actual key velocities and pedals. `data/` is not under version control; to retrieve them:

```bash
curl -L -o data/maestro/maestro-v3.0.0-midi.zip https://storage.googleapis.com/magentadata/datasets/maestro/v3.0.0/maestro-v3.0.0-midi.zip
```

## Crates

| Crate | Role |
|---|---|
| `piano-core` | DSP engine, no dependencies (`serde` feature for presets) |
| `piano-host` | TOML presets, MIDI decoding |
| `piano-params` | parameter registry (paths, ranges, live or rebuild), MIDI learn, `PianoControl` trait |
| `piano-control` | control of a playing instrument: live parameters, background rebuild and engine swap without cutting notes (standalone player and plugin) |
| `piano-ui` | egui interface on top of `PianoControl` |
| `piano-plugin` | CLAP and VST3 instrument plugin (nice-plug), with the `piano-ui` editor |
| `piano-mcp` | MCP server of the standalone player (`--mcp`): parameters, presets and status for AI assistants |
| `xtask` | plugin bundles for the current platform (`cargo xtask bundle piano-plugin --release`) |
| `piano-play` | real-time application (`unsampled-piano` binary): PipeWire/JACK node (audio, MIDI in, MIDI mirror), or cpal + midir |
| `piano-render` | deterministic offline MIDI → WAV rendering (`unsampled-piano-render` binary) |
| `piano-analysis` | analysis: partials, B, decays, ESPRIT, spectral distances |
| `piano-eval` | comparison of the model with a reference bank |
| `piano-bench` | modal kernel throughput, f32 precision |

## Documentation

For playing and setting up the instrument, see the [website](https://unsampled.mgth.fr). The documents below are the development notes: the physics, the measurements and how each part was calibrated.

- State of the art and plan: [reports/piano_physical_synthesis.md](reports/piano_physical_synthesis.md)
- Phase 0 (tooling, CPU and precision measurements): [docs/phase0.md](docs/phase0.md)
- Analysis of Salamander Grand (Yamaha C5): [docs/salamander.md](docs/salamander.md)
- Phase 1 (string + hammer, calibration): [docs/phase1.md](docs/phase1.md)
- Phase 2 (coupled unisons, polarisations, bridge conductance): [docs/phase2.md](docs/phase2.md)
- Phase 4 (modal soundboard, radiation, stereo): [docs/phase4.md](docs/phase4.md)
- Phase 3 (dampers, pedals, sympathetic resonance): [docs/phase3.md](docs/phase3.md)
- Phase 5 (mechanical noises, presets, real time, calibration): [docs/phase5.md](docs/phase5.md)
- Cast-iron frame and rim: [docs/frame_rim.md](docs/frame_rim.md)
- Note sustain (bridge, soundboard thickness, string losses): [docs/sustain.md](docs/sustain.md)
- Dynamics, pedal and sympathetic resonance: [docs/dynamics_pedal.md](docs/dynamics_pedal.md)
- Comparison with Salamander, Iowa and a commercial modelled piano: [docs/comparison.md](docs/comparison.md)
- Test bench: the model scored against real pianos (`cargo xtask bench`): [docs/bench.md](docs/bench.md)
- The bench's channel-power protocol and Plath's fixed-gain import, before/after (French): [docs/bench_bias_correction.md](docs/bench_bias_correction.md)
- Recorded performances (Disklavier, Bösendorfer SE) against the model, note by note: [docs/pieces.md](docs/pieces.md)
- A graded hammer felt instead of the two-layer hammer: [docs/felt.md](docs/felt.md)
- Top treble: the knock of the truncated string, the decay of the top strings: [docs/top_treble.md](docs/top_treble.md)
- Bass decay: a board loss growing towards the low frequencies: [docs/bass_decay.md](docs/bass_decay.md)
- Bass string losses (unison detuning, cabinet piston mode): [docs/bass.md](docs/bass.md)
- Timbre change with dynamics (felt): [docs/timbre_dynamics.md](docs/timbre_dynamics.md)
- String nonlinearity (phantom partials): [docs/nonlinearity.md](docs/nonlinearity.md)
- Attack presence (two-layer hammer, microphone setups): [docs/presence.md](docs/presence.md)
- Sustain after the first second (unison detuning, piston, string losses): [docs/long_sustain.md](docs/long_sustain.md)
- Radiation of bass fundamentals (acceleration-driven radiation, soundboard crown): [docs/bass_radiation.md](docs/bass_radiation.md)
- Brightness of attack and sustain (soundboard damping, felt, hammer velocity): [docs/brightness.md](docs/brightness.md)
- Middle register: the 4–8 kHz highs (hammer pulse): [docs/middle_highs.md](docs/middle_highs.md)
- What rings on after the release (undamped top strings, damper range): [docs/release_ring.md](docs/release_ring.md)
- Releases and soft blows against a Disklavier grand (damper felt, sideways motion, uneven soft blows): [docs/release.md](docs/release.md)
- The action's knock, heard directly (keyframe, shanks; level of the real grands below the top keys): [docs/action_noise.md](docs/action_noise.md)
- The level of the top, the register balance and the flexible hammer shank: [docs/top_level.md](docs/top_level.md)
- Evenness from key to key (the board's colouring): [docs/evenness.md](docs/evenness.md)
- Musician's controls (tone, touch, sustain, resonance, endings, unisons, noises, size, lid, space, stretch): [docs/macros.md](docs/macros.md)
- A range of instruments from one engine (hammer mass, strike point, preset fitting): [docs/instrument_range.md](docs/instrument_range.md)
- Room level (piano directivity, microphone patterns): [docs/room_level.md](docs/room_level.md)
- The lid: its shape (the case's) and the shut lid: [docs/lid.md](docs/lid.md)
- Uprights (half blow, practice mute, repetition, wall, case, placement in the room, directivity): [docs/upright.md](docs/upright.md)
- The bridges: where the bass bridge ends on real grands, its place and bow on the bench: [docs/bridges.md](docs/bridges.md)

## License

Copyright © 2026 Mathieu GRENET.

This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. It is distributed WITHOUT ANY WARRANTY. See [LICENSE](LICENSE).
