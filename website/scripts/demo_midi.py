#!/usr/bin/env python3
"""Writes the MIDI files the website's audio demos are rendered from: the
opening of Bach's Prelude in C (BWV 846, public domain, written here note by
note), a staccato chord without then with the sustain pedal, and one chord
from pianissimo to fortissimo. No recorded performance is used.
usage: python3 -I scripts/demo_midi.py <out dir>"""
import random, struct, sys
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
    """events: list of (seconds, bytes); note-offs sort before note-ons."""
    events = sorted(events, key=lambda e: (e[0], e[1][0] & 0xF0 == 0x90))
    data, last = b"", 0
    for t, msg in events:
        tick = int(round(max(t, 0.0) * 2 * TPB))
        data += vlq(tick - last) + msg
        last = tick
    data += vlq(0) + b"\xff\x2f\x00"
    hdr = b"MThd" + struct.pack(">IHHH", 6, 0, 1, TPB)
    Path(path).write_bytes(hdr + b"MTrk" + struct.pack(">I", len(data)) + data)


on = lambda n, v: bytes([0x90, n, max(1, min(127, int(round(v))))])
off = lambda n: bytes([0x80, n, 64])
cc = lambda c, v: bytes([0xB0, c, v])

out = Path(sys.argv[1])
out.mkdir(parents=True, exist_ok=True)
rng = random.Random(846)

# 1. Prelude in C, bars 1–11 and a final C major chord. Each half bar: the
#    lowest note held, the second one held from the next sixteenth, the upper
#    three broken twice. The pedal changes just after each new harmony.
BARS = [
    (48, 52, 55, 60, 64), (48, 50, 57, 62, 65), (47, 50, 55, 62, 65),
    (48, 52, 55, 60, 64), (48, 52, 57, 64, 69), (48, 50, 54, 57, 62),
    (47, 50, 55, 62, 67), (47, 48, 52, 55, 60), (45, 48, 52, 55, 60),
    (38, 45, 50, 54, 60), (43, 47, 50, 55, 59),
]
SIXTEENTH = 60 / 66 / 4  # crotchet = 66
# A gentle swell to bar 5, a dip, and a rise to the cadence (velocity base).
SHAPE = [48, 52, 55, 58, 62, 58, 54, 50, 52, 58, 62]
ev, t = [], 0.4
for bar, (chord, base) in enumerate(zip(BARS, SHAPE)):
    ev += [(t - 0.02, cc(64, 0)), (t + 0.08, cc(64, 110))]
    # Slight broadening of the last bar before the final chord.
    step = SIXTEENTH * (1.0 + 0.12 * (bar == len(BARS) - 1))
    for half in range(2):
        h = t + half * 8 * step
        jit = lambda: rng.uniform(-0.008, 0.008)
        ev += [(h + jit(), on(chord[0], base + 2 + rng.uniform(-3, 3))),
               (h + 8 * step - 0.03, off(chord[0]))]
        ev += [(h + step + jit(), on(chord[1], base - 4 + rng.uniform(-3, 3))),
               (h + 8 * step - 0.03, off(chord[1]))]
        for i, k in enumerate((2, 3, 4, 2, 3, 4)):
            s = h + (2 + i) * step + jit()
            # The top of each arpeggio leans a little.
            v = base - 6 + 5 * (k == 4) + rng.uniform(-4, 4)
            ev += [(s, on(chord[k], v)), (s + step * 0.95, off(chord[k]))]
    t += 16 * step
# Final chord, slightly rolled from the bass, held under the pedal.
ev += [(t - 0.02, cc(64, 0)), (t + 0.08, cc(64, 110))]
for i, n in enumerate((36, 48, 52, 55, 60, 64)):
    ev += [(t + 0.035 * i, on(n, 56 - 2 * i)), (t + 3.5, off(n))]
ev.append((t + 4.2, cc(64, 0)))
write(out / "prelude.mid", ev)

# 2. The same staccato chord without, then with the sustain pedal: with the
#    dampers lifted every string answers through the bridge.
ev = []
for t0, pedal in ((0.4, False), (4.4, True)):
    if pedal:
        ev.append((t0 - 0.3, cc(64, 127)))
    for n in (36, 48, 52, 55, 60):
        ev += [(t0, on(n, 100)), (t0 + 0.2, off(n))]
    if pedal:
        ev.append((t0 + 5.0, cc(64, 0)))
write(out / "pedal.mid", ev)

# 3. One chord at four dynamics, from pianissimo to fortissimo.
ev = []
for i, v in enumerate((22, 50, 85, 122)):
    t0 = 0.3 + 2.6 * i
    for n in (43, 55, 59, 62, 67):
        ev += [(t0, on(n, v)), (t0 + 2.2, off(n))]
write(out / "dynamics.mid", ev)
print("written to", out)
