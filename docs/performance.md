# Engine performance audit

October and November 2026. Where the engine's time goes, what makes it miss a deadline, what was fixed, which compromises smaller machines can take, and what multi-threading gives. Three rounds: the first (October 23) found the costs and spikes and fixed the worst; the second (October 24) added the Overkill reference level, dropped silent partials, spread the soundboard over threads and kept replaced engines from piling up; the third (November 22) took the shipped Concert grand, far heavier than the default instrument of the first two, and cut its load by a third to a half without changing an output bit. The sections up to "Fixes made in the first round" describe the state before the first round.

All figures come from a Ryzen 9 9950X (Zen 5, AVX-512) unless stated otherwise. Smaller machines are emulated by capping the SIMD level (`PIANO_ISA=avx2` or `generic`). This removes the vector width but not the clock or cache differences, so real small machines are slower still.

## How to measure

```
cargo run --release -p piano-render --example perf_audit -- \
    --midi-dir data/midi [--sr 48000] [--block 128] [--threads N] \
    [--quality overkill|full|balanced|light] [--variants] [--preset p.toml]
```

Every buffer is timed as the real-time host would see it. The load is the render time over the buffer's duration, so 100 % means the deadline is missed. The tool reports the mean, the 99th and 99.9th percentiles and the worst buffer. A host fails on the worst buffer, not on the mean. The single worst buffer is noisy (one preemption by the OS is enough), so p99.9 is the more reliable spike measure.

Scenarios:

- silence;
- pedal held with nothing played (88 strings listening);
- one note;
- a 10-key ff chord struck in one buffer (bass/middle, then treble);
- 10 keys re-struck every 60 ms;
- 30 keys with the pedal pumping;
- 30 keys struck in one buffer, again every 200 ms (the profile of the onset spike);
- a glissando without pedal;
- the whole keyboard ff with the pedal down (the old `stress_cost` case);
- the six MIDI performances of `data/midi`.

`--variants` repeats the run with each part of the physics switched off.

Other examples:

| Example | What it measures |
|---|---|
| `voice_ops.rs` (piano-core) | the per-event voice operations |
| `board_bench.rs` and `modal_kernels.rs` (piano-core) | the soundboard and string-bank kernels, on every ISA |
| `board_shapes.rs` (piano-render) | the three soundboard banks of a preset on the shapes the engine runs them with, and a hash of every output bit |
| `shortcut_diff.rs` (piano-core) | what each engine shortcut changes in the signal |
| `overkill_parts.rs` (piano-render) | what each of Overkill's differences from Full changes, on a MIDI file |
| `edit_load.rs` (piano-control) | the load while a parameter is edited during playing |

[`tools/bitexact`](../tools/bitexact/README.md) checks that a change leaves the sound alone: a fixed set of MIDI scenarios rendered offline with the shipped preset, compared byte for byte before and after (third round, below).

The `Bench` workflow runs the tests and `perf_audit` on GitHub's runners (macOS ARM, Linux, Windows). Measurements need an idle machine: another process rendering on a few cores was enough to double some means and to send p99s past 100 %.

## Starting point

48 kHz, 128-frame buffers, polyphony 128, full default model (room on):

| Scenario | mean | p99.9 | worst |
|---|---|---|---|
| silence | 1.5 % | 1.9 % | 2.2 % |
| pedal held, silent | 11.6 % | 29.7 % | 31.2 % |
| 10-key chord ff, bass | 21.6 % | 66.5 % | 70.8 % |
| 10 keys re-struck / 60 ms | 23.7 % | 58.5 % | 59.9 % |
| 30 keys, pedal pumping | 27.0 % | 83.9 % | **148 %** |
| whole keyboard ff, pedal | 38–43 % | 79 % | 79 % |
| Chopin, Nocturne op. 9 no. 2 | 24.5 % | 40.1 % | 49.1 % |
| Debussy, Reflets dans l'eau | 21.7 % | 39.2 % | 56.0 % |

On this CPU the mean leaves a wide margin. The spikes do not: one buffer in the pumping scenario takes 1.5 times its deadline.

### Where the mean goes

Profile of the whole keyboard ff and of the Chopin, rendering only:

| Part | Whole keyboard | Chopin |
|---|---|---|
| String banks, primary (struck strings, bridge + agraffe outputs) | 24 % | 16 % |
| String banks, sympathetic (listening strings, two inputs) | 12 % | 20 % |
| Soundboard bank, primary (16 inputs → 2 mics + 16 bridge velocities) | 17 % | 27 % |
| Soundboard bank, sympathetic + frame (2 outputs) | 8 % | 12 % |
| Phantom partials (longitudinal modes) | ≈ 25 % | ≈ 5 % |
| Room (early reflections + FDN) | 3 % | 4 % |
| Hammer contact | 1–14 % depending on how often keys are struck | |

Cost of each part, measured by switching it off (`--variants`, mean load, whole keyboard / Chopin):

| Switched off | Whole keyboard | Chopin |
|---|---|---|
| (full model) | 38.1 % | 23.3 % |
| room | 37.0 | 22.3 |
| sympathy | 29.9 | **11.4** |
| sympathy above 1.5 kHz only | 35.2 | 20.1 |
| phantom partials | **28.4** | — |
| frame and rim | 33 | — |
| horizontal polarisation | 32.2 | 19.2 |
| soundboard (bridge force out) | 16.8 | 2.1 |

In real music, sympathetic resonance is half the cost. With the pedal down, every string of the keyboard runs its partials below 5 kHz while the board moves.

### What makes the spikes

