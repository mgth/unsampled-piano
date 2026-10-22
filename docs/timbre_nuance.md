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
