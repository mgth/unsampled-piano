// The docs collection of a language: English in src/content/docs, French
// in src/content/docs-fr, same ids.
import { getCollection, type CollectionEntry } from 'astro:content';
import type { Lang } from '../i18n';

export type DocEntry = CollectionEntry<'docs'> | CollectionEntry<'docsFr'>;

export async function docsOf(lang: Lang): Promise<DocEntry[]> {
  const list: DocEntry[] = lang === 'fr' ? await getCollection('docsFr') : await getCollection('docs');
  return list.sort((a, b) => a.data.order - b.data.order);
}
