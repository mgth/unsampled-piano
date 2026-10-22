# Cast-iron frame and rim

Status as of 2026-10-22. Code: `structure.rs`, `board.rs` (additional outputs), `engine.rs`, `voice.rs`.

## What these parts do in a real piano

- **The frame** (the gilded cast-iron plate) takes the tension of the strings, about 20 tonnes. Each string bears on the soundboard at the bridge and on the frame at the other end, at the agraffe. So, as it vibrates, it also pushes on the frame.
- **The rim** (the laminated maple case) holds the soundboard around its edge. When the soundboard vibrates, it pulls and pushes on its edges, and the rim receives these forces.

These two parts are heavy and stiff. They move much less than the soundboard and therefore radiate little. Their main role is mechanical: holding the tension and fixing the edge of the soundboard. The model already reflects this, since the soundboard is clamped in it. Their direct contribution to the sound is small.

## Model

### Frame (`FrameParams`)

- **Excitation.** Each string computes the force it exerts on its agraffe, −T·∂y/∂x at that end. The weights are those of the bridge multiplied by (−1)^(k+1): the fundamental pulls on both ends in the same direction, the second partial in opposite directions. During hammer contact, this force also contains the quasi-static part of the hammer's push. Since the hammer strikes close to the agraffe, it is mostly the agraffe that receives it.
- **Plate.** It is a clamped isotropic plate of 1.75 × 1.3 m, computed by the same code as the soundboard: Warburton modes and Rayleigh radiation towards the microphones, from 8 cm above the soundboard.
  - Surface density 110 kg/m², i.e. about 150 kg of cast iron, holes included.
  - Stiffness chosen for a first mode around 155 Hz; 247 modes up to 12 kHz.
  - The frame is about 40 times less mobile than the soundboard.
- **Radiation.** It is a perforated plate. Only the metal fraction radiates (`solidity` 0.4), and its two faces short-circuit each other through the holes (`back_gain` 0.9 at 5 cm).
- **Damping.** η = 1 %. Bare cast iron is at 0.1–0.5 %, but once installed it is bolted to the pinblock and the rim, and loaded by 230 strings and their felts. **This is an estimate**, with no published measurement.
- **Agraffes.** They are placed 5 cm from the front edge, above the bolted pinblock. They face the bridge point of each region, since the strings run along x. The bass string crossing is ignored.
- **One way.** The frame's motion sends nothing back to the strings, which is negligible given its low mobility. The “listening” strings (sympathetic resonance) do not excite it.

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

These values were remeasured after the sustain recalibration ([sustain.md](sustain.md)). Before, the frame was 3 to 10 dB lower in the treble. Since then, the soundboard, stiffer under the treble, radiates the treble strings less, and they keep their energy longer. The frame is unchanged: proportionally, it therefore weighs more.

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
