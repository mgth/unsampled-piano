# Phase 5 — bruits mécaniques, préréglages, temps réel, calibration

État au 2026-10-21.

## Bruits mécaniques (`params::NoiseLaw`, `engine.rs`, `voice.rs`)

Aucun échantillon n'est utilisé. Chaque bruit est une impulsion injectée dans une structure déjà modélisée, qui le filtre et le rayonne physiquement :

| Bruit | Mécanisme | Ce qui le règle |
|---|---|---|
| Fond de touche | Demi-sinus de 3 ms dans la table, au point de chevalet de la touche (clavier → ceinture → table) | proportionnel à la vitesse du marteau |
| Chute d'étouffoir | Impulsion de quantité de mouvement au point de l'étouffoir : chaque mode k de chaque corde verticale reçoit Δq̇ = φ_k·I/M, projeté sur les modes couplés | vélocité de relâchement MIDI ; pour la pédale, vitesse de la pédale |
| Barre d'étouffoirs | Demi-sinus dans la table, au milieu du clavier, quand la pédale bouge ; au relâchement s'ajoutent les chutes de tous les étouffoirs | vitesse de la pédale |

La chute d'étouffoir excite les cordes, qui le transmettent ensuite à la table : le bruit prend donc la couleur des cordes qui s'arrêtent. Une corde à l'écoute reçoit l'impulsion dans son banc sympathique.

**Niveaux** (`cargo run --release -p piano-core --example noise_levels`), isolés par différence entre un rendu avec bruits et un rendu sans, par rapport à un C4 mezzo-forte :

| Bruit | Niveau |
|---|---|
| Fond de touche | −25 dB |
| Chute d'étouffoir | −25 dB |
| Barre (pédale enfoncée) | −26 dB |
| Barre (pédale relâchée, environ 70 étouffoirs qui retombent) | −17 dB |

- Ces niveaux sont *réglés*, pas mesurés : la littérature situe les bruits de toucher 20 à 30 dB sous la note. Une première version transmettait à la corde toute la quantité de mouvement du feutre (+4 dB au-dessus de la note), ce qui est irréaliste puisque l'énergie se dissipe surtout dans le feutre mou.
- Le contact doigt–touche, qui rend un « toucher frappé » audible (Goebl 2014), n'est pas modélisé. La touche n'existe pas en tant que corps.

## Préréglages (`piano-host`)

- Tout le modèle (`PianoModel`) se sérialise en TOML, grâce à la fonctionnalité `serde` de `piano-core` : le cœur reste sans dépendance par défaut.
- Un préréglage ne contient que ce qu'il change : les sections et champs absents gardent la valeur de l'instrument par défaut.
- La courbe de vélocité (vitesse du marteau de 0,4 à 4 m/s) fait désormais partie du modèle.

```bash
cargo run --release -p piano-render -- dump-preset presets/default.toml   # tous les paramètres
cargo run --release -p piano-render -- --preset presets/honky_tonk.toml note out/c4.wav --note 60
```

Exemples fournis :

| Préréglage | Ce qui change |
|---|---|
| `presets/default.toml` | instrument calé sur Salamander, 132 lignes |
| `honky_tonk.toml` | unisson de 12 à 18 cents, feutre dur, étouffoirs faibles |
| `baroque_415.toml` | A = 415 Hz, sans accordage étiré, feutre plus doux |
| `dry_close.toml` | micros à 40 cm de la table (+9 dB), sympathie réduite |

Les paramètres exposés correspondent aux familles de réglages d'un piano modélisé commercial : diapason et accordage étiré, inharmonicité, largeur d'unisson, dureté et exposant du feutre (harmonisation), courbe de vélocité, pertes des cordes, table (dimensions, raideurs, pertes, raideur série du chevalet), étouffoirs (position, force, demi-pédale, temps), sympathie, polarisation, bruits, position des micros.

## Application temps réel (`piano-play`)

- Audio : cpal 0.18 avec l'hôte **PipeWire natif** par défaut (JACK et ALSA disponibles avec `--host`) et la priorité temps réel du thread audio.
- MIDI : midir. Le premier port réel est choisi automatiquement, sinon `--midi <nom>`.
- File d'événements sans verrou (rtrb) : le rappel MIDI pousse, le rappel audio vide la file au début de chaque tampon.
- L'instrument est construit sur le fil principal (≈ 0,8 s), puis le rappel audio ne fait ni allocation ni verrou.
- La charge CPU est affichée chaque seconde.

```bash
cargo run --release -p piano-play -- --list
cargo run --release -p piano-play -- --midi "SL STUDIO" --buffer 128
cargo run --release -p piano-play -- --demo        # séquence intégrée, sans clavier
```

Mesures sur la machine de développement (Ryzen 9 9950X, PipeWire, 48 kHz), séquence de démo avec pédale et 88 voix, son coupé :

