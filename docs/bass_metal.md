# The metallic bass

Status as of 2026-10-27.

## Listening report

Beethoven op. 31 no. 2, finale, against Liang's Disklavier recording: "the bass is really far from the original: it is more metallic".

## What it was

Liang's dataset also holds every key of the same Disklavier at velocity 96, isolated. The model played the same notes (close microphones, studio); octave bands of the sustain (0.4–1.4 s) re the partials' energy, model minus recording:

| | 1–2 kHz | 2–4 kHz | 4–8 kHz |
|---|---|---|---|
| A0–B1, attack (0–0.15 s) | +0.3 dB | +0.4 dB | −3.1 dB |
| A0–B1, sustain | +6.3 dB | +15.5 dB | +19.8 dB |
| C2–B2, sustain | −0.6 dB | +3.5 dB | +3.1 dB |

The attack is right; the low bass's high partials then ring on. Their decay over 0.1–1 s, 2–4 kHz:

| | A0 | C1 | E1 | G1 | B1 | E2 | A2 |
|---|---|---|---|---|---|---|---|
| recording | −47 | −35 | −27 | −23 | −24 | −16 | −10 dB/s |
| model | −6 | −8 | −17 | −11 | −18 | −15 | −12 dB/s |

Neither the phantom partials (nonlinearity off), the sympathy, the frame nor the rim: each switched off in turn left the table within 1.5 dB. It is the strings' own loss. The loss law σ_k = b1 + b2·f_k² (b2 fitted on Salamander, the same for every string) gives the low bass's partials at 3 kHz, its 50th–100th, hardly more loss than its fundamental.

## The winding

The extra loss these partials need, from 1 to 3 kHz, grows about as the frequency: a constant loss factor, ≈ 3–5·10⁻⁴. On every string it would cut the middle's decays (−12 dB/s more on C4's partials at 1 kHz). It belongs to the wound strings, whose copper winding rubs on the steel core as the string bends, and above all to the lowest ones, wound in two layers.

`losses.winding` (η) adds ½·η·ω_k to every partial, full at A0 and fading out linearly to `losses.winding_until` (E2).

A term on the share of each partial's energy in bending, B·k²/(1 + B·k²), was tried first. It fitted the 2–8 kHz band but left 1–2 kHz ringing (that share is 0.04–0.19 there), and was dropped.

| winding | A0–B1 sustain, 1–2 / 2–4 / 4–8 kHz (re Liang) | bench |
|---|---|---|
| 0 | +6.3 / +15.5 / +19.8 dB | 1.13 |
| 2·10⁻⁴ | +2.7 / +7.7 / +8.0 dB | 1.14 |
| 4·10⁻⁴ | −0.6 / −0.1 / −5.8 dB | 1.18 |

The eight grands of the bench have a brighter bass than Liang's Disklavier: at 4·10⁻⁴ the bass's sustain at 1–2 and 8 kHz fell 4–18 dB under them, and its envelopes decayed ≈ 2 dB faster. At 2·10⁻⁴ the bass's 4 kHz sustain lands on their mean (z +1.1 → +0.3). The user heard no difference between 2 and 4·10⁻⁴; 2·10⁻⁴ is the default.

## The 220 Hz line

Listening again to the same passage (24.7–28.2 s: A1, D2, E2, F2 in the left hand, D3–D4 above): "still metallic, and above all not precise". By third-octave band the model stood +7 dB over the recording at 200 Hz, all of it one line: 220 Hz (A), fed by D2's third partial (+20–24 dB against Liang's isolated D2), A1's fourth and A3. The recording plays this passage the same with and without the pedal (≤ 1 dB), and the model's line stays without the sympathy: it is in the notes.

D2's third partial (219.7 Hz) lies 5 Hz under the board mode (1,3) at 224.8 Hz and forms a strongly coupled pair with it (T60 4.6 s). More board loss (η 0.035), more back-face short circuit (`back_gain` 0.9) and `loss_low_corner` 300 leave it within 2 dB. The grand's outline (docs/outline.md) brings it to +4–10 dB but moves the bumps elsewhere: on the passage, 200 Hz goes from +6 to −5 dB and 400 Hz from +4 to +7.

