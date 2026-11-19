#!/usr/bin/env python3
"""Coordinate search on an instrument preset, judged by the comparison tool
over all its measures, against the real pianos that do not buzz
(docs/compare_search.md). Build first: cargo build --release -p piano-compare.

A step is kept if it removes measures from the red without raising the mean gap,
or lowers the mean gap without adding any to the red. Passes repeat until one
keeps nothing; the steps are then halved once, and the search goes on."""
import argparse, json, os, re, subprocess, sys, time, traceback

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
TOOL = os.path.join(ROOT, "target", "release", "unsampled-piano-compare")
PRESET = os.path.join(ROOT, "presets", "instruments", "Concert grand", "concert_grand.toml")
WITHOUT = "bivib_grand,iowa,liang"
BEST = STATE = LOG = TRIAL = OUT = ""
MIN_OFF, MAX_MEAN_RISE, MIN_MEAN = 2, 0.002, 0.003
MAX_PASSES, MAX_STEPS = 10, 3

# (group, section, key, index, op, step, default if absent, low, high)
P = [
    ("ripple", "unison", "width_mid", None, "mul", 1.15, 1.0, 0.2, 3.0),
    ("ripple", "unison", "width_curve", None, "mul", 1.25, None, 0.3, 4.0),
    ("ripple", "unison", "width_treble", None, "mul", 1.15, None, 0.5, 6.0),
    ("ripple", "unison", "width_bass", None, "mul", 1.5, None, 0.01, 1.0),
    ("ripple", "unison", "hit_spread", None, "mul", 1.3, 0.3, 0.02, 2.0),
    ("late decay", "losses", "b2", None, "mul", 1.25, None, 1e-8, 1e-6),
    ("late decay", "losses", "b2_top", None, "mul", 1.25, None, 1e-8, 2e-6),
    ("late decay", "losses", "b2_high", None, "mul", 1.4, None, 1e-8, 2e-6),
    ("late decay", "losses", "b1_floor", None, "add", 0.05, None, 0.0, 1.0),
    ("early decay", "polarization", "radiation", None, "add", 0.1, None, 0.0, 2.0),
    ("early decay", "polarization", "slope", None, "add", 0.25, None, 0.0, 3.0),
    ("early decay", "polarization", "slope_to", None, "mul", 1.2, None, 300.0, 3000.0),
    ("early decay", "polarization", "hammer_tilt", None, "add", 0.02, None, 0.0, 0.5),
    ("early decay", "soundboard", "loss_factor", None, "mul", 1.1, None, 0.01, 0.06),
    ("early decay", "soundboard", "loss_corner", None, "mul", 1.25, None, 800.0, 12000.0),
    ("early decay", "soundboard", "loss_low_corner", None, "mul", 1.25, None, 20.0, 300.0),
    ("rise time", None, "shank_freq_mid", None, "mul", 1.15, None, 200.0, 1500.0),
    ("rise time", None, "hammer_mass_scale", None, "mul", 1.05, None, 0.7, 1.4),
    ("rise time", None, "hammer_mass_slope", None, "mul", 1.5, None, 0.0001, 0.005),
    ("t60", "polarization", "h_coupling", None, "mul", 1.3, None, 0.005, 0.3),
    ("t60", "polarization", "detune_cents", None, "add", 0.15, None, 0.0, 2.0),
    ("t60", "polarization", "radiation_bass", None, "add", 0.1, None, 0.0, 2.0),
    ("t60", "polarization", "hammer_tilt_bass", None, "add", 0.1, None, 0.0, 1.5),
    ("t60", "polarization", "cross_bass", None, "add", 0.01, None, 0.0, 0.2),
    ("spectral tilt", None, "hardness", None, "mul", 1.15, None, 0.3, 4.0),
    ("spectral tilt", None, "hardness_slope", None, "add", 0.01, None, -0.15, 0.05),
    ("spectral tilt", None, "hardness_slope_top", None, "add", 0.015, None, 0.0, 0.3),
    ("spectral tilt", None, "hardness_slope_bass", None, "add", 0.006, None, -0.05, 0.1),
    ("spectral tilt", None, "strike_ratio_top", None, "add", 0.01, None, 0.02, 0.12),
    ("spectral tilt", None, "p_offset", None, "add", 0.1, 0.0, -0.6, 0.6),
    ("between partials", "body", "gain", None, "mul", 1.25, None, 0.005, 0.5),
    ("between partials", "body", "slope", None, "add", 0.75, None, -9.0, 3.0),
    ("between partials", "sympathetic", "gain", None, "mul", 1.25, 1.0, 0.2, 3.0),
] + [("between partials", "body", "levels", i, "add", 3.0, None, -40.0, 20.0) for i in range(8)] \
  + [("between partials", "body", "t60", i, "mul", 1.3, None, 0.02, 3.0) for i in range(8)] \
  + [("between partials", "body", "speed_low", i, "add", 0.25, None, -1.0, 4.0) for i in range(4)] \
  + [("between partials", "body", "speed_high", i, "add", 0.25, None, -1.0, 4.0) for i in range(4)] \
  + [("between partials", "body", "speed_halo", i, "add", 0.1, None, 0.0, 1.0) for i in range(4)] \
  + [("tuning", "stretch", "a_t", None, "mul", 1.03, None, 0.5, 1.5),
     ("tuning", "stretch", "k_t", None, "mul", 1.1, None, 0.05, 0.2),
     ("tuning", "stretch", "a_b", None, "mul", 1.05, None, 0.05, 1.0),
     ("tuning", "stretch", "k_b", None, "mul", 1.1, None, 0.03, 0.2),
     ("tuning", "stretch", "c", None, "add", 0.3, None, -5.0, 3.0)]


