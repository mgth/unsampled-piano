# Phase 3 — étouffoirs, pédales, résonance sympathique

État au 2026-10-21. Les étouffoirs, les trois pédales et la résonance sympathique sont modélisés physiquement. Le passage par la table d'harmonie en a imposé une refonte importante.

## Étouffoirs (`params::DamperLaw`, `voice.rs`)

Un feutre est un amortisseur de coefficient c posé sur la corde en x_d = 0,14·L, sur une largeur de 2 cm. Il ajoute au partiel k le taux de décroissance :

```text
σ_k = c·⟨sin²(kπx/L)⟩_feutre/(μL) · (2Z0/(c + 2Z0))²,   c = e·2Z0,   Z0 = √(Tμ)
```

- Le dernier facteur traduit la désadaptation d'impédance : un feutre trop dur réfléchit au lieu d'absorber. L'absorption est maximale pour c = 2Z0.
- La moyenne sur la largeur du feutre empêche les partiels qui ont un nœud au point de contact de sonner indéfiniment. C'est un défaut trouvé par le test du sostenuto.
- e ∈ [0, 1] est le contact du feutre. Il suit sa cible avec une constante de 12 ms à la descente et de 5 ms à la levée.
- Seules les touches jusqu'à E6 ont un étouffoir.
- Côté calcul, le banc modal ne recalcule que la décroissance de chaque mode : une exponentielle par mode, sans trigonométrie (`ModalBank::set_damping`).