Against the bench's six grands it is Liang's Disklavier that stands apart. Partials below 700 Hz of A0–C4 at the top layer, dB re each piano's median, by third octave:

| | 112 Hz | 141 | 178 | 224 | 283 | 356 | 449 | 566 | 713 |
|---|---|---|---|---|---|---|---|---|---|
| six grands, attack | +6.9 ± 2.7 | +7.1 ± 2.3 | +4.3 ± 3.0 | **+6.0 ± 2.6** | +1.3 ± 2.4 | −1.3 | −0.3 | −3.2 | −4.6 |
| Liang, attack | +11.1 | +9.5 | +6.9 | **+1.9** | −1.2 | +0.4 | +1.5 | −1.5 | −1.0 |
| model, attack | **+16.5** | +7.5 | +6.3 | +7.8 | −4.7 | −0.3 | −2.9 | −3.5 | −7.0 |
| six grands, sustain | +8.6 ± 2.3 | +5.4 ± 2.9 | +5.7 ± 2.8 | **+5.3 ± 2.5** | −0.2 ± 3.1 | −1.8 | −1.5 | −4.5 | −5.5 ± 3.0 |
| Liang, sustain | +11.9 | +10.3 | +5.9 | **+0.3** | −6.2 | −2.5 | −0.8 | −4.6 | −3.0 |
| model, sustain | **+19.4** | +5.7 | +8.5 | +7.3 | −4.9 | +0.1 | −3.0 | −4.5 | **−13.4** |

At 224 Hz the model lies within the grands (+7.8 against +6.0 ± 2.6); Liang's Disklavier is hollow there, 4–5 dB under all the others. Lowering the model's 220 Hz would fit that one piano and leave the rest. What the model does have wrong in this register: 112 Hz, the board's first mode, 10 dB over the grands' mean (`loss_low_corner` 300 takes 3–4 dB of it), and the bass's partials near 700 Hz in the sustain (below).

For listening, out/ab_200 holds the passage (22–32 s) from the recording, the model, and the model with a 7 dB hollow at 222 Hz (an output equaliser, not a model change), to tell whether that line is what is heard.

The bass's partials 8, 16 and 24 (A0–A1) also stand 20–38 dB over Liang's: the recording has the strike point's notches there, the model hardly.

## "Not precise"

The user heard no difference with the 220 Hz hollow: the blur is elsewhere. Leads measured on the same passage:

- **Radiation damping.** The loss law gives every mode the same η, its radiation included. The Rayleigh self-impedance of the rectangle's modes, with the model's back face as an antiphase image (β 0.6, 0.6 m), gives η_rad 0.021 for (1,1) and < 0.006 for every other mode below 400 Hz. Only the first mode would gain it; the lingering below needs several times more.
- **Articulation**: how far each band falls between successive left-hand notes (the lowest level in the 30 ms before a note, re the peak of the one before). On 24.7–28.2 s without the pedal, 90–140 Hz falls 9.6 dB in the model against 14.5 dB in the recording, 40–90 Hz 11.9 against 19.1. `loss_low_corner` 300 brings 90–140 Hz to −17.0; firmer dampers (`damper.strength` 0.3) to −14.1. Neither touches 40–90 Hz. Over 22–32 s with the pedal, though, the medians agree (90–140 Hz: −16.5 model, −15.1 recording): the lingering is local to that figure (A1's second partial lies on the board's first mode, 108.8 Hz).
- **No late boom**: each band's peak after the 1–4 kHz onset comes no later in the model (median 32–41 ms) than in the recording (42–59 ms).
- **The attack's pulse train** (1–6 kHz envelope of the isolated notes A1–A2): the string's returns every period are as deep in both (12–21 dB), but the recording's first pulses are weak and grow over 2–3 periods, while the model's first is already among the strongest.