1. **Hammer contact.** While a hammer touches the strings, the [`ContactSolver`](../crates/piano-core/src/hammer.rs) advances every string mode itself. That is up to 768 modes, in f64, with two passes per step and a reduction in each, so it ran scalar. A bass strike costs about 5 % of a buffer for each of the 2 to 3 buffers its contact lasts. Ten keys in one buffer make 47 % on their own (measured without soundboard). Thirty make the 148 %. MIDI files quantised to a grid hit this often: a chord lands in a single buffer.
2. **Setting up a string** (`Voice::setup`, [`voice_ops`](../crates/piano-core/examples/voice_ops.rs)): 25 µs for a bass or middle key (≈ 1 % of a 128-frame buffer). Two banks of up to 768 modes get their poles recomputed with f64 `exp` and `sin_cos`. This happens on every note-on of a free voice, and 8 times per sub-block when the pedal goes down (listening strings are added: +16 % of a buffer for the first 30 ms). Striking a string that was only listening costs another 9 µs (`set_active_len` recomputes every pole).
3. **Dampers in motion.** `set_damping` calls a scalar `expf` per mode: 2 µs per voice and per sub-block while its damper moves. When the pedal is released with 88 strings ringing, that is ≈ 13 % of a buffer for each of the ≈ 60 sub-blocks the dampers take to settle.
4. **Small buffers.** With 32-frame buffers the mean is unchanged, but a strike's cost falls into one short buffer: chord 111 %, pumping 244 % (before the fixes below).
5. **High sample rates.** At 96 kHz everything runs twice as often, and the strings keep partials up to 0.45·fs = 43 kHz (+21 % modes). The mean goes ×2.1 (Chopin 49 %) and the spikes reach 200 to 350 %. Before the fixes, 96 kHz was not usable for dense playing, even on this CPU. After them, the Chopin holds (45 % mean, 59 % p99.9), but the whole keyboard ff still passes the deadline (132 % p99.9).
6. **Parameter edits while playing** ([`slot.rs`](../crates/piano-control/src/slot.rs)): an edit that rebuilds the instrument keeps the old engines playing their notes for up to 2 s. With up to 3 old engines next to the new one, the load is up to ×4 during that time. Not measured here, but it follows directly from the design.

### Smaller machines

The same runs with the SIMD level capped:

| Scenario (mean / p99.9) | AVX-512 | AVX2 | no AVX (SSE2) |
|---|---|---|---|
| one note C4 | 8.8 / 13 % | 8.7 / 13 % | 40 / 49 % |
| Chopin | 24.5 / 40 % | 31 / 60 % | 89 / 152 % |
| Debussy | 21.7 / 39 % | 27 / 67 % | 77 / 113 % |
| whole keyboard ff | 43 / 79 % | 49 / 70 % | 121 / 168 % |

These are measured at 5.7 GHz with Zen 5's caches. An AVX2 laptop at 3 GHz should be about 2× slower than the AVX2 column. That puts dense playing around 60 % mean, with spikes over the deadline.

The `Generic` path is the one **every ARM build uses**, including the universal macOS app on Apple Silicon. Its soundboard kernel was 4.3× slower than AVX2 (6.9 against 1.6 ns per mode and sample), more than the vector width explains. The new kernel (below) brings it to 2.8. Apple's cores are wide and may hide more of it, but nothing here measures them. Running `perf_audit` on a macOS ARM runner in CI would.

## Fixes made in the first round

Every fix keeps the model. Some sums are now taken in another order, and the damped poles are factored differently: e^{−d·w}·p₀ instead of e^{−(σ + d·w)}·e^{iθ}. Measured against an f64 reference over 200 000 random modes, both forms have the same decay-rate error (median 0.001/s, max 0.005/s), which is the f32 resolution of a pole. The renders before and after therefore differ only by that quantisation:

- −89 dB on the Bach, which has little pedal;
- −62 to −68 dB on the heavily pedalled Chopin and Rachmaninoff, where the dampers move all the time;
- at worst −52 dB in one 100 ms window.

Every test passes.

1. **Vectorised hammer contact** ([`hammer.rs`](../crates/piano-core/src/hammer.rs)). Modes are laid out in chunks of 8 f64 lanes. One pass per step instead of two: the next step's free prediction is computed with the update. The reductions are 8 lane sums in a fixed order, so the result is the same on every ISA. AVX2 and AVX-512 versions are dispatched at run time like the modal bank. Measured without soundboard, so that the strings alone are timed: ten bass keys struck in one buffer take 13.5 % of the buffer instead of 47 % (p99.9). Keys re-struck every 60 ms take 13 % instead of 36 %.
2. **Listening strings at rest are skipped** ([`voice.rs`](../crates/piano-core/src/voice.rs)). With the pedal held and the board asleep, a sympathetic bank whose inputs are all zero and whose energy is below its silence threshold is no longer run.
3. **Soundboard kernel rewritten** ([`board.rs`](../crates/piano-core/src/board.rs)). Before, every output's lane accumulators went through memory once per 16 modes. On AVX-512, LLVM also vectorised the generic code across time with gathers, 8× slower than AVX2, so AVX-512 machines were sent to AVX2. Now:
   - the chunks go by blocks of 4, so their state history (32 KB) stays in L1;
   - each block's outputs are computed by tiles of outputs × samples whose sums stay in registers across the block's chunks;
   - the recursions of several chunks are interleaved to hide their latency;
   - the impulse-invariance correction is vectorised over time (it was a serial sum of 16 terms per output and sample);
   - the code is written once over a 16-lane type with three implementations: AVX-512, AVX2 as two 8-lane halves, and plain arrays.

   Each output still sums its chunks in order and its lanes as a tree, so the result is bit for bit the old one, on every ISA (test `all_isas_are_bit_identical`). Cost per mode and sample, 16 inputs and 18 outputs:

   | ISA | before | after |
   |---|---|---|
   | AVX-512 | 1.62 (sent to AVX2) | **0.95** |
   | AVX2 | 1.62 | **1.2–1.4** |
   | none (SSE2, ARM) | 6.96 | **2.8** |
4. **Coefficients precomputed per key** ([`modal.rs`](../crates/piano-core/src/modal.rs), [`note_model.rs`](../crates/piano-core/src/note_model.rs)). Each key's `NoteModel` holds its primary and sympathetic bank coefficients, the latter with their delay-advanced gains, in the bank's own layout (`ModeTable`). `Voice::setup` copies them: **1.1 µs instead of 25** for a bass key. A listening string coming alive when struck takes 0.3 µs instead of 9. This removes the pedal-down spike (8 listening strings set up per sub-block) and part of every note-on.
5. **Vectorised damper update** ([`modal.rs`](../crates/piano-core/src/modal.rs)). `set_damping` now multiplies each undamped pole by e^{−d·w}. The exponential is computed in f32 within 2 ulp, written so that it vectorises: Cody–Waite reduction, a degree-7 series, and the exponent built in integer bits. It is dispatched to AVX2 and AVX-512. Cost: **0.2 µs per voice instead of 2.0**, so releasing the pedal over 88 ringing strings costs ≈ 1 % of a buffer per sub-block instead of 13 %.
6. **`PIANO_ISA`** ([`modal.rs`](../crates/piano-core/src/modal.rs)) caps the detected SIMD level, to measure smaller machines.

