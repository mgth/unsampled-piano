#!/usr/bin/env python3
"""Bit-exactness scenarios: writes a fixed set of Standard MIDI Files.

Every file is format 0, one track, 500 ticks per quarter note at 120 bpm
(500 000 us per beat), so one tick is exactly one millisecond and every
event lands on a whole number of samples at 48 kHz (48 frames per tick).

The output is deterministic: same script, same inputs => same bytes. The
only external inputs are the optional real performances used for the dense
excerpts (DATA_MIDI below); when they are absent a pseudo-random dense piece
with a fixed seed is written instead, and the manifest records the MIDI
hashes so that two sets made from different inputs are not compared blindly.

Usage: scenarios.py <out_dir> [--midi-dir DIR]
"""

import os
import struct
import sys

PPQ = 500
TEMPO_US = 500_000  # 1 tick = 1 ms


def _data_midi():
    """`data/midi` of the checkout (not versioned), or of the main checkout
    when this one is a git worktree; `BITEXACT_MIDI_DIR` overrides."""
    env = os.environ.get("BITEXACT_MIDI_DIR")
    if env:
        return env
    root = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    here = os.path.join(root, "data", "midi")
    if os.path.isdir(here):
        return here
    try:
        import subprocess
        common = subprocess.run(["git", "-C", root, "rev-parse", "--path-format=absolute", "--git-common-dir"], capture_output=True, text=True, check=True).stdout.strip()
        return os.path.join(os.path.dirname(common), "data", "midi")
    except Exception:
        return here


DATA_MIDI = _data_midi()
EXCERPT_MS = 20_000
EXCERPTS = [("dense_liszt", "liszt_la_campanella.mid"), ("dense_chopin", "chopin_nocturne_op9_2.mid")]


# ---------------------------------------------------------------- writing


class Score:
    """Events in milliseconds; events at the same time keep insertion order."""

    def __init__(self, description):
        self.description = description
        self.ev = []  # (ms, seq, bytes)

    def _add(self, t, data):
        t = int(t)
        if t < 0:
            raise ValueError("negative time")
        self.ev.append((t, len(self.ev), bytes(data)))

    def on(self, t, note, vel):
        self._add(t, (0x90, note, vel))

    def off(self, t, note, vel=64):
        self._add(t, (0x80, note, vel))

    def cc(self, t, ctl, val):
        self._add(t, (0xB0, ctl, val))

    def tap(self, t, note, vel, hold, rel=64):
        self.on(t, note, vel)
        self.off(t + hold, note, rel)

    def end_ms(self):
        return max((e[0] for e in self.ev), default=0)

    def smf(self):
        def vlq(n):
            out = [n & 0x7F]
            n >>= 7
            while n:
                out.append(0x80 | (n & 0x7F))
                n >>= 7
            return bytes(reversed(out))

        trk = bytearray()
        trk += vlq(0) + b"\xff\x51\x03" + TEMPO_US.to_bytes(3, "big")
        last = 0
        for t, _, data in sorted(self.ev):
            trk += vlq(t - last) + data
            last = t
        trk += vlq(0) + b"\xff\x2f\x00"
        return b"MThd" + struct.pack(">IHHH", 6, 0, 1, PPQ) + b"MTrk" + struct.pack(">I", len(trk)) + bytes(trk)


# ---------------------------------------------------------------- reading


