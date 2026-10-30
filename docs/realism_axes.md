# Realism: where the ear and the bench disagree, and new axes of comparison

Status as of 2026-10-28, on branch `claude/piano-render-realism-5cf127` (the tuning branch's tip, 9829990, with the grand's outline on by default). Pilot scripts: `tools/axes_pilot.py`, `tools/fixed_lines.py`, `tools/plath_tf.py`, `tools/tf_compare.py`, `tools/velslope.py`, and the example `bridge_to_ear` of piano-render.

## The default, and how it is rendered

`PianoModel::default()` is the 2 m grand: `SoundboardParams::GRAND_2M` with the outline (ρh 8.4, D 4590/2040, impedance slopes 0.0024/0.0076, `loss_low_corner` 200 Hz, crown 90 Hz), `losses.winding` 2·10⁻⁴, `unison.load_ramp` 36, `sympathetic.gain` 0.6, `string_load_free` 1, the single felt (`hysteresis` 0.3), the velocity law of recorded performances, the engine's own listening (a concert hall heard from 1.5 m, lid flat). The bench and every A/B render it with the close pair 40 cm above the board in the studio and the Disklavier's played-back velocity scale. What the user plays live (`~/.config/unsampled-piano/last.toml`) is not quite that default: a refitted felt (`hardness` 0.26, `core_hardness` 0.69, `hysteresis` 0.6), `loss_low_corner` 0, `output_gain_db` 20, a custom temperament.

The chain, for one note: the felt hammer (quadratised Stulov law, flexible shank) strikes the unison's modal strings, which are coupled at the bridge (two polarisations, Weinreich); their bridge force, summed over the keys of a bridge region (16 regions, the band below 400 Hz shared between two), drives the modal board: the outlined plate's own modes below 1.5 kHz (Rayleigh–Ritz), the clamped rectangle's above, η = 0.02·(1 + f/2500 Hz) plus the damped strings' load and the edge loss; each mode radiates to the two microphones through a Rayleigh integral evaluated once at its own frequency (back face, lid, frame and rim added one-way), then the room (image sources and an FDN). Sympathy is one-way: every string with its damper lifted reads the bridge velocity at its region and rings; damped strings are deaf (Full) or listen (Overkill).

`cargo xtask bench` on this default: **1.11**, 76 % of the measures within the eight real grands, 12 off, the same as the saved baseline. The twelve off: the bass's sustain partial 6 (−0.1 dB re partial 1 against +11 on the real pianos), the treble's sustain partial 8 and its 2 s envelope, and nine measures of the top octave (C7–C8): partial 3 of the attack 10–15 dB too strong, the 1 and 2 kHz bands 10–24 dB too weak at the attack and in the sustain, the sustain's partial 4 25 dB too weak, the early decays twice too slow.

## Where the work stalled

Between 2026-10-25 and 2026-10-28 the bass went through about ten A/B rounds against Liang's Disklavier (docs/bass_metal.md): the 220 Hz hollow, the lowest board modes' damping, the unisons' tension ramp, the winding losses, the felt, the board's damping law, the longitudinal modes, three microphone pairs, the crown and the back face, the outline. Four were kept (winding, `loss_low_corner`, `load_ramp`, the outline), each after the user's ear; the bench moved between 1.09 and 1.14 throughout and flagged none of what the user heard. The two complaints remain: a "PVC tube" in the attack and a "rounder resonance" the recording has and the model has not. The strike-shape work (a felt contact per string, `hysteresis` 0.6, docs/strike_shape.md; bench 1.03 on the rectangle, the C6 "plop" fixed) still sits uncommitted in the Codex worktree `piano-realism-audit` and was never rebased on the outlined board.

The pattern: the bench measures the levels and decay rates of the partials of isolated notes, with 3–4 dB floors and the spread of eight pianos as its tolerance. The model has been inside that spread on three quarters of the measures for a week; from there the bench cannot rank candidates any more, and the ear takes over, one A/B at a time, with the biases that come with it (docs: the 2026-10-28 batch where the room, not the parameter, decided). What the ear calls "richer", "rounder", "a tube" lies where the bench does not look: between the partials, in the instrument's own resonators, in the fine structure of the board's response, in how the stages combine. The axes below each measure one of those, on the eight real grands first, so that the model's gap is read against the pianos' own spread.

