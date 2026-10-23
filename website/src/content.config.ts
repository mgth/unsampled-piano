import { defineCollection } from 'astro:content';
import { glob } from 'astro/loaders';
import { z } from 'astro/zod';

// Docs are plain Markdown files in src/content/docs/. A guide is just a new
// `.md` with this frontmatter — the sidebar, the /docs card grid and the
// homepage docs section derive from the collection. A parameter page lives in
// docs/parameters/ and names its interface page in `page`: the parameters
// themselves come from src/data/params.json, generated from the code.
const docs = defineCollection({
  loader: glob({ pattern: '**/*.{md,mdx}', base: './src/content/docs' }),
  schema: z.object({
    title: z.string(),
    description: z.string(),
    // Drives ordering in the sidebar and card grids.
    order: z.number(),
    // Line-icon key for the docs cards (see components/DocIcon.astro).
    icon: z.string().optional(),
    // Sidebar group.
    section: z.enum(['Guides', 'Parameters']).default('Guides'),
    // Parameter pages: the interface page whose parameters are listed.
    page: z.string().optional(),
  }),
});

export const collections = { docs };
