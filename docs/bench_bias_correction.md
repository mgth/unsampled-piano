# Banc corrigé : niveaux Plath et puissance des canaux

28 octobre 2026 — branche `codex/piano-realism-audit`.

Intégré à `main` le 30 octobre 2026 (protocole seul). Les fichiers `bench/protocol-v2/` cités plus bas, et les audits menés en même temps, sont restés sur la branche d'archive [`codex/archive-realism-audit`](https://github.com/mgth/unsampled-piano/tree/codex/archive-realism-audit). Les mesures de ce document portent sur le modèle du 28 octobre ; celles de l'intégration sont dans [bench.md](bench.md).

**Le diagnostic reste concentré sur le suraigu, et fait ressortir davantage un déficit de partiel 8 dans le sustain de l’aigu.** La normalisation de Plath faussait bien les niveaux entre touches, mais sa correction seule explique peu le score agrégé. Le passage aux puissances des canaux modifie davantage les conclusions.

Le moteur et son preset n’ont pas été retouchés. Les **90 nouveaux WAV sont identiques octet pour octet** aux rendus de l’audit précédent. Les différences ci-dessous viennent des références et de l’analyse.

## Corrections appliquées

- `tools/plath_set.py` conserve les pressions relatives avec **un gain commun de 0,01 unité numérique/Pa**. La sélection de la prise médiane est conservée ; chaque note garde dans `manifest.json` la prise choisie, sa crête en Pa et le nombre de prises. Les WAV sont flottants sur 32 bits. Les 88 notes ont été réimportées depuis les CSV locaux ; leurs crêtes originales vont de 0,651 à 3,827 Pa, sans saturation avec ce gain.
- Le banc Python calcule les spectres sur la **moyenne des puissances des canaux**, et non sur leur pression moyenne. Même correction pour les onsets, les niveaux temporels, les enveloppes filtrées, les bandes et l’homogénéité. Un signal mono est dupliqué sans changer son énergie ; un partiel en opposition de phase entre L/R n’est plus perdu.
- `piano-survey --channel-power` suit les partiels et estime l’inharmonicité sur les spectres combinés. Pour les T60, il démodule chaque canal, combine les énergies, puis ajuste la décroissance. Il ne moyenne pas deux T60 calculés séparément.
- Le protocole est identifié par `analysis_version = "channel-power-v2"`. Le banc refuse les références d’une autre version et l’ancien import Plath. Il ignore une baseline incompatible au lieu de produire une comparaison trompeuse. La baseline corrigée du même moteur est désormais enregistrée ; les anciennes mesures restent archivées.

La migration porte sur le banc de **notes isolées**. Les scripts séparés de morceaux/pédale conservent leur chemin historique ; les anciens appels `bench.load()` restent mono par défaut, et le banc demande explicitement les canaux. ESPRIT reste un diagnostic sur un signal possédant sa phase, incompatible avec l’option `--channel-power`.

## Résultat global et couverture

| Analyse | Mesures | RMS z | Dans ±1 z | Hors ±2 z |
|---|---:|---:|---:|---:|
| Ancien protocole | 189 | 1,026 | 70,9 % | 9 |
| Gain Plath corrigé, analyse mono conservée | 189 | 1,024 | 70,9 % | 9 |
| Protocole corrigé, mêmes 189 mesures | 189 | 1,042 | 72,0 % | 13 |
| Protocole corrigé, toutes les mesures disponibles | 191 | 1,064 | 71,2 % | 15 |

**Deux des quinze alertes ne sont pas des confirmations solides** : les enveloppes du suraigu à 3 s en pp et mf sont devenues mesurables pour le modèle, mais leur cible n’est fournie que par une seule source. Le modèle est au plancher de −60 dB, la cible à −53 dB. Elles n’entraient pas dans le score précédent. Pour comparer les protocoles, retenir d’abord la ligne des **189 mesures communes**, sans confondre couverture accrue et détérioration acoustique.

Sur les mesures communes, **sept anciennes alertes persistent**, deux disparaissent, six apparaissent. Le score global proche de 1 masque donc encore un déplacement important des diagnostics locaux.

