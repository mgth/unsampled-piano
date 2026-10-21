# Phase 0 — outillage et mesures

État au 2026-10-21. Machine de mesure : AMD Ryzen 9 9950X (Zen 5, 16 cœurs / 32 threads, AVX-512), Rust 1.99 stable, 48 kHz, blocs de 128.

## Ce qui existe

| Crate | Rôle |
|---|---|
| `piano-core` | Moteur DSP sans dépendance de framework : banc modal SIMD (`modal`), moteur polyphonique (`engine`), paramètres (`params`), garde FTZ/DAZ (`denormal`). |
| `piano-render` | Rendu hors ligne déterministe MIDI→WAV (`midi`, `note`, `sweep`), affiche un hash du rendu. |
| `piano-analysis` | Suivi de partiels, estimation (f0, B), enveloppes par partiel, T60 simple et double, distance MR-STFT, centroïde. CLI `piano-analyze`. |
| `piano-bench` | Expériences : `throughput` (débit du banc modal) et `precision` (erreur f32). |

La voix de phase 0 est **une corde modale physique** (projection modale de la force au point de frappe, force au chevalet `4·f0·Σ sin(kπx0)/√(1+Bk²)·e^{-σt}·sin(ω_k t)`, B de Rigaud 2013) excitée par une **force imposée** (demi-sinus dont la durée suit la loi de contact de Stulov sur surface rigide et dont l'impulsion vaut 2·m·V). Ce n'est pas encore le contact couplé marteau–corde (phase 1). Les pertes (b1, b2) et l'étouffoir sont des valeurs provisoires non ajustées.

## Résultat 1 — le CPU n'est pas le facteur limitant

Capacité = nombre de modes amortis entretenus en temps réel par un cœur (`piano-bench throughput`).

| ISA | groupe | sans entrée | avec entrée |
|---|---|---|---|
| générique (SSE2) | 1 | 177 000 | 114 000 |
| AVX2 | 2 | 420 000 | 287 000 |
| AVX-512 | 4 | **842 000** | **580 000** |

- Multi-thread (AVX-512) : 8 threads → 6,5 M modes ; 16 threads → 9,7 M ; 32 threads (SMT) → 12,5 M.
- Le rapport supposait 2×10⁴ à 10⁵ modes par cœur : la réalité est **~10× au-dessus**. Un piano entier (88 notes × 3 cordes × ~200 modes ≈ 53 000 modes, toutes cordes libres) ne coûterait qu'environ 6 % d'un cœur Zen 5 pour le seul banc linéaire, ou ~20 % d'un cœur AVX2.
- Grouper plusieurs blocs de 16 modes dans la même boucle temporelle masque la latence de la récursion. En AVX2, le groupe de 4 déborde des 16 registres ymm et perd ; le groupe par défaut est donc choisi selon l'ISA (`Isa::best_group`).
- Ce chiffre ne couvre que le banc linéaire. Le contact marteau, le couplage au chevalet et les produits longitudinaux ont des parties moins vectorisables ; ils restent à mesurer en phase 1–2.
- Conséquence : on peut se permettre plus de modes, un couplage plus riche, voire un suréchantillonnage local, sans sortir du budget d'un cœur.

## Résultat 2 — f32 suffit à 48 kHz avec la forme complexe, pas avec un biquad

`piano-bench precision` : erreur due à l'arrondi des coefficients en f32.

| fs | fréquence | forme complexe (cents / σ) | biquad réel (cents / σ) |
|---|---|---|---|
| 48 kHz | 27,5 Hz (A0) | 0,0001 / 0,1 % | **1,6** / 0,1 % |
| 96 kHz | 27,5 Hz (A0) | 0,0000 / 0,3 % | **10,3** / 0,4 % |
| 96 kHz | 55 Hz | 0,0000 / 0,3 % | 1,6 / 0,1 % |

- La forme complexe (rotation) est exacte en fréquence ; le biquad direct dérive de plusieurs cents dans le grave, au-delà du seuil de perception des battements. Choix confirmé : forme complexe.
- Erreur d'arrondi accumulée sur 30 s de décroissance libre de A0 : −74 dB à 48 kHz, −58 dB à 96 kHz. Acceptable à 48 kHz ; à 96 kHz et au-delà, prévoir un état f64 (ou une renormalisation) pour les modes graves très peu amortis. Cela tranche la question ouverte du rapport.

## Résultat 3 — les extracteurs retrouvent les paramètres injectés

- Signaux synthétiques (tests unitaires) : B à moins de 3 % et f0 à moins de 0,2 cent de A0 à C7 ; T60 simple à 1 % ; double décroissance (σ1, σ2) à 5 % et rapport des composantes à 1 dB.
- Boucle synthèse → analyse sur nos propres rendus : A0 : B = 5,094e-4 (injecté 5,094e-4), f0 à 0,02 cent ; T60 des 4 premiers partiels à moins de 0,5 % ; C4 : B = 3,270e-4 (injecté 3,269e-4), T60 identiques au centième.
- Le rendu est identique au bit près d'une exécution à l'autre et quels que soient l'ISA et le groupe (pas de contraction FMA, ordre de sommation fixe) ; test `all_isas_and_groups_are_bit_identical`.
- Le chemin audio n'alloue pas (test `realtime.rs` avec `assert_no_alloc`, 1 500 blocs de notes, vélocités, relâchements et pédale).

## Reste à faire pour clore la phase 0

1. ~~Données Salamander~~ : fait, voir [salamander.md](salamander.md). Restent Iowa MIS (second piano) et la numérisation des courbes de Conklin et Giordano.
2. ~~ESPRIT~~ : fait, voir [salamander.md](salamander.md#analyse-haute-résolution-esprit-en-sous-bande).
3. Liaisons Python (pyo3) si l'on veut ajuster avec scipy ; FAD avec un plongement déclaré.
4. `rtsan-standalone` en CI, et une mesure sur une machine ARM.

## Commandes

```bash
cargo test --workspace
cargo run --release -p piano-bench -- throughput --modes 256,2048,16384 --threads 1,8,16
cargo run --release -p piano-bench -- precision
cargo run --release -p piano-render -- note out/c4.wav --note 60 --velocity 100 --hold 6
cargo run --release -p piano-render -- midi morceau.mid out/morceau.wav
cargo run --release -p piano-analysis --bin piano-analyze -- out/c4.wav --note 60 --until 5.9
```
