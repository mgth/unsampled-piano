# Archive de recherche : tenue de H1

Cette étude est conservée sur `codex/archive-fundamental-sustain` et retirée
de la PR destinée à `main`. Elle dépend des sources et des exports locaux du
prototype Rayleigh de `codex/piano-main-revalidation` (`b3b2808`), des corpus
de référence et des rendus sous `out/`. Ce n'est pas un outil autonome fourni
par un clone de `main`.

Le détail des essais, les paramètres, les empreintes, les résultats et les
commandes historiques sont dans `docs/fundamental_sustain.md` sur cette branche.
Les graphes et JSON restent ceux de l'étude du 29 octobre 2026. Les mesures
spectrales historiques appellent explicitement `fft_padding=1` pour ne pas
mélanger ces résultats avec la nouvelle spécification v3 du banc autonome.
La mesure H1 adaptative conserve son propre FFT à padding ×8.

Les corrections de review présentes dans l'archive refusent les voisins qui
changent de grille temporelle ou d'ancrage et rejettent proprement une variante
si H1 est illisible à l'ancrage du témoin ou du candidat.

Tests synthétiques, sans corpus ni moteur expérimental :

```sh
python3 -m unittest discover -s bench/fundamental-sustain -p 'test_*.py'
```

Les WAV et les MP3 de l'écoute restent disponibles dans le workspace local,
notamment `out/bench/fundamental-sustain/concert-midi/`. Cette séparation ne
change ni les fichiers audio écoutés ni le réglage de l'application.
