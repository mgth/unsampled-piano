#!/usr/bin/env python3
"""Lays out Plath's concert-grand measurements (anechoic chamber, every key,
5 takes at one dynamic, Zenodo 3274772) as a bench note set: the dummy
head's two ears, as <note>v13.wav at 50 kHz, from the median take by peak
pressure. PROD07: the new piano; PROD08: the same after a year on stage.
  tools/plath_set.py <data dir> <out dir> [--state PROD08]"""
import argparse, glob, os, re, subprocess
import numpy as np

NAMES = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"]
def name(m): return NAMES[m % 12] + str(m // 12 - 1)

def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("data"); ap.add_argument("out"); ap.add_argument("--state", default="PROD08")
    a = ap.parse_args()
    os.makedirs(a.out, exist_ok=True)
    for key in range(1, 89):
        takes = sorted(glob.glob(os.path.join(a.data, f"D1_{a.state}_{key:02d}_*M*.csv")))
        if not takes: continue
        ears = [np.loadtxt(f, delimiter=";", skiprows=1, usecols=(3, 4), dtype=np.float32) for f in takes]
        peaks = [np.abs(e).max() for e in ears]
        e = ears[int(np.argsort(peaks)[len(peaks) // 2])]
        # The recording starts 0.29 s before the key's impact: kept, so the
        # onset is found as in any other set.
        e = e / max(np.abs(e).max(), 1e-9) * 0.5
        dst = os.path.join(a.out, f"{name(20 + key)}v13.wav")
        subprocess.run(["sox", "-t", "f32", "-r", "50000", "-c", "2", "-", "-b", "24", dst], input=e.tobytes(), check=True)
    print(len(os.listdir(a.out)), "files")

if __name__ == "__main__":
    main()
