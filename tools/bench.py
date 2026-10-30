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
layers 4/8/13 (pp/mf/ff), played by the model at velocities 45/56/100.
Register measures use every third key; local harmonic profiles use all
88 keys, with separate short renders for the intervening keys.
Docs: docs/bench.md."""
import argparse, csv, json, os, re, subprocess, sys, time
from concurrent.futures import ThreadPoolExecutor
import warnings
import numpy as np, pandas as pd
warnings.filterwarnings("ignore", category=RuntimeWarning)

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
REFS = os.path.join(ROOT, "bench", "references.json")
BASELINE = os.path.join(ROOT, "bench", "baseline.json")
HARMONIC_REFS = os.path.join(ROOT, "bench", "harmonic_references.json")
ANALYSIS_VERSION = "channel-power-v3"
SR = 48000
NAMES = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"]
REGS = [(21, 47, "bass"), (48, 71, "middle"), (72, 95, "treble"), (96, 108, "top")]
KEYS = list(range(21, 109, 3))
# Layer → model velocity: those BiVib recorded (the one reference whose
# layers are measured MIDI velocities).
LAYERS = {4: 45, 8: 56, 13: 100}
# Keep the historical register score on its original keys. The local
# comparison fills the intervening keys with shorter, separate renders.
PROFILE_KEYS = list(range(21, 109))
PROFILE_PARTIALS = list(range(2, 9))
PROFILE_WINDOWS = {"attack": (0.0, 0.06), "sustain": (0.5, 0.5)}
PROFILE_SPEC = dict(version=3, channels="mean spectral power; RMS onset", fft_padding=8, partials=PROFILE_PARTIALS,
                    windows={k: list(v) for k, v in PROFILE_WINDOWS.items()},
                    floor_db=4.0, min_refs=2, watch_db=6.0)
HOLD = 20.0
# The attack's high band key to key (hf_even) needs every key: C3–B4 and
# the six keys on either side for their trend, at mf. On the model the keys
# KEYS lacks come from the harmonic profiles' renders (harmonic_notes), so
# the other measures keep the same keys on the model as on the references.
EVEN_SPAN = (48, 71)
EVEN_KEYS = list(range(EVEN_SPAN[0] - 6, EVEN_SPAN[1] + 7))
EVEN_LAYER = 8
# How the model is heard: close microphones in a studio, as the references
# were recorded (the engine's default is a concert hall heard from 1.5 m,
# whose reverberation would sit in every decay and between the partials).
LISTENING = [os.path.join(ROOT, "presets", "listening", "Microphones", "close.toml"),
             os.path.join(ROOT, "presets", "listening", "Room", "studio.toml")]

# The notes the references hold were played back by their instruments (the
# Disklavier's scale, BiVib's measured velocities): the model plays them on
# that scale, not on its default (a pianist's fingers).
PLAYBACK = os.path.join(ROOT, "presets", "touch", "disklavier_playback.toml")

def playback_args():
    return ["--listening", PLAYBACK]

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

def load(path, *, stereo=False):
    """Legacy callers keep mono; the note-set bench requests channel power.

    Mono sources are duplicated by SoX for stereo analysis: their power
    stays the same. No channel averaging is done before the spectrum.
    """
    raw = subprocess.run(["sox", path, "-t", "f32", "-r", str(SR), "-c", "2" if stereo else "1", "-"], check=True, capture_output=True).stdout
    x = np.frombuffer(raw, dtype=np.float32).astype(float)
    return x.reshape(-1, 2) if stereo else x

def channels(x):
    return x[:, None] if x.ndim == 1 else x

def power(x):
    return np.mean(channels(x) ** 2, axis=1)

def spectrum_power(x, n=None):
    """Mean of channel powers, not power of their mean pressure."""
    seg = channels(x)
    S = np.fft.rfft(seg * np.hanning(len(seg))[:, None], n, axis=0)
    return np.mean(np.abs(S) ** 2, axis=1)

def highpass(x, cutoff):
    seg = channels(x)
    X = np.fft.rfft(seg, axis=0)
    X[np.fft.rfftfreq(len(seg), 1 / SR) < cutoff] = 0
    return np.fft.irfft(X, len(seg), axis=0)

def load_stereo(path):
    """Preserve the two channels for local timbre analysis; mono is duplicated."""
    raw = subprocess.run(["sox", path, "-t", "f32", "-r", str(SR), "-c", "2", "-"],
                         check=True, capture_output=True).stdout
    return np.frombuffer(raw, dtype=np.float32).reshape(-1, 2).astype(float)

def key_of(f):
    m = re.fullmatch(r"([A-G]#?)(-?\d)v\d+\.(wav|flac)", f)
    return NAMES.index(m.group(1)) + 12 * (int(m.group(2)) + 1) if m else None

def find(d, m, layer):
    for ext in ("wav", "flac"):
        p = os.path.join(d, f"{name(m)}v{layer}.{ext}")
        if os.path.exists(p): return p

def onset(x):
    a = np.sqrt(power(x)); return int(np.argmax(a > 0.1 * a.max()))

def peak_db(x):
    h = 480; n = len(x) // h; return 10 * np.log10(power(x[: n * h]).reshape(n, h).mean(1).max() + 1e-30)

def onset_level(x):
    """Peak level (dB, 10 ms frames) of the first 0.2 s above 250 Hz: the
    attack as heard, without the fundamentals of the bass."""
    s = max(onset(x) - int(0.01 * SR), 0); seg = x[s: s + int(0.2 * SR)]
    return peak_db(highpass(seg, 250))

def partial_level(x, m, k, t0=0.5, t1=1.5):
    """Level (dB) of partial k between t0 and t1 s after the onset, its
    peak found within ±3 % of k·f0."""
    s = onset(x); seg = x[s + int(t0 * SR): s + int(t1 * SR)]; N = 1 << 18
    P = spectrum_power(seg, N); f = np.fft.rfftfreq(N, 1 / SR)
    g = f0(m) * k; w = (f > g * 0.97) & (f < g * 1.03); fp = f[w][np.argmax(P[w])]
    lv = 10 * np.log10(P[(f > fp * 0.995) & (f < fp * 1.005)].sum() + 1e-30)
    if 0.06 * g < 8 * 2 * SR / len(seg): return lv
    return lv if 10 * np.log10(P[w].max() / (np.median(P[w]) + 1e-30)) >= NOISE_MARGIN else np.nan

def envelope(x, m, times):
    """Level (dB re the peak) at `times` s after the peak, above 0.7·f0."""
    y = highpass(x, 0.7 * f0(m)); h = 960; n = len(y) // h
    e = 10 * np.log10(power(y[: n * h]).reshape(n, h).mean(1) + 1e-30); k = int(np.argmax(e))
    # A level within 6 dB of the recording's own noise (its quietest
    # frames after the peak) cannot be read.
    noise = np.percentile(e[k:], 5) + 6
    def at(t):
        i = k + int(t / 0.02)
        if i >= n or e[i] < noise: return np.nan
        return max(e[i] - e[k], FLOOR_DB)
    return [at(t) for t in times]

# A partial must stand this far (dB) above the median of its search window
# (the recording's noise between the partials), else it is not measured:
# the real top notes' sustain sits on the recordings' noise floor.
NOISE_MARGIN = 12.0
# Partials below this (dB re partial 1) are counted at it: the recordings
# read 4 of their ~3100 partials below it (none below −95), the model's
# notches went down to −120 and moved a register's mean by 10 dB as one
# key's partial was read or lost (docs/bench.md, "Weak partials").
PARTIAL_FLOOR_DB = -80.0

def partials(x, m, t0, dur, ks, fmax=12000):
    """Levels of partials `ks` (dB re partial 1) in a window t0 s after the
    onset, at least PARTIAL_FLOOR_DB; NaN for a partial lost in a noise
    above that floor (a partial lost below it is at the floor)."""
    s = onset(x) + int(t0 * SR); n = int(dur * SR); seg = x[s: s + n]
    if len(seg) < n: return [np.nan] * len(ks)
    N = max(n, 4096)
    S = np.sqrt(spectrum_power(seg, N)); f = np.fft.rfftfreq(N, 1 / SR)
    def lev(k):
        """(peak, noise) in dB; noise NaN when the partial stands out."""
        fk = k * f0(m) * np.sqrt(1 + 4e-4 * k * k * (1 if m > 60 else 0.3))  # loose inharmonic window
        w = (f > fk * 0.94) & (f < fk * 1.08)
        if not w.any() or fk >= fmax: return np.nan, np.nan
        peak = 20 * np.log10(S[w].max() + 1e-12)
        # The median is a floor only when the window spans several of the
        # analysis' resolution cells (Hann main lobe ±2·SR/n).
        if 0.14 * fk < 8 * 2 * SR / n: return peak, np.nan
        noise = 20 * np.log10(np.median(S[w]) + 1e-12)
        return (peak, np.nan) if peak - noise >= NOISE_MARGIN else (np.nan, noise)
    p1 = lev(1)[0]
    def rel(k):
        peak, noise = lev(k)
        if np.isfinite(peak): return max(peak - p1, PARTIAL_FLOOR_DB)
        return PARTIAL_FLOOR_DB if noise - p1 < PARTIAL_FLOOR_DB else np.nan
    return [rel(k) if np.isfinite(p1) else np.nan for k in ks]

def centroid(x, m, t0, dur):
    s = onset(x) + int(t0 * SR); seg = x[s: s + int(dur * SR)]
    S = spectrum_power(seg); f = np.fft.rfftfreq(len(seg), 1 / SR)
    sel = f > 0.7 * f0(m)
    return (S[sel] * f[sel]).sum() / S[sel].sum() / f0(m)

def band_levels(x, t, cens, dur):
    s = onset(x) + int(t * SR); seg = x[s: s + int(dur * SR)]
    S = spectrum_power(seg); f = np.fft.rfftfreq(len(seg), 1 / SR)
    return np.array([10 * np.log10(S[(f >= c / 1.414) & (f < c * 1.414)].sum() + 1e-30) for c in cens])

def hf_share(x):
    s = onset(x); seg = x[s: s + int(0.06 * SR)]
    S = spectrum_power(seg); f = np.fft.rfftfreq(len(seg), 1 / SR); tot = S.sum()
    return [10 * np.log10(S[(f >= a) & (f < b)].sum() / tot + 1e-12) for a, b in ((2000, 4000), (4000, 8000))]

def hf_even_share(x, gate=True):
    """Share (dB) of 4–8 and 8–11.3 kHz in the first 60 ms, as hf_share.
    With `gate` (recordings), NaN for a band within 6 dB of the recording's
    noise (the same window before the onset, else over its last second).
    The model's renders have no recording noise, and what follows their
    attack depends on how long the key is held: they are read ungated."""
    s = onset(x); n = int(0.06 * SR); seg = x[s: s + n]
    if len(seg) < n: return [np.nan, np.nan]
    f = np.fft.rfftfreq(n, 1 / SR)
    S = spectrum_power(seg); tot = S.sum()
    if not gate:
        return [10 * np.log10(S[(f >= a) & (f < b)].sum() / tot + 1e-30) for a, b in ((4000, 8000), (8000, 11300))]
    pre = x[: max(s - int(0.01 * SR), 0)]
    quiet = pre if len(pre) >= int(0.1 * SR) else x[-SR:]
    k = len(quiet) // n
    N = np.mean([spectrum_power(quiet[i * n: (i + 1) * n]) for i in range(k)], axis=0) if k else np.full_like(S, np.inf)
    out = []
    for a, b in ((4000, 8000), (8000, 11300)):
        band = (f >= a) & (f < b); e = S[band].sum()
        out.append(10 * np.log10(e / tot + 1e-30) if e > 4 * N[band].sum() else np.nan)
    return out

def hf_even(levels):
    """Key to key: rms and largest gap (dB) of each band's level about its
    trend (median of the other keys within ±6), over C3–B4. `levels`: key →
    [4–8k, 8–11k]. None for a band read on fewer than 10 keys."""
    rms, top = [], []
    for i in range(2):
        lv = {m: v[i] for m, v in levels.items() if np.isfinite(v[i])}
        res = []
        for m in lv:
            nb = [lv[j] for j in lv if j != m and abs(j - m) <= 6]
            if EVEN_SPAN[0] <= m <= EVEN_SPAN[1] and len(nb) >= 6: res.append(lv[m] - np.median(nb))
        ok = len(res) >= 10
        rms.append(float(np.sqrt(np.mean(np.square(res)))) if ok else None)
        top.append(float(np.max(np.abs(res))) if ok else None)
    return rms + top

def frame_power(x, s0, s1, W):
    """Mean power spectrum (channels and half-overlapping Hann frames
    averaged) of x[s0:s1]; None when shorter than a frame."""
    seg = channels(x)[s0:s1]; hop = W // 2; n = (len(seg) - W) // hop + 1
    if n < 1: return None
    win = np.hanning(W)[:, None]
    return np.mean([np.mean(np.abs(np.fft.rfft(seg[i * hop: i * hop + W] * win, axis=0)) ** 2, axis=1) for i in range(n)], axis=0)

def found_partials(f, P, m, kmax, fmax, margin=8):
    """Peak frequencies of the partials standing `margin` dB above their
    window, the note's inharmonicity fitted on partials 1–6."""
    f1 = f0(m); pk = {}
    for k in range(1, 7):
        g = k * f1; w = (f > g * 0.96) & (f < g * 1.05)
        if not w.any(): break
        i = np.argmax(P[w])
        if 10 * np.log10(P[w][i] / (np.median(P[w]) + 1e-30)) >= margin: pk[k] = f[w][i]
    f1 = pk.get(1, f1); B = 0.0
    ks = np.array([k for k in pk if k > 1]); fk = np.array([pk[k] for k in ks])
    if 1 in pk and len(ks) >= 2:
        y = (fk / (ks * f1)) ** 2 - 1; B = float(np.clip((y * ks ** 2).sum() / (ks ** 4).sum(), 0, 0.01))
    out = []; df = f[1] - f[0]
    for k in range(1, kmax):
        g = k * f1 * np.sqrt(1 + B * k * k)
        if g > fmax: break
        w = (f > g * 0.985 - 2 * df) & (f < g * 1.015 + 2 * df)
        if not w.any(): continue
        # Against a wider window: a partial whose unison beats is a few
        # bins wide and would raise the median of its own narrow window.
        wide = np.abs(f - g) < max(0.05 * g, 10 * df)
        i = np.argmax(P[w])
        if 10 * np.log10(P[w][i] / (np.median(P[wide]) + 1e-30)) >= margin: out.append(f[w][i])
    return out

