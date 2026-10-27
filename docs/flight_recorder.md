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

## Found with it

"Now and then a note comes out very metallic, as if struck very hard, and playing it again at the same force sounds normal." The recording of the session where it happened held every note twice. The keyboard sent each one on channels 1 and 2 (a second zone), mostly at the same sample, 47 times out of 2014 one sample later, and once 96 samples (2 ms) later: an E3 played at velocity 56, the note heard. Its replay matched the player, bit for bit. A second note-on 2 ms after the first threw a new hammer at full speed onto a string the first one had just set moving, and its attack came out 5.7 dB louder with 21 dB more above 3 kHz. At 0, 0.5, 4 or 10 ms the double hardly changed anything, hence the rarity.

The engine now ignores a note-on for a key still down within 30 ms of the previous one (a double: no key repeats that fast without rising), and counts it: the interface's status bar shows "doubled MIDI notes", and the player without a window says so once. Nor does a key strike again while its hammer still presses the string, even after a note-off (a few ms: no action throws a hammer that has not left the string). Tests `midi_doubles`.

## Limits

- Changes made in the interface while playing (parameters, a rebuilt instrument) are not in the recording; the instrument is the one the engine started with.
- A controller mapped to a live parameter (MIDI learn) is recorded as the controller, and the replay does not map it.
