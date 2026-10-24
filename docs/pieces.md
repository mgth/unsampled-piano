# Recorded performances against the model

Status as of 2026-10-24.

The note-set bench ([bench.md](bench.md)) compares isolated notes. Pianos that record their own performance, a Yamaha Disklavier or a Bösendorfer SE, give both the sound and the MIDI of what was played. The model renders the same MIDI, and the notes are analysed one by one in both recordings, wherever a note can be heard on its own.

## Tools

    tools/piece_bench.py <audio> <midi> [--name N] [--preset P]
    tools/piece_bench.py --batch pairs.tsv [--jobs 8] [--reuse]       one pooled report
    tools/piece_bench.py --summarise out/pieces/<name>/result.json    summarise again
    tools/pedal_bench.py <no-pedal audio> <midi> <pedal audio> <midi> [--name N]

They write `out/pieces/<name>/report.md` and `result.json` (every note's measures).

To listen rather than measure, `tools/ab_real.py <audio> <midi> <prefix> [--start S] [--dur S] [--listening default|close]` renders the recording's MIDI with the model, cuts the same passage from both, aligned on the MIDI, and brings both to −20 LUFS (EBU R128): `<prefix>_real.mp3` and `<prefix>_model.mp3`. The recordings are not redistributed: the files stay in `out/`. With `--listening auto` the model is rendered with each microphone set (but the close one) and each room of `presets/listening`, and the pair kept whose room sounds closest to the recording's over the passage: L/R coherence at 0.5–4 kHz, and the depth of the envelope's dips between notes (p90 − p10 of its 10 ms level). A MAESTRO 2014 performance (coherence 0.28, dips 15.3 dB) gets the small hall (0.26, 16.2; the default 0.33, 20.3). The Vienna Bösendorfer recordings (0.26, 14.7) stay out of reach: the best pair gives 0.35, 16.2, and a more reverberant hall fills the dips without lowering the coherence, which the direct sound and the microphones set.

**Listening.** The model is heard as the references were recorded: close microphones in a studio (`presets/listening/Microphones/close.toml` and `Room/studio.toml`, passed to the renderer with `--listening`; `--listening none` keeps the engine's own). The engine's default listening is a concert hall heard from 1.5 m above the board. Its reverberation sits between the partials and in every decay, and the first comparisons, made with it, read the hall as part of the instrument (below). `tools/midifile.py` reads the MIDI files: notes, sustain pedal and the tempo map.

**Alignment.** The recording's spectral flux is cross-correlated with the MIDI onsets, over the first and the last third separately: a recording's clock may drift from the MIDI's (Vienna: up to 185 ppm, 22 ms over two minutes). The flux places onsets a few milliseconds off: the offset is refined on the envelope of lone notes, where it crosses 30 % of its peak. On the model, which plays the MIDI exactly, the onsets land within 2 ms.

**Same notes on both sides.** A note enters a measure only when both analyses kept it, partial by partial for the decays. The two sides are therefore the same notes at the same velocities.

**Measures**, per register:

| Measure | What | Room |
|---|---|---|
| `attack_*`, `hf_*` | partials 2, 3, 4, 6 re 1, and the share at 2–4 / 4–8 kHz, of what the onset adds (40 ms after minus 60 ms before), by velocity; partials another note struck within 45 ms comes near are left out | coloured by the microphones and the room |
| `vel_slope_db`, `vel_slope_cen` | slope of the onset's partial-1 level (dB) and centroid (%) per 10 velocity steps, fitted within each performance (gains differ between recordings), then averaged | robust |
| `decay_*` | rate (dB/s) of partials 1–2, 3–6 and 7–12 while the key is held, for the partials no other sounding note comes within 3 % of | lengthened by the room once the note falls below its reverberation |
| `release_drop` | partials 1–2, 0.3 s after the damper falls | bounded by the room's own decay |

**The pedal.** `tools/pedal_bench.py` uses Liang's isolated notes, played by a Disklavier grand once without the pedal and again with the pedal at three depths (CC64 = 64, 96, 127). Every measure is a difference between a pedalled note and the same key and velocity without the pedal, in the same recording, so that the room and the microphones cancel:
- **halo**: energy away from the note's own partials (60 Hz–6 kHz) re the partials, 0.2–0.9 s: what the other strings add;
- **hold**: partials 1–4 at 1.3–1.8 s re 0.3–0.8 s, with the key released at 1 s and the pedal holding the dampers, against the key held without the pedal;
- **after**: everything 0.3–0.6 s after the dampers fall, re 0.4–0.1 s before.

## Corpora

| Name | Piano | Content | Licence |
|---|---|---|---|
| Vienna 4x22 | Bösendorfer Imperial SE290, ORTF pair | 4 excerpts × 22 pianists (90 performances) | CC BY 4.0 |
| Liang's pedalling dataset | Yamaha Disklavier grand, spaced pair | 4 pieces with and without pedal; isolated notes, chords, arpeggios and repeated notes with and without pedal; every key at velocity 96 | CC BY 4.0 |

MAESTRO (Disklavier, 200 h): 30 performances, three from each of ten years (`out/pieces/maestro.tsv`, taken from the zip by year and length). Then come SMD, MAPS (a Disklavier upright, isolated notes and pieces), KRAISLER, PianoVAM and the MAESTRO subset re-recorded at JKU. The recordings are kept out of the repository (`/data` and external storage), and none of their audio is redistributed.

## Findings

### Dynamics: the model's level follows the velocity half as steeply

| Register | Vienna | Liang | MAESTRO | Model | Model, velocity law fitted |
|---|---|---|---|---|---|
| bass | — | +2.3 | +2.0 | +0.7 to +0.9 | +1.1 |
| middle | +2.5 | +3.0 | +2.9 | +0.5 to +1.6 | +1.9 to +2.0 |
| treble | +4.7 | +4.0 | +3.2 | +1.3 to +2.2 | +2.4 to +2.7 |

Partial-1 level at the onset, dB per 10 velocity steps, model heard with close microphones. Last column: the velocity law fitted on two Disklaviers' isolated notes ([bench.md](bench.md#velocity-curve)), against Liang and MAESTRO.

MAESTRO: 30 performances drawn from ten years of the competition (2004–2018), so several Disklaviers, halls and microphone set-ups. A Bösendorfer SE and Disklaviers, with different recording systems, agree. On these instruments a MIDI velocity step is worth 2 to 3 times as much level as on the model. The sample libraries of the note-set bench agree with the model, because their layers are mapped to velocities by their authors. A Disklavier's isolated notes, recorded at 10 velocities per key with calibrated levels (BiVib), give the whole curve: [bench.md](bench.md#velocity-curve).

### Pedal: the sympathetic gain set to its physical value, the half pedal moved

Liang's isolated notes and chords at velocity 96, model with close microphones in a studio. **Halo**: energy between the partials re the partials, 0.2–0.9 s after the onset, pedal fully down (dB). **Hold**: partials 1–4 at 1.3–1.8 s re 0.3–0.8 s (dB).

| | Disklavier | model, gain 0.25 | model, gain 1 |
|---|---|---|---|
| isolated notes, halo, bass | −12.6 | −21.4 | −14.0 |
| isolated notes, halo, middle | −19.4 | −33.3 | −25.3 |
| isolated notes, halo, treble | −21.6 | −35.0 | −24.9 |
| chords, halo, bass | −10.7 | −14.1 | −12.3 |
| chords, halo, middle | −19.7 | −26.9 | −19.3 |
| chords, halo, treble | −21.8 | −34.9 | −24.1 |
| chords, hold, middle | −10.9 | −8.2 | −6.9 |
| chords, hold, treble | −7.8 | −10.6 | −8.1 |

The sympathetic gain was 0.25: at 1, the one-way physical coupling, a pedalled chord had gained +10 dB, and that was taken for an overshoot at the octaves. The Disklavier's chords gain as much (+10 dB in the middle between pedal up and down). At 1, the halo matches the Disklavier within 0.5–2.5 dB on chords and 1.4–6 dB on isolated notes. The chords' own partials sustain as on the Disklavier, with no growth at the octaves. The gain is now 1 (`sympathetic.gain`), at no CPU cost (`perf_audit`: unchanged within its noise).

**Half pedal.** On this Disklavier, CC64 = 64 still damps. A note released at 1 s with the pedal at 64 loses, in its first second, 7 dB (bass), 15 dB (middle) and 28 dB (treble) more than with the pedal at 127, and 96 is already a full pedal. The model's dampers left the strings from CC64 ≈ 70 (`pedal_contact = 0.55`, set from MAESTRO's convention "down from 64"), so 64 barely damped (2, 3 and 10 dB). With `pedal_contact = 0.6` (off the strings from ≈ 76) the model loses 10, 15 and 25 dB, and 96 stays a full pedal.

| CC64 = 64, hold (dB) | Disklavier | contact 0.55 | 0.59 | 0.6 | 0.62 |
|---|---|---|---|---|---|
| bass | −10.7 | −4.1 | −10.8 | −12.6 | −16.9 |
| middle | −21.7 | −9.9 | −20.0 | −22.6 | −27.8 |
| treble | −33.6 | −17.2 | −31.0 | −32.6 | −37.0 |

**The first reading was wrong.** Rendered in the engine's default concert hall, the model showed between its partials −25 dB without the pedal and −24 dB with it. The hall's reverberation filled the gaps, the pedal added only +2 dB, and the halo looked 12–16 dB too weak "in difference". With close microphones the pedal adds +15 dB, as on the Disklavier, and the true gap is the one in the table above.

### Decays in context

The bass partials 1–2 decay faster on every piano than on the model (Vienna −9.6 dB/s, Liang −7.2, MAESTRO −10.3, model −4.0 to −4.6 with close microphones). A room cannot make a decay faster: the gap is the instrument's. The isolated-note bench, which fits the decay over 20 s, shows no gap. The difference is in the first second of a note, which is most of what is heard in a piece.

The upper partials (7–12) decay at −9 to −11 dB/s in the recordings, against −27 to −29 dB/s on the model. In the recordings these partials fall below the room's reverberation within a few hundred milliseconds, so the measure is not conclusive.
