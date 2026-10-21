# piano

Piano virtuel par modélisation physique, temps réel depuis MIDI, en Rust. Référence : Commercial. Aucun sample dans la production du son (les enregistrements ne servent qu'à l'analyse et à la calibration).

- État de l'art et plan : [reports/Synthèse physique de piano.md](reports/Synthèse%20physique%20de%20piano.md)
- Phase 0 (outillage, mesures CPU et précision) : [docs/phase0.md](docs/phase0.md)
- Phase 1 (corde + marteau, calibrage) : [docs/phase1.md](docs/phase1.md)
- Phase 2 (unissons couplés, polarisations, conductance du chevalet) : [docs/phase2.md](docs/phase2.md)
- Phase 3 (étouffoirs, pédales, résonance sympathique) : [docs/phase3.md](docs/phase3.md)
- Phase 4 (table d'harmonie modale, rayonnement, stéréo) : [docs/phase4.md](docs/phase4.md)
- Analyse de Salamander Grand (Yamaha C5) : [docs/salamander.md](docs/salamander.md)

```bash
cargo test --workspace
cargo run --release -p piano-render -- note out/c4.wav --note 60
```
