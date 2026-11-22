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

The model is rendered as the bench renders it: close microphones in a studio, played on the Disklavier's velocity scale (pp/mf/ff = velocities 45/56/100), each note alone in a fresh engine, held 3.5 s. The renders are cached in `out/compare-tool/model/<preset>-<quality>-<hash>/`. For the release drops only, the notes are rendered a second time with the key released 2 s after the note-on, as Liang's and MAPS's are, and stopped 0.5 s later; those renders have their own folder and key, `out/compare-tool/model-release/`, so the 3.5 s renders keep theirs. The window renders them after the others, at the keys and layers Liang or MAPS recorded (every key at ff, MAPS's 34 keys at mf). The hash covers the whole preset (listening included), the engine's sources (`crates/piano-core`, read by content when the tool is built, local changes included) and the rendering protocol: a changed preset or engine renders again, and an unchanged one never does. Worktrees with the same engine share their renders. The selected key renders first, then every octave, then every third key, then the others. With two renders at once (`--renderers`, each about 3 cores) all 264 notes take about 7 minutes.

`--preset` chooses the instrument (default: the Concert grand), `--default-instrument` the engine's default, `--quality` the engine quality and `--no-model` leaves the model out.

## The measures

Every measure averages the channels' powers. It never sums their pressures, as on the bench. The time windows start at the onset, where the summed power first reaches 1 % of its peak.

| Measure | Definition |
|---|---|
| Tuning | first partial of the inharmonic fit against equal temperament at 440 Hz, cents |
| Inharmonicity B | B of f_k = k f0 √(1 + B k²) over the partials found 20 dB above their neighbourhood (0.05–1.05 s) |
| Rise time | 10 % to 90 % of the peak amplitude, windows of 5 ms or one period |
| Brightness, attack / sustain | spectral centroid from f1/2 to 12 kHz over 0–60 ms / 0.5–1 s, as a multiple of f1 |
| Partials 2–4, sustain | mean power of partials 2–4 against partial 1, 0.5–1 s, dB, each partial's power summed over its band (±f1/4), so that a unison's partial split into several lines counts whole: the overtones against the fundamental as the note sounds on. A window gone silent (under −100 dB re the first second) reads nothing, in mono as in stereo |
| Spectral tilt | slope of the partial levels (1–16, below 8 kHz) against log2 of their rank, 0.02–0.32 s |
| Between partials | median level between neighbouring partials relative to their level, below 5 kHz, 0.02–0.32 s |
| Early / late decay | slope of the overall level, 0.1–1 s / 1–3 s, above the noise floor + 6 dB |
| T60 of partial 1 | straight-line decay of the first partial's envelope, 0.1–3 s |
| Partial 1 ripple | RMS distance of that envelope from the line (beats, double decay) |
| Dynamics ff − pp | level of the first second, ff minus pp: BiVib and the model only (the sample sets were normalised) |
| Lines 1.2–2.5 kHz, 1.2–2.2 s | power on the partial lines of the band, late, dB re partials 1–6 in the first 5–45 ms (below): the control band |
| Lines 2.5–4 kHz, 0.6–1.2 s and 1.2–2.2 s | the same on 2.5–4 kHz |
| Lines 4–7 kHz, 0.6–1.2 s and 1.2–2.2 s | the same on 4–7 kHz |
| Lines 2.5–4 kHz, early decay; Lines 4–7 kHz, early decay | slope of the power on the band's partial lines over 0.02–0.32 s, dB/s (below): the treble's first stage |
| Release drop 150–600 Hz, 0.6–1.2 kHz, 1.2–2.5 kHz, 2.5–5 kHz | level of the band 100–250 ms after the key's release against 150–30 ms before it, dB: Liang, MAPS and the model released at 2 s only |

The noise floor of the decay fits (+6 dB) is the quietest level within 12 s of the onset. Read within the measured 3.3 s alone, a long note's floor was its own decay, and the late decay came out empty. Liang's and MAPS's isolated notes are released 1.9–2.0 s after their onset, where their level drops by 15–26 dB, so their decays are fitted up to 1.85 s. The other sets hold past 3.3 s.

### The partial lines' late level

The spectral windows of the measures above stop at 0.32 s, or read 0.5–1 s where the low partials dominate, and the decays follow the overall level or partial 1: none of them sees partials above 2.5 kHz that die three to four times too fast. The *Lines* measures sum the power on the partial lines of a band in a late window. Each line is the bins within ±f1/4 of the partial, where the note's inharmonic fit found it (or puts it), so that a unison's split partial counts whole. The level between the lines (the median of the band's other bins) stands for the noise and is taken off. The reference is the power from f1/2 to f6 + f1/2 in the first 5–45 ms: the start of the low partials, which the late level is read against. The windows are Hann windows, which weigh a decay towards their middle. The measure reads nothing when fewer than two lines lie in the band, when the lines are not 10 dB clear of the level between them (the real sets' noise floors differ by tens of dB, and a line lost in the noise must not become a target), when the key is released before the window ends (Liang and MAPS lose 1.2–2.2 s), or when the file is too short.

Their floor is 3 dB, as for partials 2–4: the five pianos of the band differ by 4.0–4.6 dB (sd) at the median key and layer, and by 2.0–2.7 dB at the lower quartile. The 1.2–2.5 kHz band is the control. On 2026-11-21 the model stayed within 2 dB of the band there (median over C3–C6 every third key, ff), while it read 11 dB under at 2.5–4 kHz and 0.6–1.2 s and 22 dB under at 1.2–2.2 s, and 29 dB under at 4–7 kHz and 0.6–1.2 s. At 4–7 kHz and 1.2–2.2 s the real lines are mostly lost in their noise: few keys are judged there.

