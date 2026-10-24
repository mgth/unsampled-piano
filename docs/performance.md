# Engine performance audit

October 2026. Where the engine's time goes, what makes it miss a deadline, what was fixed, which compromises smaller machines can take, and what multi-threading gives. Two rounds: the first (October 23) found the costs and spikes and fixed the worst; the second (October 24) added the Overkill reference level, dropped silent partials, spread the soundboard over threads and kept replaced engines from piling up. The sections up to "Fixes made in the first round" describe the state before the first round.

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
- a glissando without pedal;
- the whole keyboard ff with the pedal down (the old `stress_cost` case);
- the six MIDI performances of `data/midi`.

`--variants` repeats the run with each part of the physics switched off.

Other examples:

| Example | What it measures |
|---|---|
| `voice_ops.rs` (piano-core) | the per-event voice operations |
| `board_bench.rs` and `modal_kernels.rs` (piano-core) | the soundboard and string-bank kernels, on every ISA |
| `shortcut_diff.rs` (piano-core) | what each engine shortcut changes in the signal |
| `overkill_parts.rs` (piano-render) | what each of Overkill's differences from Full changes, on a MIDI file |
| `edit_load.rs` (piano-control) | the load while a parameter is edited during playing |

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
| A string damped beyond 99 % no longer hears the bridge | every string hears it |
| Partials of a struck string dropped once silent (new, below) | never |
| Sympathetic resonance on partials below 5 kHz | every partial |
| Phantom partials up to E4, below 5 kHz, 48 partials, 16 longitudinal modes | every key, every partial and longitudinal mode below 0.45·fs |
| 768 coupled modes per key at most | up to 0.45·fs |
| Hammer contact stepped as finely as convergence requires | twice the steps |
| Player behind outputs above 48 kHz: engine at 44.1/48 kHz | engine at the output's rate |

It costs 2 to 3 times Full: Chopin 44.9 % of a core on one thread, 14.3 % on four. The whole keyboard ff does not hold on one thread (110 %) but does on four (31 %).

**What each shortcut changes.** Measured on the first 60 s of the Bach and of the Chopin (examples `overkill_parts.rs`, `shortcut_diff.rs`). Each Overkill choice is applied to Full alone; the figures are the difference, overall / worst 100 ms window, in dB below the signal:

| Applied alone to Full | Bach | Chopin |
|---|---|---|
| silences 200 dB down | −75 / −66 | −63 / −42 |
| damped strings hear the bridge | −41 / −27 | −300 (no difference) |
| no partial dropping | −90 / −78 | −70 / −62 |
| modes up to 0.45·fs | no difference | −118 / −102 |
| sympathy on every partial | −90 / −82 | −87 / −75 |
| phantom partials on every key | −46 / −34 | −47 / −33 |
| phantom partials on every partial | −38 / −31 | −45 / −34 |
| twice the contact steps | −43 / −33 | −45 / −34 |
| **Full against Overkill** | **−32 / −25** | **−38 / −26** |

The engine's shortcuts are below audibility: −62 dB at worst. The 5 kHz sympathy limit and the 768-mode cap change nothing measurable. What separates Full from Overkill, about −25 dB in the worst window, is three resolution choices:

- the phantom partials kept to the bass and middle;
- the contact's step;
- the damped strings left deaf to the bridge (in the Bach, which plays without pedal).

They were judged inaudible when they were made; Overkill now lets them be compared by ear.

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
| Windows (CPU not reported, see below) | AVX2 | 0.136 / 0.266 | 2.95 |
| *Ryzen 9950X, here* | *AVX-512* | *0.039 / 0.078* | *0.95* |

Mean load, whole keyboard ff with the pedal:

| Runner | Full, 1 / 2 / 4 threads | Balanced, 1 thread | Light, 1 thread |
|---|---|---|---|
| Apple M1 (NEON) | 117 / 56 / 50 % | 73 % | 25 % |
| EPYC Zen 5 (AVX-512) | 37 / 22 / 20 % | 32 % | 10 % |
| Windows (AVX2) | 78 / 46 / 44 % | 67 % | 23 % |

The 10-key ff chord, Full, on 1 / 2 / 4 threads: M1 66 / 46 / 33 %, EPYC 21 / 13 / 13 %, Windows 48 / 30 / 29 %.

- **Apple M1, virtual.** Per clock cycle, NEON does a little better than the x86 path without AVX, but this machine is slow and shared. On one thread Full does not hold the heaviest scenarios; it needs 2 or 3 threads, or Light. A real Mac, and the newer M2 to M4, should do clearly better; nothing here measures them.
- **EPYC.** As here. Four threads gain little over two: four vCPUs are probably two physical cores.
- **Windows.** The profile of a small machine: Full wants two threads, and Light holds on one.

There is no `main` baseline on these runners, as the workflow only exists on this branch. The Windows job printed no CPU name because `wmic` is gone from its image; the workflow now asks PowerShell (`Get-CimInstance`).

## Quality levels

Each level is a `Quality` ([`engine.rs`](../crates/piano-core/src/engine.rs)), chosen in the player's Audio and MIDI page, with `--quality` on the command line or `quality` in `play.toml`. A level only changes the model an instrument is built from and the engine's shortcuts: presets keep their values.

| Level | What it leaves out | Chopin, 1 / 4 threads (AVX-512) | Whole keyboard ff, 1 / 4 |
|---|---|---|---|
| **Overkill** | nothing (see above) | 44.9 / 14.3 % | 110 / 30.8 % |
| **Full** (default) | the shortcuts judged inaudible | 16.4 / 6.9 % | 29.5 / 10.3 % |
| **Balanced** | sympathy above 2 kHz, phantom partials above C3, beyond 96 voices | 13.4 / 6.1 % | 25.9 / 9.4 % |
| **Light** | sympathy, phantom partials, frame and rim, the horizontal polarisation | 5.6 / 3.8 % | 8.1 / 4.8 % |

Against Overkill, over the whole pieces, Full differs by −30 dB on the Bach and −33 dB on the Rachmaninoff (−18 and −20 dB in the worst window). Balanced and Light differ more, as they leave parts of the model out. None of the levels has been compared by ear.

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
2. **The structures.** The soundboard's modes are split in two halves (`BoardBank::split`, `BoardPart`), and the frame and the sympathetic board each run whole into their own buffers. The audio thread then adds everything in the same order as one thread would. The halves are taken in every case, one thread or several, so the result does not depend on it.

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
