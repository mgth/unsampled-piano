# Geometric string nonlinearity (phantom partials)

Status as of 2026-10-22. Code: `params::StringNonlinearity`, `note_model::Longitudinal`, `Voice::nl_start` and `Voice::nl_run`.

## Physics

A string vibrating strongly stretches, following Bank & Sujbert (2005). Its slope is y' = Σ a_k·cos(kπx/L), with a_k = (kπ/L)·q_k. The stretching drives longitudinal mode n (shape sin(nπx/L), modal mass μL/2) with the force

F_n = −(EA·nπ/8)·(Σ_{j+k=n} a_j·a_k + 2·Σ_k a_k·a_{k+n}),

and this mode pulls on the bridge with EA·(nπ/L)·ξ_n. Products of partials give sum and difference frequencies: these are the “phantom partials”, a few hertz below each transverse partial because of inharmonicity. The longitudinal resonances fall between 14 and 20 times f0. The effect grows as the square of the amplitude: none at pp, marked at ff, especially in the bass.

- **EA** is derived from the already calibrated inharmonicity: B = π³·E·d⁴/(64·T·L²), hence EA = E·π·d²/4 with E = 200 GPa. This gives a core of 0.9 to 1.4 mm, and a first longitudinal mode at 544 Hz for A0 and 1.1 kHz for E2, i.e. 14 to 20 times f0. Giordano and Conklin measure 15 to 20 times.
- **One-way**: the transverse motion stays linear, as in Bank & Sujbert. Pitch modulation at ff is not modelled.
- **Transmission to the bridge**: the longitudinal force pulls the bridge in the plane of the soundboard. The soundboard bends only through the moment at the foot of the bridge, which grows with its wavenumber. The model therefore applies a first-order high-pass at 1 kHz, then the coupling `coupling` = 0.3. Without the high-pass, the quasi-static tension modulation added to the first partials and muffled the bass.

## Real-time implementation

At the start of a free segment (outside hammer contact), the voice reads the mean vertical displacement of each partial from the states of the coupled mode bank. It then rotates it with the pole of the main mode, without touching the SIMD kernel.

Per sample: products of partials, with 8 vectorisable accumulators, and longitudinal resonators.

Computation limits:
- keys up to E4, partials below 5 kHz (at most 48), longitudinal modes below 8 kHz;
- 2 ms ramp-up after contact;
- computation suspended below 0.1 mN of estimated force.

## Calibration

`tools/phantom_levels.py` measures the level of the peaks below each partial n = 8..30 at ff, relative to partial n, as the median over keys C1 to A2. Each piano is measured with its own inharmonicity.

| | Salamander | Iowa | Commercial | Model without | Model (`coupling` 0.3) |
|---|---|---|---|---|---|
| phantoms (median) | −28.7 dB | −26.0 dB | −31.4 dB | −38.8 dB (floor) | −27.9 dB |

With `coupling` = 1, the model's phantoms were about 10 dB too strong.

## Effect and cost

- **pp→ff brightening of the bass** (slope of partials ≥ 6 versus 1–2): +2.5 → +2.6 dB, against +6 dB in the references. At the references' actual level, the phantoms are not enough to explain this brightening: another part remains to be found (bass hammer, radiation).
- **Hammer score**: 1.73 → 1.70.
- **CPU**:
  - worst case `stress_cost` (88 voices ff, pedal): 28 → 37 % of one core;
  - Rachmaninoff: mean 23.4 → 25.2 %, worst buffer unchanged (about 55 %).

Listening excerpts: `out/nonlinear/` (A1 ff and 45 s of the Rachmaninoff, with and without).

## What remains

- Pitch glide at ff (tension modulation on the transverse modes).
- Phantoms during hammer contact: the first 2 to 5 ms are not computed.
- Origin of the rest of the bass brightening.
