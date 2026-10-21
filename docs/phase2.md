# Phase 2 — unissons couplés au chevalet

État au 2026-10-21. Chaque touche a 1 à 3 cordes, chacune avec ses deux polarisations, couplées par l'admittance du chevalet. On obtient la double décroissance (prompt sound / aftersound) et les battements, calés sur Salamander.

## Modèle

**Couplage de Weinreich** (`unison.rs`). Pour un partiel k, chaque oscillateur (corde j, polarisation verticale ou horizontale) suit ζ̇ = d·ζ avec d = −σ0 + iω. Le chevalet, qui bouge sous la somme des forces des cordes, renvoie la même excitation à toutes :

```text
A = diag(d) − ε_V·1_V·1_Vᵀ − ε_H·1_H·1_Hᵀ − ε_X·(1_V·1_Hᵀ + 1_H·1_Vᵀ),   ε_V = (T/L)·Y(f)
```

- Re ε_V est la perte d'une corde au chevalet ; Im ε_V écarte les fréquences.
- ε_H et ε_X valent 0,1·ε_V et 0,5·ε_V. La partie horizontale du chevalet est beaucoup plus raide, et le terme croisé (bascule du chevalet) transfère l'énergie entre polarisations.
- A est complexe symétrique. Ses vecteurs propres se normalisent en Vᵀ·V = I, donc V⁻¹ = Vᵀ.
- Solveur général jusqu'à 6×6, sans allocation : polynôme caractéristique de la matrice centrée (Faddeev–LeVerrier), racines de Durand–Kerner, affinage par itération inverse et quotient de Rayleigh.
- Centrer est indispensable : les valeurs propres valent ~10⁴ rad/s mais ne diffèrent que de ~10⁻².
- La table complète (88 notes, ≤ 768 modes couplés chacune) est précalculée en 33 ms à la création du moteur (`note_model.rs`). Une frappe ne fait que copier.
- Sur tout le piano : |V_jm| ≤ 1,13, donc aucun point exceptionnel ni annulation catastrophique en f32.

**Rendu** (`voice.rs`, `modal.rs`).
- Le banc SIMD fait tourner les modes couplés, avec des poids de sortie complexes : force au chevalet = Re(w·z).
- Pendant le contact du marteau (quelques ms), le solveur fait évoluer les oscillateurs physiques non couplés : le couplage agit sur 1/|ε| ≈ 0,1–1 s.
- À la fin du contact, l'état est rendu au banc par c = Vᵀ·ζ.
- Une refrappe fait l'inverse : ζ = V·c.

## Une mesure physique tirée des enregistrements : la conductance du chevalet

Si le modèle est juste, la différence de décroissance entre prompt et aftersound mesurée par ESPRIT, Δσ = N·(T/L)·G(f), doit donner une conductance G = Δσ·L/(N·T) qui ne dépend que de la fréquence (`tools/bridge_conductance.py`).

![conductance](img/bridge_conductance.png)

- Δσ varie de 0,1 à 30 s⁻¹. G, lui, est presque plat : G(f) ≈ 4,6·10⁻⁴·(f/1 kHz)^0,11 s/kg.
- C'est le même ordre de grandeur que la mobilité moyenne mesurée sur une table par Ege et al. (2013), 1,3·10⁻³ s/kg.
- La dispersion résiduelle, un facteur ≈ 2,5, est la signature attendue des modes de la table d'harmonie (phase 4).
- Ce G mesuré sert directement de paramètre du modèle.

## Calibrage sur les statistiques ESPRIT

Les rendus du modèle passent par exactement la même chaîne d'analyse que les enregistrements (`tools/unison_trial.sh` : `piano-eval --write-wav`, puis `piano-survey --esprit`, puis `esprit_summary.py`). Les recherches sur grille (`tools/unison_grid.py`, `tools/pol_grid.py`) ont fixé :

