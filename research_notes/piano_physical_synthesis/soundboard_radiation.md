# Piano "body" modeling for real-time synthesis: bridge, soundboard, case/frame, sympathetic resonance, radiation (state of the art to 2026)

Scope note: these notes cover string–bridge coupling through to the pressure at a listener or microphone. Every bullet under "Cited Findings" was read in this session, either in a full text (Bank/Zambon/Fontana 2010, Zambon/Lehtonen/Bank 2008, Chabassier/Duruflé/Joly Part 2, Ege & Boutillon mobility synthesis, Plath ISMA 2017 abstract) or in an abstract or search snippet, which is flagged where relevant. Classic references I know of but could not verify in this session are listed under "Gaps", not presented as findings.

Real-time status tags: **[RT-proven]** means a published real-time implementation exists. **[offline]** means offline or HPC only. **[data]** means measured data usable for calibration.

---

## 1. Soundboard physics and published driving-point admittance (mobility) data

### Takeaway
The soundboard behaves like a homogeneous, roughly isotropic clamped plate up to about 1–1.1 kHz, and above that like a few rib-bounded waveguides with localized modes and a lower apparent modal density. It is essentially linear (the nonlinear part is about 40 dB below the linear part at ff), with a loss factor of about 2%. Only four published bridge mobility/impedance datasets exist (Wogram, Nakamura, Conklin, Giordano), all normal to the board and given as curves only. The best compact calibration recipe is the Ege–Boutillon "synthetic mobility": mass, modal density and loss factor give a mean mobility and its resonance/antiresonance envelopes.

