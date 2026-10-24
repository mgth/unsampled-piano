#!/usr/bin/env python3
"""The sustain pedal, from a Disklavier's isolated notes played once without
the pedal and again with it (Liang's pedalling dataset): the model renders
both MIDI files, and the same differences are read in the recordings and
in the model, so that the room and the microphones cancel.

  tools/pedal_bench.py <no-pedal audio> <no-pedal midi> <pedal audio> <pedal midi> [--name N] [--preset P]

Per register and pedal depth (CC64 at the onset), each pedalled note
against the same key and velocity without the pedal:
- halo: energy away from the note's own partials (60 Hz–6 kHz, ±2 %
  around each partial left out) re the partials' own, 0.2–0.9 s, pedal
  minus no pedal (dB): what the other strings add;
- hold: partials 1–4 at 1.3–1.8 s re 0.3–0.8 s, pedal (key up at 1 s, the
  pedal holding the dampers) minus no pedal (key held to 2 s) (dB): the
  half pedal damps, the sympathetic strings sustain;
- after: everything 0.3–0.6 s after the dampers fall (pedal up, or key up
  without the pedal) re 0.4–0.1 s before, pedal minus no pedal (dB)."""
import argparse, json, os, subprocess, sys
import numpy as np
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import midifile
import piece_bench as pb
import bench

def render_notes(notes, pedal, out, preset, jobs=6, listening=None):
    """Each note alone, with the pedal changes from 1 s before it to 4 s
    after, as its own render (in parallel): the model plays the same notes
    as the recording, without the hour-long file."""
    os.makedirs(out, exist_ok=True)
    exe = os.path.join(pb.ROOT, "target", "release", "unsampled-piano-render")
    tasks = []
    for i, (m, v, on, off) in enumerate(notes):
        t0 = on - 1.0
        ped = [(max(t - t0, 0.0), val) for t, val in pedal if on - 1.0 <= t <= on + 4.0]
        before = [val for t, val in pedal if t < on - 1.0]
        if before: ped.insert(0, (0.0, before[-1]))
        mid, wav = os.path.join(out, f"{i}.mid"), os.path.join(out, f"{i}.wav")
        midifile.write(mid, [(m, v, 1.0, off - t0)], ped)
        cmd = [exe, "midi", mid, wav, "--tail", "4"] + (["--preset", preset] if preset else []) + bench.listening_args(listening)
        tasks.append(cmd)
    from concurrent.futures import ThreadPoolExecutor
    with ThreadPoolExecutor(jobs) as ex:
        list(ex.map(lambda c: subprocess.run(c, check=True, capture_output=True), tasks))

def measures(x, T, m, on, damp, tune):
    fr = pb.partial_freqs(m, 80, tune)
    def win(t, dur=0.5):
        S, f = pb.spectrum(x, T(t), dur, 65536)
        return S, f
    def own(S, f, K=4):
        return pb.levels_at(S, f, fr[:K], 0.01).sum()
    def halo(S, f):
        band = (f > 60) & (f < 6000)
        mask = np.zeros_like(band)
        for fk in fr[fr < 6500]: mask |= (f > fk * 0.98) & (f < fk * 1.02)
        mask |= f < 0.7 * pb.f0(m)
        return S[band & ~mask].sum() / S[band & mask].sum()
    S1, f = win(on + 0.2, 0.7)
    S2, _ = win(on + 0.3); S3, _ = win(on + 1.3)
    Sb, _ = win(damp - 0.4, 0.3); Sa, _ = win(damp + 0.3, 0.3)
    if any(s is None for s in (S1, S2, S3, Sa, Sb)): return None
    db = lambda r: 10 * np.log10(r + 1e-30)
    return dict(halo=db(halo(S1, f)), hold=db(own(S3, f) / own(S2, f)), after=db(Sa.sum() / Sb.sum()))

def analyse(x, notes, pedal, tune, aligned=False):
    # Notes every 4–6 s: a lag beyond 2 s would lock on the wrong one.
    t0, drift = (0.0, 1.0) if aligned else pb.align(x, notes, 1.9)
    T = lambda t: t0 + drift * t
    down, release = pb.pedal_at(pedal)
    ts = np.array([p[0] for p in pedal] + [1e9]); vs = np.array([p[1] for p in pedal] + [0])
    out = {}
    for m, v, on, off in notes:
        i = np.searchsorted(ts, on, "right") - 1
        depth = int(vs[i]) if i >= 0 else 0
        damp = release(off) if m < pb.NO_DAMPER else off
        r = measures(x, T, m, on, damp, tune)
        if r: out.setdefault((m, v), {})[depth] = r
    return out, t0, drift