Bench: `loss_low_corner` 200 scores 1.14, 300 1.15 (baseline 1.14). With 300 the early T60 of partials 1–4 shortens, bass z −0.8 → −1.4 and middle −0.5 → −1.0. A/B in out/ab_flou (Beethoven 22–32 s with the pedal: recording, now, 200, 300): the user picked 200, now the default.

## The first pulse, and the unisons' load

The pulse-train lead does not hold. Each note of Liang's recording opens with 25–30 ms of low noise (the action, −20 to −40 dB re the string's attack); aligned on the string's own arrival, the model's first 1–6 kHz pulse is no stronger than the recording's (+0.7 dB on the mean of 13 keys, A0–D3).

What the bench does flag in the bass is the early decay (the first 15 dB): partials 5–12 at 3.7 s against 6.2 on eight grands, the only bass measure off. By key, F#1–C2 lose their first 15 dB 2–2.5 times too fast, every partial range, and A2–A3 1.4–2 times; A0–D#1 and C4 up are right. The edge loss, the windings, the frame, the rim and the sympathetic strings each move it by less than 10 % (F#1–A1's partial 1 aside, edge loss). Single strings up to E2 bring F#1–C2 to the grands.

The scale gave every string 750 N: a key's load on the bridge, Σ √(T·μ), doubled at the first bichord (E1) and rose by half at the first trichord (A2), and the in-phase unison mode lost its energy that much faster. `unison.load_ramp` starts each new unison at (n−1)/n of the tension, so the load does not jump, and brings it to full over that many semitones:

| `load_ramp` | bench | better / worse (\|Δz\| ≥ 0.5) | bass p1–4 / p5–12 early T60 (grands 8.2 / 6.2 s) |
|---|---|---|---|
| 0 | 1.14 | | 6.1 / 3.7 s |
| 12 | 1.11 | 2 / 0 | 9.0 / 4.3 s |
| 24 | 1.10 | 12 / 0 | 9.4 / 5.2 s |
| 36 (default) | 1.09 | 19 / 1 | 9.4 / 5.2 s |

36 also brings the middle's partials 4–12 and centroid towards the grands; the top's balance gets 0.5 z worse (+1.3). On the Beethoven passage (out/ab_tension: recording, 0, 24, 36) the user heard 24 and 36 a little better than before and alike, and the whole still far from the recording.

## Still far: the stereo image

Liang's pair (spaced omnis about 50 cm above the strings) hears the bass almost identically: L/R correlation +0.95–0.99 at 63–125 Hz, +0.5 at 160 and 315 Hz, the bass 2–3 dB to the left. The model with the close microphones, over 22–32 s:

- **Antiphase.** +0.8 at 50–100 Hz, then −0.76 at 160 Hz and −0.72 at 315 Hz. The rectangle's modes are symmetric or antisymmetric about its centre, and the two microphones sit symmetric about it: the (2,1)/(1,2) pair near 145 Hz and the modes near 315 Hz reach them in opposite phase. Without the room, without the back face, with the engine's own microphones at 1.5 m or other pairs at 50 cm, 315 Hz stays at −0.5 to −0.7; across the keyboard at mid-length, 160 Hz goes to −0.1. The grand's outline (docs/outline.md) with that pair gives +0.9 at 63–125 Hz and +0.6 at 160 Hz.
- **Sides.** The close pair puts the bass on the right (−8 dB at 63 Hz); the recording has it on the left.
- **Placement.** Moving a pair 10 cm swings the bass between in phase and antiphase and the balance by up to 20 dB: the model's field above the board follows its modes far more than the recording's does.

The long-term spectrum of the same 10 s, model − recording (dB re the total): +7 to +10 at 50–80 Hz, +6 at 200 Hz, +6 at 1.26 kHz, −4 to −7 at 4–8 kHz.

A diagnostic A/B in out/ab_diag: the recording; the model now; the model with its channels swapped and its two channels summed below 400 Hz; the same with a third-octave EQ onto the recording's long-term spectrum.

## The "PVC tube" in the attack

