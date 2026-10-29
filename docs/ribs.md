# The soundboard's ribs

Status as of 2026-10-29.

## What was expected

A real board's ribs confine its vibration between them above about 1.1 kHz (Ege et al. 2013), and its mobility at the bridge falls and roughens there (Giordano 1998). Two gaps in the model point there:

- **Axis 3** of [realism_axes.md](realism_axes.md): from the bridge to the player's ear, the real piano's samples spread by 7.5–15 dB per third octave above 2 kHz, the model's by 5–9 dB.
- The key-to-key evenness of partials 1–2 ([evenness.md](evenness.md)).

## What the model has

`soundboard.rib_spacing` and `rib_share` ([board_radiation.md](board_radiation.md)) put ribs parallel to the board's short side into the outlined board's Rayleigh–Ritz basis: each rib's bending stiffness is added along its line, and the same stiffness smeared over the plate is taken out of `dy`. They act below `outline_below`.

`rib_jitter` (new, 0 by default) makes them irregular, as a real board's are. Each rib moves by up to ± half this share of the spacing, and its stiffness varies by up to ± this share around the same mean. A fixed pseudo-random sequence gives the same board at every build.

## Measured

All three measures are taken with the board's own modes up to 6 kHz (`outline_below` 6000), so that the ribs cover the band. The board then takes 1.6 s to build, against 0.02 s at 1.5 kHz.

**Key-to-key evenness**, board alone (`board_evenness close`, C2–C7, partials 1–2 about the median of ±6 keys):

| | p1 | p2 |
|---|---|---|
| default (corner 1.5 kHz) | 3.53 dB | 3.10 dB |
| corner 3 kHz, no ribs | 3.08 dB | 3.21 dB |
| ribs 0.13 m, share 0.5 | 3.07 dB | 3.28 dB |
| ribs 0.13 m, share 0.8 | 3.06 dB | 3.37 dB |

The bench's own evenness measure is already within the real grands (middle p1 7.2 against 6.4 dB, p2 5.0 against 4.8).

**Bridge to the player's ear** against Plath's grand (`bridge_to_ear`, `tools/tf_compare.py`). The table gives the spread per third octave; the real piano's is in the first column:

| | 2000 Hz | 3175 | 4000 | 5040 |
|---|---|---|---|---|
| Plath | 10.4 | 11.3 | 10.9 | 12.8 |
| no ribs | 5.5 | 5.1 | 7.7 | 7.5 |
| periodic, share 0.8 | 5.8 | 5.5 | 6.5 | 6.6 |
| jitter 0.5, share 0.8 | 5.9 | 5.8 | 7.8 | 7.8 |
| jitter 1, share 0.95 | 6.5 | 7.6 | 8.3 | 7.7 |

The level stays 9–12 dB under the real piano at 3–5 kHz whatever the ribs.

**Driving-point conductance** at every key's bridge point, sampled at its partials (`rib_mobility`, 10·log10 Re Y, median / spread):

| | 1000 Hz | 2000 | 4000 | 8000 |
|---|---|---|---|---|
| no ribs | −31.3 / 3.4 | −31.2 / 1.9 | −31.3 / 1.1 | −31.5 / 0.4 |
| jitter 1, share 0.95 | −31.1 / 3.5 | −31.1 / 2.3 | −31.0 / 1.7 | −31.4 / 0.4 |
| jitter 1, share 0.99 | −31.0 / 3.7 | −31.0 / 2.0 | −31.0 / 1.7 | −31.5 / 0.4 |
| jitter 1, share 0.99, 0.2 m | −31.2 / 3.3 | −31.1 / 2.4 | −30.4 / 2.1 | −31.4 / 0.4 |

## Why nothing moves

Above 1–2 kHz the board's modes overlap: η·f·n(f) is 9 at 4 kHz (η = 0.02·(1 + f/2500), one mode every 21.6 Hz). There the driving-point mobility is the infinite plate's, 1/(8√(D·ρh)), set by the plate's stiffness and mass where it is driven. Ribs as stiffness lines, regular or not, change neither at the bridge points, and the conductance stays flat at −31 dB with less than 2 dB of spread. The real board's falling, rough mobility up there needs what the model does not have:

- **the bridge as a beam**, its mass and stiffness along its line, with the drive points on it (`bridge_mass` is 0);
- **a plate far softer across the grain than along it** (real spruce: 12–20 times; the model's smeared `dx`/`dy`: 2.25), with the ribs' mass, so that the ribs confine the waves between them;
- a numerical method fine enough for both (finite elements), with a build time far beyond the current 0.2 s.

## Kept

- `rib_jitter`, off by default, beside `rib_spacing` and `rib_share`.
- `examples/rib_mobility.rs` (piano-core): the conductance per third octave over the keys. `board_evenness` takes `RIB_SPACING`, `RIB_SHARE`, `BELOW`, `FROM` and `TO`; `rib_mobility` also `RIB_JITTER`.
