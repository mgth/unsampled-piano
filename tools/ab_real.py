#!/usr/bin/env python3
"""A/B against a real recording: the model plays the recording's MIDI, the
same passage is cut from both (aligned on the MIDI as tools/piece_bench.py
aligns), and both are brought to the same integrated loudness (EBU R128),
so that what differs is the sound, not the level.

  tools/ab_real.py <audio> <midi> <out prefix> [--start S] [--dur S]
                   [--listening default|close] [--preset P]

Writes <prefix>_real.mp3 and <prefix>_model.mp3. --listening default: the
engine's own (a concert hall heard from 1.5 m), for hall recordings;
close: close microphones in a studio (tools/bench.py), for close ones;
auto: each room of presets/listening/Room heard by the engine's
microphones and by each set of presets/listening/Microphones, keeping the pair whose room and balance sound closest to the recording's
over the passage: L/R coherence at 0.5–4 kHz, how far the envelope dips
between notes (p90 − p10 of its 10 ms level), and the long-term spectrum's
octaves from 250 Hz to 4 kHz re 1 kHz (a closed lid sounds closed; the
microphones above the board hear its 150–350 Hz modes 10 dB too loud for
MAESTRO's recordings, which close microphones match; the 8 kHz octave holds
the recordings' hiss).
KEEP=1 keeps the intermediate WAV files (their folder is printed)."""
import argparse, glob, os, re, subprocess, sys, tempfile
import numpy as np
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import bench, midifile, piece_bench as pb

def loudness(path):
    r = subprocess.run(["ffmpeg", "-hide_banner", "-nostats", "-i", path, "-af", "ebur128=framelog=quiet", "-f", "null", "-"],
                       capture_output=True, text=True)
    return float(re.findall(r"I:\s+(-?[\d.]+) LUFS", r.stderr)[-1])

def room_traits(path, notes, start, dur):
    """(L/R coherence 0.5–4 kHz, envelope dip depth dB, octave levels at
    250 Hz–8 kHz re 1 kHz) over the passage."""
    from scipy.signal import coherence, butter, sosfiltfilt
    mono = pb.load(path); t0, dr = pb.align(mono, notes)
    # From the first note on: before it the model is silent, the recording
    # holds its noise floor.
    first = min(n[2] for n in notes)
    if start < first: dur, start = dur - (first - start), first
    raw = subprocess.run(["sox", path, "-t", "f32", "-r", "48000", "-c", "2", "-"], capture_output=True, check=True).stdout
    s = np.frombuffer(raw, np.float32).reshape(-1, 2).astype(float)
    s = s[max(0, int((t0 + dr * start) * 48000)):max(0, int((t0 + dr * (start + dur)) * 48000))]
    f, c = coherence(s[:, 0], s[:, 1], fs=48000, nperseg=4096)
    y = sosfiltfilt(butter(3, (200, 5000), btype="band", fs=48000, output="sos"), s.mean(1)); h = 480; n = len(y) // h
    e = 10 * np.log10((y[:n * h].reshape(n, h) ** 2).mean(1) + 1e-20)
    P = np.abs(np.fft.rfft(s.mean(1)[: 1 << 21] * np.hanning(min(len(s), 1 << 21)))) ** 2; fr = np.fft.rfftfreq(min(len(s), 1 << 21), 1 / 48000)
    oct_ = np.array([10 * np.log10(P[(fr >= c / 1.414) & (fr < c * 1.414)].sum() + 1e-30) for c in (250, 500, 1000, 2000, 4000, 8000)])
    return c[(f > 500) & (f < 4000)].mean(), np.percentile(e, 90) - np.percentile(e, 10), oct_ - oct_[2]

