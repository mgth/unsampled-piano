---
title: The interface
description: Pages, sketches and curves, live and rebuild parameters, help on every control, presets and MIDI learn — a tour of the window.
order: 2
icon: sliders
---

The interface is the same in the standalone application (`--ui`) and in the plugin's editor. It
shows **every parameter** of the instrument — there is no simplified front panel hiding the
model — organised the way a piano is built.

![The interface, Strings page: the C4 string, its partials, and per-key losses, string length and unison spread over the keyboard](../../assets/interface.png)

## Layout

- **Top bar** — the instrument's name, and the preset menus: *Instrument* (grouped by family)
  and *Listening* (microphones, room). Loading a listening preset keeps the instrument; loading
  an instrument keeps the listening set-up. Save writes your own.
- **Left** — the pages: Player, Microphones, Hammer, Tuning, Strings, Dampers, Noises,
  Soundboard, Frame & rim, Room, and MIDI. A search box filters every parameter by name or path.
- **Centre** — the page: a **sketch** of the part of the piano it covers, drawn from the current
  settings (the felt's depth, the pedal's curve, the strike point and the unison spread follow
  the sliders), the per-key **curves** of its laws, then its parameters.
- **Bottom** — the status: rebuild in progress, how long the last one took, voices, and the audio
  thread's CPU load (peak of the last second).

Each page here in the docs matches one in the interface — see the
[parameter reference](/docs/parameters/).

## Live and rebuild

A green mark next to a parameter means **live**: the engine applies it while you play. The
others shape the precomputed instrument; changing one starts a **rebuild** in the background —
about a tenth of a second — and the new instrument is swapped in without cutting the notes you
are holding. Drag a rebuild slider while playing and you hear the piano change under your
fingers.

## Help on every control

Click a parameter's name: a side panel tells you **what it is on a real piano**, in plain
words, and **how the model does it** — the equation, where the default value comes from, the
document that measured it. Hovering a parameter lights up the part of the sketch it acts on.

## MIDI learn

Any parameter can follow a controller of your keyboard. Click the controller cell at the end of
a parameter's row, move a knob or fader over part of its travel, and it is mapped over the
parameter's whole range (click again to learn another, right-click to remove; Esc cancels). Live
parameters are then driven sample-accurately on the audio thread, even with the window closed;
rebuild parameters rebuild as you turn. The **MIDI** page lists the mappings. The map belongs to
your controllers, not to the instrument: it is saved apart from presets (`midi.toml` in the
configuration folder). See [MIDI & automation](/docs/midi/).