def log(msg):
    line = time.strftime("%H:%M:%S ") + msg
    with open(LOG, "a") as f:
        f.write(line + "\n")
    print(line, flush=True)


def span(lines, section):
    if section is None:
        return 0, next((i for i, l in enumerate(lines) if l.startswith("[")), len(lines))
    start = lines.index(f"[{section}]") + 1
    return start, next((i for i in range(start, len(lines)) if lines[i].startswith("[")), len(lines))


def fmt(v):
    return repr(round(float(v), 10))


def get(text, p):
    _, sec, key, idx, _, _, default, _, _ = p
    lines = text.split("\n")
    a, e = span(lines, sec)
    for i in range(a, e):
        m = re.match(rf"{re.escape(key)}\s*=\s*(.*)$", lines[i])
        if m:
            v = m.group(1).split("#")[0].strip()
            return float(v) if idx is None else float(v.strip("[]").split(",")[idx])
    return default


def put(text, p, value):
    _, sec, key, idx, *_ = p
    lines = text.split("\n")
    a, e = span(lines, sec)
    for i in range(a, e):
        m = re.match(rf"({re.escape(key)}\s*=\s*)(.*)$", lines[i])
        if m:
            if idx is None:
                lines[i] = m.group(1) + fmt(value)
            else:
                items = [x.strip() for x in m.group(2).split("#")[0].strip().strip("[]").split(",")]
                items[idx] = fmt(value)
                lines[i] = m.group(1) + "[" + ", ".join(items) + "]"
            return "\n".join(lines)
    lines.insert(a, f"{key} = {fmt(value)}")
    return "\n".join(lines)


def label(p):
    _, sec, key, idx, *_ = p
    return (f"{sec}." if sec else "") + key + (f"[{idx}]" if idx is not None else "")


def evaluate(text):
    with open(TRIAL, "w") as f:
        f.write(text)
    subprocess.run([TOOL, "--preset", TRIAL, "--score", OUT, "--band-without", WITHOUT, "--score-threads", "6"],
                   cwd=ROOT, check=True, capture_output=True, text=True, timeout=900)
    return json.load(open(OUT))


def better(s, st):
    off, mean = s["off"], s["mean"]
    if off <= st["off"] - MIN_OFF and mean <= st["mean"] + MAX_MEAN_RISE:
        return True
    return off <= st["off"] and mean <= st["mean"] - MIN_MEAN


