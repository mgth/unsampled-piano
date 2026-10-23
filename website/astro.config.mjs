// @ts-check
import { defineConfig } from 'astro/config';
import sitemap from '@astrojs/sitemap';
import mdx from '@astrojs/mdx';

// Single source of truth for the public origin. When the domain later changes
// update this value — it is the only
// place the domain is hardcoded; deployment is a plain FTPS mirror to the host
// (see scripts/deploy.sh), so DNS is handled entirely host-side.
// The site is served from the domain root, so `base` stays '/'.
const SITE = 'https://unsampled.mgth.fr';

// https://astro.build/config
export default defineConfig({
  site: SITE,
  trailingSlash: 'always',
  integrations: [sitemap(), mdx()],
  build: {
    // Emit `/docs/getting-started/index.html` so clean URLs work on a plain
    // static host (Apache with the shipped public/.htaccess) without rewrites.
    format: 'directory',
  },
});
