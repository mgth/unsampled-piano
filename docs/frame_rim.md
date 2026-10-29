# Cast-iron frame and rim

Status as of 2026-10-22. Code: `structure.rs`, `board.rs` (additional outputs), `engine.rs`, `voice.rs`.

## What these parts do in a real piano

- **The frame** (the gilded cast-iron plate) takes the tension of the strings, about 20 tonnes. Each string bears on the soundboard at the bridge and on the frame at the other end, at the agraffe. So, as it vibrates, it also pushes on the frame.
- **The rim** (the laminated maple case) holds the soundboard around its edge. When the soundboard vibrates, it pulls and pushes on its edges, and the rim receives these forces.

These two parts are heavy and stiff. They move much less than the soundboard and therefore radiate little. Their main role is mechanical: holding the tension and fixing the edge of the soundboard. The model already reflects this, since the soundboard is clamped in it. Their direct contribution to the sound is small.

## Model

### Frame (`FrameParams`)

- **Excitation.** Each string computes the force it exerts on its agraffe, −T·∂y/∂x at that end. The weights are those of the bridge multiplied by (−1)^(k+1): the fundamental pulls on both ends in the same direction, the second partial in opposite directions. During hammer contact, this force also contains the quasi-static part of the hammer's push. Since the hammer strikes close to the agraffe, it is mostly the agraffe that receives it.
- **Plate.** It is a clamped isotropic plate, computed by the same code as the soundboard: Rayleigh radiation towards the microphones, from 8 cm above the soundboard.
  - It spans 1.9 × 1.3 m, from 20 cm in front of the board (over the pin block) to the case's tail, where the hitch pins are.
  - It follows the case (`FrameParams::outline`): the board's outline, carried forward with a straight front edge. A board without an outline gives the rectangle. See [The outline](#the-outline) below.
  - Surface density 110 kg/m², i.e. about 150 kg of cast iron, holes included.
  - Stiffness chosen for a first mode around 155 Hz; 247 modes up to 12 kHz.
  - The frame is about 40 times less mobile than the soundboard.
