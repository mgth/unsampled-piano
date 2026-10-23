---
title: Ligne de commande
description: Les options du lecteur autonome et du moteur de rendu hors ligne, pour jouer, tester et convertir des fichiers MIDI en WAV.
order: 5
icon: terminal
---

## `unsampled-piano` — le lecteur

```bash
unsampled-piano [options]
```

| Option | Effet |
|---|---|
| `--ui` | ouvre la fenêtre de l’interface (tous les paramètres, presets, apprentissage MIDI) |
| `--preset <file>` | preset d’instrument (TOML) — surveillé et rechargé à chaque modification |
| `--presets <folder>` | le dossier de presets de l’interface (par défaut : celui du preset, sinon `./presets`) |
| `--list` | liste les périphériques audio et les ports MIDI, puis quitte |
| `--midi <name>` | port d’entrée MIDI : une partie de son nom (par défaut : le premier port matériel) |
| `--host <backend>` | `node` (par défaut sous Linux : un nœud du graphe PipeWire/JACK avec entrée et sortie MIDI), ou un hôte audio simple sans ports MIDI : `pipewire`, `jack`, `alsa`, `wasapi` (par défaut sous Windows), `asio`, `coreaudio` (par défaut sous macOS) |
| `--device <name>` | sortie audio : une partie du nom du périphérique, ou des ports de lecture (node) |
| `--buffer <frames>` | taille du tampon (node : période du graphe demandée) ; 256 par défaut |
| `--output-channel <n>` | premier canal de la paire stéréo sur une interface multicanal (à partir de 1) |
| `--gain-db <dB>` | gain de sortie |
| `--name <name>` | nom du nœud dans le graphe (par défaut `unsampled-piano`) |
| `--midi-out <name>` | connecte la sortie MIDI miroir aux entrées dont le nom contient ce texte (node) |
| `--file <files…>` | joue des fichiers MIDI standard l’un après l’autre |
| `--from <s>` | démarre le premier fichier à cet instant |
| `--demo` | joue une courte séquence intégrée au lieu d’écouter le MIDI |

Tout ce que vous passez systématiquement peut aller dans
[`play.toml`](/fr/docs/getting-started/#vos-réglages-par-défaut).

```bash
# Linux : faible latence, l'interface, un preset que vous harmonisez dans un éditeur
unsampled-piano --buffer 128 --ui --preset my_grand.toml

# jouer un fichier MIDI avec les microphones de concert
unsampled-piano --preset presets/listening/Microphones/concert.toml --file etude.mid
```

## `unsampled-piano-render` — rendu hors ligne

MIDI → WAV stéréo, déterministe, plus rapide que le temps réel, identique au bit près d’une
exécution à l’autre. Pratique pour comparer des réglages, rendre un morceau ou vérifier une
modification.

```bash
unsampled-piano-render [--preset <file>] [--sr 48000] [--block 128] [--tail 3] <command>
```

| Commande | Ce qu’elle produit |
|---|---|
| `midi <in.mid> <out.wav>` | un fichier MIDI standard, toutes pistes et tous canaux fusionnés |
| `note <out.wav> --note 60 --velocity 100 --hold 4` | une note, tenue `hold` secondes |
| `sweep <out.wav> --velocity 90 --step 0.5 [--sustain]` | toutes les touches de A0 à C8, éventuellement pédale enfoncée |
| `dump-preset <out.toml>` | pas un son : le preset par défaut complet, avec tous les paramètres |

Depuis l’arborescence des sources, les deux s’obtiennent avec `cargo run --release -p piano-play --`
et `cargo run --release -p piano-render --`.
