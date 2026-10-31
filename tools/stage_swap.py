#!/usr/bin/env python3
"""Stage swap, for the ear (docs/stage_swap.md): the model's strings heard
through a real grand's board, and the real grand's bridge heard through the
model's board. The real grand is Plath's (anechoic chamber, bridge
acceleration and dummy head at the player recorded together, every key).

  tools/stage_swap.py <plath dir> <out dir> [--keys 21,33,38,45] [--velocity auto|V]
                      [--preset P] [--listening L.toml …] [--state PROD08] [--take 2] [--analytic] [--keep-lid]

For each key (MIDI m; Plath's key is m − 20):
  real:   bridge acceleration a_r and ears p_r (50 kHz → 48 kHz);
  model:  bridge velocity v_m at the key's region (piano-render's
          bridge_signal example) → a_m by backward difference; ears p_m at
          the player's microphones, no room, no lid (Plath's grand, a
          Steinway concert grand, was measured with its lid taken off).
  boards: each board is estimated the same way from its own (a, p) pair:
          Welch cross-spectrum, bins coherent (> 0.5) and energetic, the
          log-magnitude interpolated across the partials, minimum phase
          plus the ear's delay. Everything that reaches the ears in step
          with the bridge counts as the board (rim, frame, head included);
          what does not (the action's noise, the sympathetic strings) is
          left out of the swapped files. --analytic takes the model's board
          from its modes instead (the example's transfer grid: the board
          alone, exact at every frequency).
Files (48 kHz, 24-bit, aligned on the onset, same loudness, EBU R128):
  <note>_0_real.wav                      p_r: the real piano
  <note>_M_model.wav                     p_m: the model
  <note>_A_model_strings_real_board.wav  a_m ∗ h_r
  <note>_B_real_strings_model_board.wav  a_r ∗ h_m
  check/<note>_real_via_estimate.wav     a_r ∗ h_r, should sound as 0_real
  check/<note>_model_via_estimate.wav    a_m ∗ h_m, should sound as M_model
The velocity `auto` matches the model's bridge acceleration (rms, 0.1–1 s)
to the real one."""
import argparse, glob, os, re, subprocess, sys, tempfile
import numpy as np
from scipy.signal import resample_poly, fftconvolve
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import bench as b

SR = 48000
ROOT = b.ROOT
EXE = os.path.join(ROOT, "target", "release", "examples", "bridge_signal")
BANDS = [50.0 * 2 ** (i / 3) for i in range(22)]  # third octaves 50 Hz–6.3 kHz

def load_plath(d, m, state, take):
    files = sorted(glob.glob(os.path.join(d, f"D1_{state}_{m - 20:02d}_*M{take:04d}.csv")))
    if not files: raise SystemExit(f"no Plath file for key {m - 20}")
    x = np.loadtxt(files[0], delimiter=";", skiprows=1, usecols=(2, 3, 4), dtype=np.float32).astype(float)
    x -= x.mean(0)
    return [resample_poly(c, 24, 25) for c in x.T]  # a, pL, pR at 48 kHz

def onset(x):
    a = np.abs(x); return int(np.argmax(a > 0.1 * a.max()))

def welch_xy(x, y, W, hop):
    n = (len(x) - W) // hop + 1
    idx = np.arange(W)[None, :] + hop * np.arange(n)[:, None]
    win = np.hanning(W)[None, :]
    X = np.fft.rfft(x[idx] * win, axis=1); Y = np.fft.rfft(y[idx] * win, axis=1)
    return (np.abs(X) ** 2).mean(0), (np.conj(X) * Y).mean(0), (np.abs(Y) ** 2).mean(0)

