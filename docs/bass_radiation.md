# Rayonnement des fondamentales graves

État au 2026-10-22.

## Constat

Pendant la tenue (0,5–1 s), la fondamentale des notes graves dominait : les partiels 2 à 6 étaient de 3 dB sous elle à 7 dB au-dessus, contre 10 à 15 dB au-dessus dans les trois références. C'est le son « en tonneau ».

`tools/third_octaves.py` mesure le spectre en tiers d'octave des touches la0–si2 (ff, 0,05–0,45 s). Les niveaux sont exprimés par rapport à l'énergie entre 100 Hz et 1 kHz :

| Bande | Commercial | Iowa | Salamander | modèle avant |
|---|---|---|---|---|
| 25–40 Hz | −34 à −41 | −43 à −47 | −27 à −31 | −10 à −13 |
| 50 Hz | −16 | −30 | −21 | +0,5 |
| 63 Hz | −5 | −25 | −21 | +5,6 |
| 79 Hz | −3 | −8 | +2 | +0,3 |
| 100 Hz | +3,5 | −4 | +8 | −7,4 |

Sous 70 Hz, le modèle rayonnait 15 à 30 dB de trop, et la bande de 100 Hz était en creux.

## Diagnostic

`examples/bass_radiation.rs` (crate piano-render) calcule la fonction de transfert entre la force au chevalet grave et la pression aux micros, par tiers d'octave. Il montre aussi l'effet de plusieurs variantes. Trois causes ressortent.

1. **Rayonnement calculé sur la vitesse.** Le poids de chaque mode vers les micros est calculé une fois, à sa fréquence de résonance (intégrale de Rayleigh). Il était appliqué à la vitesse modale. Or la pression rayonnée suit l'accélération de la surface. Sous une fondamentale grave, la table bouge surtout par la réponse quasi statique des modes de 100 Hz à 1 kHz. Chacun rayonnait alors comme à sa propre résonance, soit ω_n/ω fois trop : 10 à 30 dB en trop sous 50 Hz.
2. **Le mode piston rayonnait.** Le mode qui représente le mouvement en bloc de la caisse ([bass.md](bass.md)) rayonnait avec un poids de 0,3. Or l'air qu'il pousse au-dessus de la table revient par le dessous, qui est ouvert : ce rayonnement se court-circuite presque entièrement.
3. **Le premier mode de la table était trop bas.** Le mode (1,1), à 61 Hz, était le seul entre 40 et 110 Hz, au centre de la table, juste sous le chevalet grave. Les références, elles, ne rayonnent bien qu'à partir de 80 à 100 Hz. Sur une vraie table, la bombe et la pression des cordes sur le chevalet en font une coque précontrainte : sa raideur de membrane relève surtout les modes les plus graves.

Le court-circuit entre les deux faces de la table (`back_gain`) n'y est presque pour rien : passer de 0,6 à 1, ou rendre l'annulation totale aux basses fréquences, ne change que 1 à 2 dB.

## Changements

1. **Rayonnement par l'accélération** (`board::to_acceleration_weights`). Les poids des micros de tous les bancs rayonnants (table, cadre, ceinture) sont divisés par la réponse d'un différenciateur à la fréquence du mode. Le moteur applique ensuite ce différenciateur, y[n] = (x[n] − x[n−1])·fs, à la somme des micros.
   - À la résonance, le résultat est inchangé ; plus bas, il est proportionnel à ω.
   - Le critère de mise en sommeil des bancs (`energy`) tient compte de ce facteur.
   - `board::mic_transfer` donne le transfert tel qu'il est rendu.
2. **Le mode piston ne rayonne presque plus** : `piston_radiation` passe de 0,3 à 0,03.
3. **Bombe** : `crown_freq` = 90 Hz, une raideur répartie (fondation élastique) qui ajoute (2π·90)² à ω² de chaque mode de la table. Le mode (1,1) passe de 61 à 109 Hz ; au-dessus de 300 Hz, les modes bougent à peine.
4. **Côté grave de la table plus raide** : `impedance_slope_bass` passe de 0 à −0,03, soit une impédance ×2,2 à la0.
   - La bombe regroupe les premiers modes entre 150 et 300 Hz. Le chevalet grave y absorbait davantage, et la fin de note du grave perdait 5 dB à 3 s.
   - Réduire plutôt les pertes propres des cordes filées (`b1_floor` 0,1) donnait la bonne enveloppe. Mais les décroissances composante par composante devenaient deux fois trop longues : 23 s contre 8 à 11 s.

## Résultat

Spectre des notes graves (même mesure que plus haut) :

| Bande | 25–40 Hz | 50 Hz | 63 Hz | 79 Hz | 100 Hz | 126 Hz | 200 Hz |
|---|---|---|---|---|---|---|---|
| références | −27 à −47 | −16 à −30 | −5 à −25 | −8 à +2 | −4 à +8 | −10 à +5 | −0,4 à +0,5 |
| avant | −10 à −13 | +0,5 | +5,6 | +0,3 | −7,4 | −4,0 | +5,3 |
| après | −27 à −47 | −25 | −15 | −12 | +5,7 | −2,1 | +2,3 |

Profil de la tenue du grave (0,5–1 s), partiels 2, 3, 4 et 6 par rapport au partiel 1 :

| | Commercial | Iowa | Salamander | avant | après |
|---|---|---|---|---|---|
| dB | +14 / +13 / +10 / +12 | +15 / +15 / +15 / +18 | +14 / +14 / +13 / +8 | +4 / +0,5 / −7 / −3 | +13 / +11 / +12 / +7 |

Autres mesures :
- **Attaque du grave** (partiels 2 et 3 par rapport au partiel 1) : +7 / +11 dB, contre +4 à +9 / +3 à +14 dans les références (+5 / 0 avant).
- **Score du marteau** : 1,38 → 1,11.
- **Tenue** : inchangée dans le grave (−19,6 dB à 3 s). Le médium perd 1,4 dB à 3 s (−32,1), dans la fourchette des références (−27,5 à −32,7).
- **Niveau** : le grave reste 1,8 dB sous le médium (références : de −3,3 à +1,2 dB). Le Chopin monte de 1 dB (crête −17,8 dBFS).
- **CPU** : inchangé (37 % d'un cœur au pire). Les tests passent.

## Ce qui reste

- **Grave entre 1 et 2,5 kHz** : 3 à 6 dB sous les références pendant l'attaque, d'où un grave un peu sombre. C'est le chantier de la brillance de la tenue.
- **Bandes 79 Hz et 317 Hz** : creux de 5 à 10 dB, dus à la répartition des modes d'une plaque rectangulaire uniforme. Une table réelle (forme, barres, épaisseur variable) répartirait ses modes autrement.
- **Décroissance du son « prompt » dans le grave** : un peu longue (17 s pour les partiels 1–2, contre 8 à 11 s), alors que l'enveloppe est juste.