What the user hears in the model's bass, on Beethoven and on isolated notes (A1, D2): a metallic, synthetic resonance, "as inside a long PVC tube", or "a ruler twanged on a table's edge". It is clearly in the attack; the model's sustain is a little metallic too. It is there with no room and no back face. The bench does not flag it.

Heard and ruled out (A/B, the tube stays): the 220 Hz hollow; damping the board's lowest modes (kept, cleaner articulation); the unisons' tension ramp (kept, a little better); the bass on the left and coherent below 400 Hz plus an EQ onto the recording's long-term spectrum; everything below 90 Hz lowered by 10 or 20 dB; the grand's outline; an irregular offset of each wound string's partials (it removes a sweeping "flanger" null of the unisons, see below, not the tube).

Pointed to: of five effects added to Liang's A1 (a long tube's comb resonance, a flutter echo, a dispersive "spring" echo, partials locked to exact harmonics, a rattle), the spring echo (120 first-order allpasses at 0.6, added 30 ms late at 0.8) brings the recording closest to the model, "less bad". Splicing the model's first 80 ms onto the recording brings the tube; the recording's attack onto the model removes most of it. Band and 20 ms splices inside the attack were too audible to judge.

Measured on A1 and D2 at velocity 96 (Liang's isolated notes against the model, close microphones):
- **The first pulse**: aligned on the string's arrival (Liang's notes open with 25–30 ms of action noise), the model's first 1–6 kHz pulse is no stronger than the recording's.
- **The pulse train**: a 192-point spectrogram of the first 150 ms shows a regular train of slanted chirps, one per string period, at 4–7 kHz (A1) and 2.5–5 kHz (D2). Liang's attack is more continuous there.
- **The hammer**: the contact force reaches half its peak 0.05 ms after first contact, and its peak (25–28 N) is set by the strings' impedance, 2·Z·v. The graded felt's second term (`core_p` 7, `core_hardness` × Stulov's unscaled Q0) dominates from 0.45 mm, so `hardness` and `hardness_slope_bass` do not change the force; `core_depth_bass` 1 mm makes it 0.09 ms.
- **Between the partials**: energy at 300 Hz–5 kHz off the partials, 0.08–0.6 s, re the energy on them: A1 −18 dB in the model, −7.6 in the recording; D2 −19 and −18.
- **Inharmonicity**: below Liang's (A0 2.2e-4 against 3.6e-4, A1 8e-5 against 1.5e-4), within the eight grands' register median.
- **The unisons**: a detune in cents makes every partial's beat grow as k; their nulls sweep down as 1/t (A1: 2.7 kHz at 0.3 s, 1.5 kHz at 0.55 s), absent with no detune and in the recording. A spread of B between strings only multiplies the arcs; an irregular ±0.5–1 cent per string and partial, wound strings only, removes them (bench 1.09, nothing worse). Not kept: the user heard no change in the tube.
- **High bands**: 1–4 kHz stand 4–6 dB higher at 1–1.5 s re the attack (D1–A2 mean).

Open: the hammer's force rise in the bass (felt compliance), the attack's dispersive pulse train, what fills the recording between the partials.

## The chirps the board should hide

Status as of 2026-10-28.

A 3 ms-window spectrogram of the first 120 ms of A1 and D2 (velocity 96, close microphones, studio) against Liang's isolated notes: the model's attack holds a regular train of **slanted chirps**, one per string period, each sweeping from 7 kHz down to 2 kHz over most of the period. The recording holds steady horizontal lines from the first millisecond, no slant. Five more real grands' A1 at forte (Plath's concert grand in an anechoic chamber, BiVib's Disklavier, Headroom's C3, Splendid's Steinway, Helgesson's CF-3) show no slanted chirps either. A "spring" (dispersive allpass) echo is what the user picked as the effect that brings the recording closest to the model: the chirp train is the tube.

The chirps are the stiff string's dispersion: a sharp pulse from the hammer passes the bridge once per period, its high partials ahead of its low ones (f′(k) is 85 Hz at 4 kHz and 107 Hz at 6 kHz for A1 with B = 0.8·10⁻⁴: 2.6 ms more spread per period). Two questions: do real strings chirp, and if they do, why is it not heard?

