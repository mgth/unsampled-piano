# Analyse de Salamander Grand Piano V3

Source : [sfzinstruments/SalamanderGrandPiano](https://github.com/sfzinstruments/SalamanderGrandPiano), Yamaha C5, licence CC-BY 3.0 Unported (vérifiée dans le fichier LICENSE du dépôt), 48 kHz / 24 bits, stéréo (analysée en mono : moyenne des canaux). Clone local dans `data/salamander/`, ignoré par git.

Contenu : 30 notes (A0 à C8 par tierces mineures) × 16 couches de vélocité = 480 notes tenues jusqu'à extinction (25 s à A0, 3,6 s à C8). S'y ajoutent des bruits de relâchement (`rel1..88`), des résonances d'harmoniques (`harmL/S/V3*`) et des bruits de pédale (`pedalD/U*`), utiles pour les phases 3 et 5.

Correspondance couche → vélocité MIDI (d'après `Data/notes.txt`) : 1 : 1–26, 2 : 27–34, 3 : 35–36, 4 : 37–43, 5 : 44–46, 6 : 47–50, 7 : 51–56, 8 : 57–64, 9 : 65–72, 10 : 73–80, 11 : 81–88, 12 : 89–96, 13 : 97–104, 14 : 105–112, 15 : 113–120, 16 : 121–127.

Reproduire :

```bash
cargo run --release -p piano-analysis --bin piano-survey -- data/salamander/Samples --out out/salamander/survey.csv
python3 -I tools/survey_summary.py out/salamander/survey.csv
```

Sorties : `out/salamander/survey.csv` (une ligne par fichier et par partiel), `summary.md` (tableaux complets), `fitted.json` (lois ajustées), `summary.png`.

![synthèse](img/salamander_summary.png)

## Ce qu'on en tire

**Inharmonicité : mesure fiable, première loi réelle.**
- B ne dépend pas de la vélocité (coefficient de variation médian de 1,7 % entre couches), ce qui est cohérent avec une corde linéaire et valide l'estimateur.
- La forme bi-exponentielle de Rigaud décrit très bien ce piano (écart rms de 0,07 en log B). Paramètres réajustés : aB = −0,1117, bB = −6,107, aT = 0,0886, bT = −13,495.
- Ce Yamaha C5 est 10 à 25 % moins inharmonique que la moyenne de Rigaud au médium et dans l'aigu, et jusqu'à 2,3× moins dans le grave (A0 : 2,2e-4 contre 5,1e-4). Le minimum se situe vers F#2–A2 (≈ 7e-5), là où les cordes filées laissent la place aux cordes nues.
- Conséquence : B doit être un paramètre par instrument, pas une constante de la littérature.

**Accord étiré : courbe de Railsback retrouvée.**
- De −15 cents à A0 à +23 cents à A7, en passant par environ 0 au médium. Cela concorde avec les corrections que l'auteur de la banque applique dans son SFZ (F#7 : +25 cents mesurés contre −27 de correction, A7 : +37 contre −38).
- Anomalie : C8 sonne vers 4 410 Hz (≈ +90 cents) dès l'attaque, alors que la correction SFZ n'est que de −38 cents. Échantillon ou corde désaccordée : on l'exclut de la loi d'accord.

**Décroissances : la double décroissance domine, et une seule loi de pertes ne suffit pas.**
- La décroissance tardive est en médiane 4,5 fois plus lente que la précoce (interquartile 2,4–8,1). C'est le « prompt sound » et l'« aftersound » de Weinreich, dus au couplage des cordes d'unisson au chevalet.
- Les enveloppes sont fortement modulées par les battements des unissons (écart médian à une droite : 5 dB). Une loi de pertes σ_k = b1 + b2·f_k² ajustée sur les pentes précoces n'explique qu'une petite partie de la variance (R² < 0,5 pour presque toutes les notes).
- Tendance utilisable faute de mieux : log b1 = 0,0493·midi − 1,737 (b1 ≈ 0,5 s⁻¹ à A0, 3,4 à C4, 36 à C8).
- Le modèle bi-exponentiel ne passe le filtre de qualité que pour 351 lignes partiel sur 4 631 : quand il passe, la composante lente est ≈ 30 dB sous la rapide (interquartile −38 à −23 dB). Le reste du temps, les battements empêchent l'ajustement.
- Conséquence directe pour le plan : il faut séparer les composantes de chaque partiel (2 à 3 cordes légèrement désaccordées) par une méthode haute résolution (ESPRIT) pour mesurer les désaccords d'unisson et les amortissements individuels. La FFT n'y suffit pas. C'est aussi la preuve que la phase 2 (unissons couplés) est indispensable au réalisme.

**Vélocité.**
- 21,6 dB d'écart de crête entre les couches 1 et 16 (médiane sur les notes). C'est la plage de cette banque ; le manuel d'un piano modélisé commercial cite ≈ 50 dB pour un vrai piano, et la couche 1 couvre déjà les vélocités 1 à 26.
- Le centroïde d'attaque (50 premières ms) passe de 484 Hz à 1 083 Hz (×2,3). C'est la cible de brillance pour le modèle de marteau de la phase 1 (raidissement du feutre avec la vitesse, exposant p).

## Limites

- Mono (moyenne de deux micros) : des filtrages en peigne peuvent biaiser les niveaux de partiels, pas les fréquences.
- Les T60 incluent la salle et la prise de son.
- Les notes très aiguës (A7, C8) n'ont que 3 partiels mesurables, donc leur B est peu contraint.
- Un seul piano : il faudra un second instrument (Iowa MIS, Steinway) pour séparer ce qui relève du modèle de ce qui relève du réglage.

## Analyse haute résolution (ESPRIT en sous-bande)

Implémentation : `crates/piano-analysis/src/esprit.rs`. Pour chaque partiel, la bande voisine (±min(0,4·espacement, max(5 Hz, 7 cents))) est découpée dans une FFT longue et ramenée en bande de base. On applique ensuite ESPRIT (Hankel, SVD, invariance par décalage) jusqu'à 6 composantes amorties, puis on calcule les amplitudes par moindres carrés.

Validation sur signaux synthétiques (tests unitaires) : un bichorde à 0,6 Hz d'écart, un tricorde à 0,45 Hz d'écart, et deux décroissances à fréquence identique (3 et 0,3 s⁻¹, à −28 dB). Tous sont retrouvés à 0,01 Hz près pour la fréquence, 5 % pour σ et 0,5 dB pour l'amplitude.

```bash
cargo run --release -p piano-analysis --bin piano-survey -- data/salamander/Samples \
    --out out/salamander/survey.csv --esprit out/salamander/components.csv
python3 -I tools/esprit_summary.py out/salamander/components.csv
cargo run --release -p piano-analysis --bin piano-analyze -- data/salamander/Samples/C4v12.flac --note 60 --esprit 6
```

![ESPRIT](img/salamander_esprit.png)

Pour chaque partiel, on appelle « prompt » la composante la plus forte à t = 0, et « aftersound » la plus forte des composantes au moins 1,5 fois plus lentes (d'après le modèle de Weinreich : modes en phase et en opposition de phase de l'unisson couplé au chevalet). Tableaux complets : `out/salamander/esprit_summary.md`.

**La double décroissance est la règle.**
- On trouve un aftersound pour 86 % des partiels (couches ≥ 8).
- Au médium (C3–C6) : il démarre à −14 dB (médiane) sous le prompt et dure 2,6 fois plus longtemps.
- Dans le grave (A0–D#1) : il est plus faible (−27 à −32 dB).
- C'est très reproductible. Exemple de C4, partiel 1, dans les couches 8 à 16 : l'aftersound est à 261,48 Hz, T60 ≈ 33 s, −22 dB à chaque fois. La composante rapide est environ 1 cent plus basse, comme le prédit la théorie des cordes couplées.
- Sur l'ensemble du clavier, l'écart interquartile relatif du T60 d'aftersound entre couches est de 33 % en médiane.

**Les écarts de fréquence dans l'unisson sont petits.**
- L'étendue des composantes à plus de −20 dB va de 0,5 à 2 cents au médium (médiane 1,3) et de 3 à 5 cents dans l'aigu (médiane 4,0).
- Attention : ce sont les fréquences des **modes couplés**, pas le désaccord brut des cordes. Le désaccord brut s'obtiendra en inversant le modèle de couplage au chevalet de la phase 2. Ces écarts serviront de cible.

**Séparer pertes propres de la corde et pertes par le chevalet.**
- Le mode antisymétrique (aftersound) charge très peu le chevalet. Son σ approche donc les pertes propres de la corde (air, viscoélasticité).
- Δσ = σ_prompt − σ_after approche les pertes par le chevalet, proportionnelles à N·Re(Y_chevalet).
- Δσ croît d'environ 0,2 s⁻¹ vers 100 Hz à environ 10 s⁻¹ vers 5–10 kHz (nuage en bas à droite de la figure). C'est une trace indirecte de la conductance de la table, la grandeur qui manque dans la littérature sous forme numérique.
- Tendance des pertes propres : b1 ≈ exp(0,059·midi − 3,98) s⁻¹ (0,15 à A0, 0,5 à D#3, 3,5 à A5, 6,8 à A7) ; b2 ≈ 7e-8 s (médiane, notes ≥ A2).
- La loi b1 + b2·f² reste un ajustement médiocre partiel par partiel (R² médian 0,31 pour la corde, 0,14 pour le chevalet). Les pertes réelles ne sont pas une fonction lisse de la fréquence : les résonances de la table modulent Re(Y). Le modèle devra prendre la conductance du chevalet en fonction de la fréquence plutôt qu'une loi en f². C'est un argument direct pour la table modale de la phase 4 couplée dès la phase 2.

**Limites de cette analyse.** Le nombre de cordes par note n'est pas connu, donc N n'est pas séparé dans Δσ. Dans le grave, le fondamental est trop faible pour être analysé (A0–D#1). Les composantes au-delà de 3 par partiel (polarisations, modes de table) sont présentes mais pas interprétées.