| Hôte | Tampon | Sous-alimentations | Charge moyenne | Charge max |
|---|---|---|---|---|
| ALSA (via PipeWire) | 256 | nombreuses | 18 % | 36 % |
| ALSA (via PipeWire) | 512 | aucune | 17 % | 25 % |
| PipeWire natif + priorité temps réel | 256 | aucune | 17 % | 22 % |
| PipeWire natif + priorité temps réel | 128 (2,7 ms) | aucune | 17 % | 21 % |

La charge est exprimée en part de la durée du tampon. Avec l'hôte ALSA, les sous-alimentations à 256 échantillons venaient du pont ALSA → PipeWire et de l'absence de priorité temps réel, pas du calcul.

Limite actuelle : les événements d'un tampon sont tous appliqués à son début. La gigue vaut donc au plus un tampon (2,7 ms à 128 échantillons). Les horodatages de midir permettraient de placer chaque événement à l'échantillon près.

## Calibration de bout en bout (`tools/calibrate.sh`)

Le script prend n'importe quelle banque d'échantillons nommée `<note>v<couche>` et produit en environ 47 s pour Salamander :
1. le relevé de la référence : partiels, B, décroissances, composantes ESPRIT, lois ajustées, conductance du chevalet ;
2. les rendus du modèle pour les mêmes notes et vélocités, éventuellement avec un préréglage ;
3. l'analyse de ces rendus avec exactement les mêmes outils ;
4. les comparaisons : dynamique, brillance, pente spectrale, fluctuations, double décroissance.

```bash
tools/calibrate.sh data/salamander/Samples out/calib [presets/mon_piano.toml]
```

Les recherches sur grille (`tools/grid_hammer.py`, `unison_grid.py`, `pol_grid.py`, `coupling_grid.py`, `board_grid.py`, `voicing_grid.py`) restent disponibles pour réajuster un groupe de paramètres.

## Ce qui reste

- **Plugin** (CLAP, puis VST3) : le moteur est déjà indépendant de tout cadre, il faut un emballage avec nice-plug ou clack. Pour VST3, les liaisons Rust imposent la GPLv3 ; CLAP peut rester sous licence permissive.
- **Interface graphique** des réglages, et reconstruction de l'instrument hors du thread audio quand un paramètre change. Pour l'instant, il faut un nouveau préréglage et un redémarrage.
- **Excès d'aigus au-dessus de 3 kHz** (phase 4, ouvert).
- **Second piano de référence** (Iowa MIS) pour séparer ce qui vient du modèle de ce qui vient de l'enregistrement Salamander.
- **Écoute comparative en aveugle** (MUSHRA) contre Salamander et le piano commercial.

## Retour d'écoute (2026-10-22) : « très étouffé, comme une sourdine » et « bruit de relâchement trop clair »

Premier essai au clavier (Novation SL Studio, `piano-play --buffer 128`). Les mesures ont trouvé quatre causes, toutes corrigées physiquement :

