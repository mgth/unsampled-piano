# The board's forced radiation at the partials' frequencies

Status as of 2026-11-21. Option `soundboard.forced_radiation` (off by default; the shipped preset is unchanged bit for bit). Code: `crates/piano-core/src/forced.rs`, `Soundboard::forced_field` / `forced_direct` / `radiation_parts` (soundboard.rs), `ModalBank::follow` (modal.rs), `Voice::reserve_forced` / `mix_forced` (voice.rs), the engine's `forced_mics`. Measure: piano-render's `forced_radiation` example. Candidates in `out/presets/p5/`, listening page `out/ab_p5/` (neither in the repository). Nothing here has been listened to.

## The question

The board is a bank of modes, and each mode reaches each microphone through one complex weight: the Rayleigh integral of its shape at the mode's own frequency ω_n (`Soundboard::radiation`). That is exact for a mode ringing freely. The sustained sound of a note is the board's forced response at the strings' partials: a partial at ω drives every mode off its resonance, and what those modes radiate together at ω depends on the wavelength at ω (over a 2.1 × 1.5 m board the path phases k·r change by many radians between 300 Hz and 3 kHz). docs/board_radiation.md found the exact sum 13–39 dB under the per-mode approximation at grazing positions on an earlier board. The stage swap (docs/stage_swap.md) singled out the board and its radiation, and the bass came nearest a real grand by each partial's level at the ears (docs/bass_gaps.md, "After listening").

## Step 1: the size of it

