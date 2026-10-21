# piano

Piano virtuel par modélisation physique, temps réel depuis MIDI, en Rust. Référence : Commercial. Aucun sample dans la production du son (les enregistrements ne servent qu'à l'analyse et à la calibration).

- État de l'art et plan : [reports/Synthèse physique de piano.md](reports/Synthèse%20physique%20de%20piano.md)
- Phase 0 (outillage, mesures CPU et précision) : [docs/phase0.md](docs/phase0.md)
- Analyse de Salamander Grand (Yamaha C5) : [docs/salamander.md](docs/salamander.md)

```bash
cargo test --workspace
cargo run --release -p piano-render -- note out/c4.wav --note 60
```
