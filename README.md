# piano

Piano virtuel par modélisation physique, temps réel depuis MIDI, en Rust. Référence : Commercial. Aucun sample dans la production du son (les enregistrements ne servent qu'à l'analyse et à la calibration).

Chaîne physique : marteau en feutre (contact passif quadratisé) → 1 à 3 cordes raides par touche, deux polarisations, couplées au chevalet (modes de Weinreich) → table d'harmonie modale (plaque orthotrope encastrée) → rayonnement de Rayleigh vers deux micros (couvercle optionnel) → salle (réflexions par sources images, réverbération). S'y ajoutent les étouffoirs, les pédales (demi-pédale, sostenuto, una corda), la résonance sympathique par le chevalet et les bruits mécaniques par impulsions.

## Utilisation

```bash
# jouer depuis un clavier MIDI (PipeWire natif, priorité temps réel)
cargo run --release -p piano-play -- --list
cargo run --release -p piano-play -- --buffer 128 [--preset presets/concert.toml]

# rendu hors ligne (WAV stéréo)
cargo run --release -p piano-render -- midi morceau.mid out/morceau.wav
cargo run --release -p piano-render -- note out/c4.wav --note 60 --velocity 90
cargo run --release -p piano-render -- dump-preset presets/mon_piano.toml

# calibration contre une banque d'échantillons <note>v<couche>
tools/calibrate.sh data/salamander/Samples out/calib

cargo test --workspace
```

## Crates

| Crate | Rôle |
|---|---|
| `piano-core` | moteur DSP, sans dépendance (feature `serde` pour les préréglages) |
| `piano-host` | préréglages TOML, décodage MIDI |
| `piano-play` | application temps réel (cpal + midir) |
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
