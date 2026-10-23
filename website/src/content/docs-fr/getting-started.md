---
title: Premiers pas
description: Installer l’application autonome ou le plugin CLAP/VST3 sous Windows, macOS ou Linux, brancher un clavier et jouer.
order: 1
icon: rocket
---

Unsampled Piano existe sous deux formes, qui partagent le même moteur et la même interface :

- l’**application autonome**, `unsampled-piano`, qui ouvre une fenêtre, écoute votre clavier
  MIDI et joue sur votre interface audio ;
- le **plugin**, CLAP et VST3, pour toute STAN (DAW) capable d’accueillir des instruments.

Les binaires sont produits par les GitHub Actions du projet et joints à chaque
[version publiée](https://github.com/mgth/unsampled-piano/releases) ; la
[page de téléchargement](/fr/download/) choisit pour vous le bon fichier.

## Windows

L’installeur, `unsampled-piano-setup-windows-x86_64.exe`, propose trois composants, tous
facultatifs : l’application autonome (avec un raccourci dans le menu Démarrer), les plugins VST3
et CLAP, et les presets qu’ils partagent. Installez-le pour tous les utilisateurs (`Program Files`,
`Common Files\VST3` et `Common Files\CLAP`, presets dans `ProgramData\Unsampled Piano\presets`)
ou pour vous seul ; un désinstalleur est fourni.

Pas envie d’un installeur ? L’archive `unsampled-piano-standalone-windows-x86_64.zip` contient
`unsampled-piano.exe` et les presets à côté — rien d’autre à installer.

Le son sort par défaut via **WASAPI**. La version Windows parle aussi l’**ASIO** :

```bash
unsampled-piano --host asio
```

En ASIO, la taille du tampon est celle réglée dans le panneau de configuration du pilote.

## macOS

`Unsampled Piano.app` est universelle (Intel et Apple Silicon) et joue via CoreAudio, presets
inclus dans le bundle. Elle n’est **pas notariée** : la première fois, ouvrez-la par un clic
droit → *Ouvrir*, ou retirez l’attribut de quarantaine depuis un terminal :

```bash
xattr -dr com.apple.quarantine "Unsampled Piano.app"
```

Les plugins sont dans `unsampled-piano-macos-universal.zip` : copiez `Unsampled Piano.clap` dans
`~/Library/Audio/Plug-Ins/CLAP` et `Unsampled Piano.vst3` dans `~/Library/Audio/Plug-Ins/VST3`.

## Linux

Sous Linux, le piano se compile depuis les sources — c’est un workspace Rust, rien de plus :

```bash
git clone https://github.com/mgth/unsampled-piano
cd unsampled-piano
cargo run --release -p piano-play -- --ui
```

Il devient un nœud du graphe **PipeWire** (via son API JACK), avec une sortie stéréo, une entrée
MIDI au timing précis à l’échantillon près et une sortie MIDI qui renvoie ce qu’il joue. Il se
connecte tout seul à la sortie par défaut et au premier clavier matériel. Pour le compiler, il
faut une chaîne d’outils Rust stable, `pkg-config` et `clang`, les en-têtes audio (ALSA, JACK et
PipeWire : `libasound2-dev libjack-jackd2-dev libpipewire-0.3-dev` sous Debian et Ubuntu) et,
pour la fenêtre, OpenGL et X11/xkbcommon (`libgl-dev libxkbcommon-dev libxcursor-dev libx11-xcb-dev`).

Pour l’installer proprement — le binaire, une entrée de menu qui ouvre l’interface, son icône et
les presets fournis, dans `~/.local` (ou `PREFIX=/usr/local` pour tous les utilisateurs, en root) :

```bash
tools/install-app.sh
```

Les plugins pour Linux sont dans `unsampled-piano-linux-x86_64.zip` ; vous pouvez aussi les
compiler et les installer dans `~/.clap` et `~/.vst3` :

```bash
cargo xtask bundle piano-plugin --release
tools/install-plugin.sh
```

## Premier son

1. Branchez un clavier MIDI avant de lancer l’application ; c’est le premier trouvé qui est
   utilisé (`--midi <name>` en choisit un autre — `--list` les affiche tous).
2. Lancez l’application. Ouverte depuis l’Explorateur ou le Finder, elle affiche l’interface ;
   depuis un terminal, ajoutez `--ui`.
3. Jouez. La barre d’état affiche le nombre de voix et la charge du thread audio.

Trop de latence ? Réduisez le tampon : `--buffer 128` (ou 64 sur une machine rapide). Des
craquements ? Augmentez-le.

## Vos réglages par défaut

Les options que vous utilisez à chaque fois se mettent dans `play.toml`, dans le dossier de
configuration :

| Système | Dossier |
|---|---|
| Linux | `~/.config/unsampled-piano/` |
| Windows | `%APPDATA%\unsampled-piano\` |
| macOS | `~/Library/Application Support/unsampled-piano/` |

```toml
# play.toml — tout ce que la ligne de commande ne précise pas
device = "Scarlett 2i2"
buffer = 128
preset = "/home/me/pianos/my_grand.toml"
ui = true
```

Les clés sont celles de la [ligne de commande](/fr/docs/command-line/) : `host`, `device`, `midi`,
`midi_out`, `name`, `preset`, `presets`, `buffer`, `output_channel`, `gain_db`, `ui`.