| Paramètre | Valeur | Remarque |
|---|---|---|
| Largeur d'unisson (écart entre cordes extrêmes) | 1 cent jusqu'à C4 → 5 cents à C8 | répartition irrégulière par note |
| Im Y / Re Y | 1,5 | l'écart de fréquences dans l'unisson exige une partie imaginaire |
| G (conductance) | mesurée, sans correction | le facteur ×0,7 ou ×1,6 dégrade le score |
| Polarisation horizontale | couplage 0,1·ε, transfert 0,5·ε, +0,3 cent, rayonnement 10 %, marteau incliné 5 % | |
| Pertes propres des cordes | loi de l'aftersound ESPRIT | `LossLaw::SALAMANDER_STRING` |

![unissons](img/phase2_unison.png)

**Écarts médians au modèle (couches ≥ 8, partiels k ≤ 12).**

| Grandeur | Écart médian |
|---|---|
| T60 du prompt | 21 % |
| T60 de l'aftersound | 27 % |
| Niveau de l'aftersound | 2,1 dB |
| Étendue en cents | 0,5 cent |
| Notes à une corde (A0–D#1) : T60 de l'aftersound | 11 % |
| Notes à une corde (A0–D#1) : niveau de l'aftersound | 1,7 dB |

Sans la polarisation horizontale, ces notes n'ont aucun aftersound et le score global passe de 2,41 à 5,85.

**Ce qui reste.**
- Dans l'aigu, l'étendue mesurée en cents est irrégulière (accordage réel), celle du modèle lisse.
- Dans l'aigu, l'aftersound de quelques notes (A6, C7, A7) tombe trop bas.
- Le T60 du prompt des notes à une corde est 1,7× trop long : les pertes au chevalet des cordes filées graves ne suivent pas la loi T/L générique.

## Comparaison spectrale

![phase 2](img/phase2_eval.png)

- **Métrique de profil corrigée.** L'ancienne normalisait chaque profil sur son partiel le plus fort. Quand le suivi de partiels perdait ce partiel, tout le profil se décalait, d'où un 8,5 dB peu fiable en phase 1. La nouvelle compare les niveaux des partiels communs à une constante près.
- **Suivi de partiels corrigé.** Le plancher du rejet des aberrants était de 0,05 cent. Il est relevé à 0,5 cent : le couplage au chevalet décale légèrement les partiels bas, comme sur un vrai piano.
- Avec ces deux corrections, l'écart de profil médian est de **5,5 dB**. La configuration équivalente à la phase 1 donne 5,8 dB.
- Par note, l'écart va de 2,9 à 7 dB dans le grave et le médium, et de 6 à 14 dB dans l'aigu (A7 : 14,3 dB).
- Le reste est attendu de la table d'harmonie, du rayonnement et d'une vraie échelle de cordes.
- Dynamique et brillance restent calées : centroïde ×1,59 en fortissimo contre ×1,51 pour la référence, ×0,69 contre ×0,65 en pianissimo. Le marteau n'a pas eu besoin d'être recalé.

## Coût

Clavier entier à vélocité 127, une note toutes les 30 ms, pédale enfoncée : 88 voix (256 à 768 modes couplés chacune) rendues 15,9 fois plus vite que le temps réel sur un seul thread, soit ≈ 6 % d'un cœur Zen 5. Le gain de sortie est abaissé de 6 dB : crête −1,2 dBFS dans ce pire cas.

## Pistes

1. **Phase 3** : étouffoirs physiques, demi-pédale, sostenuto, una corda (le marteau ne frappe que 2 des 3 cordes : déjà prévu par le champ `hit` du solveur), résonance sympathique par le chevalet.
2. **Phase 4** : table d'harmonie modale. Elle remplacera la loi lisse G(f) par une vraie admittance avec ses pics et ses creux. C'est la principale source d'écart restante, en spectre comme en décroissance.
3. Pertes spécifiques aux cordes filées graves, et échelle de cordes réelle.

## Commandes

```bash
cargo test --workspace
tools/unison_trial.sh essai --unison-mid=1.0 --bridge-b-over-g=1.5
python3 -I tools/unison_grid.py
python3 -I tools/pol_grid.py
python3 -I tools/bridge_conductance.py out/salamander/components.csv
cargo run --release -p piano-render -- note out/c4.wav --note 60 --velocity 90 --hold 10
```
