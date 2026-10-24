# Unsampled Guitar: physics plan

Same rules as the piano: no samples to make the sound (recordings only to
analyse and calibrate), physics before mimicry, and anything mimetic is said
to be so. The guitar lives in this workspace and reuses the piano's crates.
`guitar-core` starts by depending on piano-core's instrument-agnostic
modules. A module moves to a shared crate only once both instruments use it
and the piano work is at a quiet point.

## Scope

- **Instruments**: every plucked, fretted string instrument of the guitar
  family. Acoustic guitars: steel-string folk, classical/flamenco nylon,
  archtop, 12-string. Electric guitars: solid and semi-hollow bodies.
  Basses: 4, 5 and 6 strings, fretted and fretless, electric and acoustic.
- **Playing techniques**: finger, nail and plectrum plucks, thumb slap and
  pop, tapping, hammer-on and pull-off, slides, bends, vibrato, tremolo
  arm, palm mute, natural, artificial and pinch harmonics, fret buzz, left-
  and right-hand mutes.
- **Outputs**: the raw instrument signal, with its own volume and tone
  controls, from
  - magnetic pickups: single coil, humbucker, at their positions with the
    selector, through the guitar's passive circuit (volume and tone pots,
    cable);
  - piezo under the saddle;
  - microphone in front of an acoustic instrument, in a room (shared with
    the piano's rooms and mics).
- **Out of scope**: amplifiers, cabinets and effects. The user plugs the
  raw signal into their own chain.

Where the character lies differs from the piano. A guitar string is light,
so it sees its supports strongly: **the body and the frets set the decay of
every partial**. The two polarisations decay at very different rates, and
the excitation (flesh, nail, plectrum, thumb, fret contact) shapes the
attack more than a hammer does. The cost is small: at most 12 strings of
~250 modes each, against the piano's hundreds of thousands. Accuracy can be
bought freely.

## G0 — Skeleton (done, `crates/guitar-core`)

- Six stiff strings: f_n = n·f₀·√(1 + B·n²), μ from tension and open pitch,
  B from the core diameter. The default set is a light-gauge steel-string
  acoustic, 650 mm scale (catalogue values, not fitted).
- Losses after Valette & Cuesta: air viscosity, viscoelastic bending loss,
  internal friction, and a **constant bridge loss** standing in for the
  supports.
- Ideal pluck: a triangle released from rest at a fixed distance from the
  bridge, its corner rounded by the contact width. Velocity scales the
  displacement linearly (placeholder).
- Output: the transverse force on a rigid bridge. That is roughly what a
  piezo under the saddle sees, without its electronics.
- MIDI: a note goes to the lowest fret on a free string, one note per
  string. Note-off lays a dashpot near the fret (the mute).
- `cargo run --release -p guitar-core --example pluck` writes
  `out/guitar_pluck.wav`.

Reused from piano-core: `modal::ModalBank`/`ModeTable`/`ModeSpec` (complex
one-pole modes, SIMD, damper weights), `denormal`.

## G1 — The string, complete (done)

Everything that follows needs this foundation.

- **Two polarisations** (`string.rs`): perpendicular to the top and
  parallel to it.
  - The bridge is a 2×2 admittance Y (`support`: perpendicular, parallel,
    cross). An end of admittance Y shifts each partial's complex frequency
    by i·Z₀·Y·ω_n/(nπ) (Gough's weak coupling). The 2×2 eigenproblem per
    partial gives two modes, each mixing both directions. The modal bank
    runs them, with the perpendicular and the parallel bridge force as its
    two outputs.
  - Passive by construction: if Re Y is positive semi-definite, no mode
    grows.
  - The cross term trades energy between the directions, which gives the
    two-stage decay and the beats. The pluck angle shares the energy.
  - A resistive bridge gives a decay rate independent of the partial
    (2f₀·Z₀·G). The rising conductance (`conductance_per_khz`) stands in
    for the density of body modes until G5/G6 give Y(ω). It replaces G0's
    constant loss factor.
- **Per-sample contact solver** (`contact.rs`): the piano hammer's scheme
  (exact two-step recursion per mode, quadratised contact laws, Ducceschi
  et al.), generalised to up to 8 points in either polarisation.
  - Bilateral spring–dashpot (fingertip) and unilateral power law (fret,
    fingerboard, nail), with fixed or ramped targets and timed removal.
  - Each sample solves one small linear system (I + D·Ĝ)·λ, with an active
    set for unilateral contacts. It is passive with fixed targets (tested
    against a stiff obstacle).
  - The bank's coupled state goes in and comes back through E and E⁻¹.
    During contacts the cross term is left out (milliseconds against
    tenths of a second), as the piano does with its unison coupling.
  - The static residual of the truncated modes is not added yet. A finger
    sees the string ≈ 2 % too stiff, which the piano corrects for its
    hammer and G3's stiff frets may need.
- **Finger pluck** (`PluckModel::Finger`, the default): a fingertip lands on
  the string (stopping what it does), drags it over `drag_time` and lets go
  at once. A slow drag and release gives the ideal triangle (tested). The
  instant release stays until G2 brings slip and friction.
- **Change of length while vibrating** (`project.rs`): displacement and
  velocity projected from the sine basis of L₁ onto that of L₂. Closed
  form: N + M sines and N·M divisions per projection.
  - Used when a held string gets a new fret, and at note-off, where the
    fretting finger lifts. The string returns to its open length, damped by
    the fingertip spread over its width just behind the old fret. Lifting
    near a node lets the harmonics there ring a little, as on a real
    guitar.
- Cost: all of this runs about 800× faster than real time on one core
  (13 s example, instrument build included, in 16 ms).
- Moved to G4: tension re-evaluated live (bends).

## G2 — Right-hand excitations

**Basics done** (`PluckModel::Finger`, `PluckModel::Plectrum`):
- The finger or plectrum is a spring–dashpot dragging the string in the
  pluck's direction. It slips off once the force on the string reaches the
  pluck force (∝ velocity, placeholder law); its grip then fades out over
  the slip time.
- The slip time is what separates them: flesh ≈ 0.5 ms, plectrum
  ≈ 0.05 ms, so the plectrum comes out brighter (tested). The released
  displacement is F/k with k = T·L/(x₀(L − x₀)) (tested).
- Stiffnesses and slip times are orders of magnitude, to be measured.
  Friction and the tip's geometry (nail, plectrum edge) remain to do.
- Slap, pop, tap/hammer-on and pull-off: see G3 (they need the frets).
  Harmonics and palm mute remain.


All of them use the G1 contact solver with different contact laws.

- **Finger, nail, plectrum**: a mass–spring–dashpot that drags the string
  and then slips off (Cuzzucoli & Lombardo; Evangelista & Eckerholm;
  Chadefaux's measurements of real plucks). Parameters: stiffness and
  friction of the material, release angle (which feeds the polarisations),
  rest stroke vs free stroke, pluck position (live CC), plectrum thickness.
- **Thumb slap**: an impact of the thumb near the end of the neck, hard
  enough to throw the string onto the frets (G3 makes the slap's rattle).
- **Pop**: the string pulled away from the body and released, so it hits
  the frets on its way back.
- **Tapping, and hammer-on/pull-off**: a fingertip strikes the string onto
  a fret, which creates a new boundary (G1 projection). The impact itself
  excites the string on both sides of the fret. A pull-off is a pluck by
  the left-hand finger as it leaves.
- **Harmonics**: natural (a light touch at L/k, a point damper on the nodes
  of the other modes), artificial (the same touch made by the right hand),
  pinch (a plectrum pluck with the thumb grazing a node).
- **Mutes**: palm mute (a distributed damper near the saddle), left-hand
  mute (the G0 dashpot, refined).

## G3 — Frets and collisions (done, except fretless)

- **Geometry** (`frets.rs`, `FretboardParams`):
  - fret heights, neck relief (a parabola between the nut and the body
    fret), nut clearance;
  - the action at the 12th fret, interpolated from the bass to the treble
    string, which sets the saddle height;
  - each stopped string runs straight from the saddle to its stop, and
    every fret under it gets its gap (tested: the action is the gap at the
    12th fret).
- **Contacts**: each fret within reach is a unilateral contact K·u^1.5 in
  the G1 solver (energy-quadratised, passive). This gives buzz, the slap
  and pop rattle, and the hammer-on's landing.
- **Cost kept low**, while staying rigorous about not missing a contact:
  - a reach bound Σ|sin(nπx/L)|·|ζ_n| picks the frets to watch;
  - those are checked every 8 samples in the bank: actual displacement
    plus the velocity bound times the interval, against the gap;
  - the solver takes over only when a fret can really be touched (with a
    margin while a hand drives the string), and hands back after 10 ms
    without a contact force;
  - soft plucks never enter the solver (tested). The six demo renders
    (≈ 57 s of audio) take 0.9 s on one core.
- **Rest-line change**: when the stop changes suddenly (hammer-on landing,
  pull-off, finger lifting), the string starts displaced from its new rest
  line, by a ramp or a triangle projected analytically. This is the small
  pluck of a string leaving or landing on a fret. Without it, a hammer-on
  overshot by 2.5× (tested).
- **Techniques** (`Technique`, `Event::Note`), in the solver:
  - slap: the thumb as a unilateral contact on a strike path toward the
    board;
  - pop: a finger pulling away from the board;
  - tap and hammer-on: a finger strikes the string just behind the fret,
    and once string, fret and finger have touched for 1 ms the vibrating
    length becomes the fret's;
  - pull-off: the length grows, and the leaving finger plucks sideways.

  Tested: slap and pop hit the frets and are brighter than a pluck, a
  tapped note sounds at its pitch, legato stays on its string, and a hard
  pluck on low action buzzes.
- **Remaining**:
  - fretless basses (the finger as a soft, lossy stop);
  - friction along the fret;
  - the fingerboard between frets (very deep bends or very low action);
  - the residual of truncated modes, for sharper contacts;
  - contact parameters and technique speeds measured on recordings.

## G4 — Bends, vibrato, tremolo arm, nonlinearity (done, except phantom partials)

- **Tension, live** (`TensionParams`, `StringTable::bent`):
  - each partial at tension ratio ρ = T/T₀ gets its frequency (stiffness
    falls as 1/T), the string's own losses at the new frequency, and the
    bridge shifts with the new Z₀ and Y(f);
  - the mode shapes and the coupled modes' eigenvectors are kept (first
    order, λ_k = (E⁻¹·M·E)_kk), so the state carries over;
  - every 64 samples, a string whose tension moved by more than 2·10⁻⁵
    (≈ 0.02 cent) is retuned in place: `set_modes` in the bank, `retune` in
    the contact solver (each mode's previous sample recomputed for its new
    pole);
  - modes pushed past 0.48·fs are silenced, not aliased.
- **Bend**: `Event::Bend { note, semitones }`, for one note or the held
  strings (wheel). The tension that brings the fundamental to pitch is
  exact for a stiff string: ρ = r²(1 + B₀) − B₀ (tested to 0.01 cent). The
  control is followed with a 10 ms time constant. A new note starts
  unbent.
- **Vibrato**: `Event::Vibrato { depth }`, a periodic bend at
  `vibrato_rate` on the held strings (tested both ways).
- **Tremolo arm**: `Event::Tremolo { position }`. The bridge moves by up
  to `tremolo_travel`, and each string's tension changes by EA·δ/L_total,
  with L_total from the saddle's anchor to the tuner (friction-free nut and
  saddle). The strings detune unevenly: 0.8 to 2.7 semitones at half
  travel on the electric (tested).
- **Tension modulation** (Kirchhoff–Carrier, averaged):
  ΔT = EA·L·Σk²|ζ|²/(8·L_total), from both polarisations. Hard plucks
  start sharp and glide down: about 5–6 cents on the guitars and 14 cents
  on the bass at velocity 127, fading over seconds (tested).
- **Remaining**:
  - the fast part of the modulation (longitudinal modes and phantom
    partials, from the piano's nonlinearity code);
  - friction at the nut and saddle (bends that do not return exactly);
  - the arm's springs (bridge balance, pulling up past its rest);
  - bends set by finger displacement rather than pitch;
  - MIDI mapping (G9).

## G5 — Electric outputs

**First part done** (`Output`, `output.rs`, presets `electric_solid` and
`electric_bass`):
- Magnetic pickups give an EMF proportional to the string's velocity toward
  the poles, over each coil's window (aperture sinc).
  - A humbucker is its two coils in series.
  - Pickups switched in parallel average (taken as identical).
  - The bank outputs this EMF directly: velocity weights (−σ + iω) per
    mode, and the same from the contact solver's state.
- The passive circuit is solved exactly as a 1st-over-4th-order rational
  transfer, discretised by the bilinear transform prewarped at the coil
  resonance. It covers the coil (L, R, C), volume and tone pots (audio
  taper), tone capacitor, cable and input. Tests check the resonance (LC),
  humbucker lower than single coil, the tone and volume knobs, and the
  peak kept after discretisation.
- Piezo: the perpendicular bridge force (V/N) through a first-order
  high-pass.
- Presets: solid-body electric (3 single coils, plectrum) and 4-string bass
  (2 single coils, fingers), with small solid-body admittances (long
  sustain, tested).
- `output_gain` maps volts to digital full scale.

Remaining in G5:
- the flux nonlinearity;
- different pickups in parallel (exact Thévenin) and per-pickup volumes
  (Jazz bass);
- parallel-polarisation pickup sensitivity;
- live knobs (they are fixed at build time today);
- active bass preamps;
- the neck modes (dead spots).


The magnetic pickup reads the string almost directly, so this is also the
best test bench for G1–G4.

- **Magnetic pickup**: the coil sees the velocity of the string over the
  magnet's window, mostly in the polarisation toward the pole pieces.
  - The flux is nonlinear in displacement (Horton & Moore), which adds
    harmonics on hard playing.
  - Position along the string: a comb on the modes (neck pickup round,
    bridge pickup thin).
  - The window width rolls off the high modes.
- **Single coil vs humbucker**: one window, or two windows a few cm apart
  in reverse phase, with higher inductance. Selector positions (neck,
  middle, bridge, in-between) as sums of windows, including the out-of-
  phase "quack".
- **Passive circuit**, solved as a linear network: coil (L ≈ 2–5 H,
  R ≈ 6–15 kΩ, its capacitance), volume pot (250/500 kΩ), tone pot and
  capacitor (22–47 nF), cable capacitance and the input impedance it feeds.
  The coil resonance (2–5 kHz) is what makes a single coil and a humbucker
  sound different. The user's two controls are these pots. An active
  preamp with EQ (common on basses) comes later.
- **Piezo under the saddle**: the saddle's force (mostly the perpendicular
  polarisation) through the piezo element and its preamp. This is G0's
  output, plus the element's electrical model.
- **Solid-body supports**: a heavy, stiff body with neck modes. The decay
  is long and set by the neck and the bridge. The neck modes cause the
  known "dead spots" where a note dies faster.

## G6 — Acoustic bodies and microphone

- **Hollow body**: the Helmholtz air mode (≈ 100 Hz) and the top's T(1,1)
  (≈ 200 Hz) coupled through the cavity, as in the two- and three-
  oscillator models (Christensen & Vistisen; Caldersmith), plus higher top
  and back modes. Their density and damping come from published bridge
  mobilities (Woodhouse, Elie, Gautier) and later from our own fits.
- **Coupling**: as the piano's coupled modes (`note_model`). The string
  modes plus the body admittance Y(ω) are solved for the coupled
  eigenmodes. The decay of each partial follows Re Y, its frequency shift
  follows Im Y, and a partial meeting a body mode gives a wolf. This
  replaces `eta_bridge`.
- **Variants**: steel folk (X bracing), classical (fan bracing, nylon),
  archtop and semi-hollow (floating bridge, f-holes), acoustic bass.
- **Radiation**: each body mode as a monopole and a dipole, plus the sound
  hole's volume velocity. The microphones sit where the listening presets
  put them.
- **Shared work**: `room` must take a source point and a directivity
  instead of the piano's `SoundboardParams`. Then it becomes common, and
  the guitar gets rooms and mics for free.

## G7 — Sympathetic strings, courses

- Free strings driven by the bridge motion, one-way as in the piano's
  sympathetic bank. It matters on the guitar: open strings ring under
  fretted notes.
- 12-string: octave and unison pairs plucked together, slightly out of
  step. This reuses the piano's unison machinery.

## G8 — Instrument range (presets)

Families, each with its strings, scale, body and outputs: steel folk
(dreadnought, parlour), classical, flamenco, 12-string, archtop jazz,
semi-hollow, solid body (single coils, humbuckers), electric bass
(4/5/6-string, fretted/fretless, passive/active), acoustic bass. Long scale
(864 mm, 34") and multiscale are part of the geometry.

## G9 — MIDI

A bend on one string out of a chord is the hard case. Plain MIDI has one
pitch bend per channel, so the problem is the channel layout. Three modes:

- **One string per channel** (channels 1–6, or an MPE zone). Each string
  has its own pitch bend, pressure and CCs, so a bend applies to that
  string only. This is what guitar-to-MIDI systems send (Fishman
  TriplePlay, Roland GK in mono mode, Jamstik) and what MPE controllers
  send. Reaper and Bitwig handle it.
- **Per-note expressions**: CLAP note expressions (tuning, vibrato,
  pressure, brightness) and MIDI 2.0 per-note pitch bend reach one note
  even on a single channel. Use them when the host offers them.
- **Keyboard mode** (one channel, the default): automatic string choice
  (G0).
  - The pitch bend goes only to notes whose key went down after the last
    bend reset, or only to the most recent note (option). A bend on top of
    a held chord then works, as in commercial guitar libraries (mimetic
    control logic; the bend itself stays physical).
  - Articulations by keyswitches (notes outside the range) and CCs.
  - Automatic legato: a new note on a string still held becomes a
    hammer-on (up) or pull-off (down).
  - Slides by CC or portamento, palm mute and pluck position by CC,
    velocity → pluck force, release velocity → mute style.

## G10 — Calibration and product

- Tools: `piano-analysis` (partials, inharmonicity, decays, ESPRIT) and
  `piano-eval` are instrument-agnostic and apply as they are.
- References to find:
  - GuitarSet (hexaphonic pickup, one channel per string): check its
    licence first.
  - DI recordings of electric guitars and basses, which show the string
    and pickup without a body.
  - Home recordings, note by note and per string, are the best reference.
- First fits: B per string from the partials, then σ_n per string and fret,
  which measures the support admittance indirectly.
- Product:
  - `piano-params`, `piano-control`, `piano-ui` and `piano-mcp` become
    generic over an `Instrument` trait (model type, engine type, live
    parameters). The registry already reads the model's serde form, so
    `GuitarModel` gets paths for free.
  - Its own plugin IDs. The piano's IDs never change.

## Order

1. **G1**: every technique depends on the two polarisations, the contact
   solver and the length projection.
2. **G5 with G2 basics**: an electric guitar and an electric bass through
   magnetic pickups. The pickup sees the string with no body in between,
   so it is the cleanest way to validate the string and the excitations
   against DI recordings. It also gives a usable instrument early.
3. **G3, then G4**: frets and collisions (slap, buzz, tapping), then bends
   and the tremolo arm.
4. **G6**: acoustic bodies and the microphone, the hardest part to get
   right, built on strings already validated.
5. **G7**, **G8** and **G9** alongside, with G10 running from step 2 on.
