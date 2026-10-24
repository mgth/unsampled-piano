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

## G1 — The string, complete

Everything that follows needs this foundation.

- **Two polarisations**: perpendicular to the top and parallel to it. Each
  has its own support admittance, and the saddle couples them slightly. The
  pluck angle shares the energy between them, which gives the two-stage
  decay and the beats.
- **Per-sample contact solver in modal space.** The bank runs block-wise
  and freely, but contacts (finger, plectrum, thumb, frets) need forces
  computed every sample, at a few points of the string.
  - While a contact is active, the string is advanced sample by sample.
    Its displacement at the contact points comes from the mode shapes, and
    the contact forces are projected back onto the modes.
  - This is the architecture of the piano's `ContactSolver` (a few physical
    oscillators handed back to the bank), generalised to several points and
    to unilateral contacts. It is the main candidate for a shared module.
  - Order of cost: 250 modes × 20 points × 12 strings × 48 kHz ≈ 3 GMAC/s
    in the worst case (every fret in contact on every string). That is
    affordable, and the solver only runs while something touches the
    string.
- **Change of length while vibrating** (new fret, finger lifting): project
  the modal state (displacement and velocity) of length L₁ onto the modes
  of L₂. This is exact for a linear string and cheap.
- **Tension changes live**: f₀, B and the losses re-evaluated while
  playing (needed for bends, G4).

## G2 — Right-hand excitations

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

## G3 — Frets and collisions

- **String–fret contact** as unilateral constraints at each fret crown, in
  the G1 solver. Ducceschi, Bilbao, Issanchou and Touzé have published
  energy-conserving modal schemes for strings against obstacles.
  Issanchou et al. validated theirs on a real bass string.
- This gives fret buzz, the slap and pop rattle, and the "clank" of hard
  playing, all from the same physics.
- Geometry: neck relief, action height, fret heights, saddle height. A
  low-action electric bass buzzes, a classical guitar hardly.
- Fretless basses: the finger is the boundary (a soft, lossy stop), and no
  fret means no buzz. This is the source of the fretless "mwah".

## G4 — Bends, vibrato, tremolo arm, nonlinearity

- **Bend**: the finger pushes the string sideways along the fret, which
  raises the tension. Tension, then f₀, B and the losses follow live (G1).
  The bend amount comes from MIDI (see G9).
- **Vibrato**: a periodic bend, or a lengthwise rocking of the finger
  (classical style, a smaller pitch change).
- **Tremolo arm**: the tension of every string changes together. The
  springs make it detune unevenly per string; this is part of the model,
  not a bug.
- **Tension modulation** (geometric nonlinearity): pitch glides down after
  a hard attack, and longitudinal modes appear (the "phantom" partials,
  important on basses and plain strings). The piano's nonlinearity code is
  the model to share.

## G5 — Electric outputs

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