**Real strings chirp.** Plath's data holds the bridge acceleration at the hitch pins. For A1 it shows the same slanted chirps, one per period, over 0.5–4 kHz, for the whole first 120 ms. His binaural recording at the player's head, in the same anechoic chamber, shows none: steady lines at 1.5–4.5 kHz and unslanted pulses below 2 kHz. The coherence between the bridge acceleration and the ear's pressure is 0.33–0.37 at 1–4 kHz. Between the bridge and the ear, the real board and its radiation smear the chirp into a continuum; the model's do not.

Two differences with the real pianos:
- **The model's chirps reach 7 kHz, the real string's stop near 4 kHz.** The model's hammer force rises to its peak in 0.2 ms whatever the felt (`hardness` 0.1, `core_hardness` 0.5, `hysteresis` 1, `contact_width_scale` 3: the peak is 2·Z·v and the felt reaches it within 0.6 mm). The real force's spectrum must fall faster above 3 kHz. Not resolved.
- **The model's board rings too briefly at 2–6 kHz.** η(f) = 0.02·(1 + f/2500) gives 0.044 at 3 kHz and 0.06 at 5 kHz (T60 17 and 7 ms, shorter than the period). Real boards measure 2–3 % roughly constant ([brightness.md](brightness.md)); above the coincidence frequency (≈ 0.9 kHz) the radiation damping 415/(2π·f·ρh) adds only 0.003 at 3 kHz. With η flat the chirps blur into lines and texture: 0.02 noticeably (A1 and D2), 0.015 more, 0.01 almost like the recordings. The room (studio or none) and the microphones (close or concert) change nothing; `loss_corner` 10 000 too little.

What else was tried on A1/D2, by spectrogram: a 1.75× bass inharmonicity (B 1.25·10⁻⁴, Liang's A1 has 1.4·10⁻⁴; the eight grands' bass medians span 0.65–1.25·10⁻⁴, the model's 0.89) merges the chirps after three periods instead of eight; `losses.winding` 4·10⁻⁴ fades them earlier; a softer core adds little; neither removes the first periods.

Attack levels (0–60 ms, third octaves re the note's own 200–800 Hz) put the model's 4–5 kHz inside the six real A1s' spread and its 6.3–8 kHz on their bright side; its 1.26–2 kHz sit 5–10 dB under most of them.

Bench (eight grands): flat η 0.02 scores 1.26 (36 worse, 9 better), flat 0.01 1.35, against 1.09. The bass register holds (0.76 → 0.78 / 0.79); the middle, treble and top attacks' 4–8 kHz and the top's balance get too bright: the rising η was doing the work of some other missing loss above 2 kHz in those registers. A/B in out/ab_tube (Beethoven 22–32 s with the pedal, plus A1 and D2 isolated in out/ab_tube/notes): the recording, now, η 0.02 flat, η 0.01 flat. If the ear confirms, the next step is to keep the flat damping and find what the treble's high bands need instead.

Scripts in the shared notes folder (tube_scripts2/): `atk.py` (attack close-up), `attk3.py` (attack third octaves), `hf.py` (comb fit, early decays), `plath.py` (bridge acceleration against the ear), `notes_ab.py`.

## Heard: not the board; the percussion is Liang's

The user heard no difference between the three board dampings: the visible blur of the chirps is not what the ear calls the tube. What the user heard instead on the isolated notes: A1 less percussive in the recording than in the model, D2 more. Attack (0–30 ms) over sustain (0.3–0.6 s), broadband:

| | Liang | model | Plath | BiVib | Headroom |
|---|---|---|---|---|---|
| A1 | 2.8 dB | 7.8 | 8.9 | 6.4 | 9.0 |
| D2 (D♯2 for the sets) | 11.1 | 7.8 | 7.2 | 5.5 | 9.5 |

Liang's Disklavier is the one with the inversion: its A1's second partial (110 Hz) builds up for 100 ms to −3 dB and stays, a high-Q low resonance of that board; its D2's partials 1–3 fall 10–15 dB over 400 ms (Plath's D♯2 7 dB, the model's 4). On the other grands the two keys are about equally percussive, the model in their middle. Not followed (the user chose to stay with the real pianos' mean).

Two side findings:
- **D2's partial 4 (294 Hz) is 20 dB down at the close microphones only**: −20 dB at the bridge force, −16 with the player's microphones, −20 with the concert pair, −28/−30 dB on each close channel and −39 on their sum. The close pair sits at mirror-image points about the rectangle's centre ((0.29, 0.35) and (0.71, 0.65) of the board), so every antisymmetric board mode reaches the two microphones in antiphase (the stereo image's −0.7 correlation at 160 and 315 Hz, above). A real board has no such symmetry.
- **A softer bass felt is not it.** A top felt 3 × softer at A1 with the core pushed 0.74 mm deep (`hardness_slope_bass` 0.15, `core_depth_bass` 2) slows the force's rise to 0.45 ms and takes 25–35 dB off the attack above 2 kHz; the recordings have 10 dB *more* there than the model (bench 1.44–1.47). The model's bass felt is stiffer than the published laws at 1 mm (≈ 440 N against Chaigne & Askenfelt's 50 N for C2 and Giordano's 10–30 N), yet the force's rise stays near 0.2–0.3 ms whatever the felt: the peak is 2·Z·v, reached within 0.6–1 mm.