## Axes piloted

Since 2026-10-30 axes 1 and 2 are bench measures (`between`, `lines`, family `resonance`; docs/bench.md, "Weak partials, single-source targets and the resonance measures"), read on channel power and with partials judged against a wider window; the bench's means of the eight grands come within 2 dB of the medians below, and the model's middle reads −32 dB there (it has changed since these pilots).

All the pilots use the bench's keys (every third from A0) at ff (velocity 100 or the references' top layer), the real note sets in `out/bench/sets` and `out/compare`, and the model rendered as the bench renders it. Scripts in `tools/`; about 6 s per set.

### 1. Energy between the partials

`tools/axes_pilot.py`. In a window after the onset, the noise-subtracted spectrum is split into what lies on the note's partials (±2 bins or ±0.4 % around each found peak) and what lies between them, 300 Hz–5 kHz; the measure is their ratio in dB. Medians over the register's keys, early window 0.08–0.6 s:

| | bass | middle | treble | treble 1.2–2.4 kHz | treble 2.4–4.8 kHz | top, late 0.6–1.5 s |
|---|---|---|---|---|---|---|
| eight real grands, range | −17 to −26 dB | −20 to −31 | −17 to −34 | −19 to −29 | −11 to −22 | −2 to −11 |
| their median | −21 | −25 | −26 | −25 | −19 | −9 |
| model (Full) | −26.3 | −26.0 | −34.3 | −31.9 | **−32.9** | **−21.3** |
| model (Overkill: damped strings listen) | −22.3 | −25.4 | −33.4 | −30.5 | −32.6 | −21.0 |

The bass and the middle sit at the clean edge of the real pianos; the damped strings' halo (Overkill, `listen_all`) brings the bass 4 dB in, to the real median, and changes nothing above. The treble is 8–14 dB cleaner than any real piano at 1.2–4.8 kHz, and the top octave's late sustain 10 dB cleaner. Plath's anechoic notes are among the real pianos here: it is not the rooms.

### 2. The instrument's fixed lines

`tools/fixed_lines.py`. Off-partial peaks (0.1–0.5 s, 0.8–6 kHz, 12 dB above their ±150 Hz median, within 45 dB of the strongest partial) that appear at the same frequency (±0.5 %) on at least three different notes: resonators the instrument rings whatever the key. On the nine top notes C6–C8 (every third):

| | lines shared by ≥ 3 notes | the strongest, dB re the note's strongest partial |
|---|---|---|
| BiVib (Disklavier grand) | 8 | −20.5, −23.3, −25.8 (1518, 1605, 1890 Hz) |
| Liang (Disklavier grand) | 17 | −29 to −32 |
| Headroom (C3) | 29 | −26, −31 |
| Helgesson (CF-3) | 7 | −21, −26, −28 |
| Plath (anechoic concert grand) | 64 | −33 to −38 |
| Splendid (Steinway) | 40 | −30 to −33 |
| Iowa (Steinway B) | 27 | −25 to −34 |
| Salamander (C5) | 20 | −29 to −36 |
| **model** | 9 | **−36 to −42** (1880 and 1993 Hz: its own A#6 and B6 strings) |

Liang's lines fall on A6, B6, C#7, D#7, F7, all 15 cents sharp, as a stretched treble is: the undamped strings of the top, ringing in sympathy with every note, including E4 and C3 (where BiVib and Helgesson show lines at −24 and −46 dB). The model has them 10–15 dB weaker. `sympathetic.gain` moves them linearly (C7's 1993 Hz line: −35 dB at 0.6, −31 at 1.0, −25 at 2.0), but the gain is uniform and 1.0 already overshoots the middle's pedal halo (docs/pedal_drone.md): the top's sympathy is too weak *relative* to the middle's. This is the same thing as the bench's "sound below the fundamental" of the top octave (1–2 kHz bands 10–24 dB low) and most of axis 1's treble gap: the real halo at 1.3–5 kHz is the undamped treble and whatever rings with it (the duplex segments behind the bridge, which the model has not).

