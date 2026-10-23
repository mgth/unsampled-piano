#!/usr/bin/env python3
"""Writes the MIDI files the website's audio demos are rendered from: the
openings of Debussy's Clair de lune and Chopin's "Revolutionary" Étude, cut
from Bernd Krüger's MIDI files (midi/, piano-midi.de, CC BY-SA 3.0 DE), a
staccato chord without then with the sustain pedal, and one chord from
pianissimo to fortissimo.
usage: python3 -I scripts/demo_midi.py <out dir>"""
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


def read(path):
    """A MIDI file's note and controller messages as (seconds, bytes), all
    tracks merged, through the file's tempo map."""
    data = Path(path).read_bytes()
    ntrk, tpq = struct.unpack(">HH", data[10:14])
    pos, raw = 14, []
    for _ in range(ntrk):
        size = struct.unpack(">I", data[pos + 4:pos + 8])[0]
        p, end, tick, status = pos + 8, pos + 8 + size, 0, 0
        while p < end:
            dt = 0
            while True:
                dt = (dt << 7) | (data[p] & 0x7F)
                p += 1
                if not data[p - 1] & 0x80:
                    break
            tick += dt
            if data[p] == 0xFF:  # meta: only the tempo matters
                kind, n = data[p + 1], data[p + 2]
                if kind == 0x51:
                    raw.append((tick, 0, int.from_bytes(data[p + 3:p + 6], "big")))
                p += 3 + n
                continue
            if data[p] in (0xF0, 0xF7):
                p += 2 + data[p + 1]
                continue
            if data[p] & 0x80:
                status, p = data[p], p + 1
            n = 1 if status & 0xF0 in (0xC0, 0xD0) else 2
            if status & 0xF0 in (0x80, 0x90, 0xB0):
                raw.append((tick, 1, bytes([status & 0xF0]) + data[p:p + n]))
            p += n
        pos = end
    raw.sort(key=lambda e: (e[0], e[1]))
    out, last, sec, tempo = [], 0, 0.0, 500000
    for tick, kind, val in raw:
        sec += (tick - last) * tempo / tpq / 1e6
        last = tick
        if kind == 0:
            tempo = val
        else:
            if val[0] == 0x90 and val[2] == 0:
                val = off(val[1])
            out.append((sec, val))
    return out


def excerpt(events, cut, ring):
    """Everything before `cut` seconds; the notes and the pedal still down
    then are let go `ring` seconds later."""
    held, pedal, out = set(), 0, []
    for t, msg in events:
        if t >= cut:
            break
        out.append((t, msg))
        if msg[0] == 0x90:
            held.add(msg[1])
        elif msg[0] == 0x80:
            held.discard(msg[1])
        elif msg[1] == 64:
            pedal = msg[2]
    out += [(cut + ring, off(n)) for n in sorted(held)]
    if pedal:
        out.append((cut + ring + 0.5, cc(64, 0)))
    return out


# 1. Clair de lune, bars 1–14: the first section, up to the "tempo rubato",
#    its last chord left to ring under the pedal.
here = Path(__file__).parent
write(out / "clair.mid", excerpt(read(here / "midi" / "deb_clai.mid"), 54.2, 2.5))

# Étude op. 10 no. 12, bars 1–28 and the chord on the first beat of bar 29:
# the left hand's runs over the whole keyboard, up to fortissimo.
write(out / "etude.mid", excerpt(read(here / "midi" / "chpn_op10_e12.mid"), 46.2, 2.0))

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