def read_smf(path):
    """Minimal SMF reader: all tracks merged, tempo map applied.
    Returns [(ms float, status & 0xF0, data1, data2)] for note on/off and
    control change, sorted by (tick, track, order) like the engine's reader."""
    with open(path, "rb") as f:
        d = f.read()
    if d[:4] != b"MThd":
        raise ValueError(f"{path}: not a MIDI file")
    hlen, _fmt, ntrk, div = struct.unpack(">IHHH", d[4:14])
    if div & 0x8000:
        raise ValueError(f"{path}: SMPTE timing not supported")
    pos = 8 + hlen
    raw = []  # (tick, track, idx, kind, payload)
    for ti in range(ntrk):
        while d[pos:pos + 4] != b"MTrk":
            (clen,) = struct.unpack(">I", d[pos + 4:pos + 8])
            pos += 8 + clen
        (tlen,) = struct.unpack(">I", d[pos + 4:pos + 8])
        p, end = pos + 8, pos + 8 + tlen
        pos = end
        tick, status, idx = 0, 0, 0

        def vlq():
            nonlocal p
            n = 0
            while True:
                b = d[p]
                p += 1
                n = (n << 7) | (b & 0x7F)
                if not b & 0x80:
                    return n

        while p < end:
            tick += vlq()
            b = d[p]
            if b == 0xFF:
                mtype = d[p + 1]
                p += 2
                n = vlq()
                if mtype == 0x51 and n == 3:
                    raw.append((tick, ti, idx, "tempo", int.from_bytes(d[p:p + 3], "big")))
                p += n
                status = 0
            elif b in (0xF0, 0xF7):
                p += 1
                p += vlq()
                status = 0
            else:
                if b & 0x80:
                    status = b
                    p += 1
                hi = status & 0xF0
                if hi in (0xC0, 0xD0):
                    p += 1
                    continue
                a, v = d[p], d[p + 1]
                p += 2
                if hi in (0x80, 0x90, 0xB0):
                    raw.append((tick, ti, idx, "midi", (hi, a, v)))
            idx += 1
    raw.sort(key=lambda e: (e[0], e[1], e[2]))
    out, us_per_beat, last_tick, ms = [], 500_000, 0, 0.0
    for tick, _, _, kind, payload in raw:
        ms += (tick - last_tick) * us_per_beat / 1000.0 / div
        last_tick = tick
        if kind == "tempo":
            us_per_beat = payload
        else:
            out.append((ms,) + payload)
    return out


# ---------------------------------------------------------------- scenarios

KEYS_EVERY_4 = list(range(21, 108, 4)) + [108]  # A0 .. C8, 23 keys


def notes(vel):
    s = Score(f"single notes A0..C8 (every 4th key incl. 21 and 108) at velocity {vel}, held 0.5 s, 0.9 s apart")
    for i, k in enumerate(KEYS_EVERY_4):
        s.tap(i * 900, k, vel, 500)
    return s


def chords():
    s = Score("bass, middle, treble and 10-key chords, soft (35) and ff (127), without then with sustain pedal")
    groups = [[24, 31, 36, 43], [60, 64, 67, 72], [84, 88, 91, 96], [36, 43, 48, 52, 55, 60, 64, 67, 72, 76]]
    t = 0
    for vel in (35, 127):
        for g in groups:
            for k in g:
                s.tap(t, k, vel, 500)
            t += 800
    for vel in (35, 127):
        for g in groups:
            s.cc(t, 64, 127)
            for k in g:
                s.tap(t + 20, k, vel, 400)
            s.cc(t + 1000, 64, 0)
            t += 1300
    return s


def restrikes():
    s = Score("fast repetitions (60 ms, 20 ms, 1 ms) with and without release between, "
              "re-strike under a falling damper and under the pedal; out-of-range notes 20 and 109 (ignored)")
    t = 0
    for k, period, hold in ((64, 60, 40), (45, 20, 10), (88, 20, 10)):
        for i in range(10):  # with release between
            s.tap(t + i * period, k, 80, hold)
        t += 10 * period + 600
        for i in range(10):  # without release
            s.on(t + i * period, k, 80)
        s.off(t + 10 * period + 200, k)
        t += 10 * period + 900
    # two note-ons 1 ms apart, held; then on / off+on at the same tick.
    s.on(t, 60, 90)
    s.on(t + 1, 60, 90)
    s.off(t + 500, 60)
    t += 1200
    s.on(t, 60, 90)
    s.off(t + 1, 60)
    s.on(t + 1, 60, 90)
    s.off(t + 500, 60)
    t += 1200
    # Re-struck while the damper is coming down (still ringing).
    s.tap(t, 50, 100, 300)
    s.tap(t + 340, 50, 100, 560)
    t += 1500
    # Re-struck while ringing under the pedal.
    s.cc(t, 64, 127)
    s.tap(t + 10, 57, 100, 200)
    s.tap(t + 700, 57, 70, 200)
    s.cc(t + 1500, 64, 0)
    t += 1900
    # Out of the 21..108 range: the engine must ignore these.
    s.tap(t, 20, 100, 400)
    s.tap(t + 50, 109, 100, 400)
    s.tap(t + 100, 69, 60, 400)
    return s


