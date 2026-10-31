//! A grand's cast-iron plate as most are made, drawn rather than modelled:
//! the model's frame is a plate of some mass and stiffness over the
//! frame's outline at the frame's height, without a shape of its own.
//! Drawn over the strings as they lie: low, from where the strings' rear
//! lengths end at their hitch pins out to the walls, the plate that holds
//! them, its weight cut by round holes where it is broad, in a row along
//! the curve; a bar across the front between the hammers and the dampers;
//! struts standing on edge, deep and narrow, from the bar along the
//! strings: either side of the bass strings, at their slant; beside the
//! tenor's lowest strings (under the bass strings crossing them), twice
//! among the treble's in the gaps between unisons, and past the last;
//! crossing where they will, each sloping gently down to its bridge and,
//! past it, down onto the plate.
//! Bar and struts stand over the dampers, lifted.

use super::*;

/// The hitch pins' plate: its top (under the hitch pins of the lowest
/// strings), how far it reaches in front of them, clear of the bridge and
/// its pins, and short of the walls.
pub(super) const RAIL_TOP: f64 = 0.017;
const RAIL_FRONT: f64 = 0.03;
const BRIDGE_CLEAR: f64 = 0.025;
const WALL: f64 = 0.004;
/// The bar's and struts' depth, the same all along; their underside over
/// the dampers' heads lifted (8.9 cm on the default grand) and, where
/// they cross their bridge, over the strings (the bass's 5.5 cm, their
/// motion shown in the live view); where the dampers' zone ends (along
/// the strings, from the board's front edge) and the struts start down.
const DEPTH: f64 = 0.045;
pub(super) const CLEAR: f64 = 0.093;
const LOW: f64 = 0.068;
const DAMPERS_END: f64 = 0.08;
/// An upright's: low over the strings all along (its dampers and hammers
/// between them), in front of the bass strings crossing the others; their
/// depth.
pub(super) const U_CLEAR: f64 = STRING_Z + BASS_RISE + 0.01;
pub(super) const U_DEPTH: f64 = 0.025;
/// A strut's web (thickness), its flange along the top (width, depth),
/// how wide it flares to where it leaves the bar and over how long; how
/// far past its bridge it goes down, and how far onto the plate it lands.
const WEB: f64 = 0.011;
const FLANGE: (f64, f64) = (0.019, 0.005);
const FLARE_W: f64 = 0.05;
const FLARE_L: f64 = 0.09;
const PAST_BRIDGE: f64 = 0.015;
const LAND: f64 = 0.03;
/// The bar across the front: its back short of the dampers' heads, its
/// thickness along the strings (the hammers strike in front of it).
pub(super) const BAR_BACK: f64 = -0.003;
const BAR_W: f64 = 0.024;
/// The holes in the plate under the hitch pins: their largest radius,
/// their spacing along the curve, the plate they leave round them, and the
/// row's distance from the wall past their rims.
const HOLE_R: f64 = 0.045;
const HOLE_GAP: f64 = 0.14;
const HOLE_N: usize = 20;
const HOLE_MARGIN: f64 = 0.015;
const HOLE_IN: f64 = 0.04;
/// How far a strut beside a group of strings stands from its outermost
/// string; how far clear of the bass strings, across, the tenor and
/// treble's own struts begin.
const BESIDE: f64 = 0.012;
const PAST_BASS: f64 = 0.03;

/// Where a strut runs along the strings: beside a key's unison, on its
/// bass side or its treble side, or in the gap below it.
#[derive(Clone, Copy)]
enum Strut {
    Beside(u8, bool),
    Below(u8),
}

/// The point at `t` from `p` along `u`.
fn along(p: (f64, f64), u: (f64, f64), t: f64) -> (f64, f64) {
    (p.0 + u.0 * t, p.1 + u.1 * t)
}

/// How far from `p` along `u` the polygon `outline` is.
fn reach(outline: &[(f64, f64)], p: (f64, f64), u: (f64, f64)) -> f64 {
    let n = outline.len();
    (0..n)
        .filter_map(|i| {
            let (a, b) = (outline[i], outline[(i + 1) % n]);
            let e = (b.0 - a.0, b.1 - a.1);
            let denom = u.0 * e.1 - u.1 * e.0;
            if denom.abs() < 1e-12 {
                return None;
            }
            let w = (a.0 - p.0, a.1 - p.1);
            let (t, s) = ((w.0 * e.1 - w.1 * e.0) / denom, (w.0 * u.1 - w.1 * u.0) / denom);
            (t > 0.0 && (0.0..=1.0).contains(&s)).then_some(t)
        })
        .fold(f64::MAX, f64::min)
}