## The longitudinal modes

What the recordings have at 1–3 kHz in the attack that the model lacks: spectral lines off the stiff-string comb, 30–230 ms after the onset, at −17 to −25 dB re the strongest partial at ff (Plath A1 1438 and 1550 Hz; Headroom A1 1605–1857; BiVib A1 1438 and 2178; Plath, BiVib and Headroom D♯2 2046–2196 and 3081–3236; Liang D2 1774 and 2768). They grow much faster than the hammer speed (BiVib A1's 2178 Hz line: −58, −48, −31 dB at velocities 45, 56, 100; D♯2's 2193 Hz: −59, −51, −23): a nonlinear excitation, the longitudinal modes of the strings, at 2–3 times a first mode of 720–930 Hz (A1) and 1.0–1.1 kHz (D♯2). And they ring: −13 to −55 dB/s, mostly −15 to −40, i.e. a T60 of 1.5–4 s, a loss factor of 3–10·10⁻⁴.

The model had them at −32 to −41 dB with `long_loss` 0.005 (−160 dB/s at 1143 Hz: gone in 0.3 s). Its first longitudinal modes lie at 930 Hz (A1) and 1143 Hz (D2), 10–25 % above the recordings' (its bass strings carry 427–479 N and are lighter). Two changes:
- the longitudinal modes are now driven **while the hammer is on the strings** too (the partials' displacements are read off the contact solver; before, the drive started 2 ms after the contact, with a 2 ms ramp). On its own it changes the output by −16 dB re the signal and no measure (bench 1.09 → 1.09);
- `long_loss` 0.001 (candidate B) or 0.0005 (C). D2's first longitudinal mode then stands at −27 dB at 30 ms (recordings' lines −26 to −33) and decays at −32 / −16 dB/s (recordings −13 to −55); the phantom partials' median stays −26.9 dB (Salamander −28.7, Iowa −26.0; `coupling` 0.6 would put it at −21.9). Bench 1.10 (one measure worse: the bass's mf 4–8 kHz, z +0.3 → +1.1). On the 3 ms spectrogram the lines fill the space between the chirps, as in the recordings.

