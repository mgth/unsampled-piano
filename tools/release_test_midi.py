#!/usr/bin/env python3
"""Writes a MIDI file for listening to what rings after the keys are
released, without pedal: staccato runs and held chords in the bass, middle
and treble, each followed by 2.5 s of silence; prints the segments
(name, start s, release s).  tools/release_test_midi.py <out.mid>"""
import struct, sys
TPQ, TEMPO = 1000, 1_000_000  # 1 tick = 1 ms
evs, segs = [], []
t = 1.0
def note(t0, dur, key, vel):
    evs.append((t0, 0x90, key, vel)); evs.append((t0 + dur, 0x80, key, 64))
for name, keys in (("run_mid", [60, 64, 67, 72, 76, 79, 84]), ("run_bass", [36, 40, 43, 48, 52, 55]), ("run_treble", [79, 84, 88, 91, 96])):
    for vel in (60, 100):
        start = t
        for i, k in enumerate(keys):
            note(t + 0.18 * i, 0.15, k, vel)
        rel = t + 0.18 * (len(keys) - 1) + 0.15
        segs.append((f"{name}_v{vel}", start, rel)); t = rel + 2.5
for name, keys in (("chord_mid", [60, 64, 67, 72]), ("chord_bass", [36, 43, 48, 52]), ("chord_treble", [72, 76, 79, 84])):
    for vel in (60, 100):
        for k in keys:
            note(t, 1.0, k, vel)
        segs.append((f"{name}_v{vel}", t, t + 1.0)); t += 3.5
for s in segs:
    print(s[0], f"{s[1]:.3f}", f"{s[2]:.3f}")
evs.sort(key=lambda e: (e[0], e[1]))
def vlq(n):
    b = [n & 0x7f]
    while n > 127:
        n >>= 7; b.insert(0, (n & 0x7f) | 0x80)
    return bytes(b)
trk = bytearray(b"\x00\xff\x51\x03" + TEMPO.to_bytes(3, "big"))
last = 0
for tm, st, k, v in evs:
    tick = round(tm * 1000)
    trk += vlq(tick - last) + bytes([st, k, v]); last = tick
trk += vlq(1000) + b"\xff\x2f\x00"
open(sys.argv[1], "wb").write(b"MThd" + struct.pack(">IHHH", 6, 0, 1, TPQ) + b"MTrk" + struct.pack(">I", len(trk)) + trk)
