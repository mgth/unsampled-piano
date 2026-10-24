#!/usr/bin/env python3
"""A recorded performance against the model playing the same MIDI: the
notes are analysed one by one, guided by the MIDI, wherever a note can be
heard on its own, and the same analysis runs on both recordings.

  tools/piece_bench.py <audio> <midi> [--name N] [--preset P] [--out DIR]
  tools/piece_bench.py --batch pairs.tsv [--name N] …   (one pooled report)

Per register (and velocity range for the attack):
- attack: partial profile (2, 3, 4, 6 re 1) of what the onset adds (40 ms
  after minus the 60 ms before), for onsets with no other onset within
  150 ms before or 60 ms after; its share at 2–4 / 4–8 kHz; from C6, the
  energy it adds below the fundamental (f0/4–f0/1.5) re partial 1;
- dynamics: slope of the onset's partial-1 level and of its centroid
  against MIDI velocity (dB and % per 10 velocity steps): the instrument's
  own velocity curve shows only in the slope, not in the level;
- decay: rate (dB/s) of partials 1–2, 3–6 and 7–12 while the key is held,
  for the partials no other sounding note comes within 3 % of;
- release: drop of partials 1–2 in the 0.3 s after the damper falls (key up
  and pedal up), notes below F#6 with no new onset near their partials.

The recording is aligned on the MIDI by cross-correlating its spectral
flux with the MIDI onsets (start and end separately, for clock drift).
The model renders dry; a recording's room lengthens its decays and
releases (its reverberation is the floor of what can be read)."""
import argparse, json, os, subprocess, sys, warnings
import numpy as np
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import midifile
warnings.filterwarnings("ignore", category=RuntimeWarning)

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SR = 48000
REGS = [(21, 47, "bass"), (48, 71, "middle"), (72, 95, "treble"), (96, 108, "top")]
VELS = [(1, 50, "p"), (51, 80, "mf"), (81, 127, "f")]
NO_DAMPER = 89  # F5 + … : the top strings have no damper on most grands

def reg_of(m):
    for lo, hi, r in REGS:
        if lo <= m <= hi: return r

def f0(m): return 440 * 2 ** ((m - 69) / 12)

def load(path):
    raw = subprocess.run(["sox", path, "-t", "f32", "-r", str(SR), "-c", "1", "-"], check=True, capture_output=True).stdout
    return np.frombuffer(raw, dtype=np.float32).astype(float)

def inharm_law():
    """B(m) from the bench's survey of the model (fitted to real pianos),
    else a typical grand's."""
    p = os.path.join(ROOT, "out", "bench", "model", "survey.csv")
    if os.path.exists(p):
        import pandas as pd
        s = pd.read_csv(p); s = s[(s.layer == 13) & (s.B > 0)].groupby("midi").B.first()
        ms, lb = s.index.values, np.log(s.values)
        return lambda m: float(np.exp(np.interp(m, ms, lb)))
    return lambda m: float(np.exp(0.06 * (m - 60)) * 4e-4 + np.exp(-0.1 * (m - 21)) * 2e-4)

B = inharm_law()

def partial_freqs(m, K, tune=1.0):
    k = np.arange(1, K + 1)
    return tune * k * f0(m) * np.sqrt(1 + B(m) * k * k)

# Alignment --------------------------------------------------------------

def flux(x, hop=480, n=2048):
    frames = (len(x) - n) // hop
    w = np.hanning(n); prev = None; out = np.zeros(frames)
    for i in range(frames):
        S = np.log(np.abs(np.fft.rfft(x[i * hop: i * hop + n] * w))[40:] + 1e-6)
        if prev is not None: out[i] = np.maximum(S - prev, 0).sum()
        prev = S
    out -= np.convolve(out, np.ones(50) / 50, "same")
    return np.maximum(out, 0)