### 3. From the bridge to the ear, stage by stage (Plath)

Plath's dataset holds, for every key of a concert grand in an anechoic chamber, the bridge acceleration (an accelerometer "at the bridge, at the hitch pins of the string") and the pressure at a dummy head in the player's seat, recorded together. Their ratio at the partials of each note, 0.1–1 s after the onset, samples the transfer from the bridge's motion to the ear, independent of the hammer and the strings: `tools/plath_tf.py` (2,600 samples over 88 keys). The model's side is the same ratio computed from the board's modes, `cargo run --release -p piano-render --example bridge_to_ear -- presets/default.toml presets/listening/Microphones/player.toml` (pressure per bridge force over the driving-point mobility, at the key's bridge point, the player's microphones), and `tools/tf_compare.py` puts them side by side per third octave:

| third octave | real, median | real, spread (p75 − p25) | model, median | model, spread | model − real |
|---|---|---|---|---|---|
| 125 Hz | −23.3 dB | 5.0 | −24.4 | 2.7 | −1.1 |
| 250 | −25.2 | 5.5 | −35.4 | 9.0 | **−10.2** |
| 400 | −35.3 | 8.7 | −28.4 | 10.8 | **+6.9** |
| 500–800 | −34 | 4.5–6.4 | −29 | 7–10 | +5 |
| 1000 | −36.4 | 6.9 | −35.9 | 11.0 | +0.5 |
| 1260–2520 | −38 to −52 | 6.6–10.4 | −43 to −55 | 5.5–8 | −2.5 to −5 |
| 3175–5040 | −49 to −54 | 11–13 | −58 to −66 | 5–7.5 | **−9 to −13** |
| 6350–8000 | −63 to −68 | 15 | −70 | 8–10 | −3 to −7 |

(dB re (Pa per m/s²)²; the level over 200 Hz–4 kHz is within 1.8 dB of the real piano, in SI units, with nothing fitted. The table was first computed on `presets/default.toml`, then a stale dump of the rectangular board; it is now the outlined default's, within a decibel or two of the rectangle's at every band: docs/board_radiation.md.) Two readings:

- **Shape.** Per unit of bridge motion, the model's board and radiation give 4–8 dB too much at 400–800 Hz and 11–13 dB too little at 3–5 kHz. The first is the register of the "lumps" heard at 200–400 Hz in the Beethoven passage; the second is the treble darkness that the bench's bands keep flagging, now located in the board and its radiation rather than in the strings or the hammer. The damping law η = 0.02·(1 + f/2500) is the first suspect: at 3–5 kHz the model's board (η 0.044–0.06) confines the vibration near the driving point, and a confined patch radiates less per unit point velocity than a board that rings as a whole.
- **Fine structure.** The spread of the samples within a third octave is the board's colouring from key to key and partial to partial. Below 1 kHz the model is *more* uneven than the real piano (10.8 against 8.7 dB at 400 Hz, 11 against 7 at 1 kHz): the key-to-key evenness problem, seen from the bridge. Above 2 kHz it is *less* (5–8 against 11–15 dB): the rectangle's regular modes, where a real board's ribs localise the vibration (Ege 2013: waveguides between the ribs above ≈ 1.1 kHz) and make every bridge point hear a different board. Both read "ribs and two bridges", with a number to hit.

Caveats: the dummy head (HSU 3.2) has its own frequency response at the ear, unknown equalisation, worth a few dB above 2 kHz; the accelerometer's exact spot is "at the bridge, at the hitch pins". The setup photos show a Steinway concert grand with its lid taken off; the model's lid at 35° changes nothing at the player's position (no cell shadowed, few reflections reach the ears), so the table holds with or without it. The real piano's own spread (4–15 dB) is the yardstick.

