# Flight recorder

Status as of 2026-10-27.

A fault heard while playing, now and then, cannot always be brought back on demand. The standalone player can keep a flight recorder (`--blackbox`, or `blackbox = true` in `play.toml`; off by default):

- every event the engines received since the player started, at its exact sample, as the engine slot applied them: notes, controllers, the live parameters edited in the interface or sent by a mapped controller;
- every instrument rebuilt (an edit of a parameter that is not live), and the block it came in at;
- the length of every block rendered (the output depends on them);
- the instrument the player started with;
- the last minute of its output, after the output stage (limiter).

The slot gives each block's journal (`EngineSlot::journal`): the recorder only copies it.

When the player stops (the window closed, Ctrl-C or SIGTERM), it writes them into a new folder of the configuration directory's `blackbox` folder (`~/.config/unsampled-piano/blackbox/<time>/` on Linux): `preset.toml`, `events.txt`, `audio.wav` and a `rebuilt_<k>.toml` per instrument rebuilt. It keeps the last three.

```
unsampled-piano-render replay <folder> <out.wav>
```

renders it again through a fresh engine slot (`piano_control::replay`), with the same blocks, events, rebuilt instruments injected where they came in, rate, threads and quality, so the engines finishing their notes after a rebuild and the limiter are there too; and compares the minute the player kept with its own render. The engine gives the same output whatever the thread count, so a replay that differs points at something the recording missed rather than at the engine's arithmetic.

The audio thread only ever tries the recorder's lock (it never waits and never allocates). A block it could not record is counted, and the folder then says `complete false`.

## Found with it

"Now and then a note comes out very metallic, as if struck very hard, and playing it again at the same force sounds normal." The recording of the session where it happened held every note twice. The keyboard sent each one on channels 1 and 2 (a second zone), mostly at the same sample, 47 times out of 2014 one sample later, and once 96 samples (2 ms) later: an E3 played at velocity 56, the note heard. Its replay matched the player, bit for bit. A second note-on 2 ms after the first threw a new hammer at full speed onto a string the first one had just set moving, and its attack came out 5.7 dB louder with 21 dB more above 3 kHz. At 0, 0.5, 4 or 10 ms the double hardly changed anything, hence the rarity.

The engine now ignores a note-on for a key still down within 30 ms of the previous one (a double: no key repeats that fast without rising), and counts it: the interface's status bar shows "doubled MIDI notes", and the player without a window says so once. Nor does a key strike again while its hammer still presses the string, even after a note-off (a few ms: no action throws a hammer that has not left the string). Tests `midi_doubles`.

The test `flight_recorder_keeps_what_was_played` (piano-play) plays in irregular buffers, edits a live parameter and rebuilds the instrument on the way, and gets the same output again from the recording, sample for sample.

## Limits

- Before 2026-10-27 the recordings missed the interface's edits and rebuilt instruments: a session where the sound was changed does not replay (one with the output gain raised and the microphones moved came out uncorrelated with what was heard).
