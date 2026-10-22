#!/usr/bin/env python3
"""Cuts a recording of tools/note_sequence_midi.py into Salamander-style
files <note>v<layer>.wav (velocities 40/60/100 → layers 4/8/13), aligning on
the first onset; or links/converts a sample set into the same layout.
  tools/split_notes.py seq <recording.wav> <notes.txt> <out dir> [slot]
  tools/split_notes.py iowa <iowa dir> <out dir>
  tools/split_notes.py salamander <samples dir> <out dir>"""
import os, subprocess, sys
import numpy as np
NAMES = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"]
LAYER = {40: 4, 60: 8, 100: 13}
def name(m): return NAMES[m % 12] + str(m // 12 - 1)
def load(path):
    raw = subprocess.run(["sox", path, "-t", "f32", "-r", "48000", "-"], check=True, capture_output=True).stdout
    ch = int(subprocess.run(["soxi", "-c", path], capture_output=True, text=True).stdout.strip() or 1)
    return np.frombuffer(raw, dtype=np.float32).reshape(-1, ch)
def save(path, x):
    subprocess.run(["sox", "-t", "f32", "-r", "48000", "-c", str(x.shape[1]), "-", "-b", "24", path], input=x.astype(np.float32).tobytes(), check=True)
mode = sys.argv[1]
if mode == "seq":
    rec, notes, out = sys.argv[2], sys.argv[3], sys.argv[4]
    slot = float(sys.argv[5]) if len(sys.argv) > 5 else 5.5
    os.makedirs(out, exist_ok=True)
    x = load(rec); m = np.abs(x).max(1)
    rows = [l.split() for l in open(notes)]
    first = float(rows[0][3])
    on = int(np.argmax(m > 0.05 * m.max() * 0.1))  # first onset (A0 pp)
    # refine: first sample above 20 dB below the first note's own peak
    seg = m[on: on + 48000]; on += int(np.argmax(seg > 0.01 * seg.max())) - 240  # 5 ms before
    for i, k, v, t in rows:
        s = on + int((float(t) - first) * 48000)
        save(os.path.join(out, f"{name(int(k))}v{LAYER[int(v)]}.wav"), x[s: s + int(slot * 48000) - 2400])
elif mode == "iowa":
    src, out = sys.argv[2], sys.argv[3]; os.makedirs(out, exist_ok=True)
    flats = {"Db": "C#", "Eb": "D#", "Gb": "F#", "Ab": "G#", "Bb": "A#"}
    for f in sorted(os.listdir(src)):
        _, dyn, note, _ = f.split(".")
        n = flats.get(note[:2], note[:-1]) + note[-1] if note[:2] in flats else note
        lay = {"pp": 4, "mf": 8, "ff": 13}[dyn]
        x = load(os.path.join(src, f)); m = np.abs(x).max(1)
        s = max(int(np.argmax(m > 0.1 * m.max())) - 96, 0)  # trim leading silence
        save(os.path.join(out, f"{n}v{lay}.wav"), x[s:])
elif mode == "salamander":
    src, out = sys.argv[2], sys.argv[3]; os.makedirs(out, exist_ok=True)
    for f in os.listdir(src):
        if any(f.endswith(f"v{l}.flac") for l in (4, 8, 13)) and not f.startswith(("harm", "rel", "pedal")):
            p = os.path.join(out, f)
            if not os.path.exists(p): os.symlink(os.path.abspath(os.path.join(src, f)), p)