| Famille | Avant | Après, protocole complet |
|---|---:|---:|
| Attaque | 0,99 | 1,08 |
| Brillance | 1,21 | 1,24 |
| Dynamique | 0,29 | 0,31 |
| Sustain | 1,47 | 1,42 |
| Décroissance | 0,91 | 0,97 |
| Accordage | 0,54 | 0,54 |

Les onze sources ont été recalculées, dont huit pianos à queue utilisés comme cibles. Le suivi Rust retrouve des partiels dans **1 149 fichiers cibles**, et 1 566 fichiers en incluant les sources informatives. Les notes sans assez de partiels sont consignées dans les journaux. **Aucun fichier ff auparavant analysable n’a disparu.** Deux notes douces de Headroom deviennent analysables, une note pp de BiVib grand ne l’est plus. Ces changements n’affectent pas la couverture ff des T60 et de l’accordage. Les 90 fichiers du modèle ont été analysés.

La source Liang disponible dans le jeu de notes est déjà mono : sa duplication ne recrée pas une stéréo absente. Les autres jeux inspectés sont stéréo. Les différences de piano, de placement des micros et de vélocité entre banques restent des limites du score ; les deux corrections n’en font pas une mesure universelle du réalisme.

## Les écarts qui subsistent ou se renforcent

Les niveaux des partiels sont relatifs à p1 ; les bandes sont relatives à la bande de référence du banc. La « cible » est la moyenne de la métrique transformée des pianos disponibles, pas une valeur obligatoire pour chaque piano réel. Pour les T60, la moyenne est prise en logarithme puis reconvertie en secondes.

| Mesure | z avant → après | Modèle corrigé | Cible | Sources cibles |
|---|---:|---:|---:|---:|
| Grave, T60 initial des partiels 5–12 | −2,15 → **−2,33** | 3,6 s | 6,5 s | 8 |
| Aigu C5–B6, p8 en sustain | −1,74 → **−3,27** | −73,3 dB | −53,4 dB | 8 |
| Suraigu, p2 à l’attaque mf | +3,07 → **+3,86** | −23,4 dB | −35,0 dB | 6 |
| Suraigu, p3 à l’attaque mf | +2,09 → **+2,34** | −43,1 dB | −52,0 dB | 6 |
| Suraigu, p3 à l’attaque ff | +2,75 → **+2,30** | −34,1 dB | −45,1 dB | 8 |
| Suraigu, p4 à l’attaque ff | +1,51 → **+2,35** | −43,7 dB | −59,8 dB | 7 |
| Suraigu, part d’énergie 4–8 kHz à mf | +1,36 → **+2,45** | −18,5 dB | −28,3 dB | 6 |
| Suraigu, bande 1 kHz à l’attaque | −4,05 → **−4,31** | −46,5 dB | −24,2 dB | 8 |
| Suraigu, bande 2 kHz à l’attaque | −1,74 → **−2,74** | −21,9 dB | −12,0 dB | 8 |
| Suraigu, bande 1 kHz en sustain | −1,97 → **−2,02** | −55,3 dB | −43,4 dB | 8 |
| Suraigu, bande 2 kHz en sustain | −2,19 → **−2,79** | −40,5 dB | −29,4 dB | 8 |
| Suraigu, p2 en sustain | +1,22 → **+2,09** | −28,8 dB | −37,1 dB | 8 |
| Suraigu, p4 en sustain | −3,85 → **−2,56** | −75,3 dB | −60,8 dB | 3 |

L’alerte sur p4 en sustain est moins forte et repose sur trois sources seulement, avec des niveaux très faibles : elle mérite une inspection par note et du rapport signal/bruit avant de guider une modification. La bande 1 kHz du sustain, juste à −2,02 z, est une alerte de seuil fragile. Le déficit de p8 dans l’aigu reste à vérifier par touche et par fenêtre, malgré le soutien des huit sources : une moyenne de registre ne dit pas si toutes les touches présentent ce défaut.

Deux alertes du protocole précédent ne subsistent pas :

- **Médium, bande 8 kHz de l’attaque** : +2,03 → +1,44 z.
- **Suraigu, T60 initial des partiels 1–4** : +2,21 → +1,63 z.

Il serait donc mal orienté de commencer par corriger ces deux points sur la base des anciennes alertes.

## Homogénéité et correction Plath