def between_partials(x, m, windows=((0.08, 0.6), (0.6, 1.5)), lo=300, hi=5000, W=8192):
    """Energy between the partials re the energy on them (dB), 300 Hz–5 kHz,
    in each window (s after the onset): the halo of the strings and
    resonators that ring with the note (docs/realism_axes.md, axis 1). NaN
    when no partial is found in the band. No noise is subtracted: only one
    set has silence before its onsets, and the quietest stretch after one
    still holds the note, so what it took off depended on the file's
    length. Without it the attack reads the same on every set (±0.1 dB);
    the top's sustain, on the recordings' noise, is left out by the caller."""
    s = onset(x); f = np.fft.rfftfreq(W, 1 / SR); df = f[1] - f[0]
    ref = frame_power(x, s + int(0.3 * SR), s + int(0.8 * SR), W)
    peaks = found_partials(f, ref, m, 80, hi * 1.02) if ref is not None else []
    on = np.zeros_like(f, dtype=bool)
    for pk in peaks:
        half = max(2 * df, 0.004 * pk); on |= (f >= pk - half) & (f <= pk + half)
    band = (f >= lo) & (f < hi)
    out = []
    for t0, t1 in windows:
        P = frame_power(x, s + int(t0 * SR), s + int(t1 * SR), W)
        if P is None or not (band & on).any(): out.append(np.nan); continue
        e_on = P[band & on].sum()
        out.append(10 * np.log10((P[band & ~on].sum() + 1e-30) / e_on) if e_on > 0 else np.nan)
    return out

