#!/usr/bin/env python3
"""Ear pressure per bridge acceleration, real (plath_tf.json) against the
model (bridge_to_ear output), per third octave: median and the spread
(p75 − p25) of the samples, and the model minus the real median.
  tf_compare.py plath_tf.json model.txt [model2.txt …]"""
import os, json, sys
import numpy as np

def bins(f, v):
    out = {}
    c = 31.25
    while c < 9000:
        sel = (f >= c / 2 ** (1 / 6)) & (f < c * 2 ** (1 / 6))
        if sel.sum() >= 3:
            out[round(c)] = (np.median(v[sel]), np.percentile(v[sel], 75) - np.percentile(v[sel], 25), int(sel.sum()))
        c *= 2 ** (1 / 3)
    return out

def main():
    real = np.array(json.load(open(sys.argv[1])))
    rb = bins(real[:, 2], real[:, 3])
    models = []
    for p in sys.argv[2:]:
        m = np.array([[float(x) for x in l.split()] for l in open(p) if l.strip()])
        models.append((p, bins(m[:, 2], m[:, 3])))
    print("third oct | real median  spread | " + " | ".join(f"{p.split('/')[-1][:18]:>18} spread   diff" for p, _ in models))
    for c in sorted(rb):
        row = f"{c:8} | {rb[c][0]:8.1f} {rb[c][1]:7.1f} |"
        for _, mb in models:
            if c in mb: row += f" {mb[c][0]:11.1f} {mb[c][1]:7.1f} {mb[c][0] - rb[c][0]:+6.1f} |"
            else: row += " " * 27 + "|"
        print(row)
    # the overall level, 200 Hz–4 kHz, and the shape about it
    sel = lambda b: [b[c][0] for c in sorted(b) if 200 <= c <= 4000]
    r0 = np.mean(sel(rb))
    for p, mb in models:
        m0 = np.mean(sel(mb)); d = [mb[c][0] - rb[c][0] - (m0 - r0) for c in sorted(rb) if c in mb and 200 <= c <= 4000]
        print(f"{p.split('/')[-1]}: level 200 Hz–4 kHz {m0 - r0:+.1f} dB re real; shape rms about it {np.sqrt(np.mean(np.square(d))):.1f} dB")

if __name__ == "__main__":
    main()