def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("np_audio"); ap.add_argument("np_midi"); ap.add_argument("p_audio"); ap.add_argument("p_midi")
    ap.add_argument("--name", default="pedal"); ap.add_argument("--preset"); ap.add_argument("--jobs", type=int, default=6)
    ap.add_argument("--listening", action="append", help="listening preset(s) (default: close microphones, studio)")
    ap.add_argument("--velocity", type=int, action="append", help="only the notes at this velocity (49, 80, 96; repeatable): fewer renders")
    ap.add_argument("--out", default=os.path.join(pb.ROOT, "out", "pieces"))
    a = ap.parse_args()
    out = os.path.join(a.out, a.name); os.makedirs(out, exist_ok=True)
    srcs = {}
    for tag, audio, midi in (("np", a.np_audio, a.np_midi), ("p", a.p_audio, a.p_midi)):
        notes, pedal = midifile.read(midi)
        # The recordings' measures do not change between runs: cached.
        cache = os.path.join(a.out, f"recording_{os.path.basename(audio)}.json")
        if os.path.exists(cache):
            res = {tuple(json.loads(k)): {int(d): r for d, r in v.items()} for k, v in json.load(open(cache)).items()}
        else:
            x = pb.load(audio)
            tune = pb.tuning(x, notes, *pb.align(x, notes, 1.9))
            res, t0, drift = analyse(x, notes, pedal, tune)
            json.dump({json.dumps(list(k)): v for k, v in res.items()}, open(cache, "w"))
        srcs[("recording", tag)] = res
        # The recording is analysed whole (its cache serves every run); the
        # model plays only the notes asked for.
        if a.velocity: notes = [n for n in notes if n[1] in a.velocity]
        d = os.path.join(out, f"model_{tag}"); render_notes(notes, pedal, d, a.preset, a.jobs, a.listening)
        res = {}
        for i, (m, v, on, off) in enumerate(notes):
            x = pb.load(os.path.join(d, f"{i}.wav"))
            ped = [(t - on + 1.0, val) for t, val in pedal if on - 1.0 <= t <= on + 4.0]
            before = [val for t, val in pedal if t < on - 1.0]
            if before: ped.insert(0, (0.0, before[-1]))
            r, _, _ = analyse(x, [(m, v, 1.0, off - on + 1.0)], ped or [(0.0, 0)], 1.0, aligned=True)
            for k2, dd in r.items(): res.setdefault(k2, {}).update(dd)
        srcs[("model", tag)] = res
        print(f"{tag}: {sum(len(d) for d in srcs[('recording', tag)].values())} recorded notes, {sum(len(d) for d in res.values())} rendered", file=sys.stderr)
    if a.velocity:
        for k in [k for k in srcs if k[0] == "recording"]:
            srcs[k] = {mv: d for mv, d in srcs[k].items() if mv[1] in a.velocity}
    table = {}
    for src in ("recording", "model"):
        nop, ped = srcs[(src, "np")], srcs[(src, "p")]
        for (m, v), depths in ped.items():
            base = nop.get((m, v), {}).get(0)
            if not base: continue
            for d, r in depths.items():
                if d == 0: continue
                key = (pb.reg_of(m), d)
                for k in r:
                    table.setdefault(key, {}).setdefault(src, {}).setdefault(k, []).append(r[k] - base[k])
    L = [f"# {a.name}: the sustain pedal, recording against the model", "",
         "Pedal minus no pedal, same key and velocity (median over the notes, dB); model − recording in the last column of each.", "",
         "| register | CC64 | notes | halo rec / model / Δ | hold rec / model / Δ | after rec / model / Δ |", "|---|---|---|---|---|---|"]
    summary = {}
    for (reg, d) in sorted(table, key=lambda k: ([r for _, _, r in pb.REGS].index(k[0]), k[1])):
        t = table[(reg, d)]
        if "recording" not in t or "model" not in t: continue
        row = f"| {reg} | {d} | {len(t['recording']['halo'])} |"
        for k in ("halo", "hold", "after"):
            r, mo = float(np.median(t["recording"][k])), float(np.median(t["model"][k]))
            row += f" {r:+.1f} / {mo:+.1f} / {mo - r:+.1f} |"
            summary.setdefault(reg, {}).setdefault(str(d), {})[k] = dict(recording=r, model=mo)
        L.append(row)
    open(os.path.join(out, "report.md"), "w").write("\n".join(L) + "\n")
    json.dump(summary, open(os.path.join(out, "result.json"), "w"), indent=1)
    # Every note's own measures (not differences), for a closer look.
    json.dump({f"{src}|{tag}": {json.dumps(list(k)): v for k, v in r.items()} for (src, tag), r in srcs.items()},
              open(os.path.join(out, "notes.json"), "w"))
    print(os.path.relpath(os.path.join(out, "report.md"), pb.ROOT))

if __name__ == "__main__":
    main()