Tried and dropped:

- The phantom-partial products rewritten as 16-lane windows: the gain was within the noise, because the existing 8-accumulator code is already close to throughput.
- **Bridge velocities computed only where a string listens.** The board would skip the regions whose strings are all damped. Once the new kernel made those outputs cheap, it no longer gained anything measurable (≤ 1 point). It also departs from the model: a string struck in a skipped region misses the bridge motion of its first sub-block, which made a −44 dB difference in the Bach. Removed.

Result on one thread, from the same run for both columns (mean / p99.9 / worst):

| Scenario | before | after |
|---|---|---|
| 10-key chord ff, bass | 21.7 / 64.7 / 68.9 % | 18.4 / **29.3** / 29.9 % |
| 10 keys re-struck | 23.4 / 60.3 / 65.1 % | 18.1 / **29.7** / 30.2 % |
| 30 keys, pedal pumping | 26.5 / 87.1 / 144 % | 23.0 / **43.5** / 52.4 % |
| pedal held, silent | 11.8 / 32.7 / 32.7 % | 9.7 / 18.7 / 19.1 % |
| whole keyboard ff | 38.4 / 50.8 / 64.7 % | 35.3 / 42.7 / 46.7 % |
| Bach | 10.3 / 18.1 / 54.9 % | 8.4 / 11.3 / 13.0 % |
| Chopin | 24.5 / 39.1 / 47.4 % | 21.2 / 33.4 / 37.9 % |
| Debussy | 21.8 / 41.4 / 55.2 % | 18.6 / 28.4 / 38.7 % |
| Chopin, AVX2 | 29.9 / 45.9 / 63.1 % | 29.6 / 44.0 / 66.9 % |
| whole keyboard ff, AVX2 | 46.4 / 56.6 / 58.5 % | 45.7 / 56.0 / 58.3 % |
| Chopin, no AVX | 84.6 / 122 / 165 % | **55.0 / 73.5 / 85.2 %** |
| whole keyboard ff, no AVX | 115 / 141 / 144 % | **83.9 / 107 / 117 %** |
| Chopin, 96 kHz | 49.0 / 78.0 / 104 % | 45.0 / 58.9 / 94.4 % |

The onset spikes are halved on every ISA. The mean drops by 13 to 18 % on AVX-512 and by a third on the generic path. It hardly moves on AVX2, where the string banks dominate and were already near the hardware's limits.

## Second round (October 24)

### Overkill: the reference without shortcuts

A fourth quality level, `Quality::Overkill`, computes everything the model describes, without any of the shortcuts that change the signal, even those judged inaudible. It is the reference the other levels are measured against. Float rounding is not a shortcut: sums taken in another order stay in every level.

What Overkill turns off ([`engine.rs`](../crates/piano-core/src/engine.rs), `Shortcuts`, `Quality::resolution`):

| Shortcut (Full) | Overkill |
|---|---|
| A voice is freed below −100 dB of a loud note | 200 dB down, 56 dB under 24-bit resolution |
| Soundboard, frame and rim sleep below a threshold | same, 200 dB down |
| A sympathetic bank at rest is skipped; phantom partials are skipped below a tension threshold | same, 200 dB down |
| Strings under their damper get no listening voice, and those damped beyond 99 % ignore the bridge | every string, damped or not, listens to the bridge and is kept |
| Partials of a struck string dropped once silent (new, below) | never |
| Sympathetic resonance on partials below 5 kHz | every partial |
| Phantom partials up to E4, below 5 kHz, 48 partials, 16 longitudinal modes | every key, every partial and longitudinal mode below 0.45·fs |
| 768 coupled modes per key at most | up to 0.45·fs |
| Hammer contact stepped as finely as convergence requires | twice the steps |
| Player behind outputs above 48 kHz: engine at 44.1/48 kHz | engine at the output's rate |

It costs about 3 times Full: Chopin 47 % of a core on one thread, 15 % on four. The whole keyboard ff does not hold on one thread (126 %) but does on four (33 %). These figures were measured on a lightly loaded machine.

**What each shortcut changes.** Measured on the first 60 s of the Bach and of the Chopin (examples `overkill_parts.rs`, `shortcut_diff.rs`). Each Overkill choice is applied to Full alone; the figures are the difference, overall / worst 100 ms window, in dB below the signal:

| Applied alone to Full | Bach | Chopin |
|---|---|---|
| silences 200 dB down | −75 / −66 | −63 / −42 |
| every string listens, damped ones included | **−17 / −7** | −54 / −30 |
| no partial dropping | −90 / −78 | −70 / −62 |
| modes up to 0.45·fs | no difference | −118 / −102 |
| sympathy on every partial | −90 / −82 | −87 / −75 |
| phantom partials on every key | −46 / −34 | −47 / −33 |
| phantom partials on every partial | −38 / −31 | −45 / −34 |
| twice the contact steps | −43 / −33 | −45 / −34 |
| **Full against Overkill** | **−17 / −7** | **−38 / −26** |

The thresholds are below audibility: −62 dB at worst. The 5 kHz sympathy limit and the 768-mode cap change nothing measurable.

The largest difference is the damped strings. Full gives a listening voice only to the strings whose damper is lifted. Overkill lets all 88 strings hear the bridge, damper down or not.

- In the Bach, played without pedal, this alone changes the render by −17 dB overall and −7 dB in the worst 100 ms window.
- Under a single C4 mf, the halo of the 87 damped strings stays 25 to 33 dB below the note while it is held (example `shortcut_diff.rs`).

That level depends on how strongly the model's dampers stop the strings. The sympathetic gain is now at its physical value of 1, but the damped case has not been checked against recordings. Whether it is realistic is a modelling question, to settle by ear or against a recording of a real piano.

In pedalled music (the Chopin), what separates Full from Overkill is about −26 dB in the worst window. It comes from three resolution choices:

- the phantom partials kept to the bass and middle;
- the contact's step;
- the damped strings, as above.

**A defect found on the way.** A key re-struck while its released string still rang under the damper was struck damped: the damper only started lifting at the next sub-block, 5 to 20 ms later. A voice already freed was struck free. The attack of a repeated note therefore depended on whether its voice had been freed, which is what made the silence threshold look like a −15 dB change. The key lifts its damper on its way down, before the hammer arrives: the strike now always finds it lifted (test `a_restruck_key_is_struck_with_its_damper_lifted`). This changes the sound of repeated notes without pedal: louder, fuller attacks, as on a piano.

