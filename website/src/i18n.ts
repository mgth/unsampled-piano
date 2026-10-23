// Languages of the site. English lives at the root, French under /fr/;
// every page exists in both, at the same path after the prefix. Strings of
// the shared chrome (header, footer, page furniture) are here; diagrams keep
// their own strings next to their drawing, and long texts are content files
// (src/content/docs for English, src/content/docs-fr for French).
import { SITE } from './site';

export type Lang = 'en' | 'fr';
export const LANGS: Lang[] = ['en', 'fr'];

/** "/docs/presets/" in `lang`: "/fr/docs/presets/" for French. */
export const localePath = (lang: Lang, path: string) => (lang === 'fr' ? `/fr${path === '/' ? '/' : path}` : path);

/** The language of a URL path, and the path without its prefix. */
export function splitPath(pathname: string): { lang: Lang; path: string } {
  const fr = pathname === '/fr' || pathname.startsWith('/fr/');
  const path = fr ? pathname.slice(3) || '/' : pathname;
  // The not-found pages switch language to the other home page.
  return { lang: fr ? 'fr' : 'en', path: path.startsWith('/404') ? '/' : path };
}

const UI = {
  en: {
    tagline: SITE.tagline,
    description: SITE.description,
    skip: 'Skip to content',
    titleSuffix: 'a physically modelled piano',
    navDownload: 'Download',
    navDocs: 'Docs',
    navParams: 'Parameters',
    navHome: 'home',
    toggleNav: 'Toggle navigation',
    langName: 'English',
    otherLang: 'Français',
    footerDocs: 'Documentation',
    footerParams: 'Parameter reference',
    footerRights: 'Licensed under',
    footerJoke: 'No samples were harmed',
    builtWith: 'Built with',
    docs: 'Docs',
    guides: 'Guides',
    parameters: 'Parameters',
    onThisPage: 'On this page',
    inTheApp: 'In the application',
    inTheAppCaption: (page: string) =>
      `The ${page} page of the interface, default instrument. Its sketch is drawn from the current settings; hover a parameter to light up the part it acts on, click its name for the help shown below.`,
    inTheAppAlt: (page: string) => `The ${page} page of the Unsampled Piano interface`,
    read: 'Read',
    // Parameter cards
    live: 'live',
    rebuild: 'rebuild',
    liveTitle: 'Applied by the running engine, and automatable as a plugin parameter',
    rebuildTitle: 'Shapes the precomputed instrument: changing it rebuilds it in the background, without cutting held notes',
    sliderCurated: 'Slider range in the interface',
    sliderGuessed: 'Slider range around the default (not curated)',
    default: 'default',
    on: 'on',
    off: 'off',
    inTheModel: 'in the model',
    // 404
    notFoundTitle: 'Page not found',
    notFoundHead: 'Wrong note',
    notFoundSub: "No string is tuned to that page. Let's get you back in tune.",
    home: 'Home',
    documentation: 'Documentation',
  },
  fr: {
    tagline: 'Un piano sans le moindre échantillon — chaque note calculée à partir de la physique, en temps réel.',
    description:
      'Unsampled Piano est un piano par modélisation physique, en Rust : un marteau de feutre frappe des cordes raides, les cordes font vibrer une table d’harmonie en épicéa, la table rayonne vers deux micros dans une salle — calculé en direct depuis le MIDI, en application autonome ou en plugin CLAP/VST3. Sans échantillons.',
    skip: 'Aller au contenu',
    titleSuffix: 'un piano par modélisation physique',
    navDownload: 'Télécharger',
    navDocs: 'Docs',
    navParams: 'Paramètres',
    navHome: 'accueil',
    toggleNav: 'Ouvrir le menu',
    langName: 'Français',
    otherLang: 'English',
    footerDocs: 'Documentation',
    footerParams: 'Référence des paramètres',
    footerRights: 'Sous licence',
    footerJoke: 'Aucun échantillon n’a été maltraité',
    builtWith: 'Construit avec',
    docs: 'Docs',
    guides: 'Guides',
    parameters: 'Paramètres',
    onThisPage: 'Sur cette page',
    inTheApp: 'Dans l’application',
    inTheAppCaption: (page: string) =>
      `La page ${page} de l’interface, instrument par défaut. Son croquis est dessiné d’après les réglages en cours ; survolez un paramètre pour éclairer la pièce sur laquelle il agit, cliquez sur son nom pour son aide (en anglais dans l’application, traduite ci-dessous).`,
    inTheAppAlt: (page: string) => `La page ${page} de l’interface d’Unsampled Piano`,
    read: 'Lire',
    live: 'direct',
    rebuild: 'reconstruction',
    liveTitle: 'Appliqué par le moteur en marche, et automatisable comme paramètre du plugin',
    rebuildTitle: 'Façonne l’instrument précalculé : le changer le reconstruit en arrière-plan, sans couper les notes tenues',
    sliderCurated: 'Plage du curseur dans l’interface',
    sliderGuessed: 'Plage du curseur autour de la valeur par défaut (non choisie)',
    default: 'par défaut',
    on: 'activé',
    off: 'désactivé',
    inTheModel: 'dans le modèle',
    notFoundTitle: 'Page introuvable',
    notFoundHead: 'Fausse note',
    notFoundSub: 'Aucune corde n’est accordée sur cette page. Revenons au diapason.',
    home: 'Accueil',
    documentation: 'Documentation',
  },
} as const;

export type UiStrings = (typeof UI)['en'];
export const ui = (lang: Lang): UiStrings => UI[lang] as UiStrings;

/** The language of the page being rendered. */
export const langOf = (url: URL): Lang => splitPath(url.pathname).lang;
