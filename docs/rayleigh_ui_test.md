# UI de test : contacts retenus et rayonnement intégral

29 octobre 2026, dans `codex/piano-main-revalidation`, après `02a7005`.

La version de test joue maintenant le **rayonnement intégral en temps réel**, avec les contacts par touche retenus dans [l’étude Do8 au pianiste](rayleigh_c8_player.md). L’UI affiche un bandeau **TEST — Rayleigh intégral · contacts retenus · pianiste, couvercle 35°, studio**. Le défaut, les presets livrés et la configuration habituelle restent inchangés. La nouvelle fonction est compilée seulement avec la feature `experimental-rayleigh` et activée seulement avec `--experimental-rayleigh BUNDLE`.

## Ce qui est effectivement joué

- La table reçoit toujours les forces primaires et sympathiques du moteur ; le retour mécanique vers les cordes, le cadre, la ceinture et l’action sont conservés.
- Le rayonnement de la table est remplacé entièrement, avant la salle, par les 16 × 2 réponses impulsionnelles causales du prototype. Les **65 536 coefficients** sont conservés, sans égalisation, gain ajusté ni troncature supplémentaire.
- La convolution utilise trois tailles de partitions (128, 512, 2 048 échantillons) en double précision, avec plans FFT, historiques et mémoire préparés hors du callback audio. Aucun thread auxiliaire de convolution ni file d’attente asynchrone ne modifie le délai.
- Un retard commun de **128 échantillons, 2,67 ms à 48 kHz**, aligne rayonnement, cadre, ceinture et action. La salle est appliquée ensuite. Ce retard s’ajoute au tampon audio.
- Les contacts des notes 93–108 reprennent le profil gelé du banc : Fa♯7 avec q=1, h=0,3, frappe=0,105, hampe 300 Hz/pertes 2 ; Do8 `recovered` avec q=0,3, h=0,6, Δp=−1, hauteur d’unisson 15 µm, hampe 300 Hz/pertes 0,05. Les autres touches de cette plage reprennent leurs contacts précédents, dont Do♯7 et Ré♯7 revenus au contrôle. Sous 93, le contact est celui du modèle de base du banc.
- Il s’agit de paramètres **par touche**, pas d’un preset de Do8 appliqué globalement. La dynamique continue de provenir du contact non linéaire et des vélocités MIDI ; aucun audio préenregistré n’est joué.

Le point d’écoute lancé est le véritable preset pianiste : couvercle 35°, micros omnis, gain −6,1 dB et salle studio fixe. Le modèle conserve les limites géométriques de la recherche : contour réel sous 1,5 kHz, modes rectangulaires au-dessus, surface rayonnante découpée. Le bruit d’action n’est pas encore spatialisé selon les oreilles ; cette intégration ne prétend pas corriger le dernier écart diagnostiqué.

## Comparaison à couverture constante

Le même `Engine` que dans l’UI produit les WAV de validation ; aucun post-traitement externe ne lui ajoute le rayonnement. Seul le retard commun connu de 128 échantillons est retiré des fichiers de comparaison. Les fichiers ne sont pas normalisés.

| Contrôle | Résultat |
|---|---:|
| Rendus aux micros proches | 48 attaques + 400 vélocités |
| Rendus au pianiste | 48 attaques + 400 vélocités |
| Observations historiques conservées | 459 attaques + 71 tenues |
| Erreur RMS maximale face au prototype, micros proches | −149,14 dB |
| Erreur RMS maximale face au prototype, pianiste | −140,34 dB |
| Variation maximale des partiels d’attaque admissibles | 0,000162 dB |
| Variation maximale de tenue, toutes lignes conservées | 0,01830 dB |

