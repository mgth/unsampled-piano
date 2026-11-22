# Bit-exactness guard rail

A fixed set of MIDI scenarios rendered offline by `unsampled-piano-render`
with the shipped preset, so that an optimisation of the engine can be checked
byte for byte: render before, render after, compare the WAV files.

Python 3, standard library only (numpy is used for the difference statistics
when it can be imported; there is a pure-Python fallback).

## Files

- `scenarios.py` writes the scenario MIDI files (format 0, 500 PPQ at 120 bpm,
  so one tick is exactly one millisecond = 48 samples at 48 kHz). The output is
  deterministic. The two dense excerpts are cut from real performances in
  `data/midi` (of the main checkout when run from a git worktree; override with
  `BITEXACT_MIDI_DIR`); when
  neither is there, a pseudo-random dense piece with a fixed seed replaces
  them. `python3 scenarios.py <dir>` lists the scenarios.
- `guard.py` renders, compares and runs the determinism study.
- `difference.py` measures what a shortcut that is not bit-exact costs: the
  level of the difference between two renders, by octave band and in time
  (needs numpy and scipy).

## Usage

Render a set (MIDI files go to `<dir>/midi`, WAVs to `<dir>/<name>.wav`,
hashes, sizes, durations and options to `<dir>/manifest.json`):

```sh
python3 tools/bitexact/guard.py render --bin target/release/unsampled-piano-render \
    --preset "presets/instruments/Concert grand/concert_grand.toml" \
    --out out/bitexact/candidate --threads 1 --isa native --block 128 \
    --jobs 3 --cores 14,15,30,31
```

Every render runs as `nice -n 19 ionice -c3 taskset -c <cores> <bin> …`, at
most `--jobs` at once (default 2), reduced so that jobs × threads does not
exceed the number of cores given. `--cores` defaults to `$BITEXACT_CORES`,
else to every core the process may use. `--isa generic|avx2` sets
`PIANO_ISA`; `native` leaves the best SIMD level of the CPU. `--reuse` keeps
WAVs already rendered with the same binary, preset, MIDI and options;
`--only a b` renders some scenarios only.

Compare two sets (exit code 0 only if every WAV is byte-identical):

```sh
python3 tools/bitexact/guard.py compare out/bitexact/ref out/bitexact/candidate
```

One line per scenario: `identical`, or the first differing frame and time,
the number of differing samples and the peak and RMS of the difference in dB
relative to the RMS of A. A changed scenario MIDI is reported as such rather
than compared.

Determinism study (run-to-run, `--threads` 1/2/4, `PIANO_ISA`
native/avx2/generic, `--block` 128/64/32/100), one sub-directory per option
set, renders already present are reused; the table is also written to
`<dir>/matrix.txt` and `<dir>/matrix.json`:

```sh
python3 tools/bitexact/guard.py matrix --bin … --preset … --out out/bitexact/matrix --jobs 3 --cores 14,15,30,31
```

Delete the WAVs of a set that are byte-identical to a reference (the manifest
keeps their hash, and `compare` still works through it):

```sh
python3 tools/bitexact/guard.py prune out/bitexact/matrix/t2_native_b128 --against out/bitexact/ref
```

## Notes

- The reference `out/bitexact/ref` is rendered with `--threads 1`, native
  ISA, block 128. Compare a candidate rendered with the same options. An
  ISA-dependent result means a reference is only valid on a machine with the
  same SIMD level (see `matrix.txt`).
- Every render process builds the instrument first (about 2–3 s, boards
  cached in `~/.cache/unsampled-piano`); the first scenario of a set is
  rendered alone so that parallel processes do not race on that cache.
- The renderer is slower than real time on one thread for these scenarios;
  a full set is about 200 s of audio.