def releases():
    s = Score("staccato, long holds, release velocities 10/64/127, release with the pedal half down (64)")
    t = 0
    for i, k in enumerate((33, 45, 57, 69, 81, 93)):  # staccato
        s.tap(t + i * 300, k, 90, 80)
    t += 6 * 300 + 700
    s.on(t, 40, 90)
    s.on(t + 5, 76, 90)
    s.off(t + 3000, 40)
    s.off(t + 3005, 76)
    t += 3900
    for k in (55, 84):
        for rel in (10, 64, 127):
            s.tap(t, k, 90, 400, rel)
            t += 1000
    s.cc(t, 64, 64)
    for k in (43, 62, 79):
        s.tap(t + 20, k, 90, 400)
    s.cc(t + 1600, 64, 0)
    return s


def sustain():
    s = Score("sustain pedal: down with nothing played then notes, pumped every 150 ms over 30 ringing keys, "
              "half-pedal 40/64/90, pedal down after the notes")
    s.cc(0, 64, 127)
    for i, k in enumerate((48, 55, 64)):
        s.tap(1000 + i * 150, k, 80, 200)
    s.cc(2600, 64, 0)
    t = 3000
    s.cc(t, 64, 127)
    for i in range(30):
        s.tap(t + 20 + i * 15, 33 + 2 * i, 75, 300)
    t += 1000
    for i in range(20):  # pump: up / down alternately every 150 ms
        s.cc(t + i * 150, 64, 0 if i % 2 == 0 else 127)
    t += 20 * 150
    s.cc(t, 64, 0)
    t += 600
    for v in (40, 64, 90):
        s.cc(t, 64, v)
        for k in (38, 57, 74):
            s.tap(t + 20, k, 90, 300)
        s.cc(t + 1300, 64, 0)
        t += 1600
    for k in (41, 60, 77):  # pedal down after the notes, keys still held
        s.tap(t, k, 90, 400)
    s.cc(t + 200, 64, 127)
    s.cc(t + 2000, 64, 0)
    return s


def soft_pedals():
    s = Score("sostenuto (cc 66) catching a held chord under staccato notes, una corda (cc 67) on a phrase")
    for k in (48, 52, 55):
        s.on(0, k, 85)
    s.cc(300, 66, 127)
    for k in (48, 52, 55):
        s.off(500, k)
    for i, k in enumerate((67, 71, 74, 79, 74, 71)):
        s.tap(700 + i * 200, k, 80, 90)
    s.cc(2400, 66, 0)
    t = 3200
    phrase = (60, 64, 67, 72, 67, 64)
    for corda in (127, 0):
        s.cc(t, 67, corda)
        for i, k in enumerate(phrase):
            s.tap(t + 50 + i * 250, k, 70 + 10 * (i % 3), 220)
        t += 2000
    return s


WHITE = [k for k in range(21, 109) if k % 12 in (0, 2, 4, 5, 7, 9, 11)]


def glissando():
    s = Score("white-key glissando A0..C8 up then down, 12 ms per key, no pedal")
    t = 0
    for i, k in enumerate(WHITE):
        s.tap(t + i * 12, k, 70, 60)
    t += len(WHITE) * 12 + 800
    for i, k in enumerate(reversed(WHITE)):
        s.tap(t + i * 12, k, 70, 60)
    return s