def brief(s):
    parts = " ".join(f"{k.split(',')[0].split()[0][:5]}{v['off']}" for k, v in s["measures"].items() if v["off"])
    return f"off {s['off']} mean {s['mean']:.4f} | {parts}"


def save(st, text):
    with open(BEST, "w") as f:
        f.write(text)
    with open(STATE + ".tmp", "w") as f:
        json.dump(st, f, indent=1)
    os.replace(STATE + ".tmp", STATE)


def main():
    global PRESET, WITHOUT, BEST, STATE, LOG, TRIAL, OUT
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--preset", default=PRESET, help="the preset to start from (it is not modified)")
    ap.add_argument("--band-without", default=WITHOUT, help="real pianos left out of the band")
    ap.add_argument("--work", default=os.path.join(ROOT, "out", "compare-search"), help="folder of the state, the log and best.toml; a run left there is resumed")
    args = ap.parse_args()
    PRESET, WITHOUT = os.path.abspath(args.preset), args.band_without
    os.makedirs(args.work, exist_ok=True)
    BEST, STATE, LOG = (os.path.join(args.work, n) for n in ("best.toml", "state.json", "log.txt"))
    TRIAL, OUT = os.path.join(args.work, "trial.toml"), os.path.join(args.work, "trial.json")
    if os.path.exists(STATE) and os.path.exists(BEST):
        st = json.load(open(STATE))
        text = open(BEST).read()
        log(f"RESUME pass {st['pass']} at {st['next']}: off {st['off']} mean {st['mean']:.4f}")
    else:
        text = open(PRESET).read()
        s = evaluate(text)
        log("BASE " + brief(s) + " band " + ",".join(s["band"]))
        st = {"off": s["off"], "mean": s["mean"], "base_off": s["off"], "base_mean": s["mean"], "pass": 1, "next": 0,
              "scale": 1.0, "kept": [], "kept_this_pass": 0, "trials": 0}
        save(st, text)
    while st["pass"] <= MAX_PASSES:
        for i in range(st["next"], len(P)):
            p = P[i]
            kept_here = False
            step = p[5] ** st["scale"] if p[4] == "mul" else p[5] * st["scale"]
            for sign in (1, -1):
                for _ in range(MAX_STEPS):
                    cur = get(text, p)
                    if cur is None:
                        log(f"SKIP {label(p)}: absent")
                        break
                    new = cur * step ** sign if p[4] == "mul" else cur + sign * step
                    if not p[7] <= new <= p[8]:
                        break
                    trial = put(text, p, new)
                    try:
                        s = evaluate(trial)
                    except Exception as e:
                        log(f"FAIL {label(p)} {new:.5g}: {e}\n{traceback.format_exc(limit=1)}")
                        break
                    st["trials"] += 1
                    ok = better(s, st)
                    log(f"{'KEEP' if ok else 'drop'} p{st['pass']} [{p[0]}] {label(p)} {cur:.5g} -> {new:.5g}: {brief(s)}")
                    if not ok:
                        break
                    text, kept_here = trial, True
                    st.update(off=s["off"], mean=s["mean"], kept_this_pass=st["kept_this_pass"] + 1)
                    st["kept"].append([st["pass"], p[0], label(p), cur, new, s["off"], s["mean"]])
                    save(st, text)
                if kept_here:
                    break
            st["next"] = i + 1
            save(st, text)
        log(f"PASS {st['pass']} (step x{st['scale']}): kept {st['kept_this_pass']}, off {st['off']}, mean {st['mean']:.4f}, {st['trials']} trials")
        if not st["kept_this_pass"]:
            if st["scale"] < 1.0:
                break
            st["scale"] = 0.5
        st.update(next=0, kept_this_pass=0)
        st["pass"] += 1
        save(st, text)
    log(f"DONE off {st['base_off']} -> {st['off']}, mean {st['base_mean']:.4f} -> {st['mean']:.4f}, {len(st['kept'])} steps kept, {st['trials']} trials")
    return 0


if __name__ == "__main__":
    sys.exit(main())