La variation de tenue maximale porte sur P4 de Ré♯7 pp, autour de −112 dB/P1, avec un SNR candidat de 6,74 dB dans l’étude antérieure. Cette ligne reste incluse. Les tolérances de portage sont de −140 dB sur les WAV, 0,001 dB sur les attaques et 0,05 dB sur toutes les tenues ; elles qualifient l’équivalence numérique, pas le réalisme du piano. Les écarts aux pianos réels ne sont pas retouchés. Les creux P2/P3 et les niveaux de Do8/Fa♯7 restent ceux du profil retenu à la précision numérique du calcul.

Le premier port en simple précision donnait −126,6 dB au pire, mais jusqu’à 0,143 dB de variation sur la tenue très faible. La version lancée utilise la double précision. Aucune observation n’a été supprimée pour faire passer la comparaison.

Douze rendus de contrôle sans activation expérimentale (notes 48, 64, 102, 108 ; vélocités 45, 56, 100) sont **identiques bit à bit** à l’exécutable gelé avant intégration. Les valeurs des presets livrés n’ont pas changé.

## Jeu temps réel et contrôles

Un test de sept secondes enchaîne des accords étendus, des répétitions, la pédale forte, la demi-pédale, la pédale douce, le sostenuto, le relâchement et une queue. Avec deux threads et un tampon de 512 échantillons : charge moyenne **20,4 %**, percentile 99 **50,8 %**, maximum **58,8 %**, zéro dépassement de période. Le compteur atteint 88 voix avec les résonances sympathiques ; cela ne signifie pas 88 touches frappées simultanément. Le pic brut est 0,3782. Le contrôle global des allocations et libérations mémoire reste à zéro pendant chaque rendu, y compris sur les threads de calcul. Ces valeurs sont celles de ce test local, pas une garantie pour toute machine ou polyphonie.

Les deux tests de convolution couvrent les frontières de partitions, les 16 entrées et deux oreilles, les historiques, des blocs de taille variable, la somme primaire/sympathique et le délai du signal résiduel. Les suites moteur, contrôleur et presets passent. Les neuf tests exécutables de l’application passent ; son test de changement de périphérique nécessitant un serveur audio reste ignoré par la suite automatique. Le lancement effectif confirme séparément le backend et les connexions.

Pour cette première UI, les modifications du modèle, les presets et les contrôles MIDI de paramètres sont refusés explicitement : les filtres correspondent à une géométrie figée. Le clavier, les pédales et la visualisation 3D restent utilisables. Le taux moteur doit être 48 kHz, la qualité Full ; les changements incompatibles du backend sont refusés. L’enregistreur de vol est désactivé, car son format de replay ne transporte pas ce bundle expérimental. La configuration est isolée de celle de l’app habituelle.

## Reproduction et lancement

Les filtres et presets générés restent sous `out/rayleigh-ui/`, avec leurs empreintes dans `bench/rayleigh-ui/*-bundle.json`. L’export réutilise les noyaux de recherche existants ; sur une copie sans `out/`, les régénérer selon les études précédentes.

```sh
OPENBLAS_NUM_THREADS=1 python bench/rayleigh-ui/export.py
cargo build --release --offline -p piano-render --example rayleigh_live --features experimental-rayleigh
OPENBLAS_NUM_THREADS=1 python bench/rayleigh-ui/validate.py
bench/rayleigh-ui/launch.sh --midi 'SL STUDIO: MIDI 1' --midi-channel 1 --device 'Omniphony Router'
```

`launch.sh` compile la version de test et utilise un répertoire de configuration temporaire propre. Le bundle `close/bundle.toml` permet aussi de tester les micros proches par la CLI ; il faut relancer l’application pour changer de bundle.

L’instance laissée ouverte utilise `out/rayleigh-ui/bin/unsampled-piano-test`, le bundle `player_actual/bundle.toml`, SL STUDIO canal 1, Omniphony Router canaux 1/2, 48 kHz et 512 échantillons. Service utilisateur : `piano-rayleigh-test-ui.service`. Son bandeau a été vérifié par une capture produite par l’UI elle-même. L’ancienne instance du moteur normal a été arrêtée.
