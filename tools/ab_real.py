#!/usr/bin/env python3
"""A/B against a real recording: the model plays the recording's MIDI, the
same passage is cut from both (aligned on the MIDI as tools/piece_bench.py
aligns), and both are brought to the same integrated loudness (EBU R128),
so that what differs is the sound, not the level.

  tools/ab_real.py <audio> <midi> <out prefix> [--start S] [--dur S]
                   [--listening default|close] [--preset P]

Writes <prefix>_real.mp3 and <prefix>_model.mp3. --listening default: the
engine's own (a concert hall heard from 1.5 m), for hall recordings;
close: close microphones in a studio (tools/bench.py), for close ones.
KEEP=1 keeps the intermediate WAV files (their folder is printed)."""
import argparse, os, re, subprocess, sys, tempfile
import numpy as np
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import bench, midifile, piece_bench as pb

def loudness(path):
    r = subprocess.run(["ffmpeg", "-hide_banner", "-nostats", "-i", path, "-af", "ebur128=framelog=quiet", "-f", "null", "-"],
                       capture_output=True, text=True)
    return float(re.findall(r"I:\s+(-?[\d.]+) LUFS", r.stderr)[-1])

def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("audio"); ap.add_argument("midi"); ap.add_argument("out")
    ap.add_argument("--start", type=float, default=0.0); ap.add_argument("--dur", type=float, default=60.0)
    ap.add_argument("--listening", choices=("default", "close"), default="default"); ap.add_argument("--preset")
    a = ap.parse_args()
    tmp = tempfile.mkdtemp(dir=os.path.dirname(os.path.abspath(a.out)))
    wav = os.path.join(tmp, "model.wav")
    cmd = [os.path.join(bench.ROOT, "target", "release", "unsampled-piano-render"), "midi", a.midi, wav, "--tail", "3"]
    if a.preset: cmd += ["--preset", a.preset]
    if a.listening == "close": cmd += bench.listening_args()
    subprocess.run(cmd, check=True, capture_output=True)
    notes, _ = midifile.read(a.midi)
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
