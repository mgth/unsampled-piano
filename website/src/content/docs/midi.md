---
title: MIDI & automation
description: What the piano reads from MIDI, the pedals and their half positions, MIDI learn, the MIDI mirror output, and host automation in the plugin.
order: 4
icon: midi
---

## What it reads

| Message | What it does |
|---|---|
| Note on | throws the hammer; the velocity sets its speed (see the [velocity map](/docs/parameters/player/#from-velocity-to-hammer-speed)) |
| Note off | lets the damper fall; the **release velocity** sets how fast, so how loud its landing thud is |
| CC 64 | sustain pedal, **continuous**: half pedal included |
| CC 66 | middle pedal: sostenuto, or the practice mute on an upright preset (`mute_felt`) |
| CC 67 | soft pedal: una corda on a grand, half blow on an upright, continuous |
| other CCs | free for [MIDI learn](#midi-learn) |

Events are applied **sample-accurately** inside the audio buffer — in the standalone player
through the PipeWire/JACK MIDI port, in the plugin at the offsets the host gives — so timing does
not depend on the buffer size.

A **continuous** sustain pedal matters: most of what a pianist does with the right foot is
neither up nor down. Keyboards with a continuous pedal (or a half-pedal-capable one) send CC 64
with values between 0 and 127; on/off switches only send 0 and 127.

## MIDI learn

Every parameter can be mapped to a controller: click the controller cell of its row in the
interface, move a knob or fader, done. A controller covers the parameter's whole range; a learnt
map with min above max inverts it. Bank select, the three pedal CCs and channel-mode messages
are never learnt.

- Mapped **live** parameters are driven on the audio thread, sample-accurately, with or without
  the window open.
- Mapped **rebuild** parameters start a background rebuild as you turn, without cutting notes.

The map is about your controllers, not the instrument: the standalone player keeps it in
`midi.toml` in the configuration folder, the plugin saves it with the project.

## MIDI mirror (standalone, Linux)

On Linux the player's node also has a **MIDI output** that mirrors what it plays.
`--midi-out <name>` connects it to another synthesiser in the graph, to record or layer.

## Plugin automation

In the CLAP and VST3 plugin, the **live** parameters are host parameters: automate them, map them
to a control surface, modulate them. Each has a stable id derived from its path (an FNV-1a hash),
so automation survives new versions that add parameters. The rest of the instrument — the rebuild
parameters — and the learnt controllers are saved in the project's state; editing them rebuilds
the instrument in the background without cutting notes.
