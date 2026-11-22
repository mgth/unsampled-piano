# Microphones: up to eight, mixed or one output each

State as of 2026-11-17.

## Request

"The model must take more than two microphones, and by default use the mixed set-up, with at least four microphones." Output either a stereo mix (gain and pan per microphone) or one channel per microphone (gain only, the mix left to the host).

## Model

A listening set-up holds 1 to 8 microphones (`soundboard.mics`, `piano_core::mics`). A fixed array of 8 slots plus a count keeps the model `Copy`. Each microphone has:

| Field | Role |
|---|---|
| `at`, `upright` | Position on a grand and on an upright (shares of the board with `mics_relative`, height in metres) |
| `pattern` | a + (1 − a)·cos θ: 1 omni, 0.5 cardioid, 0 figure of eight |
| `aim`, `aim_upright` | Point the microphone is aimed at (default: the board's centre) |
| `gain_db` | Level in the mix, or on its own output |
| `pan` | −1 left … 1 right, constant power |

Set-up-wide fields:
- **`mic_gain_db`** is the set-up's level.
- **`outputs_per_mic`** (switch) chooses one output per microphone instead of the stereo mix.

In a preset, `[[soundboard.mics]]` tables replace `mics = [[x, y, z], [x, y, z]]`. Older presets still load: their two positions, `mics_upright` and single `mic_pattern` become two microphones panned hard left and right, with no aim (the board's centre). An omni pair renders as before, −132 dB from the previous engine.

## What each microphone hears

The pattern now weighs the direct sound too, not only the room. Every path of the Rayleigh integral is weighed at its angle of arrival from the microphone's aim:
- the board's two faces;
- the lid's image;
- the wall behind an upright;
- the scattering layer.

The case's field, the room's reflections and its tail arrive from every direction. Each microphone hears them with its pattern's diffuse efficiency, a² + (1 − a)²/3.

Per microphone:
- **Banks:** one output of the board's and the frame's banks, and one of the rim's.
- **Room:**
  - its own early reflections;
  - its own tail output, decorrelated from the others above the corner set by its distance to the nearest other microphone.
- **Action and body:** the action's knock and the body's halo are rendered on two sides and shared among the microphones by their pan, each microphone hearing them from where it stands (below).

The room's source estimate from the microphones is now their mean.

### The noises heard directly, microphone by microphone

Until 2026-11-21 the knock and the halo reached every microphone alike: a pair 2 m away heard them as loud and as early as the close pair, and through its own gain. The set-up's `mic_gain_db` scales the board's weights, not those noises. In `mixed.toml`, the default, the AB pair takes +7.8 dB and the two identical copies added in phase, where the strings, reaching each microphone differently, add only in part. E1 at velocity 80 sounded "awful" there and fine through either pair alone. The noises against the strings over the first 0.1 s: close −17.6 dB, concert −13.1, mixed −4.1; above 4 kHz in mixed they stood 2.8 dB over the strings.

The halo is the board and the case ringing, and the action stands against them: each microphone now hears them as it hears the board (`mics::noise_paths`, `Instrument::noise_paths`):
- **Gain:** the board's radiation at the microphone against at the reference pair, the close pair of `close.toml` where the noises were set (`NOISE_REFERENCE`), in mean |p|² per octave averaged in dB over 125 Hz–4 kHz (the built modes below 4 kHz, omni), times the set-up's gain against the reference's (−7.4 dB, `NOISE_REFERENCE_GAIN_DB`, as the listening presets give them, before the musician's controls move both alike).
- **Delay:** the mean path from the board and from the action along the keys, against the reference's.

A microphone at the reference hears them as before: close renders bit for bit as before. On E1: close −17.6 dB, mixed −16.7, concert −19.7 (it had 4.5 dB more than close, from the gain the noises did not take). Heard: E1 right in mixed, concert still good. The board's radiation carries the lid's shadow and reflection, and so the noises' gain does: taking the lid off moves a far microphone's from −15.8 to −13.8 dB at the same place, as it moves the board's sound there (the action and the body sound from under the lid too). Patterns are not counted. Near a reference microphone the gain and the delay tend to its own (the references weighed by 1/d²), and the delay is held under 50 ms (`MAX_NOISE_DELAY`): a preset's coordinates cannot size the engine's history without bound. With every microphone at the reference the radiation is not integrated at all. A board with no mode in the measured octaves (`f_max` very low) takes the mean path's 1/r for the gain instead, continuous at the reference as well. Test `the_noises_reference_is_the_close_set_up` (piano-host) keeps the reference on `close.toml`.

## Mixdown and outputs

- **Stereo:** each microphone goes to left and right by its gain and constant-power pan. A pair panned hard left and right gives the previous engine's output.
- **One per microphone:** each channel carries its microphone times its gain. A host with only two channels gets the stereo mix.

| Host | Stereo | One per microphone |
|---|---|---|
| Player (JACK) | `out_left`, `out_right` | mics 1–2 on `out_left`/`out_right`, the others on `out_3`…`out_8` |
| Player (sound card) | channels `first`, `first + 1` | from `first` on, if the device has room for them |
| Plugin | main pair | "Multi-out" layout: mics 1–2 on the main pair, 3–8 on three auxiliary pairs |
| Renderer | 2-channel WAV | one WAV channel per microphone |

The limiter is linked over all the channels.

## Set-ups

The listening presets are in archive/presets/listening/Microphones. The microphones' page offers the same set-ups (Player was missing there):

| Set-up | Microphones | Gain |
|---|---|---|
| **mixed** (default) | Close cardioid pair aimed straight down + AB omni pair 2 m high in front of the opening, the AB pair 3 dB under the close one | −9.8 dB (AB +7.8 dB) |
| close | Cardioids 40 cm above the bass and treble parts of the bridge, aimed straight down | −7.4 dB |
| concert | AB omni pair 2 m high in front of the opening | +3.4 dB |
| close_room | Close pair + omni pair 6 m out in the room, 6 dB under | −8.4 dB (room pair +2.6 dB) |
| tree | Three omnis 2.5 m high: left, right, the centre one 45 cm further out and 3 dB under | +2.9 dB |
| audience_lid_closed | The concert pair 1 m high, lid shut | 0 dB |
| player | The pianist's ears | −6.1 dB |

The gains were measured on C2, C4 and C6 at mf in the small hall, as the power summed over channels. The new set-ups match the concert set-up's loudness to within 0.1 dB.
- **close:** the cardioids now hear the board through their pattern, so the set-up lost 2.2 dB. Its gain went from −9.6 to −7.4 dB.
- **mixed:** each pair was first measured alone, then the sum was levelled.

## Result

- **Bench** (close mics, studio): 1.07 → 1.03, 10 measures off instead of 13. The close cardioids now weigh the lid's reflection, which reaches them from above and behind.
- **Cost, on one core** (machine loaded, so noisy): with two microphones, the same as before. The mixed set-up with four microphones adds about 2.4 s to the instrument build (radiation at two more points, on ~9.5 s) and 5–25 % to the rendering.
