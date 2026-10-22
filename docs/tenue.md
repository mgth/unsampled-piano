# Tenue des notes : chevalet, épaisseur de la table, pertes des cordes

État au 2026-10-22.

## Retour d'écoute

« La tenue de la note semble un peu courte. Elle dure bien dans le temps, mais perd très rapidement de l'énergie au début. Ça fait un piano assez doux, pas très puissant. »

## Mesure

[`tools/envelope_compare.py`](../tools/envelope_compare.py) compare l'enveloppe du modèle à celle de Salamander. Le protocole :

- une touche sur trois, couche 13 contre vélocité 100, touche tenue 4 s ;
- rendu à sec : sans salle ni bruits ;
- niveaux exprimés par rapport à la crête.

Un passe-haut à 0,7·f0 retire le plancher grave des enregistrements. Dans l'aigu, ce plancher domine la fin de la note : bruit, pièce et résonances graves. Sans ce filtre, les notes aiguës de Salamander semblaient tenir à −40 dB pendant des secondes, et la comparaison était fausse au-delà de 1 s.

[`tools/t60_bins.py`](../tools/t60_bins.py) complète cette vue. Il compare la durée (T60) de chaque partiel, par registre et par bande de fréquence, à partir des composantes ESPRIT de `tools/calibrate.sh`.

### Écart moyen modèle − référence (dB), avant

| Registre | 0,1 s | 0,3 s | 0,6 s | 1 s | 2 s | 3 s |
|---|---|---|---|---|---|---|
| A0–B2 | −2,6 | −0,7 | +0,1 | +1,4 | +2,9 | +3,8 |
| C3–B4 | −2,9 | −5,0 | −5,2 | −3,9 | −4,0 | −3,7 |
| C5–B6 | −2,7 | −5,4 | −7,6 | −7,2 | −18,8 | −23,1 |
| C7–C8 | −2,3 | −6,9 | −10,6 | −10,3 | −11,6 | −8,3 |

L'impression d'écoute était juste. Du médium à l'aigu, la note perdait 5 à 8 dB de trop dans la première demi-seconde.

## Diagnostic

La fuite d'énergie d'un mode de corde vers le chevalet vaut σ = N·G·T/L, avec N cordes, G la conductance du chevalet, T la tension et L la longueur. Pour do6, la décroissance rapide mesurée (T60 1,46 s) donne G ≈ 3,7e-4 s/kg. C'est aussi ce que donne l'ajustement global de Salamander (`tools/bridge_conductance.py`) : log G = −8,84 + 0,289·log f − 0,0146·touche. Le modèle avait 7,3e-4 au chevalet de do6 : deux fois trop.

Comme les cordes aiguës sont courtes (T/L grand), elles sont les plus sensibles à cet excès.

[`examples/bridge_conductance_model.rs`](../crates/piano-core/examples/bridge_conductance_model.rs) ajuste la même loi sur le modèle. Avant correction, la conductance croissait trop vite avec la fréquence (pente 0,465 au lieu de 0,289) et ne baissait pas assez vers l'aigu (−0,0094 par touche au lieu de −0,0146). [`examples/decay_budget.rs`](../crates/piano-core/examples/decay_budget.rs) détaille, note par note, les pertes propres de la corde et les T60 des modes couplés.

## Changements (`soundboard.rs`, `params.rs`)

1. **Le chevalet répartit la force** (`bridge_stiffness` 470 N·m², `bridge_line_mass` 0,6 kg/m, `bridge_spread` 0,3). Le chevalet est une poutre d'érable d'environ 3 × 2,5 cm collée sur la table. La force d'une corde n'attaque pas la table en un point : elle est étalée le long du chevalet.
   - Elle est étalée sur 0,3 fois la longueur d'onde de flexion du chevalet, 2π·(EI/(μω²))^¼, plafonnée à 0,5 m, avec une fenêtre de Hann le long du tracé.
   - Ce lissage s'applique mode par mode : `Soundboard::bridge_shapes` donne la forme vue par chaque touche.
   - Il sert aux entrées de la banque de table, aux vitesses du chevalet (sympathie, réciprocité) et à l'admittance qui couple les cordes.
   - Effet : les petites ondulations de la table, c'est-à-dire les modes aigus, sont moins excitées. La conductance monte donc plus lentement avec la fréquence.
2. **La table est plus raide sous l'aigu** (`impedance_slope` 0,035). Les tables sont plus épaisses et plus serrées en barres côté aigu, et la fin du chevalet aigu est près de la ceinture.
   - L'impédance sous le chevalet varie comme e^{0,035·(touche − 60)} : ×4,6 à do8, ×0,4 à la0, par rapport à do4.
   - Elle est appliquée aux formes, donc la table répond aussi moins aux cordes aiguës.
   - La plaque modale reste uniforme : c'est un gradient équivalent, pas une épaisseur variable résolue.
3. **Pertes des cordes à haute fréquence** : b2 = 1e-7 au lieu de 3e-8.
   - L'étalement du chevalet ralentissait trop les partiels supérieurs. Avec ce b2, ils retrouvent la vitesse de décroissance mesurée.
   - Cela retire aussi l'essentiel de l'excès au-dessus de 6 kHz.
   - b2 = 2e-7 (la valeur de la décroissance rapide mesurée) aligne encore mieux les partiels supérieurs, mais raccourcit les fondamentales de l'aigu de 2 à 3 dB.
