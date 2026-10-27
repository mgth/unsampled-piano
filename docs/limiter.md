# Clipping indicator and limiter

Status as of 2026-10-27.

## Why

A piano's attack is peaky: the treble's above all reaches full scale long before the music sounds loud. With close microphones and `output_gain_db` +3 dB, a single C6 or F#6 at velocity 110 peaked at +2 to +3 dBFS (+6.7 at 127), and dense fortissimo treble chords at +12 to +14 dBFS. The player and the plugin send floating-point samples as they are, so these overs clip further down the chain (a host's bus, a sound card, a Bluetooth encoder): a harsh crack on the loudest notes.

## The output level

The engine's level was raised by 9 dB (`OUTPUT_LEVEL_DB` in the engine; `output_gain_db` 0 means this level). Before, with the close microphones, performances sat at −30 to −37 LUFS and peaked at −8 to −19 dBFS, far under the system's other sounds. Unprocessed recordings of real grands (SMD, the same pieces) sit at −20 to −23 LUFS and peak near −2 dBFS. The ratio of peak to loudness was already the same for both (about 20 dB): the attacks were not too peaky, the whole was too low. Commercial piano records are louder still (−14 to −18 LUFS), after a limiter.

At the new level the limiter, on by default, catches the loudest treble chords. Over a Chopin nocturne it works on 0.01 % of the time, at most 1.4 dB.

## The indicator

The last stage of the audio path (`EngineSlot`, after the engines finishing their notes are added) counts the samples above full scale and keeps the output's peak. The controller's status carries them (`Status::clipped`, `peak`, since the last status), and so does the limiter's largest gain reduction (`limiting_db`).

- In the interface, the status bar shows a red **CLIP** for three seconds after samples went over, with their count and the peak on hover, and **Limiter −x dB** while the limiter works.
- The player without a window adds them to its load report.

## The limiter

`limiter` (on by default) and `limiter_ceiling_db` (−1 dBFS by default, at most 0) are live parameters, on the interface's Player page (a switch and a slider).

It works on the sum of the engines, both channels alike (the stereo image stays), without latency: the gain falls at once to what a sample above the ceiling needs, so nothing leaves above the ceiling, and comes back with a 100 ms time constant. A look-ahead would let it fall more smoothly, but it would delay the piano.

Tests: `limiter` in piano-core (the ceiling holds, the gain comes back, below the ceiling nothing changes) and `tests/limiter.rs` in piano-control (a chord 20 dB hot clips and is counted without the limiter, stays under the ceiling with it, and the limiter switches live).

## Limits

- The player's own `--gain-db` (`gain_db` in play.toml) applies after the meter and the limiter.
- Offline renders (`unsampled-piano-render`) write the engine's output without this stage: floating-point files, nothing clipped.
