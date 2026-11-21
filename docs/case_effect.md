# The "case" effect against Splendid

Status as of 2026-11-21.

## The question

After the attack's and the halo's work (docs/attack_rise.md, docs/body.md) the Concert grand still lacked, by ear, a "case effect" that the Splendid Grand has: a reverberation the user put down to the case, heard mostly on the bass keys, in their middle and high frequencies. Each lead was measured on the model (the Concert grand at the commit below, close microphones in the studio, the bench's playback velocities) against Splendid and five other real grands (Plath, Helgesson, Headroom, BiVib, Osiris), then put to the ear on a listening page changing one thing at a time. The scripts are in the session's scratchpad (not versioned); the pages in `out/worktrees/timbre-caisse/ab_*`.

## What the ear ruled out

| Lead | Measured | Heard |
|---|---|---|
| A fixed low mode of the body | Under the keys E4–C8 every real grand has one resonance common to all keys: Splendid 77 Hz, Plath 86 (93 a year earlier), Helgesson 91, Headroom 84, BiVib 92, Osiris 86; 9–24 dB above its neighbours, ±2–4.6 dB from key to key, T60 1–1.9 s. On Plath it is at the ears and in the key bed's force, not at the bridge: the case and key bed struck by each key. The model has a 4 dB bump at 76 Hz varying ±6.5 dB from key to key | One coherent 77 Hz mode at Splendid's level (−31 dB re the note at ff, −40 at mf): no difference |
| The low end's coherence | 40–160 Hz between the microphones: real 0.65–0.97, model 0.34–0.36 (the halo and the knock draw a sign per microphone and resonance, `body.rs`, `action.rs`) | The halo and knock made coherent below 200 Hz: no difference |
| The colour, note by note | Static third-octave equalisation on Splendid | Closer, "but not the effect" |
| The partials' wandering between the microphones | B0–B2: real ±1.1–3.3 dB, model ±0.1–0.3 (one force per key at the bridge) | Built in the engine (docs/bridge_rocking.md): ±0.7–2.8 dB, "does not come closer" |
| The partials' beats | Real 0.9 dB rms about the smooth decay, model 0.3 | Adding Splendid's beats to its decay: nothing more |
| The decay's trend per band | 630 Hz–2 kHz of the bass keys: the real partials drop 2 dB more over the first 0.6 s, then decay 4 dB/s slower | Engine variants (`polarization.bass_until` 47, `unison.width_bass` 0.5, both): measured nearer, not heard nearer |
| The decay's irregularity from partial to partial | Real 2.0–3.9 dB (prompt drop) and 2.6–5.9 dB/s (aftersound), model 1.4–1.7 and 1.2–2.8; only the unisons' mistuning moves it (1.5 cent brings it in range) | Splendid's trend alone, its partials' own deviations alone, the engine at 1.5 cent: none |
| The real grands' mean colour | B0–B2 partials: 100–125 Hz −2.4 to −4.3 dB, 250–400 Hz +3.4 to +4.5 dB (reals within 1–2 dB of each other) | A fixed filter on that gap: not it |

Also measured and left: between the partials of B0–C#2 the model is 5–9 dB short at 500 Hz–2 kHz after 0.15 s, and its residual at 4–8 kHz is far more coherent between the microphones than the real ones' (0.84 against 0.19–0.46), where those are near their own recording noise.

## What the ear kept

Two offline transfers onto the model's own partials (an STFT mask around both notes' partials, Splendid set to the model's partial power over 0–0.3 s), the residual between them kept as the model's:

- **Splendid's partials** with the model's residual sound very close to Splendid; the model's partials with Splendid's residual do not (G6 and C7 aside, where Splendid's residual carries undamped strings and noise). The effect is on the partials.
- On the bass, giving the model's partials **each partial's mean level** as Splendid's brings it closer, and so does **each partial's smoothed decay shape** alone.

## Why it is Splendid's own

- The real grands' mean colour does not do what Splendid's does.
- Splendid's partial levels against the model's, each key's mean taken off, pooled by frequency in twelfths of an octave over A0–C5 (30 keys): a fixed response D(f) of ±4 dB. On B0–B2 it explains 4 % of the per-partial difference (5.6 → 5.5 dB rms), and nothing when fitted on C3–C5 only. It is not a filter common to the keys.
- Not the strike point: read off the partials' comb, 0.119–0.125 on Splendid and on the model, as on Plath, Helgesson and BiVib. The comb explains the model's partial levels better (correlation 0.91) than any real grand's (0.63–0.79): theirs carry more structure of each key's own.

What the ear calls the case here is Splendid's fingerprint key by key: its board and bridge seen from each string's bearing point, its strings, its voicing. The model's built board is the D's from its drawings, with a fingerprint as irregular (5.0–7.3 dB from partial to partial against 4.4–11 on the real grands) but its own.

## Open

- Reproducing one instrument's fingerprint would mean inferring its board and bridges from its recordings: not attempted.
- The fixed low mode (77–92 Hz) and the coherence of the low end are real gaps, not heard on these pages; they may matter elsewhere (in a room, or on a full-range system).