def align(x, notes, max_lag=10.0):
    """Time of the MIDI's zero in the recording (s), and the drift
    (recording s per MIDI s), from the first and last thirds."""
    hop = 480; fl = flux(x, hop)
    T = len(fl)
    def lag(t0, t1):
        imp = np.zeros(T)
        for k, v, on, off in notes:
            if t0 <= on < t1:
                i = int(on * SR / hop)
                if i < T: imp[i] += v / 127
        lim = int(max_lag * SR / hop)
        c = np.fft.irfft(np.fft.rfft(fl, 2 * T) * np.conj(np.fft.rfft(imp, 2 * T)), 2 * T)
        c = np.concatenate([c[-lim:], c[:lim]])
        j = int(np.argmax(c))
        # Parabolic interpolation between frames.
        frac = 0.0
        if 0 < j < len(c) - 1:
            den = c[j - 1] - 2 * c[j] + c[j + 1]
            if den < 0: frac = 0.5 * (c[j - 1] - c[j + 1]) / den
        # A flux frame starts n/2 before the instant it stands for.
        return (j + frac - lim) * hop / SR + 1024 / SR
    end = max(n[2] for n in notes)
    if end < 30:
        t0, drift = lag(0, end + 1), 1.0
    else:
        a, b = lag(0, end / 3), lag(2 * end / 3, end + 1)
        t_a, t_b = end / 6, 5 * end / 6
        drift = 1 + (b - a) / (t_b - t_a)
        # Below the resolution of the lags (a few ms): no drift.
        if abs(b - a) < 0.004: t0, drift = (a + b) / 2, 1.0
        else: t0 = a - (drift - 1) * t_a
    return t0 + onset_bias(x, notes, t0, drift), drift