For every third key (A0, C1, … C8) and each of its partials up to 8 kHz (1201 partials; the top keys' partials are those of the speaking length), the transfer from the key's bridge force (at its own bridge point, every board mode, the bank's own one-poles) to each microphone at the partial's frequency, two ways: as the engine renders it (each mode's weight at ω_n, acceleration weights, delay zones) and exactly (every mode's weight at ω on the same modes' velocity response). The case's field (`case_field`) is the bank's in both (below). Level: power summed over the set-up's microphones, exact − engine (dB). "Direct" is the board's direct field alone; "heard" adds the case's field, which is what the microphones get.

By register, heard (mean / rms dB), and the direct field alone (rms):

| set-up | A0–B1 | C2–B2 | C3–B3 | C4–B4 | C5–B5 | C6–B6 | C7–C8 | all | direct alone, all | profile rms (note mean removed) |
|---|---|---|---|---|---|---|---|---|---|---|
| close pair (`close.toml`) | −0.8 / 4.3 | −1.9 / 5.1 | −0.3 / 3.5 | −0.7 / 3.6 | −0.7 / 4.4 | −1.6 / 3.6 | −1.2 / 3.4 | −1.0 / 4.3 | −1.8 / 5.8 | 4.0 |
| player (`player.toml`, lid 35°) | −1.0 / 5.2 | −1.4 / 5.2 | −1.7 / 4.8 | −0.2 / 4.1 | −0.1 / 3.8 | −0.3 / 3.8 | +0.1 / 7.6 | −1.1 / 5.0 | −4.3 / 9.3 | 4.8 |
| concert pair (`concert.toml`) | −0.3 / 4.6 | +0.2 / 4.3 | +0.8 / 4.1 | −0.3 / 4.9 | +1.3 / 3.7 | −0.9 / 4.2 | −4.3 / 5.2 | 0.0 / 4.5 | −1.0 / 6.4 | 4.3 |
| the preset's own (mixed) | −0.7 / 3.9 | −1.7 / 4.7 | −0.2 / 3.2 | −0.6 / 3.4 | −0.6 / 3.8 | −1.6 / 3.5 | −1.2 / 3.4 | −0.9 / 4.0 | −1.6 / 5.2 | 3.7 |

By third octave, heard (mean dB; the direct field alone in brackets where it differs by more than 2 dB):

| Hz | 50 | 100 | 200 | 315 | 400 | 630 | 1000 | 1600 | 2000 | 2500 | 3150 | 4000 | 5000 | 6300 | 8000 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| close | −2.8 | −0.3 | +2.3 | 0.0 | −0.6 | −1.4 | +0.5 | −0.8 | −1.7 | −1.6 (−3.4) | −1.5 | −1.2 | −2.0 (−3.9) | −0.4 | −0.6 |
| player | +5.5 | +1.9 | +2.7 | +4.5 | +4.9 | +1.7 | −0.3 | +1.8 | −2.4 | −1.1 (−3.2) | −1.9 (−5.3) | −2.4 (−5.3) | −1.5 (−4.0) | −3.0 (−11.7) | −2.7 (−16.3) |
| concert | −2.7 | +2.7 | −1.4 | +4.3 | +3.4 | −0.1 | −0.5 | −0.7 | −1.2 | −0.6 | +0.8 | +1.3 | +1.1 | −1.9 (−5.5) | −2.2 (−8.5) |

- **It is not small**: 4–5 dB rms per partial at every set-up, against the 2 dB the brief set as the threshold. But it is mostly a reshuffle of single partials, the same kind and size as moving the board's fine structure (6–8 dB rms by itself in this model): the mean bias stays within ±2 dB in every register at the close pair, within ±5 dB in every band at the player (+3 to +5 dB at 300–500 Hz, −2 to −3 dB above 2 kHz). Against the model's known per-partial gap at the ears (8.7 dB rms, docs/bass_gaps.md, where two states of the same real piano differ by 4.9), the change is about as large as the real piano's own year-to-year spread, and not a direction: nothing in these numbers says the exact sum closes that gap.
- **The direct field alone moves much more at grazing positions** (player: −5 dB at 3–5 kHz, −12 to −16 dB at 6–8 kHz, as docs/board_radiation.md found), but there the case's field, identical in both, is as loud as the direct field or louder above 1 kHz (−2 to +3 dB re the engine's direct field at the player above 1 kHz, +2 to +3 dB at 1–2 kHz at the close pair), so the heard change is a third of it.
- **The inter-microphone phase** is reshuffled: the median |Δφ| between the pair's two microphones is 2–8° below 100 Hz and 35–105° from 200 Hz up at the close pair, 5–20° up to 600 Hz and 50–90° above 1 kHz at the player: above a few hundred hertz the pair's phase relation per partial is as good as redrawn.
- **The partials that move most** (heard): bass partials at 3–7 kHz at the close pair (D#1/31 at 1.29 kHz −20 dB, D#2/49 at 4.25 kHz −18, C2/55 −18), single low partials at the player (A2/3 at 330 Hz +23 dB, F#2/4 +19, D#2/24 at 1.9 kHz −23, F#1/103 at 7.4 kHz −24). No register stands out.

**What each part of the radiation contributes, and whether "at the driving frequency" means anything for it** (`--parts`; exact − engine of each part alone, its power re the engine's whole direct field):

- *Front face (through the case, shaded by the scattering layer)*: the bulk of the direct field (0 to −3 dB of it). It moves by 3–7 dB rms per partial, its median −3 to −7 dB above 3 kHz at the player. Meaningful: a deterministic integral over the board's surface.
- *Back face and its wall image*: 4–14 dB under the front at the close pair and the player; moves like the front. Meaningful, the same integral from below.
- *Lid image*: 8–20 dB under the front at the close pair and the player, 20–50 dB at the concert pair. At the player it moves the most (5–14 dB rms per partial, −11 to −16 dB median at 6–8 kHz). Meaningful: a mirrored source, a deterministic path.
- *Scattering layer* (`scatter`, 0 in the preset): obstacles that add in power, each with a fixed pseudo-random phase. "At the driving frequency" has no meaning for a phase that is drawn, not computed; left to the bank (the option needs it off to apply at all: `Soundboard::forced_field`).
- *The case's field* (`case_field`): statistical by design, one pseudo-random phase per mode and microphone, its level from the mode's radiated power. No meaning either: it stays the bank's, in both columns above.
- *The frame and the rim*: banks of their own (`structure::frame_modes`, `RimModel`), driven at the agraffes and by the board's edge forces, also weighted at their own modes' frequencies. The same question applies to them in principle; not measured here (their levels are 20 dB or more under the board's, docs/frame_rim.md), and left as they are.

## What was coded

The decision: implement it, as the brief's step 2 describes, because the change is over the threshold everywhere; but only as an option, since nothing above says it is a correction towards the real instrument (and the Plath check below says it is not, on the whole).

- **`forced::Forced::delta`**: for a key's bridge shapes and a frequency ω, ΔH(ω) at each microphone = Σ_n V_n(ω)·R_n(ω) − the bank's transfer, the sum over the built board's modes (the 1104 finite-element modes below 6 kHz on the concert grand), V_n the bank's own one-pole velocity response to the force, R_n(ω) their direct field at ω, the bank's transfer as the engine renders it (weights at ω_n as acceleration weights, each delay zone with its delay, each mode driven as the engine drives it: at the key itself below `soundboard.exact_below`, above it at the centre of the key's region, and with `exact_below` at 0 the force's band below 400 Hz shared between the two regions around the key, `Forced::drive`). V_n is always the response to the force at the key's own point: where the engine drives a mode from a region's centre, the correction also puts the partial back at the level the key's own point gives. On the concert grand every built mode lies below `exact_below` (6 kHz both), and the two drives are the same. The modes above the built board's (the plate modes above 6 kHz, 1020 of them) keep their weights: the correction does not touch them (on the player's seat lid off, fixed and full-exact transfers differ by under 0.3 dB per third octave up to 6.3 kHz, 1 dB at 8 kHz: below).
- **The cost** of R_n(ω) for every mode at every partial (0.25 s per frequency on 16 threads for 2124 modes) is avoided: `Soundboard::forced_direct` sums the modes' velocities at the finite-element nodes once (the board's velocity field at ω) and integrates that once over the paths to the microphones. It gives back each mode's own integral to 1e-9 (test `the_node_sum_is_each_modes_integral`). No frequency grid is tabulated: each key's clusters get their exact value.
- **`forced::corrections`** computes ΔH for every key's every cluster of coupled modes below 8 kHz (`FORCED_MAX_HZ`; a cluster is one partial of the unison, its mean frequency), in parallel, and caches the table on disk (`~/.cache/unsampled-piano/forced/<hash>.bin`, the hash of the board's parameters with the microphones, the sample rate and every cluster's frequency; 0.4 MB for the preset's own microphones).
- **Rendering**: each key gets, per pair of microphones, a `ModeTable` of its coupled modes with no input and as outputs ΔH·w·(mic gain), w each mode's bridge-force weight. The voice holds one more `ModalBank` per pair, which takes the main bank's poles, states and length every block (`ModalBank::follow`) and renders the two outputs from them: the correction rides on the strings' own states, the board's bank renders all it did before. It fades in over 3 ms after the hammer leaves the strings (`voice::FORCED_RAMP`; during the contact the bank does not run, and a correction starting at once was a step). It is added to each microphone's zone-0 slot in pressure, after the engine's differentiator (added before it, as acceleration weights, its start was a step differentiated into a click 25 dB over the note's peak: found on the listening page's first renders, fixed before any measure below). No allocation on the audio thread: the banks and buffers are reserved when the engine is made (`Voice::reserve_forced`).
- **What it leaves to the board**: the hammer's contact (a few ms), the longitudinal force and the phantom partials, the knock, the bridge's rocking (out3, `bridge_rocking`), the sympathetic banks and the rear lengths (their lines force the board too; they stay uncorrected, a design choice: they are quieter and run with their own tables), the duplex lengths above 8 kHz. The room's feed (its probes and `board_mics`) does not get the correction: the probes' own weights are per mode as before, and the engine adds the correction to the microphones after the room, which takes what they hold beyond the board's part as its source (`tests/forced_radiation.rs`: what the room adds is the same with the option on and off). The delay zones: the correction is a steady complex gain per partial; it has no zones of its own, its phase holds the paths.
- **A live change** that moves the strings' frequencies or the board or the microphones changes the hash: the table is recomputed on the next rebuild.

Costs, Ryzen 9 9950X, renders pinned to 16 cores:

| | shipped preset (off) | A+ (on), table computed | A+, table on disk |
|---|---|---|---|
| whole instrument build (`build_parts`, a fresh process) | 2.17 s | 7.36 s | 2.18 s |
| engine, one thread, whole keyboard ff with pedal (`perf_audit`) | 1.9× real time (mean 52.7 %) | 1.6× (63.9 %) | |
| glissando, no pedal | 2.35× (42.5 %) | 2.0× (49.4 %), p99.9 88 → 112 % | |
| 10-key chord ff, bass/mid | 2.9× (34.6 %) | 2.7× (36.8 %) | |

The preset's own set-up has more than two microphones (two pairs of correction banks per voice); with the close pair alone it is one.

Tests (`forced.rs`): the bank plus ΔH is the exact transfer with every mode driven at the key, every mode at the regions' centres, and the low ones at the key and the others at the regions (`the_bank_is_subtracted_as_the_engine_drives_it`); with the switch off no key carries a table (`off_adds_nothing`; the shipped preset's score is unchanged, below); on, the bank plus ΔH is the exact transfer to 1e-6, where the bank alone is off, and a steady sine force rendered through the correction bank comes out at ΔH's level and phase within 1 % (`a_steady_sine_reaches_the_mics_at_the_exact_transfer`); a mode driven alone at its own frequency gets a correction within its damping (2 % + 2σ/ω of its own transfer: the bank radiates its response's negative-frequency part with the conjugate weight, `a_mode_at_its_own_frequency_is_unchanged`).

