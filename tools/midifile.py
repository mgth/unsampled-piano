"""Reads a standard MIDI file into notes and sustain-pedal changes, in seconds
(all tracks and channels merged, tempo map applied).
  notes, pedal = read(path)
  notes: [(key, velocity, on_s, off_s)], pedal: [(time_s, value)]"""
import struct

def _vlq(b, i):
    n = 0
    while True:
        c = b[i]; i += 1; n = (n << 7) | (c & 0x7F)
        if not c & 0x80: return n, i

def read(path):
    b = open(path, "rb").read()
    if b[:4] != b"MThd": raise ValueError(f"{path}: not a MIDI file")
    hl, fmt, ntrk, div = struct.unpack(">IHHH", b[4:14])
    if div & 0x8000: raise ValueError(f"{path}: SMPTE time division not handled")
    i = 8 + hl
    events, tempos = [], [(0, 500000)]
    for _ in range(ntrk):
        if b[i:i + 4] != b"MTrk": break
        ln = struct.unpack(">I", b[i + 4:i + 8])[0]; j, end = i + 8, i + 8 + ln
        tick, status = 0, 0
        while j < end:
            d, j = _vlq(b, j); tick += d
            c = b[j]
            if c == 0xFF:
                t, ln2 = b[j + 1], 0
                ln2, k = _vlq(b, j + 2)
                if t == 0x51: tempos.append((tick, int.from_bytes(b[k:k + 3], "big")))
                j = k + ln2; continue
            if c in (0xF0, 0xF7):
                ln2, k = _vlq(b, j + 1); j = k + ln2; continue
            if c & 0x80: status = c; j += 1
            kind = status & 0xF0
            if kind in (0xC0, 0xD0): data = (b[j], 0); j += 1
            else: data = (b[j], b[j + 1]); j += 2
            events.append((tick, kind, data[0], data[1]))
        i = end
    tempos.sort()
    def seconds(tick):
        t, last_tick, us = 0.0, 0, 500000
        for tt, tempo in tempos:
            if tt >= tick: break
            t += (tt - last_tick) * us / div / 1e6; last_tick, us = tt, tempo
        return t + (tick - last_tick) * us / div / 1e6
    events.sort(key=lambda e: e[0])
    notes, pedal, held = [], [], {}
    for tick, kind, a, v in events:
        t = seconds(tick)
        if kind == 0x90 and v > 0:
            if a in held: notes.append((a, held[a][1], held[a][0], t))
            held[a] = (t, v)
        elif kind == 0x80 or (kind == 0x90 and v == 0):
            if a in held:
                on, vel = held.pop(a); notes.append((a, vel, on, t))
        elif kind == 0xB0 and a == 64:
            pedal.append((t, v))
    end = max([n[3] for n in notes] + [0.0])
    for a, (on, vel) in held.items(): notes.append((a, vel, on, end))
    notes.sort(key=lambda n: n[2])
    return notes, pedal
