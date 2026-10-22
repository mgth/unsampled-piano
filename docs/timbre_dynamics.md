# Changement de timbre avec la nuance

État au 2026-10-22.

## Le phénomène

Sur un vrai piano, une note jouée fort n'est pas seulement plus forte : elle est plus brillante. Le feutre du marteau durcit beaucoup plus vite qu'il ne s'écrase.

- **Jeu doux** : le marteau reste dans la couche souple du feutre. Le contact est long (environ 4 ms pour do4) et n'excite que le bas du spectre.
- **Jeu fort** : il atteint la partie dure. Le contact est bref (1,5 à 2 ms) et excite toutes les harmoniques.

Dans le grave, une seconde cause s'ajoute : la non-linéarité de la corde elle-même. Une forte amplitude modifie la tension, ce qui crée des partiels « fantômes » et le grondement du fortissimo.

## Mesure

`tools/hammer_score.py` a une nouvelle mesure, la pente (`slope`). On prend le gain ff − pp des partiels 6 à 12, moins celui des partiels 1 et 2, à l'attaque (0–100 ms). La mesure ne porte que sur le grave et le médium : au-delà, les partiels 6 et plus dépassent 6 kHz et se perdent dans le bruit des enregistrements.

| Note | Salamander | Iowa | Commercial | Modèle avant |
|---|---|---|---|---|
| do2 | +0,7 | +8,0 | +5,8 | +2,5 |
| do3 | +5,0 | +23,7 | +14,4 | +4,8 |
| do4 | +14,4 | +27,3 | +24,6 | +8,8 |
| do5 | +17,2 | +21,5 | +20,0 | +16,2 |

Le contact du do4 du modèle ne raccourcissait que de 30 % de pp à ff (2,8 → 2,0 ms) : le feutre était trop dur aux faibles écrasements.

## Changement : un feutre plus souple au départ, plus non linéaire

Grille `tools/hammer_grid.sh` (environ 50 combinaisons) :

| Paramètre | Avant | Après |
|---|---|---|
| `hardness` | 0,7 | 0,1 |
| `hardness_slope` | 0,05 | 0,02 |
| `p_offset` | +0,5 | +2,0 (p ≈ 5,7–7) |
| `hysteresis` | 0,25 | 0,6 |

L'exposant dépasse les mesures d'un feutre homogène (2 à 5). Il tient lieu d'un marteau multicouche, avec un feutre de surface souple sur une sous-couche dure : sa raideur croît plus vite qu'une seule loi de puissance. Un modèle explicite à deux couches serait plus juste.

## Résultat

| | Avant | Après | Références |
|---|---|---|---|
| contact do4, pp → ff | 2,8 → 2,0 ms | 3,7 → 2,3 ms | ≈ 4 → 1,5–2 ms |
| pente, médium | +8,8 dB | +12,8 dB | +17,4 dB |
| pente, grave | +1,9 dB | +2,5 dB | +6,1 dB |
| dynamique ff − pp, aigu | 9,1 dB | 11,4 dB | 12,2 dB |
| score marteau | 2,24 | 1,73 | — |

Volume inchangé : la Campanella culmine à −15 dBFS, le Chopin à −18,7 dBFS.

## Ce qui reste

- **Grave** : le feutre n'y change presque rien (+2,5 dB contre +6), car la corde lourde fixe la durée du contact. Il faut la non-linéarité géométrique de la corde : modulation de tension, modes longitudinaux, partiels fantômes.
- **Médium** : encore 4 à 5 dB d'éclaircissement en moins que les références.
- **Centroïde de l'attaque** : son rapport ff/pp bouge peu (×1,14 contre ×1,4–1,5). Il est dominé par les premiers partiels et par le filtrage de la table.

## Éclaircissement du grave (2026-10-22)

Les partiels fantômes ([nonlinearity.md](nonlinearity.md)), à leur niveau mesuré, n'apportaient que +0,1 dB d'éclaircissement. La cause principale est ailleurs.

Le marteau du grave frappe une corde lourde : environ 0,03 kg/m, deux cordes pour do2. Avec la dureté de feutre du médium, il ne s'enfonce que d'environ 1 mm, même ff. Le feutre ne filtre alors qu'au-dessus de 2 kHz. Entre 150 Hz et 1,5 kHz, où vivent les partiels 6 à 12 du grave, le contact est un choc linéaire entre la masse du marteau et la corde : sa forme ne dépend pas de la vitesse. Les vrais marteaux du grave sont gros et moelleux, et leur feutre filtre plus bas, d'autant moins que la frappe est forte.

Changements :
- `hardness_slope_bass` = 0,10, une pente de dureté propre au grave (Q0 × e^{0,10·(touche − 40)} sous do4) ;
- vélocité 0,5–3,5 m/s.

Les durées de contact restent physiques : 11 ms pour A0 pp, contre 9,4 ms avant, dominées par l'aller-retour vers l'agrafe ; 7 ms pour do2.

| | Avant | Après | Références |
|---|---|---|---|
| pente, grave | +2,6 dB | +5,5 dB | +6,1 (de +0,7 à +8) |
| pente, médium | +12,9 dB | +15,5 dB | +17,4 |
| dynamique ff − pp, grave | 6,6 dB | 8,1 dB | 10,0 (Salamander et piano commercial) |
| dynamique ff − pp, médium | 7,5 dB | 9,1 dB | 10,1 |
| dynamique ff − pp, aigu | 11,4 dB | 13,6 dB | 12,2 |
| score marteau | 1,73 | 1,79 | — |

L'attaque du médium est un peu plus sombre (partiels 2 à 6 de −1 à −2 dB). Le score écarte désormais la dynamique du suraigu (C7–C8) : ces cordes sans étouffoir résonnent encore des notes précédentes dans une séquence enregistrée, et leurs pics pp ne sont pas les leurs. Une vélocité plus large (0,4–4,0) atteint la cible au grave et au médium, mais élargit trop l'aigu (16,8 dB contre 12,2). Durcir les marteaux de l'aigu ne la corrige pas.

Reste :
- **Fondamentale du grave à l'attaque**, encore trop forte face aux partiels 2 à 6. Les références les mettent 10 à 30 dB au-dessus d'elle ; c'est ce qui creuse la moyenne du partiel 4 (−7,7 contre +3 dB).
- **Centroïde ff/pp** : ×1,12 au grave, contre ×1,2 à ×2,1 dans les références.

## Révision : marteau à deux couches (voir [presence.md](presence.md))

Le feutre très souple et très non linéaire de cette page coupait les aigus de l'attaque (−63 dB à 4–8 kHz au grave contre −44). Il est remplacé par une surface souple (dureté 0,2, p de Stulov) sur un cœur dur. `hardness_slope_bass` revient à 0,02 et `p_bass` à 0. L'éclaircissement devient +4,0 dB au grave et +13,3 au médium, contre +5,5 et +15,5 avec le feutre de cette page, et l'attaque retrouve ses aigus.
