# Phase 4 — table d'harmonie, rayonnement, micros

État au 2026-10-21. La force au chevalet de chaque touche excite une table d'harmonie modale. Ses modes rayonnent vers deux micros (sortie stéréo), et la même table fournit l'admittance qui couple les cordes de chaque unisson.

## Modèle

**Plaque** (`soundboard.rs`).
- Plaque orthotrope (épicéa et barrages) de 1,7 × 1,3 m, encastrée dans la ceinture.
- Fréquences par l'approximation de Warburton pour bords encastrés (m → m + ½), déformées approchées par des sinus normalisés en masse.
- Facteur de pertes η = 2 % (Ege et al. 2013).
- 1 083 modes sous 20 kHz, premier mode à 61 Hz, un mode tous les 21,6 Hz (Ege : 19,5 Hz).
- Les bords simplement appuyés mettaient le premier mode à 27 Hz, pile sur la fondamentale de A0 : A0 ressortait alors 57 dB au-dessus de C4.
- ρh/D fixe la densité modale ; ρh·D fixe le niveau de mobilité, calé sur les décroissances de Salamander (voir plus bas). Résultat : ρh = 7 kg/m², Dx = 3 825 N·m, Dy = 1 700 N·m, mobilité moyenne 9,4·10⁻⁴ s/kg, du même ordre que les 1,3·10⁻³ mesurés par Ege.

**Admittance vue par les cordes.**
- Y(x, ω) = Σ iω·Φ_n²/(ω_n² − ω² + iη·ω_n·ω), évaluée au point exact de chaque touche sur le chevalet (chevalet principal en ligne brisée, chevalet des basses séparé).
- Une raideur série de 2·10⁷ N/m (pointes, chapeau du chevalet) y ajoute iω/k. C'est elle qui donne la partie réactive « ressort » que la phase 2 avait dû introduire comme Im Y = 1,5·Re Y.
- Une masse locale du chevalet, Y/(1 + iω·m·Y), est disponible ; le calage la met à 0.
- Le couplage ε = (T/L)·Y de chaque partiel porte maintenant les pics et creux de la table.

**Rayonnement** (`instrument.rs`).
- Pour chaque mode et chaque micro, l'intégrale de Rayleigh (plaque bafflée) est calculée une fois, à la fréquence du mode, sur une grille de 1,5 cm : p = iωρ₀/(2π)·∫Φ·e^{−ikR}/R dS.
- Deux micros à 1,5 m au-dessus de la table, réglables (`SoundboardParams::mics`).

**Rendu** (`engine.rs`, `modal.rs`).
- Les touches sont groupées en 16 zones du chevalet. Chaque zone pilote un banc SIMD des modes de la table : gain d'entrée Φ_n(x_zone)·T, et deux sorties complexes, une par micro.
- Le noyau modal calcule désormais deux sorties dans la même passe (`process2`).
- Un banc de zone sans entrée et devenu silencieux est mis en sommeil.
- L'instrument (table des notes et données de la table) est précalculé en ≈ 0,35 s et partagé par `Arc` (`Engine::with_instrument`). C'est aussi la base pour reconstruire hors du thread audio quand un paramètre change.
- Sortie stéréo (`process_stereo`), et mono = moyenne des deux micros.

## Performance

- **Piège trouvé.** Avec les poids de sortie complexes ajoutés en phase 2, un banc *piloté* (avec entrée) traité par groupes de 4 blocs de 16 modes déborde des registres : il devient environ 20 fois plus lent sur toutes les ISA (1,05 ns/mode·échantillon au lieu de 0,06). Les bancs pilotés sont désormais limités à des groupes de 2, et les bancs pilotés à double sortie à des groupes de 1 en AVX2.
- **Pire cas** (clavier entier à vélocité 127, une note toutes les 30 ms, pédale enfoncée, 88 voix plus 16 bancs de table de 1 083 modes) : 4,64 s de son rendus en 0,85 s, dont 0,35 s de construction de l'instrument. Soit ≈ 9× le temps réel sur un seul thread, ≈ 11 % d'un cœur Zen 5.

## Validation

**Une prédiction non ajustée qui tombe juste.** La table n'a été contrainte que par sa densité modale et son niveau moyen de mobilité. La *distribution* de sa conductance au chevalet reproduit pourtant celle extraite des décroissances de Salamander.

![admittance](img/soundboard_admittance.png)

**Décroissances** : statistiques ESPRIT mesurées sur le son rayonné, exactement comme sur l'enregistrement (calage par `tools/coupling_grid.py`).

