# Présence et attaque : marteau à deux couches, prises de son

État au 2026-10-22.

## Retour d'écoute

« La version du piano commercial est plus percussive, et la nôtre donne l'impression de jouer piano fermé. »

## Mesure

`tools/presence.py` compare deux choses sur la séquence de comparaison :
- **la percussivité** : crête sur 5 ms moins le niveau entre 150 et 300 ms ;
- **la part d'énergie par bande** à l'attaque (0–60 ms) et dans la tenue.

La percussivité du modèle était normale : 6 à 7 dB au grave et au médium, comme les références. Ce qui manquait, c'étaient les aigus de l'attaque du grave et du médium. Attaque ff du grave, en dB :

| | 1–2 kHz | 2–4 kHz | 4–8 kHz |
|---|---|---|---|
| Commercial | −10 | −17 | −29 |
| Iowa | −8 | −20 | −45 |
| Salamander | −17 | −27 | −57 |
| Modèle | −25 | −43 | −78 |

## Causes

1. **Le feutre calibré pour l'éclaircissement selon la nuance** ([timbre_dynamics.md](timbre_dynamics.md)) était très souple et très non linéaire (dureté 0,1, p + 2). Il coupait les aigus : au grave, −63 dB à 4–8 kHz contre −44 avec le feutre nominal. La calibration ne regardait que jusqu'au partiel 12, soit environ 3 kHz au médium.
   - La force du marteau sur le chevalet manquait déjà d'aigus.
   - Le rayonnement de la table en retire ensuite 5 à 10 dB.
   - Les autres changements (mode piston, unissons, non-linéarité) n'y sont pour rien.
2. **La prise de son.** Les micros concert à 1 m de haut étaient sous la réflexion du couvercle, ce qui retirait environ 10 dB à 2–4 kHz au médium. Les micros au-dessus de la table, à 1,5 m, sont eux aussi plus sombres que des micros proches.

## Changements

**Score.** `tools/hammer_score.py` mesure maintenant la présence de l'attaque, c'est-à-dire la part d'énergie à 2–4 kHz et à 4–8 kHz, en ff et en mf, par registre.

**Marteau à deux couches** (`hammer.rs`, `felt()`) : F(u) = Q·u^p + Q_c·(u − d)₊^p_c. C'est un feutre de surface sur un cœur dur. Le schéma quadratisé reste passif (g = F/√(2V), hystérésis α·dF/dt), et les tests de conservation et de décroissance d'énergie passent avec le cœur. L'écrasement mesuré va de 0,35 à 1,2 mm (`examples/contact_stats.rs`) ; le cœur est placé entre pp et ff.

Paramètres retenus (grilles `tools/core_grid.sh`) :

| Paramètre | Valeur |
|---|---|
| `hardness` | 0,2 (surface) |
| `p_offset` | 0 (p de Stulov, 3,7 à 5) |
| `hysteresis` | 0,6 |
| `core_hardness` | 30 (× Q0 de Stulov de la touche) |
| `core_p` | 2,5 |
| `core_depth_bass` / `core_depth_treble` | 0,7 / 0,4 mm |
| `hardness_slope` / `hardness_slope_bass` | 0,02 / 0,02 |
| `p_bass` | 0 |
| vélocité | 0,5–3,5 m/s |

**Préréglages.**
- `concert.toml` : micros relevés à 2 m.
- `proche.toml` (nouveau) : micros à 40 cm au-dessus de la table, couvercle à 35°. C'est le plus proche de l'équilibre des références.
- `output_gain_db` (nouveau paramètre) égalise le volume entre prises : +3,4 dB pour concert, −9,6 dB pour proche. La crête du Chopin reste à −18,8 dBFS dans les deux.

## Résultat

Score du marteau, avec la présence : 3,23 → 1,54. Attaque ff, part d'énergie (dB) :

| | Grave 2–4 kHz | Grave 4–8 kHz | Médium 2–4 kHz | Médium 4–8 kHz |
|---|---|---|---|---|
| références (moyenne) | −21,5 | −43,8 | −19,1 | −36,6 |
| modèle avant | −36,5 | −68,5 | −24,3 | −42,0 |
| modèle après (micros au-dessus de la table) | −24,6 | −36,4 | −23,9 | −37,8 |
| modèle après, préréglage proche | −23,8 | −38,4 | −25,9 | −42,2 |

- **Éclaircissement pp → ff** (pente, cible +6,1 au grave, +17,4 au médium) : grave +5,5 → +4,0 dB, médium +15,5 → +13,3 dB. C'est le prix de la présence. Le marteau à deux couches rend la brillance du ff, mais aussi un peu celle du pp.
- **Dynamique ff − pp** : 7,7 dB au grave, 9,0 au médium, 13,6 à l'aigu.
- **Coût CPU** : inchangé, 37 % d'un cœur au pire.

## Ce qui reste

- **Le piano commercial reste plus brillant que les deux vrais pianos** au-dessus de 4 kHz (−29 dB au grave contre −45 et −57). Le modèle se cale sur la moyenne des trois.
- **Éclaircissement du grave et du médium** encore 2 à 4 dB sous les références.
- **Aigu** un peu trop contrasté entre pp et ff (13,6 contre 12,2 dB).