### 4. How each partial grows with velocity (BiVib)

`tools/velslope.py`. BiVib's Disklavier recorded every key at ten velocities with calibrated levels; the model plays the same (`tools/velocity_bench.py`'s renders). For partials 1–8, the level 0.03–0.2 s after the onset against velocity, its slope over 23–100 in dB per 10 velocity steps, median over the register's keys, re partial 1's slope (how much faster the upper partials grow than the fundamental):

| | p2 | p3 | p4 | p5 | p6 | p7 | p8 |
|---|---|---|---|---|---|---|---|
| middle, Disklavier | +0.6 | +0.4 | +1.0 | +1.4 | +1.4 | +2.9 | +4.0 |
| middle, model | +0.7 | +0.6 | +0.9 | +1.1 | +1.3 | +1.4 | +2.7 |
| treble, Disklavier | +1.7 | +2.2 | +2.2 | +1.5 | +0.5 | +1.0 | −1.7 |
| treble, model | +0.8 | +1.0 | +2.9 | +3.2 | +4.0 | +4.1 | +4.4 |
| top, Disklavier | −0.4 | −3.3 | −4.2 | −4.4 | | | |
| top, model | −2.0 | −1.3 | −0.8 | +1.8 | | | |

The fundamentals' own slopes match (1.6/1.8/3.0/4.3 dB on the Disklavier against 1.7/1.9/3.0/4.4: the velocity law was fitted on them), and so does the bass. Above: the middle's partials 7–8 brighten half as fast as the Disklavier's, the treble's partials 2–3 too little and 4–8 too much, and in the top octave the real partials 3–5 grow 3–4 dB per 10 steps *slower* than the fundamental where the model's grow as fast. The felt's law (its exponent, the graded core, the hysteresis) and the strike point are what this measures; it is where the strike-shape work belongs, and it needs a second instrument (Salamander's 16 layers, as slope ratios, which do not depend on the author's velocity mapping).

### 5. Beats and the irregularity of the decays: weak

Same script: the rms of each partial's envelope about a quadratic fit (0.25–2.5 s), and the spread of the decay rates of partials 1–6. The real pianos disagree with one another too much (bass p1: 0.1–0.6 dB rms on seven of them, 3–4 on Liang's) for a floor to mean anything. The model's bass partials are steadier than seven of the eight (0.1 dB), the rest sits inside. Worth refining (beat rate against the unisons' detuning, per partial) rather than adopting as is.

### 6. The long-term spectrum's ripple on a passage: not discriminative

On the Beethoven passage of the A/B files (10 s), the 1/48-octave ripple about the 1/3-octave smoothing is 4–7 dB rms in both the recording and the model, in every octave: the notes' own partials dominate it. Dropped; a *difference* spectrum model − recording on the same notes, at fine resolution, would be the version to try.

## Axes proposed, not piloted

7. **Stage swap, for the ear.** Built the same day: docs/stage_swap.md, `tools/stage_swap.py`, the `bridge_signal` example. Plath's pairs give, per key, the real board's response from the bridge to the ear; the model's bridge velocity goes through it, and Plath's bridge acceleration through the model's board. One listening then says whether the bass "tube" lives in the strings and hammer or in the board and radiation, which ten A/B rounds on parameters could not. Before listening, the measures already show the two stages erring in opposite directions above 2 kHz: the model's bridge carries 10–20 dB more at 5–6 kHz at the attack, its board and radiation take 10–20 dB off 2–5 kHz at the ears. The same for a whole passage once the engine can write the per-key bridge forces.
8. **The bridge stage alone.** Plath's channel 2 against the model's bridge force for the same key: the attack's pulse train, the force's rise (channel 1 gives the key's impact for the timing), the first 100 ms of every partial. docs/bass_metal.md already read the chirps there; make it a measure (per-period pulse decay, the energy above 4 kHz in the first 20 ms).
9. **Space: 3D-MARCo.** The same two piano performances captured at about sixty microphone positions at once (heights, distances, angles, a dummy head, an ORTF pair, spot microphones). The ratio of the long-term spectra at two positions is the ratio of the transfers from the piano to those positions, the piece cancelling. The model, rendered with the same geometry, gives the same ratios: a test of the radiation, the lid and the distance laws that every listening preset depends on, which no isolated-note set can give.
10. **Pieces with the pedal.** Liang's pedal/no-pedal pairs through axes 1 and 2 (the halo's spectrum and the fixed lines while the pedal is down), note by note as `tools/pedal_bench.py` does, so that the "rounder resonance" is measured where the user hears it, in Beethoven and Chopin with the pedal.
11. **The ear's own distance, and blind trials.** Two guards: a learned distance (an audio embedding, Fréchet distance between the model's note set and each real set against the distances between the real sets) that may move when the hand-made measures saturate; and, for the user's verdicts, ABX trials with repeats and a hidden reference, since one A/B batch this week was decided by the room rather than the parameter.
12. **Binaural at the player.** BiVib and Plath are heard at a dummy head; the model's `player.toml` has two bare omnis. A head (shadow and the ears' spacing) in the listening would make those two sets comparable on the stereo measures as well.

