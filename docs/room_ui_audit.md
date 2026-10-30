# Ambiance excessive dans l’UI expérimentale

Diagnostic du 29 octobre 2026 sur les binaires figés après `0dfd311`, bundle `out/upstream-4a00f00/player_actual/bundle.toml`. Aucun réglage, moteur ou défaut modifié. L’utilisateur précise entendre une ambiance très présente pendant le jeu, plutôt qu’une tenue de plusieurs secondes. L’application n’était plus active lors du diagnostic : les événements MIDI et le routage de sa session ne sont donc pas reconstitués.

## Cause reproduite

Le bundle fixe le preset d’écoute pianiste (omnis, couvercle 35°) et la salle studio : 8 × 6 × 3,5 m, absorption 0,45/0,60, `tail_gain=1`. La salle est appliquée une seule fois, après le rayonnement intégral. Sa durée théorique est courte (T60 de 0,229 à 0,310 s), mais son niveau n’est pas discret à cette position.

Une impulsion stéréo identique est passée dans la salle seule. La puissance est sommée dans le temps et moyennée entre les deux canaux, sans sommation mono ni normalisation individuelle :

| Rapport énergie réfléchie / énergie directe | Micros rapprochés | Pianiste |
|---|---:|---:|
| Salle complète | −18,13 dB | **−2,21 dB** |
| Réflexions précoces seules (`tail_gain=0`) | −30,12 dB | −14,71 dB |
| Queue seule (différence complète − précoces) | −18,60 dB | **−2,57 dB** |

La queue diffuse domine donc l’ambiance. Le changement d’écoute augmente son dosage relatif d’environ **16 dB**. Ce n’est pas un allongement du T60. Ces rapports d’énergie d’impulsion ne sont pas des pourcentages perceptifs de réverbération.

Le code reconstruit une source à 1 m à partir de la pression aux micros multipliée par leur distance au centre de la table, puis calibre la queue sur une distance critique. Dans ces presets, la distance source–micro passe de 0,502 m à 1,836 m et les cardioïdes deviennent des omnis. La distance critique effective passe de 4,270 m à 2,465 m. La formule prévoit −18,59 et −2,56 dB pour la queue, ce qui correspond aux mesures. Cette approximation ponctuelle et sa directivité fixe ne constituent pas une validation physique de l’ambiance au pianiste avec le rayonnement intégral.

## Ablation audio

`python bench/room-ui-audit/audit.py [dossier_sortie]` produit 28 WAV : notes MIDI 48, 60, 69, 81, 93, 102, 108 à vélocité 56, tenues 250 ms puis relâchées pendant 4 s, sans pédale. Chaque moteur est rendu avec et sans salle ; tous les autres paramètres de chaque paire sont conservés. L’intégral utilise les contacts expérimentaux ; le témoin normal utilise le contact de base, donc les différences entre moteurs dans le suraigu ne sont pas une ablation du seul rayonnement.

Sur Do4, l’énergie de la contribution de salle est à **+0,10 dB du direct** avec l’intégral, contre **−0,27 dB** avec le moteur normal. Le phénomène existe donc sans rayonnement intégral. La salle et le direct sont cohérents : leurs puissances ne s’additionnent pas, ce qui explique qu’une ambiance forte puisse s’accompagner d’un niveau total plus faible.

Sur Do4 intégral, la fenêtre située 50–150 ms après relâchement passe de −27,4 dB sans salle à −16,5 dB avec salle, par rapport au pic sec de 20 ms. À 250–350 ms après relâchement, les deux sont déjà vers −55 dB. Cela reproduit une ambiance forte et courte, sans établir de problème de pédale ou de queue infinie.

## Pourquoi les tests ne rejettent pas cette écoute

- `tools/bench.py` et le banc global utilisent volontairement des micros rapprochés + studio, où la salle est environ 16 dB moins présente. Ils ne certifient pas le preset d’écoute de l’UI.
- Le contrôle pianiste compare les contacts, partiels, bandes et creux avec la même salle pour tous. Une erreur commune de dosage peut donc traverser cette comparaison à couverture constante.
- Les tests unitaires de salle contrôlent surtout la pente de décroissance pour les presets hall/concert hall et les valeurs finies. Ils n’encadrent ni le rapport réfléchi/direct du studio au pianiste, ni sa coloration.
- Les tests de charge et d’équivalence à la convolution hors ligne prouvent une implémentation stable et conforme au prototype, pas un réalisme perceptif.
- Le banc acoustique publie des scores et impose la couverture ; il n’a pas de seuil de rejet pour cette ambiance. Les notes du banc global sont tenues 20 s et leurs enveloppes ne constituent pas un test de relâchement court.

Il manque un contrôle explicite de la configuration écoutée : énergie réfléchie/directe par bande, décroissance après note-off et pédale-off sur notes amorties, accords et écoute A/B à gain commun. Les seuils d’acceptation doivent être documentés à partir d’une cible d’écoute ou de prises appariées ; figer arbitrairement le niveau actuel comme référence ne corrigerait pas le défaut de validation. Les diagnostics présents ne sont donc pas présentés comme un nouveau test de réalisme réussi.