# Fixed lines: off-partial peaks found at the same frequency on several of
# the C6–C8 keys, the resonators the instrument rings whatever the key
# (docs/realism_axes.md, axis 2). Their level counts the three strongest;
# a missing one counts at the detection limit.
LINE_KEYS = list(range(84, 109, 3))
LINE_LIMIT_DB = -45.0

def off_peaks(x, m):
    """(frequency, dB re the note's strongest partial) of the peaks between
    the partials, 0.1–0.5 s, 0.8–6 kHz, 12 dB above their ±150 Hz median."""
    s = onset(x) + int(0.1 * SR); n = int(0.4 * SR); seg = x[s: s + n]
    if len(seg) < n: return []
    N = 1 << 16; L = 10 * np.log10(spectrum_power(seg, N) + 1e-30); f = np.fft.rfftfreq(N, 1 / SR)
    P = 10 ** (L / 10); on = np.zeros_like(f, dtype=bool); top = -1e9
    for pk in found_partials(f, P, m, 120, 7000, margin=-np.inf):
        top = max(top, L[np.argmin(np.abs(f - pk))]); on |= (f > pk * 0.985) & (f < pk * 1.015)
    out = []
    for i in np.where((f >= 800) & (f <= 6000) & ~on)[0][1:-1]:
        if L[i] > L[i - 1] and L[i] >= L[i + 1] and L[i] - top > LINE_LIMIT_DB:
            w = (f > f[i] - 150) & (f < f[i] + 150)
            if L[i] - np.median(L[w]) >= 12: out.append((f[i], L[i] - top))
    return out