def onset_bias(x, notes, t0, drift):
    """The flux peaks a few ms off the start of the sound: the median gap
    between the MIDI onsets and where the envelope of lone notes crosses
    30 % of its peak."""
    ons = np.array([n[2] for n in notes]); gaps = []
    env = np.abs(x)
    for i, (m, v, on, off) in enumerate(notes):
        if ((np.abs(ons - on) < 0.25) & (np.arange(len(ons)) != i)).any(): continue
        c = int((t0 + drift * on) * SR); w = int(0.06 * SR)
        if c - w < 0 or c + w > len(x): continue
        e = np.convolve(env[c - w: c + w], np.ones(48) / 48, "same")
        base = np.median(e[: w // 2])
        if e.max() < 4 * base: continue
        k = int(np.argmax(e > base + 0.3 * (e.max() - base)))
        gaps.append((k - w) / SR)
        if len(gaps) >= 300: break
    return float(np.median(gaps)) if len(gaps) >= 5 else 0.0

# Analysis ---------------------------------------------------------------

def spectrum(x, t0, dur, N=None):
    s, n = int(t0 * SR), int(dur * SR)
    if s < 0 or s + n > len(x): return None, None
    N = N or max(4 * n, 8192)
    S = np.abs(np.fft.rfft(x[s:s + n] * np.hanning(n), N)) ** 2
    return S, np.fft.rfftfreq(N, 1 / SR)

def levels_at(S, f, freqs, rel=0.015):
    out = []
    for fk in freqs:
        w = (f > fk * (1 - rel)) & (f < fk * (1 + rel))
        out.append(S[w].max() if w.any() and fk < 16000 else np.nan)
    return np.array(out)

def pedal_at(pedal):
    ts = np.array([p[0] for p in pedal] + [1e9]); vs = np.array([p[1] for p in pedal] + [0])
    def down(t):
        i = np.searchsorted(ts, t, "right") - 1
        return i >= 0 and vs[i] >= 64
    def release(t):
        """First time ≥ t with the pedal up."""
        if not down(t): return t
        j = np.searchsorted(ts, t, "right")
        while j < len(ts) and vs[j] >= 64: j += 1
        return ts[j] if j < len(ts) else 1e9
    return down, release

def analyse(x, notes, pedal, t0, drift, tune=1.0):
    """Per-note measures. Times in the recording = t0 + drift·t_midi."""
    def T(t): return t0 + drift * t
    down, release = pedal_at(pedal)
    ons = np.array([n[2] for n in notes])
    res = dict(attack=[], decay=[], release=[])
    # When each note stops sounding (damper down, or 4 s for the undamped).
    # A pedal still down at the end of the file never lets the dampers
    # fall: the note ends with the recording.
    piece_end = max(n[3] for n in notes) + 3
    damp = [min(release(off) if k < NO_DAMPER else off + 4, piece_end) for k, v, on, off in notes]
    for i, (m, v, on, off) in enumerate(notes):
        reg = reg_of(m)
        if reg is None: continue
        fr = partial_freqs(m, 12, tune)
        # Attack: no onset in the 60 ms before (the window subtracted), and
        # the partials that another note struck within 45 ms comes near are
        # left out (partial 1 must be clear).
        others = (np.arange(len(ons)) != i)
        if not ((ons > on - 0.08) & (ons < on - 0.01) & others).any():
            chord = [notes[j][0] for j in np.nonzero((ons >= on - 0.01) & (ons < on + 0.045) & others)[0]]
            clash = np.zeros(12, bool)
            for c in chord:
                fc = partial_freqs(c, 30, tune)
                clash |= (np.abs(fc[None, :] / fr[:, None] - 1) < 0.03).any(1)
            A, f = spectrum(x, T(on) - 0.005, 0.045)
            Bf, _ = spectrum(x, T(on) - 0.07, 0.045)
            if A is not None and not clash[0]:
                la, lb = levels_at(A, f, fr), levels_at(Bf, f, fr)
                add = np.where((la > 2 * lb) & ~clash, la - lb, np.nan)
                if np.isfinite(add[0]) and la[0] > 10 * lb[0]:
                    d = np.maximum(A - Bf, 0); tot = d[f > 0.7 * f0(m)].sum()
                    # Band shares and centroid: lone onsets only.
                    if chord:
                        hf, sub, cen = [np.nan, np.nan], np.nan, np.nan
                    else:
                        hf = [10 * np.log10(d[(f >= a) & (f < b)].sum() / tot + 1e-12) for a, b in ((2000, 4000), (4000, 8000))]
                        sub = 10 * np.log10(d[(f > f0(m) / 4) & (f < f0(m) / 1.5)].sum() / add[0] + 1e-12) if m >= 84 else np.nan
                        # The centroid of the sound itself (as tools/bench.py), not of the
                        # difference: the latter swings with what was ringing before.
                        sel = f > 0.7 * f0(m)
                        cen = (A[sel] * f[sel]).sum() / A[sel].sum() / f0(m)
                    prof = 10 * np.log10(add[[1, 2, 3, 5]] / add[0])
                    res["attack"].append(dict(i=i, m=m, v=v, reg=reg, p1=10 * np.log10(add[0]), prof=prof.tolist(), hf=hf, sub=sub, cen=cen))
        # Decay while held.
        again = [n[2] for n in notes[i + 1:] if n[0] == m]
        end = min([damp[i]] + again)
        if end - on >= 0.6:
            others = [j for j, n in enumerate(notes) if j != i and n[2] < end and damp[j] + 0.3 > on]
            bad = np.zeros(12, bool)
            for j in others:
                fj = partial_freqs(notes[j][0], 30, tune)
                for k in range(12):
                    if (np.abs(fj / fr[k] - 1) < 0.03).any(): bad[k] = True
            if not bad.all():
                ts = np.arange(on + 0.1, end - 0.1, 0.05)
                if len(ts) >= 8:
                    L = []
                    for t in ts:
                        S, f = spectrum(x, T(t) - 0.04, 0.08 if m >= 48 else 0.16, 16384)
                        L.append(10 * np.log10(levels_at(S, f, fr, 0.01) + 1e-30) if S is not None else np.full(12, np.nan))
                    L = np.array(L)
                    rates = []
                    for k in range(12):
                        y = L[:, k]
                        ok = np.isfinite(y)
                        if bad[k] or ok.sum() < 8: rates.append(np.nan); continue
                        p = np.polyfit(ts[ok] - ts[0], y[ok], 1)
                        rates.append(p[0])
                    res["decay"].append(dict(i=i, m=m, v=v, reg=reg, rates=rates, dur=float(end - on)))
        # Release.
        if m < NO_DAMPER and damp[i] - on >= 0.3 and damp[i] < piece_end:
            r = damp[i]
            onsets_near = [n for n in notes if r - 0.2 < n[2] < r + 0.4]
            clash = any((np.abs(partial_freqs(n[0], 20, tune)[:, None] / fr[None, :2] - 1) < 0.03).any() for n in onsets_near)
            if not clash:
                lv = []
                for dt in (-0.1, 0.3):
                    S, f = spectrum(x, T(r + dt) - 0.04, 0.08, 16384)
                    lv.append(10 * np.log10(levels_at(S, f, fr[:2], 0.01) + 1e-30) if S is not None else np.full(2, np.nan))
                res["release"].append(dict(i=i, m=m, v=v, reg=reg, drop=float(np.nanmean(lv[1] - lv[0]))))
    return res

def summarise(res):
    out = {}
    for lo, hi, reg in REGS:
        a = [r for r in res["attack"] if r["reg"] == reg]
        o = {"n_attack": len(a)}
        for vlo, vhi, vl in VELS:
            av = [r for r in a if vlo <= r["v"] <= vhi]
            if len(av) >= 3:
                o[f"attack_{vl}"] = np.nanmedian([r["prof"] for r in av], 0).tolist()
                hv = [r["hf"] for r in av if np.isfinite(r["hf"][0])]
                if len(hv) >= 3: o[f"hf_{vl}"] = np.nanmedian(hv, 0).tolist()
                o[f"n_{vl}"] = len(av)
        if reg == "top" or reg == "treble":
            sub = [r["sub"] for r in a if np.isfinite(r["sub"])]
            if len(sub) >= 3: o["sub_f0"] = float(np.median(sub))
        # Within each performance: their gains differ (the renderer and the
        # recordings are levelled per piece).
        def slope(rows, val):
            sl, w = [], []
            for pc in {r["piece"] for r in rows}:
                q = [r for r in rows if r["piece"] == pc and np.isfinite(val(r))]
                if len(q) >= 8 and np.ptp([r["v"] for r in q]) >= 20:
                    sl.append(np.polyfit([r["v"] for r in q], [val(r) for r in q], 1)[0] * 10); w.append(len(q))
            return float(np.average(sl, weights=w)) if sl else None
        o["vel_slope_db"] = slope(a, lambda r: r["p1"])
        cs = slope(a, lambda r: np.log(r["cen"]) if np.isfinite(r["cen"]) else np.nan)
        o["vel_slope_cen"] = None if cs is None else cs * 100
        d = [r for r in res["decay"] if r["reg"] == reg]
        o["n_decay"] = len(d)
        if d:
            R = np.array([r["rates"] for r in d])
            for (k0, k1), lab in (((0, 2), "decay_1_2"), ((2, 6), "decay_3_6"), ((6, 12), "decay_7_12")):
                vals = R[:, k0:k1].ravel(); vals = vals[np.isfinite(vals)]
                if len(vals) >= 3: o[lab] = float(np.median(vals))
        rl = [r["drop"] for r in res["release"] if r["reg"] == reg and np.isfinite(r["drop"])]
        o["n_release"] = len(rl)
        if len(rl) >= 3: o["release_drop"] = float(np.median(rl))
        out[reg] = o
    return out

def tuning(x, notes, t0, drift):
    """Ratio of the recording's partial 1 to equal temperament at A4 = 440."""
    rs = []
    for m, v, on, off in notes:
        if 48 <= m <= 84 and off - on > 0.3:
            S, f = spectrum(x, t0 + drift * (on + 0.1), 0.2, 65536)
            if S is None: continue
            w = (f > f0(m) * 0.97) & (f < f0(m) * 1.03)
            rs.append(f[w][np.argmax(S[w])] / f0(m))
        if len(rs) >= 40: break
    return float(np.median(rs)) if rs else 1.0

def paired(rec, mod):
    """The notes both analyses kept, partial by partial for the decays: the
    same notes are compared."""
    out = ({}, {})
    for cat in ("attack", "decay", "release"):
        a = {(r["piece"], r["i"]): r for r in rec[cat]}; b = {(r["piece"], r["i"]): r for r in mod[cat]}
        common = sorted(set(a) & set(b))
        ra, rb = [dict(a[k]) for k in common], [dict(b[k]) for k in common]
        if cat == "decay":
            for x, y in zip(ra, rb):
                both = np.isfinite(x["rates"]) & np.isfinite(y["rates"])
                x["rates"] = np.where(both, x["rates"], np.nan).tolist(); y["rates"] = np.where(both, y["rates"], np.nan).tolist()
        out[0][cat], out[1][cat] = ra, rb
    return out

def run_one(audio, midi, out, piece, preset, reuse):
    os.makedirs(out, exist_ok=True)
    notes, pedal = midifile.read(midi)
    model_wav = os.path.join(out, "model.wav")
    if not (reuse and os.path.exists(model_wav)):
        cmd = [os.path.join(ROOT, "target", "release", "unsampled-piano-render"), "midi", midi, model_wav, "--tail", "3"]
        if preset: cmd += ["--preset", preset]
        subprocess.run(cmd, check=True, capture_output=True)
    info, recs = {}, {}
    for src, path in (("recording", audio), ("model", model_wav)):
        x = load(path)
        t0, drift = align(x, notes)
        tune = tuning(x, notes, t0, drift)
        res = analyse(x, notes, pedal, t0, drift, tune)
        for cat in res:
            for r in res[cat]: r["piece"] = piece
        info[src] = dict(offset=t0, drift=drift, tuning_cents=1200 * np.log2(tune))
        recs[src] = res
        print(f"{piece} {src}: offset {t0:+.3f} s, drift {1e6 * (drift - 1):+.0f} ppm, tuning {1200 * np.log2(tune):+.1f} c, "
              f"{len(res['attack'])} attacks, {len(res['decay'])} decays, {len(res['release'])} releases", file=sys.stderr)
    return info, recs

def jsonable(o):
    if isinstance(o, dict): return {k: jsonable(v) for k, v in o.items()}
    if isinstance(o, (list, tuple)): return [jsonable(v) for v in o]
    if isinstance(o, (np.floating, float)): return None if not np.isfinite(o) else float(o)
    if isinstance(o, np.integer): return int(o)
    return o

def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("audio", nargs="?"); ap.add_argument("midi", nargs="?")
    ap.add_argument("--batch", help="file of lines '<audio> <midi>' (tab-separated): one pooled report")
    ap.add_argument("--name"); ap.add_argument("--preset")
    ap.add_argument("--out", default=os.path.join(ROOT, "out", "pieces"))
    ap.add_argument("--reuse", action="store_true", help="keep existing model renders")
    ap.add_argument("--jobs", type=int, default=8)
    ap.add_argument("--summarise", metavar="RESULT_JSON", help="summarise a previous run's notes again")
    a = ap.parse_args()
    if a.summarise:
        def nan(o):
            if isinstance(o, dict): return {k: nan(v) for k, v in o.items()}
            if isinstance(o, list): return [nan(v) for v in o]
            return np.nan if o is None else o
        r = nan(json.load(open(a.summarise)))
        rec, mod = r["notes"]["recording"], r["notes"]["model"]
        result = dict(pieces=r["pieces"], preset=r.get("preset"), recording=dict(summary=summarise(rec)), model=dict(summary=summarise(mod)))
        out = os.path.dirname(a.summarise)
        json.dump(jsonable(dict(result, notes=r["notes"])), open(a.summarise, "w"))
        report(os.path.basename(out), result, os.path.join(out, "report.md"))
        print(os.path.relpath(os.path.join(out, "report.md"), ROOT)); return
    if a.batch:
        pairs = [l.rstrip("\n").split("\t") for l in open(a.batch) if l.strip() and not l.startswith("#")]
        name = a.name or os.path.splitext(os.path.basename(a.batch))[0]
    else:
        pairs = [(a.audio, a.midi)]
        name = a.name or os.path.splitext(os.path.basename(a.audio))[0]
    out = os.path.join(a.out, name)
    pool = {"recording": {"attack": [], "decay": [], "release": []}, "model": {"attack": [], "decay": [], "release": []}}
    infos = {}
    from concurrent.futures import ProcessPoolExecutor
    jobs = []
    for audio, midi in pairs:
        piece = os.path.splitext(os.path.basename(audio))[0]
        d = out if len(pairs) == 1 else os.path.join(out, piece)
        jobs.append((audio, midi, d, piece, a.preset, a.reuse))
    with ProcessPoolExecutor(a.jobs) as ex:
        done = list(ex.map(run_one, *zip(*jobs)))
    for (_, _, _, piece, _, _), (info, recs) in zip(jobs, done):
        infos[piece] = info
        for src in pool:
            for cat in pool[src]: pool[src][cat] += recs[src][cat]
    rec, mod = paired(pool["recording"], pool["model"])
    result = dict(pieces=infos, preset=a.preset, recording=dict(summary=summarise(rec)), model=dict(summary=summarise(mod)))
    os.makedirs(out, exist_ok=True)
    json.dump(jsonable(dict(result, notes=dict(recording=rec, model=mod))), open(os.path.join(out, "result.json"), "w"))
    report(name, result, os.path.join(out, "report.md"))
    print(os.path.relpath(os.path.join(out, "report.md"), ROOT))

def report(name, result, path):
    rec, mod = result["recording"]["summary"], result["model"]["summary"]
    P = result["pieces"]
    L = [f"# {name}: recordings against the model", "",
         f"{len(P)} performance(s); the same notes are measured in the recording and in the model's rendering of its MIDI.", "",
         "| piece | offset (s) | drift (ppm) | tuning (cents re 440) | model offset (s) |", "|---|---|---|---|---|"]
    L += [f"| {p} | {i['recording']['offset']:+.3f} | {1e6 * (i['recording']['drift'] - 1):+.0f} | {i['recording']['tuning_cents']:+.1f} | {i['model']['offset']:+.3f} |" for p, i in list(P.items())[:40]]
    L += ["", "| register | measure | recording | model | model − recording |", "|---|---|---|---|---|"]

    keys = ["attack_p", "attack_mf", "attack_f", "hf_p", "hf_mf", "hf_f", "sub_f0", "vel_slope_db", "vel_slope_cen",
            "decay_1_2", "decay_3_6", "decay_7_12", "release_drop", "n_attack", "n_decay", "n_release"]
    for _, _, reg in REGS:
        for k in keys:
            r, m = rec[reg].get(k), mod[reg].get(k)
            if r is None and m is None: continue
            def f(v): return "—" if v is None else (" ".join(f"{u:+.1f}" for u in v) if isinstance(v, list) else (f"{v}" if isinstance(v, int) else f"{v:+.1f}"))
            diff = "" if (r is None or m is None or k.startswith("n_")) else f(list(np.subtract(m, r)) if isinstance(r, list) else m - r)
            L.append(f"| {reg} | {k} | {f(r)} | {f(m)} | {diff} |")
    L += ["", "attack_*: partials 2, 3, 4, 6 re 1 (dB) of what the onset adds, by velocity (p ≤ 50 < mf ≤ 80 < f); "
          "hf_*: its share at 2–4 / 4–8 kHz (dB); sub_f0: what it adds at f0/4–f0/1.5 re partial 1 (dB, from C6); "
          "vel_slope_db / vel_slope_cen: partial-1 level (dB) and centroid (%) per 10 velocity steps; "
          "decay_*: median decay of the partials while held (dB/s); release_drop: partials 1–2, 0.3 s after the damper (dB)."]
    open(path, "w").write("\n".join(L) + "\n")

if __name__ == "__main__":
    main()
