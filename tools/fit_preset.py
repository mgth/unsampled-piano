#!/usr/bin/env python3
"""Fits the engine's inharmonicity and stretch-tuning laws on a piano-survey
CSV of a recorded instrument, and prints them as preset sections.
  tools/fit_preset.py <survey.csv> [a4]
B(m) = e^{a_bass·m + b_bass} + e^{a_treble·m + b_treble} (least squares on
log B); stretch(m) = a_t·e^{k_t(m−69)} − a_b·e^{−k_b(m−69)} + c (cents re
equal temperament at a4, least squares, robust)."""
import sys
import numpy as np, pandas as pd
from scipy.optimize import least_squares
d = pd.read_csv(sys.argv[1])
a4 = float(sys.argv[2]) if len(sys.argv) > 2 else 440.0
d = d.sort_values("layer", ascending=False).drop_duplicates("midi").sort_values("midi")
d = d[np.isfinite(d.B) & (d.B > 0)]
m, B = d.midi.values.astype(float), d.B.values
def bl(p, m): return np.exp(p[0] * m + p[1]) + np.exp(p[2] * m + p[3])
r = least_squares(lambda p: np.log(bl(p, m)) - np.log(B), [-0.1117, -6.107, 0.0886, -13.495], loss="soft_l1", f_scale=0.3)
pb = r.x
f0 = d.f0.values; cents = 1200 * np.log2(f0 / (a4 * 2 ** ((m - 69) / 12)))
# The topmost keys' pitch is poorly defined (few, short partials): left out,
# as for the default law.
keep = (m <= 105) & (np.abs(cents) < 60)
m_s, cents = m[keep], cents[keep]
def st(p, m):
    x = m - 69
    return p[0] * np.exp(p[1] * x) - p[2] * np.exp(-p[3] * x) + p[4]
s = least_squares(lambda p: st(p, m_s) - cents, [4.22, 0.049, 0.0195, 0.135, -3.7], loss="soft_l1", f_scale=2.0,
                  bounds=([0, 0.005, 0, 0.01, -50], [50, 0.1, 50, 0.4, 50]))
ps = s.x
print(f"# fitted on {sys.argv[1]}: {len(m)} keys ({len(m_s)} for the stretch), log B rms {np.sqrt(np.mean(r.fun ** 2)):.2f}, stretch rms {np.sqrt(np.mean(s.fun ** 2)):.1f} cents")
print(f"# B: A0 {bl(pb, 21):.1e}, C2 {bl(pb, 36):.1e}, C4 {bl(pb, 60):.1e}, C6 {bl(pb, 84):.1e}, C8 {bl(pb, 108):.1e}")
print(f"# stretch: A0 {st(ps, 21):+.1f}, C2 {st(ps, 36):+.1f}, C6 {st(ps, 84):+.1f}, C8 {st(ps, 108):+.1f} cents")
print(f"[inharmonicity]\na_bass = {pb[0]:.6f}\nb_bass = {pb[1]:.6f}\na_treble = {pb[2]:.6f}\nb_treble = {pb[3]:.6f}")
print(f"[stretch]\na_t = {ps[0]:.6f}\nk_t = {ps[1]:.6f}\na_b = {ps[2]:.6f}\nk_b = {ps[3]:.6f}\nc = {ps[4]:.6f}")