/// Distance from `q` to the polyline `line`.
fn distance(q: (f64, f64), line: &[(f64, f64)]) -> f64 {
    line.windows(2)
        .map(|s| {
            let (a, b) = (s[0], s[1]);
            let (ex, ey) = (b.0 - a.0, b.1 - a.1);
            let t = (((q.0 - a.0) * ex + (q.1 - a.1) * ey) / (ex * ex + ey * ey).max(1e-18)).clamp(0.0, 1.0);
            (q.0 - a.0 - t * ex).hypot(q.1 - a.1 - t * ey)
        })
        .fold(f64::MAX, f64::min)
}

fn inside(q: (f64, f64), p: &[(f64, f64)]) -> bool {
    let mut c = false;
    for i in 0..p.len() {
        let (a, b) = (p[i], p[(i + 1) % p.len()]);
        if (a.1 > q.1) != (b.1 > q.1) && q.0 < (b.0 - a.0) * (q.1 - a.1) / (b.1 - a.1) + a.0 {
            c = !c;
        }
    }
    c
}

/// Points, triangles over them and loops, as [`Builder::prism`] takes them.
type Mesh = (Vec<(f64, f64)>, Vec<[usize; 3]>, Vec<(usize, usize)>);

/// The polygon `p` without the points where it runs straight on (turning
/// by less than a milliradian).
fn trimmed(p: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let n = p.len();
    let d = |a: (f64, f64), b: (f64, f64)| (a.0 - b.0).hypot(a.1 - b.1);
    (0..n)
        .filter(|&i| {
            let (a, b, c) = (p[(i + n - 1) % n], p[i], p[(i + 1) % n]);
            cross2(a, b, c).abs() > 1e-3 * d(a, b) * d(b, c)
        })
        .map(|i| p[i])
        .collect()
}

/// A ring from radius `r0` to `r1` about `c`, `n` points round.
fn annulus(c: (f64, f64), r0: f64, r1: f64, n: usize) -> Mesh {
    let at = |r: f64, k: usize| {
        let a = std::f64::consts::TAU * k as f64 / n as f64;
        (c.0 + r * a.cos(), c.1 + r * a.sin())
    };
    // Outside counter-clockwise, inside clockwise (its kth point at −k).
    let mut pts: Vec<(f64, f64)> = (0..n).map(|k| at(r1, k)).collect();
    pts.extend((0..n).map(|k| at(r0, (n - k) % n)));
    let inner = |k: usize| n + (n - k % n) % n;
    let tris = (0..n).flat_map(|k| [[k, (k + 1) % n, inner(k + 1)], [k, inner(k + 1), inner(k)]]).collect();
    (pts, tris, vec![(0, n), (n, n)])
}

/// The polygon `outer` (counter-clockwise) with its `holes` (clockwise)
/// as one simple polygon, each hole joined to it by a cut there and back,
/// the shortest it sees to a point of the outline not cut to yet; and its
/// loops (the outline's and each hole's, as [`Builder::prism`] takes
/// them) over the same points.
fn bridged(outer: &[(f64, f64)], holes: &[Vec<(f64, f64)>]) -> Mesh {
    let mut pts: Vec<(f64, f64)> = outer.to_vec();
    let mut loops = vec![(0, outer.len())];
    let mut hole_idx = Vec::new();
    for h in holes {
        loops.push((pts.len(), h.len()));
        hole_idx.push((pts.len()..pts.len() + h.len()).collect::<Vec<_>>());
        pts.extend(h);
    }
    let crosses = |a: (f64, f64), b: (f64, f64), c: (f64, f64), d: (f64, f64)| cross2(c, d, a) * cross2(c, d, b) < 0.0 && cross2(a, b, c) * cross2(a, b, d) < 0.0;
    let mut poly: Vec<usize> = (0..outer.len()).collect();
    let mut used = vec![false; outer.len()];
    let d = |p: (f64, f64), q: (f64, f64)| (p.0 - q.0).hypot(p.1 - q.1);
    for h in &hole_idx {
        let edges: Vec<(usize, usize)> = (0..poly.len()).map(|i| (poly[i], poly[(i + 1) % poly.len()])).chain(hole_idx.iter().flat_map(|g| (0..g.len()).map(move |j| (g[j], g[(j + 1) % g.len()])))).collect();
        // The shortest cut it sees, to an outline point not cut to yet.
        let mut pairs: Vec<(usize, usize)> = (0..h.len()).flat_map(|k| (0..poly.len()).filter(|&i| poly[i] < outer.len() && !used[poly[i]]).map(move |i| (k, i))).collect();
        pairs.sort_by(|&(k, i), &(l, j)| d(pts[h[k]], pts[poly[i]]).total_cmp(&d(pts[h[l]], pts[poly[j]])));
        let seen = pairs.into_iter().find(|&(k, i)| {
            let (v, hk) = (poly[i], h[k]);
            edges.iter().all(|&(a, b)| a == v || b == v || a == hk || b == hk || !crosses(pts[hk], pts[v], pts[a], pts[b]))
        });
        let Some((k, i)) = seen else { continue };
        used[poly[i]] = true;
        let ring: Vec<usize> = h[k..].iter().chain(&h[..k]).copied().chain([h[k], poly[i]]).collect();
        poly.splice(i + 1..i + 1, ring);
    }
    let shape: Vec<(f64, f64)> = poly.iter().map(|&i| pts[i]).collect();
    let tris = triangulate(&shape).into_iter().map(|t| t.map(|i| poly[i])).collect();
    (pts, tris, loops)
}

