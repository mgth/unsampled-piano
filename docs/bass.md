# Pertes des cordes graves

État au 2026-10-22.

## Constat

D'après la comparaison avec Salamander, l'Iowa et le piano commercial ([comparison.md](comparison.md)), la tenue du grave était déséquilibrée. La fondamentale dominait de 10 à 20 dB, et les partiels 7 à 12 s'éteignaient deux fois trop vite. Le résultat sonnait « en tonneau ».

T60 médian des partiels, en ff, touches A0–B2 (outil `tools/bass_trial.sh`) :

| Partiels | Références | Modèle avant |
|---|---|---|
| 1–2 | 7,4 s | 17,7 s |
| 3–6 | 7,4 s | 8,1 s |
| 7–12 | 6,8 s | 3,5 s |

## Diagnostic

Trois pistes ont été écartées.

- **Pertes propres des cordes.** Elles donnent un T60 de 20 à 30 s, donc elles ne limitent rien (`examples/bass_budget.rs`).
- **Conductance du chevalet.** Elle est proche de l'estimation tirée de Salamander : +1 à +2 dB en moyenne (`examples/bass_conductance.rs`).
- **Masse du chevalet, sympathie, polarisation horizontale et rayonnement.** Aucun n'influe sur ces décroissances.

`examples/partial_modes.rs` donne la vraie cause pour do2, partiel 8. Les deux modes principaux tiennent 8 s, mais ils ont le même poids et sont séparés de 0,3 Hz, car les deux cordes sont désaccordées d'environ 1 cent. Leurs battements creusent l'enveloppe vers 1,6 s, ce qui donne une décroissance apparente de 3,2 s. Les références mesurent un écart de fréquence entre composantes (`after_cents`) d'environ 0,4 cent dans le grave et le médium, contre 0,9 cent dans le modèle.

Les fondamentales graves (30–120 Hz) tombent sous le premier mode de la table encastrée, qui est vers 60 Hz. Là, une table seule n'absorbe presque rien. Sur un piano, la table, la ceinture et la caisse bougent aussi en bloc, ce qui fait un mode large et amorti qui absorbe ces fondamentales.

## Changements

1. **Désaccord d'unisson jusqu'à do4** : `UnisonLaw::width_mid` passe de 1,0 à 0,35 cent, l'écart mesuré.
2. **Mode piston** (`SoundboardParams::piston_*`). Il ajoute aux modes de la table un mode de forme uniforme, réglé sur les décroissances des partiels graves des trois références :
   - fréquence 40 Hz, facteur de perte 0,7, masse effective 1,5 kg ;
   - son rayonnement est réduit à 0,3, car quand toute la caisse bouge, l'air poussé au-dessus revient largement en dessous. Avec 1,0, la bande 0–100 Hz dépassait la référence de 4,6 dB ;
   - c'est un mode équivalent : la masse de 1,5 kg n'est pas une mesure.
3. **Plancher de pertes des cordes filées** maintenu à 0,2 s⁻¹. Avec 0,08, les partiels 3 à 6 tenaient trop longtemps.

## Résultat

T60 médian des partiels :

| Registre | Partiels | Références | Avant | Après |
|---|---|---|---|---|
| grave (A0–B2) | 1–2 | 7,4 s | 17,7 s | 9,6 s |
| grave (A0–B2) | 3–6 | 7,4 s | 8,1 s | 10,0 s |
| grave (A0–B2) | 7–12 | 6,8 s | 3,5 s | 5,4 s |
| médium (C3–B4) | 1–2 | 2,9 s | 3,3 s | 3,0 s |
| médium (C3–B4) | 3–6 | 2,4 s | 1,8 s | 2,2 s |
| médium (C3–B4) | 7–12 | 2,2 s | 1,2 s | 1,8 s |

Spectre à long terme par rapport à Salamander (`tools/calibrate.sh`, `spectral_stats.txt`) :

| | 0–100 Hz | 100–200 Hz | 200–400 Hz | 6,4–20 kHz | erreur d'inclinaison | écart de profil |
|---|---|---|---|---|---|---|
| Salamander | 11,8 | 14,0 | 5,3 | −23,4 | — | — |
| avant | 11,9 | 17,3 | 6,6 | −14,1 | 3,5 dB | 6,3 dB |
| après | 12,2 | 11,5 | 6,5 | −22,2 | 2,5 dB | 4,5 dB |

- **Enveloppes** (`envelope_compare.py`). Le grave et le médium restent à moins de 2,5 dB de la référence jusqu'à 1 s. Ensuite, ils perdent 3 à 6 dB de trop à 2–3 s.
- **Score du marteau.** Il passe de 1,9 à 2,2.
- **Coût et tests.** Construction de l'instrument : 1,09 s. Les tests passent.

## Ce qui reste

- **Profil de la tenue du grave.** Les partiels 3 à 6 restent 10 à 15 dB sous les références par rapport à la fondamentale entre 0,5 et 1 s. Dans les références, la fondamentale grave chute d'environ 8 dB dès la première demi-seconde, une décroissance initiale rapide que le modèle n'a pas. Les fondamentales graves sont toutefois les mesures les plus dispersées des références : leur T60 va de 4,5 à 39 s selon la source et la note.
- **Fin des notes du médium à 2–3 s**, un peu trop courte depuis le désaccord resserré.

Mise à jour : le désaccord (1 cent), la masse du piston (6 kg) et les pertes propres ont été revus sur la tenue après 1 s ([long_sustain.md](long_sustain.md)).
