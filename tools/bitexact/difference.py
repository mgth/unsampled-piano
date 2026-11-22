#!/usr/bin/env python3
"""Level of the difference between two renders of the same events, by band
and in time: what an engine shortcut costs (docs/performance.md).

  difference.py sets  DIR_A DIR_B          every WAV of two guard.py sets
  difference.py notes --bin B --preset A.toml --preset-b B.toml --out DIR
                      [--notes 36 48 60 72 84 96] [--velocities 80 127]
                      [--hold 2.0] [--tail 3.0] [--cores LIST]

`sets`: per scenario, the difference's level relative to A's (dB, both over
the whole file), its worst 100 ms window (against A's level in the same
window, windows quieter than 60 dB under A's loudest left out), and the same
per octave band.

`notes`: renders single notes held `hold` s then released, with both presets,
and prints the difference per octave band in time windows from the onset and
after the release, in dB under A's level in the same band and window (a dash
where A itself is more than 90 dB under its own peak band level), then how
much B's level differs from A's there: a part that only changes phase (a
reverberant tail fed a little differently) shows in the first table, not in
the second.

Needs numpy and scipy.
"""
import argparse
import os
import subprocess
import sys

import numpy as np
from scipy import signal

BANDS = [63, 125, 250, 500, 1000, 2000, 4000, 8000, 16000]


