#!/usr/bin/env python3
"""What a piano sounds like between its partials (docs/body.md).

Each partial of a note is found (a stiff-string comb fitted on a 1 s
spectrum, then every partial searched near it, the law refitted on the clear
peaks, and a step-by-step tracker in competition for wound strings that leave
the law) and subtracted: demodulated at its frequency, low-passed at a
quarter of the partial spacing (the envelope reflected at the onset, so that
nothing is smeared before it), remodulated. What remains is the knock, the
body, the other strings, the lines of the bass strings that do not follow the
series, the room and the recording's noise. It works down to A0, where the
partials are 27 Hz apart.

  between_partials.py residual IN.wav OUT.wav --note 60
        write the residual of one note (float WAV, same rate)
  between_partials.py table DIR [DIR…] [--layer 13] [--keys 33,45,…]
        for note sets laid out as <note>v<layer>.{wav,flac}: the residual's
        octave bands (63 Hz–8 kHz) in five windows (0–20 ms, 20–60, 60–150,
        150–400, 400–1000), dB re the note's level over its first 100 ms,
        medians per register; the share of the residual in narrow lines
        (0.1–0.6 s); with --json, every key's table.
"""
import argparse, glob, io, json, os, re, subprocess, sys, warnings
import numpy as np
from scipy.io import wavfile
from scipy.signal import butter, sosfiltfilt

# ffmpeg's piped WAV header has no length.
warnings.filterwarnings("ignore", category=wavfile.WavFileWarning)

NAMES = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"]
OCT = [63, 125, 250, 500, 1000, 2000, 4000, 8000]
WIN = [(0, .02), (.02, .06), (.06, .15), (.15, .4), (.4, 1.0)]
FMAX = 10000.0


def midi_of(name):
    m = re.match(r"([A-G]#?)(-?\d)$", name)
    return NAMES.index(m.group(1)) + 12 * (int(m.group(2)) + 1)


def name_of(m):
    return f"{NAMES[m % 12]}{m // 12 - 1}"


def load(path):
    if path.endswith(".flac"):
        raw = subprocess.run(["ffmpeg", "-v", "quiet", "-i", path, "-f", "wav", "-"], capture_output=True, check=True).stdout
        sr, x = wavfile.read(io.BytesIO(raw))
    else:
        sr, x = wavfile.read(path)
    if x.dtype.kind == "i":
        x = x / float(np.iinfo(x.dtype).max)
    x = x.astype(np.float64)
    if x.ndim == 1:
        x = x[:, None]
    return sr, x[:, :2]


def onset(x):
    e = np.abs(x).max(axis=1)
    return int(np.argmax(e > 0.1 * e.max()))


def spectrum(x, sr, a, n, pad=8):
    seg = x[max(a, 0):a + n]
    if len(seg) < n:
        seg = np.vstack([seg, np.zeros((n - len(seg), seg.shape[1]))])
    w = np.hanning(n + 1)[:-1][:, None]
    L = 1 << int(np.ceil(np.log2(n * pad)))
    P = (np.abs(np.fft.rfft(seg * w, L, axis=0)) ** 2).mean(axis=1)
    return 10 * np.log10(P + 1e-300), sr / L


def partials(x, sr, on, m):
    """Frequencies of the note's partials up to FMAX."""
    fn = 440 * 2 ** ((m - 69) / 12)
    db, bh = spectrum(x, sr, on + int(.05 * sr), int(1.0 * sr), pad=4)
    sm = np.maximum(db - db.max(), -80)
    fmax = min(FMAX, 0.45 * sr, 6000.0)
    best = (-1e18, fn, 0.0)
    for B in np.concatenate([[0], np.geomspace(1e-6, 3e-2, 90)]):
        for cents in np.arange(-100, 100.1, 1.0):
            f1 = fn * 2 ** (cents / 1200)
            k = np.arange(1, 400)
            f = (k * f1 * np.sqrt(1 + B * k * k) / np.sqrt(1 + B))
            f = f[f < fmax][:80]
            if len(f):
                v = np.mean(sm[np.rint(f / bh).astype(int)])
                if v > best[0]:
                    best = (v, f1, B)
    _, f1, B = best
    top = min(FMAX, 0.45 * sr)

    def peak(fc, half):
        a = max(int((fc - half) / bh), 1)
        b = min(int((fc + half) / bh), len(db) - 1)
        if b <= a + 2:
            return fc, False
        i = a + int(np.argmax(db[a:b]))
        if i in (a, b - 1):
            return fc, False
        w = int(.5 * f1 / bh)
        return i * bh, db[i] - np.median(db[max(i - w, 0):i + w]) > 6

    a2, b2 = f1 * f1 / (1 + B), f1 * f1 * B / (1 + B)
    out = []
    for it in range(3):
        out, ks, fs, k = [], [], [], 1
        while True:
            law = k * np.sqrt(max(a2 + b2 * k * k, 1e-9))
            if law > top:
                break
            f, ok = peak(law, (.15 if it == 0 else .08) * f1)
            out.append(f if ok else law)
            if ok:
                ks.append(k)
                fs.append(f)
            k += 1
        if len(ks) < 4:
            break
        ks = np.array(ks, float)
        y = (np.array(fs) / ks) ** 2
        A = np.vstack([np.ones_like(ks), ks ** 2]).T
        keep = np.ones(len(ks), bool)
        for _ in range(3):
            coef, *_ = np.linalg.lstsq(A[keep], y[keep], rcond=None)
            r = np.abs(y - A @ coef)
            keep = r <= 3 * np.median(r[keep]) + 1e-12
        a2, b2 = coef
    law_out = np.array(out)
    seq = list(law_out[:3])
    sp = [seq[1] - seq[0], seq[2] - seq[1]] if len(seq) >= 3 else [f1]
    while seq:
        step = float(np.median(sp[-4:]))
        pred = seq[-1] + step
        if pred > top:
            break
        f, ok = peak(pred, .2 * step)
        if ok:
            sp.append(f - seq[-1])
        seq.append(f if ok else pred)
    seq = np.array(seq)

    def score(fr):
        return np.mean(db[np.clip(np.rint(fr[:150] / bh).astype(int), 0, len(db) - 1)])

    return f1, (seq if len(seq) > 3 and score(seq) > score(law_out) + 0.5 else law_out)