def fixed_lines(notes, min_notes=3):
    """Median level (dB re the strongest partial) of the three strongest
    lines shared (±0.5 %) by at least `min_notes` of `notes` (key → signal);
    None with fewer than `min_notes` notes."""
    if len(notes) < min_notes: return None
    found = {}
    for m, x in notes.items():
        for fr, lv in off_peaks(x, m):
            g = next((g for g in found if abs(g - fr) / fr < 0.005), fr)
            found.setdefault(g, []).append((m, lv))
    levels = sorted((float(np.median([l for _, l in v])) for v in found.values() if len({m for m, _ in v}) >= min_notes), reverse=True)
    return float(np.median((levels + [LINE_LIMIT_DB] * 3)[:3]))

def stereo_partial_levels(x, m, t0, duration, ks=range(1, 9), *, fft_padding=8):
    """Peak amplitudes in dBFS from mean channel powers, never a mono sum.

    A full-scale bin-centred sinusoid in both channels reads 0 dBFS.
    Both onset and spectrum are invariant to channel polarity changes.
    Search windows, noise exclusions and the 12 kHz ceiling match partials().
    """
    if x.ndim == 1: x = x[:, None]
    if not len(x) or np.max(np.abs(x)) <= 1e-12:
        return np.full(len(ks), np.nan)
    energy = np.sqrt(np.mean(x * x, axis=1))
    start = onset(energy) + int(t0 * SR)
    n = int(duration * SR); seg = x[start:start + n]
    if len(seg) < n: return np.full(len(ks), np.nan)
    window = np.hanning(n)
    N = max(fft_padding * n, 4096)
    power = np.mean(np.abs(np.fft.rfft(seg * window[:, None], N, axis=0)) ** 2, axis=1)
    power *= (2 / window.sum()) ** 2
    f = np.fft.rfftfreq(N, 1 / SR)
    result = []
    for k in ks:
        fk = k * f0(m) * np.sqrt(1 + 4e-4 * k * k * (1 if m > 60 else 0.3))
        w = (f > fk * 0.94) & (f < fk * 1.08)
        if fk >= 12000 or not w.any():
            result.append(np.nan); continue
        peak = 10 * np.log10(power[w].max() + 1e-30)
        if 0.14 * fk >= 8 * 2 * SR / n and peak - 10 * np.log10(np.median(power[w]) + 1e-30) < NOISE_MARGIN:
            peak = np.nan
        result.append(peak)
    return np.array(result)

def measure_profiles(directories):
    """Exact-key profiles, with no interpolation or averaging over notes.

    Decode at most four recordings concurrently; unreadable partials stay null.
    Earlier directories take precedence (the historical long renders).
    """
    jobs = []
    for m in PROFILE_KEYS:
        for layer in LAYERS:
            path = next((p for d in directories if os.path.isdir(d)
                         for p in [find(d, m, layer)] if p), None)
            if path is None: continue
            jobs.append((m, layer, path))
    def measure(job):
        m, layer, path = job
        # Keep the full signal: onset is relative to its global peak. A fixed
        # 1.2 s trim would change that threshold or lose delayed attacks.
        x = load_stereo(path)
        profiles = {}
        for phase, (start, duration) in PROFILE_WINDOWS.items():
            levels = stereo_partial_levels(x, m, start, duration, [1] + PROFILE_PARTIALS)
            values = levels[1:] - levels[0]
            profiles[f"{m}|{layer}|{phase}"] = [float(v) if np.isfinite(v) else None for v in values]
        return profiles
    with ThreadPoolExecutor(max_workers=4) as pool:
        return {key: values for result in pool.map(measure, jobs) for key, values in result.items()}

