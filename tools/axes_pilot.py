#!/usr/bin/env python3
"""Pilot of new comparison axes on isolated notes (bench layout:
<note>v<layer>.wav in a folder).

  axes.py <set dir> [<set dir>…] [--layer 13] [--keys bench|all]

Per set, per register (bass A0–B2, middle C3–B4, treble C5–B6, top C7–C8),
medians over the keys of:
  between: energy between the partials re the energy on them (dB), noise
           subtracted, 300 Hz–5 kHz, windows 0.08–0.6 s and 0.6–1.5 s, and
           per octave band for the early window;
  modul:   rms (dB) of each partial's envelope about its exponential fit,
           0.25–2.5 s, partials 1–3; and the dominant modulation rate (Hz);
  spread:  std over partials 1–6 of their decay rates (dB/s), 0.25–1.5 s.
"""
import argparse, os, sys
import numpy as np
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import bench as b

SR = b.SR
REG = [("bass", 21, 47), ("middle", 48, 71), ("treble", 72, 95), ("top", 96, 108)]

def frames(x, s0, s1, W, hop):
    seg = x[s0:s1]
    n = (len(seg) - W) // hop + 1
    if n < 1: return None
    idx = np.arange(W)[None, :] + hop * np.arange(n)[:, None]
    return seg[idx] * np.hanning(W)[None, :]

def psd(x, s0, s1, W, hop):
    fr = frames(x, s0, s1, W, hop)
    if fr is None: return None
    return (np.abs(np.fft.rfft(fr, axis=1)) ** 2).mean(0)