def cluster():
    s = Score("30 keys struck at the same tick, four times (velocities 100, 50, 127, 30)")
    keys = [30 + 2 * i for i in range(30)]
    for j, vel in enumerate((100, 50, 127, 30)):
        t = j * 1400
        for k in keys:
            s.on(t, k, vel)
        for k in keys:
            s.off(t + 800, k)
    return s


def excerpt(path, ms=EXCERPT_MS):
    name = os.path.basename(path)
    evs = read_smf(path)
    first = next((e[0] for e in evs if e[1] == 0x90 and e[3] > 0), 0.0)
    shift = max(0.0, first - 200.0)  # first note 0.2 s in
    s = Score(f"first {ms / 1000:g} s of {name} (from its first note), note-offs and pedals up at the cut")
    held, ctl = set(), {}
    for t, hi, a, v in evs:
        t = round(t - shift)
        if t < 0:
            t = 0
        if t >= ms:
            break
        if hi == 0x90 and v > 0:
            s.on(t, a, v)
            held.add(a)
        elif hi in (0x80, 0x90):
            s.off(t, a, v if hi == 0x80 else 64)
            held.discard(a)
        elif a in (64, 66, 67):  # only the pedals the engine models
            s.cc(t, a, v)
            ctl[a] = v
    for k in sorted(held):
        s.off(ms, k)
    for c in sorted(ctl):
        if ctl[c]:
            s.cc(ms, c, 0)
    return s


def dense_random(ms=EXCERPT_MS, seed=12345):
    s = Score(f"pseudo-random dense piece, {ms / 1000:g} s, both hands, pedal changes (LCG seed {seed})")
    x = seed

    def rnd(n):
        nonlocal x
        x = (x * 1103515245 + 12345) & 0x7FFFFFFF
        return (x >> 8) % n

    for t in range(0, ms, 250):  # left hand
        k = 28 + rnd(28)
        s.tap(t, k, 40 + rnd(60), 100 + rnd(300))
    for t in range(0, ms, 125):  # right hand, sometimes chords
        for _ in range(1 + (rnd(4) == 0) * 2):
            k = 56 + rnd(44)
            s.tap(t + rnd(5), k, 35 + rnd(85), 60 + rnd(400))
    s.cc(0, 64, 127)
    for t in range(2000, ms, 2000):
        s.cc(t, 64, 0)
        s.cc(t + 60, 64, 100 + rnd(28))
    s.cc(ms, 64, 0)
    return s


def build(midi_dir=DATA_MIDI):
    """Returns [(name, Score)] in a fixed order."""
    out = [("notes_soft", notes(30)), ("notes_mf", notes(80)), ("notes_ff", notes(127)),
           ("chords", chords()), ("restrikes", restrikes()), ("releases", releases()),
           ("sustain", sustain()), ("soft_pedals", soft_pedals()), ("glissando", glissando()),
           ("cluster30", cluster())]
    found = False
    for name, fn in EXCERPTS:
        p = os.path.join(midi_dir, fn)
        if os.path.isfile(p):
            out.append((name, excerpt(p)))
            found = True
    if not found:
        out.append(("dense_random", dense_random()))
    return out


def write(out_dir, midi_dir=DATA_MIDI):
    """Writes <out_dir>/<name>.mid for every scenario, returns
    [(name, path, description, last_event_ms)]."""
    os.makedirs(out_dir, exist_ok=True)
    res = []
    for name, s in build(midi_dir):
        p = os.path.join(out_dir, name + ".mid")
        data = s.smf()
        old = None
        if os.path.exists(p):
            with open(p, "rb") as f:
                old = f.read()
        if old != data:
            with open(p, "wb") as f:
                f.write(data)
        res.append((name, p, s.description, s.end_ms()))
    return res


if __name__ == "__main__":
    args = sys.argv[1:]
    md = DATA_MIDI
    if "--midi-dir" in args:
        i = args.index("--midi-dir")
        md = args[i + 1]
        del args[i:i + 2]
    if len(args) != 1:
        sys.exit(__doc__)
    for name, p, desc, end in write(args[0], md):
        print(f"{name:14s} {end / 1000:6.2f} s  {desc}")
