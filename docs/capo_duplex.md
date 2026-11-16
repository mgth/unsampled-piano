# The capo bar, the front duplex and the rear duplex

Status as of 2026-11-15. Code: `duplex.rs` (the string on three supports), `FrameParams::duplex*` (`structure.rs`), `NoteModel::build_with` (`note_model.rs`), `ModeShape` (`hammer.rs`). Analysis: `piano-render`'s examples `capo_probe` and `frame_level`.

On a concert grand the two top sections of strings (keys 54–88, D5–C8) have no agraffes. They pass under a bar cast in the iron frame, the capo bar, and run on for 34 to 48 mm to a short bearing bar, six unisons to each bar (docs/string_sections.md, "The keyboard side"). The Model page drew all that since `frame.capo_bar`; the engine held every string's far end the same way. Three questions were put, each on its own:

1. the capo bar is stiffer than an agraffe: do its strings lose less to the frame, and sustain longer?
2. do they excite the frame less, and is the frame heard at all in the treble?
3. the short length in front of the bar, the front duplex, rings on a real grand: can it be modelled honestly?

A fourth was added the day after: the strings' rear lengths behind the bridge (section 4).

In short: 1 and 2 have nothing to give on this model (the frame is already far stiffer at the strings' far end than any bar, and silent, but for the seven top keys where it rings for another reason); 3 is real, measured on a real concert grand's bridge, and is now in the engine behind `frame.duplex`.

## 1. A stiffer end under the capo bar: nothing to gain

**How the frame enters the strings.** Each partial of a key is coupled to the others of its unison through the admittance its two ends see: the board's at the bridge and, since `frame.coupling`, the frame's at the far end, on the line `frame.agraffe_x` (`FrameAdmittance::at`, added in `NoteModel::build_with`). The real part of that sum, times T/L, is what the partial loses to its supports; the string's own losses (`losses`) come on top.

**What the frame takes today.** `capo_probe` builds each key's coupled modes with the frame and without it and prints each partial's decay rate. On the Concert grand, keys 74–108 (dB/s, mean over the unison's modes):

| keys | partial | the string's own | with the board | with the board and the frame | the frame's share |
|---|---|---|---|---|---|
| D5–G6 | 1 | 7.1 | 17.6 | 17.6 | 0.0 % |
| D5–G6 | 2 | 12.9 | 20.4 | 20.5 | 0.1 % (0.7 at most) |
| D5–G6 | 4 | 37.3 | 45.3 | 45.6 | 0.6 % (1.4) |
| G#6–C8 | 1 | 28.2 | 46.7 | 47.0 | 0.7 % (3.0) |
| G#6–C8 | 2 | 75.3 | 91.4 | 92.5 | 1.2 % (1.8) |
| G#6–C8 | 4 | 303.4 | 320.1 | 320.7 | 0.3 % (0.7) |

The frame takes 3 % of one partial's decay at most, 1 % on average: a bar of infinite stiffness would lengthen the top notes by less than 1 dB/s out of 47. The slowest mode of each unison, the one that makes the aftersound, is the string's own loss with or without the frame (7.1 and 28.2 dB/s for the fundamentals).

**Why.** The frame's conductance on that line is 4·10⁻⁹ s/kg at 500 Hz, 1·10⁻⁷ at 2 kHz and 1 to 4·10⁻⁵ above 3 kHz, where the board's at the bridge is 2 to 10·10⁻⁴: the plate is clamped 5 cm away, a small share of its bending wavelength below 3 kHz. As an impedance that is 10⁷ kg/s falling to 3·10⁴–10⁵ kg/s above 3 kHz. A capo bar taken as a cast-iron beam of 30 × 50 mm (11 kg/m, EI 3·10⁴ N·m²) has, as an infinite beam, 2.5·10⁴ kg/s at 2 kHz and 3.5·10⁴ at 4 kHz: no stiffer than what the strings already bear on. The model's far end errs on the stiff side already; the earlier trial that put the agraffes at their real place along the plate erred the other way (18 dB too much drained).

**Not implemented.** There is nothing for a parameter to act on. The sustain of the top two sections is set by the strings' own losses and by the board.

## 2. The frame's sound in the treble

`frame_level` renders a note with the frame and without it (no room, no noises, velocity 100) and gives the level of the difference re the note. With `frame.coupling` 0 the difference is what the frame radiates and nothing else (with the coupling on, the partials move by a fraction of a hertz and the difference rises to −35/−45 dB, which is not a sound of the frame).

| keys | first 0.2 s | 0.5–1.5 s | loudest key |
|---|---|---|---|
| D4–C#5 (agraffes) | −55 dB | −62 | −51 |
| D5–G6 (capo) | −53 | −53 | −45 |
| G#6–E7 | −48 | −47 | −42 |
| F7–C8 | −15 | −16 | −9 (B7, C8) |

- **Up to E7 the frame is not heard**: 42 dB under the note at the loudest key, where the board's own radiation changes by more than that from one key to the next. A stiffer bar would lower a level that is already nowhere.
- **The seven top keys are another matter, and not the capo's.** From F7 up the fundamental lies above 3 kHz, where the frame plate has the rectangle's modes instead of its outline's own (`FRAME_OWN_MODES_BELOW`, docs/frame_rim.md): its conductance at the strings' end jumps a hundredfold there, and the frame comes 9 to 27 dB under the note, on keys whose own fundamental is weak (the board's front corner, docs/string_sections.md). A bar's own impedance on top of the plate's would take 2 to 3 dB off that. The cause is the plate's modes above 3 kHz: with the outline's own modes up to 6 kHz (a trial build, not kept) the same keys' frame falls to −25 (C8) to −69 dB. To be taken up on its own: it costs 0.1 s of build and changes the seven top keys.
- The trial preset "4 rigid silent frame" (`frame.gain` 0, `frame.coupling` 0) is the upper bound of points 1 and 2 together, to listen to.

**Not implemented.**

## 3. The front duplex

### What couples the short length to the note

Two paths, very unequal:

- **The capo's own motion.** The string's wave impedance √(Tμ) is 1.7 kg/s; the frame's mobility at the bar is 10⁻⁵ s/kg at most. A wave of the speaking length shakes the short length through the bar's motion by 2·10⁻⁵ of its amplitude (−94 dB). Nothing.
- **The wire's stiffness.** The capo holds the string down on an edge: no displacement, but the wire is one piece and its slope and bending moment carry through. For a stiff string (EI y'''' − T y'' = μ ω² y) held at one point, a wave of wavenumber k passes with the amplitude k/√(k² + κ²), κ² = k² + T/EI. On C7's strings (0.83 mm, 670 N) κ is 380 rad/m: 4 % of the amplitude passes at 1 kHz, 8 % at 2 kHz, 20 % at 5 kHz. This is the path.

### The string on three supports

`duplex::modes` solves one stiff string on three supports: the bridge and the bearing bar (pinned), the capo in between (no displacement, slope and moment continuous). Its modes are the roots of

    k·(cot kL + cot kl) = κ·(coth κL + coth κl),

L the speaking length and l the short one. Each mode has a share of its motion on either side:

- **the partials** keep nearly all of theirs in the speaking length (half where a mode of the short length falls on one). They sit 9 (D5) to 40 cents (C8) above the plain string's, all nearly alike: the tuner's business, so every frequency is taken re the first partial. What is left between partials is small (a few cents) except where a mode of the short length falls near one;
- **the short length's modes** (3.9–6.6 kHz for the first, then 8–14 kHz and up) have 2 to 50 % of their motion in the speaking length: 2 % when they fall between two partials, half when they fall on one, which they then share and push away (G7's second partial: +60 cents on one side, the duplex mode 50 cents on the other).

With the lengths the Model page draws (48 to 34 mm along each bar, read off a photograph of a D within 5 mm), the first mode of the short length, key by key (`out/capo/duplex_modes.py` in the scratch; b, L, T from the preset's scale):

| key | L (mm) | l (mm) | first mode (Hz) | in units of the note's fundamental | its line re a partial that would stand there |
|---|---|---|---|---|---|
| D5 | 309 | 48.0 | 3990 | 6.8 | −11 dB |
| G5 | 236 | 34.0 | 5970 | 7.6 | −7 |
| G#5 | 224 | 48.0 | 4060 | 4.9 | −11 |
| C#6 | 171 | 34.0 | 6080 | 5.5 | −8 |
| D6 | 162 | 48.0 | 4170 | 3.5 | −15 |
| G6 | 124 | 34.0 | 6130 | 3.9 | −11 |
| G#6 | 117 | 48.0 | 4260 | 2.6 | −17 |
| C7 | 94 | 36.8 | 5740 | 2.7 | −15 |
| D7 | 85 | 48.0 | 4350 | 1.8 | −15 |
| G7 | 65 | 34.0 | 6360 | 2.0 | −6 |
| G#7 | 61 | 48.0 | 4490 | 1.3 | −19 |
| C8 | 49 | 34.0 | 6620 | 1.5 | −18 |

The frequencies rise along each bar and fall back at the next: a saw of six keys.

### A real concert grand has them, where and as strong as computed

Plath's Steinway (bridge accelerometer, every key at f; the corpus of docs/treble_halo.md), first 150 ms of each note: the strongest line that is not one of the note's partials, within −14/+6 % of the computed first mode (`out/capo/duplex_lines.py`):

| key | computed (Hz) | real grand: Hz, dB re the note's strongest partial, decay |
|---|---|---|
| D6 | 4166 | 4250, −35, 66 dB/s |
| D#6 | 4457 | 4540, −37, 78 |
| E6 | 4789 | 4720, −45, 85 |
| F6 | 5170 | 4989, −32, 85 |
| F#6 | 5613 | 5389, −33, 109 |
| G6 | 6128 | 5749, −31, 81 |
| G#6 | 4257 | 4110, −39, 21 |
| A6 | 4555 | 4311, −32, 52 |
| A#6 | 4895 | 4718, −28, 70 |
| B6 | 5285 | 5039, −31, 90 |
| C7 | 5739 | 5296, −31, 102 |
| C#7 | 6271 | 5718, −36, 105 |

The saw is there: the line climbs by a semitone or so per key along six keys, D6 to G6, drops back at G#6 (the first key of the top section), climbs again to C#7. Its frequency is within 2 to 9 % of the computed one, that is 1 to 4 mm on the short length, inside what the photograph gives, and on another instrument of the same model. Its level, 28 to 39 dB under the note at the bridge, and its decay, 50 to 110 dB/s, are the figures to hold the model to. Below D6 the lines found are weaker (−45 to −60 dB; the short length's mode falls among partials 5 to 7, themselves weak) and above C#7 irregular (−25 to −54).

Not a duplex, seen on the way: under the notes from E7 up the real bridge has lines at fixed frequencies whatever the key (3316, 3454, 3658, 3885, 3924, 4119, 4149 Hz, 20 to 50 dB under the note), which are neither partials of the note nor fundamentals of the free strings. They are not modelled and not explained here; the strings' rear lengths beyond the bridge are the first candidates.

### In the engine

`frame.duplex` (a switch, off by default; with `frame.capo_bar`). For each string of a key under the capo:

- its partials take the three-support string's frequencies re the first, its shape at the hammer and its slope at the bridge (`ModeShape`; a plain partial has sin(kπx/L) and k);
- the short length's modes below the bank's limit are added after the partials, one cluster per order (the first of each string of the unison, then the second …), with the shape they have in the speaking length: the hammer rings them and the bridge hears them by that share only;
- each mode loses, beyond the string's own, `frame.duplex_loss` times its share of motion in front of the capo: what the bearing bar and its felt take. 3.5 s⁻¹ (30 dB/s) by default, which with the string's own 30 to 90 dB/s at these frequencies gives the real lines' 50 to 110 dB/s;
- the bearing bars' places are parameters now, the ones the Model page draws: `frame.duplex_group` (6 unisons to a bar), `frame.duplex_far` and `frame.duplex_near` (48 and 34 mm). A unison's three strings stand a step apart along the slanted bar, 0.9 mm from one to the next: their short lengths are 2 % apart and do not beat as a unison does.

Left out, knowingly: the short lengths do not answer the bridge (another note cannot ring them: their share at the bridge is small) and do not pull on the frame (far too stiff there, section 1); the residual static push of the hammer is the plain string's; the dampers take a short length's mode by its share in the speaking length. The cost is 1 to 4 clusters more on 35 keys.

Off, and in any preset without the key, every render is the same to the sample (checked against the build before the change on C4 and C7; tests `the_front_duplex_adds_its_modes_under_the_capo`, `the_duplex_rings_under_the_capo_and_nowhere_else`).

### The model's lines against the real grand's

The same reading on the model's bridge acceleration (`bridge_signal`, velocity 100), the Concert grand with `frame.duplex`; without it the model has nothing there (−107 to −134 dB):

| keys | real grand: level, decay | model: level, decay |
|---|---|---|
| D5–C#6 | −44.8 dB, 72 dB/s | −42.5 dB, 71 dB/s |
| D6–C#7 | −34.1 dB, 83 dB/s | −46.5 dB, 107 dB/s |
| D7–C8 | −42.2 dB, 96 dB/s | −34.9 dB, 103 dB/s |

Mean level of the line re the note's strongest partial, median decay over the first half second. Key by key the model's lines go from −25 to −65 dB where the real ones go from −16 to −61: which partial the mode falls near, and whether the hammer sits on one of its nodes (D6: −65 dB), depends on the short length to the millimetre. The middle group is 12 dB under the real grand's and the top one 7 dB over; nothing was tuned to close that (the loss only, on the decay).

### On the notes: 88 keys, two velocities

88 keys at velocities 60 and 100, close microphones, no room (`out/capo/render_all.sh`, `capo_cmp.py`, `dscore.py` in the scratch). The keys below D5 keep their own strings; their halo of free top strings moves with those strings' partials, 54 dB under the note at C5 and 68 to 93 dB under it from C2 to C4 (not rendered again for the tables: the same files). Duplex on − off, mean over the section's keys (rms of the keys' differences), velocity 100:

| keys | attack 0–0.2 s | sustain 0.5–1.5 s | partial 1 | the note's decay | between the partials, 3.5–12 kHz |
|---|---|---|---|---|---|
| D5–G6 | +0.2 dB (0.25) | 0.0 (0.4) | 0.0 (0.2) | +0.5 dB/s (2.2) | +9.9 dB (11.0) |
| G#6–C8 | +1.5 (1.7) | +1.0 (1.7) | +1.3 (2.1) | +1.1 dB/s (6.2) | +10.3 (11.3) |

At velocity 60: attack +0.3 and +1.9 dB, sustain −0.2 and −0.7, between the partials +10.1 and +7.6.

- **The notes themselves hardly move** below G#6. In the top section the fundamental gains 1.3 dB: a string that runs on under the capo is not hinged there, its shape near the bar is flatter, and the hammer, 3 to 6 mm from the bar on those keys, meets more of the first partial.
- **The sustain is the same**: the note's decay within 1 dB/s on average, and the scatter from key to key (2 to 6 dB/s) is that of any redraw of the partials.
- **What changes is between the partials**, +10 dB from 3.5 to 12 kHz, 30–330 ms after the blow, re the note's whole energy:

| keys | off | on | with `duplex_loss` 0 | the commercial D | a real concert grand (dummy head) |
|---|---|---|---|---|---|
| D5–G6, velocity 100 | −61.8 dB | −51.9 | −48.4 | −57.5 | −44.7 |
| G#6–C8, velocity 100 | −44.4 | −34.1 | −30.5 | −33.7 | −21.7 |
| D5–G6, velocity 60 | −74.0 | −64.0 | −59.9 | −73.1 | (one dynamic) |
| G#6–C8, velocity 60 | −48.0 | −40.4 | −37.5 | −39.4 | |

The model was cleaner between its partials than both references in the top section and than the real grand everywhere; with the duplex it is at the commercial D's level in the top section and halfway to the real grand's (whose figure also holds its room-free noise and the other free strings' halo, docs/treble_halo.md). In the lower section at mf the commercial D has nothing there and the model now has 9 dB more than it.

**Distance to the commercial D** (`dscore.py`, all 88 keys): 8.19 → 8.11; top register 6.30 → 5.97 (its sustain at f 5.2 → 2.7 dB rms, its attack 9.0 → 8.4), treble 10.00 → 9.95, bass and middle unchanged. With `duplex_loss` 0: 8.15, top 6.07.

**Treble sustain against the references** (the note's decay over its first 25 dB, dB/s; velocity 100):

| keys | off | on | the commercial D | a real concert grand |
|---|---|---|---|---|
| D5–G6 | 50.6 | 51.1 | 37.3 | 26.7 |
| G#6–C8 | 45.1 | 46.2 | 53.2 | 39.7 |

The lower section's notes die faster than both references, with the duplex or without: that gap is not the capo's (section 1) nor the duplex's. It sits on the preset's present state (the board's front corner moved back, docs/string_sections.md).

### Kept or not

`frame.duplex` is on in the Concert grand: the lines are on a real concert grand's bridge where and as strong as the three-support string puts them, the notes' level and sustain do not move, the distance to the commercial D goes down a little, and what the model lacked between its partials in the treble is halfway filled. Off by default and in every other preset.

To be heard before it is taken for granted: `out/ab_capo/index.html` (scratch; twelve notes and three runs of six keys along one bar: the preset before, the duplex, the duplex without its loss as an exaggeration, the rigid silent frame of section 2, the commercial D and the real grand), and at the keyboard the trial instrument "Capo trial" (`target/release/unsampled-piano --ui --presets out/keyboard`: `1_preset` is the preset before the change, `2_front_duplex`, `3_front_duplex_long`, `4_rigid_silent_frame`). The bench of real grands was not run.

**Played at the keyboard on 2026-11-16** against the preset before the change: subtle, the user is not sure of hearing it. Kept.

## 4. The rear duplex

Taken up on 2026-11-16, after the user's remark that on the D the rear lengths of the top fifty keys or so carry no felt. Code: `sympathetic.rear`, `rear_from`, `rear_loss` (`params.rs`), `note_model::rear_modes`, the engine's `rear` banks.

### What a rear length is to the bridge

Behind the bridge each string runs on to a plate before its hitch pin. That length l, at the string's tension, is a short string with one end on the bridge: the bridge's motion shakes it, and it pulls back on the bridge, exactly as a free speaking string does (the engine's listening strings). Its modes stand at n·f0·L/l. Two things set its strength:

- its force on the bridge per unit of the bridge's acceleration goes as T/(l·σ·ω): the shorter, the stronger;
- but it loses to the bridge what the bridge's conductance takes, (T/l)·Re Y per string, and a unison's strings move together: 100 to 300 dB/s for 5 cm on this board, far more than the wire's own losses. A first trial without that loss had the rear lengths louder than the note after half a second.

With that loss in its decay, a mode driven by the bridge's motion as it is without it is the two-way answer for that mode alone (the loss to the bridge is the mode's own reaction on it). One way otherwise: the rear lengths do not change the strings' modes, and they push on the board that radiates the free strings' halo.

### The lengths

`soundboard.structure.back_length`, the table the board's stiffness already uses: an estimate (4 cm plus a tenth of the speaking length). Read again on the photograph of the D (1.10 mm a pixel, ± 4 mm; the bridge's dark stepped line to the bright stepped plates behind it, along the strings):

| keys | photograph | the table |
|---|---|---|
| C4–C5 | 9 to 12 cm | 10.5 to 8.1 cm |
| B5–D6 | 5.7 to 5.9 cm | 5.8 cm |
| B6 | 5.5 cm | 5.2 cm |
| E7 | 2.5 to 3.5 cm | 4.9 cm |
| A7 | 2.5 cm | 4.7 cm |

The table holds within the reading up to B6; the ten top keys' rear lengths look half as long as the estimate (an octave higher). Not changed: the table also sets the strings' hold on the board, and a reading of 25 ± 4 mm is not a measure.

On the real grand's bridge the rear lengths cannot be told from the speaking strings: in the median spectrum under the notes from C6 up, every line that stands 12 dB over its surroundings lies within 35 cents of a speaking string's first three partials (free strings, and strings a few tens of cents apart within Plath's unisons). A ladder of five lines a semitone apart, 3316 to 4119 Hz, under the notes from E7 up would be rear lengths of 62 to 50 mm; nothing more can be said from this recording.

### In the engine

`sympathetic.rear` (a switch, off by default). From `rear_from` (B3, key 39: the top fifty keys) each key's rear length gives its modes below `sympathetic.max_freq` to a bank of its board region: input the bridge's acceleration there, output a force on the board, whatever the dampers and the pedal do. `rear_loss` adds to their decay (0: the wire's own losses and the bridge's; nothing was found to set it on).

### Measured

Single notes of 1 s, then 2.5 s after the key-up (velocity 100): the rear lengths' sound is 22 to 32 dB under the note from C5 to C7 while it is held, 37 to 50 dB under it at C3–C4; after the key-up it is at the level of what already rings there, 73 dB under the note.

88 keys at velocities 60 and 100, against the same preset with the front duplex only. The notes' level, their partials and their decay do not move (attack and sustain within 0.1 dB on average, the note's decay within 1 dB/s). Between the partials, re the note's whole energy (velocity 100):

| keys | band, window | front duplex only | with the rear duplex | the commercial D | a real concert grand |
|---|---|---|---|---|---|
| C3–A#3 | 1–3 kHz, 30–330 ms | −74.2 dB | −68.9 | −54.0 | −52.0 |
| B3–C#5 | 1–3 kHz, 30–330 ms | −67.9 | −57.8 | −53.4 | −47.1 |
| B3–C#5 | 3–8 kHz, 30–330 ms | −76.1 | −69.6 | −63.7 | −48.9 |
| D5–G6 | 1–3 kHz, 30–330 ms | −53.3 | −46.4 | −42.0 | −40.5 |
| D5–G6 | 3–8 kHz, 30–330 ms | −51.0 | −48.7 | −56.2 | −43.3 |
| G#6–C8 | 1–3 kHz, 30–330 ms | −34.4 | −33.3 | −26.3 | −21.8 |
| B3–C#5 | 1–3 kHz, 0.5–1.5 s | −81.2 | −80.9 | −62.8 | −48.0 |
| D5–G6 | 1–3 kHz, 0.5–1.5 s | −50.4 | −50.4 | −47.7 | −41.1 |

- **In the first third of a second the middle and the treble gain 7 to 10 dB between their partials at 1–3 kHz**, where the model was 14 to 21 dB cleaner than both references; at mf the same (−67.6 → −60.4 at B3–C#5, the commercial D −59.3).
- **Nothing after half a second**: the rear lengths have died into the bridge. What the references keep between their partials later in the note (20 to 30 dB more than the model in the middle) is not the rear duplex.
- **Distance to the commercial D**: 8.11 → 8.13, every register within 0.05.

### Kept

On in the Concert grand, on the same grounds as the front duplex: the part exists and is bare on the instrument, the model moves towards both references where it acts and nowhere else. The preset gains a `[sympathetic]` section, which must say `brake = true` (a section without it reads the brake off; checked: with the section and without `rear` the render is the same file). Played at the keyboard by the user on 2026-11-16 (`5_rear_duplex` of the trial instrument against the front duplex alone): accepted.

## Sources

- The lengths, the grouping and the bars' slant: a photograph of a Steinway D from above, 1.10 mm a pixel (docs/string_sections.md); ± 5 mm on the short lengths.
- The real lines: N. Plath's recordings of a Steinway concert grand (bridge acceleration and dummy head, every key; the corpus of docs/treble_halo.md).
- The transmission past a point support of a stiff string: the beam-string equation, as in Fletcher and Rossing, *The Physics of Musical Instruments*, ch. 2 (stiff strings) and standard treatments of supports on beams; derived here, not taken from a paper on duplex scales.
- Askenfelt and Jansson, "From touch to string vibrations. III: String motion and spectra" (JASA 1993), compares string and bridge vibrations "including the influence of the resonant tail lengths of the string (duplex scale)": the abstract only was seen. Öberg and Askenfelt's study of duplex stringing (JASA, 2012) was looked for and not found: not read, nothing here rests on it.

## Not measured, not done

- The D's own short lengths: the photograph's, ± 5 mm, that is ± 12 % on the lines' frequencies. Which partial a line falls near on a given key is therefore not the real D's.
- The bearing bars' loss: one figure for every key, set on the decay of twelve lines of one real grand.
- The capo's edge has a radius and the string a down-bearing angle: a string held over 1 mm instead of a point passes a little less. Not modelled; the levels agree with the real grand's without it within the scatter.
- The rear lengths themselves: the board's estimate, read again on the photograph within 4 mm; the ten top keys' look half as long. Whether the rear plates are set to intervals of the notes is not known. The user's observation (2026-11-16) is that the top fifty keys' rear lengths carry no felt; the key where the felt stops was not counted on a photograph.
- The rear lengths' own loss at the plate (`rear_loss` 0), their stiffness at the bridge's pins, and any motion they give back to the strings.
- The frame above 3 kHz (section 2).