def min_phase(mag, L):
    """Minimum-phase impulse response (length L) with magnitude `mag` on the rfft grid of L."""
    c = np.fft.irfft(np.log(np.maximum(mag, 1e-9)), L)
    c[1:L // 2] *= 2; c[L // 2 + 1:] = 0
    return np.fft.irfft(np.exp(np.fft.rfft(c)), L)

def board_estimate(a, p, s):
    """h(t) for one ear: pressure per bridge acceleration, from the note after its onset."""
    W = 16384; hop = W // 4
    seg = slice(max(s - int(0.05 * SR), 0), len(a))
    Saa, Sap, Spp = welch_xy(a[seg], p[seg], W, hop)
    coh = np.abs(Sap) ** 2 / (Saa * Spp + 1e-30)
    f = np.fft.rfftfreq(W, 1 / SR)
    keep = (coh > 0.5) & (Saa > 1e-4 * Saa.max()) & (f >= 20) & (f <= 10000)
    H = np.abs(Sap[keep] / Saa[keep]); fk = f[keep]
    L = 1 << 17
    fl = np.fft.rfftfreq(L, 1 / SR)
    mag = np.exp(np.interp(np.log(np.maximum(fl, 1.0)), np.log(fk), np.log(H)))
    # nothing below 20 Hz, a smooth roll-off above 10 kHz
    mag *= np.clip((fl - 10) / 10, 0, 1) * np.clip((14000 - fl) / 4000, 0, 1)
    h = min_phase(mag, L)[: int(1.5 * SR)]
    h *= np.hanning(2 * len(h))[len(h):]
    # the ear's delay re the bridge, from the cross-correlation over the attack
    seg = slice(s, s + int(0.3 * SR))
    xc = fftconvolve(p[seg], a[seg][::-1], mode="full"); mid = len(a[seg]) - 1
    d = int(np.argmax(np.abs(xc[mid: mid + int(0.01 * SR)])))
    return np.concatenate([np.zeros(d), h]), (int(keep.sum()), fk.min(), fk.max())

def model_signals(prefix, preset, listening, m, velocity, hold, tail, transfer):
    cmd = [EXE, preset or os.path.join(ROOT, "presets", "default.toml"), prefix, "--note", str(m), "--velocity", str(velocity),
           "--hold", str(hold), "--tail", str(tail)] + [x for l in listening for x in ("--listening", l)]
    if transfer: cmd += ["--transfer", str(transfer)]
    subprocess.run(cmd, check=True, capture_output=True)
    v = np.fromfile(prefix + "_bridge.f32", dtype=np.float32).astype(float)
    a = np.diff(v, prepend=0.0) * SR
    p = np.fromfile(prefix + "_mics.f32", dtype=np.float32).astype(float).reshape(-1, 2)
    h = None
    if transfer:
        t = np.fromfile(prefix + "_transfer.f32", dtype=np.float32).astype(float).reshape(-1, 2, 2)
        H = t[:, :, 0] + 1j * t[:, :, 1]
        h = np.stack([np.fft.irfft(H[:, k], transfer) for k in range(2)], 1)
    return a, p, h

def rms_db(x, s, t0, t1):
    seg = x[s + int(t0 * SR): s + int(t1 * SR)]; return 10 * np.log10((seg ** 2).mean() + 1e-30)

def thirds(x, s, t0=0.1, t1=1.0):
    seg = x[s + int(t0 * SR): s + int(t1 * SR)]
    P = np.abs(np.fft.rfft(seg * np.hanning(len(seg)))) ** 2; f = np.fft.rfftfreq(len(seg), 1 / SR)
    return np.array([10 * np.log10(P[(f >= c / 2 ** (1 / 6)) & (f < c * 2 ** (1 / 6))].sum() + 1e-30) for c in BANDS])

def check(label, x, y, s_x, s_y=None, t0=0.1, t1=1.0):
    """Third-octave levels of x re y (both channels' power), over the
    bands within 40 dB of y's strongest: how well a board estimate
    reproduces what was recorded through it, or what a swap changes."""
    s_y = s_x if s_y is None else s_y
    both = lambda z, s: 10 * np.log10(10 ** (thirds(z[:, 0], s, t0, t1) / 10) + 10 ** (thirds(z[:, 1], s, t0, t1) / 10))
    tx, ty = both(x, s_x), both(y, s_y)
    sel = ty >= ty.max() - 40; d = (tx - ty)[sel]
    off = " ".join(f"{c:.0f}:{v:+.0f}" for c, v in zip(np.array(BANDS)[sel], d) if abs(v) > 3)
    print(f"   {label} ({t0}–{t1} s): {d.mean():+.1f} dB mean, {d.std():.1f} dB rms about it" + (f"; > 3 dB at {off}" if off else ""))

def loudness(path):
    r = subprocess.run(["ffmpeg", "-hide_banner", "-nostats", "-i", path, "-af", "ebur128=framelog=quiet", "-f", "null", "-"], capture_output=True, text=True)
    return float(re.findall(r"I:\s+(-?[\d.]+) LUFS", r.stderr)[-1])

def write(path, st, gain=1.0):
    y = (st * gain).astype(np.float32)
    subprocess.run(["sox", "-t", "f32", "-r", str(SR), "-c", "2", "-", "-b", "24", path], input=y.tobytes(), check=True)

def cut(st, s, dur=4.9, pre=0.05):
    s0 = max(s - int(pre * SR), 0); n = int((dur + pre) * SR)
    out = st[s0: s0 + n]
    return out if len(out) == n else np.pad(out, ((0, n - len(out)), (0, 0)))

def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("plath"); ap.add_argument("out"); ap.add_argument("--keys", default="21,33,38,45")
    ap.add_argument("--velocity", default="auto"); ap.add_argument("--preset")
    ap.add_argument("--listening", action="append", default=[os.path.join(ROOT, "presets", "listening", "Microphones", "player.toml")])
    ap.add_argument("--state", default="PROD08"); ap.add_argument("--take", type=int, default=2)
    ap.add_argument("--analytic", action="store_true", help="the model's board from its modes, not from its own signals")
    ap.add_argument("--keep-lid", action="store_true", help="keep the listening preset's lid (Plath's grand had none)")
    a = ap.parse_args()
    os.makedirs(os.path.join(a.out, "check"), exist_ok=True)
    tmp = tempfile.mkdtemp(dir=a.out)
    if not a.keep_lid:
        nolid = os.path.join(tmp, "nolid.toml"); open(nolid, "w").write("[soundboard]\nlid_removed = true\n"); a.listening.append(nolid)
    hold, tail, L = 4.8, 0.4, 1 << 18
    outputs = []
    for m in [int(k) for k in a.keys.split(",")]:
        note = b.name(m)
        ar, pl, pr = load_plath(a.plath, m, a.state, a.take)
        s_r = onset(np.abs(pl) + np.abs(pr))
        target = rms_db(ar, s_r, 0.1, 1.0)
        # the velocity whose bridge acceleration matches the real one
        if a.velocity == "auto":
            vs = [50, 70, 90, 110, 127]; got = []
            for v in vs:
                am, _, _ = model_signals(os.path.join(tmp, f"{note}_v{v}"), a.preset, a.listening, m, v, 1.5, 0.1, 0)
                got.append(rms_db(am, onset(am), 0.1, 1.0))
            velocity = int(round(np.clip(np.interp(target, got, vs), 1, 127)))
        else:
            velocity = int(a.velocity)
        am, pm, hm_an = model_signals(os.path.join(tmp, note), a.preset, a.listening, m, velocity, hold, tail, L)
        s_m = onset(am)
        print(f"{note}: Plath key {m - 20}; velocity {velocity} (bridge acceleration rms 0.1–1 s: real {target:.1f}, model {rms_db(am, s_m, 0.1, 1.0):.1f} dB re 1 m/s²)")
        hr, hm = [], []
        for p in (pl, pr):
            h, info = board_estimate(ar, p, s_r); hr.append(h)
        print(f"   real board: {info[0]} coherent bins from {info[1]:.0f} to {info[2]:.0f} Hz")
        for k in range(2):
            h, info = board_estimate(am, pm[:, k], s_m); hm.append(h)
        print(f"   model board: {info[0]} coherent bins from {info[1]:.0f} to {info[2]:.0f} Hz")
        n = len(ar)
        real = np.stack([pl, pr], 1)
        conv = lambda x, hs: np.stack([fftconvolve(x, h)[:len(x)] for h in hs], 1)
        chk_r = conv(ar, hr); chk_m = conv(am, hm); chk_an = conv(am, [hm_an[:, 0], hm_an[:, 1]])
        check("check: real through its own estimate, re real", chk_r, real, s_r)
        check("check: model through its own estimate, re model", chk_m, pm, s_m)
        check("check: model through its modes' transfer, re model", chk_an, pm, s_m)
        A = conv(am, hr)
        B = conv(ar, [hm_an[:, 0], hm_an[:, 1]]) if a.analytic else conv(ar, hm)
        # the two stages themselves, model re real: the bridge accelerations
        # (matched in rms over 0.1–1 s) and the ears
        for t0, t1 in ((0.0, 0.05), (0.1, 1.0)):
            check("bridge acceleration, model re real", np.stack([am, am], 1), np.stack([ar, ar], 1), s_m, s_r, t0, t1)
            check("ears, model re real", pm, real, s_m, s_r, t0, t1)
        # what each swap changes: the strings' share (A re real, same board)
        # and the board's share (B re model, same strings)
        for t0, t1 in ((0.0, 0.05), (0.1, 1.0)):
            check("A (model strings, real board) re the real piano", A, real, s_m, s_r, t0, t1)
            check("B (real strings, model board) re the model", B, pm, s_r, s_m, t0, t1)
        for name, st, s in ((f"{note}_0_real", real, s_r), (f"{note}_M_model", pm, s_m),
                            (f"{note}_A_model_strings_real_board", A, s_m), (f"{note}_B_real_strings_model_board", B, s_r),
                            (f"check/{note}_real_via_estimate", chk_r, s_r), (f"check/{note}_model_via_estimate", chk_m, s_m)):
            outputs.append((os.path.join(a.out, name + ".wav"), cut(st, s)))
    # written once at a safe level, measured, then brought to the same loudness
    peak = max(np.abs(st).max() for _, st in outputs)
    for path, st in outputs: write(path, st, 0.5 / peak)
    lufs = {path: loudness(path) for path, _ in outputs}
    print("files (loudness before normalising, re the real piano's):")
    ref = lufs[outputs[0][0]]
    for path, st in outputs:
        g = 10 ** ((-23 - lufs[path]) / 20); pk = np.abs(st).max() * g
        if pk > 0.9: g *= 0.9 / pk
        write(path, st, g)
        print(f"   {os.path.relpath(path, ROOT)}  ({lufs[path] - ref:+.1f} dB)")
    for f in os.listdir(tmp): os.remove(os.path.join(tmp, f))
    os.rmdir(tmp)

if __name__ == "__main__":
    main()
