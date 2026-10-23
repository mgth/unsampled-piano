// Site-wide constants and shared links. Single source of truth for external
// URLs so they are never duplicated across components/pages.
export const SITE = {
  name: 'Unsampled Piano',
  tagline: 'A piano with no samples in it — every note computed from the physics, in real time.',
  description:
    'Unsampled Piano is a physically modelled piano in Rust: a felt hammer strikes stiff strings, the strings drive a spruce soundboard, the board radiates to two microphones in a room — computed live from MIDI, as a standalone app or a CLAP/VST3 plugin. No samples.',
  github: 'https://github.com/mgth/unsampled-piano',
  // The Ko-fi page the application's status bar links to.
  kofi: 'https://ko-fi.com/mgth',
  donate: 'https://github.com/sponsors/mgth',
  liberapay: 'https://liberapay.com/mgth/donate',
  license: 'GPL-3.0-or-later',
} as const;

/** A file of the repository on GitHub (main branch). */
export const repoFile = (path: string) => `${SITE.github}/blob/main/${path}`;
