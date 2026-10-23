#!/usr/bin/env python3
"""Lays out a multisampled piano (an SFZ instrument) as a bench note set:
<note>v<layer>.<ext> for the layers 4/8/13, i.e. the regions the instrument
plays at velocities 40/60/100, at each key it recorded (pitch_keycenter),
pedal up, first round robin. Links to the samples (wav/flac) or converts
them (others). With --pedal, the regions played with the sustain pedal down
instead (locc64 ≥ 64), when the instrument has them.
  tools/sfz_set.py <instrument.sfz> <out dir> [--prefer close] [--pedal]"""
import argparse, os, re, subprocess, sys

NAMES = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"]
LAYERS = {4: 40, 8: 60, 13: 100}
def name(m): return NAMES[m % 12] + str(m // 12 - 1)

def note_number(v):
    v = v.strip()
    if re.fullmatch(r"-?\d+", v): return int(v)
    m = re.fullmatch(r"([a-gA-G])([#b]?)(-?\d+)", v)
    if not m: return None
    n = "C D EF G A B".index(m.group(1).upper()) + {"#": 1, "b": -1, "": 0}[m.group(2)]
    return n + 12 * (int(m.group(3)) + 1)

DIRECTIVE = re.compile(r'#define\s+(\$\w+)\s+(\S+)|#include\s+"([^"]+)"')

def preprocess(path, defines, root=None, depth=0):
    """The text of `path` with #define applied and #include expanded in
    reading order (a macro may change between regions; include paths are
    relative to the top instrument file, as in the SFZ format)."""
    root = root or os.path.dirname(path)
    text = re.sub(r"//[^\n]*", "", open(path, encoding="utf-8", errors="replace").read())
    def subst(t):
        for k in sorted(defines, key=len, reverse=True): t = t.replace(k, defines[k])
        return t
    out, pos = [], 0
    while True:
        # Macros apply to the directives too ("#include "$DIR/a.txt"").
        m = DIRECTIVE.search(text, pos)
        if not m: out.append(subst(text[pos:])); break
        out.append(subst(text[pos:m.start()]))
        if m.group(1): defines[m.group(1)] = subst(m.group(2))
        elif depth < 8: out.append(preprocess(os.path.join(root, subst(m.group(3))), defines, root, depth + 1))
        pos = m.end()
    return "\n".join(out)

def regions(sfz):
    text = preprocess(sfz, {})
    levels = {"control": {}, "global": {}, "master": {}, "group": {}}
    order = ["control", "global", "master", "group"]
    found = []
    # Headers and opcodes; a value runs up to the next opcode or header
    # (sample paths have spaces).
    for tok in re.finditer(r"<(\w+)>|(\w+)=(.*?)(?=\s+\w+=|\s*<|\s*$)", text, re.S | re.M):
        if tok.group(1):
            h = tok.group(1)
            cur = h
            if h in levels:
                for lv in order[order.index(h):]: levels[lv] = {}
            elif h == "region":
                r = {}
                for lv in order: r.update(levels[lv])
                found.append(r)
            continue
        k, v = tok.group(2), tok.group(3).strip()
        if cur == "region": found[-1][k] = v
        elif cur in levels: levels[cur][k] = v
    base = os.path.dirname(sfz)
    for r in found:
        r["_path"] = os.path.normpath(os.path.join(base, r.get("default_path", "").replace("\\", "/"), r.get("sample", "").replace("\\", "/")))
    return found

def pick(regs, pedal, prefer):
    out = {}
    for r in regs:
        if r.get("trigger", "attack") not in ("attack", "first", "legato") or r.get("sample", "").startswith("*"): continue
        if not os.path.exists(r["_path"]): continue
        if (int(r.get("locc64", 0)) >= 64) != pedal: continue
        if int(r.get("seq_position", 1)) != 1 or float(r.get("lorand", 0)) > 0: continue
        key = note_number(r.get("pitch_keycenter", r.get("key", "")))
        if key is None: continue
        lv = int(r.get("lovel", r.get("xfin_lovel", 1))); hv = int(r.get("hivel", r.get("xfout_hivel", 127)))
        for layer, v in LAYERS.items():
            if lv <= v <= hv:
                score = (prefer.lower() in r["_path"].lower()) if prefer else 0
                old = out.get((key, layer))
                if old is None or score > old[0]: out[(key, layer)] = (score, r["_path"])
    return {k: p for k, (_, p) in out.items()}

def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("sfz"); ap.add_argument("out")
    ap.add_argument("--prefer", default="", help="text in the sample path that wins among several microphones")
    ap.add_argument("--pedal", action="store_true")
    a = ap.parse_args()
    sel = pick(regions(a.sfz), a.pedal, a.prefer)
    if not sel: sys.exit("no region selected")
    os.makedirs(a.out, exist_ok=True)
    for (key, layer), p in sorted(sel.items()):
        ext = os.path.splitext(p)[1].lower()
        if ext in (".wav", ".flac"):
            dst = os.path.join(a.out, f"{name(key)}v{layer}{ext}")
            if os.path.lexists(dst): os.remove(dst)
            os.symlink(p, dst)
        else:
            subprocess.run(["sox", p, "-b", "24", os.path.join(a.out, f"{name(key)}v{layer}.wav")], check=True)
    keys = sorted({k for k, _ in sel})
    print(f"{len(sel)} files, {len(keys)} keys {name(keys[0])}–{name(keys[-1])}", file=sys.stderr)

if __name__ == "__main__":
    main()