### The partial lines' early decay

On the real grands the upper partials of the treble fall in two stages: over the first 0.3 s, C5–B5's lines at 4–8 kHz lose about 68 dB/s, then 27 (docs/sustain.md, "The treble's two stages"). The Lines measures read the second stage's level; *Lines 2.5–4 kHz, early decay* and *Lines 4–7 kHz, early decay* read the first. They use the same lines as the Lines measures (±f1/4 around each partial, where the fit found it, with the level between the lines taken off and the 10 dB clear guard), in nine 0.1 s Hann windows every 25 ms from 0.02 s after the onset (to 0.32 s). A straight line is fitted through the nine levels in dB. They read nothing when a window's lines are not clear, when fewer than two lines lie in the band, when the key is released before 0.32 s, or when the first partial is under 80 Hz, as a line spreads over ±20 Hz in a 0.1 s window.

Their floor is 8 dB/s. The five real pianos differ by about 25 dB/s (sd) at the median key from C4 up, since the unisons' beats weigh in a 0.3 s fit. The floor only keeps two pianos that agree by chance from making a target.

Baseline (measures version 9, `--score`, band without bivib_grand, iowa, liang), the shipped Concert grand: *Lines 2.5–4 kHz, early decay* 44 off of 114, mean 1.61 (treble median −62.3 dB/s against −44.5 for the band, middle −49.1 against −31.3); *Lines 4–7 kHz, early decay* 61 off of 116, mean 1.90 (treble −87.5 against −57.9, middle −69.0 against −38.4). In all, 865 measures off, mean 1.194. The 22 measures of version 8 are unchanged: 760 off, the same counts and means per measure to the last digit or two of the mean (1.1563801716857094 against …076, the order of a floating sum).

### The release drops

What the damper leaves: the drop of each band from 150–30 ms before the key's release to 100–250 ms after it. Only Liang and MAPS release their keys within their files (about 2 s after the onset); after about 0.25 s past the release Liang reads its noise floor, which is why the window stops at 250 ms. The release is found, not assumed (2.0–2.2 s after the onset on these sets): the steepest 60 ms fall of the overall level from 1.5 s on anchors it, and the release is where the level first leaves the line of its decay before that fall, and stays under it (a beat's dip comes back up). A damper that stops the strings at once and one that damps them slowly are timed alike, which matters for the model. The model is measured on its second render, released 2 s after the note-on: like for like. The band's level before the release must stand 25 dB above the file's noise floor in the band (its 2nd percentile over 50 ms frames of the file), the floor is taken off the level after, and an after level within 3 dB of the floor reads nothing (the drop would only be a bound). A render has no noise floor: only its 90 dB range bounds it. Keys without dampers (Liang from E6 up) have no fall to find and read nothing.

These measures are judged by the pianos that release their keys, Liang and MAPS, whatever the band of the other measures holds: the window's band 1 and `--score` use them in place of the band, which leaves `--band-without liang` and MAPS's `info` role to the other measures. As Liang has only ff and MAPS 40 keys at ff and 34 at mf, the two rarely read the same key: one piano is enough for a z there, against a floor of 4 dB (Liang and MAPS differ by 4.3–4.7 dB, sd, at their common keys between 600 Hz and 2.5 kHz). MAPS's Disklavier is an upright (`bench/references.json`): its damper is not a grand's. The 150–600 Hz band holds the fundamental from D3 up and partials 2–4 below: what it reads moves along the keyboard.

The model's rise time reads 0–1 ms, against 4–12 ms on the real pianos. This is not a measurement artefact: a recording starts 6–8 dB below its peak and climbs to it over a few milliseconds (key noise, then the hammer), while the render starts at its peak.

z = (model − mean of the pianos in band 1) / max(their sd, floor), on log10 for B, the brightness and T60. The floor is 0.08 on log10 for the brightness (a factor of 1.2; the real pianos differ by a factor of 1.12 at the median), 3 dB for partials 2–4. Until 2026-11-19 the brightness's floor was 0.5, a factor of 3.2: a sustain three times too bright, as E5 and D#5 were, never showed. It needs at least two pianos of band 1 at that key and layer. As on the bench, a gap as large as the one between two real pianos is not a fault.

The measures are cached by file (size and date) in `out/compare-tool/measures-v<N>.json`. Change `measure::VERSION` when a measure changes. The first launch measures the 1,850 recordings in about 20 s on four low-priority threads.

`--score out.json` opens no window: it renders the model's 264 notes in memory (and the release renders where Liang or MAPS has a release measure), measures them and writes, overall, per measure and per register, how many z exceed 2 (`off`) and the mean |z| with each z clipped at 3 (`mean`); each measure also has its registers, with the median of the model's values and of the band's means. `--band-without iowa,liang` leaves real pianos out of the band (not out of the release drops' judges), and `--score-threads` sets the threads. `--score-notes` adds every key's and layer's value of the model and of each piano judging it. A script can judge a preset this way without touching the renders' cache. `tools/compare_search.py` uses it to tune a preset (docs/compare_search.md).

`PIANO_COMPARE_SHOT=file.ppm` captures the window once the queues are empty (or after `PIANO_COMPARE_SHOT_AFTER` seconds), scrolled by `PIANO_COMPARE_SCROLL` pixels, then closes it. No audio is opened in that mode.