### Silent partials dropped

A struck string, once the hammer has left, only decays: its high partials die first, and a mode under the threshold never comes back. The engine now drops the last chunks of the primary bank while their summed energy stays 20 dB below the level at which a whole voice is freed (`ModalBank::drop_tail`). A landing damper or a new strike brings them back. In pedalled music the peak number of modes falls by a third (whole keyboard: 41,400 → 27,200) and the Chopin's mean by 12 to 15 %. The renders change by −69 to −90 dB overall (−49 to −78 dB in the worst window). Overkill keeps every partial.

### Notes on the way

- **Precomputed tables and all listeners at once.** Setting up a string is a copy since the first round (1 µs). The listening strings are therefore all set up when the pedal goes down, instead of 8 per sub-block, which started some of them 15 ms late. This is more exact, and costs about 3 % of one buffer once.
- **String banks with two inputs on AVX2.** They are written explicitly, two chunks per time loop, with the 16-lane type the soundboard uses (`simd.rs`), and give bit-identical results. The kernel was limited by the latency of the recursion (one chunk at a time: 13.4 cycles per chunk and sample). It is now limited by the multiplier's throughput: 0.152 → 0.137 ns per mode and sample (−10 %). Fusing the multiply-adds would gain more, but would break the bit identity across ISAs, which the project keeps.
- **NEON.** On aarch64 (Apple Silicon, ARM Linux), the soundboard and the string banks run on an explicit NEON implementation of the same 16-lane type. Three soundboard outputs fit in a tile on its 32 registers, and the banks run two chunks per time loop. It runs on an Apple M1 in CI (below), not here. The `Bench` workflow ([`bench.yml`](../.github/workflows/bench.yml)) runs the engine's tests and `perf_audit` on a macOS ARM runner, an x86 Linux one and a Windows one. On aarch64, `Isa::Generic` is that NEON path.
- **The build leaves the audio thread's core alone.** An instrument is rebuilt while the piano plays; the build used every hardware thread, including the audio thread's SMT sibling. It now uses half of them: 78 → 91 ms per build.
- **Offline renderer.** `unsampled-piano-render --quality overkill` renders the reference.

### Tried and dropped

- **Events per voice.** The idea was to have each voice handle its own note-ons and note-offs, so that a sub-block is no longer split at every event. It was measured first by moving every event to the start of its sub-block: no difference on the Chopin, the Rachmaninoff or La Campanella, on one thread or four. Splitting costs nothing worth a restructuring.
- **Sparse sympathy** (for Balanced). Each listening string ran only its modes within a quarter semitone of a partial sounding on a struck string, or still ringing. It gained nothing (13.6 → 13.7 % on the Chopin), even when the keybed's knocks were no longer allowed to open the whole spectrum: in real music the struck partials cover the spectrum densely, and the modes once excited stay above the threshold. Removed.
- **Bridge velocities computed only where a string listens** (first round): no gain once the soundboard kernel was rewritten, and a departure from the model. Removed.

### Result

Mean / p99.9 load, best of three runs alternating `main` and this branch on an idle machine:

| | Chord ff | Pumping | Whole keyboard | Chopin | Debussy | Rachmaninoff |
|---|---|---|---|---|---|---|
| AVX-512, 1 thread, main | 17.4 / 28 | 21.9 / 40 | 34.0 / 39 | 20.3 / 32 | 17.8 / 26 | 17.9 / 28 |
| AVX-512, 1 thread, now | 16.7 / 28 | 19.3 / 40 | 29.5 / 33 | 16.4 / 28 | 15.9 / 26 | 16.0 / 26 |
| AVX2, 1 thread, main | 24.5 / 34 | 31.3 / 47 | 46.9 / 58 | 29.3 / 42 | 25.1 / 37 | 26.0 / 36 |
| AVX2, 1 thread, now | 21.9 / 34 | 24.9 / 46 | 35.7 / 42 | **21.4** / 33 | 20.8 / 31 | 20.9 / 32 |
| no AVX, 1 thread, main | 44.3 / 57 | 54.7 / 74 | 79.2 / 91 | 53.7 / 70 | 45.6 / 65 | 46.0 / 61 |
| no AVX, 1 thread, now | 41.9 / 57 | 45.8 / 72 | 62.3 / 70 | **40.7** / 56 | 39.5 / 54 | 39.6 / 54 |
| AVX-512, 2 threads, main | 12.9 / 20 | 15.2 / 28 | 21.8 / 28 | 14.4 / 27 | 13.4 / 22 | 13.0 / 24 |
| AVX-512, 2 threads, now | 10.6 / 20 | 12.0 / 25 | 16.9 / 20 | 10.5 / 21 | 10.2 / 19 | 10.3 / 19 |
| AVX2, 2 threads, main | 17.2 / 23 | 20.0 / 32 | 27.9 / 36 | 19.8 / 32 | 17.7 / 28 | 17.6 / 28 |
| AVX2, 2 threads, now | 13.4 / 21 | 15.0 / 29 | 20.7 / 25 | **13.3** / 25 | 13.0 / 22 | 13.0 / 23 |
| AVX-512, 4 threads, main | 10.5 / 20 | 11.9 / 18 | 15.5 / 19 | 11.5 / 24 | 10.7 / 22 | 10.7 / 20 |
| AVX-512, 4 threads, now | 6.9 / 15 | 7.7 / 17 | 10.3 / 14 | **6.9** / 18 | 6.8 / 18 | 6.8 / 17 |
| AVX2, 4 threads, main | 14.1 / 17 | 15.8 / 22 | 20.3 / 25 | 15.5 / 28 | 14.1 / 23 | 14.3 / 25 |
| AVX2, 4 threads, now | 8.3 / 17 | 9.2 / 17 | 12.2 / 16 | **8.4** / 20 | 8.1 / 18 | 8.1 / 18 |

Since the start of the audit, the Chopin on one AVX-512 thread went from 24.5 % to 16.4 %, and from 85 % to 41 % without AVX. With four threads it is at 6.9 %. On AVX2 with two threads it is at 13.3 %.

The renders of `main` and of this branch differ by −69 dB overall on the Chopin, −64 dB on the Bach and −51 dB on the Rachmaninoff. The worst windows are at −41, −36 and −32 dB. The two intended changes account for this: the re-strike fix and the listeners set up all at once. Dropping the partials alone stays under −49 dB.

### On GitHub's runners

