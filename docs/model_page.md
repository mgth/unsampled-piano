# The Model page

The interface's **Model** page draws the instrument as the model builds it, to scale, in three modes:

- **Static**: every part where the parameters put it (soundboard, bridges, strings from their agraffe to their bridge point, hammers at the strike point, dampers, frame, rim, lid, microphones), with the model in numbers beside it. Pointing at a parameter elsewhere lights its part; clicking a part's name opens its page.
- **Live**: the same instrument with the engine's state laid on it as it plays: keys and pedals down, hammers on the strings, dampers lifted, each key's string energy (the strings' motion is slowed down and enlarged to be seen, it is not their real motion) and the soundboard's velocity from its 96 lowest modes, against the loudest of the last seconds.
- **Editor**: the plan with handles on the geometry (board length and width, the outline's vertices, the main and bass bridge points, the agraffes, the frame's end, the microphones) and the geometric parameters beside it. Each move rebuilds the instrument once the edits pause, as any other rebuild parameter.

The board and the rim follow the board's outline (`soundboard.outline`, docs/outline.md), the full rectangle when none is set; the editor's **Board outline** buttons set a grand's or go back to the rectangle, and a handle on each vertex moves it. The frame and the lid stay the model's rectangles. What the model has no geometry for is drawn plainly: the strings start at their key's place across the keyboard, which crosses the bass strings over the tenor as on an overstrung piano; legs, keyboard and pedals only situate the instrument.

## Views

The plan, an orthographic view of the board's face (from above on a grand, from the front on an upright), is drawn by egui itself and works everywhere. The **3D** view turns around the instrument (drag to turn, right-drag to move, scroll to come closer, double-click to frame it again) and is drawn by the GPU, through egui's own OpenGL painter: an offscreen multisampled target with a depth buffer, copied into the view. It needs OpenGL 3.3 or GLES 3.0; if the context cannot do it, the page says why and shows the plan.

The 3D view is in `piano-ui`'s `gpu` feature and the host turns it on (`PianoUi::enable_gpu`): the standalone player does when eframe paints with glow. The plugin does not yet: its editor shows the plan.

## The engine's state

`Engine::observe` fills a `piano_core::Observation` (no allocation): per key, down, hammer on the strings, damper engagement and string energy; the pedals; the modulus of the board's lowest modes' states, in the order of `Soundboard::new`. The audio side (`EngineSlot`) publishes it about 60 times a second into atomics shared with the controller, summing the engines still finishing their notes after a rebuild; the interface reads it with `PianoControl::observe` and evaluates the modes' shapes on the board's grid. The interface's mock control plays an arpeggio by itself, so the live view can be worked on without an engine (`cargo run -p piano-ui --features gpu --example demo`, `PIANO_UI_PAGE="Model live"`).