A/B in out/ab_long (Beethoven 22–32 s with the pedal, and A1/D2 isolated): the recording, now, B, C. **Heard: neither B nor C changes the tube.** Not kept (the code is a patch in the sessions' notes); the tube is still not the chirps' masking either. Next: the close microphones' symmetry.

## The close microphones' pair

Tried, as the user asked, against the D2 hole. Eight bass keys (E1–C3) rendered dry with the close pair moved; each pair judged on the spread of its partials' levels (1–6, at 100 ms) relative to the bridge force, its deepest dip, and the L/R correlation per third octave (`mic_modes` example, tube_scripts2/micpair.py):

| pair (x, y of the board; z m) | spread | deepest dip | L/R at 63–125 Hz | 160–250 | 315 | bench |
|---|---|---|---|---|---|---|
| now: (0.29, 0.35) (0.71, 0.65); 0.4 | 6.7 dB | −15.1 (D2 p4) | 0.0 / +0.4 / +0.1 / 0.0 | −0.2 / +0.5 / −0.4 | **−0.88** | 1.09 |
| bass mic at (0.78, 0.56) | 5.8 | −13.9 (A1 p1) | 0.0 / +0.4 / +0.1 / +0.1 | −0.1 / +0.6 / −0.2 | −0.61 | 1.16 |
| (0.30, 0.35) (0.62, 0.75) | 5.9 | −13.2 (A1 p1) | +0.1 / +0.3 / +0.1 / 0.0 | −0.2 / +0.5 / −0.4 | −0.63 | 1.11 |
| P7: (0.33, 0.33) (0.60, 0.52) | 6.3 | −16.7 (A2 p3) | +0.4 / +0.5 / +0.7 / +0.6 | 0.0 / +0.6 / +0.2 | −0.29 | 1.22 |
| P10: (0.33, 0.40) (0.50, 0.60); 0.5 | 6.4 | −15.9 (A1 p1) | +0.2 / +0.8 / +0.6 / +0.5 | 0.0 / +0.5 / +0.4 | −0.30 | 1.16 |
| now, raised to 0.7 m | 7.6 | −22.2 | −0.8 / −0.6 / −0.5 / −0.4 | −0.5 / +0.2 / −0.5 | −0.75 | |

BiVib's and Plath's binaural pairs on the same keys: +0.4 to +0.95 in every band. The modes between 260 and 335 Hz are (3,2), (2,3), (4,1) and (1,4), one every 20 Hz with a 9 Hz bandwidth: each dominates its band alone, and reaches any two microphones 0.5–0.8 m apart in opposite lobes. Moving the pair moves the dips (D2's partial 4 −37 → −27 dB on the mono sum, A2's partial 3 then −17) and repairs the low bands' correlation, not 315 Hz; the pair's place also sets the registers' balance (the treble and top worsen: bench 1.11–1.22). The real lever is the board's regularity (ribs, outline, the modes' density near 300 Hz), the open "board colouring" item. A/B in out/ab_mics (Beethoven and A1/D2: now, P7, P10) for the ear; nothing changed.

**Heard: P10 helps.** On A1 and on the Beethoven passage, the user hears P10 "really better", drier than the recording but correcting part of the tube. What P10 changes is the attack's arrival at the two microphones: with the pair 0.8 m apart, a bass attack reaches them 3.7 ms apart with no correlation (200 Hz–4 kHz, first 40 ms), and the two channels sum into a comb; the real pairs are within 0.2–1.7 ms and correlated +0.2 to +0.5 (BiVib, Plath, Headroom/Splendid/Helgesson). The stereo flam suspected on 2026-10-27 was right, and is heard even on one channel through the room and the board's antiphase. A grid of pairs on eight bass keys:

| pair | attack lag | attack corr | L/R 63–125 Hz | bench |
|---|---|---|---|---|
| now, 0.4 m high | 3.7 ms | −0.02 | 0.0 / +0.4 / +0.1 / 0.0 | 1.09 |
| now, 0.5 m | 3.3 | −0.26 | −0.2 / +0.3 / +0.1 / −0.1 | |
| P10: (0.33, 0.40) (0.50, 0.60); 0.5 | 1.4 | +0.37 | +0.2 / +0.8 / +0.6 / +0.5 | 1.16 |
| P10 at 0.4 m | 1.5 | +0.14 | +0.1 / +0.8 / +0.6 / +0.5 | |
| P13: treble as now, bass (0.46, 0.60); 0.5 | 1.5 | +0.43 | +0.4 / +0.6 / +0.5 / +0.3 | 1.19 |
| P20: treble as now, bass (0.44, 0.62); 0.5 | 1.4 | +0.44 | +0.6 / +0.9 / +0.6 / +0.4 | 1.18 |
| P20 at 0.6 m | 1.7 | +0.26 | +0.1 / +0.5 / +0.5 / +0.6 | |

The closer pairs cost the bench 0.07–0.10, mostly the middle's key-to-key evenness of partials 1–2 (z +0.7 → +2.1–2.7: the bass microphone now hangs over the middle of the main bridge) and the top's 2 kHz bands. A/B in out/ab_mics2 (now, P10, P13, P20). **Heard: with the studio on every file, the current pair's timbre is the closest to the recording.** The first set's isolated notes with P7 and P10 had been rendered without the room while "now" had the studio: what the user liked ("drier") may have been the missing room, not the pair. A pair × room square in out/ab_square (A1, D2: now and P10, each with and without the studio) separates the two. The user then noted the studio is barely audible in these files, so both verdicts stand: P10 takes part of the tube away *and* moves the timbre away from the recording.

No gradual compromise exists: with the treble microphone kept, the bass one moved from (0.71, 0.65) to (0.64, 0.64), (0.58, 0.63), (0.52, 0.62) at 0.4 or 0.5 m leaves the attack's L/R correlation at −0.1 to −0.5 (lag 2–3 ms); only within ≈ 0.45 m of the other microphone ((0.44–0.50, 0.60–0.62) at 0.5 m) does it turn positive (+0.4). The real pairs' coherence at 0.5–0.8 m spacing says the recorded bass attack has a localised, direct component (the strings' and hammer's region, path differences of 0.2–1.7 ms) that the model's attack, radiated by the whole board's modes, has not. That is the lead behind the microphones: not where they stand, but what reaches them first.

## The attack's coherence, band by band: the outline

Where the recorded bass attack is coherent between the two channels (first 40 ms, peak of the normalised cross-correlation within ±6 ms, six bass keys): BiVib +0.25 at 250–500 Hz, +0.20 at 2–4 kHz; Plath +0.33 and +0.37; Headroom +0.38, +0.15, +0.11, +0.45 (250 Hz–4 kHz); Helgesson's coincident pair +0.9 everywhere. The model's rectangle with the current pair: **−0.53 at 250–500 Hz** (3.3 ms), −0.04, −0.11, +0.14, +0.48 at 4–8 kHz. The "flam" is not a missing direct source in the treble: it is the board's 260–335 Hz modes reaching the two microphones in antiphase during the attack, which the eight-key L/R correlation showed in the sustain too.

The board's own shape does what the microphones could not. With the grand outline ([outline.md](outline.md), `soundboard.outline`, off by default) and the same pair, the attack's coherence turns positive in every band: +0.21 / +0.17 / +0.23 / +0.30 / +0.15 (250 Hz–8 kHz; broadband lag 3.7 → 2.9 ms, correlation −0.02 → +0.20). The sustain's correlation at 315 Hz stays −0.84 and the transfer dips stay (spread 6.4 dB). Bench: outline alone 1.15 (22 worse, 16 better: the middle's 1 kHz bands and centroid, the treble's partial 8), outline with the board recalibrated as the earlier session had it (ρh 8.4, D 4590/2040, impedance slopes 0.0024/0.0076) 1.11 (17 worse, 19 better). A/B in out/ab_outline2 (Beethoven and A1/D2 with the studio throughout: rectangle, outline, outline recalibrated). On 2026-10-27 the user had heard the outline as "a bit more even" and the tube "maybe B/C, not flagrant", before the lowest modes were damped (`loss_low_corner` 200).

## Still open

- **C2–A2 lacks body.** Relative to the strongest partial, partials 4–6 lie 10–13 dB under the recording in the sustain (−1 to −5 at the attack): they decay at −14 dB/s against −9. The edge loss is not it (off: −12). The bench's eight grands agree: the bass's partials 4–12 are 4–8 dB weak in the sustain. The bridge's conductance at 300–800 Hz is next.
- Partials above the 20th of C2–A2 stand 5–20 dB over the recording.