4. **Tracé du chevalet**, en 5 points au lieu de 3. Le x suit la longueur parlante de chaque corde depuis les agrafes ; le y suit la place de la touche. L'extrémité aiguë vient dans le coin encastré de la table, comme sur un vrai piano. À lui seul, ce changement modifie peu les décroissances.
5. **Le marteau frappe inégalement les cordes d'un unisson** (`UnisonLaw::hit_spread` 0,2, soit environ ±20 % de force d'une corde à l'autre, réparti de façon irrégulière mais déterministe selon la touche). La part inégale excite directement les modes lents en opposition de phase, c'est-à-dire l'arrière-résonance. Sur l'aigu (do5–ré#6), l'arrière-résonance passe ainsi de −13,5 à −10,6 dB, pour −11,2 dB dans la référence, sans changer sa durée ni l'écart de profil (6,0 dB).
6. `LossLaw::b1_floor` : plancher de pertes pour les cordes filées. Il vaut 0 par défaut, car le gradient d'impédance a suffi à raccourcir les graves.

Les réglages ont été choisis sur une grille. Pour chaque essai : calibration complète (`tools/calibrate.sh`, une minute) et enveloppes de clavier.

## Résultat

Écart moyen modèle − référence (dB), après :

| Registre | 0,1 s | 0,3 s | 0,6 s | 1 s | 2 s | 3 s |
|---|---|---|---|---|---|---|
| A0–B2 | −2,1 | −1,2 | −1,2 | −0,0 | +1,6 | +1,9 |
| C3–B4 | −1,5 | −1,2 | −0,2 | +1,6 | −2,1 | −4,1 |
| C5–B6 | +1,2 | −0,6 | −3,6 | −3,7 | −11,9 | −20,0 |
| C7–C8 | +5,7 | +3,3 | −4,5 | −8,4 | −14,3 | −11,0 |

- **Première seconde.** Les trois premiers registres sont à moins de 4 dB de la référence, contre 5 à 8 dB de perte en trop auparavant.
- **Durée des partiels (ESPRIT).** Le médium et les partiels supérieurs sont entre −0,4 et +0,7 en log2, contre −0,5 à −0,8 auparavant pour les partiels aigus.
- **Écart de profil spectral.** 6,0 dB au lieu de 7,0 dB.
- **Bande 6,4–20 kHz.** −14,4 dB au lieu de −8,7 dB, pour −23,4 dB dans la référence.
- **Coût.** Le CPU (28 % d'un cœur au pire) et la construction (1,06 s) sont inchangés.

## Ce qui reste

- **Fin de note dans l'aigu.** De do5 à si6, entre 2 et 3 s, l'enveloppe reste en moyenne 11 à 18 dB sous la référence. Ce qui a été vérifié :
  - **Ce n'est pas la salle.** Avec la petite salle réverbérante, l'écart est le même.
  - **Ce n'est pas la polarisation horizontale.** L'excitation ×3 et le rayonnement ×2,5 ne changent rien.
  - **C'est la fondamentale.** À 1,8 s, elle domine la référence. Le modèle en a 2 dB de moins à F#5, 5 dB à A5 et 10 dB à C5.
  - **L'écart varie selon la note.** Il suit le désaccord de chaque unisson, irrégulier d'une touche à l'autre comme sur un vrai piano.
  - **Resserrer le désaccord** (`width_treble` 2,5 ou 1,5) rapproche l'écart de fréquence entre composantes de la mesure : 1,0–1,2 cent contre 0,5 dans la référence. Mais l'arrière-résonance faiblit alors (−16 à −23 dB), car dans le modèle elle naît surtout du désaccord.
  - **Piste suivante.** Le mécanisme qui, dans un vrai piano, donne à la fois un petit écart de fréquence et une arrière-résonance forte et longue : la partie réactive de l'admittance du chevalet et le couplage entre unissons voisins par le chevalet.
- **Graves.** Les partiels de 300 à 700 Hz décroissent environ 2 fois trop vite, ceux sous 300 Hz environ 2 fois trop lentement.
- **Una corda.** Le gain d'arrière-résonance n'est plus reproduit en moyenne : il varie de −7 à +7 dB selon la note. Le couplage d'unisson est plus faible depuis ces changements. Le test vérifie seulement que la tenue n'est pas perdue.
- **Limites du modèle de table.** Le gradient d'impédance et l'étalement du chevalet sont des modèles équivalents. Une table d'épaisseur variable, avec barres et chevalet en poutre couplés, demanderait des modes calculés numériquement (éléments finis) au lieu des modes analytiques.

## Retour d'écoute : grave « comme dans un tonneau » (2026-10-22)

Le gradient d'impédance s'appliquait aussi côté grave. Il rendait la table 2 à 2,5 fois plus mobile sous les basses que sous do4, et renforçait donc ses résonances graves. Dans la bande 100–200 Hz, le modèle dépassait Salamander de 12,8 dB (26,8 contre 14,0), contre 9,7 dB avant ces changements.

Il est désormais séparé en deux (`impedance_slope_bass` = 0, `impedance_slope` = 0,035 côté aigu). Les décroissances du grave sont reprises par les pertes propres des cordes filées : `LossLaw::b1_floor` = 0,2 s⁻¹ sous `wound_until` = 48, s'estompant sur l'octave au-dessus. C'est l'ordre de grandeur des taux ESPRIT de l'arrière-résonance du grave dans Salamander (0,15–0,18).

| | 0–100 Hz | 100–200 Hz | 200–400 Hz | 400–800 Hz | erreur d'inclinaison |
|---|---|---|---|---|---|
| Salamander | 11,8 | 14,0 | 5,3 | 1,0 | — |
| avant | 13,0 | 26,8 | 8,3 | 4,1 | 5,3 dB |
| après | 11,9 | 17,3 | 6,6 | 1,8 | 3,5 dB |

Enveloppes du grave (A0–B2) : entre −0,9 et +0,8 dB de la référence de 0,1 à 3 s, contre +2,1 à +2,8 dB après 2 s auparavant.
