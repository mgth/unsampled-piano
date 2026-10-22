# Brillance de l'attaque et de la tenue

État au 2026-10-22.

## Constat

Pendant la tenue (0,5–1 s), les partiels 2 à 12 du médium étaient 5 à 14 dB sous la moyenne des trois références (Salamander, Iowa, piano modélisé commercial). Le profil des partiels semblait montrer un déficit encore plus fort dans l'aigu.

## Mesure

Le profil par partiel ne suffit pas dans l'aigu. Sa fenêtre de recherche suppose une inharmonicité de 4e-4, alors que l'aigu dépasse 2e-3. Au-delà du partiel 8, elle tombe entre les partiels, sur le bruit de fond des enregistrements. Deux outils, par bandes d'octave, remplacent cette mesure :

- **`tools/band_decay.py`** donne les niveaux par bande d'octave dans des fenêtres de 0,4 s. Les instants se choisissent avec `TIMES`.
- **`tools/band_score.py`** donne, par registre, l'écart du modèle à la moyenne des références dans les bandes de 1 à 8 kHz, à l'attaque (0,05 s) et pendant la tenue (0,6 s). Il donne aussi l'écart quadratique moyen. La bande 8 kHz de l'aigu est exclue, car c'est le bruit des enregistrements de référence.

Écarts du modèle de départ, en dB (bandes 1, 2, 4 et 8 kHz) :

| Registre | Attaque | Tenue |
|---|---|---|
| grave | −4 / −9 / −3 / +8 | −8 / −6 / 0 / −4 |
| médium | −3 / −8 / −16 / −11 | −6 / −7 / −15 / −17 |
| aigu | −4 / −2 / −2 / (−9) | 0 / −1 / −1 / (−20) |

Écart quadratique moyen : 7,8 dB.

Constats :

- **L'aigu est juste**, sauf au-dessus de 8 kHz.
- **Le grave et le médium manquent d'aigus dès l'attaque.** Entre l'attaque et la tenue, leurs bandes décroissent comme celles des références. Les décroissances ESPRIT par bande de fréquence sont même un peu longues dans le modèle.

La tenue est donc terne parce que l'attaque l'est. Ni les pertes ni le couplage au chevalet ne sont en cause : l'étalement de la force sur le chevalet n'y change rien (`bridge_spread` 0 à 0,3), ni la position des micros.

## Diagnostic

1. **Amortissement de la table aux hautes fréquences.** On avait η = 0,02·(1 + f/700 Hz), soit 13 % à 4 kHz et 25 % à 8 kHz, calé jadis sur la brillance de l'attaque avant le marteau à deux couches.
   - La puissance que la corde cède à la table ne dépend pas de η. En revanche, la part que la table rayonne au lieu de la dissiper baisse comme 1/η.
   - Cet amortissement assourdissait donc les hautes fréquences sans changer la durée des notes.
   - Les tables réelles mesurent 2 à 3 %, à peu près constant.
2. **Feutre du marteau.** `examples/felt_compression.rs` mesure l'écrasement maximal du feutre :
   - dans le médium, il passe de 0,49 mm en pp à 0,60 mm en ff, pour un cœur dur situé à 0,58 mm de profondeur : le cœur n'est atteint qu'à peine en ff ;
   - amincir le feutre du médium fait travailler le cœur dès le mf. La bande 4–8 kHz du mf devenait alors 18 dB trop forte ;
   - une frappe plus rapide engage le cœur en ff seulement.
3. **Le score du marteau** (`tools/hammer_score.py`) était dominé par le profil d'attaque de do7–do8. Ses partiels 3 à 6 (8 à 25 kHz) sont au niveau du bruit des références. Ce critère est désormais exclu, comme l'était déjà la dynamique de ce registre. Score de départ sans lui : 0,79.

## Changements

Choisis sur deux grilles de 16 essais, en tenant compte de l'écart de brillance et du score du marteau :

| Paramètre | Avant | Après |
|---|---|---|
| `soundboard.loss_corner` | 700 Hz | 2500 Hz (η à 4 kHz : 13 % → 5 %) |
| `velocity.vmin` / `vmax` | 0,5 / 3,5 m/s | 0,6 / 4,2 m/s (vélocité 77 ↔ 1,94 m/s, contre 2 m/s chez Goebl & Bresin) |
| `core_depth_bass` | 0,7 mm | 0,8 mm |
| `core_depth_mid` (nouveau, à do4) | — | 0,57 mm (comme avant) |
| gain de sortie (`PRESSURE_GAIN`, `FORCE_GAIN`) | 1/22, 1/440 | 1/30, 1/600 (−2,7 dB) |

Explications :

- **Profondeur du cœur.** Le feutre est maintenant défini en trois points (la0, do4, do8), et non plus en deux. Le grave garde un feutre épais, même avec un marteau plus rapide.
- **Una corda.** Le décalage fait frapper les cordes sur un feutre que les cordes n'ont pas creusé : plus souple, comme avant, et désormais aussi plus épais au-dessus du cœur (+30 % à fond). L'attaque de do4 s'assombrit de 1,2 dB au lieu d'environ 0,3.
- **Gain de sortie.** Le marteau plus rapide jouait 2,8 dB plus fort. Le gain compense, et le Chopin garde son niveau moyen : −38,2 contre −38,3 dB RMS.

## Résultat

Écarts aux références, en dB (bandes 1, 2, 4 et 8 kHz) :

| Registre | Attaque | Tenue |
|---|---|---|
| grave | −2 / −5 / 0 / +9 | −5 / −3 / +3 / −1 |
| médium | −2 / −5 / −9 / −4 | −5 / −4 / −8 / −9 |
| aigu | −4 / 0 / +1 / (−2) | +2 / +1 / +1 / (−14) |

Écart quadratique moyen : 4,7 dB (7,8 avant).

Profil de la tenue du médium (0,5–1 s), partiels 2, 3, 4 et 6 par rapport au partiel 1 :

| | références | avant | après |
|---|---|---|---|
| dB | +0,3 à +4,6 / −7 à −4 / −10 à −4 / −16 à −11 | −2,6 / −9,9 / −14,2 / −20,6 | −0,6 / −6,5 / −10,6 / −18,5 |

Autres mesures :
- **Score du marteau** : 0,84 (0,79 avant).
- **Tenue** : la fin de note (3 s) varie de moins de 1,2 dB, dans la fourchette des références.
- **Écart de nuance ff − pp** : inchangé, car le rapport vmax/vmin reste 7.
- **CPU** : inchangé (38 % d'un cœur au pire). Les tests passent.

## Ce qui reste

- **Médium à 4–8 kHz** : encore 4 à 9 dB sous les références, à l'attaque comme pendant la tenue. Ni le feutre (dureté, profondeur, cœur) ni la table ne le corrigent sans éclaircir le mf ou les autres registres. Pistes :
  - la forme de l'impulsion de contact, avec un feutre de surface dont la raideur croît plus vite avec l'écrasement ;
  - les résonances du duplex.
- **Grave à 8 kHz** : 9 dB au-dessus des références à l'attaque.
- **Aigu au-dessus de 8 kHz** : en dessous des références pendant la tenue. Mais la mesure y est dominée par le bruit des enregistrements.
