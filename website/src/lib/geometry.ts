// The default instrument's case, in board coordinates (metres): x along the
// strings from the keyboard end of the board towards the tail, y across from
// the treble side (0) to the bass side (b). Drawn from above as the pianist
// sees it: keyboard at the bottom, treble on the right.
import { def } from './params';

export const BOARD = { a: def('soundboard.a'), b: def('soundboard.b') };

const pts = (path: string, n: number): [number, number][] =>
  Array.from({ length: n }, (_, i) => [def(`${path}[${i}][0]`) * BOARD.a, def(`${path}[${i}][1]`) * BOARD.b]);

/** Main bridge from C8 down to its lowest key, and the bass bridge below. */
export const TREBLE_BRIDGE = pts('soundboard.treble_bridge', 5);
export const BASS_BRIDGE = pts('soundboard.bass_bridge', 2);
export const MAIN_FROM = def('soundboard.main_bridge_from');

/** Point at fraction t (0–1) of a polyline's length. */
function along(line: [number, number][], t: number): [number, number] {
  const seg = line.slice(1).map((p, i) => Math.hypot(p[0] - line[i][0], p[1] - line[i][1]));
  let rest = t * seg.reduce((s, l) => s + l, 0);
  for (let i = 0; i < seg.length; i++) {
    if (rest <= seg[i] || i === seg.length - 1) {
      const u = seg[i] ? Math.min(1, rest / seg[i]) : 0;
      return [line[i][0] + u * (line[i + 1][0] - line[i][0]), line[i][1] + u * (line[i + 1][1] - line[i][1])];
    }
    rest -= seg[i];
  }
  return line[line.length - 1];
}

/** Where each key's string crosses its bridge, keys at equal steps (as the model places them). */
export function bridgePoint(note: number): [number, number] {
  if (note >= MAIN_FROM) return along(TREBLE_BRIDGE, (108 - note) / Math.max(1, 108 - MAIN_FROM));
  return along(BASS_BRIDGE, (MAIN_FROM - 1 - note) / Math.max(1, MAIN_FROM - 1 - 21));
}

/** A plan-view projection: metres in board coordinates → SVG pixels. */
export function planView(scale: number, right: number, bottom: number) {
  return {
    X: (y: number) => right - y * scale,
    Y: (x: number) => bottom - x * scale,
    s: scale,
  };
}
