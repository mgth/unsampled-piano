---
title: Getting started
description: Install the standalone application or the CLAP/VST3 plugin on Windows, macOS or Linux, plug in a keyboard and play.
order: 1
icon: rocket
---

Unsampled Piano comes in two shapes that share the same engine and the same interface:

- the **standalone application**, `unsampled-piano`, which opens a window, listens to your MIDI
  keyboard and plays through your audio interface;
- the **plugin**, CLAP and VST3, for any DAW that hosts instruments.

Builds come from the project's GitHub Actions and are attached to each
[release](https://github.com/mgth/unsampled-piano/releases); the [download page](/download/)
picks the right file for you.

## Windows

The installer, `unsampled-piano-setup-windows-x86_64.exe`, offers three parts, each optional:
the standalone application (with a Start menu shortcut), the VST3 plugin and the CLAP plugin,
and the presets they share. Install for all users (`Program Files`, `Common Files\VST3` and
`Common Files\CLAP`, presets in `ProgramData\Unsampled Piano\presets`) or for yourself only;
an uninstaller comes with it.

Prefer no installer? The zip `unsampled-piano-standalone-windows-x86_64.zip` holds
`unsampled-piano.exe` and the presets beside it — it needs nothing else installed.

Output goes through **WASAPI** by default. The Windows build also speaks **ASIO**:

```bash
unsampled-piano --host asio
```

With ASIO the buffer size is the one set in the driver's control panel.

## macOS

`Unsampled Piano.app` is universal (Intel and Apple silicon) and plays through CoreAudio, with
the presets inside the bundle. It is **not notarised**: the first time, open it with a right
click → *Open*, or clear the quarantine flag from a terminal:

```bash
xattr -dr com.apple.quarantine "Unsampled Piano.app"
```

The plugins come in `unsampled-piano-macos-universal.zip`: copy `Unsampled Piano.clap` to
`~/Library/Audio/Plug-Ins/CLAP` and `Unsampled Piano.vst3` to `~/Library/Audio/Plug-Ins/VST3`.

## Linux

On Linux the piano is built from source — it is a Rust workspace, nothing more:

```bash
git clone https://github.com/mgth/unsampled-piano
cd unsampled-piano
cargo run --release -p piano-play -- --ui
```

It becomes a node in the **PipeWire** graph (through its JACK API), with a stereo output, a MIDI
input with sample-accurate timing and a MIDI output that mirrors what it plays. It connects
itself to the default output and to the first hardware keyboard. Building needs a stable Rust
toolchain, `pkg-config` and `clang`, the audio headers (ALSA, JACK and PipeWire:
`libasound2-dev libjack-jackd2-dev libpipewire-0.3-dev` on Debian and Ubuntu) and, for the
window, OpenGL and X11/xkbcommon (`libgl-dev libxkbcommon-dev libxcursor-dev libx11-xcb-dev`).

To install it properly — the binary, a menu entry that opens the interface, its icon and the
shipped presets, under `~/.local` (or `PREFIX=/usr/local` for every user, as root):

```bash
tools/install-app.sh
```

The plugins for Linux come in `unsampled-piano-linux-x86_64.zip`, or build and install them
into `~/.clap` and `~/.vst3`:

```bash
cargo xtask bundle piano-plugin --release
tools/install-plugin.sh
```

## First sound

1. Plug in a MIDI keyboard before starting; the first one found is used (`--midi <name>` picks
   another — `--list` shows them all).
2. Start the application. Opened from the Explorer or the Finder it shows the interface; from a
   terminal, add `--ui`.
3. Play. The status bar shows the number of voices and the audio thread's load.

Too much latency? Lower the buffer: `--buffer 128` (or 64 on a fast machine). Crackles? Raise it.

## Personal defaults

Options you always use go in `play.toml`, in the configuration folder:

| System | Folder |
|---|---|
| Linux | `~/.config/unsampled-piano/` |
| Windows | `%APPDATA%\unsampled-piano\` |
| macOS | `~/Library/Application Support/unsampled-piano/` |

```toml
# play.toml — anything the command line leaves out
device = "Scarlett 2i2"
buffer = 128
preset = "/home/me/pianos/my_grand.toml"
ui = true
```

The keys are those of the [command line](/docs/command-line/): `host`, `device`, `midi`,
`midi_out`, `name`, `preset`, `presets`, `buffer`, `output_channel`, `gain_db`, `ui`.
