# piano

Piano virtuel par modélisation physique, temps réel depuis MIDI, en Rust. Référence : Commercial. Aucun sample dans la production du son (les enregistrements ne servent qu'à l'analyse et à la calibration).

Chaîne physique : marteau en feutre (contact passif quadratisé) → 1 à 3 cordes raides par touche, deux polarisations, couplées au chevalet (modes de Weinreich) → table d'harmonie modale (plaque orthotrope encastrée) → rayonnement de Rayleigh vers deux micros (couvercle optionnel) → salle (réflexions par sources images, réverbération). Le cadre en fonte (excité aux agrafes) et la ceinture (excitée par les bords de la table) rayonnent aussi. S'y ajoutent les étouffoirs, les pédales (demi-pédale, sostenuto, una corda), la résonance sympathique par le chevalet et les bruits mécaniques par impulsions.

## Utilisation

```bash
# jouer : le piano devient un nœud du graphe PipeWire (API JACK) avec sortie
# stéréo, entrée MIDI (timing à l'échantillon) et sortie MIDI miroir de ce qu'il
# joue ; il se branche sur la sortie par défaut et sur le premier clavier
# (--midi / --device pour choisir, --host pipewire pour l'ancien mode cpal ;
# réglages personnels par défaut dans ~/.config/piano/play.toml, par ex.
# device = "Stereo DJ X-Fade:In B")
cargo run --release -p piano-play -- --list
cargo run --release -p piano-play -- --buffer 128 [--preset presets/proche.toml | presets/concert.toml]
# jouer des fichiers MIDI à la suite (timing à l'échantillon près ; --from saute le début)
cargo run --release -p piano-play -- --buffer 128 --file data/midi/*.mid

# rendu hors ligne (WAV stéréo)
cargo run --release -p piano-render -- midi morceau.mid out/morceau.wav
cargo run --release -p piano-render -- note out/c4.wav --note 60 --velocity 90
cargo run --release -p piano-render -- dump-preset presets/mon_piano.toml

# calibration contre une banque d'échantillons <note>v<couche>
tools/calibrate.sh data/salamander/Samples out/calib

cargo test --workspace
```

## Interprétations de référence

Les fichiers de `data/midi/` sont tirés de [MAESTRO v3](https://magenta.tensorflow.org/datasets/maestro) (Google Magenta, CC BY-NC-SA 4.0) : des pianistes du Piano-e-Competition enregistrés sur Disklavier, avec la vélocité réelle des touches et les pédales. `data/` n'est pas versionné ; pour les retrouver :

```bash
curl -L -o data/maestro/maestro-v3.0.0-midi.zip https://storage.googleapis.com/magentadata/datasets/maestro/v3.0.0/maestro-v3.0.0-midi.zip
```

## Crates

| Crate | Rôle |
|---|---|
| `piano-core` | moteur DSP, sans dépendance (feature `serde` pour les préréglages) |
| `piano-host` | préréglages TOML, décodage MIDI |
| `piano-play` | application temps réel : nœud PipeWire/JACK (audio, MIDI in, MIDI miroir), ou cpal + midir |
| `piano-render` | rendu hors ligne déterministe MIDI → WAV |
| `piano-analysis` | analyse : partiels, B, décroissances, ESPRIT, distances spectrales |
| `piano-eval` | comparaison du modèle à une banque de référence |
| `piano-bench` | débit du noyau modal, précision f32 |

## Documentation

- État de l'art et plan : [reports/Synthèse physique de piano.md](reports/Synthèse%20physique%20de%20piano.md)
- Phase 0 (outillage, mesures CPU et précision) : [docs/phase0.md](docs/phase0.md)
- Analyse de Salamander Grand (Yamaha C5) : [docs/salamander.md](docs/salamander.md)
- Phase 1 (corde + marteau, calibrage) : [docs/phase1.md](docs/phase1.md)
- Phase 2 (unissons couplés, polarisations, conductance du chevalet) : [docs/phase2.md](docs/phase2.md)
- Phase 4 (table d'harmonie modale, rayonnement, stéréo) : [docs/phase4.md](docs/phase4.md)
- Phase 3 (étouffoirs, pédales, résonance sympathique) : [docs/phase3.md](docs/phase3.md)
- Phase 5 (bruits mécaniques, préréglages, temps réel, calibration) : [docs/phase5.md](docs/phase5.md)
- Cadre en fonte et ceinture : [docs/cadre_ceinture.md](docs/cadre_ceinture.md)
- Tenue des notes (chevalet, épaisseur de la table, pertes des cordes) : [docs/tenue.md](docs/tenue.md)
- Dynamique, pédale et résonance sympathique : [docs/dynamique_pedale.md](docs/dynamique_pedale.md)
- Comparaison avec Salamander, Iowa et Commercial : [docs/comparaison.md](docs/comparaison.md)
- Pertes des cordes graves (désaccord d'unisson, mode piston de la caisse) : [docs/graves.md](docs/graves.md)
- Changement de timbre avec la nuance (feutre) : [docs/timbre_nuance.md](docs/timbre_nuance.md)
- Non-linéarité des cordes (partiels fantômes) : [docs/non_linearite.md](docs/non_linearite.md)
- Présence de l'attaque (marteau à deux couches, prises de son) : [docs/presence.md](docs/presence.md)