def profile_reference_cache(names=None):
    old = {}
    if os.path.exists(HARMONIC_REFS):
        with open(HARMONIC_REFS) as f: old = json.load(f)
    if old.get("spec") != PROFILE_SPEC:
        if names is not None:
            raise ValueError("Harmonic specification changed or cache missing; partial refs update refused. "
                             "Run tools/bench.py refs --harmonics-only with all target corpora available first.")
        old = dict(spec=PROFILE_SPEC, sources={})
    return old

def refresh_profile_refs(refs, names=None):
    old = profile_reference_cache(names)
    for nm, source in refs["sources"].items():
        if source["role"] != "target" or (names is not None and nm not in names): continue
        directory = os.path.join(ROOT, source["dir"])
        if not os.path.isdir(directory):
            raise FileNotFoundError(f"harmonic reference {nm}: {directory}")
        print(f"measuring harmonic profiles: {nm}", file=sys.stderr)
        old["sources"][nm] = dict(dir=source["dir"], profiles=measure_profiles([directory]))
    os.makedirs(os.path.dirname(HARMONIC_REFS), exist_ok=True)
    with open(HARMONIC_REFS, "w") as f:
        json.dump(old, f, indent=1, allow_nan=False)

def load_profile_refs(refs):
    if not os.path.exists(HARMONIC_REFS):
        raise ValueError("Missing harmonic references; run tools/bench.py refs --harmonics-only")
    with open(HARMONIC_REFS) as f: data = json.load(f)
    if data.get("spec") != PROFILE_SPEC:
        raise ValueError("Outdated harmonic references; run tools/bench.py refs --harmonics-only")
    targets = {}
    for nm, source in refs["sources"].items():
        if source["role"] != "target": continue
        saved = data["sources"].get(nm)
        if saved is None or saved["dir"] != source["dir"]:
            raise ValueError(f"Missing/outdated harmonic reference {nm}; run tools/bench.py refs --harmonics-only")
        targets[nm] = saved["profiles"]
    return targets

def score_profiles(profiles, targets):
    """Local warnings survive opposite errors on other keys.

    OFF: any partial beyond 2 spreads. WATCH: at least two partials in
    the same direction beyond 1 spread AND 6 dB. This is a spectral
    discrepancy warning, not a perceptual classifier for nasality.
    """
    result = {}
    for m in PROFILE_KEYS:
        for layer, velocity in LAYERS.items():
            for phase in PROFILE_WINDOWS:
                key = f"{m}|{layer}|{phase}"
                values = profiles.get(key, [None] * len(PROFILE_PARTIALS))
                components = []
                for i, k in enumerate(PROFILE_PARTIALS):
                    rv = {nm: p[key][i] for nm, p in targets.items()
                          if key in p and p[key][i] is not None and np.isfinite(p[key][i])}
                    row = dict(partial=k, value=values[i], refs=rv, target=None, delta_db=None, z=None)
                    if values[i] is not None and np.isfinite(values[i]) and len(rv) >= PROFILE_SPEC["min_refs"]:
                        target = float(np.mean(list(rv.values())))
                        tol = max(float(np.std(list(rv.values()))), PROFILE_SPEC["floor_db"])
                        row.update(target=target, delta_db=values[i] - target, tol=tol,
                                   z=(values[i] - target) / tol)
                    components.append(row)
                scored = [c for c in components if c["z"] is not None]
                coherent = max((sum(sign * c["z"] > 1 and sign * c["delta_db"] >= PROFILE_SPEC["watch_db"]
                                    for c in scored) for sign in (-1, 1)), default=0)
                status = ("OFF" if any(abs(c["z"]) > 2 for c in scored) else
                          "WATCH" if coherent >= 2 else "ok" if len(scored) >= 2 else "insufficient")
                result[key] = dict(note=name(m), midi=m, layer=layer, velocity=velocity,
                                   phase=phase, status=status, measured=key in profiles,
                                   compared=len(scored), components=components)
    return result

def profile_summary(rows):
    counts = {status: sum(r["status"] == status for r in rows.values())
              for status in ("OFF", "WATCH", "ok", "insufficient")}
    counts["missing"] = sum(not r["measured"] for r in rows.values())
    counts["notes_alerted"] = len({r["midi"] for r in rows.values() if r["status"] in ("OFF", "WATCH")})
    return counts

def profile_reference_checks(targets):
    """Leave each real piano out before scoring it against the others.

    Local variability and sparse reference support make these alert rates
    a necessary context, rather than treating every OFF as a model fault.
    """
    checks = {}
    for nm, profiles in targets.items():
        rows = score_profiles(profiles, {n: p for n, p in targets.items() if n != nm})
        measured = {k: row for k, row in rows.items() if row["measured"]}
        checks[nm] = dict(profile_summary(measured), profiles=len(measured))
    return checks

