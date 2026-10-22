# Tenue après la première seconde

État au 2026-10-22.

## Constat

Comparé à Commercial, à l'Iowa et à Salamander, le modèle avait une première seconde juste, puis perdait 6 à 17 dB de trop entre 2 et 3 s, dans tous les registres. Les notes « chantaient » trop peu longtemps.

## Mesure

Protocole de [comparaison.md](comparaison.md) : une touche sur trois, vélocités 40, 60 et 100, touche tenue 4 s, prise proche (`presets/proche.toml`), sans salle ni bruits. Chaque essai est rendu puis découpé avec les outils de cette comparaison.

Outils ajoutés :
- `tools/partial_decay.py` : niveau des partiels 1 à 12 à 0,1, 0,5, 1, 2 et 3 s ;
- `tools/partial_t60.py` : décroissance effective de chaque partiel entre 0,3 et 3 s (battements moyennés) ;
- `tools/band_decay.py` : niveaux par bande d'octave à l'attaque, à 0,3 s et à 3 s ;
- `tools/aftersound_compare.py` : son « prompt » et arrière-résonance d'après les composantes ESPRIT de `piano-survey`, soit leur durée, l'écart de niveau et l'écart de fréquence ;
- `examples/aftersound.rs` : même décomposition calculée directement sur les modes couplés du modèle.

## Diagnostic

Les décroissances composante par composante (ESPRIT) étaient justes. Ce qui manquait tient à trois causes, une par registre.

### Médium : arrière-résonance trop faible

Dans les trois références, l'arrière-résonance part à environ 15 dB sous le son « prompt » et tient environ 9 s. Elle prend le relais vers 2 s. Dans le modèle, elle était à −26 dB : la fin de note n'était que la queue du son « prompt ».

L'écart de fréquence entre ces deux composantes trahit la cause. Mesuré de la même façon partout, il vaut :

| | Commercial | Iowa | Salamander | modèle (0,35 cent) |
|---|---|---|---|---|
| grave | 1,50 | 1,28 | 1,54 | 0,66 |
| médium | 0,65 | 0,99 | 0,78 | 0,48 |

Le désaccord d'unisson, ramené à 0,35 cent pour [graves.md](graves.md), était trop serré. Si le désaccord est petit devant le couplage par le chevalet, le mode en opposition de phase ne fait presque pas bouger le chevalet, donc il ne rayonne pas.

### Grave : un mode piston trop absorbant

Niveau par bande d'octave, touches A0–B2, perte entre 0,3 et 3 s :

| Bande | Commercial | Iowa | Salamander | modèle (piston 1,5 kg) | sans piston |
|---|---|---|---|---|---|
| 63 Hz | 21 dB | 11 dB | 6 dB | **37 dB** | 19 dB |
| 125 Hz | 14 dB | 20 dB | 14 dB | 18 dB | 16 dB |
| 500 Hz | 12 dB | 11 dB | 11 dB | 15 dB | 14 dB |

Le mode piston de la caisse (40 Hz, très amorti) avait été réglé sur la décroissance du seul son « prompt ». Or, dans les références, les fondamentales graves décroissent en deux temps : une chute rapide, puis une traîne lente. Le piston absorbait les deux. Avec 6 kg au lieu de 1,5, il absorbe quatre fois moins.

Le cadre, la ceinture, la non-linéarité et la sympathie n'y sont pour rien (essais sur do2 et fa#1).

### Aigu : pertes propres des cordes trop fortes

De la♭5 à ré#6, la composante lente de la fondamentale tient 7 à 9 s dans les références, contre 3 à 4,5 s dans le modèle. Elle était plafonnée par les pertes propres des cordes : T60 de 5,3 s à do6 et de 3,6 s à fa#6.

### Pistes sans effet

Polarisation horizontale (frappe inclinée ×3 ou ×6, rayonnement ×2 à ×3, couplage horizontal ×3), frappe plus inégale (±40 %) et rayonnement du piston : aucun de ces réglages n'a changé l'enveloppe à 3 s de plus de 2 dB.

## Changements

1. `UnisonLaw::width_mid` passe de 0,35 à 1,0 cent. L'écart prompt/arrière-résonance devient 1,3 cent dans le grave et 1,0 dans le médium, et l'arrière-résonance du médium remonte à −14 dB.
2. `SoundboardParams::piston_mass` passe de 1,5 à 6 kg.
3. `LossLaw` : b1 = exp(0,055·touche − 5,1) au lieu de exp(0,059·touche − 4,782), soit ×0,65 à do4 et ×0,5 à fa#6. b2 et le plancher des cordes filées sont inchangés.

## Résultat

Tenue ff, en dB par rapport à la crête, à 0,3 / 1 / 2 / 3 s :

| Registre | Commercial | Iowa | Salamander | avant | après |
|---|---|---|---|---|---|
| grave | −3 / −8 / −13 / −17 | −5 / −11 / −15 / −18 | −4 / −9 / −14 / −17 | −5 / −11 / −18 / **−24** | −4 / −10 / −15 / −20 |
| médium | −6 / −19 / −30 / −33 | −8 / −19 / −24 / −28 | −7 / −20 / −26 / −32 | −7 / −19 / −31 / **−39** | −7 / −18 / −25 / −31 |
| aigu | −17 / −32 / −41 / −50 | −18 / −27 / −39 / −46 | −16 / −30 / −37 / −46 | −16 / −34 / −50 / **−62** | −15 / −29 / −36 / −48 |

Le mf suit de la même façon.

Arrière-résonance (ESPRIT, ff), en T60 et en niveau par rapport au son « prompt » :

| Registre | Références | Avant | Après |
|---|---|---|---|
| grave | 20–38 s, −12 à −15 dB | 11 s, −22 dB | 18 s, −13 dB |
| médium | 9–10 s, −10 à −17 dB | 11 s, −26 dB | 10 s, −14 dB |
| aigu | 3 s, −12 dB | 3,1 s, −15 dB | 3,2 s, −11 dB |

Autres mesures :
- **Score du marteau.** Il passe de 1,69 à 1,38. L'attaque du grave est moins dominée par la fondamentale : partiel 2 à +4,9 dB, contre +1,7 dB avant et +4 à +9 dB dans les références.
- **CPU.** Inchangé : 38 % d'un cœur au pire (`examples/stress_cost.rs`).
- **Tests.** Ils passent.

## Ce qui reste

- **Fin du grave** : encore 2 à 3 dB sous les références à 3 s.
- **Spectre de la tenue du grave** : la fondamentale y domine plus qu'avant, d'environ 3 dB. Partiel 2 à +4 dB par rapport au partiel 1 entre 0,5 et 1 s, contre +14 dB dans les références. La cause est le rayonnement : la bande 63 Hz est la plus forte du modèle à l'attaque, alors qu'elle est 3 à 15 dB sous la plus forte dans les références. C'est le chantier suivant, avec le manque de brillance de la tenue dans le médium et l'aigu.
- **Durée du son « prompt »** : un peu longue dans le grave (13 s contre 8 à 11 s) et le médium (4,8 s contre 2,9 à 3,7 s), alors que l'enveloppe est juste.
