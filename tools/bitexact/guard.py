#!/usr/bin/env python3
"""Bit-exactness guard rail for the offline renderer (see README.md).

  guard.py render  --bin B --preset P --out DIR [--threads N] [--isa generic|avx2|native]
                   [--block 128] [--jobs J] [--cores LIST] [--tail 3.0] [--reuse]
  guard.py compare DIR_A DIR_B
  guard.py matrix  --bin B --preset P --out DIR [--jobs J] [--cores LIST]
  guard.py prune   DIR --against REF_DIR

Python 3 standard library only (numpy is used for the difference statistics
when it can be imported).
"""

import argparse
import array
import concurrent.futures as cf
import hashlib
import json
import math
import os
import struct
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import scenarios  # noqa: E402

try:
    import numpy as np
except Exception:  # pragma: no cover - fallback path
    np = None

SR = 48_000
QUALITY = "full"


def sha256_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def parse_cores(s):
    out = []
    for part in s.split(","):
        part = part.strip()
        if "-" in part:
            a, b = part.split("-")
            out.extend(range(int(a), int(b) + 1))
        elif part:
            out.append(int(part))
    return sorted(set(out))


def default_cores():
    env = os.environ.get("BITEXACT_CORES")
    if env:
        return env
    return ",".join(str(c) for c in sorted(os.sched_getaffinity(0)))


# ---------------------------------------------------------------- WAV


def read_wav(path):
    """Returns (channels, sample_rate, data bytes of 32-bit floats)."""
    with open(path, "rb") as f:
        d = f.read()
    if d[:4] != b"RIFF" or d[8:12] != b"WAVE":
        raise ValueError(f"{path}: not a WAV file")
    pos, fmt, data = 12, None, None
    while pos + 8 <= len(d):
        cid, clen = d[pos:pos + 4], struct.unpack("<I", d[pos + 4:pos + 8])[0]
        body = d[pos + 8:pos + 8 + clen]
        if cid == b"fmt ":
            tag, ch, sr, _, _, bits = struct.unpack("<HHIIHH", body[:16])
            if tag == 0xFFFE:  # WAVE_FORMAT_EXTENSIBLE: sub-format GUID
                tag = struct.unpack("<H", body[24:26])[0]
            fmt = (tag, ch, sr, bits)
        elif cid == b"data":
            data = body
        pos += 8 + clen + (clen & 1)
    if fmt is None or data is None:
        raise ValueError(f"{path}: no fmt or data chunk")
    tag, ch, sr, bits = fmt
    if tag != 3 or bits != 32:
        raise ValueError(f"{path}: not 32-bit float (format {tag}, {bits} bits)")
    return ch, sr, data


def floats(data):
    a = array.array("f")
    a.frombytes(data)
    if sys.byteorder != "little":
        a.byteswap()
    return a


def wav_stats(path):
    ch, sr, data = read_wav(path)
    n = len(data) // 4
    if np is not None:
        a = np.frombuffer(data, dtype="<f4")
        bad = int(np.count_nonzero(~np.isfinite(a)))
        fin = a[np.isfinite(a)]
        peak = float(np.max(np.abs(fin))) if fin.size else 0.0
    else:
        a = floats(data)
        bad = sum(1 for x in a if not math.isfinite(x))
        peak = max((abs(x) for x in a if math.isfinite(x)), default=0.0)
    return {"channels": ch, "sample_rate": sr, "frames": n // ch, "duration_s": round(n / ch / sr, 6),
            "peak_dbfs": round(20 * math.log10(peak), 2) if peak > 0 else None, "non_finite": bad}


