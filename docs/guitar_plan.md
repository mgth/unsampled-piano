# Unsampled Guitar: physics plan

Same rules as the piano: no samples to make the sound (recordings only to
analyse and calibrate), physics before mimicry, and anything mimetic is said
to be so. The guitar lives in this workspace and reuses the piano's crates.
`guitar-core` starts by depending on piano-core's instrument-agnostic
modules. A module moves to a shared crate only once both instruments use it
and the piano work is at a quiet point.

Where the character lies is not the same as on the piano. A guitar string
is light, so it sees the bridge strongly: **the body sets the decay of every
partial**, the two polarisations decay at very different rates, and the
excitation (flesh, nail, plectrum) shapes the attack more than a hammer
does. The cost is tiny: six strings of at most ~250 modes each, against the
piano's hundreds of thousands. Accuracy can be bought freely.

## G0 — Skeleton (done, `crates/guitar-core`)

- Six stiff strings: f_n = n·f₀·√(1 + B·n²), μ from tension and open pitch,
  B from the core diameter. The default set is a light-gauge steel-string
  acoustic, 650 mm scale (catalogue values, not fitted).
- Losses after Valette & Cuesta: air viscosity, viscoelastic bending loss,
  internal friction, and a **constant bridge loss** standing in for the
  body.
- Ideal pluck: a triangle released from rest at a fixed distance from the
  bridge, its corner rounded by the contact width. Velocity scales the
  displacement linearly (placeholder).
- Output: the transverse force on a rigid bridge, which is not yet a sound
  pressure.
- MIDI: a note goes to the lowest fret on a free string, one note per string.
  Note-off lays a dashpot near the fret (the mute).
- `cargo run --release -p guitar-core --example pluck` writes
  `out/guitar_pluck.wav`.

Reused from piano-core: `modal::ModalBank`/`ModeTable`/`ModeSpec` (complex
one-pole modes, SIMD, damper weights), `denormal`.

## G1 — Body and bridge admittance (the big one)

- A modal body seen from the bridge: the Helmholtz air mode (≈ 100 Hz) and
  the top's T(1,1) (≈ 200 Hz), coupled through the cavity, as in the
  two- and three-oscillator models (Christensen & Vistisen; Caldersmith),
  plus higher top and back modes. Their density and damping come from
  published bridge mobilities (Woodhouse, Elie, Gautier) and later from our
  own fits.
- String–body coupling exactly as the piano's coupled modes (`note_model`):
  string modes plus body admittance Y(ω), solved for the coupled
  eigenmodes. Decay per partial ∝ Re Y and frequency shift from Im Y,
  including the "wolf" where a partial meets a body mode. This replaces
  `eta_bridge`.
- Radiation: each body mode as a monopole + dipole, plus the sound hole's
  volume velocity (out of phase with the top below the Helmholtz mode). The
  microphones sit where the listening presets put them.
- Shared work: `room` must take a source point and directivity instead of
  the piano's `SoundboardParams`. Then it becomes common and the guitar gets
  rooms and mics for free.

## G2 — Two polarisations

- Perpendicular to the top and parallel to it, with very different bridge
  admittances: the perpendicular one is mobile, so it decays fast; the
  parallel one is stiff, so it rings long. The pluck angle shares the
  energy between them, which gives the guitar's two-stage decay. Small
  coupling through the saddle gives beats.
- Same machinery as the piano's horizontal damper work (two sets of modes,
  each with its own weights).

## G3 — Right-hand excitation

- Replace the ideal release by a contact model in modal space. The finger or
  plectrum is a mass–spring–dashpot that drags the string, then slips off
  (Cuzzucoli & Lombardo; Evangelista & Eckerholm; Chadefaux's measurements
  of real plucks).
- Parameters: flesh vs nail vs plectrum stiffness, release angle (which
  feeds G2), rest stroke vs free stroke, the pluck position live (CC).
- Velocity → force and release speed, calibrated on recordings. The piano's
  `ContactSolver` architecture (contact solved on a few modes, handed back
  to the bank) is the starting point and a candidate for a shared module.

## G4 — Left hand and fretboard

- Change of length while the string vibrates (hammer-on, pull-off, slide):
  project the modal state (displacement, velocity) of length L₁ onto the
  modes of L₂. This is exact for a linear string and cheap.
- Release: the string lifts off the fret and sounds briefly at the open
  length, damped by the finger. Fret and finger noises come from the same
  contact physics.
- Bends and vibrato: tension raised by lateral displacement → f₀, B and μ
  are re-evaluated live, as a pitch modulation of the bank.
- Natural harmonics: light touch at a node is a point damper (the same
  dashpot as the mute, at L/k). Palm mute: a distributed damper near the
  saddle.

## G5 — Nonlinearities

- Tension modulation on hard plucks: pitch glide down at the attack and
  coupling into longitudinal modes. The piano's geometric nonlinearity code
  is the model to share.
- String–fret collision (buzz) for hard plucks and low action. This needs a
  collision solver on a few modes near the frets. Later, and optional.

## G6 — Sympathetic resonance

- Free strings driven by the bridge motion, one-way as in the piano (sym
  bank). It matters on the guitar: open strings ring under fretted notes.

## G7 — Variants

- Classical nylon guitar: lower tension, much higher internal losses,
  different body (fan bracing). This is mostly presets once G1–G3 exist.
- Electric guitar: no body radiation, magnetic pickup = string velocity
  observed at one point, with the pickup's own resonance. It is cheap, but
  amps and effects are out of scope.

## G8 — Calibration and evaluation

- Tools: `piano-analysis` (partials, inharmonicity, decays, ESPRIT) and
  `piano-eval` are instrument-agnostic and apply as they are.
- References to find. GuitarSet (hexaphonic pickup, one channel per string,
  useful to fit decays without the other strings) is the first candidate;
  check its licence before use. A home recording of a real guitar, note by
  note and per string, would be the best reference.
- First fits: B per string from the partials, then σ_n per string and fret,
  which measures the bridge admittance indirectly (G1).

## G9 — Product

- `piano-params`, `piano-control`, `piano-ui`, `piano-mcp` become generic
  over an `Instrument` trait (model type, engine type, live parameters).
  The registry already reads the model's serde form, so `GuitarModel` gets
  paths for free.
- Its own plugin IDs ("Unsampled Guitar"). The piano's IDs never change.
- MIDI: one string per channel (guitar controllers, MPE), per-channel pitch
  bend → bends, CC for pluck position and mute, keyswitches for
  articulations. The automatic string choice of G0 stays for keyboards.

## Order and why

G1 first: without the body, nothing sounds like a guitar and every decay is
wrong. Then G3, then G2, because pluck and polarisation together make the
attack and the two-stage decay. G8 runs alongside from G1 on. G4 brings
playability. G5–G7 come once the open-string sound holds against
recordings.
