# What feeds the room: probes around the piano

State as of 2026-11-17.

## Listening report

E5 sounded odd next to its neighbours with the close microphones: its upper partials were loud at pp and mf, as if its fundamental were missing.

## Diagnosis

The close microphones sit 40 cm above the board. There, the board's modes near 660 Hz cancel each other at the right (treble) microphone, so E5's fundamental arrives 18 dB below its second partial. At the left microphone, it arrives 2 dB below. A real microphone at that spot would hear the same dip in the direct sound, so this part is physical.

The room, however, was fed from those same two microphones. The engine took their pressure, scaled to 1 m, as the piano's source signal for every reflection and for the reverberant tail. The near-field dip of two points was therefore copied into the whole room.

A real room receives the power the piano radiates in every direction. In its diffuse field, those directions add as powers, not as pressures, so the dips of any single point average out.

## Change

The board bank has four more outputs: the pressure at four probes 3 m from the board's centre (`board::probe_points`). The bank's outputs are the microphones, then the probes, then the bridge velocities (`board::BRIDGE`). The probes are computed whenever the bridge velocities are, which means the main board and the low bank, but not the frame or the sympathetic board. The listening set-up's gain applies to the probes as it does to the microphones.

| Probe | Grand | Upright against a wall |
|---|---|---|
| 0 | up through the lid's opening | in front, downwards |
| 1 | up on the spine's side (under the lid) | in front, upwards |
| 2 | towards the tail | in front, to one side |
| 3 | below | in front, to the other side |

The probes hear the modes below 3 kHz (`board::PROBES_BELOW`). Above that, each probe takes the old estimate from the microphones. The probes' radiation integral is the costliest part of the build: on a 2 m grand, 263 of the 2104 modes lie below the cutoff. Probes on every mode change nothing below 3 kHz.

The probes are levelled against the old estimate from the microphones, 0.5·(p₀·r₀ + p₁·r₁). The comparison runs over the modes from 50 Hz to the cutoff, each mode weighted by the energy a white bridge force puts into it per octave, Σ_r φ_r²/(σ·f). The room therefore keeps the level it was tuned with by ear, and only its colour changes.

The room (`room::Room::process_with`) works as follows:

- **Probe sources.** Each probe gives one source signal: the board's pressure at that probe scaled to 1 m, plus what the microphones hear apart from the board. That remainder covers the action, the body, the rim and the frame, and is still estimated from the microphones as p·r.
- **Early reflections.** Each one takes the probe that faces the walls it reflects from, judged by the direction from the source towards the microphone's image.
- **Tail.** The FDN takes the four probes on four orthogonal Hadamard rows, each scaled by 1/√4, so their powers add.

Without a board feed, `Room::process` still estimates the source from the microphones alone.

## Result

Close microphones in the studio, Concert grand, partial levels over 0.05–1.5 s, measured on each microphone (left / right). A sum of the two channels is not a measure: E5's fundamental arrives at the two microphones nearly in antiphase.

| Note | Partial 1, new − old | Partial 2, new − old |
|---|---|---|
| D#5 | +2.2 / −0.2 dB | 0.0 / +0.5 dB |
| E5 pp | **+1.4 / +3.4 dB** | −2.4 / −0.3 dB |
| E5 mf | **+1.5 / +3.8 dB** | −1.5 / −0.4 dB |
| F5 | +1.4 / +1.0 dB | +2.3 / +1.1 dB |

The room alone (full render minus the render without the room) carries E5's fundamental 18 dB higher than before. At 660 Hz, the room's source used to be 6 dB lower from the microphones than from the probes. Across 300 Hz–4 kHz, the source's level spread was 34 dB from the microphones and 18 dB from the probes.

The direct sound at the right microphone still has the dip: there, E5's fundamental arrives 18 dB below its second partial. That is what a microphone at that spot hears. The concert microphones, which are further away, do not show it.

Cost, measured on one core against the previous commit:

- **Instrument build:** +0.35 s on 9 s.
- **Rendering:** about +4 % (four more outputs on the board bank).
