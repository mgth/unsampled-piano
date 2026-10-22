# Cadre en fonte et ceinture

État au 2026-10-22. Code : `structure.rs`, `board.rs` (sorties supplémentaires), `engine.rs`, `voice.rs`.

## Ce que font ces pièces dans un vrai piano

- **Le cadre** (la plaque de fonte dorée) encaisse la tension des cordes, environ 20 tonnes. Chaque corde s'appuie sur la table au chevalet et sur le cadre à l'autre bout, à l'agrafe. Elle pousse donc aussi, en vibrant, sur le cadre.
- **La ceinture** (la caisse en érable lamellé) tient la table d'harmonie par son pourtour. Quand la table vibre, elle tire et pousse sur ses bords, et la ceinture reçoit ces efforts.

Ces deux pièces sont lourdes et rigides. Elles bougent beaucoup moins que la table et rayonnent donc peu. Leur rôle principal est mécanique : maintenir la tension et fixer le bord de la table. Le modèle le traduit déjà, puisque la table y est encastrée. Leur contribution sonore directe est faible.

## Modèle

### Cadre (`FrameParams`)

- **Excitation.** Chaque corde calcule la force qu'elle exerce sur son agrafe, −T·∂y/∂x à cette extrémité. Les poids sont ceux du chevalet multipliés par (−1)^(k+1) : le fondamental tire sur les deux extrémités dans le même sens, le deuxième partiel en sens opposés. Pendant le contact du marteau, cette force contient aussi la part quasi statique de la poussée du marteau. Comme le marteau frappe près de l'agrafe, c'est surtout l'agrafe qui la reçoit.
- **Plaque.** C'est une plaque isotrope encastrée de 1,75 × 1,3 m, calculée par le même code que la table : modes de Warburton et rayonnement de Rayleigh vers les micros, depuis 8 cm au-dessus de la table.
  - Masse surfacique 110 kg/m², soit environ 150 kg de fonte, trous compris.
  - Rigidité choisie pour un premier mode vers 155 Hz ; 247 modes jusqu'à 12 kHz.
  - Le cadre est environ 40 fois moins mobile que la table.
- **Rayonnement.** C'est une plaque ajourée. Seule la fraction métallique rayonne (`solidity` 0,4), et ses deux faces se court-circuitent par les trous (`back_gain` 0,9 à 5 cm).
- **Amortissement.** η = 1 %. La fonte nue est à 0,1–0,5 %, mais une fois montée elle est boulonnée au sommier et à la ceinture, et chargée par 230 cordes et leurs feutres. **C'est une estimation**, sans mesure publiée.
- **Agrafes.** Elles sont placées à 5 cm du bord avant, au-dessus du sommier boulonné. Elles font face au point de chevalet de chaque région, puisque les cordes vont selon x. Le croisement des basses est ignoré.
- **Sens unique.** Le mouvement du cadre ne renvoie rien aux cordes, ce qui est négligeable vu sa faible mobilité. Les cordes « à l'écoute » (résonance sympathique) ne l'excitent pas.

### Ceinture (`RimParams`)

- **Efforts de bord.** Pour chaque mode de table en forme sinus, le modèle prend le cisaillement de Kirchhoff le long des bords et les forces de coin. Leur somme vaut exactement ρh·ω_ss²·∫Φ ; c'est vérifié par un test, à 0,2 % près, pour les modes pairs comme impairs. Elles sont ensuite multipliées par (ω_n/ω_ss)², pour que la force nette corresponde à la fréquence encastrée utilisée par la table.
  - La force transmise suit le déplacement modal et la force d'amortissement : q + 2σ/ω²·q̇.
  - La banque de la table obtient ce terme par des poids complexes sur ses états, sans état supplémentaire.
- **Anneau.** La ceinture est un mur de 30 × 7,5 cm en érable lamellé (E 12 GPa, G 1 GPa, 650 kg/m³, η 2 %). Il est modélisé comme un anneau de même périmètre, en flexion hors plan couplée à la torsion, selon Love :
  - pour chaque ordre k ≤ 8, un problème 2 × 2 en déplacement vertical W et rotation B du mur, inertie de rotation comprise ;
  - chaque ordre donne une branche « flexion », de 43 Hz (k = 2) à 680 Hz (k = 8), et une branche « torsion », de 660 Hz à 3 kHz ;
  - les modes rigides (le piano qui bouge sur ses pieds) sont écartés.
- **Couplage.** La table tire sur le bord intérieur du mur, à t/2 de son axe. Elle excite donc à la fois la flexion et la torsion.
  - La banque de la table fournit 17 projections (k, cos/sin) de ses efforts de bord.
  - Seuls ses 93 modes sous 2 kHz y contribuent : les modes plus courts projettent peu sur les grandes ondes de la ceinture.
