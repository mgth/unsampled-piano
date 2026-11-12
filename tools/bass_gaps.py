#!/usr/bin/env python3
"""The bass (A0–B2) against its two references, partial by partial (docs/bass_gaps.md).

  tools/bass_gaps.py d <reference dir> name=dir …
      Note sets laid out as <note>v8.wav (velocity 60) and <note>v13.wav (100),
      the first being the reference (the commercial D, recorded dry). Absolute
      levels of the partials under one gain per set (set on the peaks of A2–A5),
      their decay rates, octave bands, the attack's growth from mf to ff and the
      envelope's drop, for A0–B0, C1–B1 and C2–B2.

  tools/bass_gaps.py plath <plath dir> <cache dir> name=preset.toml … [--velocity 105] [--lo 21] [--hi 47]
      Plath's concert grand (bridge acceleration and dummy head, anechoic, lid off)
      against the model's bridge and player's microphones (piano-render's
      bridge_signal example, lid off, no room): each partial's level and decay at
      the bridge (hammer, string and bridge), from the bridge to the ears (board
      and radiation) and at the ears, in physical units on both sides.

A partial is followed in each file on its own: f_k = k·f1·√(1 + B·k²), f1 and B
refitted as the partials are found. A partial less than 10 dB over the median of
the spectrum around it is not read."""
import glob, os, subprocess, sys
from concurrent.futures import ThreadPoolExecutor
import numpy as np

SR = 48000
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
NAMES = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"]
SUB = [("A0–B0", range(21, 24)), ("C1–B1", range(24, 36)), ("C2–B2", range(36, 48))]
GROUPS = [("1", 1, 1), ("2–4", 2, 4), ("5–8", 5, 8), ("9–16", 9, 16), ("17–32", 17, 32), ("33–64", 33, 64)]
KMAX, FMAX = 64, 9000.0
NFFT = 1 << 18

