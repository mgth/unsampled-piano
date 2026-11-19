# Comparing the model with the real pianos, key by key

`unsampled-piano-compare` (crate `piano-compare`) is a window for listening to and measuring the model against the bench's recorded pianos, one key at a time and across the keyboard.

    cargo run --release -p piano-compare
    cargo run --release -p piano-compare -- --preset presets/instruments/Concert\ grand/concert_grand.toml --note A0 --layer ff

## What it shows

The keyboard, the key's table, the pianos and the partials stay in view; only the plots of the measures scroll. Drag the line between the two parts to share the height (the window remembers it).

- **The keyboard.** Click a key to select it and hear it on the selected piano. Greyed keys have no recording for that piano at this layer. The strip under the keys shows the model's worst |z| over the measures shown (green ≤ 1, amber ≤ 2, red beyond).
- **The note's table.** Every piano that recorded the key at this layer, with the model first. Click a piano's name or its `>` button to play it; it becomes the piano the keyboard plays. The model's cells are coloured by z, and the last rows give the mean ± sd of the pianos in each band (the second band's row appears once it holds a piano).
- **The pianos and the partials.** The list of pianos says which are drawn and which make the band (below). The partials of the key: the level of partials 1–32 relative to the strongest, piano by piano, over the band.
- **One plot per measure** across the keyboard. Each key's value sits under the key itself. The bands are the spreads of the pianos ticked *band 1* (grey-blue) and *band 2* (amber), as min to max or mean ± sd, and the model is the thick red line. The title gives the share of keys within band 1 and the number of keys off. Hover a point to read it, and click to select that key.

Keys: left/right change the key, up/down change the piano (and play it), 1/2/3 select pp/mf/ff, and space plays again. "Match levels" plays every recording at the same mean level over its first second, because the sets were normalised in different ways. The playback starts 20 ms before the onset and lasts "length" seconds.

The pianos come from `bench/references.json`: the real pianos (`target`), and as information only the uprights and the commercial modelled piano (`info`, dashed). The commercial modelled piano's D, the Concert grand's reference, is added from `out/catalog_clean/steinway_d/` (all 88 keys at velocities 60 and 100, shown as mf and ff), in violet. It is information only, like the bench's own commercial set. The list of pianos has three ticks per piano. *show* draws the piano in the plots. *band 1* counts it in the first band and in the model's z, whether it is shown or not. *band 2* counts it in a second band with its own selection, which leaves the z alone: sort the pianos by ear into those that have a trait and those that do not, and the plots show whether a measure tells the two groups apart. By default band 1 holds the real pianos and band 2 is empty; each band's buttons *real*, *all* and *none* reset it. A piano can be in both bands, and the model can be in neither. `--set NAME=DIR[:target]` adds another note set (`<note>v<layer>.wav|flac`, layers 4/8/13).

The window remembers its selections from one start to the next in `~/.config/unsampled-piano/compare.toml` (or `PIANO_COMPARE_SETTINGS`): the key and layer, the piano played, the pianos drawn and those in each band, the measures shown, the band's form, the playback options and the height of the part that does not scroll. Pianos are stored by name, so a set added later appears with its defaults. `--note` and `--layer` override the saved key and layer.

## The model's notes

The model is rendered as the bench renders it: close microphones in a studio, played on the Disklavier's velocity scale (pp/mf/ff = velocities 45/56/100), each note alone in a fresh engine, held 3.5 s. The renders are cached in `out/compare-tool/model/<preset>-<quality>-<hash>/`. The hash covers the whole preset (listening included), the engine's sources (`crates/piano-core`, read by content when the tool is built, local changes included) and the rendering protocol: a changed preset or engine renders again, and an unchanged one never does. Worktrees with the same engine share their renders. The selected key renders first, then every octave, then every third key, then the others. With two renders at once (`--renderers`, each about 3 cores) all 264 notes take about 7 minutes.

`--preset` chooses the instrument (default: the Concert grand), `--default-instrument` the engine's default, `--quality` the engine quality and `--no-model` leaves the model out.

## The measures

Every measure averages the channels' powers. It never sums their pressures, as on the bench. The time windows start at the onset, where the summed power first reaches 1 % of its peak.

| Measure | Definition |
|---|---|
| Tuning | first partial of the inharmonic fit against equal temperament at 440 Hz, cents |
| Inharmonicity B | B of f_k = k f0 √(1 + B k²) over the partials found 20 dB above their neighbourhood (0.05–1.05 s) |
| Rise time | 10 % to 90 % of the peak amplitude, windows of 5 ms or one period |
| Brightness, attack / sustain | spectral centroid from f1/2 to 12 kHz over 0–60 ms / 0.5–1 s, as a multiple of f1 |
| Partials 2–4, sustain | mean power of partials 2–4 against partial 1, 0.5–1 s, dB: the overtones against the fundamental as the note sounds on |
| Spectral tilt | slope of the partial levels (1–16, below 8 kHz) against log2 of their rank, 0.02–0.32 s |
| Between partials | median level between neighbouring partials relative to their level, below 5 kHz, 0.02–0.32 s |
| Early / late decay | slope of the overall level, 0.1–1 s / 1–3 s, above the noise floor + 6 dB |
| T60 of partial 1 | straight-line decay of the first partial's envelope, 0.1–3 s |
| Partial 1 ripple | RMS distance of that envelope from the line (beats, double decay) |
| Dynamics ff − pp | level of the first second, ff minus pp: BiVib and the model only (the sample sets were normalised) |

The noise floor of the decay fits (+6 dB) is the quietest level within 12 s of the onset. Read within the measured 3.3 s alone, a long note's floor was its own decay, and the late decay came out empty. Liang's and MAPS's isolated notes are released 1.9–2.0 s after their onset, where their level drops by 15–26 dB, so their decays are fitted up to 1.85 s. The other sets hold past 3.3 s.

The model's rise time reads 0–1 ms, against 4–12 ms on the real pianos. This is not a measurement artefact: a recording starts 6–8 dB below its peak and climbs to it over a few milliseconds (key noise, then the hammer), while the render starts at its peak.

z = (model − mean of the pianos in band 1) / max(their sd, floor), on log10 for B, the brightness and T60. The floor is 0.08 on log10 for the brightness (a factor of 1.2; the real pianos differ by a factor of 1.12 at the median), 3 dB for partials 2–4. Until 2026-11-19 the brightness's floor was 0.5, a factor of 3.2: a sustain three times too bright, as E5 and D#5 were, never showed. It needs at least two pianos of band 1 at that key and layer. As on the bench, a gap as large as the one between two real pianos is not a fault.

The measures are cached by file (size and date) in `out/compare-tool/measures-v<N>.json`. Change `measure::VERSION` when a measure changes. The first launch measures the 1,850 recordings in about 20 s on four low-priority threads.

`--score out.json` opens no window: it renders the model's 264 notes in memory, measures them and writes, overall, per measure and per register, how many z exceed 2 (`off`) and the mean |z| with each z clipped at 3 (`mean`). `--band-without iowa,liang` leaves real pianos out of the band, and `--score-threads` sets the threads. A script can judge a preset this way without touching the renders' cache. `tools/compare_search.py` uses it to tune a preset (docs/compare_search.md).

`PIANO_COMPARE_SHOT=file.ppm` captures the window once the queues are empty (or after `PIANO_COMPARE_SHOT_AFTER` seconds), scrolled by `PIANO_COMPARE_SCROLL` pixels, then closes it. No audio is opened in that mode.