## Leads for the model, ranked by the evidence above

1. **The undamped treble's halo** (axes 1 and 2, the bench's top bands): 10–15 dB too weak, and the one-way gain is not the lever since it is uniform. Candidates, each physical: the two-way coupling planned in docs/pedal_drone.md; the bridge as a beam carrying the middle's motion along its length to the treble end, which the regions do not do; the frame and capo bar driving the top strings at their other end (the frame is one-way to the microphones only); the duplex segments behind the bridge, undamped and absent. Verify with the fixed lines' level and the between-partials ratio of the treble. On 2026-10-30 (docs/bench.md, "Tuning the default on the off measures") the top's 1–2 kHz bands were found to be the undamped strings' halo alone, rung by the knock at the bridge; no preset value (sympathetic gain, dampers, keybed knock, Overkill) closes more than 4 dB of the 12–22 dB: the frame-to-board path or the duplex is the lever. Later that day (docs/treble_halo.md), against Plath's bridge acceleration: the hammer's blow at the bridge is right, the frame path is 32–34 dB too weak to count, and the halo's start matches with the physical gain 1, which the free strings' brake now allows; what remains is the top strings' fast first decay.
2. **The board above 1–2 kHz** (axis 3): 11–13 dB too little per unit bridge motion at 3–5 kHz, and too regular. The damping law first (η flat 0.02 was heard as no change on the bass's tube, but the bench lost 0.17 through the middle's and treble's attacks: the board's radiation above 2 kHz and whatever loss replaces it are two separate knobs); then the ribs, which docs/outline.md planned and which the spread above 2 kHz now quantifies.
3. **The board at 400–800 Hz** (axis 3): 4–8 dB too much per unit bridge motion, and 4–5 dB more uneven than the real piano below 1 kHz. Same lever as 2 (ribs, two bridges); the crown and the edge loss were set on other measures.
4. **The hammer** (axis 4): the felt's velocity dependence partial by partial, where the uncommitted contact-per-string work belongs; rebase it on the outlined board and score it on this axis as well as on the bench.
5. **The damped strings' halo in Full** (axis 1): Overkill fills the bass's between-partials by 4 dB towards the real median. The dampers' strength and the cost of letting damped strings listen are the open questions noted in docs/performance.md.
6. **The bass "tube" and "rounder resonance"**: no more parameter A/Bs until the stage swap (7) says which stage carries them.

## Making the axes part of the bench

Each of axes 1, 2 and 4 is a few lines in `tools/bench.py`'s `MEASURES` with a floor (3 dB for the ratios, 3 lines for the counts) once `tools/bench.py refs` has measured the sets; the references are the same eight grands. Axis 3 needs no rendering (the example runs in 0.2 s) and its reference is one piano: use its own spread as the floor. The measures above the current ones would have flagged this week's candidates where the bench stayed silent: the treble's halo, the board's shape above 2 kHz, the hammer's velocity dependence.