- **Rayonnement.** Quatre faces discrétisées tous les 4 cm, avec un facteur d'obliquité (1 + cos θ)/2.
  - Faces extérieure et intérieure : rotation du mur.
  - Dessus et dessous : mouvement vertical.
  - La face intérieure, à moitié sous la table, est pondérée par 0,5.
  - Pas de réflexion sur le couvercle.
- **Sens unique.** La ceinture ne fait pas bouger les bords de la table.

## Niveaux

Mesurés avec `cargo run --release -p piano-core --example structure_levels`, à sec, sans salle ni bruits, vélocité 100. Chaque contribution est isolée par différence de rendus et comparée à la note sans cadre ni ceinture. Les trois fenêtres sont 0–0,2 s | 0,5–1,5 s | 2–4 s.

| Note | Cadre | Ceinture |
|---|---|---|
| E1 (28) | −34 · −27 · −27 dB | −41 · −38 · −38 dB |
| A2 (45) | −36 · −42 · −43 dB | −34 · −46 · −45 dB |
| C4 (60) | −31 · −34 · −28 dB | −40 · −38 · −36 dB |
| C5 (72) | −28 · −20 · −23 dB | −39 · −39 · −39 dB |
| C6 (84) | −18 · −13 · −13 dB | −38 · −35 · −35 dB |
| C7 (96) | −15 · −25 · −28 dB | −40 · −46 · −42 dB |

Ces valeurs ont été remesurées après la recalibration de la tenue ([sustain.md](sustain.md)). Avant, le cadre était 3 à 10 dB plus bas dans l'aigu. Depuis, la table, plus raide sous l'aigu, rayonne moins les cordes aiguës, qui gardent leur énergie plus longtemps. Le cadre reste inchangé : en proportion, il pèse donc davantage.

- **Le cadre** s'entend dans l'aigu comme un léger éclat métallique, jusqu'à −13 dB à C6. Quand un partiel tombe sur une résonance du cadre, celui-ci peut prolonger un peu ce partiel.
- **La ceinture** reste 36 à 45 dB sous la note : un fond grave à peine perceptible.
- `structure_transfer` compare la fonction de transfert du cadre (de l'agrafe au micro) à celle de la table (du chevalet au micro), par octave. Le cadre est 40 à 50 dB en dessous sous 250 Hz, et 17 à 23 dB en dessous au-dessus de 1 kHz.

Une première version donnait au cadre η = 0,4 %, toute sa surface rayonnante et des agrafes à 15 cm du bord. Le cadre couvrait alors la note dans l'aigu (jusqu'à +20 dB en fin de son), ce qu'aucun piano ne fait. Les valeurs retenues sont plus réalistes, mais **ni le niveau du cadre ni celui de la ceinture ne sont calibrés** : aucun enregistrement à notre disposition ne les isole.

Pour les entendre, `gain` (1 = physique) les amplifie sans rien changer d'autre. Par exemple `[frame] gain = 10` et `[rim] gain = 30`.

## Coût

Mesuré avec `stress_cost` (88 voix, pédale) :

| Configuration | Charge d'un cœur |
|---|---|
| Sans cadre ni ceinture | 22–23 % |
| Avec la ceinture | +1 % |
| Avec le cadre | +5 % |
| Tout | 28 % |

- **Cadre.** L'essentiel de son coût vient de la seconde sortie de chaque banque de cordes (la force à l'agrafe). Sa propre banque (247 modes) est négligeable. Sans cadre, les cordes ne calculent pas cette sortie.
- **Ceinture.** Les 17 projections ne portent que sur les 6 premiers paquets de modes de la table. Elles ont demandé deux corrections :
  - ses paramètres sont passés en vrais arguments de fonction : regroupés dans une structure, ils empêchaient la vectorisation, faute de garantie de non-recouvrement ;
  - les sommes de voies sont faites en arbre.
- Construction de l'instrument : inchangée à 0,1 s près.

## Correction en passant

Le signe de la force à l'agrafe était inversé (indices de partiels comptés à partir de 1). Le rendu n'était pas touché, puisque cette force n'était encore utilisée nulle part.

## Ce qui manque

- **Réaction sur les cordes.** Le cadre et la ceinture ne réagissent ni sur les cordes ni sur la table. Une ceinture souple abaisserait un peu les premiers modes de la table et en augmenterait l'amortissement.
- **Contributions non modélisées.** Les forces longitudinales des cordes sur le cadre (partiels « fantômes »), la transmission cadre ↔ ceinture par les boulons, et les chocs de la mécanique sur la ceinture : les bruits de fond de touche passent encore par la table.
- **Géométrie simplifiée.**
  - La ceinture est un anneau équivalent, pas la forme courbe réelle avec ses barres et ses pieds.
  - Le cadre est une plaque pleine équivalente, pas un réseau de barres.
- **Cavité d'air.** L'air entre la table et le sol, ou le dessous du piano, n'est pas modélisé. C'est probablement la contribution « de caisse » la plus audible qui reste.