### Cited Findings
- **Linearity.** Ege, Boutillon & Rébillat (JSV 332, 2013, pp. 1288–1305) separated soundboard nonlinearity from loudspeaker nonlinearity on a piano in playing condition. The nonlinear part is about 40 dB below the linear part at ff, so the board can be treated as linear. Modes were identified up to 3 kHz with a high-resolution modal analysis method (Ege et al., JSV 2009). Modal dampings are "close to those imposed by wood". An FE model indicates the boundaries act as blocked (clamped) except at very low frequencies. [data] — [arXiv 1212.2323](https://arxiv.org/abs/1212.2323); [Ege2013 PDF](https://ppi-lms.polytechnique.fr/xavier.boutillon/Annexes/Ege2013.pdf)
- **Two-regime behavior.** The board acts as a homogeneous plate below about 1 kHz. Above that, vibration is rib-confined and localized. — [arXiv 1212.2323](https://arxiv.org/abs/1212.2323)
- **Ege & Boutillon "Synthetic description of the piano soundboard mechanical mobility"**, measured on an upright with muted strings in a pseudo-anechoic room:
  - Soundboard mass M = 9 kg; plate-equivalent dimensions 1.39 × 0.91 m.
  - Low-frequency modal density n = 1/19.5 modes/Hz (about 0.051 modes/Hz). The board behaves as a homogeneous isotropic clamped plate up to about 1.1 kHz.
  - Mean loss factor η about 2%.
  - Transition at about 1.1 kHz, where half a wavelength equals the rib spacing. The inter-rib waveguide studied has p about 12.8 cm and h about 8 mm.
  - Above the transition, apparent modal density falls to about 4×10⁻³ modes/Hz at 2.5 kHz for one waveguide.
  - [data] — [ar5iv 1210.5688](https://ar5iv.arxiv.org/html/1210.5688)
- **Mobility formulas from the same paper**, directly usable to synthesize a statistically correct admittance:
  - Mean real part of driving-point mobility G_C = n(f)/(4M) (Skudrzyk).
  - Infinite-plate value G_C = (1/(4h²))·√(3(1−ν²)/(Eρ)).
  - Resonance and antiresonance envelopes G_res ≈ G_C·β and G_ares ≈ G_C/β, with β = 2/(πμ) and modal overlap μ = n·η·f.
  - Langley regular-spacing variant: coth(πμ/2) and tanh(πμ/2).
  - Measured mean mobility between 100 and 1000 Hz is about 1.3×10⁻³ s/kg (about 800 kg/s impedance), with ±10–15 dB fluctuations. Impedance falls to about 230 kg/s at 2.5 kHz.
  - Skudrzyk's formula is valid only below about 200 Hz, where modal overlap is under 20%.
  - Source: [ar5iv 1210.5688](https://ar5iv.arxiv.org/html/1210.5688)
- **Published bridge data**, as listed and compared in Ege & Boutillon:
  - **Wogram (1980):** about 10³ kg/s from 100 to 1000 Hz, then falls about 5 dB/octave to about 160 kg/s at 10 kHz. Weinreich considers the falloff probably an artefact.
  - **Nakamura (1983):** mobility rises at high frequency, attributed to inter-rib vibration. Data above about 2 kHz are questionable because of transducer resonances.
  - **Giordano (1998):** about 1–2×10³ kg/s at the bridge near a rib below about 700 Hz, and 0.6–0.7×10³ kg/s between ribs. There is a step-like falloff near 2.5 kHz at the bridge and near 700 Hz between ribs. Treble values between ribs are about 200–300 kg/s.
  - **Conklin (1996):** concert grand, measured with strings and plate removed and again fully assembled and tuned. Assembly raises the first mode frequencies and lowers peaks by about 15 dB. Mean mobility stays about constant to 3.2 kHz, and nothing is published above 3.2 kHz in the normal direction. Longitudinal (in-plane) treble mobility is 10–20 dB above normal mobility unloaded, and drops 10–15 dB when assembled.
  - The authors consider Conklin's data the most reliable. All four datasets are normal to the board and mostly on uprights.
  - Source: [ar5iv 1210.5688](https://ar5iv.arxiv.org/html/1210.5688)
- **Conklin's design observation:** pianos with larger bridge mobility tend to sound harsh and have shorter tone durations. — cited via [arXiv 1210.5688](https://arxiv.org/pdf/1210.5688) (search snippet)
- **Giordano (JASA 102, 1997):** an FE soundboard model focused on impedance between 50 and 104 Hz. It argues the ribs are essential to reproduce measured impedance. Giordano's published impedance falls above about 7 kHz, which conflicts with plate theory (constant high-frequency value). — [Purdue piano articles](https://www.physics.purdue.edu/piano/articles.shtml); [arXiv 1101.4511](https://arxiv.org/pdf/1101.4511) (search snippets)
- **Assembled impedance ratio:** Bank et al. state that soundboard impedance at the bridge is "around a thousand times larger" than string impedance. The main effect of coupling on the string is therefore a change of partial frequencies and decay times. — [Bank, Zambon, Fontana 2010](https://home.mit.bme.hu/~bank/publist/taslp10.pdf)
- **Boutillon & Ege, reduced models (JSV 332(18), 2013, pp. 4261–4279):**
  - Below f_lim: coupled orthotropic substructures (two ribbed areas split by the main bridge, plus cut-off corners), with an analytic low-frequency modal density for an orthotropic plate with arbitrary boundaries.
  - Above f_lim: three structural waveguides formed by the inter-rib spaces around the excitation point. f_lim depends on wood properties and rib spacing.
  - Both models reproduce published mobilities and modal densities.
  - The subsonic (below-coincidence) radiation regime extends over a much wider band than for a homogeneous plate.
  - Source: [arXiv 1305.3057](https://arxiv.org/abs/1305.3057v1)
- **Crown and downbearing (Mamou-Mani et al.):**
  - JASA 123 (2008) is numerical; JASA 2012 is experimental (Mamou-Mani, Le Moyne, Ollivier, Besnainou, Frelat).
  - Prestress from crowning (gluing ribs) and from downbearing (string tension) shifts soundboard eigenfrequencies.
  - The experiment used a simplified plate + bridge + single string bench with nearfield acoustic holography. It showed good qualitative agreement with the numerical and analytical (Acta Acustica 2009) predictions.
  - A review notes that a nonlinear (prestressed) FE solution is required to capture crown and downbearing effects.
  - Sources: [HAL hal-01674967](https://hal.sorbonne-universite.fr/hal-01674967); [Southampton review postprint](https://eprints.soton.ac.uk/408405/1/20160514_postprint.pdf) (snippet)
- **Trévisan, Ege, Laulagnet (2015–2017):**
  - Analytical/modal model of ribbed, non-rectangular, orthotropic, clamped plates. The board is embedded in a larger simply supported plate, with spring distributions enforcing the clamped boundary.
  - Radiation is computed from the larger plate's modal radiation impedances, applied to an upright soundboard.
  - Vienna Talk 2015 compares this model with measurements on one soundboard.
  - Part of ANR project MAESSTRO ("Acoustical modeling, prototypes and sound synthesis for piano soundboards").
  - Sources: [science.gov record](https://www.science.gov/topicpages/s/supported+rectangular+plate) (snippet); [Vienna Talk 2015 pap 56](https://viennatalk2015.mdw.ac.at/index.htmlpage_id=13002&pap=56.html); [ANR MAESSTRO](https://anr.fr/Project-ANR-14-CE07-0014)

### Inferences
- **Generating a soundboard model without measurements:**
  - Draw mode frequencies with density n(f): about 0.05 modes/Hz below about 1 kHz, falling above. This gives about 50 modes in the first 1 kHz for an upright; expect more for a larger grand board.
  - Set η about 0.02, so Q is about 50 and T60 ≈ 2.2/(η·f), roughly 110 ms at 1 kHz.
  - Draw mode amplitudes so the mean mobility at the bridge is about 1.3×10⁻³ s/kg (about 800 kg/s). Use the Skudrzyk/Langley envelopes for peak and trough statistics.
  - This is a physically grounded "statistical soundboard" that needs only a handful of parameters.
- **Linearity matters for the architecture.** It justifies LTI treatments of the board: modal filter bank, convolution, commuted synthesis. All nonlinearity can be kept in the string and hammer.
- **Data availability.** Driving-point admittance per bridge position has been published only as plotted curves in papers (Conklin, Giordano, Wogram, Nakamura, Ege). I found no downloadable numeric dataset, so a Rust implementer must digitize the plots or measure their own.

### Gaps
- I could not verify bibliographic details or content directly for several items:
  - Conklin 1996 Part II (JASA 100, "Design and tone in the mechanoacoustic piano. Part II: Piano structure").
  - Suzuki 1986 (JASA, soundboard vibration and radiation, including radiation efficiency).
  - Wogram 1980; Nakamura 1983.
  - Berthaut et al. (2003, Applied Acoustics: rib-confined waves above about 1 kHz).
  - Chaigne, Cotté & Viggiano (JASA 2013, "Dynamical properties of piano soundboards").
  - These are known references, but they were not fetched here.
- I found no openly downloadable, numeric, per-bridge-position admittance dataset for a grand piano. The Hamburg group (Plath, Bader) measured 15 driving points on the bridges of two concert grands (see §5), but the raw data was not found online.
- No f_lim, critical/coincidence frequency or radiation-efficiency numbers were obtained for grand pianos in this session.

---

## 2. Real-time soundboard implementations and string–bridge coupling

### Takeaway
The key real-time reference is still **Bank, Zambon & Fontana, IEEE TASLP 18(4):809–821, May 2010** [RT-proven]. Its design choices:
- Modal strings and modal secondary resonators.
- Soundboard as an LTI filter driven by the summed bridge force: either partitioned FFT convolution of a measured force-to-pressure IR, about 20,000 taps, or a fixed-pole parallel second-order filter bank with log-spaced poles, for example 100 sections.
- One-way string-to-soundboard coupling. The soundboard's impedance effect is folded into each string's partial frequencies and decays.

Full polyphony ran at about 30% of a 2.4 GHz Core 2 Duo in 2010.

### Cited Findings
- **Architecture.**
  - Hammer–string coupling is bidirectional. String–soundboard coupling is treated as one-way: only the soundboard's "radiation effects" are modeled.
  - Its "impedance effects" (frequency shifts and extra damping of string partials) are included in the string model's parameters.
  - Justification: soundboard impedance is about 1000 times string impedance.
  - The bridge force F_b feeds the soundboard filter.
  - Source: [Bank, Zambon, Fontana 2010 PDF](https://home.mit.bme.hu/~bank/publist/taslp10.pdf)
- **Taxonomy of soundboard methods**, from the same paper:
  - **Commuted synthesis:** the soundboard IR becomes the excitation wavetable and the hammer becomes a filter. Lowest cost, but assumes LTI, so it cannot handle restrikes of a sounding string or nonlinear strings.
  - **Reverberator-like soundboards:** coupled digital waveguides, or feedback delay networks. Only the statistical modal distribution and overall damping can be fitted to data.
  - **Filter-based soundboards:** the measured force-to-pressure transfer function is the target.
  - Source: [Bank et al. 2010](https://home.mit.bme.hu/~bank/publist/taslp10.pdf)
- **FIR length requirements.** A windowed measured IR needs 1000–2000 taps at 44.1 kHz for good tonality. Reproducing the characteristic "knock" of mid and high notes needs "ten thousands of taps". Remedies are multirate filtering (Bank's earlier work) or fast convolution. — [Bank et al. 2010](https://home.mit.bme.hu/~bank/publist/taslp10.pdf)
- **Partitioned FFT convolution cost.**
  - With block size N and IR length M, the cost per sample is O(log N + M/N). The frequency-domain multiply-accumulate costs 4(N+1)⌈M/N⌉ per block.
  - Example: a 20,000-tap soundboard with N = 128 costs 633 flops/sample for the spectral multiply alone.
  - Non-uniform partitioning (Gardner; Garcia) removes the latency/cost trade-off but needs careful scheduling for constant load.
  - Source: [Bank et al. 2010](https://home.mit.bme.hu/~bank/publist/taslp10.pdf)
- **Fixed-pole parallel second-order filter (Bank's "parallel filter").**
  - H(z) = Σ_k (d_k0 + d_k1 z⁻¹)/(1 + a_k1 z⁻¹ + a_k2 z⁻²) + optional FIR part Σ b_m z⁻ᵐ.
  - Poles are fixed on a logarithmic frequency grid. Pole radii form an exponentially damped sequence approximating constant-Q resolution, with damping parameter R setting the radius at fs/2.
  - The numerators are then linear-in-parameters and solved in closed form by least squares against the measured soundboard response.
  - Example: 100 log-spaced poles, R = 0.98, order 200, no FIR part. It follows the long-ringing low modes well.
  - Stereo or multiple outputs share the same poles and add only extra numerator sets.
  - Pole angles can be scaled to simulate a larger or smaller body, and pole radii changed to alter body decay.
  - Each section can be read as one normal mode of the board, which gives a "fully modal" piano.
  - Source: [Bank et al. 2010](https://home.mit.bme.hu/~bank/publist/taslp10.pdf)
- **Bridge-position dependence.**
  - The soundboard IR varies with bridge excitation point. Options are separate filters per bridge region, each stereo, or one high-order soundboard filter (or stereo pair) plus low-order shaping filters per bridge region (their ref. [7]).
  - The final real-time system used 4 convolutions of 20,000 taps.
  - Source: [Bank et al. 2010](https://home.mit.bme.hu/~bank/publist/taslp10.pdf)
- **CPU figures (2010).**
  - Cost is 3 multiplications per output sample (MPOS) per resonator with input, and 2 when free-running.
  - Relative cost at full polyphony: transverse modes 40%, hammer 5%, secondary resonators (beating, two-stage decay, sympathetic) 30%, longitudinal 25%.
  - Soundboard convolutions cost about a quarter of the string+hammer load at full polyphony, and can run on a separate core.
  - Total: 10,000 second-order resonators plus four 20,000-tap convolutions at about 30% load on a Core 2 Duo at 2.4 GHz, with SIMD.
  - Rates: audio rate; "MIDI rate" fs/N_a for polyphony and hammer initialization; "calibration rate" of 20–50 Hz for parameter updates.
  - Source: [Bank et al. 2010](https://home.mit.bme.hu/~bank/publist/taslp10.pdf)
- **Outlook stated in the paper.** All parts are physical models except the soundboard, which is a digital filter. Future work could compute the soundboard IR offline (at "calibration rate") with FD/FE models and then run it as a filter, allowing geometry and material edits. — [Bank et al. 2010](https://home.mit.bme.hu/~bank/publist/taslp10.pdf)
- **Sound examples:** grand, fortepiano (different soundboard response) and honky-tonk presets, all played in real time from MIDI. — [paper homepage](https://home.mit.bme.hu/~bank/publist/taslp-piano/index.html)
- **Frequency-zooming ARMA (FZ-ARMA, Karjalainen et al.).**
  - Splits the spectrum into subbands, decimates, runs ARMA/Prony in each band, then maps the poles back to full rate by counter-rotating by 2πf_m/fs and rescaling radii.
  - Suited to dense modal clusters.
  - Used by Zambon, Lehtonen & Bank (DAFx 2008) as the pole-finding step before a least-squares fit of numerators.
  - Source: [DAFx 2008 paper](https://dafx.de/paper-archive/2008/papers/dafx08_37.pdf)
- **Other physics-based options (secondary sources).**
  - Mixed FD/modal piano string schemes have used a measurement-based multirate soundboard filter. — [ResearchGate: longitudinal vibrations paper](https://www.researchgate.net/publication/7842213_Generation_of_longitudinal_vibrations_in_piano_strings_From_physics_to_sound_synthesis) (snippet)
  - Real-time FD whole-instrument models have been run on FPGA. — [DAFx 2012](https://dafx12.york.ac.uk/papers/dafx12_submission_47.pdf)
  - Large-scale real-time modular coupling (strings and connections) ran on CPU in 2019. — [DAFx 2019 paper 22](https://dafx.de/paper-archive/2019/DAFx2019_paper_22.pdf)
  - A 2024 piano physical-modeling write-up recommends modal superposition (decoupling and dimension reduction) over direct FD time stepping for coupled systems. — [arXiv 2409.03481](https://arxiv.org/pdf/2409.03481)
  - 2026: "nlm", real-time non-linear modal synthesis in Max. — [arXiv 2603.10240](https://arxiv.org/pdf/2603.10240)

### Inferences
- **Cost scaling on a modern CPU.**
  - A 100-section parallel filter costs about 4 MACs per section per sample, about 400 MAC/sample per output channel at fs, using shared poles.
  - Four soundboard filters plus stereo is still under 5k MAC/sample, which is trivial for 2026 CPUs.
  - A 20k-tap uniformly partitioned convolution (N = 128) is about 600 flops/sample per channel.
  - Either approach is real-time with a large margin. The 2010 figure (30% of a Core 2 Duo for everything) implies under 5% of one modern core.
- **For a no-sample Rust engine.** The parallel modal filter is the natural soundboard. Its poles and residues can be:
  1. fitted (least squares) to a measured force-to-pressure IR when one is available (calibration only);
  2. generated statistically from the Ege–Boutillon parameters (n(f), η, M); or
  3. computed offline by FE or modal analysis of an orthotropic ribbed plate (Trévisan-type or Chabassier-type model), then truncated to perceptually relevant modes.

  Option 3 matches the "calibration-rate FE" outlook in Bank et al. 2010.
- **Two-way coupling.** Since board impedance is about 1000 times string impedance, a two-way modal coupling (string end terminated by the board's admittance, Y_b(ω) = Σ residues/poles) is affordable. It is mainly needed to get string partial decays, Weinreich-style unison coupling and sympathetic transfer "for free". The one-way approach requires baking those effects into per-string parameters.

### Gaps
- I did not verify the following directly:
  - Smith & Van Duyne (1995, ICMC) commuted piano synthesis.
  - Van Duyne & Smith "coupled-string / soundboard" waveguide work.
  - Välimäki et al. multirate/commuted piano.
  - The FDN soundboards cited in Bank 2010 as refs [6] and [7] (likely Bank's own thesis and Rauhala/Välimäki work; references not resolved).
  - Karjalainen's FZ-ARMA original papers.
- I found no source that measured perceptual differences between one-way and two-way bridge coupling in a real-time piano.
- No 2020–2026 peer-reviewed paper was found that reports a real-time physical (FD/FE/modal-PDE) piano soundboard with two-way string coupling. Searches turned up only older work, generic modular frameworks, or offline models.

---

## 3. Sympathetic resonance (sustain pedal and held keys)

### Takeaway
Real-time methods range from cheap reverberator-like string registers (12–28 string models) to calibrated parallel-filter "string register" banks (about 1000–1500 resonators at about 4 MAC each). Bank 2010 also uses uni-directional coupling: summed main-string bridge forces are fed through a small R×R region gain matrix (R = 8) into per-string secondary resonators. This structure is stable for any coupling gains.

### Cited Findings
- **Bank 2010 sympathetic model.**
  - Bidirectional coupling is easy in digital waveguides when all strings share one termination, but stable bidirectional coupling is harder in modal synthesis.
  - They therefore use uni-directional coupling: main-string outputs are summed and fed to the secondary resonators of all strings. Energy flows only main to secondary, which makes the model "structurally stable" for any coupling parameters.
  - Coupling varies across the keyboard through R regions: F_p = B·F_b, with B an R×R gain matrix. R = 8 costs 64 extra MACs per sample.
  - The same secondary resonators also model beating and two-stage decay. Dampers or the pedal enable the undamped ones, which allows half-pedaling.
  - Source: [Bank et al. 2010](https://home.mit.bme.hu/~bank/publist/taslp10.pdf)
- **Zambon, Lehtonen & Bank, DAFx 2008, "Simulation of piano sustain-pedal effect by parallel second-order filters".**
  - The bridge force F_b feeds the soundboard filter H_sb and, in parallel, a sustain-pedal block H_ped. The total is H̃_ped = H_sb + H_sb·H_ped, so the model is feed-forward.
  - Feed-forward is justified because the board's impedance effect is in the string parameters.
  - The block is LTI, so it can also be applied to sample-based or commuted synthesis.
  - Calibration uses IRs of a **Steinway Model C grand**: bridge hit with an impulse hammer, microphones 2 m away, in a studio, measured with dampers down and with dampers up (pedal).
  - FZ-ARMA subbands are centred on equal-tempered partial frequencies f_m = 440·2^((m−49)/12). This gave about 2500 poles, pruned heuristically (poles near band edges, too-short decays masked by the soundboard, too-long decays treated as noise).
  - Least squares then fits residues against the target h_t = h̃_ped − h_sb, with h_sb as the input.
  - Final bank: K = 1500 second-order sections. Cost is about 4 MACs per resonator per sample, about 6000 MAC/sample, "remarkably higher" than reverb-based models.
  - Proposed reductions: multirate (low-order resampling filters, with errors absorbed into resonator amplitudes and phases) and SIMD.
  - Keyboard split into R regions (2–8 is practical), each with its own pedal and soundboard filters.
  - Source: [DAFx 2008 PDF](https://dafx.de/paper-archive/2008/papers/dafx08_37.pdf); [DAFx archive page](https://dafx.de/paper-archive/details/5mJ31JDQWZTHNiqjmf19mQ)
- **Earlier sustain-pedal models, as summarized in Zambon et al. 2008:**
  - De Poli et al. simulated the 18 lowest strings plus 10 variable-length strings.
  - Lehtonen et al. (ref. [1], 2007) used a reverberation algorithm with 12 simplified string models for the lowest tones.
  - Physics-based synthesizers typically use a separate set of string models for sympathetic resonance.
  - Source: [DAFx 2008](https://dafx.de/paper-archive/2008/papers/dafx08_37.pdf)
- **Perceptual and acoustic effect of the pedal (Schutz et al., AES 2008):** the sustain pedal lengthens partial decays and adds a noise floor for all notes. — [EURECOM PDF](https://eurecom.fr/publication/2478/download/cm-schuan-080517.pdf) (snippet)
- **Aalto thesis (Välimäki supervisor, Rauhala advisor):** covers digital-waveguide piano synthesis, including sustain-pedal analysis from recordings and a synthetic pedal algorithm. — [Aaltodoc record](https://aaltodoc.aalto.fi/items/48d09ac6-59d3-4aee-94e9-3f047459d92b) (snippet; the author name was not visible)

### Inferences
- **For a full physical model**, a "string bank" option is affordable in 2026: all about 230 strings (88 keys × 1–3 strings), each with tens of modes, fed by the summed bridge signal and damped individually by damper state. 230 × 40 modes × 3 MAC is about 28k MAC/sample. Using the one-way main-to-secondary scheme keeps it unconditionally stable.
- **True bidirectional coupling.** Feeding the sympathetic strings back into the bridge requires a shared bridge admittance (or a passive admittance-matrix formulation). The cited sources do not quantify the audible gain from this.
- **Calibration without samples.** Region-dependent coupling (the R×R matrix) is a cheap stand-in for bridge-position dependence. In a fully physical model, mode shapes evaluated at each string's bridge point give this coupling automatically.

### Gaps
- I could not fetch Lehtonen, Penttinen, Rauhala & Välimäki, "Analysis and modeling of piano sustain-pedal effects", JASA 122(3), 2007, beyond its summary in Zambon 2008.
- Weinreich 1977 (JASA, "Coupled piano strings") is not verified here. It is the canonical reference for unison coupling through bridge admittance (double decay, aftersound).
- I found no quantitative CPU figures for modern commercial implementations (e.g. the commercial modelled piano's "full string resonance").

---

## 4. Frame, case, rim, lid, duplex scale

### Takeaway
Little modeling literature was found. The most complete model (Chabassier et al.) treats the rim only as a rigid obstacle to sound propagation, not as a vibrating structure. Conklin's data shows that assembly (plate and strings) changes bridge mobility by about 15 dB at peaks. Duplex-scale and frame-vibration synthesis models were not found.

### Cited Findings
- In Chabassier, Duruflé & Joly (Part 2), the piano geometry is the 2D soundboard ω (thin plate) plus the rim Ω_f, "considered as an obstacle to the sound propagation". The air is the exterior 3D domain. The rim is not a vibrating structure. — [Numdam M2AN 2015/2016 PDF](https://www.numdam.org/item/10.1051/m2an/2015007.pdf)
- Conklin (1996), as reported in Ege & Boutillon: assembling strings and plate raises the first-mode frequencies and lowers mobility peaks by about 15 dB. Longitudinal (in-plane) bridge mobility in the treble drops 10–15 dB when assembled. — [ar5iv 1210.5688](https://ar5iv.arxiv.org/html/1210.5688)
- Lid: most of a grand piano's sound energy radiates up and down from the soundboard, with part reflected by the lid, so lid position changes the radiation pattern. — [Room acoustics lecture notes, arXiv 2111.01900](https://arxiv.org/pdf/2111.01900) (snippet)

### Inferences
- For real-time use, frame and case effects are probably best absorbed into:
  1. the measured or assembled bridge admittance (Conklin shows assembly matters);
  2. per-direction radiation filters (lid);
  3. at most a few extra low-Q "body" modes.
- Duplex-scale (aliquot) resonances can be modeled as extra sympathetic string segments with high-frequency modes fed from the bridge or agraffe signal. This is an engineering inference; no source was found.

### Gaps
- No source found on: iron frame vibration modeling for synthesis; rim or case radiation measurements; duplex scale acoustics; lid modeling (beyond the qualitative note above).
- Askenfelt (ed.) "Five Lectures on the Acoustics of the Piano" (KTH) and Giordano's "Physics of the Piano" (2010) were not fetched in this session.

---

## 5. Radiation: efficiency, directivity, microphone position, room

### Takeaway
Measured piano directivity datasets exist:
- Nakashima/Giron/Tohyama 1996: 2 m spherical array, 10° grid, minimum-phase directivity filters intended explicitly for synthesis and binaural simulation.
- Hamburg (Plath, Bader et al.): 1289-position nearfield array over grand soundboards, back- and forward-propagated, with 15 bridge drive points. These enable Rayleigh/equivalent-source rendering.

The vendor's public statements about the commercial modelled piano indicate a virtual soundboard shaped roughly like the cabinet, with pressure computed at each virtual mic position (ideal omni). The most practical real-time mic model is therefore one filter per mic (or per bridge region × mic), derived from a radiating-plate model.

### Cited Findings
- **Nakashima, Giron & Tohyama (ASA 131st meeting, Indianapolis, 1996), "Directivity analysis of piano sounds".**
  - Anechoic measurement on a 2 m sphere: mics every 10°, from the top of the piano down to 130° below, and every 10° in azimuth (0–350°), plus a reference mic at the centre.
  - Directional/reference power-spectrum ratios become minimum-phase directivity filters "for binaural room simulation" and "synthesizing piano tones on a computer".
  - No numbers in the abstract.
  - Source: [auditory.org abstract](https://auditory.org/asamtgs/asa96ind/3aAA/3aAA10.html)
- **Plath, Pfeifle, Koehn, Bader (ISMA 2017), "Radiation Characteristics of Grand Piano Soundboards in Different Stages of Production".**
  - Two concert grand soundboards measured at seven production stages.
  - A 105-microphone array was moved parallel to the board for 1289 positions in total.
  - Excited by an acoustic vibrator at 15 positions on the bass and main bridges, with force and acceleration sensors at the drive points and swept-sine IRs.
  - Pressure was back-propagated to the board with a minimum-energy method.
  - Radiated power and radiation efficiency are reported "after Suzuki 1986", and the p/v ratio "after Giordano 1998". The effect of ribs and bridge on the coincidence range is studied.
  - Far-field patterns of the first resonances are obtained by forward propagation. The finished piano was measured binaurally in an anechoic chamber (ILD, IPD, binaural quality index).
  - Stated goal: a real-time physical-model software tool for piano makers.
  - Source: [ISMA 2017 paper 16 PDF](https://isma2017.cirmmt.mcgill.ca/proceedings/pdf/ISMA_2017_paper_16.pdf)
- **Same group, Vienna Talk 2015:** microphone-array soundboard radiation measurement with minimum-energy back-propagation. — [Vienna Talk 2015 pap 76](https://viennatalk2015.mdw.ac.at/index.htmlpage_id=13002&pap=76.html) (snippet)
- **Treble radiation non-uniformity.** Forum Acusticum 2011 (Ege/Boutillon line, "Global and local synthetic descriptions of the piano soundboard"): above about 1.1 kHz the board behaves as rib-defined waveguides, which can make treble radiation non-uniform. — [IP Paris research portal](https://researchportal.ip-paris.fr/en/publications/global-and-local-synthetic-descriptions-of-the-piano-soundboard/) (snippet)
- **Subsonic radiation regime.** The ribbed board's subsonic regime extends over a much wider band than for a homogeneous plate. Below coincidence the radiation efficiency is low, and radiation is dominated by edge and corner effects and low-order modes. — [Boutillon & Ege, arXiv 1305.3057](https://arxiv.org/abs/1305.3057v1)
- **The commercial piano, via vendor forum statements quoted in search results:**
  - "The sound radiation model simulates a soundboard that has (approximately) the shape of the piano cabinet, and the sound pressure is measured at the place where you place the microphone". The mic is therefore an ideal omni.
  - Turning reverb off for distant mics is said to be equivalent to a perfectly directional mic pointed at the piano.
  - Source: vendor forum. The page returned 403 to me; content is known only from the search snippet.
- **Object-audio approach (MPEG-I style patents).** A piano is rendered as a spatially extended source with at least two basis signals (low and high keys) and direction-dependent processing. Radiation toward the back has less high-frequency content than toward the front; pedal noise is treated as more omnidirectional. — [US patent 12445796](https://image-ppubs.uspto.gov/dirsearch-public/print/downloadPdf/12445796)
- **Other directivity databases.** BYU built a rotating-instrument, semicircular-array system for high-resolution balloon directivity that includes a concert grand. Meyer's 1970s octave-band data is the historical directivity reference. — [BYU ETD 4004](https://scholarsarchive.byu.edu/etd/4004); [arXiv 2111.01900](https://arxiv.org/pdf/2111.01900) (snippets)
- **Chabassier et al. radiation.** Radiation is computed by full 3D acoustics (pressure plus velocity, with PML) around the soundboard and rim. Simulated pressure is evaluated at a point close to the piano and compared with recordings. — [Numdam Part 2 PDF](https://www.numdam.org/item/10.1051/m2an/2015007.pdf)

### Inferences
- **How "mic placement" can work in real time.** This is consistent with the vendor's statements but is my reconstruction, not the vendor's documented algorithm.
  - The soundboard is a modal bank, with modal velocities v_k(t) as state.
  - The pressure at mic position r is p(r,t) = Σ_k (h_k(r,·) * v_k)(t), with h_k the modal radiation transfer to r (Rayleigh integral over the mode shape, including baffle/lid image sources).
  - h_k(r) can be precomputed offline per mode and per mic, and approximated by a gain plus delay (far field) or a short filter.
  - The cost is about (number of modes) × (number of mics) MACs per sample, plus per-mic delays.
  - Because the modal filter bank already shares poles, each mic is just another set of output residues. Bank 2010 notes this explicitly for stereo outputs.
  - This gives position-dependent comb-filtering and spatial cues "for free".
- **Equivalent-source alternative.** Represent the board by a coarse grid of point sources (for example 16–64 patches) driven by modal velocities projected onto the patches. Each patch-to-mic path is a delay plus 1/r gain (plus lid image). This is cheap, handles near-field mic positions physically, and lends itself to binaural rendering by applying HRTFs per patch cluster.
- **Room.** Couple the mic signals to a separate reverb, convolution with a measured or synthetic room IR (an FDN is acceptable). The piano model's radiation filters should then be "anechoic". The vendor's remark that "reverb off ≈ directional mic" shows that commercial systems treat room and source separately.
- **Calibration data for directivity.** Nakashima-type minimum-phase filters, or the Hamburg array data if it can be obtained, could tune per-direction EQ for the far field.

### Gaps
- No primary source found for the following:
  - Bork's grand piano directivity measurements (PTB).
  - Meyer's piano-specific data in "Acoustics and the Performance of Music" (5th ed., Springer 2009); not fetched.
  - Suzuki 1986 radiation efficiency values.
- The Acta Acustica 2025 open-access article (aacus240005), which appeared in a directivity search, returned 403. Its relevance to piano could not be checked.
- No published real-time piano model was found that documents a Rayleigh-integral or equivalent-source multi-mic renderer. The commercial piano's internals are proprietary.

---

## 6. Full-piano time-domain simulations (Chabassier, Chaigne, Joly): cost and transferable lessons

### Takeaway
The Chabassier/Chaigne/Joly/Duruflé model is the most complete piano simulation [offline]:
- nonlinear stiff prestressed strings (Timoshenko-type, with geometric nonlinearity and longitudinal waves);
- a nonlinear hammer;
- an orthotropic Reissner–Mindlin soundboard with ribs and bridge as heterogeneities;
- a 3D air domain with the rim as an obstacle.

It costs **24 hours on 300 processors per second of sound** (Δt = 1 µs). The soundboard is solved in a modal basis of **M = 2400 modes up to about 10 kHz**. The modal soundboard representation and energy-based (passive) coupling transfer directly to real time; the 3D air FEM does not.

### Cited Findings
- **Part 1 (ESAIM: M2AN 48(5), 2014, pp. 1241–1278), "Time domain simulation of a piano. Part 1: model description".** Aim: explain the piano's vibratory and acoustical behaviour including the main elements of sound production. — [Numdam Part 1](https://numdam.org/articles/10.1051/m2an/2013136/)
- **JASA 2013, "Modeling and simulation of a grand piano"** (Chabassier, Chaigne, Joly). A time-domain global model; the string includes internal losses, stiffness and geometric nonlinearity. — [POEMS publications 2013](https://poems.ensta.ip-paris.fr/en/publications?annee=2013) (snippet)
- **Part 2 (ESAIM: M2AN 50(1), 2016 (online Nov 2015), pp. 93–133), Chabassier, Duruflé & Joly, "numerical aspects".** All from [Numdam Part 2 PDF](https://www.numdam.org/item/10.1051/m2an/2015007.pdf):
  - The geometry is a **Steinway D**, about 3 m × 2 m, the same piano measured in their ref. [13].
  - The soundboard is a Reissner–Mindlin plate with variable density, thickness and stiffness tensors. Orthotropy and ribs are spatial heterogeneities.
  - The soundboard is solved by modal decomposition: **M = 2400 modes, chosen to cover eigenfrequencies 0–10 kHz**, with exact-in-time modal integration and a damping function f_d(λ) = αλ + β√λ + γ (frequency-dependent modal damping).
  - Strings have about 800 geometric DOFs (about 2400 unknowns). The soundboard mesh has 141,329 geometric DOFs (423,987 unknowns).
  - Air: 269,532 hexahedra without PML, about 17 M pressure unknowns and about 101 M velocity unknowns. The acoustic domain exceeds 90 M DOFs, so an explicit interleaved leap-frog scheme is used.
  - Time step Δt = 10⁻⁶ s, θ-scheme with θ = 1/4. Stability is ensured by discrete energy methods.
  - **"24 h of real time on 300 processors are needed to provide the piano sound generation during one second."**
  - Results are compared qualitatively with recordings and show energy transfers between hammer, string, soundboard and air. They claim the first comprehensive piano simulation of this realism.

### Inferences
- **Slowdown factor.** The cost is about 300 × 86,400 ≈ 2.6×10⁷ core-seconds per second of audio, about 10⁷ times slower than real time. The air domain (over 10⁸ unknowns) dominates.
- **Lessons that transfer to real time:**
  1. A modal soundboard (2400 modes to 10 kHz for a concert grand) is the right representation. Real-time could use a few hundred to 2400 modes at about 3–4 MAC each, so 2400 modes is about 10k MAC/sample, cheap.
  2. A frequency-dependent modal damping law f_d(λ) gives a compact parameterization.
  3. Energy-consistent (passive) coupling at the bridge guarantees stability of two-way coupling.
  4. The 3D air should be replaced by precomputed modal radiation transfer functions to the mic or listener points (see §5).
  5. The string side (about 800 DOFs per string with Δt = 1 µs) is the other heavy part. Real-time systems use modal or waveguide strings instead.
- These mode counts, and an offline modal analysis of a Reissner–Mindlin orthotropic ribbed plate (FEniCS or similar), can provide a "physical" modal soundboard for a Rust engine without samples.

### Gaps
- Exact per-subsystem cost breakdown (strings vs soundboard vs air) was not found in the extracted text.
- Juliette Chabassier's later (2023) piano work page was found but not read: [Chabassier 2023 piano focus](http://juliettechabassier.perso.math.cnrs.fr/public/focus/2023/piano/).

---

## 7. 2018–2026 work: neural/differentiable models, modal estimation from recordings, soundboard identification

### Takeaway
Recent differentiable piano models (Renault, Mignot & Roebel, DAFx 2022; DDSP-Piano thesis 2024; STN 2024) model inharmonicity, detuning and polyphony, but treat the body as learned filters or reverb rather than a physical soundboard. Listening tests still rank physical models higher. For estimating a soundboard model from recordings, the proven toolchain is still high-resolution pole estimation (FZ-ARMA, or ESPRIT-type as used by Ege) followed by least-squares residue fitting into a parallel second-order bank. I found no 2024–2026 paper that identifies a physical soundboard model from recordings.

### Cited Findings
- **Renault, Mignot & Roebel, "Differentiable Piano Model for MIDI-to-Audio Performance Synthesis" (DAFx 2020-in-22, Vienna).**
  - The first polyphonic DDSP piano, conditioned on MIDI.
  - Explicit inharmonicity from the tuning model, detuning for partial interactions, and an "extended pitch" for string resonance after note release.
  - It beat a neural benchmark with far fewer parameters and less training data, but physical-modeling approaches still sounded better in the listening test.
  - Sources: [DAFx paper PDF](https://dafx2020.mdw.ac.at/Proceedings/papers/DAFx20in22_paper_48.pdf); [DAFx archive](https://dafx.de/paper-archive/details/_5jvqdya3I0yV4ZgRQWTPw); [DDSP review arXiv 2308.15422](https://arxiv.org/pdf/2308.15422)
- **Renault PhD thesis (IRCAM / Sorbonne, 2024), "Neural audio synthesis of realistic piano performances" (DDSP-Piano):** includes instrument knowledge such as inharmonicity, tuning and polyphony. — [HAL tel-04732963](https://hal.univ-lille.fr/IRCAM/tel-04732963v1); [IRCAM media](https://medias.ircam.fr/en/media/0532fe660802b06c40a0)
- **"Sines, Transient, Noise Neural Modeling of Piano Notes" (arXiv 2409.06513, 2024):** a differentiable sinusoidal model guided by physics-derived formulas, with parameters estimated automatically from recordings. No physical soundboard. — [arXiv 2409.06513](https://arxiv.org/html/2409.06513)
- **Hobbyist gradient-descent fit of a model in the style of the commercial modelled piano to MAESTRO Disklavier data.** Strike position, hammer spectrum, soundboard EQ and room IR trade off against each other (identifiability problem); the author suggests priors and smoothness regularisation. Not peer-reviewed. — [GitHub VladPetk/nn_physical_model_piano](https://github.com/VladPetk/nn_physical_model_piano)
- **Ege et al.:** the soundboard modal identification up to 3 kHz used a "high-resolution modal analysis technique" (Ege et al., JSV 2009) that gives modal damping in the mid-frequency range where FFT methods fail. — [arXiv 1212.2323](https://arxiv.org/abs/1212.2323)
- **Zambon/Lehtonen/Bank 2008:** the FZ-ARMA plus least-squares two-step pipeline turned measured soundboard and string-register IRs into a 1500-section real-time filter. — [DAFx 2008](https://dafx.de/paper-archive/2008/papers/dafx08_37.pdf)
- **Bensa et al. (JASA 118(1), 2005), "Parameter fitting for piano sound synthesis by physical modeling":** fits hammer–string parameters with a perceptual criterion; it has no soundboard. — [JASA](https://pubs.aip.org/asa/jasa/article/118/1/495/540250/Parameter-fitting-for-piano-sound-synthesis-by); [HAL hal-00105865](https://hal.science/hal-00105865/document) (snippets)

### Inferences
- **Recipe to estimate a soundboard model from samples (analysis only):**
  1. Take release tails or low-velocity staccato notes per key region, or better, a hammer tap on the bridge if access exists.
  2. Deconvolve or isolate the body part. Partials of the struck string are known (inharmonic series), so they can be masked or subtracted.
  3. Run ESPRIT or FZ-ARMA in subbands to get body poles (frequency, damping).
  4. Fit residues per key region and per output channel by least squares, as in Bank's parallel filter.
  5. Optionally regularize with the Ege–Boutillon statistics (n(f), η ≈ 0.02, mean mobility).

  Because the board is linear (−40 dB nonlinearity), LTI identification is justified.
- **Identifiability warning.** Body EQ, hammer spectrum, strike point and room IR are confounded in single-microphone recordings. Recordings with known mic positions or multiple mics help separate radiation from board modes.

### Gaps
- I found no 2018–2026 JASA, Acta Acustica or DAFx paper on a new real-time physical piano soundboard or body model. Searches returned only the 2010 Bank et al. paper as the RT reference, plus generic modal-synthesis tools (2019 modular framework, 2026 nlm). Commercial systems (the commercial modelled piano, Roland V-Piano, Kawai, Yamaha) are not documented in the literature.
- Badeau and ESPRIT applied specifically to piano soundboards was not verified here. The Ege high-resolution method (JSV 2009) is likely ESPRIT-based, but this was not confirmed from text.
- Not verified: whether Chaigne, Cotté & Viggiano (JASA 2013) provide closed-form parametric soundboard models (mode frequencies and damping vs geometry) usable for synthesis.