La normalisation Plath effaçait un effet réel même si le score global y était peu sensible. Avec l’analyse mono maintenue, la balance de cette source par rapport au médium change de :

- grave : −1,23 → −3,69 dB ;
- aigu : −0,26 → +4,11 dB ;
- suraigu : −2,36 → +4,55 dB.

Les autres sources diluent ce changement dans le score. Le protocole stéréo modifie ensuite ces valeurs à nouveau, parce que l’ancienne somme des microphones produisait ses propres annulations.

L’homogénéité actuelle du modèle, comparée avec les cibles elles aussi recalculées :

| Sustain, écart touche à touche autour de la tendance | Avant modèle / cible | Après modèle / cible | z après |
|---|---:|---:|---:|
| Partiel 1 | 7,46 / 6,41 dB | 6,99 / 5,37 dB | +1,07 |
| Partiel 2 | 6,30 / 4,82 dB | 6,58 / 4,30 dB | +1,52 |

L’homogénéité reste perfectible mais n’est pas hors ±2 z sur cette mesure. Le calcul utilise toujours une touche sur trois : il ne remplace pas une gamme chromatique dense pour vérifier les transitions B→C ou E→F entendues en jeu.

## Conséquence pour la suite

Priorité à trois diagnostics ciblés, avant de changer le moteur :

1. **Suraigu : séparer attaque harmonique et corps non tonal.** Les partiels d’attaque sont trop forts pendant que l’énergie plus basse, à 1–2 kHz, manque. Un unique réglage global de brillance risque d’améliorer l’un en aggravant l’autre. Comparer bruit mécanique, vibration de table/cadre et partiels sur des fenêtres distinctes.
2. **Aigu : localiser la disparition du partiel 8 pendant le sustain.** Mesurer son niveau dès la frappe puis sa pente, par note ; séparer défaut d’excitation, pertes et rayonnement.
3. **Grave : revoir le diagnostic des pertes des partiels 5–12.** Le T60 initial reste presque deux fois trop court relativement à la cible, avec les huit pianos disponibles. La correction du banc confirme cette piste.

Les réglages d’écoute, de vélocité et de sortie brute du banc restent identiques à ceux du protocole précédent. Ces résultats ne valident pas encore le son du preset de concert entendu au lecteur avec son limiteur.

## Validation et fichiers

- 8 tests Python passent : invariance à la polarité et à l’ordre des canaux, conservation d’un partiel annulé en mono, gain commun de l’import Plath, rejet d’anciens imports/protocoles, et tests de l’audit précédent.
- 11 tests Rust passent : 7 existants et 4 nouveaux, dont récupération du T60 en opposition de phase et combinaison de deux décroissances distinctes avant ajustement.
- Compilation release réussie ; contrôle `git diff --check` propre.
- Rendus : `out/bench/protocol-v2/notes/` ; mesures détaillées : `out/bench/protocol-v2/`.
- Résultats conservés dans le dépôt : `bench/protocol-v2/scorecard.md`, `comparison.json`, `result.json`, `coverage.json`, `manifest.json` et `references-command.json`.
- Anciennes références et baseline : `bench/protocol-v2/references-before.json`, `baseline-before.json`. La baseline et les références actives utilisent toutes le protocole v2.

Le scénario « gain Plath seul » recalcule toutes les mesures de signal avec l’ancienne analyse mono ; il conserve les anciennes colonnes de survey invariantes à un gain constant par note (T60, accordage, inharmonicité). Il sert à attribuer l’effet du gain, et ne remplace pas le recalcul stéréo complet.

Reproduction après import des sources disponibles :

```sh
python3 tools/plath_set.py /mnt/local/SSD_B-CT4000/Piano/concert-piano-plath out/bench/sets/plath-calibrated
cargo build --offline --release -p piano-render -p piano-analysis
# La commande exacte des onze sources est conservée dans references-command.json.
python3 -c 'import json,subprocess; subprocess.run(json.load(open("bench/protocol-v2/references-command.json")),check=True)'
python3 tools/bench.py run --no-build --name protocol-v2
```

Le recalcul normal ne remplace pas la baseline ; `--save-baseline` doit être explicite. Les chemins des références dans la commande sauvegardée correspondent à cette machine.
