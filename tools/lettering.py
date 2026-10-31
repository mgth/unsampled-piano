#!/usr/bin/env python3
"""The name on the fallboard of the Model page's 3D view: a word's glyph
outlines from a font, flattened, laid out, their holes bridged into their
outlines and cut into triangles, written as Rust constants so that the
interface needs neither the font nor a font engine.

    python3 tools/lettering.py [font.ttf] [word] > crates/piano-ui/src/model/scene/lettering.rs

Default: "Unsampled" in Noto Serif Display Medium (SIL Open Font License
1.1, which lets a word drawn from it be embedded in software).
"""

import sys

from fontTools.pens.basePen import BasePen
from fontTools.ttLib import TTFont

FONT = "/usr/share/fonts/noto/NotoSerifDisplay-Medium.ttf"
# Segments per curve, and extra spacing between letters (em).
STEPS = 6
TRACKING = 0.02


class Flatten(BasePen):
    """Records each contour as a closed list of points, curves flattened."""

    def __init__(self, glyphset, dx):
        super().__init__(glyphset)
        self.dx = dx
        self.loops = []
        self.cur = []

    def _moveTo(self, p):
        self.cur = [p]

    def _lineTo(self, p):
        self.cur.append(p)

    def _curveToOne(self, p1, p2, p3):
        p0 = self.cur[-1]
        for k in range(1, STEPS + 1):
            t = k / STEPS
            u = 1 - t
            self.cur.append(tuple(u**3 * a + 3 * u * u * t * b + 3 * u * t * t * c + t**3 * d for a, b, c, d in zip(p0, p1, p2, p3)))

    def _qCurveToOne(self, p1, p2):
        p0 = self.cur[-1]
        for k in range(1, STEPS + 1):
            t = k / STEPS
            u = 1 - t
            self.cur.append(tuple(u * u * a + 2 * u * t * b + t * t * c for a, b, c in zip(p0, p1, p2)))

    def _closePath(self):
        p = self.cur
        if len(p) > 1 and abs(p[0][0] - p[-1][0]) < 1e-9 and abs(p[0][1] - p[-1][1]) < 1e-9:
            p = p[:-1]
        # Drop repeated points.
        q = [p[0]]
        for x in p[1:]:
            if abs(x[0] - q[-1][0]) > 1e-6 or abs(x[1] - q[-1][1]) > 1e-6:
                q.append(x)
        if len(q) >= 3:
            self.loops.append([(x + self.dx, y) for x, y in q])
        self.cur = []

    _endPath = _closePath


def area(p):
    return 0.5 * sum(p[i][0] * p[(i + 1) % len(p)][1] - p[(i + 1) % len(p)][0] * p[i][1] for i in range(len(p)))


def inside(q, p):
    c = False
    for i in range(len(p)):
        a, b = p[i], p[(i + 1) % len(p)]
        if (a[1] > q[1]) != (b[1] > q[1]) and q[0] < (b[0] - a[0]) * (q[1] - a[1]) / (b[1] - a[1]) + a[0]:
            c = not c
    return c


def cross(o, a, b):
    return (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])


def segments_cross(a, b, c, d):
    d1, d2, d3, d4 = cross(c, d, a), cross(c, d, b), cross(a, b, c), cross(a, b, d)
    return d1 * d2 < 0 and d3 * d4 < 0


def bridge(outer, holes, pts):
    """One simple polygon (indices into pts) from an outline (CCW) and its
    holes (CW): each hole joined to the outline by a cut there and back,
    from its rightmost point to the nearest point it can see."""
    poly = list(outer)
    for h in sorted(holes, key=lambda h: -max(pts[i][0] for i in h)):
        k = max(range(len(h)), key=lambda j: pts[h[j]][0])
        hp = pts[h[k]]
        edges = [(poly[i], poly[(i + 1) % len(poly)]) for i in range(len(poly))]
        others = [(h[j], h[(j + 1) % len(h)]) for j in range(len(h))]
        best = None
        for i, v in enumerate(poly):
            p = pts[v]
            ok = all(not segments_cross(hp, p, pts[a], pts[b]) for a, b in edges + others if v not in (a, b) and h[k] not in (a, b))
            d = (p[0] - hp[0]) ** 2 + (p[1] - hp[1]) ** 2
            if ok and (best is None or d < best[0]):
                best = (d, i)
        i = best[1]
        ring = h[k:] + h[:k] + [h[k]]
        poly = poly[: i + 1] + ring + [poly[i]] + poly[i + 1 :]
    return poly