def diff_stats(path_a, path_b):
    """Statistics of B - A. Returns a dict (identical => {'identical': True})."""
    cha, sra, da = read_wav(path_a)
    chb, srb, db = read_wav(path_b)
    res = {"identical": da == db and (cha, sra) == (chb, srb)}
    if res["identical"]:
        return res
    if (cha, sra) != (chb, srb):
        res["format"] = f"{cha} ch {sra} Hz vs {chb} ch {srb} Hz"
    res["frames_a"], res["frames_b"] = len(da) // 4 // cha, len(db) // 4 // chb
    n = min(len(da), len(db)) // 4
    # First differing byte, found in C-speed slices.
    lo, step = 0, 1 << 16
    while lo < n * 4 and da[lo:lo + step] == db[lo:lo + step]:
        lo += step
    first = None
    for i in range(lo // 4, min(n, lo // 4 + step // 4 + 1)):
        if da[4 * i:4 * i + 4] != db[4 * i:4 * i + 4]:
            first = i
            break
    if first is None:  # equal over the common length: only the length differs
        res.update(first_frame=n // cha, first_time_s=round(n / cha / sra, 6), differing=0)
        return res
    res["first_frame"] = first // cha
    res["first_time_s"] = round(first / cha / sra, 6)
    if np is not None:
        a = np.frombuffer(da[:4 * n], dtype="<f4")
        b = np.frombuffer(db[:4 * n], dtype="<f4")
        res["differing"] = int(np.count_nonzero(a.view("<u4") != b.view("<u4")))
        a64, b64 = a.astype(np.float64), b.astype(np.float64)
        fin = np.isfinite(a64) & np.isfinite(b64)
        res["non_finite"] = int(n - np.count_nonzero(fin))
        d = (b64 - a64)[fin]
        rms_a = math.sqrt(float(np.mean(a64[fin] ** 2))) if fin.any() else 0.0
        peak = float(np.max(np.abs(d))) if d.size else 0.0
        rms_d = math.sqrt(float(np.mean(d ** 2))) if d.size else 0.0
        imax = int(np.flatnonzero(fin)[np.argmax(np.abs(d))]) if d.size else 0
    else:
        ia, ib = array.array("I"), array.array("I")
        ia.frombytes(da[:4 * n])
        ib.frombytes(db[:4 * n])
        res["differing"] = sum(1 for x, y in zip(ia, ib) if x != y)
        a, b = floats(da[:4 * n]), floats(db[:4 * n])
        sa = sd = 0.0
        peak, imax, cnt, bad = 0.0, 0, 0, 0
        for i, (x, y) in enumerate(zip(a, b)):
            if not (math.isfinite(x) and math.isfinite(y)):
                bad += 1
                continue
            e = y - x
            sa += x * x
            sd += e * e
            cnt += 1
            if abs(e) > peak:
                peak, imax = abs(e), i
        res["non_finite"] = bad
        rms_a = math.sqrt(sa / cnt) if cnt else 0.0
        rms_d = math.sqrt(sd / cnt) if cnt else 0.0
    db_ = (lambda v: round(20 * math.log10(v / rms_a), 1) if v > 0 and rms_a > 0 else None)
    res.update(peak_db=db_(peak), rms_db=db_(rms_d), peak_time_s=round(imax / cha / sra, 6))
    return res


def diff_text(r):
    if r.get("missing"):
        return r["missing"]
    if r["identical"]:
        return "identical"
    parts = []
    if "format" in r:
        parts.append(r["format"])
    if r["frames_a"] != r["frames_b"]:
        parts.append(f"length {r['frames_a']} vs {r['frames_b']} frames")
    if r.get("differing"):
        parts.append(f"first diff at {r['first_time_s']:.4f} s (frame {r['first_frame']}), "
                     f"{r['differing']} samples differ, peak {r['peak_db']} dB, rms {r['rms_db']} dB re rms(A), "
                     f"largest at {r['peak_time_s']:.3f} s")
    if r.get("non_finite"):
        parts.append(f"{r['non_finite']} non-finite samples")
    return "; ".join(parts) or "differs"


def short_diff(r):
    if r.get("missing"):
        return "missing"
    if r["identical"]:
        return "="
    if r.get("differing") and r.get("peak_db") is not None:
        return f"{r['peak_db']:.0f}dB"
    if r["frames_a"] != r["frames_b"]:
        return "len"
    return "diff"


# ---------------------------------------------------------------- render


def options_of(a):
    return {"threads": a.threads, "isa": a.isa, "block": a.block, "sr": SR, "tail": a.tail, "quality": QUALITY}


def load_manifest(d):
    p = os.path.join(d, "manifest.json")
    if os.path.exists(p):
        with open(p) as f:
            return json.load(f)
    return None


def save_manifest(d, m):
    p = os.path.join(d, "manifest.json")
    with open(p + ".part", "w") as f:
        json.dump(m, f, indent=1, sort_keys=True)
        f.write("\n")
    os.replace(p + ".part", p)


def render_set(bin_path, preset, out, threads=1, isa="native", block=128, jobs=2, cores=None,
               tail=3.0, reuse=False, only=None, quiet=False):
    """Renders every scenario into <out>; returns the manifest."""
    a = argparse.Namespace(threads=threads, isa=isa, block=block, tail=tail)
    bin_path, preset, out = os.path.abspath(bin_path), os.path.abspath(preset), os.path.abspath(out)
    cores = cores or default_cores()
    ncores = len(parse_cores(cores))
    jobs = max(1, min(jobs, ncores // max(1, threads)))
    midi_dir = os.path.join(out, "midi")
    scen = scenarios.write(midi_dir)
    if only:
        scen = [s for s in scen if s[0] in only]
    opts = options_of(a)
    bin_sha, preset_sha = sha256_file(bin_path), sha256_file(preset)
    # What made a render, kept with each scenario: a set holds one
    # configuration, and a render is only reused by the one that made it.
    made_by = {"binary_sha256": bin_sha, "preset_sha256": preset_sha, "options": opts}
    old = (load_manifest(out) or {}).get("scenarios", {})
    kept = {k: v for k, v in old.items() if all(v.get(f) == x for f, x in made_by.items())}
    rendered_now = {s[0] for s in scen}
    if only:
        # The scenarios left alone must be this configuration's too.
        foreign = sorted(k for k in old if k not in kept and k not in rendered_now)
        if foreign:
            raise SystemExit(f"{out}: {', '.join(foreign)} rendered with another binary, preset or options "
                             f"(or by an earlier guard.py); render the whole set, or into another directory")
    else:
        kept = {k: v for k, v in kept.items() if k in rendered_now}
    man = {"binary": bin_path, "cores": cores, "jobs": jobs, "scenarios": kept, **made_by}
    env = dict(os.environ)
    env.pop("PIANO_ISA", None)
    if isa != "native":
        env["PIANO_ISA"] = isa

    todo = []
    for name, mid, desc, end_ms in scen:
        wav = os.path.join(out, name + ".wav")
        mid_sha = sha256_file(mid)
        prev = kept.get(name) if reuse else None
        if prev and "error" not in prev and prev.get("midi_sha256") == mid_sha and (
                prev.get("pruned") or (os.path.exists(wav) and sha256_file(wav) == prev.get("sha256"))):
            continue
        todo.append((name, mid, desc, mid_sha, wav))

    def run(item):
        name, mid, desc, mid_sha, wav = item
        part = wav[:-4] + ".part.wav"
        cmd = ["nice", "-n", "19", "ionice", "-c3", "taskset", "-c", cores, bin_path,
               "--sr", str(SR), "--block", str(block), "--tail", repr(float(tail)), "--preset", preset,
               "--threads", str(threads), "--quality", QUALITY, "midi", mid, part]
        t0 = time.monotonic()
        p = subprocess.run(cmd, env=env, capture_output=True, text=True)
        wall = time.monotonic() - t0
        if p.returncode != 0:
            # An earlier render's WAV is not this attempt's result.
            for stale in (part, wav):
                if os.path.exists(stale):
                    os.remove(stale)
            return name, {"error": f"exit {p.returncode}: {p.stderr.strip()[-2000:]}", "wall_s": round(wall, 2),
                          "midi_sha256": mid_sha, "description": desc, **made_by}
        os.replace(part, wav)
        ent = {"description": desc, "midi_sha256": mid_sha, "sha256": sha256_file(wav), "size": os.path.getsize(wav),
               "wall_s": round(wall, 2), "renderer_says": p.stdout.strip().split(": ", 1)[-1], **made_by}
        ent.update(wav_stats(wav))
        return name, ent

    t0 = time.monotonic()
    if not quiet:
        print(f"render {out}: {len(todo)} to do ({len(scen) - len(todo)} reused), {jobs} jobs x {threads} threads "
              f"on cores {cores}, isa {isa}, block {block}", flush=True)
    done = 0

    def record(res):
        nonlocal done
        name, ent = res
        man["scenarios"][name] = ent
        save_manifest(out, man)
        done += 1
        if not quiet:
            msg = ent.get("error") or f"{ent['duration_s']:.2f} s in {ent['wall_s']:.1f} s, sha {ent['sha256'][:12]}"
            if ent.get("non_finite"):
                msg += f", {ent['non_finite']} NON-FINITE samples"
            print(f"  [{done}/{len(todo)}] {name}: {msg}", flush=True)

    if todo:
        # The first render alone: it may build and cache the instrument
        # (~/.cache/unsampled-piano), which parallel processes would race on.
        record(run(todo[0]))
        with cf.ThreadPoolExecutor(max_workers=jobs) as ex:
            for res in ex.map(run, todo[1:]):
                record(res)
    man["total_duration_s"] = round(sum(v.get("duration_s", 0) for v in man["scenarios"].values()), 3)
    man["total_size"] = sum(v.get("size", 0) for v in man["scenarios"].values())
    save_manifest(out, man)
    if not quiet:
        print(f"  done in {time.monotonic() - t0:.1f} s: {man['total_duration_s']:.1f} s of audio, "
              f"{man['total_size'] / 1e6:.1f} MB", flush=True)
    errs = [k for k, v in man["scenarios"].items() if "error" in v]
    if errs:
        print(f"  FAILED: {', '.join(errs)}", file=sys.stderr)
    return man


# ---------------------------------------------------------------- compare


def wav_path(d, name, ent):
    """The scenario's WAV, or the identical file a pruned entry points to."""
    p = os.path.join(d, name + ".wav")
    if not os.path.exists(p) and ent.get("pruned") and os.path.exists(ent.get("identical_to", "")):
        if os.path.getsize(ent["identical_to"]) == ent.get("size"):
            return ent["identical_to"]
    return p


def compare_dirs(da, db, quiet=False):
    """Returns {name: diff dict}; uses manifest hashes for pruned WAVs."""
    ma, mb = load_manifest(da) or {"scenarios": {}}, load_manifest(db) or {"scenarios": {}}
    names = sorted(set(ma["scenarios"]) | set(mb["scenarios"]) |
                   {f[:-4] for d in (da, db) for f in os.listdir(d) if f.endswith(".wav") and ".part" not in f})
    order = [s for s in ma["scenarios"] if s in names] + [s for s in names if s not in ma["scenarios"]]
    out = {}
    for name in order:
        ea, eb = ma["scenarios"].get(name, {}), mb["scenarios"].get(name, {})
        pa, pb = wav_path(da, name, ea), wav_path(db, name, eb)
        failed = [d for d, e in ((da, ea), (db, eb)) if "error" in e]
        if failed:
            # Whatever WAV lies there is not that render's.
            r = {"identical": False, "missing": f"render failed in {' and '.join(failed)}"}
        elif ea.get("midi_sha256") and eb.get("midi_sha256") and ea["midi_sha256"] != eb["midi_sha256"]:
            r = {"identical": False, "missing": "different MIDI input (scenario changed)"}
        elif os.path.exists(pa) and os.path.exists(pb):
            if sha256_file(pa) == sha256_file(pb):
                r = {"identical": True}
            else:
                r = diff_stats(pa, pb)
        elif ea.get("sha256") and eb.get("sha256") and ea["sha256"] == eb["sha256"]:
            r = {"identical": True}  # pruned copy of an identical file
        else:
            r = {"identical": False, "missing": f"missing in {da if not os.path.exists(pa) else db}"}
        out[name] = r
        if not quiet:
            print(f"{name:14s} {diff_text(r)}", flush=True)
    return out


# ---------------------------------------------------------------- matrix

CONFIGS = [  # (directory, threads, isa, block)
    ("t1_native_b128", 1, "native", 128),
    ("t1_native_b128_run2", 1, "native", 128),
    ("t2_native_b128", 2, "native", 128),
    ("t4_native_b128", 4, "native", 128),
    ("t1_avx2_b128", 1, "avx2", 128),
    ("t1_generic_b128", 1, "generic", 128),
    ("t1_native_b64", 1, "native", 64),
    ("t1_native_b32", 1, "native", 32),
    ("t1_native_b100", 1, "native", 100),
]
PAIRS = [  # (group, label, A, B)
    ("run-to-run", "run1/run2", "t1_native_b128", "t1_native_b128_run2"),
    ("threads", "t1/t2", "t1_native_b128", "t2_native_b128"),
    ("threads", "t1/t4", "t1_native_b128", "t4_native_b128"),
    ("isa", "native/avx2", "t1_native_b128", "t1_avx2_b128"),
    ("isa", "native/generic", "t1_native_b128", "t1_generic_b128"),
    ("isa", "avx2/generic", "t1_avx2_b128", "t1_generic_b128"),
    ("block", "128/64", "t1_native_b128", "t1_native_b64"),
    ("block", "128/32", "t1_native_b128", "t1_native_b32"),
    ("block", "128/100", "t1_native_b128", "t1_native_b100"),
]


def matrix(a):
    os.makedirs(a.out, exist_ok=True)
    for d, threads, isa, block in CONFIGS:
        render_set(a.bin, a.preset, os.path.join(a.out, d), threads=threads, isa=isa, block=block,
                   jobs=a.jobs, cores=a.cores, tail=a.tail, reuse=True)
    results = {}
    for group, label, x, y in PAIRS:
        print(f"\n== {group}: {label} ({x} vs {y})", flush=True)
        results[label] = compare_dirs(os.path.join(a.out, x), os.path.join(a.out, y))
    names = list(next(iter(results.values())).keys())
    labels = [p[1] for p in PAIRS]
    w = max(len(n) for n in names) + 1
    lines = ["", "Determinism matrix ('=' byte-identical, else peak |B-A| in dB re rms(A))", ""]
    lines.append(" " * w + " ".join(f"{l:>14s}" for l in labels))
    for n in names:
        lines.append(f"{n:<{w}s}" + " ".join(f"{short_diff(results[l][n]):>14s}" for l in labels))
    lines.append("")
    for l in labels:
        same = all(r["identical"] for r in results[l].values())
        lines.append(f"{l:>15s}: {'identical' if same else 'DIFFERS'}")
    text = "\n".join(lines)
    print(text)
    with open(os.path.join(a.out, "matrix.txt"), "w") as f:
        f.write(text + "\n")
    with open(os.path.join(a.out, "matrix.json"), "w") as f:
        json.dump({"configs": CONFIGS, "pairs": PAIRS, "results": results}, f, indent=1)
    return 0


def prune(d, ref):
    """Deletes WAVs of <d> byte-identical to the same scenario in <ref>;
    keeps their hash in the manifest (marked pruned)."""
    m, mr = load_manifest(d), load_manifest(ref)
    freed = 0
    for name, e in m["scenarios"].items():
        p, pr = os.path.join(d, name + ".wav"), os.path.join(ref, name + ".wav")
        er = mr["scenarios"].get(name, {})
        # The files themselves, not what the manifests say of them.
        if os.path.exists(p) and os.path.exists(pr) and e.get("sha256") and e["sha256"] == er.get("sha256") \
                and sha256_file(p) == e["sha256"] and sha256_file(pr) == e["sha256"]:
            freed += os.path.getsize(p)
            os.remove(p)
            e["pruned"] = True
            e["identical_to"] = os.path.abspath(pr)
    save_manifest(d, m)
    print(f"{d}: freed {freed / 1e6:.1f} MB")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)

    def common(p):
        p.add_argument("--bin", required=True, help="unsampled-piano-render binary")
        p.add_argument("--preset", required=True, help="instrument preset (TOML)")
        p.add_argument("--out", required=True, help="output directory")
        p.add_argument("--jobs", type=int, default=2, help="renders at once (default 2)")
        p.add_argument("--cores", default=None, help="CPU list for taskset (default: $BITEXACT_CORES or all)")
        p.add_argument("--tail", type=float, default=3.0, help="seconds rendered after the last event")

    p = sub.add_parser("render", help="render every scenario")
    common(p)
    p.add_argument("--threads", type=int, default=1)
    p.add_argument("--isa", choices=["generic", "avx2", "native"], default="native")
    p.add_argument("--block", type=int, default=128)
    p.add_argument("--reuse", action="store_true", help="keep WAVs rendered with the same binary/preset/options")
    p.add_argument("--only", nargs="*", help="render only these scenarios")
    p = sub.add_parser("compare", help="byte-compare two rendered sets")
    p.add_argument("a")
    p.add_argument("b")
    p = sub.add_parser("matrix", help="determinism study (run-to-run, threads, ISA, block)")
    common(p)
    p = sub.add_parser("prune", help="delete WAVs identical to a reference set (keeps manifests)")
    p.add_argument("dir")
    p.add_argument("--against", required=True)
    a = ap.parse_args()

    if a.cmd == "render":
        m = render_set(a.bin, a.preset, a.out, a.threads, a.isa, a.block, a.jobs, a.cores, a.tail, a.reuse, a.only)
        return 1 if any("error" in v for v in m["scenarios"].values()) else 0
    if a.cmd == "compare":
        res = compare_dirs(a.a, a.b)
        bad = [n for n, r in res.items() if not r["identical"]]
        print(f"VERDICT: {'IDENTICAL' if not bad else 'DIFFERENT'} "
              f"({len(res) - len(bad)}/{len(res)} identical{': ' + ', '.join(bad) if bad else ''})")
        return 0 if res and not bad else 1
    if a.cmd == "matrix":
        return matrix(a)
    if a.cmd == "prune":
        prune(a.dir, a.against)
        return 0


if __name__ == "__main__":
    sys.exit(main())