def noise_psd(x, s, W, hop):
    """Noise spectrum from before the onset when there is enough of it,
    else from the quietest 0.3 s after the peak."""
    if s >= int(0.2 * SR):
        return psd(x, int(0.02 * SR), s - int(0.02 * SR), W, hop)
    h = int(0.3 * SR); n = (len(x) - s) // h
    if n < 2: return np.zeros(W // 2 + 1)
    e = (x[s:s + n * h].reshape(n, h) ** 2).mean(1)
    i = int(np.argmin(e))
    return psd(x, s + i * h, s + (i + 1) * h, W, hop)

def inharm(x, m, s, W):
    """B and the partials' peak frequencies (k → f) in a 0.3–0.8 s window."""
    P = psd(x, s + int(0.3 * SR), s + int(0.8 * SR), W, W // 2)
    f = np.fft.rfftfreq(W, 1 / SR)
    f1 = b.f0(m); peaks = {}
    for k in range(1, 7):
        g = k * f1; w = (f > g * 0.96) & (f < g * 1.05)
        if not w.any(): break
        i = np.argmax(P[w]); pk = f[w][i]
        if 10 * np.log10(P[w][i] / (np.median(P[w]) + 1e-30)) >= 8: peaks[k] = pk
    if 1 not in peaks: return 0.0, f1
    f1 = peaks[1]
    ks = np.array([k for k in peaks if k > 1]); fk = np.array([peaks[k] for k in ks])
    if len(ks) < 2: return 0.0, f1
    y = (fk / (ks * f1)) ** 2 - 1
    B = max(float((y * ks ** 2).sum() / (ks ** 4).sum()), 0.0)
    return B, f1

def partial_mask(f, m, B, f1, P, lo, hi, nbins):
    """Boolean mask of the bins on a partial (found peaks only) in [lo, hi]."""
    on = np.zeros_like(f, dtype=bool); found = 0
    df = f[1] - f[0]
    for k in range(1, 80):
        g = k * f1 * np.sqrt(1 + B * k * k)
        if g > hi * 1.02: break
        if g < lo * 0.98: continue
        w = (f > g * 0.985 - 2 * df) & (f < g * 1.015 + 2 * df)
        if not w.any(): continue
        i = np.argmax(P[w]); pk = f[w][i]
        if 10 * np.log10(P[w][i] / (np.median(P[w]) + 1e-30)) < 8: continue
        half = max(nbins * df, 0.004 * pk)
        on |= (f >= pk - half) & (f <= pk + half); found += 1
    return on, found

def between(x, m, s, B, f1, t0, t1, W=8192):
    hop = W // 2
    P = psd(x, s + int(t0 * SR), s + int(t1 * SR), W, hop)
    if P is None: return None
    N = noise_psd(x, s, W, hop)
    P = np.maximum(P - N, 0.0)
    f = np.fft.rfftfreq(W, 1 / SR)
    out = {}
    for name, lo, hi in (("300-5k", 300, 5000), ("300-600", 300, 600), ("600-1.2k", 600, 1200),
                         ("1.2-2.4k", 1200, 2400), ("2.4-4.8k", 2400, 4800)):
        band = (f >= lo) & (f < hi)
        on, found = partial_mask(f, m, B, f1, P, lo, hi, 2)
        e_on = P[band & on].sum(); e_off = P[band & ~on].sum()
        out[name] = 10 * np.log10((e_off + 1e-30) / (e_on + 1e-30)) if found >= 1 and e_on > 0 else np.nan
    return out

def modulation(x, m, s, B, f1, ks=(1, 2, 3), t0=0.25, t1=2.5, W=4096, hop=480):
    """Per partial: (rms residual dB, p2p dB, dominant rate Hz, decay dB/s)."""
    fr = frames(x, s + int(t0 * SR), s + int(t1 * SR), W, hop)
    if fr is None: return {}
    S = np.abs(np.fft.rfft(fr, axis=1)) ** 2
    f = np.fft.rfftfreq(W, 1 / SR); df = f[1] - f[0]
    N = noise_psd(x, s, W, W // 2)
    res = {}
    for k in ks:
        g = k * f1 * np.sqrt(1 + B * k * k)
        w = np.where((f > g * 0.985 - 2 * df) & (f < g * 1.015 + 2 * df))[0]
        if len(w) == 0: continue
        c = w[np.argmax(S[:5, w].mean(0))]
        sel = slice(max(c - 2, 0), c + 3)
        e = S[:, sel].sum(1); nz = N[sel].sum() * 1.0
        ed = 10 * np.log10(e + 1e-30); nd = 10 * np.log10(nz + 1e-30) + 10
        ok = ed > nd
        if ok.sum() < 20: continue
        last = int(np.argmin(ok)) if not ok.all() else len(ok)
        if last < 20: continue
        t = np.arange(last) * hop / SR; y = ed[:last]
        a, c0 = np.polyfit(t, y, 1)
        # the slow curvature (double decay) apart from the beats
        q = np.polyfit(t, y, 2); r = y - np.polyval(q, t)
        rms = float(np.sqrt((r ** 2).mean())); p2p = float(2 * abs(q[0]) * (t[-1] - t[0]) ** 2 / 4)
        R = np.abs(np.fft.rfft(r - r.mean(), 1 << 12)); fm = np.fft.rfftfreq(1 << 12, hop / SR)
        band = (fm >= 0.3) & (fm <= 12)
        rate = float(fm[band][np.argmax(R[band])]) if band.any() else np.nan
        res[k] = (rms, p2p, rate, float(a))
    return res

def analyse(path, m):
    x = b.load(path); s = b.onset(x)
    B, f1 = inharm(x, m, s, 8192)
    out = {"B": B}
    e = between(x, m, s, B, f1, 0.08, 0.6); l = between(x, m, s, B, f1, 0.6, 1.5)
    if e: out.update({"btw_early_" + k: v for k, v in e.items()})
    if l: out["btw_late_300-5k"] = l["300-5k"]
    mod = modulation(x, m, s, B, f1, ks=(1, 2, 3, 4, 5, 6))
    for k in (1, 2, 3):
        if k in mod: out[f"mod_rms_p{k}"], out[f"mod_p2p_p{k}"], out[f"mod_rate_p{k}"] = mod[k][:3]
    rates = [mod[k][3] for k in mod]
    if len(rates) >= 3: out["decay_spread"] = float(np.std(rates))
    return out

def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("sets", nargs="+"); ap.add_argument("--layer", type=int, default=13)
    ap.add_argument("--keys", default="bench"); ap.add_argument("--json")
    a = ap.parse_args()
    keys = b.KEYS if a.keys == "bench" else list(range(21, 109))
    cols = ["btw_early_300-5k", "btw_late_300-5k", "btw_early_300-600", "btw_early_600-1.2k", "btw_early_1.2-2.4k",
            "btw_early_2.4-4.8k", "mod_rms_p1", "mod_rms_p2", "mod_rms_p3", "mod_rate_p1", "decay_spread"]
    allres = {}
    for d in a.sets:
        per = {}
        for m in keys:
            p = b.find(d, m, a.layer)
            if not p: continue
            try: per[m] = analyse(p, m)
            except Exception as ex: print(f"  {b.name(m)}: {ex}", file=sys.stderr)
        allres[os.path.basename(d.rstrip("/"))] = per
        print(f"\n== {d} ({len(per)} keys)")
        print("register  " + "  ".join(f"{c[:14]:>14}" for c in cols))
        for reg, lo, hi in REG:
            row = []
            for c in cols:
                v = [per[m][c] for m in per if lo <= m <= hi and c in per[m] and np.isfinite(per[m][c])]
                row.append(f"{np.median(v):14.1f}" if v else f"{'—':>14}")
            print(f"{reg:8}  " + "  ".join(row))
    if a.json:
        import json
        json.dump({k: {str(m): v for m, v in per.items()} for k, per in allres.items()}, open(a.json, "w"), indent=1)

if __name__ == "__main__":
    main()
