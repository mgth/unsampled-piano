#!/usr/bin/env python3
"""Lays out Plath's concert-grand measurements (anechoic chamber, every key,
5 takes at one dynamic, Zenodo 3274772) as a bench note set: the dummy
head's two ears, as <note>v13.wav at 50 kHz, from the median take by peak
pressure. One fixed conversion (0.01 full-scale units/Pa by default)
preserves the levels between keys. Float WAVs and manifest.json retain
calibration and take provenance. PROD07: the new piano; PROD08: the same after a year on stage.
  tools/plath_set.py <data dir> <out dir> [--state PROD08]"""
import argparse, glob, json, os, subprocess
import numpy as np

NAMES = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"]
def name(m): return NAMES[m % 12] + str(m // 12 - 1)

def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("data"); ap.add_argument("out"); ap.add_argument("--state", default="PROD08")
    ap.add_argument("--gain-per-pa", type=float, default=0.01,
                    help="one fixed digital gain per pascal for every note (default: 0.01)")
    a = ap.parse_args()
    if not np.isfinite(a.gain_per_pa) or a.gain_per_pa <= 0:
        ap.error("--gain-per-pa must be finite and positive")
    manifest = dict(format_version=2, state=a.state, gain_per_pa=a.gain_per_pa,
                    sample_rate=50000, source=os.path.abspath(a.data), notes=[])
    os.makedirs(a.out, exist_ok=True)
    for key in range(1, 89):
        takes = sorted(glob.glob(os.path.join(a.data, f"D1_{a.state}_{key:02d}_*M*.csv")))
        if not takes: continue
        ears = [np.loadtxt(f, delimiter=";", skiprows=1, usecols=(3, 4), dtype=np.float32) for f in takes]
        peaks = [np.abs(e).max() for e in ears]
        selected = int(np.argsort(peaks, kind="stable")[len(peaks) // 2])
        e = ears[selected]
        if not np.isfinite(e).all() or np.abs(e).max() * a.gain_per_pa >= 1:
            raise ValueError(f"{takes[selected]}: invalid or clipping pressure; choose a smaller common --gain-per-pa")
        # The recording starts 0.29 s before the key's impact: kept, so the
        # onset is found as in any other set.
        e = e * a.gain_per_pa
        dst = os.path.join(a.out, f"{name(20 + key)}v13.wav")
        subprocess.run(["sox", "-t", "f32", "-r", "50000", "-c", "2", "-", "-e", "floating-point", "-b", "32", dst], input=e.tobytes(), check=True)
        manifest["notes"].append(dict(midi=20 + key, file=os.path.basename(dst),
                                      selected_take=os.path.basename(takes[selected]),
                                      takes=len(takes), peak_pa=float(peaks[selected])))
    with open(os.path.join(a.out, "manifest.json"), "w") as f:
        json.dump(manifest, f, indent=2, allow_nan=False)
        f.write("\n")
    print(len(manifest["notes"]), "files; fixed gain", a.gain_per_pa, "/Pa")

if __name__ == "__main__":
    main()