1. **Point de frappe du mauvais côté.** Le chevalet est en x = 0 et le marteau frappait à 0,12·L… *depuis le chevalet*. Le spectre des modes est identique (|sin(kπx0)| est symétrique), donc aucun calage spectral ne l'avait vu. Mais pendant le contact, le chevalet recevait quasi statiquement environ 88 % de la poussée du marteau, au lieu d'environ 12 % quand la frappe est près de l'agrafe. La table en faisait un « boum » grave : 64 à 92 % de l'énergie d'attaque tombait entre les partiels, contre 1 à 6 % pour Salamander. Désormais, le point de frappe et les étouffoirs se mesurent depuis l'agrafe.
2. **Rayonnement en baffle infini.** Une vraie table rayonne par ses deux faces. Il faut ajouter la face arrière en opposition de phase (intégrale de Rayleigh avec image à 0,6 m, gain 0,6, calés sur la pente spectrale) : le court-circuit acoustique atténue les modes graves. Erreur de pente : de 4,7 à 3,8 dB.
3. **Pertes de la table constantes en fréquence.** L'amortissement interne et le rayonnement croissent avec la fréquence : η = 0,02·(1 + f/700 Hz). Pour les modes de table très amortis, la réponse impulsionnelle exacte de l'oscillateur (facteur 1 + iσ/ω) remplace l'approximation en cosinus.
4. **Décroissances trop courtes** (prompt ×0,7, aftersound ×0,67 au médium ; ×0,6 et ×0,5 dans l'aigu). En phase 3, la correction de passivité avait triplé le couplage horizontal, et la loi des pertes propres de la phase 2 était surestimée. Recalage sur le système complet (`tools/decay_grid.py`) : couplage horizontal 0,03, couplage croisé à la limite de passivité (0,173), pertes propres ×0,45. Biais résiduels : médium ×1,03 et ×1,05 ; aigu ×0,81 et ×0,74.

Le marteau a ensuite été recalé sur la brillance *absolue* d'attaque, jusque-là seuls des rapports entre vélocités étaient calés (`tools/attack_grid.py`) : dureté 0,35, exposant p − 0,5, pente d'harmonisation 0,02 par touche. Erreur de brillance d'attaque (rms log du rapport modèle/Salamander) : de 0,62 à 0,31. Avant, le grave était 2 fois trop brillant et l'aigu 2 à 3 fois trop terne ; maintenant presque tout le clavier tombe entre 0,7 et 1,3. Seule l'octave du haut reste terne (A7 à 0,4).

**Bruit de relâchement trop clair.** La chute d'étouffoir était une impulsion instantanée, donc à large bande. Le feutre se pose maintenant en 4 ms : le spectre de l'impulsion est celui d'un demi-sinus, et le bruit devient un « tomp » sourd, 23 dB sous la note.

**Una corda.** Elle donne bien un timbre plus sombre et, dans cette version, un aftersound plus long, comme le prédit Weinreich. Depuis la recalibration de la tenue ([tenue.md](tenue.md)), le gain d'aftersound dépend de la note et devient nul en moyenne ; le test s'appelle désormais `una_corda_is_darker_and_keeps_its_sustain`. Mais le modèle ne reproduit *pas* la baisse de niveau d'un vrai piano : frapper 2 cordes au lieu de 3 ne réduit pas l'énergie transmise par le marteau dans notre modèle.

Après ces changements : gain de sortie +6 dB (pire cas à −3,2 dBFS) ; bruits à −22 / −23 dB sous la note ; relâchement de pédale à −14 dB.

## Salle et couvercle (2026-10-22)

Retour d'écoute : « le son reste un peu sec ». C'était attendu : le modèle s'arrêtait aux deux micros, comme dans une pièce parfaitement absorbante.

**Salle** (`room.rs`, section `[room]` du préréglage).
- Une pièce rectangulaire. Par défaut, une salle de concert de 40 × 28 × 16 m (≈ 18 000 m³, T60 ≈ 2,0 s dans le grave et 1,6 s dans l'aigu, distance critique ≈ 5,7 m). À 1,5 m, la réverbération arrive environ 12 dB sous le son direct, et les premiers échos après 30 à 60 ms. Préréglages : `small_hall.toml` (12 × 8 × 5 m, l'ancien défaut, perçu à l'écoute comme une « petite pièce très réverbérante ») et `studio.toml` (pièce traitée).
- La source est la pression rayonnée ramenée à 1 m, estimée à partir des micros.
- **Premières réflexions** : sources images jusqu'à l'ordre 3. Chaque réflexion a son retard exact, une atténuation √(1 − α) par mur et un passe-bas d'absorption qui se renforce avec le nombre de réflexions.
- **Queue de réverbération** : réseau de 16 lignes à retard (matrice de Householder), longueurs réparties autour du libre parcours moyen 4V/S.
  - Atténuation sur deux bandes donnant le T60 de Sabine : 1,1 s dans le grave, 0,8 s dans l'aigu (absorption de l'air comprise).
  - Le niveau est fixé physiquement : énergie réverbérée = (r/r_c)² × énergie directe, avec la distance critique r_c = 0,057·√(V/T60) ≈ 1,3 m.
  - Avec les micros à 1,5 m, la réverbération arrive environ 2 dB sous le son direct.
- Test : le T60 rendu tombe entre les deux T60 de Sabine.
- Coût : environ 1 % d'un cœur.
- La salle est coupée dans `piano-eval`, car les calibrations comparent l'instrument seul aux enregistrements ; `--room` la réactive.
- Préréglage `presets/dry.toml` : sans salle.

**Couvercle** (`lid_*` dans `[soundboard]`).
- Rectangle réfléchissant articulé sur le grand côté droit, ouvert vers le public.
- Pour chaque élément de table, la source image est le symétrique par rapport au plan du couvercle. Elle compte si le point de réflexion tombe sur le couvercle et si le micro est du côté de la table.
- Un micro placé derrière le couvercle est dans son ombre (transmission 0,1, sans diffraction).
- Par défaut, le couvercle est retiré : les micros calés sur Salamander sont suspendus au-dessus de la table.
- `presets/concert.toml` : couvercle à 35°, micros à 1,6 m devant l'ouverture. Pour ces micros, le couvercle ajoute en moyenne 2 dB (jusqu'à +4,7 dB dans le grave), avec les creux d'interférence direct/réfléchi propres à chaque note. Sans diffraction au bord du couvercle, ces creux sont plus marqués que dans la réalité.

**Pas encore modélisé : le reste du corps du piano.**
- La ceinture (le côté courbe) et le fond de caisse : apport surtout grave. La table est supposée encastrée dans une ceinture rigide.
- Le cadre en fonte, excité par les forces des cordes à l'agrafe et aux pointes d'attache : résonance métallique faible.
- La cavité d'air sous la table : modes graves.

Ces trois éléments relèvent de la même méthode : des modes, excités par les forces déjà calculées et rayonnant vers les micros.
