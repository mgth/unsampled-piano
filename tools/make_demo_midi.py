#!/usr/bin/env python3
"""Writes small MIDI files demonstrating dampers, pedals and sympathy.
usage: python3 -I tools/make_demo_midi.py out/demos"""
import struct, sys
from pathlib import Path

TPB = 480  # ticks per beat at 120 bpm: 1 s = 960 ticks


def vlq(n):
    out = [n & 0x7F]
    n >>= 7
    while n:
        out.insert(0, (n & 0x7F) | 0x80)
        n >>= 7
    return bytes(out)


def write(path, events):
    """events: list of (seconds, bytes)."""
    events = sorted(events, key=lambda e: e[0])
    data, last = b"", 0
    for t, msg in events:
        tick = int(round(t * 2 * TPB))
        data += vlq(tick - last) + msg
        last = tick
    data += vlq(0) + b"\xff\x2f\x00"
    hdr = b"MThd" + struct.pack(">IHHH", 6, 0, 1, TPB)
    Path(path).write_bytes(hdr + b"MTrk" + struct.pack(">I", len(data)) + data)


on = lambda n, v=90: bytes([0x90, n, v])
off = lambda n: bytes([0x80, n, 64])
cc = lambda c, v: bytes([0xB0, c, v])

out = Path(sys.argv[1])
out.mkdir(parents=True, exist_ok=True)

# 1. Same staccato chord without, then with the sustain pedal: the pedal lets
#    every string resonate through the bridge (sympathetic halo).
ev = []
for t0, pedal in ((0.5, False), (5.0, True)):
    if pedal:
        ev.append((t0 - 0.3, cc(64, 127)))
    for n in (48, 52, 55):
        ev += [(t0, on(n, 100)), (t0 + 0.25, off(n))]
    if pedal:
        ev.append((t0 + 3.5, cc(64, 0)))
write(out / "pedal_halo.mid", ev)

# 2. Release with the pedal up, half way, and down.
ev = []
for i, p in enumerate((0, 40, 127)):
    t0 = 0.3 + 2.5 * i
    ev += [(t0 - 0.1, cc(64, p)), (t0, on(60, 100)), (t0 + 0.4, off(60)), (t0 + 2.3, cc(64, 0))]
write(out / "half_pedal.mid", ev)

# 3. A chord normal, then with una corda.
ev = []
for t0, soft in ((0.3, 0), (3.3, 127)):
    ev.append((t0 - 0.1, cc(67, soft)))
    for n in (60, 64, 67):
        ev += [(t0, on(n, 80)), (t0 + 2.5, off(n))]
write(out / "una_corda.mid", ev + [(6.0, cc(67, 0))])

# 4. Sostenuto: C2 held when the pedal goes down keeps ringing under a
#    staccato melody that stays damped.
ev = [(0.2, on(36, 100)), (0.5, cc(66, 127)), (0.8, off(36))]
for i, n in enumerate((72, 74, 76, 77, 79, 77, 76, 74, 72)):
    t = 1.0 + 0.35 * i
    ev += [(t, on(n, 80)), (t + 0.15, off(n))]
ev.append((5.0, cc(66, 0)))
write(out / "sostenuto.mid", ev)
print("written to", out)
