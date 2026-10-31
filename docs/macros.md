# Musician's controls

Status as of 2026-10-25.

The parameters describe the physics: felt stiffness in N/mm^p, string losses b1 and b2, bridge impedance. A musician thinks in effects: softer, longer, more resonance, more room. Eleven controls (`[macros]` in a preset, `crate::macros`) each move a group of parameters together.

## How they work

- **Offsets on top of the preset.** Each control runs from −1 to +1. At 0 the preset sounds as it is; the advanced values stay the preset's base, and the controls are applied on top of them (`PianoModel::with_macros`) where the instrument is built and where live parameters reach a playing engine. Applying twice changes nothing, and values beyond ±1 are held at the ends.
- **Steps.** Most controls scale their parameters by span^v, so equal steps of the control give equal steps in dB or in log brightness.
- **Real ends.** The spans keep each end a plausible piano: at ±1 the effects stay within what real grands show, or near it.
- **Live or rebuilt.** Touch, note endings and action noises move live parameters only (velocity law, damper strength and fall time, noise levels), and are live parameters themselves (`LiveParam::MacroTouch`, `MacroRelease`, `MacroMechanism`). The engine keeps the live parameters before the controls (`Instrument::source`) and applies the controls to them at each change, so moving one never compounds; a live change sounds exactly as a rebuild with the same value (test `macros_live`). The other eight need the instrument rebuilt.

## The controls

Measured with `tools/macro_sweep.py` (close microphones, the default instrument). Each figure is the change from 0, at −1 / −0.5 / +0.5 / +1:

| Control | Moves | Effect at −1 / −0.5 / +0.5 / +1 |
|---|---|---|
| `tone` (soft ↔ bright) | top felt hardness × 4^v, inner felt × 12^v | attack centroid, C3: −18 / −9 / +9 / +18 %; C5: −9 / −4 / +3 / +6 % |
| `touch` (even ↔ contrasted) | log-distance of the hammer speeds from velocity 60's × 1.5^v (mezzo forte unchanged) | C4 velocity 100 − 30: −6.2 / −3.4 / +3.9 / +8.6 dB |
| `sustain` (short ↔ long) | b1, its floor, b2 and the edge loss × 3^−v | at 2 s, C3: −7.2 / −2.7 / +1.6 / +2.6 dB; C5: −13.9 / −5.1 / +3.0 / +4.7 dB |
| `resonance` (dry ↔ rich) | sympathetic gain × 2^v, highest damper 4·v keys lower | halo of a pedalled C4: −5.8 / −2.9 / +2.7 / +5.3 dB |
| `release` (clean ↔ soft endings) | damper strength × 3^−v, fall time × 3^(v/2) | C4's drop 0.25 s after the key-up: +3.7 / +1.9 / −2.4 / −7.3 dB |
| `unisons` (pure ↔ lively) | unison spread × 3^v (× 1.6^v towards pure), hit spread × 1.5^v | C5 over 1–2 s: −3.7 / −2.3 / +4.5 / +5.7 dB; C4: within 1 dB |
| `mechanism` (discreet ↔ present) | action knock, keybed, damper and pedal noise × 8^v (defaults 7.5 dB below the real grands' level, which +0.42 reaches) | the knock under C7: −4.7 / −4.0 / +8.0 / +17.0 dB (lower down it sinks under the note's own sound) |
| `size` (baby ↔ concert grand) | bass string length × 1.25^v (bass inharmonicity ∝ 1/L²), board and frame × 1.12^v, hammer mass × 1.1^v | (by construction) |
| `lid` (shut ↔ open) | lid angle from the preset's towards 0 (shut) or 45°; a removed lid is put back on, shut, then opened | C4's centroid, engine's set-up, open side: +11 / +5 %; shut side: see [lid.md](lid.md) |
| `space` (close ↔ roomy) | room tail × 3^v | room level after a staccato C4: −6.7 / −3.8 / +4.3 / +8.2 dB |
| `stretch` (strict ↔ stretched) | stretch law × (1 + v) | equal temperament at −1, twice the preset's stretch at +1 |

## Limits

- **Sustain, long side.** The strings' own losses stop counting beyond +0.5: the board's coupling then sets the sustain, and moving it would change the timbre.
- **Tone in the treble.** C5's attack is dominated by its fundamental, so the measured brightness moves less there. The treble's felt answers little to its hardness. The felt's contact width was tried and does nothing at C5.
- **Pure unisons.** A third of the spread made C5 lose 12 dB at 1.5 s: the slow modes that carry the sustain need some detuning. The pure side stops at 0.6 of the spread.
- **Lid.** Since 2026-10-30 the closed side shuts the lid ([lid.md](lid.md)): the highs of the board's top face go through the shut case. Halfway, the angle is halved and the closure is at 0.5.
