# Unsampled Piano website

The public site and documentation of Unsampled Piano, <https://unsampled.mgth.fr>, built with
[Astro](https://astro.build) (same structure as the Omniphony and Little Big Mouse sites). The
built site is static, hosted on **o2switch** and published over **FTP**.

## Develop

```sh
npm install        # first time only
npm run dev        # http://localhost:4321
```

| Command | Action |
|---|---|
| `npm run dev` | Start the dev server |
| `npm run build` | Production build to `dist/` |
| `npm run preview` | Serve the built `dist/` locally |
| `npm run check` | Type-check + validate content frontmatter |
| `npm run params` | Regenerate `src/data/params.json` from the code (needs cargo) |
| `npm run demos` | Render the home page's audio clips again (needs cargo, ffmpeg, lame) |
| `npm run deploy` | Build, then upload `dist/` to o2switch over FTP |

## Where the content comes from

- **Parameters** — `src/data/params.json` is exported from the code by
  `crates/piano-ui/examples/param_reference.rs`: every parameter's path, interface page,
  live/rebuild kind, default, range and the two help texts the application shows, plus each
  page's per-key curves on the default instrument. Run `npm run params` after changing a
  parameter, its help or its range, and commit the JSON with the change. The diagrams read their
  values from it too, so they follow the default instrument.
- **Audio demos** — `public/media/*.mp3` and their waveforms in `src/data/demo-peaks.json` are
  rendered by the engine from the MIDI written by `scripts/demo_midi.py`: the openings of
  Debussy's *Clair de lune* and Chopin's *Revolutionary* Étude, cut from `scripts/midi/`, and a
  few pedal and velocity tests. `npm run demos` renders and encodes them again; commit the MP3s and the JSON together.
  Both MIDI files are Bernd Krüger's, from <http://www.piano-midi.de>, under
  [CC BY-SA 3.0 DE](https://creativecommons.org/licenses/by-sa/3.0/de/deed.en): `clair.mp3`,
  `etude.mp3` and `mics-*.mp3` are adaptations and carry the same licence, credited on the home page and in
  the MP3 tags.
- **Presets** — the figures and lists that show presets read `../presets` at build time.
- **Interface screenshots** — `src/assets/ui/<page>.png`, captured by the UI demo:
  `PIANO_UI_PAGE=Hammer PIANO_UI_SHOT=hammer.ppm cargo run -p piano-ui --example demo presets`,
  then converted to PNG.
- **Guides and parameter pages** — Markdown/MDX in `src/content/docs/`. A parameter page names its
  interface page in `page:` and lives in `docs/parameters/`; its parameter list, its screenshot
  and its table of contents entries are added by the route.

## Deploying to o2switch (FTP)

Deployment is manual and runs from your machine — no credentials are stored on GitHub. Copy
`.env.deploy.example` to `.env.deploy` (gitignored), fill in the FTP credentials and the
subdomain's docroot, then `npm run deploy`. Leave `FTP_DELETE=false` unless the remote directory
contains only this site.