def residual(x, sr, m, fade=(1.2, 2.0)):
    """The note without its partials; zero before the onset, faded out over `fade` (s after it)."""
    on = onset(x)
    f1, fk = partials(x, sr, on, m)
    y = x[on:].copy()
    t = np.arange(len(y)) / sr
    pad = min(int(0.5 * sr), len(y) - 1)
    sos = butter(4, 0.25 * f1, fs=sr, output="sos")
    for f in fk:
        ph = np.exp(-2j * np.pi * f * t)
        for ch in range(y.shape[1]):
            z = y[:, ch] * ph
            zz = np.concatenate([z[pad:0:-1], z])
            a = (sosfiltfilt(sos, zz.real) + 1j * sosfiltfilt(sos, zz.imag))[pad:]
            y[:, ch] -= 2 * np.real(a * np.conj(ph))
    r = np.zeros_like(x)
    r[on:] = y
    g = np.ones(len(x))
    t0, t1 = on + int(fade[0] * sr), on + int(fade[1] * sr)
    g[t0:t1] *= np.linspace(1, 0, len(g[t0:t1]))
    g[t1:] = 0
    return r * g[:, None], on


def bandpow(sig, sr, a, b):
    seg = sig[a:b]
    if len(seg) < 16:
        return [np.nan] * len(OCT)
    w = np.hanning(len(seg))[:, None]
    P = (np.abs(np.fft.rfft(seg * w, axis=0)) ** 2).mean(1)
    f = np.fft.rfftfreq(len(seg), 1 / sr)
    norm = (w[:, 0] ** 2).sum() * len(seg)
    return [10 * np.log10(P[(f >= c / np.sqrt(2)) & (f < c * np.sqrt(2))].sum() / norm + 1e-30) for c in OCT]


def analyse(path, m):
    sr, x = load(path)
    on = onset(x)
    x = x[:on + int(1.3 * sr)]
    r, _ = residual(x, sr, m, fade=(5, 6))
    ref = 10 * np.log10((x[on:on + int(.1 * sr)] ** 2).mean() + 1e-30)
    bands = [[v - ref for v in bandpow(r, sr, on + int(a * sr), on + int(b * sr))] for a, b in WIN]
    db, bh = spectrum(r, sr, on + int(.1 * sr), int(.5 * sr), 4)
    fr = np.arange(len(db)) * bh
    k = int(25 / bh)
    med = np.repeat(np.array([np.median(db[max(i - k, 0):i + k]) for i in range(0, len(db), 4)]), 4)[:len(db)]
    sel = (fr > 50) & (fr < 8000)
    P = 10 ** (db / 10)
    lines = P[sel & (db > med + 10)].sum() / P[sel].sum()
    return dict(bands=bands, lines=float(lines))


def files(d, layer):
    out = {}
    for p in glob.glob(os.path.join(d, f"*v{layer}.*")):
        stem = os.path.basename(p).rsplit(".", 1)[0]
        try:
            out[midi_of(stem.rsplit("v", 1)[0])] = p
        except (AttributeError, ValueError):
            pass
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    r = sub.add_parser("residual")
    r.add_argument("input")
    r.add_argument("output")
    r.add_argument("--note", type=int, required=True)
    t = sub.add_parser("table")
    t.add_argument("dirs", nargs="+")
    t.add_argument("--layer", default="13")
    t.add_argument("--keys")
    t.add_argument("--json")
    a = ap.parse_args()
    if a.cmd == "residual":
        sr, x = load(a.input)
        res, _ = residual(x, sr, a.note)
        wavfile.write(a.output, sr, res.astype(np.float32))
        return
    keys = [int(k) for k in a.keys.split(",")] if a.keys else None
    regs = [("bass A0-B2", 21, 47), ("middle C3-B4", 48, 71), ("treble C5-C8", 72, 108)]
    dump = {}
    for d in a.dirs:
        fs = files(d, a.layer)
        res = {m: analyse(p, m) for m, p in sorted(fs.items()) if keys is None or m in keys}
        dump[d] = {name_of(m): v for m, v in res.items()}
        print(f"== {d} ({len(res)} keys); octaves 125 Hz-4 kHz per window: 0-20 ms | 20-60 | 60-150 | 150-400 | 400-1000")
        for lab, lo, hi in regs:
            rr = [v for m, v in res.items() if lo <= m <= hi]
            if not rr:
                continue
            B = np.nanmedian([v["bands"] for v in rr], axis=0)
            lines = 100 * np.median([v["lines"] for v in rr])
            print(f"  {lab:13} n={len(rr):2} lines {lines:3.0f}%  " + " | ".join(" ".join(f"{x:4.0f}" for x in B[w][1:7]) for w in range(5)))
    if a.json:
        json.dump(dump, open(a.json, "w"), indent=1)


if __name__ == "__main__":
    main()
