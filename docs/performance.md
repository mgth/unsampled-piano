# Engine performance audit

October 2026. Where the engine's time goes, what makes it miss a deadline, what was fixed, what is left, which compromises smaller machines can take, and what multi-threading gives.

All figures come from a Ryzen 9 9950X (Zen 5, AVX-512) unless stated otherwise. Smaller machines are emulated by capping the SIMD level (`PIANO_ISA=avx2` or `generic`). This removes the vector width but not the clock or cache differences, so real small machines are slower still.

## How to measure

```
cargo run --release -p piano-render --example perf_audit -- \
    --midi-dir data/midi [--sr 48000] [--block 128] [--threads N] [--variants] [--preset p.toml]
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

`--variants` repeats the run with each part of the physics switched off. `examples/voice_ops.rs` times the per-event voice operations. `examples/board_bench.rs` times the soundboard kernel on every ISA.

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

## Fixes made in this audit

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

## Further optimisations, not done

By expected payoff:

1. **High sample rates.** Two steps:
   - Cap string partials at 20 kHz instead of 0.45·fs. Inaudible, and −18 % at 96 kHz.
   - Render at 44.1 or 48 kHz inside and resample to the host's rate, which roughly halves the cost at 88.2 and 96 kHz.
2. **String banks on AVX2.** They are now the bulk of the load there. Their driven variants run one chunk per time loop with a second input or output, because more spills. A two-pass layout like the soundboard's (recursion, then outputs) may let them group chunks again.
3. **Events per voice.** Today every event splits the sub-block for all voices (`render(pos, f)`). Handling a voice's own note-on and note-off inside its own render would give fewer, longer segments. Dense MIDI would cost less, and it suits multi-threading.
4. **Engine swap.** Limit the old engines kept during an edit (`KEEP`) when the load is high, or fade them faster.

## Compromises for smaller configurations

Each line below can be switched off independently; most already exist as preset fields. Savings are the mean load on the whole keyboard / the Chopin, measured before the fixes, relative to 38 / 23 %. After them (35 / 21 %), switching sympathy off saves −8 / −11 points, phantom partials −10 / −1, frame and rim −6 / −3, one polarisation −7 / −4.

| Setting | Saving | What is lost |
|---|---|---|
| `sympathetic.max_freq` 5000 → 1500 Hz | −3 / −3 points | the high sympathetic shimmer; octaves and fifths still ring |
| `sympathetic.enabled = false` | −8 / **−12** points | all sympathetic resonance and the pedal halo; this is the main one for real music |
| `nonlinear.enabled = false` | **−10** / ≈ −1 points | phantom partials: the bass's growl at ff |
| `nonlinear.max_note` 64 → 48 | part of the above | phantom partials above C3 |
| `frame` and `rim` off | −5 points | the frame's ring and the rim's low body |
| `polarization.enabled = false` | −6 / −4 points | the horizontal polarisation: part of the aftersound and of the double decay |
| `room.enabled = false` | −1 point | the room: the host's reverb takes over |
| polyphony 128 → 64 | bounds the worst case | voice stealing in long pedalled passages |
| render at 48 kHz inside a 96 kHz host | ≈ −50 % | nothing audible |

A "CPU quality" setting could group them in three levels:

- **Full**: today's model.
- **Balanced**: sympathy below 2 kHz, phantom partials up to C3, polyphony 96. This cuts about a third of the mean in dense music.
- **Light**: no sympathy, no phantom partials, no frame or rim, one polarisation. It costs about a third of the Full level, and is meant for AVX2 laptops and ARM.

These levels are proposals; they have not been listened to. Sympathetic resonance is a large part of the instrument's character with the pedal down, so it should go last.

## Multi-threading

### What can run in parallel

- **Voices**, within a sub-block. The bridge motion reaches the strings 64 samples late (`note_model::BRIDGE_DELAY`), so no voice depends on another during a sub-block. That makes this the natural cut. It is also where the onset spikes live.
- **The independent banks**: sympathetic board, frame, rim and room could each run next to the primary board.
- **The board itself**: its modes are independent given the inputs, so it can be split into halves whose outputs are summed in a fixed order.
- **Pipelining** the board of one sub-block with the voices of the next would need a bridge delay of two sub-blocks (128 samples, 2.7 ms). The sympathetic coupling is one-way, so that would not threaten stability.

### Prototype

The prototype ([`par.rs`](../crates/piano-core/src/par.rs), [`engine.rs`](../crates/piano-core/src/engine.rs)) is opt-in through `EngineConfig::threads`, default 1:

- A pool of persistent workers, started with the engine. Workers spin for 200 µs, which covers the sub-blocks of one buffer, then park. They have FTZ/DAZ set and never allocate.
- Each run hands out the active voices one at a time; the main thread works too.
- Each voice computes its forces into its own buffers (`Voice::compute`). The audio thread then adds them to the regions in voice order (`Voice::mix`). The output is therefore **identical, bit for bit, whatever the thread count** (test `threads_do_not_change_the_output`).
- Segments with fewer than 4 voices stay on the audio thread.

Results with the fixes above (mean / p99.9):

| Scenario | 1 thread | 2 threads | 4 threads |
|---|---|---|---|
| whole keyboard ff, AVX-512 | 35.3 / 42.7 % | 22.3 / 28.6 % | 15.8 / 22.5 % |
| whole keyboard ff, AVX2 | 45.7 / 56.0 % | 28.5 / 36.3 % | 20.7 / 31.6 % |
| Chopin, AVX-512 | 21.2 / 33.4 % | 15.2 / 28.7 % | 12.2 / 25.1 % |
| Chopin, AVX2 | 29.6 / 44.0 % | 20.6 / 33.3 % | 16.3 / 28.8 % |
| 10-key chord ff, AVX2 | 25.5 / 35.8 % | 18.5 / 25.2 % | 15.0 / 20.8 % |
| 30 keys, pedal pumping, AVX2 | 30.5 / 48.2 % | 21.3 / 33.9 % | 17.1 / 34.5 % |

Two threads remove a third of the load and flatten the onset spikes. Four threads add less: what stays on the audio thread is the soundboard banks, the room and the event handling. In the Chopin that is ≈ 10 of the 21 %, and it now bounds the gain (Amdahl). Going further means parallelising the board, by splitting it in halves or by running the independent banks side by side. That is the same order of work as the voices, and it would also make the board's cost scale.

### Caveats before shipping it

- **Real-time priority.** A worker that the OS preempts makes the audio thread wait, which turns a spike into a dropout. In the standalone app the workers should get the audio thread's priority (rtkit / PipeWire on Linux, `THREAD_TIME_CONSTRAINT_POLICY` on macOS, MMCSS on Windows). Inside a DAW the host already uses every core for its own tracks. Many hosts offer the plugin no thread pool (CLAP has an optional `thread-pool` extension that hands work to the host's pool). The default should stay at 1 thread in plugins, and the CLAP thread pool is worth using where the host provides it.
- **Spinning** costs one core per worker during playing, for at most 200 µs after each sub-block. That is fine for a standalone player, and less so on a laptop's battery.
- Hosts that call with large buffers (1024 frames) gain the most, since each buffer holds 16 sub-blocks.

**Verdict:** worth it for the standalone player, especially on 4-core machines with AVX2 or ARM, for the voices first (done in the prototype) and the board second. Keep it optional and off by default in the plugins.
