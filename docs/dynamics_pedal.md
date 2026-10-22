# Dynamique, pédale et résonance sympathique

État au 2026-10-22.

## Retour d'écoute

Premières écoutes d'interprétations réelles (MAESTRO, Disklavier) : « Ça joue un peu comme un bûcheron, la courbe dynamique n'est pas bonne. Ça résonne très longtemps et fort : est-ce la pédale qui ne marche pas, ou ça joue trop fort ? »

Trois causes ont été trouvées ; aucune ne venait du timbre.

## 1. Vélocité MIDI → vitesse du marteau

L'ancienne loi (0,4 → 4 m/s, exponentielle) avait été ajustée sur les couches de Salamander. Mais la correspondance entre couches et vélocités de Salamander est celle d'un échantillonneur, pas une mesure.

Les pianos enregistreurs suivent une échelle logarithmique : vélocité = 52 + 25·log₂(v), avec v la vitesse du marteau en m/s. C'est la carte du Bösendorfer SE, et le Disklavier place 2 m/s vers 77 ([Goebl & Bresin, JASA 2003](https://iwk.mdw.ac.at/goebl/papers/Goebl-Bresin_JASA2003_reproAccuracy.pdf)). Cette carte va de 0,24 à 8 m/s. L'ancienne écrasait les deux bouts : à vélocité 20, 0,57 m/s au lieu de 0,41 ; à 110, 3,0 m/s au lieu de 5,0. Les nuances douces étaient trop fortes et les fortes pas assez, ce qui donnait un jeu uniforme.

- Premier défaut adopté : `VelocityLaw { vmin: 0.243, vmax: 8.0 }`. Après écoute comparative, il a été remplacé par un intermédiaire (voir plus bas).
- L'ancienne loi reste disponible sous `VelocityLaw::SALAMANDER_LAYERS`.
- À 127, un la0 seul culmine à −6,7 dBFS. Sur les six pièces, la crête maximale est −6,5 dBFS (Campanella).

## 2. Course morte de la pédale

Les pianistes laissent le pied sur la pédale. Dans le Bach de MAESTRO, CC64 ne descend jamais sous 41, et 49 % du temps il est entre 64 et 90.

L'ancien modèle levait complètement les étouffoirs dès 0,4 de course (CC 51), et à moitié dès CC 13. Le Bach entier était donc joué pédale enfoncée.

Sur un vrai piano, la pédale a une course morte avant de toucher la barre d'étouffoirs (`DamperLaw::pedal_free`).

Premier réglage (0,35 → 0,8 : demi-pédale vers CC 64, étouffoirs levés vers CC 102) : la résonance était maîtrisée, mais les notes paraissaient « étouffées sèchement, sans lien ». Le Bach passe la moitié du temps entre CC 64 et 90, zone que ce réglage traitait comme une demi-pédale. Or les fichiers enregistrés suivent la convention MIDI « pédale enfoncée à partir de 64 », celle que les auteurs de MAESTRO appliquent eux-mêmes.

Réglage retenu, 0,3 → 0,55 :

| CC64 | 30 | 40 | 45 | 50 | 55 | 60 | 64 | 70 |
|---|---|---|---|---|---|---|---|---|
| contact des étouffoirs | 1,00 | 0,91 | 0,69 | 0,49 | 0,32 | 0,17 | 0,08 | 0 |

Ces valeurs restent des estimations, Yamaha ne publiant pas la correspondance.

**Non-linéarité du feutre** (`DamperLaw::contact_exponent` = 2,5). Le feutre était un frottement proportionnel au contact, réglé à l'absorption maximale. Un étouffoir posé à moitié amortissait donc déjà à 89 % du maximum : il n'y avait pas de vraie demi-pédale, et chaque note s'arrêtait net dès que l'étouffoir effleurait la corde.

Le frottement suit maintenant le contact à la puissance 2,5, comme le feutre des marteaux de Stulov. Le taux d'amortissement d'un do4 vaut 49 % du maximum à mi-contact et 7 % à 0,2 de contact. Étouffoir posé, rien ne change : T60 de 0,11 s pour do4, 0,47 s pour mi1.

## 3. Niveau de la résonance sympathique

`examples/sympathy_level.rs` mesure, pédale enfoncée, ce que les cordes à l'écoute ajoutent au son. Avec le couplage « physique » (gain 1) :

- un do4 seul gagnait +8 dB entre 1,5 et 3 s ;
- un do2 gagnait +11 dB, et la corde do1 finissait plus forte que la note jouée ;
- un accord do–mi–sol–do gagnait +6 dB dès l'attaque, et +10 dB ensuite.

Dans le Chopin, la résonance sympathique ajoutait 3,6 dB à l'ensemble.

La cause est le couplage à sens unique (phase 3) : les cordes à l'écoute prennent leur excitation à la table sans lui retirer d'énergie. Quand des partiels coïncident, ce qui arrive aux octaves, elles grossissent bien au-delà de ce que permet un système passif. Le couplage dans les deux sens, retardé, était instable.

En attendant un couplage passif, `SympatheticLaw::gain` passe à 0,25. Ce n'est pas une mesure, c'est une compensation. Avec ce gain :

- un accord pédale enfoncée gagne +1,3, +2,5 puis +1,4 dB ;
- une note seule reste à ±2 dB ;
- le Chopin entier gagne 0,1 dB.

## Toucher : écoute comparative

Les vélocités de Salamander viennent d'un échantillonneur, celles de MAESTRO d'un Disklavier. Les mesures seules ne disent pas quelle échelle convient à ces fichiers. Par rapport à Salamander, l'échelle Disklavier étend la dynamique : 30 dB du plus doux au plus fort au lieu de 21. Elle éclaircit aussi l'attaque des nuances fortes : centroïde ×1,4–1,7 au lieu de ×1,2–1,5. Les trois réglages `out/ab/*.toml` ont été départagés à l'écoute (A : Disklavier, B : Salamander, C : intermédiaire). Le choix s'est porté sur **C**, qui est devenu le réglage par défaut : `VelocityLaw { vmin: 0.3, vmax: 5.5 }`, avec 2 m/s vers la vélocité 84.

Après la correction du contact marteau–cordes ([comparison.md](comparison.md)), la loi a été recalibrée avec le marteau, sur la dynamique de Salamander et du piano commercial aux mêmes vélocités : 0,6–3,0 m/s. Ce choix fait par l'écoute est donc à refaire.

## À faire

- **Couplage sympathique passif dans les deux sens.** Ce serait la vraie correction : la corde à l'écoute reprend de l'énergie à la table. Par exemple, une matrice de couplage instantanée restreinte aux paires de partiels proches, sans retard.
- **Mesurer la course de pédale d'un Disklavier.** En l'enregistrant, on remplacerait les deux estimations ci-dessus.
- **Tests de calibration sur Salamander.** Ils utilisent encore les vélocités des couches de l'échantillonneur (`piano-eval --velocities`). Pour comparer la dynamique, il faut maintenant passer `--vmin 0.4 --vmax 4`.