/// The plate over a grand's strings with their `ends`, in `c`.
pub(super) fn plate(b: &mut Builder, model: &PianoModel, sb: &SoundboardParams, ends: &[StringEnds], c: Color32) {
    let place = b.place;
    let at = |x: f64, y: f64, z: f64| place.at(x, y, z);
    let (over, low, depth) = if place.vertical { (U_CLEAR, U_CLEAR, U_DEPTH) } else { (CLEAR, LOW, DEPTH) };
    let board = board_outline(sb);
    // The case's inside, carried forward over the pin block.
    let case = frame_outline(model, sb);
    let mut walls = case.clone();
    walls.push(case[0]);
    // A string's line: its bridge point, direction, speaking length.
    let line = |m: u8| {
        let ((ax, ay), (bx, by), _) = ends[(m - 21) as usize];
        let l = (bx - ax).hypot(by - ay).max(1e-9);
        ((ax, ay), (bx, by), ((bx - ax) / l, (by - ay) / l), l)
    };

    // The struts' lines (start, direction, length, where they start down):
    // in the gap below a key's unison, from the bar's middle on until
    // they meet the walls.
    // Seen from the keyboard: the bass strings with one on their left and
    // one on their right; the tenor and treble's with one on their left
    // (under the bass strings crossing them), two among them clear of the
    // bass strings, a third and two thirds of the way on, and one past
    // the last.
    let main = sb.main_bridge_from.clamp(22, 107);
    let bass_reach = (21..main).map(|m| line(m).1 .1.min(line(m).0 .1)).fold(f64::MAX, f64::min);
    let clear = (main..=108).find(|&m| line(m).0 .1.max(line(m).1 .1) < bass_reach - PAST_BASS).unwrap_or(main);
    let third = |k: u8| clear + ((108 - clear) as u32 * k as u32 / 3) as u8;
    let mut places = vec![Strut::Beside(21, true), Strut::Beside(main - 1, false), Strut::Beside(main, true)];
    places.extend([third(1), third(2)].into_iter().filter(|&m| m > main && m < 108).map(Strut::Below));
    places.push(Strut::Beside(108, false));
    let struts: Vec<_> = places
        .into_iter()
        .filter_map(|place| {
            let m = match place {
                Strut::Beside(m, _) | Strut::Below(m) => m,
            };
            let (a, bp, u, l) = line(m);
            // Across (towards the bass, plate y up on a grand).
            let nrm = (-u.1, u.0);
            let off = match place {
                Strut::Below(m) => {
                    let ((px, py), _, _) = ends[(m - 22) as usize];
                    ((px - a.0) * nrm.0 + (py - a.1) * nrm.1) / 2.0
                }
                Strut::Beside(m, bass) => {
                    let half = (model.note(m).strings.max(1) - 1) as f64 * UNISON_GAP / 2.0 + BESIDE;
                    if bass { half } else { -half }
                }
            };
            let p0 = (a.0 + nrm.0 * off, a.1 + nrm.1 * off);
            // From the bar's middle (or where it is first inside the case,
            // the bass's outermost), on until it meets the walls.
            let ok = |q: (f64, f64)| inside(q, &case) && distance(q, &walls) >= WALL + 0.002;
            let mut start = along(p0, u, (BAR_BACK - BAR_W / 2.0 - p0.0) / u.0);
            let mut skip = 0.0;
            while !ok(start) && skip < 1.0 {
                start = along(start, u, 0.005);
                skip += 0.005;
            }
            let mut t = 0.0;
            while t < 4.0 && ok(along(start, u, t + 0.005)) {
                t += 0.005;
            }
            // Over its bridge, then down onto the plate just behind it.
            let on = |p: (f64, f64)| (p.0 - start.0) * u.0 + (p.1 - start.1) * u.1;
            let bridge = on(bp);
            let land = on(along(bp, u, (hitch_length(&board, bp, u, l) - RAIL_FRONT).max(BRIDGE_CLEAR) + LAND)).min(t);
            (land >= 0.06 && land > bridge + PAST_BRIDGE + 0.01).then_some((start, u, land, bridge, skip == 0.0))
        })
        .collect();

    // Under each bridge's hitch pins, from a little in front of them out
    // to the walls (every other key's); where it is broad, along the curve,
    // round holes with raised rims.
    let circle = |(c, r): ((f64, f64), f64)| (0..HOLE_N).map(|k| -std::f64::consts::TAU * k as f64 / HOLE_N as f64).map(|a| (c.0 + r * a.cos(), c.1 + r * a.sin())).collect::<Vec<_>>();
    for bass in [true, false] {
        let mut front = Vec::new();
        let mut back = Vec::new();
        for m in (21..=108u8).filter(|&m| (m < sb.main_bridge_from) == bass).step_by(3) {
            let (_, bp, u, l) = line(m);
            let rear = hitch_length(&board, bp, u, l);
            let wall = reach(&board, bp, u);
            if wall == f64::MAX || wall < rear || model.note(m).strings == 0 {
                continue;
            }
            front.push(along(bp, u, (rear - RAIL_FRONT).max(BRIDGE_CLEAR)));
            back.push(along(bp, u, wall - WALL));
        }
        if front.len() < 2 {
            continue;
        }
        let mut rest = vec![front[0]];
        rest.extend(&back);
        rest.push(front[front.len() - 1]);
        let pairs: Vec<_> = front.iter().copied().zip(back.iter().copied()).collect();
        let back_line = back.clone();
        let mut poly = front.clone();
        poly.extend(back.into_iter().rev());
        if signed_area(&poly) < 0.0 {
            poly.reverse();
        }
        let poly = trimmed(&poly);
        let mut edge = poly.clone();
        edge.push(poly[0]);
        // In a row along the curve, as large as the plate leaves room for
        // with its rim (the hitch pins in front), clear of the edges, of
        // the struts (which would hide them) and of each other, one every
        // so far along.
        let mut kept: Vec<((f64, f64), f64)> = Vec::new();
        for (f, w) in pairs {
            // Back from the wall along the string until the wall is far
            // enough, square to it.
            let depth = (w.0 - f.0).hypot(w.1 - f.1);
            let want = HOLE_MARGIN + 1.25 * HOLE_R + HOLE_IN;
            let point = |d: f64| (w.0 + (f.0 - w.0) * d / depth, w.1 + (f.1 - w.1) * d / depth);
            let mut d = want;
            while d < depth && distance(point(d), &back_line) < want {
                d += 0.01;
            }
            let centre = point(d);
            let r = HOLE_R.min((distance(centre, &front) - RAIL_FRONT - HOLE_MARGIN) / 1.25).min((distance(centre, &rest) - HOLE_MARGIN) / 1.25);
            let clear = r > 0.015 && inside(centre, &poly) && distance(centre, &edge) > 1.25 * r + HOLE_MARGIN;
            let free = struts.iter().all(|&(s, u, t, _, _)| distance(centre, &[s, along(s, u, t)]) > FLANGE.0 / 2.0 + 1.25 * r + HOLE_MARGIN);
            let apart = kept.iter().all(|&(c2, r2)| (c2.0 - centre.0).hypot(c2.1 - centre.1) > (1.25 * (r + r2) + HOLE_MARGIN).max(HOLE_GAP));
            if clear && free && apart {
                kept.push((centre, r));
            }
        }
        // The cut checked by its area (a fan whose ends cross the outline's
        // corners can fold over): without the holes if they fail it, none
        // if it fails without.
        let cut = |holes: &[((f64, f64), f64)]| {
            let circles: Vec<_> = holes.iter().copied().map(circle).collect();
            let (pts, tris, loops) = bridged(&poly, &circles);
            let area: f64 = tris.iter().map(|t| cross2(pts[t[0]], pts[t[1]], pts[t[2]]) / 2.0).sum();
            let want = signed_area(&poly) + circles.iter().map(|h| signed_area(h)).sum::<f64>();
            ((area - want).abs() < 1e-6).then_some((pts, tris, loops))
        };
        let (mesh, holes) = match cut(&kept) {
            Some(m) => (Some(m), kept),
            None => (cut(&[]), Vec::new()),
        };
        if let Some((pts, tris, loops)) = mesh {
            b.prism(&pts, &loops, &tris, (0.0, RAIL_TOP), &at, c);
            for &(centre, r) in &holes {
                let (pts, tris, loops) = annulus(centre, r, 1.25 * r, HOLE_N);
                b.prism(&pts, &loops, &tris, (RAIL_TOP - 0.002, RAIL_TOP + 0.004), &at, c);
            }
        }
    }

    // The bar across the front, between the hammers and the dampers, on
    // edge as the struts, from wall to wall.
    let (y_lo, y_hi) = board.iter().fold((f64::MAX, f64::MIN), |(a, z), p| (a.min(p.1), z.max(p.1)));
    let side = place.dir(0.0, 0.0, 1.0);
    let x = BAR_BACK - BAR_W / 2.0;
    let ring = |y: f64, z: (f64, f64), w: f64, r: f64| (at(x, y, (z.0 + z.1) / 2.0), ((z.1 - z.0) / 2.0) as f32, (w / 2.0) as f32, r as f32);
    let (y0, y1) = (y_lo + WALL, y_hi - WALL);
    b.sweep(&[ring(y0, (over, over + depth), BAR_W, 0.004), ring(y1, (over, over + depth), BAR_W, 0.004)], side, c);

    // The struts: on edge, as deep all along, flaring where they leave the
    // bar, over the dampers, sloping gently down to their bridge, past
    // which they go down onto the plate; a flange along the top.
    let up = place.dir(0.0, 0.0, 1.0);
    for &(start, u, t, bridge, from_bar) in &struts {
        // Past the dampers, the slope beginning.
        let at_x = |x: f64| ((x - start.0) / u.0).clamp(0.0, t);
        let level = at_x(DAMPERS_END).min(bridge);
        // Its sections where its shape bends, straight between.
        let mut cuts: Vec<f64> = (0..=6).map(|k| FLARE_L * k as f64 / 6.0).collect();
        cuts.extend([level, bridge]);
        cuts.extend((0..=6).map(|k| bridge + PAST_BRIDGE + (t - bridge - PAST_BRIDGE) * k as f64 / 6.0));
        cuts.retain(|&s| (0.0..=t).contains(&s));
        cuts.sort_by(f64::total_cmp);
        cuts.dedup_by(|a, b| (*a - *b).abs() < 0.003);
        let mut web = Vec::with_capacity(cuts.len());
        let mut flange = Vec::with_capacity(cuts.len());
        let smooth = |v: f64| {
            let v = v.clamp(0.0, 1.0);
            v * v * (3.0 - 2.0 * v)
        };
        for &s in &cuts {
            let (x, y) = along(start, u, s);
            // Its underside over the dampers, sloping down to just over
            // the strings at its bridge, then down onto the plate; as deep
            // all along.
            let slope = over + (low - over) * ((s - level) / (bridge - level).max(1e-6)).clamp(0.0, 1.0);
            let lo = slope + (RAIL_TOP - 0.001 - slope) * smooth((s - bridge - PAST_BRIDGE) / (t - bridge - PAST_BRIDGE).max(1e-6));
            let hi = lo + depth;
            let f = if from_bar { (1.0 - s / FLARE_L).max(0.0) } else { 0.0 };
            let w = WEB + (FLARE_W - WEB) * f * f;
            let mid = |z: f64| add(at(x, y, 0.0), scale(up, z as f32));
            web.push((mid((lo + hi) / 2.0), ((hi - lo) / 2.0) as f32, (w / 2.0) as f32, 0.004f32));
            let fw = FLANGE.0.max(w);
            flange.push((mid(hi - FLANGE.1 / 2.0), (FLANGE.1 / 2.0) as f32, (fw / 2.0) as f32, (FLANGE.1 / 2.0 - 0.0005) as f32));
        }
        b.sweep(&web, up, c);
        b.sweep(&flange, up, c);
    }
}