def profile_report(rows, baseline=None, reference_checks=None):
    s = profile_summary(rows)
    lines = ["## Harmonic profiles: every key, three velocities", "",
             f"**{s['OFF']} OFF, {s['WATCH']} WATCH**, on {s['notes_alerted']} keys. "
             f"{s['insufficient']} profiles have insufficient usable comparisons; {s['missing']} profiles lack a recording.", "",
             "Each cell compares p2–p8 / p1 at the same key and layer against real target pianos. "
             "Channel powers are averaged after the FFT; there is no mono summing cancellation. "
             "OFF: any |z| > 2. WATCH: at least two partials in the same direction with |z| > 1 and ≥ 6 dB deviation. "
             "Tolerance: max(reference standard deviation, 4 dB), at least two references per partial. "
             "These are local spectral warnings, not a diagnosis of nasality; they do not enter the historical register score.", "",
             "Attack: 0–60 ms; sustain: 0.5–1 s. Velocities 45/56/100 use reference layers 4/8/13 "
             "(only BiVib has these exact calibrated MIDI velocities). "
             "Cells show status and the largest |z| component's deviation in dB. "
             "Insufficient data is never counted as a pass. Full values, z and reference counts: [harmonic_profiles.csv](harmonic_profiles.csv).", ""]
    if reference_checks:
        lines += ["Real pianos also trigger local alerts when compared with the others. "
                  "Below, each reference is excluded from its own targets; coverage differs between sources. "
                  "High counts here mean the alert thresholds are not selective enough to certify audible defects. "
                  "Use the local differences and baseline changes for diagnosis, not OFF counts alone.", "",
                  "| held-out real piano | comparable profiles | OFF | WATCH | insufficient |",
                  "|---|---|---|---|---|"]
        for nm, check in reference_checks.items():
            comparable = check["profiles"] - check["insufficient"]
            lines.append(f"| {nm} | {comparable} | {check['OFF']} | {check['WATCH']} | {check['insufficient']} |")
        lines.append("")
    if baseline is None:
        lines += ["Baseline: no harmonic profiles saved yet.", ""]
    else:
        alerts = {k for k, r in rows.items() if r["status"] in ("OFF", "WATCH")}
        previous = {k for k, r in baseline.items() if r["status"] in ("OFF", "WATCH")}
        comparable = {k for k in rows if rows[k]["status"] != "insufficient" and baseline[k]["status"] != "insufficient"}
        lines += [f"Against baseline: {len((alerts - previous) & comparable)} new alerted profiles, "
                  f"{len((previous - alerts) & comparable)} cleared (comparable profiles only).",
                  f"Baseline coverage: {len(comparable)}/{len(rows)} profiles comparable; "
                  f"{sum(not r['measured'] for r in baseline.values())} baseline recordings missing.", ""]
    lines += ["| note (MIDI) | 45 attack | 45 sustain | 56 attack | 56 sustain | 100 attack | 100 sustain |",
              "|---|---|---|---|---|---|---|"]
    for m in PROFILE_KEYS:
        cells = []
        for layer in LAYERS:
            for phase in PROFILE_WINDOWS:
                row = rows[f"{m}|{layer}|{phase}"]
                valid = [c for c in row["components"] if c["z"] is not None]
                cell = row["status"]
                if valid:
                    peak = max(valid, key=lambda c: abs(c["z"]))
                    cell += f" p{peak['partial']} {peak['delta_db']:+.1f}"
                cells.append(cell)
        lines.append(f"| {name(m)} ({m}) | " + " | ".join(cells) + " |")
    return lines + [""]

def write_profile_csv(rows, path):
    fields = ["note", "midi", "velocity", "layer", "phase", "status", "partial", "value_db", "target_db", "delta_db", "z", "reference_count", "references"]
    with open(path, "w", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=fields); writer.writeheader()
        for row in rows.values():
            for c in row["components"]:
                writer.writerow(dict({k: row[k] for k in fields[:6]}, partial=c["partial"],
                                     value_db=c["value"], target_db=c["target"], delta_db=c["delta_db"], z=c["z"],
                                     reference_count=len(c["refs"]), references=json.dumps(c["refs"], sort_keys=True)))

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
    # Key to key: the attack's 4–8 and 8–11 kHz (share of the first 60 ms,
    # mf) about their trend, rms and largest gap, every key from C3 to B4;
    # filed under the middle. The hammer's second impact one period after
    # the strike made D#4–F4 "fizz" (docs/bench.md).
    "hf_even":     ("attack", 3.0, ["4-8k rms", "8-11k rms", "4-8k max", "8-11k max"], False),
    "bands_att":   ("brightness", 3.0, ["1k", "2k", "4k", "8k"], False),
    "bands_sus":   ("brightness", 3.0, ["1k", "2k", "4k", "8k"], False),
    "sustain_ff":  ("sustain", 4.0, ["p2", "p3", "p4", "p6", "p8", "p12"], False),
    # Energy between the partials re on them, 300 Hz–5 kHz, ff, 0.08–0.6
    # and 0.6–1.5 s after the onset: the halo of what rings with the note,
    # which the ear hears as a round, rich sound (docs/realism_axes.md).
    "between":     ("resonance", 3.0, ["early", "late"], False),
    # The level of the lines the C6–C8 keys share; filed under the top.
    "lines":       ("resonance", 4.0, ["level"], False),
    "env_ff":      ("decay", 3.0, ["0.3s", "1s", "2s", "3s"], False),
    "env_mf":      ("decay", 3.0, ["0.3s", "1s", "2s", "3s"], False),
    "env_pp":      ("decay", 3.0, ["0.3s", "1s", "2s", "3s"], False),
    "t60_ff":      ("decay", 0.25, ["p1-4 early", "p5-12 early", "p1-4 late"], True),
    "inharm":      ("tuning", 0.15, ["log10 B"], False),
    "stretch":     ("tuning", 5.0, ["cents"], False),
}

