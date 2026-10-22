# Comparaison avec Salamander, Iowa et Commercial

État au 2026-10-22.

## Protocole

Chaque source joue la même série de notes : une touche sur trois à partir de la0 (les 30 notes de Salamander), aux vélocités 40, 60 et 100, rangées comme les couches 4, 8 et 13 de Salamander. La touche est tenue 4 s, et l'enregistrement est découpé en fichiers `<note>v<couche>.wav`.

| Source | Instrument | Prise de son |
|---|---|---|
| Salamander | Yamaha C5, échantillons | couches de l'échantillonneur |
| Iowa MIS (`data/iowa/`) | Steinway modèle B, pp/mf/ff | micros à 20 cm au-dessus des cordes |
| Commercial 6 | préréglage chargé, sans réverbération | joué en temps réel par `piano-play --midi-out "Commercial 6"`, enregistré par `jack_rec` |
| Modèle | rendu hors ligne | sans salle |

Commercial ne retombe au silence numérique qu'après le relâchement des touches : il n'y a donc pas de réverbération à soustraire.

Outils, dans l'ordre d'utilisation :
- `tools/note_sequence_midi.py` génère la séquence ;
- `tools/split_notes.py` découpe et convertit ;
- `tools/compare_sources.py` calcule tenue, profil des partiels, brillance et dynamique ;
- `piano-survey` mesure inharmonicité et décroissances ;
- `examples/hammer_stats.rs` donne le contact du marteau.

## Constats

### 1. Le contact marteau–cordes des unissons était faux (cause principale)

Les profils de partiels du modèle ne bougeaient presque pas quand on changeait la dureté du feutre (×8,6), son exposant ou son hystérésis. Le contact non plus : 5 ms et 15 N pour do4 ff, quelle que soit la dureté. La littérature (Askenfelt & Jansson, Chaigne & Askenfelt) donne environ 2 ms et plusieurs dizaines de newtons.

La cause : le solveur sommait les déplacements des 2 ou 3 cordes d'un unisson, et le feutre comprimait cette somme. Les cordes étaient donc en série, alors que sous un feutre elles sont en parallèle. Le marteau voyait des cordes N² fois trop souples (×4 pour deux cordes, ×9 pour trois). La calibration avait compensé comme elle pouvait : dureté 0,35, exposant −0,5.

Correctif (appliqué, voir plus bas) : le feutre presse sur le déplacement moyen des cordes, chaque corde recevant sa part de la force. Contact obtenu :

| Note | Durée | Force |
|---|---|---|
| do4 ff | 1,9 ms | 39 N |
| do4 pp | 2,7 ms | 8 N |
| la1 ff | 3,7 ms | 36 N |

La dureté du feutre recommence à agir. Profil de l'attaque ff (partiels 2 et 3 en dB par rapport au partiel 1) :

| Registre | Salamander | Iowa | Commercial | Avant | Corrigé |
|---|---|---|---|---|---|
| grave | +6 / +3 | +9 / +14 | +4 / +3 | +2 / −6 | +10 / +8 |
| médium | −2 / −13 | +5 / −3 | −2 / −6 | −11 / −10 | −3 / −10 |
| aigu | −19 / −26 | −22 / −34 | −17 / −26 | −13 / −20 | −20 / −26 |

Le timbre de l'attaque rejoint les références dans tous les registres : le grave et le médium s'éclaircissent, l'aigu, auparavant trop brillant, s'adoucit.

Reste à recalibrer :
- dureté, exposant et carte des vélocités ;
- niveau : +2 à +3 dB au grave et au médium, jusqu'à +8 dB à do6 ff ;
- suraigu aux nuances douces : C8 pp et mf perdent 6 à 9 dB, et l'écart ff − pp monte à +36 dB contre +13 à +25 dB dans les références ;
- una corda.

### 2. Dans le grave, les partiels supérieurs s'éteignent trop vite

T60 médian (s) du début de décroissance, ff, partiels 1–4 / 5–12 :

| Registre | Salamander | Iowa | Commercial | Modèle |
|---|---|---|---|---|
| grave | 9,5 / 6,8 | 6,5 / 5,1 | 9,5 / 7,7 | **12,8 / 3,9** |
| médium | 2,6 / 2,3 | 2,0 / 1,3 | 2,9 / 2,6 | 2,1 / 1,3 |
| aigu | 1,0 / 0,4 | 0,8 / 0,3 | 0,9 / 0,4 | 1,0 / 0,6 |

Dans le grave, les fondamentales tiennent trop longtemps et les partiels 5 à 12 meurent deux fois trop vite. Pendant la tenue (0,5–1 s), la fondamentale domine donc de 10 à 20 dB : c'est le grave « en tonneau » et le son « doux ». Même corrigé, le contact ne rattrape pas cet écart : le profil de la tenue du grave reste entre −13 et −25 dB sur les partiels 4 à 12.

