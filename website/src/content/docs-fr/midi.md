---
title: MIDI et automation
description: Ce que le piano lit en MIDI, les pédales et leurs positions intermédiaires, l’apprentissage MIDI, la sortie MIDI miroir et l’automation des paramètres dans le plugin.
order: 4
icon: midi
---

## Ce qu’il lit

| Message | Effet |
|---|---|
| Note on | lance le marteau ; la vélocité fixe sa vitesse (voir la [correspondance des vélocités](/fr/docs/parameters/player/#de-la-vélocité-à-la-vitesse-du-marteau)) |
| Note off | laisse retomber l’étouffoir ; la **vélocité de relâchement** fixe sa vitesse, donc la force du petit bruit qu’il fait en se posant |
| CC 64 | pédale forte, **continue** : demi-pédale comprise |
| CC 66 | pédale du milieu : pédale tonale, ou la sourdine sur un preset de piano droit (`mute_felt`) |
| CC 67 | pédale douce : una corda sur un piano à queue, rapprochement des marteaux sur un piano droit, continue |
| autres CC | libres pour l’[apprentissage MIDI](#apprentissage-midi) |

Les événements sont appliqués **à l’échantillon près** à l’intérieur du tampon audio — dans le
lecteur autonome via le port MIDI PipeWire/JACK, dans le plugin aux positions fournies par
l’hôte — si bien que le timing ne dépend pas de la taille du tampon.

Une pédale forte **continue**, ça compte : l’essentiel de ce que fait un pianiste du pied droit
n’est ni tout en haut ni tout en bas. Les claviers équipés d’une pédale continue (ou compatible
demi-pédale) envoient le CC 64 avec des valeurs entre 0 et 127 ; les simples interrupteurs
n’envoient que 0 et 127.

## Apprentissage MIDI

Chaque paramètre peut être assigné à un contrôleur : cliquez sur la case du contrôleur de sa
ligne dans l’interface, bougez un bouton ou un fader, c’est fait. Un contrôleur couvre toute la
plage du paramètre ; une assignation apprise avec un minimum au-dessus du maximum l’inverse. Le
changement de banque, les trois CC de pédales et les messages de mode de canal ne sont jamais
appris.

- Les paramètres **en direct** assignés sont pilotés sur le thread audio, à l’échantillon près,
  fenêtre ouverte ou non.
- Les paramètres **à reconstruction** assignés lancent une reconstruction en arrière-plan à
  mesure que vous tournez, sans couper les notes.

La table d’assignation concerne vos contrôleurs, pas l’instrument : le lecteur autonome la
conserve dans `midi.toml` dans le dossier de configuration, le plugin l’enregistre avec le projet.

## Sortie MIDI miroir (application autonome, Linux)

Sous Linux, le nœud du lecteur dispose aussi d’une **sortie MIDI** qui renvoie ce qu’il joue.
`--midi-out <name>` la connecte à un autre synthétiseur du graphe, pour enregistrer ou superposer.

## Automation dans le plugin

Dans le plugin CLAP et VST3, les paramètres **en direct** sont des paramètres de l’hôte :
automatisez-les, assignez-les à une surface de contrôle, modulez-les. Chacun a un identifiant
stable dérivé de son chemin (un hachage FNV-1a), si bien que l’automation survit aux nouvelles
versions qui ajoutent des paramètres. Le reste de l’instrument — les paramètres à reconstruction —
et les contrôleurs appris sont enregistrés dans l’état du projet ; les modifier reconstruit
l’instrument en arrière-plan sans couper les notes.