def read(path):
    """Float32 stereo WAV (as the renderer writes them) → (rate, [frames, ch])."""
    with open(path, "rb") as f:
        data = f.read()
    # hound's header: the format chunk says IEEE float, 32 bits.
    i = data.index(b"fmt ")
    ch = int.from_bytes(data[i + 10:i + 12], "little")
    rate = int.from_bytes(data[i + 12:i + 16], "little")
    j = data.index(b"data", i)
    n = int.from_bytes(data[j + 4:j + 8], "little")
    x = np.frombuffer(data, dtype="<f4", count=n // 4, offset=j + 8)
    return rate, x.reshape(-1, ch).astype(np.float64)


def db(p, ref):
    return 10 * np.log10(max(p, 1e-300) / max(ref, 1e-300))


def band_filters(rate):
    out = []
    for fc in BANDS:
        lo, hi = fc / 2 ** 0.5, min(fc * 2 ** 0.5, 0.49 * rate)
        out.append(signal.butter(4, [lo, hi], btype="band", fs=rate, output="sos"))
    return out


def power(x):
    return float(np.mean(x * x)) if len(x) else 0.0


def compare_files(a_path, b_path):
    rate, a = read(a_path)
    _, b = read(b_path)
    n = min(len(a), len(b))
    a, d = a[:n], b[:n] - a[:n]
    overall = db(power(d), power(a))
    # Worst 100 ms window, where A is not near silence.
    w = rate // 10
    k = n // w
    pa = (a[:k * w] ** 2).reshape(k, w, -1).mean(axis=(1, 2))
    pd = (d[:k * w] ** 2).reshape(k, w, -1).mean(axis=(1, 2))
    live = pa > pa.max() * 1e-6
    worst = float(np.max(10 * np.log10(np.maximum(pd[live], 1e-300) / pa[live]))) if live.any() else float("-inf")
    at = float(np.argmax(np.where(live, pd / np.maximum(pa, 1e-300), 0)) * w / rate)
    bands = []
    for sos in band_filters(rate):
        bands.append(db(power(signal.sosfilt(sos, d, axis=0)), power(signal.sosfilt(sos, a, axis=0))))
    return overall, worst, at, bands


def cmd_sets(args):
    names = sorted(f for f in os.listdir(args.a) if f.endswith(".wav") and os.path.exists(os.path.join(args.b, f)))
    print(f"{'scenario':16} {'overall':>8} {'worst 0.1 s':>12} {'at':>7}   " + " ".join(f"{b:>6}" for b in BANDS))
    for f in names:
        overall, worst, at, bands = compare_files(os.path.join(args.a, f), os.path.join(args.b, f))
        print(f"{f[:-4]:16} {overall:8.1f} {worst:12.1f} {at:6.1f}s   " + " ".join(f"{x:6.1f}" for x in bands))


def render_note(args, preset, note, vel, path):
    cmd = ["nice", "-n", "19", "ionice", "-c3"]
    if args.cores:
        cmd += ["taskset", "-c", args.cores]
    cmd += [args.bin, "--preset", preset, "--tail", str(args.tail), "note", path, "--note", str(note), "--velocity", str(vel), "--hold", str(args.hold)]
    subprocess.run(cmd, check=True, stdout=subprocess.DEVNULL)


def cmd_notes(args):
    os.makedirs(args.out, exist_ok=True)
    h = args.hold
    windows = [(0.0, 0.05), (0.05, 0.3), (0.3, 1.0), (1.0, h), (h, h + 0.3), (h + 0.3, h + 1.0), (h + 1.0, h + args.tail)]
    labels = ["0-50ms", "50-300ms", "0.3-1s", f"1-{h:g}s", "rel+0.3s", "rel+1s", "rel tail"]
    worst = {}
    level = {}
    for note in args.notes:
        for vel in args.velocities:
            pa = os.path.join(args.out, f"a_{note}_{vel}.wav")
            pb = os.path.join(args.out, f"b_{note}_{vel}.wav")
            render_note(args, args.preset, note, vel, pa)
            render_note(args, args.preset_b, note, vel, pb)
            rate, a = read(pa)
            _, b = read(pb)
            n = min(len(a), len(b))
            a, d = a[:n], b[:n] - a[:n]
            print(f"\nnote {note} velocity {vel}: difference, dB under the signal in the same band and window")
            print(f"{'band':>7} " + " ".join(f"{l:>9}" for l in labels))
            peak = 0.0
            rows = []
            for fc, sos in zip(BANDS, band_filters(rate)):
                fa, fd = signal.sosfilt(sos, a, axis=0), signal.sosfilt(sos, d, axis=0)
                row = []
                for (t0, t1) in windows:
                    i0, i1 = int(t0 * rate), min(int(t1 * rate), n)
                    row.append((power(fa[i0:i1]), power(fd[i0:i1]), power(fa[i0:i1] + fd[i0:i1])))
                peak = max(peak, max(p for p, _, _ in row))
                rows.append((fc, row))
            for fc, row in rows:
                cells = []
                for k, (p, q, _) in enumerate(row):
                    if p < peak * 1e-9:
                        cells.append(f"{'-':>9}")
                    else:
                        x = db(q, p)
                        cells.append(f"{x:9.1f}")
                        worst[labels[k]] = max(worst.get(labels[k], -999.0), x)
                print(f"{fc:>7} " + " ".join(cells))
            print("  level of B against A's in the same band and window, dB")
            for fc, row in rows:
                cells = []
                for k, (p, _, p_b) in enumerate(row):
                    if p < peak * 1e-9:
                        cells.append(f"{'-':>9}")
                    else:
                        x = db(p_b, p)
                        cells.append(f"{x:+9.2f}")
                        level[labels[k]] = max(level.get(labels[k], 0.0), abs(x))
                print(f"{fc:>7} " + " ".join(cells))
            if not args.keep:
                os.remove(pa)
                os.remove(pb)
    print("\nworst band over all notes, per window:")
    print(" ".join(f"{l}: {worst.get(l, float('nan')):.1f}" for l in labels))
    print("largest level change over all notes and bands, per window (dB):")
    print(" ".join(f"{l}: {level.get(l, float('nan')):.2f}" for l in labels))


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    s = sub.add_parser("sets")
    s.add_argument("a")
    s.add_argument("b")
    s.set_defaults(fn=cmd_sets)
    s = sub.add_parser("notes")
    s.add_argument("--bin", required=True)
    s.add_argument("--preset", required=True)
    s.add_argument("--preset-b", required=True)
    s.add_argument("--out", required=True)
    s.add_argument("--notes", type=int, nargs="+", default=[36, 48, 60, 72, 84, 96])
    s.add_argument("--velocities", type=int, nargs="+", default=[80, 127])
    s.add_argument("--hold", type=float, default=2.0)
    s.add_argument("--tail", type=float, default=3.0)
    s.add_argument("--cores", default=os.environ.get("BITEXACT_CORES", ""))
    s.add_argument("--keep", action="store_true", help="keep the rendered WAVs")
    s.set_defaults(fn=cmd_notes)
    args = ap.parse_args()
    args.fn(args)


if __name__ == "__main__":
    sys.exit(main())
