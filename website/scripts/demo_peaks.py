#!/usr/bin/env python3
"""Adds a clip's waveform outline (peak per bar, 0–1, and its duration) to the
JSON the players draw from.
usage: python3 -I scripts/demo_peaks.py <16-bit wav> <name> <json>"""
import array, json, sys, wave
from pathlib import Path

BARS = 160
wav, name, out = sys.argv[1], sys.argv[2], Path(sys.argv[3])
with wave.open(wav) as w:
    ch, sr, n = w.getnchannels(), w.getframerate(), w.getnframes()
    s = array.array("h", w.readframes(n))
step = len(s) // ch // BARS
peaks = [max(abs(x) for x in s[i * step * ch:(i + 1) * step * ch]) / 32768 for i in range(BARS)]
data = json.loads(out.read_text()) if out.exists() else {}
data[name] = {"duration": round(n / sr, 2), "peaks": [round(p, 3) for p in peaks]}
out.write_text("{\n" + ",\n".join(f" {json.dumps(k)}: {json.dumps(data[k])}" for k in sorted(data)) + "\n}\n")
