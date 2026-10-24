# Recorded performances against the model

Status as of 2026-10-24.

The note-set bench ([bench.md](bench.md)) compares isolated notes. Pianos that record their own performance, a Yamaha Disklavier or a Bösendorfer SE, give both the sound and the MIDI of what was played. The model renders the same MIDI, and the notes are analysed one by one in both recordings, wherever a note can be heard on its own.

## Tools

    tools/piece_bench.py <audio> <midi> [--name N] [--preset P]
    tools/piece_bench.py --batch pairs.tsv [--jobs 8] [--reuse]       one pooled report
    tools/piece_bench.py --summarise out/pieces/<name>/result.json    summarise again
    tools/pedal_bench.py <no-pedal audio> <midi> <pedal audio> <midi> [--name N]

They write `out/pieces/<name>/report.md` and `result.json` (every note's measures). `tools/midifile.py` reads the MIDI files: notes, sustain pedal and the tempo map.

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

| Register | Vienna | Liang | MAESTRO | Model |
|---|---|---|---|---|
| bass | — | +2.4 | +2.0 | +0.7 to +0.9 |
| middle | +2.8 | +3.0 | +3.0 | +1.1 to +1.3 |
| treble | +4.5 | +3.8 | +3.2 | +1.5 to +2.5 |
| top | — | +5.1 | +3.7 | +3.6 to +5.2 |

Partial-1 level at the onset, dB per 10 velocity steps.

MAESTRO: 30 performances drawn from ten years of the competition (2004–2018), so several Disklaviers, halls and microphone set-ups. A Bösendorfer SE and Disklaviers, with different recording systems, agree. On these instruments a MIDI velocity step is worth 2 to 3 times as much level as on the model, and from velocity 20 to 120 the middle register gains about 30 dB on them, against 12 dB on the model. The sample libraries of the note-set bench agree with the model, because their layers are mapped to velocities by their authors. A Disklavier's isolated notes, recorded at 10 velocities per key with calibrated levels (BiVib), give the whole curve: [bench.md](bench.md#velocity-curve).

The brightening with velocity is also weaker on the model: the centroid rises by 3.7 % per 10 steps in the middle against 7.5 % (Liang).

### Pedal: the sympathetic halo is far too weak

Liang's isolated notes, pedal minus no pedal (dB), median over 18 notes per register and depth (9 at the top):

| Register | CC64 | halo rec / model | hold rec / model | after rec / model |
|---|---|---|---|---|
| bass | 127 | +5.4 / +2.6 | +2.4 / +0.2 | +2.6 / +1.0 |
| middle | 127 | +15.6 / +2.9 | +5.4 / +0.2 | −3.4 / +2.6 |
| treble | 127 | +13.7 / 0.0 | +4.4 / +1.8 | −6.2 / +19.9 |
| top | 127 | −3.2 / −0.6 | +7.8 / +14.1 | −5.2 / +11.7 |
| middle | 64 | +11.4 / +2.1 | −12.7 / −1.0 | +11.4 / −2.0 |
| treble | 64 | +12.2 / +0.1 | −15.8 / −5.0 | +7.0 / +16.6 |

- With the dampers raised, the other strings add 12–16 dB of energy between the partials of a middle or treble note on the Disklavier, against 0–3 dB on the model. This is the halo of the pedal, and the largest gap found so far.
- With the pedal down, a released note keeps 4–5 dB more than a note held without the pedal (the sympathetic strings feed it); on the model, 0–2 dB.
- CC64 = 64 still damps on this Disklavier: −13 to −16 dB in one second, where the model barely damps (−1 to −5 dB). The model's half pedal was set from MAESTRO's performances, where the dampers are off from 64.
- After the pedal is released, the model's treble and top keep ringing 12–26 dB more than the Disklavier's, which matches the top strings' too slow decay found by the note-set bench.

### Decays in context

The bass partials 1–2 decay faster on every piano than on the model (Vienna −9.6 dB/s, Liang −7.2, MAESTRO −10.3, model −3.8 to −4.7). A room cannot make a decay faster: the gap is the instrument's. The isolated-note bench, which fits the decay over 20 s, shows no gap. The difference is in the first second of a note, which is most of what is heard in a piece.

The upper partials (7–12) decay at −9 to −11 dB/s in the recordings, against −27 to −29 dB/s on the model. In the recordings these partials fall below the room's reverberation within a few hundred milliseconds, so the measure is not conclusive.
