// The repository's presets, read at build time (../presets), so the figures
// that show a microphone set-up or a room draw the real files.
import fs from 'node:fs';
import path from 'node:path';
import { parse } from 'smol-toml';
import fr from '../data/presets.fr.json';
import type { Lang } from '../i18n';

// Astro runs dev/build from the website folder; the presets sit beside it.
const ROOT = path.resolve('../presets');

export interface Preset {
  /** File stem, e.g. "close". */
  name: string;
  /** Group folder, e.g. "Microphones". */
  group: string;
  /** First comment lines of the file. */
  summary: string;
  values: Record<string, any>;
}

function read(file: string, group: string): Preset {
  const text = fs.readFileSync(file, 'utf8');
  const summary = text
    .split('\n')
    .filter((l) => l.startsWith('#'))
    .map((l) => l.replace(/^#\s?/, ''))
    .join(' ');
  return { name: path.basename(file, '.toml'), group, summary, values: parse(text) };
}

/** Presets of a kind ("listening" or "instruments"), by group folder. */
export function presets(kind: 'listening' | 'instruments'): Preset[] {
  const dir = path.join(ROOT, kind);
  return fs
    .readdirSync(dir, { withFileTypes: true })
    .filter((d) => d.isDirectory())
    .flatMap((d) =>
      fs
        .readdirSync(path.join(dir, d.name))
        .filter((f) => f.endsWith('.toml'))
        .sort()
        .map((f) => read(path.join(dir, d.name, f), d.name)),
    );
}

export const preset = (kind: 'listening' | 'instruments', name: string) => {
  const p = presets(kind).find((p) => p.name === name);
  if (!p) throw new Error(`no ${kind} preset ${name}`);
  return p;
};

// French titles and summaries, by file stem (src/data/presets.fr.json).
const FR: Record<string, { title: string; summary: string } | undefined> = fr;

/** "concert_hall" → "Concert hall" ("Salle de concert" in French). */
export const presetTitle = (name: string, lang: Lang = 'en') =>
  (lang === 'fr' && FR[name]?.title) || (name.charAt(0).toUpperCase() + name.slice(1)).replace(/_/g, ' ');

/** The comment at the top of a preset, in `lang`. */
export const presetSummary = (p: Preset, lang: Lang = 'en') => (lang === 'fr' && FR[p.name]?.summary) || p.summary;
