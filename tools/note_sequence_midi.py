#!/usr/bin/env python3
"""Writes a MIDI file playing every third key from A0 (the Salamander set),
held `hold` s, one per `slot` s, at each given velocity in turn; prints the
note list (index, key, velocity, start s) for cutting the recording.
  tools/note_sequence_midi.py <out.mid> <slot> <hold> <vel> [<vel> …]"""
import struct, sys
out, slot, hold = sys.argv[1], float(sys.argv[2]), float(sys.argv[3])
vels = [int(v) for v in sys.argv[4:]]
TPQ, TEMPO = 1000, 1_000_000  # 1 tick = 1 ms
evs = []
t = 1.0
i = 0
for v in vels:
    for key in range(21, 109, 3):
        evs.append((t, 0x90, key, v)); evs.append((t + hold, 0x80, key, 64))
        print(i, key, v, f"{t:.3f}")
        t += slot; i += 1
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
open(out, "wb").write(b"MThd" + struct.pack(">IHHH", 6, 0, 1, TPQ) + b"MTrk" + struct.pack(">I", len(trk)) + trk)