def measure_set(d, survey_csv, even_dir=None, recorded=True):
    """Measures of a note set; `even_dir`: further keys, read by hf_even
    only (the model's harmonic_notes); `recorded`: real recordings, whose
    noise hf_even leaves out (False for the model's renders)."""
    cache = {}
    def get(m, l):
        if (m, l) not in cache:
            p = find(d, m, l); cache[(m, l)] = load(p, stereo=True) if p else None
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
                lv = {m: v for m, v in plev[k].items() if np.isfinite(v)}; span = [m for m in sorted(lv) if 36 <= m <= 96]
                res = [lv[m] - np.median([lv[j] for j in lv if abs(j - m) <= 6]) for m in span]
                ev.append(float(np.sqrt(np.mean(np.square(res)))) if len(res) >= 8 else None)
            r["evenness"] = ev if any(v is not None for v in ev) else None
            hl = {}
            for m in EVEN_KEYS:
                x = get(m, EVEN_LAYER)
                if x is None and even_dir and find(even_dir, m, EVEN_LAYER): x = load(find(even_dir, m, EVEN_LAYER), stereo=True)
                if x is not None: hl[m] = hf_even_share(x, gate=recorded)
            he = hf_even(hl)
            r["hf_even"] = he if any(v is not None for v in he) else None
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
        # The top's sustain between its partials is the recordings' noise
        # (up to 5 dB of it): attack only there.
        r["between"] = mean(lambda m, x: between_partials(x, m) if m < 96 else [between_partials(x, m, windows=((0.08, 0.6),))[0], np.nan], [13])
        if reg == "top":
            v = fixed_lines({m: get(m, 13) for m in LINE_KEYS if m in present and get(m, 13) is not None})
            r["lines"] = None if v is None else [v]
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
    run = subprocess.run([exe, d, "--out", csv, "--channel-power"], check=True, capture_output=True, text=True)
    with open(csv + ".log", "w") as f:
        f.write(run.stderr)

def build():
    subprocess.run(["cargo", "build", "--release", "-q", "-p", "piano-render", "-p", "piano-analysis"], cwd=ROOT, check=True)

def render(out, preset, listening=None, keys=None, hold=HOLD):
    exe = os.path.join(ROOT, "target", "release", "unsampled-piano-render")
    os.makedirs(out, exist_ok=True)
    jobs = [(m, l, v) for m in (KEYS if keys is None else keys) for l, v in LAYERS.items()]
    def one(j):
        m, l, v = j
        cmd = [exe, "note", "--note", str(m), "--velocity", str(v), "--hold", str(hold), "--tail", "0.5"]
        if preset: cmd += ["--preset", preset]
        cmd += listening_args(listening) + playback_args()
        subprocess.run(cmd + [os.path.join(out, f"{name(m)}v{l}.wav")], check=True, capture_output=True)
    with ThreadPoolExecutor(6) as ex: list(ex.map(one, jobs))

# A target needs this many real pianos: one alone has no spread, and the
# floor then judges the model against one recording's quirks (the top's
# 3 s envelope read on one source only). Except `dyn`: only BiVib was
# recorded at measured velocities.
MIN_REFS = 2
SINGLE_SOURCE = {"dyn"}

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
                if v[i] is None or len(rv) < (1 if meas in SINGLE_SOURCE else MIN_REFS): continue
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

