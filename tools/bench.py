#!/usr/bin/env python3
"""Test bench: renders the model's note set and scores it against recorded
real pianos with one fixed set of measures, per register.

Each measure is compared with the mean of the real pianos; the gap is
counted in units of their spread (at least a floor per measure), so that a
gap the real pianos themselves show is not a fault. |z| ≤ 1: within the
real pianos, ≤ 2: close, beyond: off. Sources marked "info" (another
model) are measured and shown but never enter the targets.

  tools/bench.py refs name=dir[:info][:nodyn] …
                                            measure the references into
                                            bench/references.json (info: not
                                            a target; nodyn: no ff − pp)
  tools/bench.py run [--preset P] [--name N] [--save-baseline]
                                            render, measure, score, compare
                                            with bench/baseline.json

Note sets are laid out by tools/split_notes.py: <note>v<layer>.{wav,flac},
every third key from A0, layers 4/8/13 (pp/mf/ff), played by the model at
velocities 45/56/100. Docs: docs/bench.md."""
import argparse, json, os, re, subprocess, sys, time
from concurrent.futures import ThreadPoolExecutor
import warnings
import numpy as np, pandas as pd
warnings.filterwarnings("ignore", category=RuntimeWarning)

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
REFS = os.path.join(ROOT, "bench", "references.json")
BASELINE = os.path.join(ROOT, "bench", "baseline.json")
SR = 48000
NAMES = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"]
REGS = [(21, 47, "bass"), (48, 71, "middle"), (72, 95, "treble"), (96, 108, "top")]
KEYS = list(range(21, 109, 3))
# Layer → model velocity: those BiVib recorded (the one reference whose
# layers are measured MIDI velocities).
LAYERS = {4: 45, 8: 56, 13: 100}
HOLD = 20.0
# How the model is heard: close microphones in a studio, as the references
# were recorded (the engine's default is a concert hall heard from 1.5 m,
# whose reverberation would sit in every decay and between the partials).
LISTENING = [os.path.join(ROOT, "presets", "listening", "Microphones", "close.toml"),
             os.path.join(ROOT, "presets", "listening", "Room", "studio.toml")]

def listening_args(files=None):
    """Renderer options for the listening presets (default: LISTENING;
    ["none"]: the engine's own)."""
    files = LISTENING if files is None else [f for f in files if f != "none"]
    return [a for f in files for a in ("--listening", f)]
# Levels below this (dB re the peak) are taken as equal: the recordings'
# noise floor.
FLOOR_DB = -60.0

KNOWN = {
    "iowa": "University of Iowa MIS: Steinway model B, pp/mf/ff, microphones 20 cm above the strings",
    "salamander": "Salamander Grand: Yamaha C5 samples, layers 4/8/13",
    "headroom": "Headroom Piano (Bengt Nilsson): Yamaha C3, close microphones, CC BY 4.0",
    "splendid": "Splendid Grand Piano (AKAI, public domain): Steinway, 4 layers",
    "helgesson": "Maestro Concert Grand (Mats Helgesson): Yamaha CF-3, KM84 X/Y pair, every key",
    "liang": "Yamaha Disklavier grand (Liang's pedalling dataset): every key at velocity 96, no pedal, spaced pair, CC BY 4.0",
    "bivib_grand": "BiVib: Yamaha Disklavier grand DC3 M4, lid open, binaural at the player, recorded at MIDI velocities 45/56/100, CC BY-NC-SA 4.0",
    "bivib_upright": "BiVib: Yamaha Disklavier upright DU1A, open, binaural at the player (an upright: info only)",
    "maps_close": "MAPS ENSTDkCl: Yamaha Disklavier upright, close microphones, M/F at MAPS's own velocities (an upright: info only)",
    "plath": "Plath: concert grand in an anechoic chamber, dummy head at the player, one dynamic (f), CC BY 4.0",
    "commercial": "a commercial modelled piano, recorded note by note in a sequence (another model: info only)",
}

