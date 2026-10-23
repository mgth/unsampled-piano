// The instrument's parameters, as exported from the code by
// `npm run params` (crates/piano-ui/examples/param_reference.rs): every
// path, its interface page, live or rebuild, default, range and help, and
// each page's per-key curves evaluated on the default instrument.
import data from '../data/params.json';
import { repoFile } from '../site';

export type Scalar = number | boolean;

export interface Param {
  path: string;
  id: number;
  page: string;
  live: boolean;
  type: 'float' | 'int' | 'bool';
  default: Scalar;
  label: string;
  unit: string;
  min: number;
  max: number;
  log: boolean;
  curated: boolean;
  what: string | null;
  how: string | null;
}

export interface Curve {
  title: string;
  unit: string;
  log: boolean;
  points: [number, number][];
}

export const PARAMS = data.params as Param[];
export const PAGES = data.pages as { title: string; curves: Curve[] }[];

const BY_PATH = new Map(PARAMS.map((p) => [p.path, p]));

/** Default value of a parameter of the default instrument. */
export function def(path: string): number {
  const p = BY_PATH.get(path);
  if (!p) throw new Error(`unknown parameter ${path}`);
  return typeof p.default === 'boolean' ? Number(p.default) : p.default;
}

export function curve(title: string): Curve {
  for (const page of PAGES) {
    const c = page.curves.find((c) => c.title === title);
    if (c) return c;
  }
  throw new Error(`unknown curve ${title}`);
}

/** Value of a per-key curve at a MIDI note (or velocity). */
export const curveAt = (title: string, x: number) => curve(title).points.find((p) => p[0] === x)![1];

/** "soundboard.mics[1][2]" → "soundboard.mics". */
export const stripIndices = (path: string) => path.replace(/\[\d+\]/g, '');

/** URL fragment of a parameter's entry. */
export const anchor = (path: string) => stripIndices(path).replace(/[^a-z0-9]+/gi, '-').replace(/^-|-$/g, '');

const NAMES = ['C', 'C♯', 'D', 'D♯', 'E', 'F', 'F♯', 'G', 'G♯', 'A', 'A♯', 'B'];
/** MIDI note → "F♯6" (middle C is C4). */
export const noteName = (n: number) => `${NAMES[n % 12]}${Math.floor(n / 12) - 1}`;

/** Parameters stored as MIDI notes, shown with their note name. */
export const isNote = (p: Param) =>
  p.type === 'int' && /note|_from$|_until$/.test(p.path.split('.').pop() ?? '');

/** A number for display: few significant digits, engineering notation for extremes. */
export function fmt(x: number): string {
  if (x === 0) return '0';
  const a = Math.abs(x);
  if (a >= 1e5 || a < 1e-3) {
    const e = Math.floor(Math.log10(a));
    const m = x / 10 ** e;
    return `${+m.toFixed(2)}·10${String(e).replace(/-/g, '⁻').replace(/\d/g, (d) => '⁰¹²³⁴⁵⁶⁷⁸⁹'[+d])}`;
  }
  return String(+x.toPrecision(4));
}

export function fmtValue(p: Param, v: Scalar = p.default): string {
  if (typeof v === 'boolean') return v ? 'on' : 'off';
  if (isNote(p)) return `${v} (${noteName(v)})`;
  return fmt(v) + (p.unit && !p.unit.startsWith('/') ? ` ${p.unit}` : p.unit);
}

/** Help text → HTML: escaped, `code` spans, repository files linked. */
export function helpHtml(text: string): string {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/`([^`]+)`/g, '<code>$1</code>')
    .replace(/\b((?:docs|tools)\/[\w.-]+\.(?:md|py|sh))/g, (f) => `<a href="${repoFile(f)}" rel="noopener">${f}</a>`);
}

/** One entry of the reference: a parameter, or an array of them sharing their help. */
export interface Entry {
  key: string;
  group: string;
  params: Param[];
}

/** A page's parameters, grouped as the interface does (sub-section = path before the last '.'), switches first. */
export function pageEntries(page: string): { group: string; entries: Entry[] }[] {
  const entries = new Map<string, Entry>();
  for (const p of PARAMS.filter((p) => p.page === page)) {
    const key = stripIndices(p.path);
    const group = key.includes('.') ? key.slice(0, key.lastIndexOf('.')) : 'general';
    if (!entries.has(key)) entries.set(key, { key, group, params: [] });
    entries.get(key)!.params.push(p);
  }
  const groups = new Map<string, Entry[]>();
  for (const e of entries.values()) {
    if (!groups.has(e.group)) groups.set(e.group, []);
    groups.get(e.group)!.push(e);
  }
  const isSwitch = (e: Entry) => e.params[0].type === 'bool';
  return [...groups.entries()]
    .sort(([a], [b]) => (a === 'general' ? -1 : b === 'general' ? 1 : a.localeCompare(b)))
    .map(([group, list]) => ({
      group,
      entries: list.sort((a, b) => Number(isSwitch(b)) - Number(isSwitch(a)) || a.key.localeCompare(b.key)),
    }));
}

const GROUP_TITLES: Record<string, string> = {
  general: 'General',
  velocity: 'Velocity',
  damper: 'Dampers',
  sympathetic: 'Sympathetic resonance',
  losses: 'Losses',
  scale: 'Scale (string lengths)',
  unison: 'Unisons',
  polarization: 'Polarisations',
  nonlinear: 'Nonlinearity (phantom partials)',
  inharmonicity: 'Inharmonicity',
  stretch: 'Stretch tuning',
  soundboard: 'Soundboard',
  bridge: 'Bridge (without a soundboard)',
  noise: 'Mechanical noises',
  frame: 'Cast-iron frame',
  rim: 'Rim',
  room: 'Room',
};
export const groupTitle = (g: string) => GROUP_TITLES[g] ?? g;

export const COUNTS = {
  total: PARAMS.length,
  live: PARAMS.filter((p) => p.live).length,
};
