#!/usr/bin/env python3
"""Velocity curve: BiVib's Disklavier recorded every key at 10 MIDI
velocities (12 to 111, levels calibrated); the model plays every third key
at the same velocities. Per register: peak level re velocity 100 (dB), and
attack brightness (centroid / f0, 0–50 ms) re velocity 100.
  tools/velocity_bench.py "<BiVib>/Grand/binaural samples/grand-open" [--name N] [--preset P]"""
import argparse, json, os, re, subprocess, sys
from concurrent.futures import ThreadPoolExecutor
import numpy as np
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import bench as b

VELS = [12, 23, 34, 45, 56, 67, 78, 89, 100, 111]

def measure(path, m):
    x = b.load(path)
    return b.peak_db(x), b.centroid(x, m, 0, 0.05)

def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("bivib"); ap.add_argument("--name", default="model"); ap.add_argument("--preset")
    ap.add_argument("--reuse", action="store_true")
    ap.add_argument("--listening", action="append", help="listening preset(s) (default: close microphones, studio)")
    a = ap.parse_args()
    out = os.path.join(b.ROOT, "out", "velocity", a.name); os.makedirs(out, exist_ok=True)
    files = {}
    for f in os.listdir(a.bivib):
        mm = re.fullmatch(r"([A-G]#?)(-?\d)_(\d+)_\d+\.wav", f)
        if mm: files[(b.NAMES.index(mm.group(1)) + 12 * (int(mm.group(2)) + 1), int(mm.group(3)))] = os.path.join(a.bivib, f)
    exe = os.path.join(b.ROOT, "target", "release", "unsampled-piano-render")
    jobs = [(m, v) for m in b.KEYS for v in VELS]
    def render(j):
        m, v = j; p = os.path.join(out, f"{b.name(m)}_{v}.wav")
        if not (a.reuse and os.path.exists(p)):
            cmd = [exe, "note", "--note", str(m), "--velocity", str(v), "--hold", "2", "--tail", "0.2"] + (["--preset", a.preset] if a.preset else []) + b.listening_args(a.listening) + b.playback_args()
            subprocess.run(cmd + [p], check=True, capture_output=True)
        return p
    with ThreadPoolExecutor(6) as ex: paths = dict(zip(jobs, ex.map(render, jobs)))
    res = {}
    for src in ("recording", "model"):
        rows = {}
        for m in b.KEYS:
            got = {}
            for v in VELS:
                p = files.get((m, v)) if src == "recording" else paths[(m, v)]
                if p: got[v] = measure(p, m)
            if 100 in got: rows[m] = {v: (got[v][0] - got[100][0], np.log(got[v][1] / got[100][1])) for v in got}
        res[src] = rows
    L = [f"# Velocity curve: {a.name} against BiVib's Disklavier grand", "",
         "Peak level re velocity 100 (dB) / attack centroid re velocity 100 (%), mean over the register's keys.", "",
         "| register | source | " + " | ".join(str(v) for v in VELS) + " |", "|---|---|" + "---|" * len(VELS)]
    summary = {}
    for lo, hi, reg in b.REGS:
        for src in ("recording", "model"):
            cells = []
            for v in VELS:
                vals = [r[v] for m, r in res[src].items() if lo <= m <= hi and v in r]
                if not vals: cells.append("—"); continue
                lv, cv = np.mean([x[0] for x in vals]), np.mean([x[1] for x in vals])
                summary.setdefault(reg, {}).setdefault(src, {})[v] = [float(lv), float(cv)]
                cells.append(f"{lv:+.1f} / {100 * (np.exp(cv) - 1):+.0f}")
            L.append(f"| {reg} | {src} | " + " | ".join(cells) + " |")
    open(os.path.join(out, "report.md"), "w").write("\n".join(L) + "\n")
    json.dump(summary, open(os.path.join(out, "result.json"), "w"), indent=1)
    print(os.path.relpath(os.path.join(out, "report.md"), b.ROOT))

if __name__ == "__main__":
    main()