def earclip(poly, pts):
    """Triangles (CCW) of a simple CCW polygon, bridges allowed."""
    idx = list(poly)
    tris = []
    guard = 0
    while len(idx) > 3 and guard < 100000:
        guard += 1
        n = len(idx)
        for i in range(n):
            a, b, c = idx[i - 1], idx[i], idx[(i + 1) % n]
            pa, pb, pc = pts[a], pts[b], pts[c]
            if cross(pa, pb, pc) <= 1e-12:
                continue
            ear = True
            for j in idx:
                if j in (a, b, c):
                    continue
                q = pts[j]
                if q in (pa, pb, pc):
                    continue
                if cross(pa, pb, q) >= 0 and cross(pb, pc, q) >= 0 and cross(pc, pa, q) >= 0:
                    ear = False
                    break
            if ear:
                tris.append((a, b, c))
                del idx[i]
                break
        else:
            raise SystemExit("ear clipping stuck")
    tris.append(tuple(idx))
    # The bridges' cuts leave triangles of no area: nothing to draw.
    return [t for t in tris if abs(cross(pts[t[0]], pts[t[1]], pts[t[2]])) > 1e-12]


def main():
    font = TTFont(sys.argv[1] if len(sys.argv) > 1 else FONT)
    word = sys.argv[2] if len(sys.argv) > 2 else "Unsampled"
    upem = font["head"].unitsPerEm
    gs = font.getGlyphSet()
    cmap = font.getBestCmap()
    hmtx = font["hmtx"]
    x = 0.0
    loops = []
    for ch in word:
        g = cmap[ord(ch)]
        pen = Flatten(gs, x)
        gs[g].draw(pen)
        loops += [[(px / upem, py / upem) for px, py in l] for l in pen.loops]
        x += hmtx[g][0] + TRACKING * upem
    width = (x - TRACKING * upem) / upem
    # Outlines counter-clockwise, holes clockwise; each hole to the
    # smallest outline holding it.
    pts, out_loops = [], []
    for l in loops:
        if area(l) < 0:
            l = l[::-1]
        out_loops.append(l)
    outer = [i for i, l in enumerate(out_loops) if not any(j != i and inside(l[0], m) for j, m in enumerate(out_loops))]
    holes = {i: [] for i in outer}
    for i, l in enumerate(out_loops):
        if i in outer:
            continue
        owner = min((j for j in outer if inside(l[0], out_loops[j])), key=lambda j: abs(area(out_loops[j])))
        holes[owner].append(i)
    index_loops = []
    for l in out_loops:
        start = len(pts)
        pts += l
        index_loops.append(list(range(start, start + len(l))))
    for i in outer:
        for h in holes[i]:
            index_loops[h] = index_loops[h][::-1]
    tris = []
    total = 0.0
    for i in outer:
        poly = bridge(index_loops[i], [index_loops[h] for h in holes[i]], pts)
        t = earclip(poly, pts)
        tris += t
        want = area(out_loops[i]) - sum(area(out_loops[h]) for h in holes[i])
        got = sum(cross(pts[a], pts[b], pts[c]) / 2 for a, b, c in t)
        assert abs(got - want) < 1e-6, (i, got, want)
        total += got
    # Loops as written: points in order (holes reversed, clockwise).
    order = []
    loop_ranges = []
    remap = {}
    for l in index_loops:
        s = len(order)
        for i in l:
            remap[i] = len(order)
            order.append(i)
        loop_ranges.append((s, len(l)))
    out = sys.stdout
    font_name = font["name"].getDebugName(4)
    out.write(f"//! Generated by tools/lettering.py: \"{word}\" in {font_name} (SIL Open Font\n")
    out.write("//! License 1.1), its curves flattened, its holes bridged and cut into\n")
    out.write("//! triangles. Do not edit; run the tool again.\n")
    out.write("#![allow(clippy::approx_constant)]\n\n")
    out.write("/// The word's width (em; its baseline at 0, its capitals some 0.7 em\n/// high).\n")
    out.write(f"pub(super) const WIDTH: f32 = {width:.5};\n\n")
    out.write("/// Its outline points (em).\n")
    out.write(f"pub(super) const POINTS: [(f32, f32); {len(order)}] = [\n")
    for i in order:
        out.write(f"    ({pts[i][0]:.5}, {pts[i][1]:.5}),\n")
    out.write("];\n\n")
    out.write("/// Its loops (first point, count): outlines counter-clockwise, holes\n/// clockwise.\n")
    out.write(f"pub(super) const LOOPS: [(u16, u16); {len(loop_ranges)}] = [\n")
    for s, n in loop_ranges:
        out.write(f"    ({s}, {n}),\n")
    out.write("];\n\n")
    out.write("/// Its face's triangles, counter-clockwise.\n")
    out.write(f"pub(super) const TRIS: [[u16; 3]; {len(tris)}] = [\n")
    for a, b, c in tris:
        out.write(f"    [{remap[a]}, {remap[b]}, {remap[c]}],\n")
    out.write("];\n")
    sys.stderr.write(f"{len(order)} points, {len(loop_ranges)} loops, {len(tris)} triangles, width {width:.3} em, area {total:.4} em²\n")


main()
