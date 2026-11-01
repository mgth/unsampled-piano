# The Steinway D's dimensions

What could be found in print about the Steinway D-274 (the concert grand), gathered on 2026-10-31 for a concert-grand preset modelled on it: the target is the commercial modelled piano's D, the preset gets a generic name. Each figure is given with its source; the last column of the mapping table says what the model takes from it.

## Sources

- Steinway & Sons, Model D specifications: https://www.steinway.com/pianos/steinway/grand/model-d (also https://msteinert.com/steinway-model-d and https://www.troy.edu/student-life-resources/arts-culture/davis-theatre/technical-details/steinway-sons-concert-grand-model-d.html, which repeat it, with a few differences noted below).
- Steinway stringing scales: https://www.steinwayreference.com/steinway-stringing-scales/
- Steinway action regulation: https://www.steinwayreference.com/action-regulation-specifications/ and the Worldwide Technical Reference Guide, chap. 2: https://www.indyptg.org/PTG/SteinwayServiceManuals/Chp2.pdf
- Wikipedia, Steinway D-274: https://en.wikipedia.org/wiki/Steinway_D-274
- J. Chabassier's thesis (2012, the IRCAM Steinway D), read earlier for docs/bridges.md: https://pastel.hal.science/pastel-00690351/file/These.pdf.
- J. Chabassier & M. Duruflé, *Physical parameters for piano modeling*, INRIA RT-425 (2012): https://hal.inria.fr/hal-00688679v2/file/RT-425.pdf. Every key C1–B7 of the same piano: speaking length, (homogenised) diameter and density, tension, hammer, strike point and bridge point; the soundboard's woods and thicknesses. Parsed into `out/steinway_d/rt425_{wrapped,unwrapped}.csv` (not versioned; `parse_rt425.py` beside them reads the `pdftotext -layout` output). The "wrapped" table is the real piano's lengths, its wound strings homogenised to one diameter and one density; the "unwrapped" one is a virtual all-steel piano with longer bass strings.
- A. Chaigne, J. Chabassier, M. Duruflé, *Energy analysis of structural changes in pianos*, Vienna Talk 2015, Table 1 (the Steinway D's C♯5): https://viennatalk2015.mdw.ac.at/proceedings/ViennaTalk2015_submission_19.pdf
- Overs Pianos, scaling charts ("F21 at 183 cm speaking length for the D"): https://www.overspianos.com.au/chrts.html
- Rib count: a figure caption, "Steinway model D. Geometry of the soundboard with 17 ribs, one cut-off corner": https://www.researchgate.net/figure/Steinway-model-D-Geometry-of-the-soundboard-with-17-ribs-one-cut-off-corner-and-a_fig3_232244409
- String count per note: a Piano World post quoting a Steinway article, "8 notes with single bass strings, 5 notes with double bass strings, and the rest … 3 steel strings": https://forum.pianoworld.com/ubbthreads.php/topics/225867/number-of-bass-strings.html (secondhand).
- Hammer strike weights of a Steinway (model not stated), CAUT list: https://www.moypiano.com/ptg/caut.php/2011-March/011123.html (secondhand).

## The instrument

| | Steinway D | Source |
|---|---|---|
| Length × width | 274 × 156 cm (Steinway UK: 157) | Steinway |
| Net weight | 480 kg (steinway.com: 483 kg) | Steinway |
| Rim | hard rock maple, 17 laminations (Troy: 18), 3¼″ = 8.26 cm thick | Steinway, Troy |
| Soundboard | quarter-sawn Sitka spruce, 9 mm at the centre tapering to 6 mm towards the rim, "double crowned" | Steinway |
| Soundboard size | ≈ 1.99 m along the spine × 1.46 m (read off Chabassier's Figure 6); area "3209 sq in" (2.07 m²) or "almost 2 m²" | docs/bridges.md, dealer pages |
| Ribs | sugar pine, 17, one corner cut off; cross-section not found | Steinway, ResearchGate caption |
| Braces | 5, solid spruce, 47,637 cm³ (Troy: 66,400 cm³) | Steinway |
| Bridges | treble: vertically laminated hard rock maple, capped; bass: continuous with the treble's design, doweled, glued and screwed | Steinway |
| Plate | grey iron | Steinway |
| Scale | overstrung, front and rear duplex, combination agraffe | Steinway |
| Total tension | 45,373 lb = 20,418 kg (≈ 200 kN) | Steinway |
| Strings | 8 single (A0–E1), 5 bichords (F1–A1), the rest trichords: 8 + 10 + 225 = 243 strings | Piano World (secondhand) |
| Bass bridge | 20 keys (the 1884 scale; 17 before), A0–E2; plain wire from F2 | Wikipedia, steinwayreference |
| Wound strings | on the bass bridge only: Swedish steel core, pure copper winding; gauges not found | Steinway, steinwayreference |
| Treble wire | 12 whole and half sizes, high-tensile Swedish steel: 13½ (×4), 14 (4), 14½ (4), 15 (5), 15½ (8), 16 (6), 16½ (6), 17 (8), 18 (6), 19 (7), 20 (6), 21 (4); size per string count, top first presumably | steinwayreference |
| Longest string | 201 cm, agraffe to bridge | Steinway |
| Speaking lengths | C1 2.007, E2 1.378 (last wound), F2 1.837 (first plain), C4 0.657, C5 0.344, C6 0.180, C7 0.095, B7 0.052 m | RT-425 |
| Agraffes | ≈ 30 cm in front of the board's front edge at D♯1, 16 cm at F3, 7 cm at C♯5 | Chabassier, docs/bridges.md |
| Bass bridge's place | A0 end at 87 % of the board's length and 25 % of its width from the spine, E2 end at 55 % and 44 %; ≈ 0.69 m long, bowed ≈ 4 cm towards the tail | Chabassier, docs/bridges.md |
| Tensions | wound strings 1.72–1.80 kN at C1–E1 falling to 0.74 kN at E2; plain wire 0.80 kN at F2, 0.74 at C4, 0.70 at C5, 0.67 at C7, 0.59 at B7; mean over C1–B7 825 N | RT-425 |
| Plain wire | 1.117 mm at F2, 1.006 at C4, 0.932 at C5, 0.891 at C6, 0.831 at C7, 0.743 at B7; steel 7850 kg/m³, E 2.02 × 10¹¹ Pa | RT-425 |
| Strike point | 0.12 of the speaking length at every key (0.241 m from the agraffe at C1, 0.079 at C4) | RT-425 |
| Hammers | mass M_H = 11.2 − 0.062·i g (i = 1 at A0): 10.95 g at C1, 8.71 at C4, 5.78 at B7; felt F = K_H·u^p, p from 2.31 (C1) to 2.28 (C2), 2.42 (C4), 2.84 (C6), 3.51 (B7); K_H = 10^(0.0531·i + 7.64) N/m^p; Steinway's own figures: felt set 18.5 lb (8.39 kg, Troy only), strike weights of a Steinway ≈ 14.8 g in the bass to ≈ 6.9 g at the top (CAUT, model not stated) | RT-425 eqs. 2–5 |
| Soundboard woods | spruce 380–390 kg/m³, E_x 11.0–11.6 GPa, E_y 0.65–0.9 GPa; bridges in beech, 750 kg/m³, E_x 13.7 GPa | RT-425 |
| Thicknesses | plate 6–9 mm; ribs 9–35 mm (plate included); bridge 29–69 mm | RT-425 |
| Board damping | per mode, a = 2 × 10⁻⁵, b = 0.07, c = 0 (RT-425 eq. 1; the law's form is in the thesis) | RT-425 |
| Keys | longest 62.2 cm; dip 0.400″ (10.2 mm, 0.390–0.420); down weight 51 g bass, 48 g middle; keybed 4.45 cm | Steinway, steinwayreference, Technical Reference Guide |

RT-425 tunes A4 at 441 Hz and gives no A0–B0 or C8. Its C1 string, 2.007 m, is Steinway's "longest string, 201 cm": A0–B0, on the same bass bridge, are presumably no longer.

Not found: height of the case and of the rim, the lid's size and its prop's angles, rib spacing and widths, the real bass strings' core and winding, the damper's highest key, plate dimensions.

## What the tables give the model

**Scale.** The plain strings F2–B7 follow the model's law (`StringScale::length`) closely: a least-squares fit of log L gives 1.91 per octave and C8 at 4.94 cm, every key within −0.5 % (A5) to +1.9 % (G♯2) of the law. Per range: 1.92 (F2–B3), 1.91 (C4–B5), 1.91 (C6–B7). The wound strings are not the model's straight line in key number from `len_a0` down to `break_note`: they fall from 2.007 m (C1) to 1.378 m (E2), shorter than the plain F2 above them (1.837 m). The model's line cannot step down at the break; with `break_note` 41 and `len_a0` 2.01 it makes E2 1.84 m where the D has 1.38.

**Inharmonicity.** B = π³·E·d⁴ / (64·T·L²) from the "wrapped" table, fitted to the model's law (`InharmonicityLaw::b`, log B rms 0.064): a_bass −0.1163, b_bass −6.727, a_treble 0.0903, b_treble −13.526. Against the default (fitted on Salamander, a Yamaha C5):

| Key | C1 | F♯1 | C2 | F2 | C3 | C4 | C5 | C6 | C7 | B7 |
|---|---|---|---|---|---|---|---|---|---|---|
| B, Steinway D | 6.8e-5 | 6.4e-5 | 5.1e-5 | 5.6e-5 | 1.04e-4 | 3.1e-4 | 9.0e-4 | 2.7e-3 | 7.7e-3 | 1.9e-2 |
| D / default | 0.41 | 0.66 | 0.69 | 0.76 | 0.97 | 1.11 | 1.11 | 1.17 | 1.14 | 1.05 |

The D's bass is 1.5–2.5 times less inharmonic (its longer strings), the middle and treble 10–17 % more. The homogenised wound strings were fitted to the strings measured on the IRCAM piano (RT-425, Figure 1), so their B is the real one, their d and ρ are not.

**Hammers.** Stulov's masses (`hammer_stulov`, the model's) are 11.0 g at A0, 8.3 g at C4 and 5.3 g at C8; the D's are 11.1, 8.7 and 5.7 g: `hammer_mass_scale` 1.0–1.07 (1.01 at A0, 1.05 at C4, 1.07 at C8). Their felt exponents differ: Stulov 3.7 → 5.0, the D's 2.3 → 3.5. The two felt laws are not the same model (the model's has Stulov's hysteresis, RT-425's a dissipation coefficient R_H), and the model's hardness is calibrated on the bench: the exponent is left to a listening test, not copied.

**Bridge points.** RT-425's frame has its origin at the board's corner at the spine and the belly rail, x across (towards the treble), y along the spine. In the model's normalised coordinates (along the board from the front, across from the treble side; board 1.99 × 1.46 m):

| Key | C1 | D♯1 | A1 | E2 | F2 | A2 | C3 | C4 | C5 | C6 | C7 | B7 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| along | 0.849 | 0.819 | 0.698 | 0.558 | 0.794 | 0.633 | 0.533 | 0.266 | 0.126 | 0.055 | 0.010 | −0.015 |
| across | 0.726 | 0.685 | 0.616 | 0.575 | 0.842 | 0.781 | 0.726 | 0.548 | 0.411 | 0.281 | 0.151 | 0.055 |

The bass bridge is where the model already puts it (its A0 end at 0.867, 0.753; E2 0.558, 0.575 against the model's G♯2 end at 0.553, 0.562). The main bridge's bass end, F2, lies at (0.794, 0.842), further back and nearer the spine than the model's (0.666, 0.749); the treble end lies on the front edge (B7 1.5 % in front of the spine's corner: the belly rail is not square to the spine). These are the strings' real, slanted ends: `treble_bridge` describes upright strings that `main_string_slant` then turns, so they cannot be copied into it as they are.

## Measured references

- `out/catalog/steinway_d/`: the commercial modelled piano's D, recorded on 2026-10-23 (effects, reverb, limiter and EQ off; every sixth key at velocities 60 and 100, held 3.5 s; docs/instrument_range.md). `out/catalog_clean/steinway_d/`: the same D on 1 November 2026, all 88 keys, with a control take showing the earlier one was indeed dry. `out/catalog_clean/steinway_d_no_hammer_noise/`: the same, with its hammer noise set to 0.
- Plath's dataset (`/mnt/local/SSD_B-CT4000/Piano/concert-piano-plath/`): a new Steinway concert grand (the record does not say D; the setup photos show a Steinway concert grand, lid off), anechoic, bridge acceleration and dummy-head pressure for every key.

## Mapping to the model

| Parameter | Default (`GRAND_2M`) | Steinway D | From |
|---|---|---|---|
| `scale.len_a0` | 1.45 | 2.01 | longest string |
| `scale.len_c8` | 0.052 | 0.049 | fit, above |
| `scale.ratio_per_octave` | 1.85 | 1.91 | fit, above |
| `scale.break_note` | 48 | 41 (F2) | first plain-wire key; the wound strings' fall to 1.38 m at E2 does not fit the law (above) |
| `scale.tension` | 750 | 750 | RT-425's plain wire through the model's ramp (`unison.tension_share`): 753 N fits F2–B7 within 1.4 dB rms. The mean, 825 N (200 kN over 243 strings), is the wound strings' 1.7 kN pulling it up |
| `inharmonicity` | Salamander's fit | −0.1163, −6.727, 0.0903, −13.526 (RT-425); the preset takes the recorded D's, −0.1608, −5.842, 0.0843, −13.062 | fit, above; "Matched to the recorded D" |
| `hammer_mass_scale` | 1.0 | 1.0–1.07 | RT-425 against Stulov |
| `soundboard.main_bridge_from` | 45 | 41 | 20 keys on the bass bridge |
| `unison.bichord_from` | 28 | 29 (F1) | 8 single strings |
| `unison.trichord_from` | 45 | 34 (A♯1) | 5 bichords |
| `losses.wound_until` | 48 | 41 | wound strings on the bass bridge only |
| `soundboard.a` × `b` | 1.70 × 1.30 | 1.99 × 1.46 | Chabassier's Figure 6 |
| `soundboard.rib_spacing` | 0 (off) | ≈ 0.11 | 17 ribs over ≈ 2 m (an inference: spacing not found) |
| `rim.thickness` | 0.075 | 0.083 | Steinway |
| `strike_ratio` | 0.12 | 0.12 | C♯5 |
| `soundboard.bass_bridge` | the D's proportions | unchanged | docs/bridges.md |

The preset is `presets/instruments/Concert grand/concert_grand.toml`: these dimensions, then the recorded D's inharmonicity and stretch, `losses.b2` and `core_depth_mid` (below, "Matched to the recorded D"). What the model's laws cannot follow, per key:

| | Model, with the preset | Steinway D (RT-425) |
|---|---|---|
| Wound strings' lengths | A1 1.90, C2 1.88, E2 1.84 m | 1.76, 1.60, 1.38 m |
| Bass tension | 750 N on the single strings (C1–E1), the ramp's 375–550 N on F1–F2 | 1.70–1.80 kN, 1.62 kN at F1, 0.74 kN at E2 |
| Treble tension | 750 N from C5 up | 696 (C5), 670 (C7), 588 N (B7) |
| Strike point above C6 | 0.12 falling to 0.07 at C8 | 0.115–0.12 |
| Felt exponent | 3.7 (A0) to 5.0 (C8) | 2.3 to 3.5 |

The wound strings' wave impedance √(T·μ) = T/(2Lf) is about half the D's in the single strings (C1: 5.8 against 13.1 kg/s).

Left as they are for now: the board's surface mass and stiffnesses (`rho_h`, `dx`, `dy`), calibrated on the bench and not a raw plate's (spruce at 380 kg/m³ over 6–9 mm gives 2.3–3.4 kg/m² before the ribs, the model has 10.5); the felt's exponent and hardness; the frame, the lid and the case heights.

## Matched to the recorded D

The target is the commercial modelled piano's D (`out/catalog/steinway_d/`, every sixth key A0–A7 at velocities 60 and 100, held 3.5 s, its reverb and effects off). The model plays the same notes through the close microphones without a room (`archive/presets/listening/Microphones/close.toml`, `Room/dry.toml`). The distance (dB rms, per register, then over the registers and families) takes, per key: partials 2–8 re partial 1 at the attack (0–60 ms) and in the sustain (0.5–1 s) at both velocities, the envelope at 0.3, 1, 2 and 3 s, the attack centroid / f0, the level across the keyboard re its median, and ff − mf. Partials and levels below −60 dB count as −60.

The other commercial models, recorded the same way, lie 6.4 (Bechstein DG) to 7.0 dB (Steinway B) from the D: below that, the distance no longer tells one D from another grand. Their treble also keeps 7–10 dB more partials than the D's, at the attack and in the sustain: the D's pure treble is the D's own.

| | Distance | Bass | Middle | Treble | Top |
|---|---|---|---|---|---|
| Default instrument | 9.11 | 7.69 | 6.78 | 12.61 | 8.26 |
| Dimensions only (above) | 9.61 | 7.82 | 9.19 | 12.48 | 8.25 |
| + the recorded D's inharmonicity and stretch | 9.65 | 7.79 | 8.86 | 12.34 | 8.98 |
| + `losses.b2` 3.0e-7 (5 × the default's) | 7.69 | 7.81 | 8.03 | 9.16 | 5.19 |
| + `core_depth_mid` 0.5 mm: **the preset** | 7.43 | 7.80 | 7.77 | 8.53 | 5.17 |

The dimension groups alone (scale, board, layout, inharmonicity with hammer masses) each move the distance by less than 0.2. The treble's sustain was 12–16 dB too rich in partials 2–8: only the frequency-dependent string loss reaches it (`b2` × 2: 8.64, × 4: 7.85, × 6: 7.60, × 8: 7.73, × 12: 9.49; beyond × 6 the mf treble dies too fast). Tried one at a time over `b2` 3.0e-7, with no gain: `hardness_slope_bass` 0 or −0.02 and `hardness` 0.15 (the core under the felt sets the contact), `core_hardness` 1 or 4, `core_p` 5, `core_depth_bass` 0.5, `trichord_from` 45, `b1_offset` −4.8, `rib_spacing` 0.11. The default board's size (1.70 × 1.30) brings the bass from 7.80 to 7.47 but the middle from 7.77 to 9.22 (7.83 overall); `rho_h` 8 or 13: 7.47 and 7.76.

The recorded D's inharmonicity (`tools/fit_preset.py` on its survey, log B rms 0.15): A0 1.1e-4, C2 5.3e-5, C4 3.3e-4, C6 2.5e-3, C8 1.9e-2, within 5–10 % of RT-425's strings from C2 to C8 (5.1e-5, 3.1e-4, 2.7e-3, 1.9e-2 at B7). The commercial D's strings are the real D's.

Left (mean deviation from the D): the bass, 4 dB short of the D's partials 2–8 at the attack and 7–8 dB in the sustain, whatever the hammer; the middle's attack, 4–6 dB too rich, and its envelope 2–4 dB too long; the treble's decay, 4 (ff) to 10 dB (mf) too fast with the higher `b2`, and its level 4 dB low; ff − mf at the top, 7.7 dB larger than the D's.

On the bench (real pianos, docs/bench.md) that first version scored 1.93 against the default's 1.06 (brightness 3.30, sustain 2.21, decay 1.47; registers 1.68 / 2.11 / 1.60 / 2.28): the recorded D's treble, poorer in partials than the commercial piano's other grands, is also poorer than the real pianos'. The preset follows the D, not the bench.

### After a first listening

Heard live (2026-10-31): A2 rang on; the fourth octave, and less the fifth, sounded dull; the first octave's attack had lost a high component. Measured on every key around them, at velocities 60 and 100 (energy above 2 kHz in the first 60 ms re the whole, G#2–A#2's peak re F2 and B2–D3):

- **Partial presets.** A preset read on its own sets every missing field to its serde default, and for three fields that is the value of presets saved before them: `shank_freq_mid` 0 (uniform shank: the D#4–F4 frieze back, the fourth octave's mf attack 9 dB duller), and in a `[soundboard]` section `bass_bridge_bow` 0 and `main_string_slant` 0. The preset now sets them to the default's. Every single-variable comparison above was made with them reset; the group runs and the grids keep their ranking, not their figures.
- **A2 and the first octave, measured again.** The first versions of these two fixes (`crown_freq` 110, `load_ramp` 0) were judged key by key against each note's own total, which hides a note's level and its lost low end, and they removed some: heard as "only lost bass", confirmed on the 88-key take (`out/catalog_clean/steinway_d_no_hammer_noise/`) with one gain per set, set on A2–A5. At A0–D2, `crown_freq` 110 leaves the attack 5 dB short at 50–100 Hz and 5 dB over at 100–200 Hz. 90 / 80 / 70 / 60 Hz give −1.8 / −0.1 / +1.3 / +2.2 and +4.0 / +2.7 / +1.4 / +0.8 dB. A2's fundamental against its neighbours (1–3 s): 110 Hz −1.5, 90 Hz +3.3, 80 Hz +2.4, the D −0.9, whose own G#2 reads +3.1: `crown_freq` 80. `back_gain`, the piston's frequency, loss and radiation, and `loss_low_corner` hardly move these bands. `load_ramp` 0 pushes A#1 and the keys above it too loud, and 36 or 12 bring the loudness per key from 2.4 to 2.1 dB rms: back to 36, pinned. `strike_ratio` 0.125, suggested by the notches at partials 8, 16 and 24 on four keys, is worse over A0–B2 (10.9 against 10.2 dB rms), and the comb fitted on each note's partials puts the D at 0.113–0.1225 from A0 to B1, 0.12 from C2 to B2 (the fit gives back 0.12, 0.125 and 0.13 on the model's renders): left at 0.12. `main_bridge_from` stays at the default's 45.

`concert_grand` was kept by ear (2026-11-01) over a brighter variant, without top felt over the core and with `b2` 1.2e-7, which scored better on the bench and further from the D. Both had the three fields, `crown_freq` 80, `main_bridge_from` 45 and `load_ramp` 36. Against the D without hammer noise:

| | Distance to the D | Bass | Middle | Treble | Top | Bench |
|---|---|---|---|---|---|---|
| default | 8.88 | 7.57 | 6.27 | 12.49 | 7.91 | 1.04 |
| `concert_grand`: `core_depth_mid` 0.5, `b2` 3.0e-7 | 6.86 | 6.31 | 7.36 | 8.48 | 4.74 | 2.06 |
| brighter variant | 7.47 | 6.32 | 7.49 | 9.41 | 6.24 | 1.36 |

Still open at A0–D2 for `concert_grand`, one gain per set: the fundamentals under 50 Hz 11 dB over in the body, which keeps the rms of the partials under 200 Hz at 7.6 dB (default 6.4) although 50–100 Hz is now level at the attack and 3 dB over in the body; 800 Hz–3.2 kHz 3–6 dB over at the attack, more at mf than at ff, where the D is much darker; A#1 5 dB over in loudness, and C8 8–10 dB. The bench's bass register goes from 1.49 to 1.64 with these two settings.

### What the model does not reach: the D's bell in the low bass

Heard against the recorded D (2026-11-01, A0–D#1, mostly B0 and C1 at velocity 100): a high percussive component, "like a bell", "as if the string were much tighter". A chain of listening tests on the recording, each answered by ear, narrowed it down:

| Test | Heard |
|---|---|
| The D without a harmonic series of narrow lines at 1.2–8 kHz (B0: multiples of 463 Hz), the lines alone | not them |
| Our preset, its content above 1 kHz taken from the D | acceptable, much better |
| The D with one band removed over the whole note | 2–4 kHz removes the most, depending on the note |
| Our preset above 1 kHz: level equalised on the D (static), pulses smeared, channels balanced, and all three | none |
| The D's content above 1 kHz split by median filtering: only its tonal part moved to our preset | much closer; a low component is still missing, above the dominant |
| `strike_ratio` 0.125, 0.13, 0.14, hammers × 0.8 | 0.13 best, still far |
| `b2` ÷ 2.5 and ÷ 5, no winding loss (the high partials' decay then within 0.7 dB/s of the D) | none |
| All partials: amplitudes against frequencies swapped with the D | the amplitudes |
| Amplitudes in steps: fixed level of each partial, then its decay, then its whole envelope | the fixed level |
| That fixed level split in two: a profile by partial number common to the low keys, and what is left per note | what is left per note |

So the bell lies in the level of each partial of each note, around 6–8 dB rms once the common profile is taken out (B0, C1), not in a law the model could follow. The rest of the gap, by partial number, is a pattern of period 8: the D is 8–27 dB under the model at partials 8, 16, 24, 32–33, 40, 48 and 4–9 dB over at 13, 21, 29. This does not look like a plain strike comb, whose peaks would fall half way between the notches, and it is not heard as the bell. An estimate fitted to a strike comb gives 0.113–0.1225 from A0 to B1; the notches at multiples of 8 would rather say 1/8, so the D's strike point in this register is not settled.

The roughness of the partials' levels is the same in the D and the model (9.9 and 9.8 dB rms on B0), so nothing shows the model's pattern to be too smooth: the two differ in which partials stand out. Copying the D's levels note by note would reproduce it, and would no longer be a model. The tools, in the session's scratch files, are not versioned; `out/ab_*` holds the listening pages.