The `Bench` workflow ([run of October 24](https://github.com/mgth/unsampled-piano/actions/runs/40083458954)) ran the engine's tests and `perf_audit` on three shared virtual machines. Every test passes on all three. On the Apple M1 that includes the bit-identity tests, so the NEON path gives the same output as the x86 ones. Shared runners are noisy: read the means to about 10 %, and not the worst buffers.

Kernels, ns per mode and sample (string bank free / with two inputs; soundboard with 16 inputs and 18 outputs):

| Runner | ISA | String bank | Soundboard |
|---|---|---|---|
| Apple M1 (virtual, 3 vCPUs) | NEON | 0.24 / 0.38–0.43 | 3.84 |
| AMD EPYC 9V45, Zen 5 (4 vCPUs) | AVX-512 | 0.049 / 0.100 | 1.10 |
| Windows, AMD EPYC 7763, Zen 3 (2 cores, 4 threads) | AVX2 | 0.136 / 0.266 | 2.95 |
| *Ryzen 9950X, here* | *AVX-512* | *0.039 / 0.078* | *0.95* |

Mean load, whole keyboard ff with the pedal:

| Runner | Full, 1 / 2 / 4 threads | Balanced, 1 thread | Light, 1 thread |
|---|---|---|---|
| Apple M1 (NEON) | 117 / 56 / 50 % | 73 % | 25 % |
| EPYC Zen 5 (AVX-512) | 37 / 22 / 20 % | 32 % | 10 % |
| Windows, EPYC Zen 3 (AVX2) | 78 / 46 / 44 % | 67 % | 23 % |

The 10-key ff chord, Full, on 1 / 2 / 4 threads: M1 66 / 46 / 33 %, EPYC 21 / 13 / 13 %, Windows 48 / 30 / 29 %.

- **Apple M1, virtual.** Per clock cycle, NEON does a little better than the x86 path without AVX, but this machine is slow and shared. On one thread Full does not hold the heaviest scenarios; it needs 2 or 3 threads, or Light. A real Mac, and the newer M2 to M4, should do clearly better; nothing here measures them.
- **EPYC.** As here. Four threads gain little over two: four vCPUs are probably two physical cores.
- **Windows.** A Zen 3 server core shared by a virtual machine, without AVX-512: the profile of a small machine. Full wants two threads, and Light holds on one. Four threads gain nothing over two on its two cores.

There is no `main` baseline on these runners, as the workflow only exists on this branch. The Windows job's CPU name comes from PowerShell (`Get-CimInstance`): `wmic` is gone from the current images.

## Third round (November 22): the shipped Concert grand

The first two rounds measured the default instrument. The Concert grand as shipped is much heavier: its board is built by finite elements (2124 modes below 0.45·fs), driven key by key below 6 kHz; four microphones, each with the case's field in a slot of its own (8 mic slots); the room fed by four probes; the body's halo after every strike. The player's own set-up runs it on two threads.

Rule of this round: every output bit stays the same.

### Starting point

48 kHz, 128-frame buffers, Ryzen 9 9950X, mean / p99.9 / worst buffer, best of three runs:

| Scenario | 1 thread | 2 threads |
|---|---|---|
| idle | 2.8 / 3.0 / 4 % | 2.7 / 2.9 / 4 % |
| pedal held, nothing played | 7.4 / 31.6 / 42 | 6.6 / 24.8 / 37 |
| one note C4 mf | 22.6 / 28.2 / 28 | 19.2 / 23.1 / 23 |
| 10-key chord ff, bass/middle | 34.9 / 57.2 / 58 | 26.3 / 40.5 / 47 |
| 10-key chord ff, treble | 31.6 / 51.3 / 55 | 24.9 / 36.6 / 44 |
| 10 keys re-struck every 60 ms | 46.9 / 64.3 / 65 | 37.8 / 51.6 / 52 |
| 30 keys, pedal pumping | 39.6 / 93.6 / **105** | 29.9 / 65.1 / 77 |
| 30-key chord every 200 ms | 52.2 / **106.7** / 107 | 40.8 / 78.2 / 79 |
| glissando, no pedal | 42.5 / 87.8 / 88 | 33.2 / 65.3 / 66 |
| whole keyboard ff, pedal | 52.8 / 59.7 / 60 | 38.1 / 43.4 / 44 |

### Where the time went

Profile of the audio thread (`perf record -e cycles:u`, one thread), share of the render time:

| Part | One note | 10-key chord | 10 keys re-struck | Whole keyboard ff |
|---|---|---|---|---|
| Soundboard banks (kernel, `prepare`, `finish`) | 64 % | 50 % | 38 % | 40 % |
| String banks, sympathetic (two inputs) | 1 % | 14 % | 11 % | 10 % |
| String banks, primary | 1 % | 4 % | 4 % | 9 % |
| `Voice::compute` (phantom partials, hammer hand-over) | 1 % | 6 % | 4 % | 20 % |
| Body's halo | 9 % | 6 % | **24 %** | 6 % |
| Room | 8 % | 6 % | 4 % | 4 % |
| Hammer contact | — | — | 3 % | — |

- **The soundboard is a fixed cost of about 20 points of a buffer.** Without it (bridge force straight to the output) one note costs 0.2 %. As soon as anything sounds, three banks run: the low modes driven key by key (825 modes, 28 outputs and the rim's 17), the modes above (1299, the same 28 outputs) and the sympathetic board (all 2124, 8 mic slots). The 28 outputs are the 8 mic slots, the 4 probes and the 16 bridge velocities. Counting one item per chunk of 16 modes, output and sample, that is about 4800 items a sample, at 3.7 cycles each where the multipliers alone allow about 1: the lane sums of one bank (115 KB) went through memory once per 4 chunks and did not fit the first-level cache, 16-lane loads straddled cache lines, and each small tile paid its bounds checks and index arithmetic.
- **The body's halo ran scalar.** Its 1024 resonances were not dispatched to AVX. A strike that settles in over `body.rise` (0.1 s in this preset) adds a step to every resonance every 16 samples: with ten keys re-struck, a quarter of the engine.
- **The room cost 1.8 points at all times**, silence included: per sample, each microphone's taps were summed through memory (`by_order[t.order] +=`, a store-to-load chain per tap).
- **On several threads the low modes' bank ran on the audio thread alone**, after the pool had finished the others.
- **The onset spike** of thirty keys in one buffer (105 %): about 50 points of hammer contact, 10 to 15 of the halo's strikes, on top of the 40 of the steady state.

### The guard rail

[`tools/bitexact`](../tools/bitexact/README.md): twelve MIDI scenarios, 208 s (single notes over the keyboard at three velocities, chords, re-strikes, releases, the sustain pedal held, pumped and half down, sostenuto and una corda, a glissando, thirty keys in one tick, 20 s of the Chopin and of La Campanella), rendered by `unsampled-piano-render` with the shipped preset; `guard.py compare` compares the WAV files byte for byte.

What the render depends on, measured before any change (`guard.py matrix`):

| Compared | Result |
|---|---|
| two runs | identical |
| 1, 2 and 4 threads | identical |
| AVX-512, AVX2 (`PIANO_ISA=avx2`), none (`generic`) | identical |
| buffers of 128 and of 64 frames | identical |
| buffers of 128 and of 32 or 100 frames | **different**: peak difference 13 to 31 dB under the signal's rms |

The engine cuts a buffer in sub-blocks of at most 64 samples, and the dampers, the gains and the silences move once per sub-block: a host whose buffers are not multiples of 64 gets a slightly different render. This was there before this round and is left as it is.

### What changed, every output bit the same

Each step was checked on the twelve scenarios against the reference; the final state on one and two threads and on the three ISAs, all identical. `cargo test -p piano-core -p piano-params -p piano-compare` passes, with no allocation on the audio thread. The comparison tool's score of the shipped preset is the same value for value (865 off, mean 1.1938 on 24 measures).

1. **Body's halo** ([`body.rs`](../crates/piano-core/src/body.rs)). Its three loops (the settling steps, the pulse, the resonances) run on the 16-lane type of the soundboard, dispatched to AVX2 and AVX-512: plain multiplies and adds resonance by resonance, the lane sums reduced as before. A test compares every ISA with the loop as it was written.
2. **The banks' parts on the pool** ([`engine.rs`](../crates/piano-core/src/engine.rs)). The low modes' bank and the sympathetic board are split like the board's (`prepare`, the parts, `finish`): six parts and the frame for the workers, summed afterwards in the same order. One thread is unchanged.
3. **A string that only listens runs one input** ([`voice.rs`](../crates/piano-core/src/voice.rs)). Its second input is its own bridge force, cancelled from what it hears: zero until it is struck. b₂·0 adds nothing but the sign of a zero to a state at rest, which no output can read. Worth little: 0.7 point on a pedalled ten-key chord, nothing measurable elsewhere.
4. **Room** ([`room.rs`](../crates/piano-core/src/room.rs)). A block's source is computed first (it never depends on the room's output), then the taps, grouped by order when the room is built and summed across time (each sample and order keeps its taps in order), then the tail over the block, the lines read before they are written (a block is never longer than a line). The filters of every microphone and order run side by side. An exactly silent room (every state +0, every source sample +0) is skipped, the microphones still receiving the `+0` they would have. Tested bit for bit against the sample-by-sample room on seven set-ups, with and without flush-to-zero. Reflections of order 8 and more are dropped when the room is built: the old loop indexed its eight per-order sums out of bounds on them and panicked, which only happened when one survived among a microphone's 128 strongest taps (`room.order` reaches 8 at the end of its slider).
5. **Soundboard kernel** ([`board.rs`](../crates/piano-core/src/board.rs)):
   - history, weights and lane sums in 64-byte-aligned lines (they were `Vec`s of plain `f32` arrays, aligned to 4 bytes);
   - blocks of 16 chunks over segments of 8 samples instead of 4 chunks over the 64: the working set is about 22 KB and fits a 32 KB first-level cache, each output's lane sums go through memory once per 16 chunks;
   - the weights of the outputs actually computed packed apart from the poles and the 176 input gains;
   - tiles with their bounds checked once, then raw pointers;
   - with more than four inputs, the drive Σφ·x summed ahead of the recursion, several samples at a time (the inputs in the same order);
   - `finish` compiled per ISA.

   Each output still sums its chunks in order lane by lane, then its lanes as a tree. A test compares the kernel on every ISA with a scalar reference written lane by lane. Per mode and sample on this preset's shapes ([`board_shapes.rs`](../crates/piano-render/examples/board_shapes.rs)): the low modes with one key 1.38 → 0.70 ns on AVX-512, 1.83 → 1.22 on AVX2, 4.13 → 2.62 without AVX; the sympathetic board 0.40 → 0.21, 0.55 → 0.40 and 1.23 → 0.92. About 1.5 to 1.9 cycles per item on AVX-512.

Mean / p99.9 after each step, one thread, in the order the steps were measured (best of three; the machine was not idle, read the means to about half a point and the p99.9 to a few):

| Scenario | start | halo | pool | listening | room | kernel |
|---|---|---|---|---|---|---|
| idle | 2.9 / 3.0 | 3.0 / 3.2 | 2.9 / 3.5 | 3.2 / 3.4 | 1.1 / 1.2 | 1.1 / 1.4 |
| pedal held, nothing played | 7.4 / 31.7 | 7.6 / 31.0 | 7.5 / 30.8 | 8.0 / 31.7 | 5.8 / 28.2 | 4.7 / 20.9 |
| one note C4 mf | 22.6 / 28.3 | 21.7 / 25.9 | 21.9 / 26.3 | 22.2 / 26.3 | 20.1 / 23.6 | **12.2** / 15.3 |
| 10-key chord ff, bass/middle | 34.7 / 56.4 | 34.0 / 51.2 | 33.7 / 52.1 | 33.1 / 51.3 | 31.6 / 49.2 | 23.3 / 41.7 |
| 10-key chord ff, treble | 32.1 / 50.5 | 30.9 / 46.0 | 30.3 / 45.6 | 30.2 / 45.4 | 28.3 / 43.4 | 20.2 / 36.0 |
| 10 keys re-struck every 60 ms | 48.1 / 66.4 | **39.0** / 62.6 | 37.5 / 57.8 | 39.1 / 66.3 | 35.9 / 58.6 | 27.6 / 49.4 |
| 30 keys, pedal pumping | 40.8 / 96.9 | 39.2 / **82.5** | 38.2 / 77.4 | 38.9 / 81.9 | 37.3 / 80.2 | 28.0 / 70.8 |
| 30-key chord every 200 ms | 54.2 / 110.8 | 45.0 / 98.3 | 44.1 / 98.7 | 44.9 / 101.8 | 43.4 / 97.9 | 33.7 / 88.6 |
| glissando, no pedal | 43.5 / 90.4 | 39.4 / **75.7** | 38.6 / 73.4 | 39.9 / 76.0 | 37.6 / 74.2 | 28.6 / 63.1 |
| whole keyboard ff, pedal | 54.9 / 64.4 | 51.9 / 58.5 | 50.8 / 57.2 | 52.7 / 63.5 | 49.5 / 60.6 | 40.4 / 46.9 |

On two threads the pool step is the large one (one note 18.3 → 15.3, whole keyboard 36.2 → 31.5).

### Result

Mean / p99.9 / worst buffer, best of three runs alternating the start and the end on an idle machine (four threads: one run):

| Scenario | 1 thread, before | 1 thread, now | 2 threads, before | 2 threads, now | 4 threads, before | 4 threads, now |
|---|---|---|---|---|---|---|
| idle | 2.8 / 3.0 / 4 | **1.0** / 1.1 / 2 | 2.7 / 2.9 / 4 | 0.9 / 1.1 / 2 | 2.7 / 3.0 / 4 | 0.9 / 1.1 / 2 |
| pedal held, nothing played | 7.4 / 31.6 / 42 | 4.8 / 22.8 / 35 | 6.6 / 24.8 / 37 | 4.0 / 13.1 / 26 | 6.1 / 18.5 / 32 | 3.5 / 9.6 / 26 |
| one note C4 mf | 22.6 / 28.2 / 28 | **11.9** / 15.1 / 16 | 19.2 / 23.1 / 23 | **8.2** / 10.1 / 11 | 15.6 / 18.8 / 19 | 6.3 / 8.1 / 8 |
| 10-key chord ff, bass/middle | 34.9 / 57.2 / 58 | 22.8 / 39.9 / 42 | 26.3 / 40.5 / 47 | 14.0 / 29.1 / 30 | 20.4 / 33.3 / 40 | 9.8 / 24.9 / 26 |
| 10-key chord ff, treble | 31.6 / 51.3 / 55 | 19.8 / 35.0 / 35 | 24.9 / 36.6 / 44 | 12.6 / 26.9 / 28 | 19.6 / 31.5 / 39 | 9.0 / 23.8 / 24 |
| 10 keys re-struck every 60 ms | 46.9 / 64.3 / 65 | 26.9 / 48.2 / 49 | 37.8 / 51.6 / 52 | 17.6 / 32.9 / 33 | 32.0 / 46.1 / 46 | 13.1 / 25.5 / 26 |
| 30 keys, pedal pumping | 39.6 / 93.6 / 105 | 27.5 / 67.1 / **83** | 29.9 / 65.1 / 77 | 17.0 / 46.4 / 60 | 22.9 / 54.6 / 69 | 11.9 / 30.5 / 51 |
| 30-key chord every 200 ms | 52.2 / 106.7 / 107 | 33.3 / 85.9 / 87 | 40.8 / 78.2 / 79 | 20.7 / 58.4 / 61 | 34.2 / 79.9 / 118 | 14.3 / 48.0 / 51 |
| glissando, no pedal | 42.5 / 87.8 / 88 | 28.3 / 62.8 / 63 | 33.2 / 65.3 / 66 | 17.6 / 37.5 / 38 | 27.3 / 65.4 / 67 | 12.4 / 25.4 / 26 |
| whole keyboard ff, pedal | 52.8 / 59.7 / 60 | **39.8** / 46.4 / 47 | 38.1 / 43.4 / 44 | **23.4** / 27.5 / 30 | 29.7 / 54.7 / 65 | 15.2 / 18.2 / 29 |

On one thread the mean falls by a quarter (whole keyboard) to a half (one note), and no buffer of these scenarios passes its deadline any more. On two threads, the player's setting, it falls by 40 to 57 %.

Smaller machines, emulated with `PIANO_ISA` on one thread (one run each, mean / p99.9):

| Scenario | AVX2, before | AVX2, now | no AVX, before | no AVX, now |
|---|---|---|---|---|
| one note C4 mf | 27.2 / 33 | 18.9 / 23 | 58.2 / 107 | 40.8 / 46 |
| 10-key chord ff, bass/middle | 44.3 / 68 | 33.6 / 69 | 86.6 / 116 | 67.2 / 110 |
| 10 keys re-struck every 60 ms | 57.1 / 76 | 37.6 / 58 | 101.2 / 186 | 74.1 / 96 |
| 30 keys, pedal pumping | 50.6 / 109 | 39.5 / 82 | 96.5 / 177 | 76.8 / 149 |
| whole keyboard ff, pedal | 66.4 / 95 | 55.0 / 78 | 119.3 / 211 | 97.7 / 109 |

AVX2 on two threads now: one note 11.8 / 14, whole keyboard 30.5 / 57.

Where the time goes now (one thread): one held note is the soundboard for three quarters (12 % of a buffer in all); the whole keyboard ff is the phantom partials and the hammer hand-over for 30 %, the soundboard for 31 %, the string banks for 20 %.

What is left of the onset spike is the hammer contact: thirty keys struck in one buffer still take about 50 points for the three buffers their contact lasts (83 % worst buffer on one thread, 60 % on two).

### Tried and dropped, and what is left

- **Phantom partials dispatched to AVX** (the products of `Voice::nl_run`): no gain, as in the first round.
- **Partials dropped earlier** (`Shortcuts::drop_partials` 10 and 100 times higher): no gain on these scenarios, where the notes have not decayed.
- **Skipping (chunk, output) weights that are zero**: there are none; the probes above 3 kHz carry the microphones' estimate.
- Not done: the hammer contact (the spike that is left; 13 cache lines per 8 modes in f64, up to 768 modes a key); the strings that listen at rest with the pedal held and nothing played (3.8 points for 66 voices that compute nothing useful); `prepare` scanning the 176 inputs of the low modes' bank where the engine knows which keys sound, and `finish` reducing the parts' lane sums on the audio thread (15 to 25 % of that bank's time; both need a change of the banks' interface).

## Quality levels

Each level is a `Quality` ([`engine.rs`](../crates/piano-core/src/engine.rs)), chosen in the player's Audio and MIDI page, with `--quality` on the command line or `quality` in `play.toml`. A level only changes the model an instrument is built from and the engine's shortcuts: presets keep their values.

| Level | What it leaves out | Chopin, 1 / 4 threads (AVX-512) | Whole keyboard ff, 1 / 4 |
|---|---|---|---|
| **Overkill** | nothing (see above) | 47 / 15 % | 126 / 33 % |
| **Full** (default) | the shortcuts judged inaudible | 16.4 / 6.9 % | 29.5 / 10.3 % |
| **Balanced** | sympathy above 2 kHz, phantom partials above C3, beyond 96 voices | 13.4 / 6.1 % | 25.9 / 9.4 % |
| **Light** | sympathy, phantom partials, frame and rim, the horizontal polarisation | 5.6 / 3.8 % | 8.1 / 4.8 % |

Against Overkill, on the first 60 s, Full differs by −17 dB on the Bach and −38 dB on the Chopin (−7 and −26 dB in the worst window): see above. Balanced and Light differ more, as they leave parts of the model out. None of the levels has been compared by ear.

Individual settings, for presets ([`params.rs`](../crates/piano-core/src/params.rs)), measured in the first round on the whole keyboard / the Chopin (mean, from 38 / 23 %):

| Setting | Saving | What is lost |
|---|---|---|
| `sympathetic.max_freq` 5000 → 1500 Hz | −3 / −3 points | the high sympathetic shimmer; octaves and fifths still ring |
| `sympathetic.enabled = false` | −8 / **−12** points | all sympathetic resonance and the pedal halo |
| `nonlinear.enabled = false` | **−10** / ≈ −1 points | phantom partials: the bass's growl at ff |
| `frame` and `rim` off | −5 points | the frame's ring and the rim's low body |
| `polarization.enabled = false` | −6 / −4 points | the horizontal polarisation: part of the aftersound and of the double decay |
| `room.enabled = false` | −1 point | the room: the host's reverb takes over |

## Outputs above 48 kHz

The engine's cost grows with its sample rate: on the Chopin with AVX2, the first 30 s take 25.4 % of the buffer at 48 kHz (mean; p99.9 38.3 %) and 53.4 % at 96 kHz (p99.9 83.6 %). An output at 96 kHz therefore doubles the load for nothing audible, since the model has nothing to add above 20 kHz. Windows makes this common: in shared mode the player must take the rate set for the device in the system's sound settings, and many interfaces default to 96 or 192 kHz. A 13900K playing this Chopin at 96 kHz reached 85–93 % mean.

So the standalone player computes the piano at 44.1 or 48 kHz behind any output at 88.2 to 384 kHz, and upsamples ([`upsample.rs`](../crates/piano-core/src/upsample.rs), [`player.rs`](../crates/piano-play/src/player.rs)):

- The factor is a power of two (2, 4 or 8), chosen so that the engine stays at or above 44.1 kHz: 96 kHz → 48 kHz, 192 kHz → 48 kHz, 88.2 kHz → 44.1 kHz.
- Each 2× is a Kaiser-windowed half-band filter, flat to 20 kHz (ripple below 0.01 dB) with the images 100 dB down. One of its two output phases is the input itself, so each stage costs about 40 multiply-adds per engine sample and channel. That is negligible against the engine.
- The delay is about 0.4 ms (39 samples at 96 kHz).
- The output buffers need not be a multiple of the factor: the frames left over from the last engine block go out at the start of the next buffer. MIDI events keep their place to within one engine sample.

`--full-rate` (`full_rate = true` in `play.toml`, or the "Sample rate" box on the Audio and MIDI page) computes the piano at the output's rate. The plugin still computes at the host's rate.

## Multi-threading

`EngineConfig::threads` (the player's Audio and MIDI page; 1 by default), with every output **identical, bit for bit, whatever the thread count** (tests `threads_do_not_change_the_output`, `swap_on_two_threads_keeps_held_notes`).

**What runs in parallel.** The bridge motion reaches the strings 64 samples late (`note_model::BRIDGE_DELAY`), so no voice depends on another within a sub-block. Each sub-block makes two runs on a pool of persistent workers ([`par.rs`](../crates/piano-core/src/par.rs)):

1. **The voices.** Each voice computes its forces into its own buffers (`Voice::compute`). The audio thread adds them in voice order (`Voice::mix`).
2. **The structures.** Each of the soundboard's banks (its modes above the ones driven key by key, those, and the sympathetic board) is split in two halves (`BoardBank::split`, `BoardPart`), and the frame runs whole into its own buffers. The audio thread then adds everything in the same order as one thread would. The halves are taken in every case, one thread or several, so the result does not depend on it. (Until the third round the low modes' bank and the sympathetic board ran whole, the first on the audio thread.)

The rim (small, fed by the board) and the room stay on the audio thread.

**Measured** (table above): two threads take a third off, four take 60 % off. On the Chopin at AVX-512: 16.4 → 10.5 → 6.9 %.

**Real-time priority.** The player gives its workers the audio thread's scheduling (`EngineConfig::worker_init`, [`audio.rs`](../crates/piano-play/src/audio.rs)). On Linux this is SCHED_FIFO at PipeWire's priority (88) within RLIMIT_RTPRIO; the `audio_thread_priority` crate cannot help there, as it needs rtkit over D-Bus. Windows uses MMCSS "Pro Audio" and macOS a time-constraint policy, through that crate. A test checks the Linux case where the system allows it. A worker the OS preempts would make the audio callback wait for it.

**Edits while playing.** An edit that rebuilds the instrument keeps the replaced engines playing their notes for up to 2 s. Before, each engine had its own workers, and the audio thread ran them one after the other.

Now the engine slot ([`slot.rs`](../crates/piano-control/src/slot.rs)) holds a single pool. It lends the pool to the current engine when that engine plays alone. When replaced engines are finishing, it runs the engines side by side, one per thread. Two threads therefore stay two threads. Nothing changes for the sound, or for how long the old notes last.

Measured with `edit_load.rs`: thirty keys ring with the pedal down while a slider moves every 250 ms for 3 s, with a real-time audio thread.

| p99 load | 1 thread | 2 threads | 4 threads |
|---|---|---|---|
| main (up to 4 engines) | 75.8 % | 67.8 % | 66.7 % |
| now (up to 2 engines) | 47.8 % | 37.1 % | 26.0 % |

The worst buffer of the scenario (≈ 80 %) is the initial chord of thirty keys struck in one buffer, not the edits. It falls to 55 % on four threads.

**Plugins.** They stay at one thread. A plugin's own threads compete with the host's, which already uses every core for its tracks. The way out is CLAP's `thread-pool` extension, which hands the work to the host's pool, but nice-plug 0.4.2 does not implement it.