def name(m): return NAMES[m % 12] + str(m // 12 - 1)
def f0(m): return 440 * 2 ** ((m - 69) / 12)

def load(path):
    raw = subprocess.run(["sox", path, "-t", "f32", "-r", str(SR), "-c", "1", "-"], check=True, capture_output=True).stdout
    return np.frombuffer(raw, dtype=np.float32).astype(float)

def key_of(f):
    m = re.fullmatch(r"([A-G]#?)(-?\d)v\d+\.(wav|flac)", f)
    return NAMES.index(m.group(1)) + 12 * (int(m.group(2)) + 1) if m else None

def find(d, m, layer):
    for ext in ("wav", "flac"):
        p = os.path.join(d, f"{name(m)}v{layer}.{ext}")
        if os.path.exists(p): return p

def onset(x):
    a = np.abs(x); return int(np.argmax(a > 0.1 * a.max()))

def peak_db(x):
    h = 480; n = len(x) // h; return 10 * np.log10((x[: n * h].reshape(n, h) ** 2).mean(1).max() + 1e-30)

def onset_level(x):
    """Peak level (dB, 10 ms frames) of the first 0.2 s above 250 Hz: the
    attack as heard, without the fundamentals of the bass."""
    s = max(onset(x) - int(0.01 * SR), 0); seg = x[s: s + int(0.2 * SR)]
    X = np.fft.rfft(seg); X[np.fft.rfftfreq(len(seg), 1 / SR) < 250] = 0
    return peak_db(np.fft.irfft(X, len(seg)))

def partial_level(x, m, k, t0=0.5, t1=1.5):
    """Level (dB) of partial k between t0 and t1 s after the onset, its
    peak found within ±3 % of k·f0."""
    s = onset(x); seg = x[s + int(t0 * SR): s + int(t1 * SR)]; N = 1 << 18
    P = np.abs(np.fft.rfft(seg * np.hanning(len(seg)), N)) ** 2; f = np.fft.rfftfreq(N, 1 / SR)
    g = f0(m) * k; w = (f > g * 0.97) & (f < g * 1.03); fp = f[w][np.argmax(P[w])]
    return 10 * np.log10(P[(f > fp * 0.995) & (f < fp * 1.005)].sum() + 1e-30)

def envelope(x, m, times):
    """Level (dB re the peak) at `times` s after the peak, above 0.7·f0."""
    X = np.fft.rfft(x); X[np.fft.rfftfreq(len(x), 1 / SR) < 0.7 * f0(m)] = 0
    y = np.fft.irfft(X, len(x)); h = 960; n = len(y) // h
    e = 10 * np.log10((y[: n * h].reshape(n, h) ** 2).mean(1) + 1e-30); k = int(np.argmax(e))
    # A level within 6 dB of the recording's own noise (its quietest
    # frames after the peak) cannot be read.
    noise = np.percentile(e[k:], 5) + 6
    def at(t):
        i = k + int(t / 0.02)
        if i >= n or e[i] < noise: return np.nan
        return max(e[i] - e[k], FLOOR_DB)
    return [at(t) for t in times]

def partials(x, m, t0, dur, ks, fmax=12000):
    """Levels of partials `ks` (dB re partial 1) in a window t0 s after the onset."""
    s = onset(x) + int(t0 * SR); n = int(dur * SR); seg = x[s: s + n]
    if len(seg) < n: return [np.nan] * len(ks)
    N = max(n, 4096)
    S = np.abs(np.fft.rfft(seg * np.hanning(n), N)); f = np.fft.rfftfreq(N, 1 / SR)
    def lev(k):
        fk = k * f0(m) * np.sqrt(1 + 4e-4 * k * k * (1 if m > 60 else 0.3))  # loose inharmonic window
        w = (f > fk * 0.94) & (f < fk * 1.08)
        return 20 * np.log10(S[w].max() + 1e-12) if w.any() and fk < fmax else np.nan
    p1 = lev(1)
    return [lev(k) - p1 for k in ks]

def centroid(x, m, t0, dur):
    s = onset(x) + int(t0 * SR); seg = x[s: s + int(dur * SR)]
    S = np.abs(np.fft.rfft(seg * np.hanning(len(seg)))) ** 2; f = np.fft.rfftfreq(len(seg), 1 / SR)
    sel = f > 0.7 * f0(m)
    return (S[sel] * f[sel]).sum() / S[sel].sum() / f0(m)

def band_levels(x, t, cens, dur):
    s = onset(x) + int(t * SR); seg = x[s: s + int(dur * SR)]
    S = np.abs(np.fft.rfft(seg * np.hanning(len(seg)))) ** 2; f = np.fft.rfftfreq(len(seg), 1 / SR)
    return np.array([10 * np.log10(S[(f >= c / 1.414) & (f < c * 1.414)].sum() + 1e-30) for c in cens])

def hf_share(x):
    s = onset(x); seg = x[s: s + int(0.06 * SR)]
    S = np.abs(np.fft.rfft(seg * np.hanning(len(seg)))) ** 2; f = np.fft.rfftfreq(len(seg), 1 / SR); tot = S.sum()
    return [10 * np.log10(S[(f >= a) & (f < b)].sum() / tot + 1e-12) for a, b in ((2000, 4000), (4000, 8000))]

# Measures: name → (family, floor of the tolerance, component labels, log
# scale). Families group them in the summary.
MEASURES = {
    "attack_ff":   ("attack", 3.0, ["p2", "p3", "p4", "p6"], False),
    "attack_mf":   ("attack", 3.0, ["p2", "p3", "p4", "p6"], False),
    "hf_ff":       ("attack", 4.0, ["2-4k", "4-8k"], False),
    "hf_mf":       ("attack", 4.0, ["2-4k", "4-8k"], False),
    "bright_ff":   ("brightness", 0.15, ["centroid"], True),
    "bright_mf":   ("brightness", 0.15, ["centroid"], True),
    "brightening": ("dynamics", 0.15, ["ff/pp"], True),
    "dyn":         ("dynamics", 3.0, ["ff-pp", "ff-mf"], False),
    # The register's attack re the middle's, same layer (above 250 Hz).
    "balance":     ("dynamics", 2.0, ["ff", "mf"], False),
    # Key to key: rms of the levels of partials 1 and 2 (sustain, ff) about
    # their trend (median of the keys within ±6), on the bench's keys from
    # C2 to C7; one figure, filed under the middle.
    "evenness":    ("sustain", 1.5, ["p1", "p2"], False),
    "bands_att":   ("brightness", 3.0, ["1k", "2k", "4k", "8k"], False),
    "bands_sus":   ("brightness", 3.0, ["1k", "2k", "4k", "8k"], False),
    "sustain_ff":  ("sustain", 4.0, ["p2", "p3", "p4", "p6", "p8", "p12"], False),
    "env_ff":      ("decay", 3.0, ["0.3s", "1s", "2s", "3s"], False),
    "env_mf":      ("decay", 3.0, ["0.3s", "1s", "2s", "3s"], False),
    "env_pp":      ("decay", 3.0, ["0.3s", "1s", "2s", "3s"], False),
    "t60_ff":      ("decay", 0.25, ["p1-4 early", "p5-12 early", "p1-4 late"], True),
    "inharm":      ("tuning", 0.15, ["log10 B"], False),
    "stretch":     ("tuning", 5.0, ["cents"], False),
}

def measure_set(d, survey_csv):
    cache = {}
    def get(m, l):
        if (m, l) not in cache:
            p = find(d, m, l); cache[(m, l)] = load(p) if p else None
        return cache[(m, l)]
    out = {}
    sv = pd.read_csv(survey_csv) if survey_csv and os.path.exists(survey_csv) else None
    present = sorted({key_of(f) for f in os.listdir(d) if key_of(f) is not None})
    def reg_level(lo, hi, l):
        v = [onset_level(get(m, l)) for m in present if lo <= m <= hi and get(m, l) is not None]
        return float(np.median(v)) if v else None
    mid_level = {l: reg_level(48, 71, l) for l in (13, 8)}
    # Partials 1–2 of every bench key that the set has, for the evenness.
    plev = {k: {m: partial_level(get(m, 13), m, k) for m in KEYS if m in present and get(m, 13) is not None} for k in (1, 2)}
    for lo, hi, reg in REGS:
        # The keys the set has: sample libraries record every key, every
        # third or every fourth.
        ks = [m for m in present if lo <= m <= hi]
        def mean(fn, layers):
            vals = []
            for m in ks:
                xs = [get(m, l) for l in layers]
                if all(x is not None for x in xs):
                    v = fn(m, *xs)
                    if v is not None: vals.append(np.atleast_1d(np.array(v, dtype=float)))
            if not vals: return None
            with np.errstate(all="ignore"):
                r = np.nanmean(np.array(vals), axis=0)
            return [None if not np.isfinite(v) else float(v) for v in r]
        r = {}
        r["attack_ff"] = mean(lambda m, x: partials(x, m, 0, 0.06, [2, 3, 4, 6]), [13])
        r["attack_mf"] = mean(lambda m, x: partials(x, m, 0, 0.06, [2, 3, 4, 6]), [8])
        r["hf_ff"] = mean(lambda m, x: hf_share(x), [13])
        r["hf_mf"] = mean(lambda m, x: hf_share(x), [8])
        r["bright_ff"] = mean(lambda m, x: np.log(centroid(x, m, 0, 0.05)), [13])
        r["bright_mf"] = mean(lambda m, x: np.log(centroid(x, m, 0, 0.05)), [8])
        r["brightening"] = mean(lambda m, a, c: np.log(centroid(a, m, 0, 0.05) / centroid(c, m, 0, 0.05)), [13, 4])
        r["dyn"] = mean(lambda m, a, b, c: [peak_db(a) - peak_db(c), peak_db(a) - peak_db(b)], [13, 8, 4])
        if reg == "middle":
            ev = []
            for k in (1, 2):
                lv = plev[k]; span = [m for m in sorted(lv) if 36 <= m <= 96]
                res = [lv[m] - np.median([lv[j] for j in lv if abs(j - m) <= 6]) for m in span]
                ev.append(float(np.sqrt(np.mean(np.square(res)))) if len(res) >= 8 else None)
            r["evenness"] = ev if any(v is not None for v in ev) else None
        if reg != "middle":
            lv = [reg_level(lo, hi, l) for l in (13, 8)]
            r["balance"] = [None if v is None or mid_level[l] is None else v - mid_level[l] for v, l in zip(lv, (13, 8))]
            if all(v is None for v in r["balance"]): r["balance"] = None
        # Octave bands re the strongest band of the first 0.1 s; the 8 kHz
        # band of the treble is the recordings' noise.
        allc = [31.5, 63, 125, 250, 500, 1000, 2000, 4000, 8000]
        def bands(m, x, t):
            ref = band_levels(x, 0, allc, 0.1).max(); b = band_levels(x, t, allc[5:], 0.4) - ref
            if m >= 72: b[3] = np.nan
            return b
        r["bands_att"] = mean(lambda m, x: bands(m, x, 0.05), [13])
        r["bands_sus"] = mean(lambda m, x: bands(m, x, 0.6), [13])
        r["sustain_ff"] = mean(lambda m, x: partials(x, m, 0.5, 0.5, [2, 3, 4, 6, 8, 12]), [13])
        times = (0.3, 1.0, 2.0, 3.0)
        r["env_ff"] = mean(lambda m, x: envelope(x, m, times), [13])
        r["env_mf"] = mean(lambda m, x: envelope(x, m, times), [8])
        r["env_pp"] = mean(lambda m, x: envelope(x, m, times), [4])
        if sv is not None:
            # Partials above 12 kHz: the recordings' noise.
            s = sv[(sv.midi >= lo) & (sv.midi <= hi) & (sv.layer == 13) & (sv.freq < 12000)]
            if len(s):
                def med(q, col):
                    v = q.groupby("midi")[col].median().median()
                    return float(np.log(v)) if np.isfinite(v) and v > 0 else None
                r["t60_ff"] = [med(s[s.k <= 4], "t60_early"), med(s[(s.k >= 5) & (s.k <= 12)], "t60_early"), med(s[s.k <= 4], "t60_late")]
                notes = s.groupby("midi").first()
                b = notes.B[notes.B > 0]
                r["inharm"] = [float(np.log10(b).median())] if len(b) else None
                r["stretch"] = [float(notes.cents_et.median())]
        out[reg] = r
    return out

def survey(d, csv):
    exe = os.path.join(ROOT, "target", "release", "piano-survey")
    subprocess.run([exe, d, "--out", csv], check=True, capture_output=True)

def build():
    subprocess.run(["cargo", "build", "--release", "-q", "-p", "piano-render", "-p", "piano-analysis"], cwd=ROOT, check=True)

def render(out, preset, listening=None):
    exe = os.path.join(ROOT, "target", "release", "unsampled-piano-render")
    os.makedirs(out, exist_ok=True)
    jobs = [(m, l, v) for m in KEYS for l, v in LAYERS.items()]
    def one(j):
        m, l, v = j
        cmd = [exe, "note", "--note", str(m), "--velocity", str(v), "--hold", str(HOLD), "--tail", "0.5"]
        if preset: cmd += ["--preset", preset]
        cmd += listening_args(listening)
        subprocess.run(cmd + [os.path.join(out, f"{name(m)}v{l}.wav")], check=True, capture_output=True)
    with ThreadPoolExecutor(6) as ex: list(ex.map(one, jobs))

def score(metrics, refs):
    """z per (register, measure, component) against the real pianos."""
    targets = [s for s in refs["sources"].values() if s["role"] == "target"]
    z = {}
    for reg in metrics:
        for meas, (fam, floor, labels, _) in MEASURES.items():
            v = metrics[reg].get(meas)
            if v is None: continue
            for i, lab in enumerate(labels):
                rv = [t["metrics"].get(reg, {}).get(meas) for t in targets]
                rv = [r[i] for r in rv if r is not None and r[i] is not None]
                if v[i] is None or not rv: continue
                tgt = float(np.mean(rv)); tol = max(float(np.std(rv)), floor)
                z[f"{reg}|{meas}|{lab}"] = dict(value=v[i], target=tgt, tol=tol, z=(v[i] - tgt) / tol, refs=rv)
    return z

def summary(z):
    def rms(vals): return float(np.sqrt(np.mean(np.square(vals)))) if vals else float("nan")
    allz = [c["z"] for c in z.values()]
    fam = {}
    for k, c in z.items():
        fam.setdefault(MEASURES[k.split("|")[1]][0], []).append(c["z"])
    reg = {}
    for k, c in z.items():
        reg.setdefault(k.split("|")[0], []).append(c["z"])
    return dict(score=rms(allz), within=float(np.mean(np.abs(allz) <= 1)), off=int(np.sum(np.abs(allz) > 2)),
                families={f: rms(v) for f, v in fam.items()}, registers={r: rms(v) for r, v in reg.items()})

def fmt_val(meas, v):
    return f"{np.exp(v):.2f}" if MEASURES[meas][3] and meas != "t60_ff" else (f"{np.exp(v):.1f}s" if meas == "t60_ff" else f"{v:+.1f}")

def mark(zv): return "ok" if abs(zv) <= 1 else ("~" if abs(zv) <= 2 else "OFF")

def report(name_, z, info, base, path):
    s = summary(z)
    L = [f"# Bench: {name_}", "", f"Score (RMS of z, lower is better): **{s['score']:.2f}** — "
         f"{100 * s['within']:.0f} % of the measures within the real pianos, {s['off']} off (|z| > 2).", ""]
    if base:
        b = summary(base["z"])
        L += [f"Baseline ({base.get('commit', '?')}): {b['score']:.2f}, {100 * b['within']:.0f} % within, {b['off']} off.", ""]
    L += ["| family | " + " | ".join(s["families"]) + " |", "|---|" + "---|" * len(s["families"]),
          "| RMS z | " + " | ".join(f"{v:.2f}" for v in s["families"].values()) + " |", "",
          "| register | " + " | ".join(s["registers"]) + " |", "|---|" + "---|" * len(s["registers"]),
          "| RMS z | " + " | ".join(f"{v:.2f}" for v in s["registers"].values()) + " |", ""]
    if base:
        moved = []
        for k, c in z.items():
            if k in base["z"]:
                d = abs(c["z"]) - abs(base["z"][k]["z"])
                if abs(d) >= 0.5: moved.append((d, k, base["z"][k]["z"], c["z"]))
        worse = sorted([m for m in moved if m[0] > 0], reverse=True)
        better = sorted([m for m in moved if m[0] < 0])
        L += [f"## Against the baseline: {len(worse)} worse, {len(better)} better (|Δz| ≥ 0.5)", ""]
        for title, rows in (("Worse", worse), ("Better", better)):
            if rows:
                L += [f"{title}:", ""] + [f"- {k}: z {zb:+.1f} → {zn:+.1f}" for _, k, zb, zn in rows[:25]] + [""]
    L += ["## All measures", "", "Value: model; target: mean of the real pianos (their values in brackets); "
          "z = (model − target) / max(spread, floor)." + (" Info: " + ", ".join(info) + " (not in the targets)." if info else ""), ""]
    hdr = "| register | measure | component | model | target [real pianos] | z | |" + "".join(f" {n} z |" for n in info)
    L += [hdr, "|---|---|---|---|---|---|---|" + "---|" * len(info)]
    for k, c in z.items():
        reg, meas, lab = k.split("|")
        refs = ", ".join(fmt_val(meas, r) for r in c["refs"])
        row = f"| {reg} | {meas} | {lab} | {fmt_val(meas, c['value'])} | {fmt_val(meas, c['target'])} [{refs}] | {c['z']:+.1f} | {mark(c['z'])} |"
        for n in info:
            iz = info[n].get(k)
            row += f" {iz['z']:+.1f} |" if iz else " — |"
        L.append(row)
    open(path, "w").write("\n".join(L) + "\n")
    return s

def git_commit():
    r = subprocess.run(["git", "describe", "--always", "--dirty"], cwd=ROOT, capture_output=True, text=True)
    return r.stdout.strip()

def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    a = sub.add_parser("refs"); a.add_argument("sources", nargs="+")
    b = sub.add_parser("run")
    b.add_argument("--preset"); b.add_argument("--name", default="model")
    b.add_argument("--listening", action="append", help="listening preset(s) over the instrument (default: close microphones, studio; none: the engine's own)")
    b.add_argument("--out", default=os.path.join(ROOT, "out", "bench"))
    b.add_argument("--reuse", action="store_true", help="measure the notes already rendered")
    b.add_argument("--no-build", action="store_true")
    b.add_argument("--save-baseline", action="store_true")
    args = ap.parse_args()
    if args.cmd == "refs":
        if not os.path.exists(os.path.join(ROOT, "target", "release", "piano-survey")): build()
        old = json.load(open(REFS)) if os.path.exists(REFS) else {"sources": {}}
        work = os.path.join(ROOT, "out", "bench", "refs"); os.makedirs(work, exist_ok=True)
        for s in args.sources:
            nm, rest = s.split("=", 1); d, *opts = rest.split(":")
            csv = os.path.join(work, f"{nm}.survey.csv")
            print(f"measuring {nm} ({d})", file=sys.stderr)
            # Cut to the model's length, so that decays are fitted on the
            # same span.
            cut = os.path.join(work, nm); os.makedirs(cut, exist_ok=True)
            for f in os.listdir(d):
                stem, ext = os.path.splitext(f)
                if ext in (".wav", ".flac"):
                    subprocess.run(["sox", os.path.join(d, f), "-b", "24", os.path.join(cut, stem + ".wav"), "trim", "0", str(HOLD + 0.5)], check=True)
            survey(cut, csv)
            prev = old["sources"].get(nm, {})
            metrics = measure_set(cut, csv)
            # Levels normalised per layer, or dynamics not tied to MIDI
            # velocities: no ff − pp.
            if "nodyn" in opts:
                for reg in metrics.values(): reg["dyn"] = None
            old["sources"][nm] = dict(role="info" if "info" in opts else "target", description=prev.get("description") or KNOWN.get(nm, ""),
                                      dir=os.path.relpath(os.path.abspath(d), ROOT), options=opts, metrics=metrics)
        os.makedirs(os.path.dirname(REFS), exist_ok=True)
        json.dump(old, open(REFS, "w"), indent=1)
        print(f"wrote {os.path.relpath(REFS, ROOT)}", file=sys.stderr)
        return
    refs = json.load(open(REFS))
    out = os.path.join(args.out, args.name); notes = os.path.join(out, "notes")
    t = time.time()
    if not args.reuse:
        if not args.no_build: build()
        render(notes, args.preset, args.listening)
    csv = os.path.join(out, "survey.csv"); survey(notes, csv)
    metrics = measure_set(notes, csv)
    z = score(metrics, refs)
    info = {n: score(s["metrics"], refs) for n, s in refs["sources"].items() if s["role"] == "info"}
    base = json.load(open(BASELINE)) if os.path.exists(BASELINE) else None
    # The baseline keeps its measures, scored against today's references.
    if base: base["z"] = score(base["metrics"], refs)
    path = os.path.join(out, "scorecard.md")
    s = report(args.name, z, info, base, path)
    json.dump(dict(commit=git_commit(), preset=args.preset, listening=args.listening, metrics=metrics, z=z), open(os.path.join(out, "result.json"), "w"), indent=1)
    print(f"score {s['score']:.2f}, {100 * s['within']:.0f} % within the real pianos, {s['off']} off — "
          + ", ".join(f"{f} {v:.2f}" for f, v in s["families"].items()))
    if base:
        b = summary(base["z"]); print(f"baseline {base.get('commit', '?')}: score {b['score']:.2f}, {b['off']} off")
    print(f"{os.path.relpath(path, ROOT)} ({time.time() - t:.0f} s)")
    if args.save_baseline:
        json.dump(dict(commit=git_commit(), preset=args.preset, metrics=metrics), open(BASELINE, "w"), indent=1)
        print(f"wrote {os.path.relpath(BASELINE, ROOT)}")

if __name__ == "__main__":
    main()
