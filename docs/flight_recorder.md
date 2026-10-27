# Flight recorder

Status as of 2026-10-27.

A fault heard while playing, now and then, cannot always be brought back on demand. The standalone player can keep a flight recorder (`--blackbox`, or `blackbox = true` in `play.toml`; off by default):

- every event the engine received since it started (notes, controllers, the parameters the keyboard sends), at its exact sample;
- the length of every block it rendered (the output depends on them);
- the instrument it was built from;
- the last minute of its output.

When the player stops (the window closed, Ctrl-C or SIGTERM), it writes them into a new folder of the configuration directory's `blackbox` folder (`~/.config/unsampled-piano/blackbox/<time>/` on Linux): `preset.toml`, `events.txt` and `audio.wav`. It keeps the last three.

```
unsampled-piano-render replay <folder> <out.wav>
```

renders it again with the same blocks, events, instrument, rate, threads and quality, and compares the minute the player kept with its own render. The engine gives the same output whatever the thread count, so a replay that differs points at something the recording missed rather than at the engine's arithmetic.

The audio thread only ever tries the recorder's lock (it never waits and never allocates). A block it could not record is counted, and the folder then says `complete false`.

## Limits

- Changes made in the interface while playing (parameters, a rebuilt instrument) are not in the recording; the instrument is the one the engine started with.
- A controller mapped to a live parameter (MIDI learn) is recorded as the controller, and the replay does not map it.