## Results

**Partial levels at the close pair, rendered** (every third key, velocity 80, dry room, 0.05–1.0 s, partials within 50 dB of the note's strongest), on − off:

| | A0–B1 | C2–B2 | C3–B3 | C4–B4 | C5–B5 | C6–B6 | C7–C8 | all |
|---|---|---|---|---|---|---|---|---|
| A+ − A, mean | −0.9 | −1.3 | −0.5 | −1.3 | −0.8 | −0.5 | −0.8 | −0.9 |
| A+ − A, profile rms (note mean removed) | 4.5 | 3.6 | 2.6 | 3.1 | 3.0 | 2.1 | 2.7 | 3.9 |
| H+ − H, profile rms | 4.2 | 3.5 | 2.4 | 3.1 | 3.6 | 2.4 | 2.7 | 3.7 |

By third octave (A+ − A, mean): +2.3 dB at 125 and 200 Hz, −0.4 to −1.7 at 400 Hz–2.5 kHz, −1.7 to −2.0 at 3.2–4 kHz. The renders follow step 1's prediction (profile 4.0, mean −1.0).

**The comparison tool** (`--score`, measures version 9, band without bivib_grand, iowa, liang; off / mean):

| | A (shipped) | A+ | H | H+ |
|---|---|---|---|---|
| 24 measures | 865 / 1.194 | 855 / 1.183 | 753 / 1.136 | 764 / 1.134 |
| the 22 before the early-decay ones | 760 | 753 | 661 | 671 |
| bass / middle / treble / top | 205 / 364 / 227 / 69 | 212 / 361 / 215 / 67 | 218 / 252 / 215 / 68 | 223 / 258 / 214 / 69 |

A is 865 again with the switch's code in (the same counts per measure). What moves, A+ − A (off): rise time −6 (bass −5), brightness at the attack −5 and in the sustain −10 (bass −6, treble −3), lines 1.2–2.5 kHz late +8 in the bass and −5 in the middle, lines 2.5–4 kHz late +4 in the bass, release drop 0.6–1.2 kHz +5, spectral tilt +3; the others by 3 or less. H+ − H: rise time −6, brightness at the attack −5, lines 2.5–4 kHz +5 to +7 (bass, middle), spectral tilt +5, lines 1.2–2.5 kHz late +5 in the bass and −7 in the middle. (Scores of the A+ and H+ columns taken again on 2026-11-22 with the correction added after the room; with it in the room's source they were 848 / 1.182 and 759 / 1.130.) Within what the board's fine structure moves by itself: neither candidate is a step.