| | Loi lisse, phase 2 (force au chevalet) | Table modale, phase 4 (son rayonné) |
|---|---|---|
| T60 du prompt (écart médian) | 21 % | 49 % |
| T60 de l'aftersound | 27 % | 52 % |
| Niveau de l'aftersound | 2,1 dB | 2,0 dB |
| Notes à une corde : niveau de l'aftersound | 1,7 dB | 1,1 dB |
| Taux de détection de l'aftersound (écart) | 0,26 | 0,14 |
| Score global | 2,41 | 2,93 |

![unissons](img/phase4_unison.png)

- Les T60 sont un peu moins justes qu'avec la loi lisse : celle-ci avait été ajustée directement sur ces statistiques, alors que la table impose ses pics et creux.
- La détection et les niveaux de l'aftersound sont meilleurs.
- Le son analysé inclut maintenant la réverbération propre de la table.

**Spectre** (`tools/spectral_stats.py`). On compare des statistiques, pas les partiels un à un : une table modale réaliste place ses résonances ailleurs que celles d'un autre piano, et une comparaison partiel par partiel la pénalise mécaniquement.

| Bande (Hz) | 0–100 | 100–200 | 200–400 | 400–800 | 800–1,6 k | 1,6–3,2 k | 3,2–6,4 k | > 6,4 k | Fluctuation |
|---|---|---|---|---|---|---|---|---|---|
| Salamander | +11,8 | +14,0 | +5,3 | +1,0 | +0,7 | −0,6 | −12,2 | −23,4 | 5,3 dB |
| Sans table (phase 2) | +6,0 | +9,9 | +2,0 | +0,5 | −1,0 | +0,2 | −1,3 | −9,9 | 3,8 dB |
| Avec table (phase 4) | +13,3 | +8,3 | +2,5 | +1,5 | −0,4 | +0,7 | −1,8 | −10,0 | 6,7 dB |

Niveaux en dB, relatifs à la médiane des partiels de chaque note.
- La table corrige le déficit de l'extrême grave.
- Elle apporte la rugosité spectrale d'un vrai piano : sans elle, le spectre est trop lisse.
- La dynamique et la brillance en fonction de la vélocité restent calées : centroïde ×0,65 / ×1,59 contre ×0,65 / ×1,51 pour la référence.

![phase 4](img/phase4_eval.png)

## Problème ouvert : trop d'aigus au-dessus de 3 kHz

Entre 3,2 et 6,4 kHz, la référence est 10 dB sous le modèle ; au-delà, 13 dB. Par note, l'écart de pente croît de +4 dB/octave à C4 à +8–20 dB/octave dans l'aigu. Ce qui a été testé et écarté :

| Mécanisme | Outil | Effet |
|---|---|---|
| Masse du chevalet | `tools/board_grid.py` | améliore un peu au-dessus de 6,4 kHz, mais dégrade les décroissances |
| Pertes de la table croissant avec la fréquence | `tools/board_grid.py` | négligeable |
| Hystérésis du feutre ×3 | `tools/board_grid.py` | négligeable |
| Largeur de contact ×2 | `tools/board_grid.py` | négligeable |
| Hauteur des micros | `tools/board_grid.py` | négligeable |
| Feutre beaucoup plus doux dans l'aigu (Q0 ÷50 à C8) | `tools/voicing_grid.py` | l'écart ne bouge presque pas |
| Décroissances des partiels aigus | — | le modèle décroît plutôt *plus vite* que la référence |

Pistes restantes, par ordre de vraisemblance :
1. La localisation de la vibration entre les barrages au-dessus de ~1 kHz (Ege 2013). La plaque homogène ne la représente pas, et elle réduit la mobilité et le rayonnement du chevalet aigu.
2. Une vraie échelle de cordes : longueurs, points de frappe et tensions de l'aigu, pour l'instant génériques.
3. La loi héréditaire complète du feutre de Stulov, plutôt que l'hystérésis linéarisée.
4. La chaîne d'enregistrement de Salamander (micros, salle, égalisation éventuelle). Un second piano de référence (Iowa MIS) dirait ce qui relève du modèle et ce qui relève de cet enregistrement.

## Commandes

```bash
cargo run --release -p piano-core --example soundboard_admittance > out/admittance.csv
cargo run --release -p piano-core --example build_time
cargo run --release -p piano-render -- note out/c4.wav --note 60 --velocity 90 --hold 8   # WAV stéréo
tools/unison_trial.sh essai --series-stiffness=2e7
python3 -I tools/coupling_grid.py
python3 -I tools/spectral_stats.py ref_et_modele=out/salamander/eval_phase4.csv
```

> Note (phase 3) : les 16 bancs de zones ont été remplacés par un banc unique multi-entrées / multi-sorties qui fournit aussi la vitesse du chevalet. Un terme direct corrige l'invariance impulsionnelle : sans lui, la conductance rendue hors résonance était fausse dans le grave. La polarisation a été rendue passive. Voir [phase3.md](phase3.md).