def report(name_, z, info, base, path, profiles=None, profile_base=None, reference_checks=None):
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
    if profiles is not None:
        L += profile_report(profiles, profile_base, reference_checks)
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
    a = sub.add_parser("refs"); a.add_argument("sources", nargs="*")
    a.add_argument("--harmonics-only", action="store_true", help="refresh harmonic profiles of all registered targets, without redoing the historical measures")
    a.add_argument("--no-build", action="store_true", help="use an already rebuilt piano-survey")
    b = sub.add_parser("run")
    b.add_argument("--preset"); b.add_argument("--name", default="model")
    b.add_argument("--listening", action="append", help="listening preset(s) over the instrument (default: close microphones, studio; none: the engine's own)")
    b.add_argument("--out", default=os.path.join(ROOT, "out", "bench"))
    b.add_argument("--reuse", action="store_true", help="measure the notes already rendered")
    b.add_argument("--no-build", action="store_true")
    b.add_argument("--save-baseline", action="store_true")
    args = ap.parse_args()
    if args.cmd == "refs":
        if args.harmonics_only:
            if args.sources: ap.error("--harmonics-only uses registered sources; omit source arguments")
            refresh_profile_refs(json.load(open(REFS)))
            return
        if not args.sources: ap.error("refs requires sources, or --harmonics-only")
        try:
            profile_reference_cache(names={s.split("=", 1)[0] for s in args.sources})
        except ValueError as e:
            ap.error(str(e))
        if not args.no_build: build()
        old = json.load(open(REFS)) if os.path.exists(REFS) else {"sources": {}}
        work = os.path.join(ROOT, "out", "bench", "refs"); os.makedirs(work, exist_ok=True)
        for s in args.sources:
            nm, rest = s.split("=", 1); d, *opts = rest.split(":")
            if nm == "plath":
                manifest_path = os.path.join(d, "manifest.json")
                if not os.path.isfile(manifest_path) or json.load(open(manifest_path)).get("format_version") != 2:
                    ap.error("Plath requires a fixed-gain import: rerun tools/plath_set.py into a new directory")
            csv = os.path.join(work, f"{nm}.survey.csv")
            print(f"measuring {nm} ({d})", file=sys.stderr)
            # Cut to the model's length, so that decays are fitted on the
            # same span.
            cut = os.path.join(work, nm); os.makedirs(cut, exist_ok=True)
            for f in os.listdir(d):
                stem, ext = os.path.splitext(f)
                if ext in (".wav", ".flac"):
                    subprocess.run(["sox", os.path.join(d, f), "-e", "floating-point", "-b", "32", os.path.join(cut, stem + ".wav"), "trim", "0", str(HOLD + 0.5)], check=True)
            survey(cut, csv)
            prev = old["sources"].get(nm, {})
            metrics = measure_set(cut, csv)
            # Levels normalised per layer, or dynamics not tied to MIDI
            # velocities: no ff − pp.
            if "nodyn" in opts:
                for reg in metrics.values(): reg["dyn"] = None
            old["sources"][nm] = dict(role="info" if "info" in opts else "target", description=prev.get("description") or KNOWN.get(nm, ""),
                                      dir=os.path.relpath(os.path.abspath(d), ROOT), options=opts, metrics=metrics,
                                      analysis_version=ANALYSIS_VERSION)
            manifest = os.path.join(d, "manifest.json")
            if os.path.isfile(manifest):
                old["sources"][nm]["import_manifest"] = json.load(open(manifest))
        os.makedirs(os.path.dirname(REFS), exist_ok=True)
        json.dump(old, open(REFS, "w"), indent=1)
        refresh_profile_refs(old, names={s.split("=", 1)[0] for s in args.sources})
        print(f"wrote {os.path.relpath(REFS, ROOT)}", file=sys.stderr)
        return
    refs = json.load(open(REFS))
    outdated = [n for n, s in refs["sources"].items() if s.get("analysis_version") != ANALYSIS_VERSION]
    if outdated:
        ap.error("recompute references with this analysis version first: " + ", ".join(outdated))
    try:
        profile_targets = load_profile_refs(refs)
    except ValueError as e:
        ap.error(str(e))
    out = os.path.join(args.out, args.name); notes = os.path.join(out, "notes")
    harmonic_notes = os.path.join(out, "harmonic_notes")
    t = time.time()
    if not args.reuse:
        if not args.no_build: build()
        render(notes, args.preset, args.listening)
        render(harmonic_notes, args.preset, args.listening,
               keys=[m for m in PROFILE_KEYS if m not in KEYS], hold=1.5)
    csv = os.path.join(out, "survey.csv"); survey(notes, csv)
    metrics = measure_set(notes, csv, harmonic_notes, recorded=False)
    z = score(metrics, refs)
    info = {n: score(s["metrics"], refs) for n, s in refs["sources"].items() if s["role"] == "info"}
    base = json.load(open(BASELINE)) if os.path.exists(BASELINE) else None
    # The baseline keeps its measures, scored against today's references.
    if base and base.get("analysis_version") != ANALYSIS_VERSION:
        print("baseline uses an older analysis version; comparison skipped (remeasure before saving a new baseline)", file=sys.stderr)
        base = None
    if base: base["z"] = score(base["metrics"], refs)
    profiles = measure_profiles([notes, harmonic_notes])
    profile_scores = score_profiles(profiles, profile_targets)
    if args.save_baseline and profile_summary(profile_scores)["missing"]:
        ap.error("Cannot save baseline: harmonic recordings are missing. "
                 "Render all 88 keys (run without --reuse) before --save-baseline.")
    reference_checks = profile_reference_checks(profile_targets)
    profile_base = (score_profiles(base["harmonic_profiles"], profile_targets)
                    if base and base.get("harmonic_spec") == PROFILE_SPEC else None)
    write_profile_csv(profile_scores, os.path.join(out, "harmonic_profiles.csv"))
    path = os.path.join(out, "scorecard.md")
    s = report(args.name, z, info, base, path, profile_scores, profile_base, reference_checks)
    json.dump(dict(analysis_version=ANALYSIS_VERSION, commit=git_commit(), preset=args.preset, listening=args.listening, metrics=metrics, z=z,
                   harmonic_spec=PROFILE_SPEC, harmonic_profiles=profiles, harmonic_scores=profile_scores,
                   harmonic_summary=profile_summary(profile_scores), harmonic_reference_checks=reference_checks),
              open(os.path.join(out, "result.json"), "w"), indent=1)
    print(f"score {s['score']:.2f}, {100 * s['within']:.0f} % within the real pianos, {s['off']} off — "
          + ", ".join(f"{f} {v:.2f}" for f, v in s["families"].items()))
    hs = profile_summary(profile_scores)
    print(f"harmonic profiles: {hs['OFF']} OFF, {hs['WATCH']} WATCH on {hs['notes_alerted']} keys; "
          f"{hs['insufficient']} insufficient, {hs['missing']} missing")
    if base:
        b = summary(base["z"]); print(f"baseline {base.get('commit', '?')}: score {b['score']:.2f}, {b['off']} off")
    print(f"{os.path.relpath(path, ROOT)} ({time.time() - t:.0f} s)")
    if args.save_baseline:
        json.dump(dict(analysis_version=ANALYSIS_VERSION, commit=git_commit(), preset=args.preset, listening=args.listening, metrics=metrics,
                       harmonic_spec=PROFILE_SPEC, harmonic_profiles=profiles), open(BASELINE, "w"), indent=1)
        print(f"wrote {os.path.relpath(BASELINE, ROOT)}")

if __name__ == "__main__":
    main()