**Against a real grand** (Plath's concert grand, anechoic, the player's seat, lid off: ear pressure per bridge acceleration, read in the attack window, 0–0.3 s up to E2, 0–80 ms above; median per third octave, the model's partials of every third key against the real one's of every key, model − real; the real piano's two states a year apart differ by 1.0 dB rms of shape here):

| Hz | 125 | 160 | 200 | 250 | 315 | 400 | 500 | 630 | 800 | 1000 | 1250 | 1600 | 2000 | 2500 | 3150 | 4000 | 5000 | 6300 | 8000 | shape rms 200 Hz–4 kHz |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| off (the bank) | −8.9 | −5.6 | −8.5 | −12.8 | −3.6 | +0.2 | −0.3 | −4.7 | +0.4 | −2.1 | −2.0 | +2.3 | +5.1 | +2.7 | −1.3 | −6.0 | −3.9 | −1.3 | −2.1 | 4.4 |
| on | −8.0 | −1.8 | −6.2 | −11.8 | −2.1 | +1.9 | +1.4 | −0.3 | +0.3 | −1.6 | +0.5 | +3.4 | +2.1 | +0.8 | −6.4 | −9.8 | −8.2 | −6.9 | −7.9 | 4.6 |
| every mode exact | −8.0 | −1.8 | −6.2 | −11.8 | −2.1 | +1.9 | +1.4 | −0.3 | +0.3 | −1.6 | +0.5 | +3.4 | +2.1 | +0.5 | −6.3 | −9.8 | −7.9 | −7.1 | −9.0 | 4.5 |

On brings 160 Hz–2.5 kHz nearer the real grand (630 Hz −4.7 → −0.3, 2 kHz +5.1 → +2.1) and takes 3–8 kHz further from it (−1 to −6 → −6 to −10 dB), the overall shape the same (4.4 → 4.6 dB rms). Only the built board's modes matter: the full exact sum and the option agree within 0.3 dB up to 6.3 kHz. Pitfalls as known: a third octave's median is firm to about 2 dB, nothing below 38 Hz is read.

## What is not explained, what is not done

- The physical claim stands (the exact forced response is what a linear board radiates), but the model then loses 5 dB at 3–8 kHz towards the player where the real grand has it: what the real grand radiates there is not the board's direct field (docs/board_radiation.md already concluded that, with the scattering layer as the stand-in). With the option on, the case's field and the frame carry more of the treble's level; whether that is right is not known.
- Not listened to. The page `out/ab_p5/` holds A, A+, H, H+ at the close pair in the studio, the rows of `out/ab_p4/` and A1, D2, A2 held forte.
- The frame's and the rim's banks, the sympathetic banks, the rear lengths and the room's probes keep their per-mode weights. The hammer's contact has no correction (the strings' bank does not run then).
- `tests/realtime` passes in debug, but its model has the option off: the forced path's absence of allocation follows from the code (preallocated banks and buffers), not from that test.
