# Non-linéarité géométrique des cordes (partiels fantômes)

État au 2026-10-22. Code : `params::StringNonlinearity`, `note_model::Longitudinal`, `Voice::nl_start` et `Voice::nl_run`.

## Physique

Une corde qui vibre fort s'allonge, d'après Bank & Sujbert (2005). Sa pente vaut y' = Σ a_k·cos(kπx/L), avec a_k = (kπ/L)·q_k. L'étirement pousse le mode longitudinal n (forme sin(nπx/L), masse modale μL/2) avec la force

F_n = −(EA·nπ/8)·(Σ_{j+k=n} a_j·a_k + 2·Σ_k a_k·a_{k+n}),

et ce mode tire sur le chevalet avec EA·(nπ/L)·ξ_n. Les produits de partiels donnent des fréquences somme et différence : ce sont les « partiels fantômes », quelques hertz sous chaque partiel transversal à cause de l'inharmonicité. Les résonances longitudinales tombent entre 14 et 20 fois f0. L'effet croît comme le carré de l'amplitude : nul en pp, marqué en ff, surtout dans le grave.

- **EA** est déduit de l'inharmonicité déjà calibrée : B = π³·E·d⁴/(64·T·L²), d'où EA = E·π·d²/4 avec E = 200 GPa. On obtient une âme de 0,9 à 1,4 mm, et un premier mode longitudinal à 544 Hz pour A0 et 1,1 kHz pour E2, soit 14 à 20 fois f0. Giordano et Conklin mesurent 15 à 20 fois.
- **Sens unique** : le mouvement transversal reste linéaire, comme chez Bank & Sujbert. La modulation de hauteur en ff n'est pas modélisée.
- **Transmission au chevalet** : la force longitudinale tire le chevalet dans le plan de la table. La table ne fléchit que par le moment au pied du chevalet, qui croît avec son nombre d'onde. Le modèle applique donc un passe-haut du premier ordre à 1 kHz, puis le couplage `coupling` = 0,3. Sans passe-haut, la modulation quasi statique de la tension s'ajoutait aux premiers partiels et assourdissait le grave.

## Réalisation temps réel

La voix lit, en début de segment libre (hors contact du marteau), le déplacement vertical moyen de chaque partiel à partir des états du banc de modes couplés. Elle le fait ensuite tourner avec le pôle du mode principal, sans toucher au noyau SIMD.

Par échantillon : produits des partiels, avec 8 accumulateurs vectorisables, et résonateurs longitudinaux.

Bornes du calcul :
- touches jusqu'à E4, partiels sous 5 kHz (au plus 48), modes longitudinaux sous 8 kHz ;
- montée de 2 ms après le contact ;
- calcul suspendu sous 0,1 mN de force estimée.

## Calibration

`tools/phantom_levels.py` mesure le niveau des pics sous chaque partiel n = 8..30 en ff, par rapport au partiel n, en médiane sur les touches C1 à A2. Chaque piano est mesuré avec sa propre inharmonicité.

| | Salamander | Iowa | Commercial | Modèle sans | Modèle (`coupling` 0,3) |
|---|---|---|---|---|---|
| fantômes (médiane) | −28,7 dB | −26,0 dB | −31,4 dB | −38,8 dB (plancher) | −27,9 dB |

Avec `coupling` = 1, les fantômes du modèle étaient environ 10 dB trop forts.

## Effet et coût

- **Éclaircissement pp→ff du grave** (pente partiels ≥ 6 contre 1–2) : +2,5 → +2,6 dB, pour +6 dB dans les références. Au niveau réel des références, les fantômes ne suffisent pas à expliquer cet éclaircissement : une autre part reste à trouver (marteau du grave, rayonnement).
- **Score marteau** : 1,73 → 1,70.
- **CPU** :
  - pire cas `stress_cost` (88 voix ff, pédale) : 28 → 37 % d'un cœur ;
  - Rachmaninov : moyenne 23,4 → 25,2 %, pire buffer inchangé (environ 55 %).

Extraits d'écoute : `out/nonlineaire/` (la1 ff et 45 s du Rachmaninov, avec et sans).

## Ce qui reste

- Glissement de hauteur en ff (modulation de tension sur les modes transversaux).
- Fantômes pendant le contact du marteau : les 2 à 5 premières ms ne sont pas calculées.
- Origine du reste de l'éclaircissement du grave.