def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("audio"); ap.add_argument("midi"); ap.add_argument("out")
    ap.add_argument("--start", type=float, default=0.0); ap.add_argument("--dur", type=float, default=60.0)
    ap.add_argument("--listening", choices=("default", "close", "auto"), default="default"); ap.add_argument("--preset")
    a = ap.parse_args()
    tmp = tempfile.mkdtemp(dir=os.path.dirname(os.path.abspath(a.out)))
    wav = os.path.join(tmp, "model.wav")
    notes, _ = midifile.read(a.midi)
    def render(out, extra):
        cmd = [os.path.join(bench.ROOT, "target", "release", "unsampled-piano-render"), "midi", a.midi, out, "--tail", "3"]
        if a.preset: cmd += ["--preset", a.preset]
        subprocess.run(cmd + extra, check=True, capture_output=True)
    if a.listening == "auto":
        want = room_traits(a.audio, notes, a.start, a.dur)
        best = None
        lst = os.path.join(bench.ROOT, "presets", "listening")
        mics = [None] + sorted(glob.glob(os.path.join(lst, "Microphones", "*.toml")))
        rooms = [None] + sorted(glob.glob(os.path.join(lst, "Room", "*.toml")))
        for i, (mic, room) in enumerate((m, r) for m in mics for r in rooms):
            out = os.path.join(tmp, f"room_{i}.wav"); pre = [p for p in (mic, room) if p]
            render(out, [x for p in pre for x in ("--listening", p)])
            got = room_traits(out, notes, a.start, a.dur)
            tilt = float(np.sqrt(np.mean((got[2][:5] - want[2][:5]) ** 2)))
            d = abs(got[0] - want[0]) / 0.05 + abs(got[1] - want[1]) / 2.0 + tilt / 1.0
            label = " + ".join(os.path.basename(p)[:-5] for p in pre) or "default"
            print(f"  {label}: coherence {got[0]:.2f}, dips {got[1]:.1f} dB, balance {tilt:.1f} dB rms (recording {want[0]:.2f}, {want[1]:.1f})")
            if best is None or d < best[0]: best = (d, out, label)
        os.replace(best[1], wav)
        print(f"  kept {best[2]}")
    else:
        render(wav, bench.listening_args() if a.listening == "close" else [])
    # Where the MIDI's zero falls in each recording (the model's is ≈ 0).
    t_real, drift = pb.align(pb.load(a.audio), notes)
    t_model, _ = pb.align(pb.load(wav), notes)
    cuts = []
    for name, src, t0, rate in (("real", a.audio, t_real, drift), ("model", wav, t_model, 1.0)):
        s = t0 + rate * a.start
        cut = os.path.join(tmp, f"cut_{name}.wav")
        pad = max(-s, 0.0)
        subprocess.run(["sox", src, "-b", "24", cut, "trim", f"{max(s, 0.0):.4f}", f"{a.dur:.3f}", "pad", f"{pad:.4f}", "0",
                        "fade", "0.05", f"{a.dur:.3f}", "1.5", "channels", "2", "rate", "48000"], check=True, capture_output=True)
        cuts.append((name, cut))
    # Both at the same integrated loudness, low enough that neither clips.
    louds = {name: loudness(cut) for name, cut in cuts}
    peaks = {name: 20 * np.log10(np.abs(pb.load(cut)).max() + 1e-12) for name, cut in cuts}
    target = min(-20.0, min(louds[n] - peaks[n] - 1.0 for n in louds))
    for name, cut in cuts:
        gain = target - louds[name]
        subprocess.run(["ffmpeg", "-loglevel", "error", "-y", "-i", cut, "-af", f"volume={gain:.2f}dB", "-b:a", "192k", f"{a.out}_{name}.mp3"], check=True)
        print(f"{a.out}_{name}.mp3 ({louds[name]:.1f} LUFS, {gain:+.1f} dB → {target:.1f} LUFS)")
    if os.environ.get("KEEP"): print("kept", tmp); return
    for f in os.listdir(tmp): os.remove(os.path.join(tmp, f))
    os.rmdir(tmp)

if __name__ == "__main__":
    main()