- **Radiation.** It is a perforated plate. Only the metal fraction radiates (`solidity` 0.4), and its two faces short-circuit each other through the holes (`back_gain` 0.9 at 5 cm).
- **Damping.** η = 2 % (1 % until 2026-10-29, see [Two ways](#two-ways)). Bare cast iron is at 0.1–0.5 %, but once installed it is bolted to the pinblock and the rim, and loaded by 230 strings and their felts. **This is an estimate**, with no published measurement.
- **Agraffes.** They are placed 5 cm from the front edge, above the bolted pinblock. They face the bridge point of each region, since the strings run along x. The bass string crossing is ignored.
- **Two ways** (`frame.coupling`, 1 by default, since 2026-10-29): the frame's driving-point admittance at each key's agraffe is added to the bridge's in the strings' coupled modes, as the board's is. The “listening” strings (sympathetic resonance) do not excite it.

### Rim (`RimParams`)

- **Edge forces.** For each sine-shaped soundboard mode, the model takes the Kirchhoff shear along the edges and the corner forces. Their sum is exactly ρh·ω_ss²·∫Φ; a test checks this to within 0.2 %, for even and odd modes alike. They are then multiplied by (ω_n/ω_ss)², so that the net force matches the clamped frequency used by the soundboard.
  - The transmitted force follows the modal displacement and the damping force: q + 2σ/ω²·q̇.
  - The soundboard bank obtains this term through complex weights on its states, with no extra state.
- **Ring.** The rim is a 30 × 7.5 cm wall of laminated maple (E 12 GPa, G 1 GPa, 650 kg/m³, η 2 %). It is modelled as a ring of the same perimeter, in out-of-plane bending coupled to torsion, following Love:
  - for each order k ≤ 8, a 2 × 2 problem in vertical displacement W and wall rotation B, rotary inertia included;
  - each order gives a “bending” branch, from 43 Hz (k = 2) to 680 Hz (k = 8), and a “torsion” branch, from 660 Hz to 3 kHz;
  - the rigid-body modes (the piano moving on its legs) are discarded.
- **Coupling.** The soundboard pulls on the inner edge of the wall, at t/2 from its axis. It therefore excites both bending and torsion.
  - The soundboard bank provides 17 projections (k, cos/sin) of its edge forces.
  - Only its 93 modes below 2 kHz contribute: shorter modes project little onto the long waves of the rim.
- **Radiation.** Four faces discretised every 4 cm, with an obliquity factor (1 + cos θ)/2.
  - Outer and inner faces: wall rotation.
  - Top and bottom: vertical motion.
  - The inner face, half under the soundboard, is weighted by 0.5.
  - No reflection from the lid.
- **One way.** The rim does not move the edges of the soundboard.

## The outline

Until 2026-10-28 the frame was a 1.75 × 1.3 m rectangle. When the board's outline went on by default that day, the frame plate inherited it by accident: the board's normalised polygon, laid on the frame's own rectangle, put its bentside 15–20 cm in front of the case's, and nobody measured it. The interface still drew a rectangle.

Since then the frame has the case's shape on purpose, as a real plate does, and reaches the tail. The interface's Model page and plan draw that outline.

Its outlined plate has its own modes up to 3 kHz (the board's stop at 1.5 kHz), the rectangle's above. Above the corner, the rectangle's modes cut at the outline radiate from its edge, where their cells no longer cancel. With the board's 1.5 kHz the frame came out louder than the note at C7 (+4 dB) and at D7 (−9 dB, against −26 with its own modes). The iron plate is stiff and has few modes: building them takes 23 ms up to 3 kHz (9 ms at 1.5 kHz, 130 ms at 6 kHz, 1 s at 12 kHz).

Frame level re the note, 0.5–1.5 s, velocity 100, every other key from C4 to C8 (`structure_levels 60 62 … 108`):

| | rectangle (before) | inherited by accident | case, to the tail, own modes to 3 kHz |
|---|---|---|---|
| mean, C4–C8 (power) | −20.9 dB | −16.9 dB | −16.8 dB |
| mean without C7 | about −24 dB | about −25 dB | about −27 dB |
| loudest | −12.0 dB (C7) | −3.8 dB (C7) | −3.2 dB (C7) |

The bench does not move (1.10 → 1.11, no measure by 0.3 z or more).

At C7 one of the frame's modes falls on the fundamental (2093 Hz), and the frame rings as loud as the note in its sustain. The frame is one way: it takes the agraffe's force and gives nothing back, so a mode that coincides with a partial is driven at resonance without draining the string, as a real frame coupled both ways would. Its level and damping are not calibrated either.

## Two ways

At C7 a frame mode rang within 4 dB of the note in its sustain (see [The outline](#the-outline)), and the frame then gave nothing back to the strings. Now the strings' modes take the frame's admittance at their agraffes beside the bridge's (`FrameAdmittance`): a partial loses energy to the frame as it does to the board. The total damping of a key's modes grows (test `the_frame_takes_energy_from_the_strings`). One coupled mode of a unison may lose a little less, since the frame's admittance mixes them differently.

The frame takes little: its conductance at the agraffe is 11–24 dB under the board's at the bridge over C6–C7's partials (`examples/frame_drain.rs`), 17 dB at C7's fundamental. The coupling changes the frame's level by less than a decibel (C7: −3.2 → −3.7 dB), and the bench not at all (1.11).

What sets that peak is the frame's own damping: a mode with η = 1 % is a narrow resonance, and C7's fundamental sits on one. Frame level re the note, 0.5–1.5 s, every other key from C4 to C8:

| η | loudest (C7) | mean (power) | bench |
|---|---|---|---|
| 1 % | −3.7 dB | −16.9 dB | 1.11 |
| 2 % | −8.7 dB | −20.9 dB | 1.11, no measure moved by 0.3 z |
| 3 % | −13.0 dB | −23.5 dB | 1.11; the treble's sustain partial 6 z −0.6 → −1.9 |

At 3 % the frame's sheen leaves the treble's sustain partial 6, which the real grands have. The default is 2 %.

## Levels

Measured with `cargo run --release -p piano-core --example structure_levels`, dry, with no room or noises, velocity 100. Each contribution is isolated by differencing renders and compared with the note without frame or rim. The three windows are 0–0.2 s | 0.5–1.5 s | 2–4 s.

| Note | Frame | Rim |
|---|---|---|
| E1 (28) | −34 · −27 · −27 dB | −41 · −38 · −38 dB |
| A2 (45) | −36 · −42 · −43 dB | −34 · −46 · −45 dB |
| C4 (60) | −31 · −34 · −28 dB | −40 · −38 · −36 dB |
| C5 (72) | −28 · −20 · −23 dB | −39 · −39 · −39 dB |
| C6 (84) | −18 · −13 · −13 dB | −38 · −35 · −35 dB |
| C7 (96) | −15 · −25 · −28 dB | −40 · −46 · −42 dB |

The table below is the rectangular frame's (see [The outline](#the-outline) for the current one). These values were remeasured after the sustain recalibration ([sustain.md](sustain.md)). Before, the frame was 3 to 10 dB lower in the treble. Since then, the soundboard, stiffer under the treble, radiates the treble strings less, and they keep their energy longer. The frame is unchanged: proportionally, it therefore weighs more.

- **The frame** can be heard in the treble as a slight metallic sheen, up to −13 dB at C6. When a partial falls on a frame resonance, the frame can prolong that partial a little.
- **The rim** stays 36 to 45 dB below the note: a barely perceptible low background.
- `structure_transfer` compares the transfer function of the frame (agraffe to microphone) with that of the soundboard (bridge to microphone), per octave. The frame is 40 to 50 dB lower under 250 Hz, and 17 to 23 dB lower above 1 kHz.

A first version gave the frame η = 0.4 %, its whole surface radiating, and agraffes 15 cm from the edge. The frame then drowned the note in the treble (up to +20 dB at the end of the sound), which no piano does. The chosen values are more realistic, but **neither the frame level nor the rim level is calibrated**: no recording available to us isolates them.

To hear them, `gain` (1 = physical) amplifies them without changing anything else. For example `[frame] gain = 10` and `[rim] gain = 30`.

## Cost

Measured with `stress_cost` (88 voices, pedal):

| Configuration | Load on one core |
|---|---|
| Without frame or rim | 22–23 % |
| With the rim | +1 % |
| With the frame | +5 % |
| Everything | 28 % |

- **Frame.** Most of its cost comes from the second output of each string bank (the force at the agraffe). Its own bank (247 modes) is negligible. Without the frame, the strings do not compute this output.
- **Rim.** The 17 projections only cover the first 6 packets of soundboard modes. They required two fixes:
  - its parameters became actual function arguments: grouped in a struct, they prevented vectorisation, for lack of a no-aliasing guarantee;
  - the lane sums are done as a tree.
- Instrument construction: unchanged to within 0.1 s.

## Fix along the way

The sign of the force at the agraffe was inverted (partial indices counted from 1). The render was unaffected, since this force was not yet used anywhere.

## What is missing

- **Reaction on the strings.** The frame and the rim react neither on the strings nor on the soundboard. A compliant rim would slightly lower the first soundboard modes and increase their damping.
- **Unmodelled contributions.** The longitudinal forces of the strings on the frame (“phantom” partials), frame ↔ rim transmission through the bolts, and action impacts on the rim: key-bottom noises still go through the soundboard.
- **Simplified geometry.**
  - The rim is an equivalent ring, not the real curved shape with its beams and legs.
  - The frame is an equivalent solid plate, not a network of bars.
- **Air cavity.** The air between the soundboard and the floor, or the underside of the piano, is not modelled. It is probably the most audible “case” contribution remaining.