def name(m): return NAMES[m % 12] + str(m // 12 - 1)
def f0(m): return 440 * 2 ** ((m - 69) / 12)

def load_wav(path):
    raw = subprocess.run(["sox", path, "-t", "f32", "-r", str(SR), "-"], check=True, capture_output=True).stdout
    ch = int(subprocess.run(["soxi", "-c", path], check=True, capture_output=True, text=True).stdout)
    return np.frombuffer(raw, dtype=np.float32).astype(float).reshape(-1, ch)

def onset(x):
    a = np.abs(x).sum(1) if x.ndim > 1 else np.abs(x); return int(np.argmax(a > 0.1 * a.max()))

def spec(x, s, t0, t1, n=NFFT):
    """Power spectrum (mean of the channels) of a Hann window, scaled so that a sine of amplitude 1 reads 0 dB."""
    seg = x[s + int(t0 * SR): s + int(t1 * SR)]
    if seg.ndim == 1: seg = seg[:, None]
    if len(seg) < int(0.8 * (t1 - t0) * SR): return None, None
    w = np.hanning(len(seg))[:, None]
    S = (np.abs(np.fft.rfft(seg * w, n, axis=0)) ** 2).mean(1) / (w.sum() / 2) ** 2
    return S, np.fft.rfftfreq(n, 1 / SR)

def track(x, s, m):
    """Frequencies of the partials, from 0.1–1.6 s after the onset: k → Hz (found ones only)."""
    S, f = spec(x, s, 0.1, 1.6)
    if S is None: S, f = spec(x, s, 0.05, 0.8)
    f1, B = f0(m), 1e-4
    df = f[1]; found = {}
    for k in range(1, KMAX + 1):
        g = k * f1 * np.sqrt(1 + B * k * k)
        if g > FMAX: break
        h = 0.3 * f1
        i0, i1 = int((g - h) / df), int((g + h) / df) + 1
        w = S[i0:i1]; c = int(np.argmax(w))
        if w[c] > 10 * np.median(w):
            found[k] = f[i0 + c]
            ks = np.array(sorted(q for q in found if found[q] is not None))
            if len(ks) >= 4:
                y = (np.array([found[q] for q in ks]) / ks) ** 2
                A = np.stack([np.ones(len(ks)), ks ** 2.0], 1); (c0, c1), *_ = np.linalg.lstsq(A, y, rcond=None)
                if c0 > 0 and c1 >= 0: f1, B = np.sqrt(c0), c1 / c0
        found.setdefault(k, None)
    law = lambda k: k * f1 * np.sqrt(1 + B * k * k)
    return {k: (v if v is not None else law(k), v is not None) for k, v in found.items()}, f1, B

def levels(x, s, fk, t0, t1):
    """k → level (dB) of each partial in a window; nan where it does not stand out."""
    S, f = spec(x, s, t0, t1)
    out = {}
    if S is None: return out
    df = f[1]; half = max(1.6 / (t1 - t0), 1.0)
    for k, (g, ok) in fk.items():
        i0, i1 = int((g - half) / df), int((g + half) / df) + 1
        pk = S[i0:i1].max()
        f1 = fk[1][0]; j0, j1 = int((g - 0.5 * f1) / df), int((g + 0.5 * f1) / df) + 1
        out[k] = 10 * np.log10(pk + 1e-30) if pk > 10 * np.median(S[j0:j1]) else np.nan
    return out

def decays(x, s, fk, spans=((0.15, 1.0), (1.0, 2.5))):
    """k → decay rate (dB/s, positive when falling) over each span; nan when the partial is lost before its end."""
    W, hop = 16384, 1024
    y = x[s:] if x.ndim > 1 else x[s:, None]
    n = (len(y) - W) // hop + 1
    if n < 4: return {}
    t = (np.arange(n) * hop + W / 2) / SR
    win = np.hanning(W)[:, None]
    St = np.stack([(np.abs(np.fft.rfft(y[i * hop: i * hop + W] * win, axis=0)) ** 2).mean(1) for i in range(n)])
    ft = np.fft.rfftfreq(W, 1 / SR); df = ft[1]; f1 = fk[1][0]
    out = {}
    for k, (g, ok) in fk.items():
        c = int(round(g / df)); e = 10 * np.log10(St[:, c - 1: c + 2].sum(1) + 1e-30)
        j0, j1 = int((g - 0.5 * f1) / df), int((g + 0.5 * f1) / df) + 1
        floor = 10 * np.log10(3 * np.median(St[:, j0:j1], axis=1) + 1e-30)
        r = []
        for a, b in spans:
            sel = (t >= a) & (t <= b)
            if sel.sum() < 4 or t[-1] < b or not ok or (e[sel] - floor[sel]).min() < 8: r.append(np.nan); continue
            r.append(-np.polyfit(t[sel], e[sel], 1)[0])
        out[k] = r
    return out

def third_bands(lo=25.0, hi=10000.0):
    c = lo; out = []
    while c < hi: out.append(c); c *= 2 ** (1 / 3)
    return out

def band_levels(x, s, t0, t1, centres, width=1 / 6):
    seg = x[s + int(t0 * SR): s + int(t1 * SR)]
    if seg.ndim == 1: seg = seg[:, None]
    w = np.hanning(len(seg))[:, None]
    S = (np.abs(np.fft.rfft(seg * w, axis=0)) ** 2).mean(1) / (w ** 2).sum(); f = np.fft.rfftfreq(len(seg), 1 / SR)
    return np.array([10 * np.log10(S[(f >= c / 2 ** width) & (f < c * 2 ** width)].sum() + 1e-30) for c in centres])

def rms_at(x, s, t, dur=0.1, lo=20.0):
    seg = x[s + int(t * SR): s + int((t + dur) * SR)]
    if len(seg) < int(dur * SR): return np.nan
    X = np.fft.rfft(seg, axis=0); f = np.fft.rfftfreq(len(seg), 1 / SR); X[f < lo] = 0
    return 10 * np.log10((np.fft.irfft(X, len(seg), axis=0) ** 2).mean() + 1e-30)

def peak_db(x):
    y = x.mean(1) if x.ndim > 1 else x
    h = 480; n = len(y) // h; return 10 * np.log10((y[: n * h].reshape(n, h) ** 2).mean(1).max())

def grp(d, lo, hi): return [d[k] for k in range(lo, hi + 1) if k in d and np.all(np.isfinite(d[k]))]
def fm(v, fmt="{:+.1f}"): return fmt.format(v) if np.isfinite(v) else "."
def med(v): return np.median(v) if len(v) else np.nan
def mean(v): return np.mean(v) if len(v) else np.nan
def rms(v): return np.sqrt(np.mean(np.square(v))) if len(v) else np.nan

# ---------------------------------------------------------------- against a recorded note set

OCT = [31.5, 63, 125, 250, 500, 1000, 2000, 4000, 8000]

def note_feats(d, m, layer):
    x = load_wav(os.path.join(d, f"{name(m)}v{layer}.wav")); s = onset(x)
    fk, f1, B = track(x, s, m)
    return dict(early=levels(x, s, fk, 0.0, 0.3), sus=levels(x, s, fk, 0.5, 1.5), dec=decays(x, s, fk), pk=peak_db(x), f1=f1, B=B,
                oct_att=band_levels(x, s, 0.0, 0.06, OCT, 0.5), oct_sus=band_levels(x, s, 0.5, 1.5, OCT, 0.5),
                att=band_levels(x, s, 0.0, 0.06, [1414, 2828, 5657], 0.5),
                env=[rms_at(x, s, t) for t in (0.2, 1.0, 2.0)])

def set_feats(d, keys):
    jobs = [(m, l) for m in keys for l in (8, 13)]
    with ThreadPoolExecutor(8) as ex: res = list(ex.map(lambda j: note_feats(d, *j), jobs))
    return dict(zip(jobs, res))

def cmd_d(argv):
    ref = argv[0]; sets = [a.split("=", 1) for a in argv[1:]]
    keys = list(range(21, 48)); gk = list(range(45, 82, 3))
    def gain_keys(d): return {m: peak_db(load_wav(os.path.join(d, f"{name(m)}v13.wav"))) for m in gk}
    R = set_feats(ref, keys); Rg = gain_keys(ref)
    print("Decay and envelope of the reference itself, velocity 100")
    def own(F, lab):
        print(f"\n{lab}: decay rate of the partials, 0.15–1 s / 1–2.5 s (dB/s, median over keys and partials)")
        print("| keys | " + " | ".join(g for g, _, _ in GROUPS) + " | drop 0.2–2 s / 0.2–1 s |"); print("|---" * (len(GROUPS) + 2) + "|")
        for sl, ks in SUB:
            cells = []
            for _, lo, hi in GROUPS:
                v = [r for m in ks for r in grp(F[(m, 13)]["dec"], lo, hi)]
                a = [r[0] for r in [F[(m, 13)]["dec"].get(k, [np.nan, np.nan]) for m in ks for k in range(lo, hi + 1)] if np.isfinite(r[0])]
                b = [r[1] for r in [F[(m, 13)]["dec"].get(k, [np.nan, np.nan]) for m in ks for k in range(lo, hi + 1)] if np.isfinite(r[1])]
                cells.append(f"{fm(med(a), '{:.1f}')} / {fm(med(b), '{:.1f}')}")
            e = np.array([F[(m, 13)]["env"] for m in ks])
            print(f"| {sl} | " + " | ".join(cells) + f" | {np.nanmean(e[:, 0] - e[:, 2]):.1f} / {np.nanmean(e[:, 0] - e[:, 1]):.1f} |")
    own(R, "reference")
    for nm, d in sets:
        M = set_feats(d, keys); Mg = gain_keys(d)
        g = np.median([Mg[m] - Rg[m] for m in gk])
        own(M, nm)
        for layer, lab in ((13, "velocity 100"), (8, "velocity 60")):
            for win, wl in (("early", "0–0.3 s"), ("sus", "0.5–1.5 s")):
                print(f"\n{nm} − reference at {lab}, {wl}: level of the partials, one gain for the set ({g:+.1f} dB): mean (rms about it, number read)")
                print("| keys | peak | " + " | ".join(gl for gl, _, _ in GROUPS) + " |"); print("|---" * (len(GROUPS) + 2) + "|")
                for sl, ks in SUB:
                    cells = []
                    for _, lo, hi in GROUPS:
                        v = [M[(m, layer)][win][k] - R[(m, layer)][win][k] - g for m in ks for k in range(lo, hi + 1)
                             if k in M[(m, layer)][win] and k in R[(m, layer)][win]]
                        v = [q for q in v if np.isfinite(q)]
                        cells.append(f"{fm(mean(v))} ({fm(np.std(v) if v else np.nan, '{:.1f}')}, {len(v)})")
                    pk = np.mean([M[(m, layer)]["pk"] - R[(m, layer)]["pk"] - g for m in ks])
                    print(f"| {sl} | {pk:+.1f} | " + " | ".join(cells) + " |")
            for key, wl in (("oct_att", "first 60 ms"), ("oct_sus", "0.5–1.5 s")):
                print(f"\n{nm} − reference at {lab}, octave bands, {wl} (dB, same gain)")
                print("| keys | " + " | ".join(f"{c:g}" for c in OCT) + " |"); print("|---" * (len(OCT) + 1) + "|")
                for sl, ks in SUB:
                    v = np.mean([M[(m, layer)][key] - R[(m, layer)][key] - g for m in ks], 0)
                    print(f"| {sl} | " + " | ".join(f"{q:+.1f}" for q in v) + " |")
        print(f"\nGrowth from velocity 60 to 100, first 60 ms: 1–2 / 2–4 / 4–8 kHz / peak (dB), reference then {nm}")
        for sl, ks in SUB:
            gr = lambda F: " / ".join(f"{np.mean([F[(m, 13)]['att'][i] - F[(m, 8)]['att'][i] for m in ks]):.1f}" for i in range(3)) + f" / {np.mean([F[(m, 13)]['pk'] - F[(m, 8)]['pk'] for m in ks]):.1f}"
            print(f"| {sl} | {gr(R)} | {gr(M)} |")
        print(f"\nPer key at velocity 100: {nm} − reference, partials 1 | 2–4 | 5–8 | 9–16 | 17–32 (0.5–1.5 s, dB, same gain) and decay 0.15–1 s of 2–4 and 5–8, reference / {nm}")
        for m in keys:
            a, b = M[(m, 13)], R[(m, 13)]
            lv = [mean([a["sus"][k] - b["sus"][k] - g for k in range(lo, hi + 1) if k in a["sus"] and k in b["sus"] and np.isfinite(a["sus"][k] - b["sus"][k])]) for _, lo, hi in GROUPS[:5]]
            dc = [f"{fm(med([r[0] for r in grp(F['dec'], lo, hi)]), '{:.1f}')}" for lo, hi in ((2, 4), (5, 8)) for F in (b, a)]
            print(f"  {name(m):4} " + " ".join(f"{fm(q):>6}" for q in lv) + f"   p2–4 {dc[0]} / {dc[1]}   p5–8 {dc[2]} / {dc[3]}")

# ---------------------------------------------------------------- against Plath's grand, stage by stage

def load_plath(d, cache, m, state="PROD08", take=2):
    p = os.path.join(cache, f"plath_{state}_{m}_{take}.npy")
    if os.path.exists(p): return np.load(p)
    from scipy.signal import resample_poly
    files = sorted(glob.glob(os.path.join(d, f"D1_{state}_{m - 20:02d}_*M{take:04d}.csv")))
    x = np.loadtxt(files[0], delimiter=";", skiprows=1, usecols=(2, 3, 4), dtype=np.float32).astype(float)
    x -= x.mean(0); y = np.stack([resample_poly(c, 24, 25) for c in x.T], 1).astype(np.float32)
    np.save(p, y); return y

def model_signals(cache, tag, preset, m, velocity, hold=3.2):
    prefix = os.path.join(cache, tag, f"k{m}v{velocity}"); os.makedirs(os.path.dirname(prefix), exist_ok=True)
    if not os.path.exists(prefix + "_mics.f32"):
        nolid = os.path.join(cache, "nolid.toml")
        if not os.path.exists(nolid): open(nolid, "w").write("[soundboard]\nlid_removed = true\n")
        lis = [os.path.join(ROOT, "archive", "presets", "listening", "Microphones", "player.toml"), nolid]
        subprocess.run([os.path.join(ROOT, "target", "release", "examples", "bridge_signal"), preset, prefix, "--note", str(m), "--velocity", str(velocity),
                        "--hold", str(hold), "--tail", "0.1"] + [q for l in lis for q in ("--listening", l)], check=True, capture_output=True)
    v = np.fromfile(prefix + "_bridge.f32", dtype=np.float32).astype(float)
    return np.diff(v, prepend=0.0) * SR, np.fromfile(prefix + "_mics.f32", dtype=np.float32).astype(float).reshape(-1, 2)

def stage_feats(a, p, m):
    sa, sp = onset(a), onset(p)
    fk, f1, B = track(a, sa, m)
    out = dict(fk=fk, f1=f1, B=B)
    for win, t0, t1 in (("early", 0.0, 0.3), ("sus", 0.5, 1.5)):
        out["a_" + win] = levels(a, sa, fk, t0, t1); out["p_" + win] = levels(p, sp, fk, t0, t1)
    out["a_dec"] = decays(a, sa, fk); out["p_dec"] = decays(p, sp, fk)
    TB = third_bands()
    out["a_att"] = band_levels(a, sa, 0.0, 0.05, TB); out["p_att"] = band_levels(p, sp, 0.0, 0.05, TB)
    out["a_body"] = band_levels(a, sa, 0.1, 1.0, TB); out["p_body"] = band_levels(p, sp, 0.1, 1.0, TB)
    out["env"] = [rms_at(p, sp, t) for t in (0.2, 1.0, 2.0)]
    out["a_rms"] = rms_at(a, sa, 0.1, 0.9); out["p_rms"] = rms_at(p, sp, 0.1, 0.9)
    return out

DEC_BANDS = [25, 45, 70, 100, 140, 200, 280, 400, 700, 1200, 2000, 3500]

def cmd_plath(argv):
    opt = {"--velocity": "105", "--lo": "21", "--hi": "47"}; pos = []
    i = 0
    while i < len(argv):
        if argv[i] in opt: opt[argv[i]] = argv[i + 1]; i += 2
        else: pos.append(argv[i]); i += 1
    plath, cache = pos[0], pos[1]; sets = [a.split("=", 1) for a in pos[2:]]; vel = int(opt["--velocity"])
    os.makedirs(cache, exist_ok=True)
    keys = list(range(int(opt["--lo"]), int(opt["--hi"]) + 1))
    def real(m):
        y = load_plath(plath, cache, m).astype(float); return stage_feats(y[:, 0], y[:, 1:], m)
    with ThreadPoolExecutor(6) as ex: R = dict(zip(keys, ex.map(real, keys)))
    sub = [(sl, [m for m in ks if m in R]) for sl, ks in SUB]; sub = [(sl, ks) for sl, ks in sub if ks]
    TB = third_bands()
    def decay_tables(F, lab, sig):
        print(f"\n{lab}: decay rate of the partials {'at the bridge' if sig == 'a_dec' else 'at the ears'}, 0.15–1 s / 1–2.5 s (dB/s, median)")
        print("| keys | " + " | ".join(g for g, _, _ in GROUPS) + " |"); print("|---" * (len(GROUPS) + 1) + "|")
        for sl, ks in sub + [("A0–E1, one string", [m for m in range(21, 29) if m in F])]:
            cells = []
            for _, lo, hi in GROUPS:
                r = [F[m][sig].get(k, [np.nan, np.nan]) for m in ks for k in range(lo, hi + 1)]
                cells.append(f"{fm(med([q[0] for q in r if np.isfinite(q[0])]), '{:.1f}')} / {fm(med([q[1] for q in r if np.isfinite(q[1])]), '{:.1f}')}")
            print(f"| {sl} | " + " | ".join(cells) + " |")
    def by_freq(F, ks, sig, j=0):
        cells = []
        for lo, hi in zip(DEC_BANDS[:-1], DEC_BANDS[1:]):
            v = [F[m][sig][k][j] for m in ks for k in F[m][sig] if lo <= F[m]["fk"][k][0] < hi and np.isfinite(F[m][sig][k][j])]
            cells.append(f"{fm(med(v), '{:.1f}')} ({len(v)})")
        return cells
    decay_tables(R, "the real grand", "a_dec"); decay_tables(R, "the real grand", "p_dec")
    e = lambda F, ks: (np.nanmean([F[m]["env"][0] - F[m]["env"][2] for m in ks]), np.nanmean([F[m]["env"][0] - F[m]["env"][1] for m in ks]))
    print("\nthe real grand: drop at the ears 0.2–2 s / 0.2–1 s: " + ", ".join(f"{sl} {e(R, ks)[0]:.1f} / {e(R, ks)[1]:.1f}" for sl, ks in sub))
    for nm, preset in sets:
        def mod(m):
            a, p = model_signals(cache, nm, preset, m, vel); return stage_feats(a, p, m)
        with ThreadPoolExecutor(6) as ex: M = dict(zip(keys, ex.map(mod, keys)))
        print(f"\n===== {nm}, velocity {vel}")
        decay_tables(M, nm, "a_dec"); decay_tables(M, nm, "p_dec")
        print(f"\n{nm}: drop at the ears 0.2–2 s / 0.2–1 s: " + ", ".join(f"{sl} {e(M, ks)[0]:.1f} / {e(M, ks)[1]:.1f}" for sl, ks in sub))
        print("\nDecay at the bridge by frequency of the partial, 0.15–1 s (dB/s, median, number of partials): real, then " + nm)
        print("| keys | " + " | ".join(f"{a}–{b}" for a, b in zip(DEC_BANDS[:-1], DEC_BANDS[1:])) + " |"); print("|---" * len(DEC_BANDS) + "|")
        for sl, ks in sub + [("A0–E1, one string", [m for m in range(21, 29) if m in R]), ("F1–E2, bass bridge", [m for m in range(29, 41) if m in R]), ("F2–B2, main bridge", [m for m in range(41, 48) if m in R])]:
            if not ks: continue
            print(f"| {sl}, real | " + " | ".join(by_freq(R, ks, "a_dec")) + " |"); print(f"| {sl}, {nm} | " + " | ".join(by_freq(M, ks, "a_dec")) + " |")
        print(f"\nMean level 0.1–1 s, {nm} − real (dB): bridge acceleration / ears / bridge-to-ear: " +
              ", ".join(f"{sl} {np.mean([M[m]['a_rms'] - R[m]['a_rms'] for m in ks]):+.1f} / {np.mean([M[m]['p_rms'] - R[m]['p_rms'] for m in ks]):+.1f} / {np.mean([M[m]['p_rms'] - M[m]['a_rms'] - R[m]['p_rms'] + R[m]['a_rms'] for m in ks]):+.1f}" for sl, ks in sub))
        for win, wl in (("early", "0–0.3 s"), ("sus", "0.5–1.5 s")):
            print(f"\n{nm} − real, {wl}, by partial number: bridge acceleration / bridge-to-ear / ears, mean in dB (rms about the mean of the bridge's and of the transfer's)")
            print("| keys | " + " | ".join(g for g, _, _ in GROUPS) + " |"); print("|---" * (len(GROUPS) + 1) + "|")
            for sl, ks in sub:
                cells = []
                for _, lo, hi in GROUPS:
                    dA, dT, dP = [], [], []
                    for m in ks:
                        for k in range(lo, hi + 1):
                            q = [F[m][s_ + win].get(k, np.nan) for F in (M, R) for s_ in ("a_", "p_")]
                            if np.all(np.isfinite(q)): dA.append(q[0] - q[2]); dP.append(q[1] - q[3]); dT.append(q[1] - q[0] - q[3] + q[2])
                    cells.append(f"{fm(mean(dA))} / {fm(mean(dT))} / {fm(mean(dP))} ({fm(np.std(dA) if dA else np.nan, '{:.1f}')}, {fm(np.std(dT) if dT else np.nan, '{:.1f}')})")
                print(f"| {sl} | " + " | ".join(cells) + " |")
            print(f"\n{nm} − real, {wl}, by frequency of the partial (third octaves): bridge / bridge-to-ear / ears, median (n)")
            print("| Hz | " + " | ".join(sl for sl, _ in sub) + " |"); print("|---" * (len(sub) + 1) + "|")
            for c in TB:
                cells = []
                for sl, ks in sub:
                    dA, dT, dP = [], [], []
                    for m in ks:
                        for k, (g, ok) in R[m]["fk"].items():
                            if not (c / 2 ** (1 / 6) <= g < c * 2 ** (1 / 6)): continue
                            q = [F[m][s_ + win].get(k, np.nan) for F in (M, R) for s_ in ("a_", "p_")]
                            if np.all(np.isfinite(q)): dA.append(q[0] - q[2]); dP.append(q[1] - q[3]); dT.append(q[1] - q[0] - q[3] + q[2])
                    cells.append(f"{fm(med(dA))} / {fm(med(dT))} / {fm(med(dP))} ({len(dA)})" if len(dA) >= 3 else ".")
                if any(q != "." for q in cells): print(f"| {c:.0f} | " + " | ".join(cells) + " |")
        for key, wl in (("att", "first 50 ms"), ("body", "0.1–1 s")):
            print(f"\n{nm} − real, {wl}, third-octave band power (not partials): bridge / bridge-to-ear / ears, mean over the keys")
            print("| Hz | " + " | ".join(sl for sl, _ in sub) + " |"); print("|---" * (len(sub) + 1) + "|")
            for i, c in enumerate(TB):
                cells = []
                for sl, ks in sub:
                    dA = np.mean([M[m]["a_" + key][i] - R[m]["a_" + key][i] for m in ks]); dP = np.mean([M[m]["p_" + key][i] - R[m]["p_" + key][i] for m in ks])
                    cells.append(f"{dA:+.1f} / {dP - dA:+.1f} / {dP:+.1f}")
                print(f"| {c:.0f} | " + " | ".join(cells) + " |")
        print(f"\nPer key: velocity {vel}; f1 and B real / {nm}; bridge rms and ears rms 0.1–1 s, {nm} − real")
        for m in keys:
            print(f"  {name(m):4} f1 {R[m]['f1']:.2f} / {M[m]['f1']:.2f}  B {R[m]['B']:.2e} / {M[m]['B']:.2e}  bridge {M[m]['a_rms'] - R[m]['a_rms']:+.1f}  ears {M[m]['p_rms'] - R[m]['p_rms']:+.1f}")

if __name__ == "__main__":
    if len(sys.argv) < 3 or sys.argv[1] not in ("d", "plath"): raise SystemExit(__doc__)
    (cmd_d if sys.argv[1] == "d" else cmd_plath)(sys.argv[2:])