Il faut revoir la dépendance en fréquence des pertes des cordes graves : le plancher `b1_floor`, constant, est remplacé par des pertes qui croissent avec la fréquence pour les cordes filées. Il faut aussi revoir la conductance du chevalet grave entre 150 Hz et 1,5 kHz.

### 3. Le timbre change trop peu avec la nuance

Rapport de brillance de l'attaque ff/pp :

| Registre | Iowa | Commercial | Modèle (avant ou corrigé) |
|---|---|---|---|
| grave | ×1,8 | ×2,1 | ×1,2 |
| médium | ×1,5 | ×1,8 | ×1,25 |

Le jeu sonne uniforme. C'est l'affaire de l'exposant et de l'hystérésis du feutre, à recalibrer une fois le contact corrigé, sur Iowa et Commercial : Salamander a lui-même un rapport plat (×1,2).

### 4. Ce qui est déjà en place

- **Inharmonicité B** : 9e-5 au grave, 2,5e-4 au médium, 2e-3 à l'aigu, conforme aux trois références.
- **Dynamique ff − mf** : 8 à 10 dB, entre Salamander et Iowa, proche de Commercial.
- **Tenue du grave et du médium** jusqu'à 1 s : dans la fourchette des références.

### 5. Écarts connus, non mesurés ici

- Fin des notes aiguës entre 1 et 3 s, plus courte que Salamander et Iowa ; Commercial est entre les deux.
- Partiels fantômes et non-linéarités de la corde en jeu fort.
- Duplex.
- Couplage sympathique passif.

## Recalibration du marteau (appliquée)

Le contact parallèle est dans `hammer.rs`, sans allocation : l'itérateur des modes est parcouru deux fois. Le marteau a ensuite été recalibré sur les trois références à la fois avec `tools/hammer_grid.sh` et `tools/hammer_score.py`. Le score est l'écart quadratique normalisé à la moyenne des références, avec pour tolérance leur dispersion, et au moins 3 dB. Il porte sur :

- le profil des partiels 2, 3, 4 et 6 à l'attaque, en ff et en mf ;
- l'éclaircissement de l'attaque de pp à ff ;
- la dynamique ff − pp et ff − mf, comparée à Salamander et Commercial seulement, car l'Iowa est joué en nuances et non en vélocités.

| | Score |
|---|---|
| avant | 9,3 |
| contact corrigé, anciens réglages | 5,3 |
| retenu | 1,9 |

Réglage retenu, environ 80 combinaisons essayées :

| Paramètre | Avant | Retenu |
|---|---|---|
| `hardness` | 0,35 | 0,7 |
| `hardness_slope` | 0,02 | 0,05 (marteaux plus durs vers l'aigu) |
| `p_offset` | −0,5 | +0,5 (p de 4,2 à 5,5, dans la plage de Stulov) |
| `hysteresis` | 1 | 0,25 |
| vélocité | 0,3–5,5 m/s | 0,6–3,0 m/s |

L'exposant +1 ou +2 gagnait 0,02 à 0,13 de score, mais sortait de la plage mesurée.

Sorties ramenées de −5,3 dB (`PRESSURE_GAIN` 1/22, `FORCE_GAIN` 1/440) : le Chopin garde sa crête de −18,5 dBFS, la Campanella monte à −14 dBFS. Tenue inchangée (`envelope_compare.py`).

Contact obtenu, en durée et force de crête :

| Note | pp | ff |
|---|---|---|
| do4 | 2,4 ms, 11 N | 2,0 ms, 28 N |
| la1 ff | — | 4,1 ms, 25 N |
| do6 | 1,3 ms, 19 N | 1,1 ms, 45 N |
| la7 | 0,8 ms, 36 N | 0,5 ms, 95 N |

Ce qui reste après recalibration :

- **Éclaircissement pp → ff trop faible** : ×1,14 au grave et au médium, contre ×1,2 à ×2,1 dans les références (moyenne ×1,5). Le resserrer élargirait encore la dynamique. Le modèle de feutre de Stulov seul ne donne pas assez de changement de timbre par décibel. Pistes : feutre à deux couches (cœur dur, surface douce), ou loi de raideur dépendant de la compression.
- **Dynamique** un peu étroite au grave et au médium (6 à 7 dB pour ff − pp, contre 10) et un peu large au suraigu (16 contre 13).
- **Partiel 4 du grave** 8 dB sous les références.
- **Suraigu en mf** trop brillant (partiel 2 à −25 dB contre −35).

## Priorités pour réduire l'écart

1. ~~Contact parallèle des unissons et recalibration du marteau~~ : fait.
2. **Pertes des cordes graves selon la fréquence**, pour retrouver l'équilibre des partiels dans la tenue.
3. **Changement de timbre avec la nuance** : un modèle de feutre plus riche.
4. Ensuite seulement, les ajouts de physique : non-linéarités de la corde, duplex, fin des notes aiguës, sympathie passive.