Résultats (tests `tests/pedals.rs`, T60 initial de l'énergie des cordes après le relâchement) :

| Note | T60 initial | Commentaire |
|---|---|---|
| E1 | 0,45 s | les étouffoirs graves sont moins efficaces, sans réglage |
| C4 | 0,12 s | |
| E7 | 0,86 s | pas d'étouffoir : ne change pas au relâchement |

Après cette chute rapide, une traîne plus lente subsiste : celle des partiels peu touchés par le feutre, comme sur un vrai piano.

## Pédales (`engine.rs`)

- **Sustain (CC64 continu)** : la course de pédale fixe le contact de tous les étouffoirs dont la touche est relevée, e = ((0,4 − course)/0,4)^1,5. La demi-pédale est donc physique. Sur C4, le T60 après relâchement vaut 0,12 s pédale levée, 0,22 s à mi-course et 5,9 s pédale enfoncée.
- **Sostenuto (CC66)** : il mémorise les touches enfoncées au moment où on appuie, et seules celles-là restent libres.
- **Una corda (CC67 continu)** : le mécanisme se décale. Le marteau quitte la première corde, et la frappe ne porte plus que sur les autres (paramètre `hit` du solveur de contact). Il frappe aussi une zone de feutre plus douce : Q0 × (1 − 0,35). La corde non frappée n'est excitée que par le couplage au chevalet de la phase 2. Résultat sur C4 : −2,8 dB.

## Résonance sympathique

### Architecture : un sens unique, inconditionnellement stable

Les cordes étant linéaires, leur mouvement est la somme de deux réponses :
1. la réponse au marteau (banc *primaire*, dont le couplage propre au chevalet est déjà dans les modes couplés) ;
2. la réponse au mouvement du chevalet dû aux *autres* touches (banc *secondaire*, mêmes modes).

Seules les forces primaires font bouger le chevalet pour les cordes. Il n'y a donc aucune boucle de rétroaction : le système est stable pour tout gain, comme chez Bank, Zambon et Fontana (2010). Les deux réponses rayonnent. On néglige la résonance de second ordre, quand une corde excitée par sympathie en excite une troisième.

**Table unique multi-entrées / multi-sorties** (`board.rs`) :
- Un seul banc de 1 083 modes avec 16 entrées (forces par zone du chevalet) et 18 sorties (2 micros, plus la vitesse du chevalet aux 16 points de zone).
- Il remplace les 16 bancs de la phase 4.
- Un second banc reçoit les forces sympathiques et ne sert qu'aux micros.

**Excitation d'une corde par son extrémité mobile.** L'équation des ondes avec y(0, t) = u(t) donne la force modale exacte −(μL/kπ)·ü. Par corde et pour une entrée d'accélération réelle, le gain vaut i·2·dt/(kπ·ω_k).

Cette forme en accélération est indispensable. La forme w_k·u, proportionnelle au déplacement, n'est juste qu'à la résonance. Hors résonance, elle fait suivre aux modes le mouvement lent de la table, c'est-à-dire la déformée statique, et la somme sur k produit une force parasite énorme.

**Soustraction de soi.** L'entrée d'une touche contient sa propre contribution, déjà comptée dans ses modes couplés. Elle est retirée par une seconde entrée, sa propre force multipliée par le transfert rendu H du banc de table, évalué à la fréquence exacte de chaque mode couplé. Dans l'aigu, cette boucle propre est forte : sans soustraction, la réponse sympathique de E7 dépasserait sa vibration directe de 13 dB. Avec, elle reste 27 dB en dessous.

**Retard et phase.** La table est traitée après les cordes : leur entrée arrive donc avec un retard. Le moteur travaille en sous-blocs de 64 échantillons, avec des tampons circulaires qui rendent ce retard *exactement* constant (64,5 échantillons, en comptant la différence finie qui donne l'accélération). Chaque gain d'entrée est avancé de e^{iω·D} : à la fréquence du mode, le couplage garde sa phase physique.

**Cordes à l'écoute.**
- Quand la pédale lève les étouffoirs, chaque touche reçoit une corde « à l'écoute ». Ces cordes sont créées 8 par sous-bloc, pour lisser le coût.
- Elles ne font tourner que leurs partiels sous 5 kHz.
- Les touches sans étouffoir écoutent en permanence.

Résultat (pédale enfoncée, C3 bref) : énergie sympathique 3,0 pour l'octave C4, 1,7·10⁻² pour la douzième G4, 1,5·10⁻⁴ pour le demi-ton C#4. Dans la démo « halo », on mesure −45 dB après l'accord sans pédale, contre −28 dB avec.

### Erreurs trouvées en chemin

1. **Boucle fermée retardée.** La première version (les cordes réinjectent dans la table qui les excite) divergeait avec deux cordes à l'écoute quelconques. Un retard d'un bloc fait tourner la phase du couplage, et la dissipation devient amplification. D'où le sens unique.
2. **Bride passive.** Le couplage croisé entre polarisations calé en phase 2 (0,5·ε avec un couplage horizontal de 0,1·ε) violait ε_X² ≤ ε_V·ε_H : le chevalet n'était plus passif. Avec la table modale, certains modes avaient alors un amortissement *négatif*. Correction : couplage horizontal 0,3, croisé 0,5 (second meilleur réglage de la phase 2, score quasi identique), avec un écrêtage automatique (`passive_cross`). Un test vérifie que tous les modes de l'instrument complet décroissent.
3. **Discrétisation de la table.** L'invariance impulsionnelle compte en entier, au premier échantillon, le saut de vitesse qu'un choc impose à chaque mode. Cela ajoutait une constante parasite par mode, et sur 1 000 modes la conductance rendue à 80 Hz était 20 fois trop forte. Un terme direct de correction est ajouté, au rendu comme dans le transfert H. Un test vérifie maintenant que la mobilité rendue reste à 8 % près de la mobilité continue.
4. **Performance.** Le noyau multi-sorties était environ 30 fois trop lent. Le passer en deux passes (récurrence avec historique en L1, puis sorties en flux) ne suffisait pas : le code AVX-512 généré par LLVM pour ce noyau reste environ 10 fois plus lent que le code AVX2 sur Zen 5. La table tourne donc en AVX2.

## Coût

Pire cas : clavier entier à vélocité 127, une note toutes les 30 ms, pédale enfoncée, 88 voix primaires et secondaires, deux bancs de table. On mesure **4,65× le temps réel, soit 22 % d'un cœur**, dont environ 9 % pour la sympathie. La construction de l'instrument prend 0,8 s, dont le transfert de soi calculé mode par mode.

## Ce qui reste ouvert

- Le niveau absolu de la résonance sympathique n'est pas calé sur des mesures. Les échantillons « résonance au relâchement » de Salamander mêlent étouffoirs et sympathie. Le sens unique néglige la réaction en retour : il surestime un peu la réponse des cordes très couplées, celles de l'aigu.
- Pas de bruits d'étouffoir ni de pédale : c'est la phase 5, mimétique.
- La loi de demi-pédale (point de contact 0,4, exposant 1,5) est générique.

## Démos et commandes

```bash
python3 -I tools/make_demo_midi.py out/demos
cargo run --release -p piano-render -- midi out/demos/pedal_halo.mid out/demos/pedal_halo.wav
cargo test -p piano-core --test pedals -- --nocapture
cargo run --release -p piano-core --example stress_cost
cargo run --release -p piano-core --example sympathy_debug
```

Démos rendues : `pedal_halo` (même accord sans puis avec pédale), `half_pedal` (relâchements pédale levée, à mi-course, enfoncée), `una_corda` (accord normal puis una corda), `sostenuto` (C2 tenu sous une mélodie étouffée).
