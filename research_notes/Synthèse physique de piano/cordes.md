# Physical modeling of piano strings for real-time synthesis (state of the art to 2026)

Scope: string models only (transverse, longitudinal, coupling, parameters), judged by whether they can run polyphonically in real time on a CPU. Hammer and soundboard appear only where they affect the string model. Every claim has a source. Throughout, "RT" means real time was demonstrated in the cited source and "offline" means it was not. Research date: 2026-10-21.

---

## 1. Comparison of approaches: digital waveguide, finite difference, modal, energy-conserving (SAV/IEQ). Cost and what each captures

### Takeaway
For a full 88-key polyphonic CPU piano, the approaches with real-time evidence are the **digital waveguide** (DWG: delay line plus lumped loss and dispersion filters, the cheapest) and **modal synthesis** (a bank of 2nd-order resonators per string). Bank, Zambon & Fontana (2010) ran a complete modal piano with longitudinal and nonlinear effects and sympathetic resonance at full polyphony, about 10,000 resonators at ~30% of a 2.4 GHz Core 2 Duo. Full FD/FEM energy-conserving models (Chaigne–Askenfelt, Chabassier–Joly) are offline reference models. The newer **explicit SAV/energy-quadratisation** schemes (Bilbao, Ducceschi, Risse, Hélie, 2022–2025) put nonlinear stiff FD strings at a few percent CPU per string, but they have only been shown on uncoupled strings without a hammer or bridge.

### Cited Findings

**General / overview**
- Bank & Chabassier, "Model-Based Digital Pianos: From Physics to Sound Synthesis", IEEE Signal Processing Magazine 36(1):103–114, Jan 2019, DOI 10.1109/MSP.2018.2872349. The pipeline is physics, then an accurate numerical model, then an optimized signal-processing model that runs in real time. Accurate models serve physicists and makers; efficient RT models serve musicians. — [bibbase entry](https://bibbase.org/network/publication/bank-chabassier-modelbaseddigitalpianosfromphysicstosoundsynthesis-2019) (full text not retrieved)
- History given by Bank et al. 2010: Garnett's DWG piano (1987); Smith & Van Duyne commuted piano (1995); Borin et al., the first complete RT piano model (1997, Padova/Generalmusic); Bank's MSc model (2000) with an RT version in 2002; Bensa et al. on coupled strings and parametric excitation; Rauhala et al. on parameter estimation and parametric implementations. Commercial products: the commercial modelled piano (2006) and Roland V-Piano (2009), the first digital piano with physical modeling. — [Bank, Zambon, Fontana, "A Modal-Based Real-Time Piano Synthesizer", IEEE TASLP 18(4):809–821, 2010](https://home.mit.bme.hu/~bank/publist/taslp10.pdf)
- Renault et al. (2022) state that the commercial piano "relies on modal synthesis". — [Renault, Mignot, Roebel, DAFx 2022](https://dafx.de/paper-archive/2022/papers/DAFx20in22_paper_48.pdf)
- A 2026 review: Tablas de Paula, Smith, Välimäki, Reiss, "Four Decades of Digital Waveguides", J. Audio Eng. Soc. 74(7–8):464–484 (2026), arXiv 2604.12878. It lists DWG piano references (Smith & Van Duyne 1995 commuted piano; Bank ICMC nonlinear interaction in DWG; Bensa et al. JNMR 2001 coupled strings; Rauhala et al. CMJ 2008 parametric piano) and covers ML/DDSP-based waveguide parameter optimization. — [arXiv abs](https://arxiv.org/abs/2604.12878)

**Common PDE (linear stiff lossy string)**
- Bank 2010, eq. (1): μ y_tt = T0 y_xx − E S κ² y_xxxx − 2Rμ y_t + d_y(x,t). T0 is tension, μ linear mass, E Young's modulus, S cross-section, κ radius of gyration, R frictional resistance (made per-mode R_k for frequency-dependent loss). — [Bank et al. 2010](https://home.mit.bme.hu/~bank/publist/taslp10.pdf)
- The variant common in FD work (Bensa/Bilbao type), as written in Risse et al. 2025: μ u_tt = (T ∂x² − EI ∂x⁴)u − 2μ(η0 − η1 ∂x²) u_t + nonlinear terms + f_in δ(x − x_in), with simply supported ends (u = u_xx = 0). η0 sets frequency-independent damping and η1 frequency-dependent damping. The two are computed from T60 at two frequencies using formulas in Bilbao, *Numerical Sound Synthesis*, ch. 7. — [Risse, Hélie, Bilbao, DAFx25 paper 24](https://dafx25.dii.univpm.it/wp-content/uploads/2025/09/DAFx25_paper_24.pdf)
- Chaigne & Askenfelt (1994, JASA 95(2):1112–1118) used a wave equation plus a stiffness term (ε, scaled by string length), two damping terms (b1: first-order velocity term; b3: third time-derivative term giving frequency-dependent loss), and a hammer force term, discretized by explicit FD. — [search summary of Bilbao/Smith notes and the paper](https://ccrma.stanford.edu/~jos/PianoString/Piano_String_Modeling.html) (parameter tables not retrieved, see Gaps)
- Bensa, Bilbao, Kronland-Martinet, Smith, "The simulation of piano string vibration: From physical models to finite difference schemes and digital waveguides", JASA 114(2):1095–1107 (2003). It uses a transverse model, second order in time, with frequency-dependent loss and dispersion. Parameters are fitted from measurements across a grand piano's range and linked to the responses of DWG loss and dispersion filters. — [JOS bibliography](https://ccrma.stanford.edu/~jos/pasp05/Bibliography.html), [Smith academia page](https://stanford.academia.edu/SmithJulius)

**Digital waveguide (DWG)**
- A DWG discretizes the traveling-wave solution. Losses and dispersion are lumped into one filter, so a string is a delay line plus a low-order IIR in a feedback loop. Most piano synthesis models (Garnett, Borin, Bank 2000, Bensa, Rauhala) are DWG "due to its efficiency". — [Bank et al. 2010](https://home.mit.bme.hu/~bank/publist/taslp10.pdf)
- **Main limitation for nonlinearity:** with a lumped dispersion filter, the modal shapes of the model become non-orthogonal, so precise nonlinear (longitudinal/tension) modeling is not possible in the efficient DWG form. The options are to distribute the dispersion filter, which raises cost a lot, or to use less physical tricks. — [Bank et al. 2010](https://home.mit.bme.hu/~bank/publist/taslp10.pdf)
- **Commuted synthesis** (Smith 1993; Smith & Van Duyne 1995) swaps hammer, string and soundboard so that the soundboard impulse response becomes a wavetable excitation. It is very cheap but assumes LTI behavior, so restrikes of a sounding string and similar effects are not handled correctly. — [Bank et al. 2010](https://home.mit.bme.hu/~bank/publist/taslp10.pdf); [Four Decades of DWG, 2026](https://arxiv.org/abs/2604.12878)

**Finite differences (FD)**
- FD is well suited to complex structures such as soundboards. For the 1D linear string the closed-form solution is known, and discretizing that solution (DWG or modal) is much cheaper than discretizing the PDE. — [Bank et al. 2010](https://home.mit.bme.hu/~bank/publist/taslp10.pdf)
- An FD string with longitudinal motion needs very high sample rates (e.g. fs = 500 kHz) to stay stable, because longitudinal waves travel fast. On a 2004 PC that meant monophony. Bank & Sujbert's reduced model (FD transverse plus resonators for longitudinal) costs 10–20 times a DWG string and allows 5–10-note real-time polyphony in C++ on an average 2004 PC, at about 10% of the cost of the full FD model with the same perceived quality. — [Bank & Sujbert, "A Piano Model Including Longitudinal String Vibrations", DAFx-04](https://dafx.de/paper-archive/2004/P_089.PDF)
- OpenPiano is an open-source real-time piano engine and JUCE plugin built on finite differences; its roadmap mentions moving to modal analysis. — [GitHub michele-perrone/OpenPiano](https://github.com/michele-perrone/OpenPiano) (no performance figures retrieved)

**Modal synthesis** (Bank, Zambon, Fontana 2010; RT)
- Hinged ends, y(x,t) = Σ y_k(t) sin(kπx/L). Each mode obeys ÿ_k + a1,k ẏ_k + a0,k y_k = b0,k F_y,k(t), with a1,k = 2R_k, a0,k = (T0/μ)(kπ/L)² + (ESκ²/μ)(kπ/L)⁴, b0,k = 2/(Lμ), F_y,k = ∫ sin(kπx/L) d_y dx. — [Bank et al. 2010](https://home.mit.bme.hu/~bank/publist/taslp10.pdf)
- f_k ≈ f0 k √(1 + B k²), with f0 = (1/2L)√(T0/μ) and B = κ² (ES/T0)(π/L)². — [Bank et al. 2010](https://home.mit.bme.hu/~bank/publist/taslp10.pdf)
- Loss: R_k = 1/τ_k = b1 + b3 (2πf_k)², a second-order function and the same loss type as the Chaigne–Askenfelt FD model and the one-pole DWG loss filter. R_k can be set freely per mode from analysis. (The exponent was lost in text extraction; the paper calls it "second-order".) — [Bank et al. 2010](https://home.mit.bme.hu/~bank/publist/taslp10.pdf)
- Discretization uses the impulse-invariant transform: each mode becomes a 2-pole filter plus a one-sample delay. The leading zero in the displacement-from-force response removes the delay-free loop with the hammer, so there is no ys[n] ≈ ys[n−1] hack. That hack can go unstable at high impact velocities. — [Bank et al. 2010](https://home.mit.bme.hu/~bank/publist/taslp10.pdf)
- Bridge force F_b = (T0π/L) Σ k y_k(t), a weighted sum of modal amplitudes. Input and output weights can be lumped so that w_b,k = 1 and the bridge force is a plain sum. — [Bank et al. 2010](https://home.mit.bme.hu/~bank/publist/taslp10.pdf)
- Advantages: a fully parallel, SIMD-friendly structure (Intel SSE gave nearly 4× speedup in single precision); every partial's frequency and decay is independently settable, which makes estimation simple and run-time edits easy; nonlinear coupling is straightforward; aliasing is easy to avoid because the modal frequencies are known. Cost is "probably higher" than DWG piano models without nonlinearity. — [Bank et al. 2010](https://home.mit.bme.hu/~bank/publist/taslp10.pdf)
- Cost: 3 multiplications per output sample (MPOS) per resonator with nonzero input, 2 MPOS when free-running. — [Bank et al. 2010](https://home.mit.bme.hu/~bank/publist/taslp10.pdf)
- Modal semi-discretization of a stiff string runs faster than real time for typical musical strings (bowed-string study). — [Efficient simulation of the bowed string in modal form, DAFx 2022](https://www.mdphys.org/PDF/dafx_2022.pdf) (search summary only)

**Energy-conserving / passive schemes (offline reference models)**
- Chabassier, Chaigne, Joly, "Modeling and simulation of a grand piano", JASA 134:648 (2013). The string includes internal losses, stiffness and geometric nonlinearity (with longitudinal coupling). The hammer is a nonlinear dissipative compression force, the soundboard a dissipative orthotropic Reissner–Mindlin plate, and radiation uses PML. Discretization uses original energy-preserving schemes. — [HAL hal-00873089](https://hal-ensta-paris.archives-ouvertes.fr/hal-00873089) (abstract via search)
- "Time domain simulation of a piano", Part 1 (model; ESAIM M2AN 48(5), 2014, Chabassier, Chaigne, Joly) and Part 2 (numerics; ESAIM M2AN 50(1):93, 2016, Chabassier, Duruflé, Joly): "the main difficulty is time discretization and stability is achieved via energy methods". — [Part 1](https://www.numdam.org/item/M2AN_2014__48_5_1241_0), [Part 2](https://www.numdam.org/articles/10.1051/m2an/2015007/)
- Chabassier PhD thesis, "Modélisation et simulation numérique d'un piano par modèles physiques", École Polytechnique 2012, advisor P. Joly. — [Math Genealogy](https://mathgenealogy.org/id.php?id=182924)
- Chabassier & Joly (2010) give a general energy-preserving, second-order accurate, fully implicit scheme for nonlinear Hamiltonian wave systems, with the piano string as the example. It is iterative, hence costly. — [IP Paris research portal](https://researchportal.ip-paris.fr/en/publications/energy-preserving-schemes-for-nonlinear-hamiltonian-systems-of-wa/)
- Castera & Chabassier (ESAIM M2AN, ~2024) give a general nonlinear energy-quadratization approach from the Euler–Lagrange form. It is linearly implicit with a discrete power balance and quadratic convergence in space and time, applied to a flexible rotating hammer shank. A "numerical analysis of quadratized schemes, application to the nonlinear piano string" by Castera & Chabassier is cited as ref. [20] in DAFx25. — [M2AN pdf](https://www.esaim-m2an.org/articles/m2an/pdf/forth/m2an230209.pdf); [DAFx25](https://dafx25.dii.univpm.it/wp-content/uploads/2025/09/DAFx25_paper_24.pdf)

**Energy quadratisation / IEQ / SAV (moving toward RT)**
- Ducceschi & Bilbao, ICA 2019: a non-iterative conservative FD scheme for the geometrically exact string from quadratising the nonlinear potential. It needs one **sparse linear solve per time step** and is about **half an order of magnitude faster** than iterative (Newton, 10 iterations/step) schemes. Tests in Matlab on a 3.6 GHz i5 (2011 iMac), with high-amplitude cases at fs = 48·10⁴ Hz (480 kHz) and above. Offline. — [ICA 2019 paper](https://www.pub.dega-akustik.de/ICA2019/data/articles/001028.pdf)
- Ducceschi & Bilbao, "Simulation of the geometrically exact nonlinear string via energy quadratisation", J. Sound Vib. 534 (2022). A linearly implicit scheme whose energy is a sum of quadratic terms only, using an auxiliary variable and a **mixed FD/modal scheme** for the transverse and longitudinal directions. — [Edinburgh research portal](https://www.research.ed.ac.uk/en/publications/simulation-of-the-geometrically-exact-nonlinear-string-via-energy/)
- Ducceschi, Bilbao, Webb (ENOC 2022): a struck piano string with geometrically exact nonlinearity using a scalar quadratic energy method, aimed at real time. — [search result, cris.unibo.it](https://cris.unibo.it/handle/11585/883682) (full text not accessible)
- IEQ adds a distributed variable (square root of the energy density). SAV adds a single scalar (square root of total nonlinear energy). Both make the energy quadratic, but they stay linearly implicit. Bilbao, Ducceschi & Zama (J. Comp. Phys.) made **fully explicit**, exactly energy-conserving schemes using time-interleaved variables and Sherman–Morrison, so no linear solve: a stabilized Störmer–Verlet. — [Risse et al., DAFx25](https://dafx25.dii.univpm.it/wp-content/uploads/2025/09/DAFx25_paper_24.pdf)
- **Explicit SAV drift problem:** at audio rates the auxiliary variable drifts (relative error up to 10⁴ in their test), giving audible pitch bends and HF noise after repeated strikes. Risse, Hélie & Bilbao (DAFx 2025) add a power-balanced control term (λ0 ≈ 1000 s⁻¹ works; divergence of the control seen only when λ0 ≥ sample rate). — [DAFx25](https://dafx25.dii.univpm.it/wp-content/uploads/2025/09/DAFx25_paper_24.pdf)
- Related: Russo, Bilbao, Ducceschi, "Scalar auxiliary variable techniques for nonlinear transverse string vibration", IFAC-PapersOnLine 58(6):160–165 (2024). Van Walstijn, Chatziioannou, Bhanuprakash on implicit and explicit energy-stable string and collision schemes. — [refs in DAFx25](https://dafx25.dii.univpm.it/wp-content/uploads/2025/09/DAFx25_paper_24.pdf)

### Inferences
- In Rust, a DWG string (delay line, fractional delay, 1-pole loss, dispersion allpass cascade) is cheapest per voice. A modal bank is more flexible and is the one CPU-proven route that includes nonlinear longitudinal effects and sympathetic resonance. A SIMD modal bank is f32x8-friendly: each resonator is an independent biquad-like recurrence.
- Explicit SAV FD is the most promising "true PDE" option for RT. It still has these limits: a stability (CFL) condition tied to the parameters, drift that needs control, and no published full-piano integration (hammer plus coupled unison plus bridge).
- Implicit energy-preserving schemes (Chabassier) give stability guarantees and serve as validation references, but they are not RT candidates.

### Gaps
- No computation-time figures for Chabassier et al. (2013/2014/2016) were retrieved; HAL was behind bot protection. Their cost is not quantified here; treat the models as offline.
- The Bank & Chabassier 2019 SPM full text was not retrieved, so any CPU numbers it reports are not included.
- ENOC 2022 (Ducceschi/Bilbao/Webb) real-time figures were not accessible (HTTP 403).
- No independently published CPU figures for the commercial modelled piano or Roland V-Piano were found.

---

## 2. Stiffness and inharmonicity (B), frequency-dependent losses, and parameter fitting

### Takeaway
Inharmonicity follows f_k = k f0 √(1 + B k²), with B = κ²(ES/T0)(π/L)² (Fletcher 1964). Across the keyboard, B is well described by Rigaud et al.'s 2-parameter, sum-of-two-exponentials model in MIDI pitch: about 5×10⁻⁴ at A0, a minimum of about 3×10⁻⁴ around the middle, rising to ~10⁻² in the top octave. Losses are modeled per mode as 1/τ_k = b1 + b3·ω_k² (modal/Chaigne–Askenfelt) or by (η0, η1) in the PDE (Bensa/Bilbao), fitted from two T60 values or per-partial analysis. In DWG they become loss filters plus allpass dispersion cascades. Several closed-form or robust design methods exist (Rauhala–Välimäki 2006; Abel–Smith 2006; Abel–Välimäki–Smith 2010).

### Cited Findings
- f_k ≈ f0 k √(1 + B k²); B = κ² (ES/T0)(π/L)². — [Bank et al. 2010](https://home.mit.bme.hu/~bank/publist/taslp10.pdf)
- Steel-string form of Fletcher's formula: B = 3.95×10¹⁰ · d²/(l⁴ f0²), with d the diameter and l the length. F and B can be obtained from accurate measurement of any two mode frequencies. Typical bass-tone B is 10⁻⁴ to 10⁻³. — [Investigating the Inharmonicity of Piano Strings, Edinburgh student journal](https://concept.lib.ed.ac.uk/esjs/article/view/9815) (secondary source)
- **Keyboard-wide B model** (Rigaud, David, Daudet, "A parametric model and estimation techniques for the inharmonicity and tuning of the piano", JASA 133(5), 2013, pp. 3107–3118; page range differs between listings): 2 parameters for B along the keyboard, which can be estimated from a few bass notes, and 4 parameters for the tuning (f0) curve that account for inharmonicity. Works on isolated notes or chords with known notes. — [HAL hal-00856735](https://hal-imt.archives-ouvertes.fr/hal-00856735), [DAFx 2011 version](https://dafx.de/paper-archive/details/BMnL7U-ZvEJmUDGh4yeqUg)
- The explicit form as used in DDSP-Piano: B(m) = exp(αT·m + βT) + exp(αB·m + βB + γB·b_i), with m the (MIDI) pitch. Initialized from Rigaud's estimates: **αB = −0.0847, βB = −5.82, αT = 0.0926, βT = −13.64**. The treble asymptote is "the same for all pianos", so the instrument-specific modifier b_i only shifts the bass asymptote. — [Renault, Mignot, Roebel, DAFx 2022](https://dafx.de/paper-archive/2022/papers/DAFx20in22_paper_48.pdf)
- A learned DNN in place of this explicit B model ("Deep Inharmonicity" variant) scored lower in the listening test, which perceptually confirms the explicit physics-based model. — [Renault et al. 2022](https://dafx.de/paper-archive/2022/papers/DAFx20in22_paper_48.pdf)
- Rauhala, Lehtonen, Välimäki, "Fast automatic inharmonicity estimation algorithm", JASA 121(5):EL184–EL189 (2007): accurate B estimation at much lower cost than earlier methods. — [academia.edu](https://www.academia.edu/22085383/Fast_automatic_inharmonicity_estimation_algorithm)
- Simionato, Fasciani, Holm (Frontiers Signal Proc., 2024): a learned network for B(f0) on a Yamaha Disklavier (C3–B4) gave mean B = 8.63×10⁻³, which the authors call "slightly larger" than Rigaud et al. 2011. — [Frontiers 10.3389/frsip.2023.1276748](https://www.frontiersin.org/journals/signal-processing/articles/10.3389/frsip.2023.1276748/full). This seems high for mid-range notes (the Rigaud curve gives about 3×10⁻⁴ near C4), and the paper's cited literature range "0.0002 to 0.4" looks wrong at the top end. Treat with caution.
- A non-peer-reviewed table computed from Fletcher's formula with the Chabassier & Duruflé (INRIA RT-0425, 2012) Steinway D string dimensions gives B(A0) ≈ 2.85×10⁻⁴ and B(A4) ≈ 7.05×10⁻⁴. — (vendor forum; calculation, not measurement)
- **Loss fitting:** in the modal model R_k = 1/τ_k = b1 + b3(2πf_k)², with example b1/b3 values in Chaigne & Askenfelt 1994; or arbitrary per-mode τ_k from analysis of real tones. — [Bank et al. 2010](https://home.mit.bme.hu/~bank/publist/taslp10.pdf)
- In the FD/SAV model, η0 and η1 come from T60 at two frequencies (Bilbao NSS ch. 7). η0 can be modulated at run time without affecting stability; the other parameters enter the stability condition. — [Risse et al. DAFx25](https://dafx25.dii.univpm.it/wp-content/uploads/2025/09/DAFx25_paper_24.pdf)
- **DWG dispersion filter design:**
  - Rocchesso & Scalcon (1996) used least-squares allpass design (Lang–Laakso) with Bark warping and tuned "the first several tens of partials" of real piano strings with total allpass order ≤ 20. Lang (1998) used L∞ design, and Bensa et al. (2004) used it to calibrate a series of allpass sections. — [JOS PASP, Dispersion Filter Design](https://ccrma.stanford.edu/~jos/pasp/Dispersion_Filter_Design.html)
  - Rauhala & Välimäki, "Tunable dispersion filter design for piano synthesis", IEEE SPL 13(5):253–256 (2006): closed-form, dynamically tunable design based on Thiran maximally flat group delay, applied to a **cascade of identical first-order allpasses**, with an extension to an arbitrary number of sections (DAFx 2006) and use in a waveguide piano. — [DAFx archive](https://dafx.de/paper-archive/details/n4Qv1ScD_VbmqShsq0ZVQQ); [JOS PASP](https://ccrma.stanford.edu/~jos/pasp/Dispersion_Filter_Design.html)
  - Abel & Smith, "Robust design of very high-order allpass dispersion filters", DAFx 2006: a nonparametric method matching a desired group delay. It gives a numerically robust cascade of biquads whose poles follow a smooth loop inside the unit circle, with examples including a piano string group delay. Perceptually weighted nonparametric methods can beat optimal parametric ones on cost/performance. — [DAFx archive](https://dafx.de/paper-archive/details/FxpIAYIVGkUKoqVYzYCHQg); [JOS PASP](https://ccrma.stanford.edu/~jos/pasp/Dispersion_Filter_Design.html)
  - Abel, Välimäki & Smith, IEEE SPL 2010: an improved closed-form design that places poles from the group-delay area function and outperforms the earlier closed form. MATLAB `adf.m` is on the CCRMA wiki. — [CCRMA wiki](https://ccrma.stanford.edu/wiki/Dispersion_Filter_Design)
- **DWG loss filters:** "Robust Loss Filter Design for Digital Waveguide Synthesis of String Tones", IEEE SPL 10(1):18–20 (2003); Lehtonen, Rauhala, Välimäki, "Sparse Multi-Stage Loss Filter Design for Waveguide Piano Synthesis", IEEE WASPAA 2005, pp. 331–334. — [refs in Four Decades of DWG, 2026](https://arxiv.org/abs/2604.12878)

### Inferences
- With the Rigaud/Renault parameters, B(m) evaluates to about 5.1×10⁻⁴ at A0 (m = 21), about 3.3×10⁻⁴ at C4 (m = 60) and about 2.6×10⁻² at C8 (m = 108). This is computed here, not quoted. It is a good default for a Rust parameter generator, refined per instrument from string-scale data (L, d, T) through B = κ²(ES/T0)(π/L)².
- For a modal engine, B goes straight into the per-mode frequencies and no dispersion filter design is needed. That is a strong reason to prefer modal synthesis when B must vary live (tuning, "string length" edits).

### Gaps
- Conklin's measured B tables (JASA 1996, "Piano strings and scale design") were not retrieved. No primary measured B table across all 88 keys was obtained.
- Exact Chaigne & Askenfelt 1994 Table I values (b1, b3, ε, N, fs for C2/C4/C7) could not be retrieved: the TLS error and connection refusal blocked the PDF.
- Bensa 2003 numerical loss parameters were not retrieved.
- No source gave the dispersion allpass order needed per register for perceptual transparency, beyond Rocchesso & Scalcon's "≤ 20 for tens of partials". Järveläinen et al. (1999) on audibility of inharmonicity is referenced in JOS PASP but not read.

---

## 3. Geometric nonlinearity, longitudinal vibrations, phantom partials, tension modulation

### Takeaway
At forte, transverse motion modulates the tension. That drives longitudinal modes: free longitudinal modes at about 16–20× f0 transverse, plus forced "phantom partials" at sums and differences of transverse mode frequencies. The effect matters perceptually mainly in the bass ("metallic" character). The cheapest physically grounded RT approach (Bank et al. 2010) uses one-way transverse-to-longitudinal coupling: spatially uniform tension as the squared sum of k·y_k, plus correction by fewer than 10 longitudinal resonators driven by pairwise products of modal amplitudes, with aliasing avoided by dropping products above Nyquist. In that model it costs about 25% of total CPU. Some experimental work (2018) suggests much of the phantom-partial power is generated in the wooden structure rather than the string.

### Cited Findings
- Longitudinal equation (Bank 2010, eq. 17): the same form as the transverse equation with ES in place of T0, no dispersion term, and forcing from the transverse slope (d_ξ ∝ ∂/∂x[(∂y/∂x)²]). — [Bank et al. 2010](https://home.mit.bme.hu/~bank/publist/taslp10.pdf)
- Longitudinal fundamental ≈ 16–20 × the transverse fundamental in piano strings. — [Bank et al. 2010](https://home.mit.bme.hu/~bank/publist/taslp10.pdf)
- Longitudinal mode k is excited only by transverse mode pairs (m, n) with m + n = k or |m − n| = k, so it is excited at the sum and difference frequencies. Odd longitudinal modes get an "odd-like" spectrum and even modes an "even-like" one. Example: G1 string, first longitudinal mode about 690 Hz, second about 1380 Hz. — [Bank et al. 2010](https://home.mit.bme.hu/~bank/publist/taslp10.pdf)
- "Longitudinal modal peaks and phantom partials are the free and the forced response of the same system." — [Bank et al. 2010](https://home.mit.bme.hu/~bank/publist/taslp10.pdf)
- Bank & Sujbert, "Generation of longitudinal vibrations in piano strings: From physics to sound synthesis", JASA 117(4):2268–2278 (2005): a modal model with one-way transverse-to-longitudinal coupling. The main source of phantom partials is that coupling; termination effects and longitudinal-to-transverse coupling have only small influence. The model was simplified for RT synthesis. — [Bank sound-example page](https://home.mit.bme.hu/~bank/publist/jasa-longitud/index.html) (abstract via search)
- Even phantoms sit at 2× a transverse mode frequency, odd phantoms at f_m + f_n or f_m − f_n (Conklin's naming). Conklin, "Generation of partials due to nonlinear mixing in a stringed instrument", JASA 105:536–545 (1999), coined "phantom partials". — [JOS PASP, Nonlinear Piano Strings](https://ccrma.stanford.edu/~jos/pasp/Nonlinear_Piano_Strings.html); [Bank 2010](https://home.mit.bme.hu/~bank/publist/taslp10.pdf)
- **Efficient RT algorithm (Bank 2010):**
  1. Uniform tension T(t) from the squared sum of k·y_k (k·y_k is already computed for the bridge force, so it is free).
  2. Dynamic correction only for the lowest K longitudinal modes, those excited near or above resonance. The correction filter is "resonant 2nd-order high-pass" = full response minus DC gain.
  3. A single excitation force for all odd longitudinal modes (e.g. F_ξ,3) and one for all even modes (F_ξ,4); only the difference-frequency part F⁻ is computed. That part is a correlation of the modal-amplitude vector with its shifted copy.
  4. Aliasing control: skip products whose sum frequency exceeds fs/2, or simply restrict to transverse modes below fs/4.
  - Typically K < 10 longitudinal resonances are audible. A naïve cost is N·K MACs (e.g. 100 transverse × 10 longitudinal = 1000), which the odd/even trick reduces.
  - Earlier models that neglected the transverse term in the bridge force produced spurious low-frequency components; the 2010 method fixes this at small extra cost. — [Bank et al. 2010](https://home.mit.bme.hu/~bank/publist/taslp10.pdf)
- **Perceptual importance:** most important in the low range, where it "greatly contributes to the metallic character". Recognized long ago by piano builders. — [Bank et al. 2010](https://home.mit.bme.hu/~bank/publist/taslp10.pdf)
- A "very efficient" perceptual method for phantom partials exists (ref. [36] in Bank 2010) but "has a loose connection to physical reality". — [Bank et al. 2010](https://home.mit.bme.hu/~bank/publist/taslp10.pdf)
- A cheap ML proxy: Simionato et al. 2024 add only "double-frequency (phantom) partials at 2f with halved decay time" to an additive model. — [Frontiers 2024](https://www.frontiersin.org/journals/signal-processing/articles/10.3389/frsip.2023.1276748/full)
- **Counterpoint:** Moore, Neldner, Rokni (Rollins College, ASA meeting 4aMU1, Nov 2018) report that "only a small part of the power in the phantom partials comes from the string". The bridge and soundboard generate them too, so source-filter piano models are incomplete. — [acoustics.org lay summary](https://acoustics.org/tag/piano/) (lay summary, no quantitative data)
- **Geometrically exact / pitch glide:** the geometrically exact model (Ducceschi & Bilbao 2019/2022) shows the spectrum shifting away from harmonic as amplitude rises (initial amplitudes 1 mm, 1 cm, 10 cm compared). — [ICA 2019](https://www.pub.dega-akustik.de/ICA2019/data/articles/001028.pdf). In the cubic (4th-order Taylor, transverse-only) SAV string, "the slight up bend observed at each impact is an expected effect due to the nonlinearity". — [DAFx25](https://dafx25.dii.univpm.it/wp-content/uploads/2025/09/DAFx25_paper_24.pdf)
- Explicit SAV with a nonlinear string creates spectral aliasing; Risse et al. suggest slightly lowering the stiffness/CFL parameter and oversampling so the linear eigenfrequencies reach only up to ~20 kHz. — [DAFx25](https://dafx25.dii.univpm.it/wp-content/uploads/2025/09/DAFx25_paper_24.pdf)
- A full FD model with longitudinal motion needs fs ≈ 500 kHz for stability (2004 PCs: monophony). — [Bank & Sujbert DAFx-04](https://dafx.de/paper-archive/2004/P_089.PDF)

### Inferences
- Cheapest useful RT approximation for Rust: compute T(t) ∝ Σ (k y_k)² and the odd and even difference-frequency correlation forces from the modal state, then drive fewer than 10 longitudinal resonators per bass string, only in the bass and lower tenor. Phantoms in the treble can be skipped.
- Pitch glide (tension-modulated frequency) is not modeled in Bank's one-way scheme, because the transverse frequencies are fixed. A cheap add-on would modulate the transverse mode frequencies by the T(t) already computed. That idea is untested in the sources read here.

### Gaps
- No source here quantifies perceptual thresholds for phantom partials or pitch glide across the keyboard.
- Tension-modulation DWG algorithms (Tolonen, Välimäki, Karjalainen, c. 2000) were not retrieved; they are known from the literature but not verified in this session.
- Bank's PhD thesis ("Physics-based sound synthesis of string instruments including geometric nonlinearities", BME 2006) was not read; its page is at [home.mit.bme.hu/~bank/phd](https://home.mit.bme.hu/~bank/phd).

---

## 4. Multiple strings per note: unison mistuning, double decay, beating, bridge coupling (Weinreich), polarizations, sympathetic resonance

### Takeaway
Summing detuned, uncoupled strings gives beating but **not** a true two-stage decay. Double decay needs coupling through a common, yielding bridge (Weinreich), or an equivalent mechanism: two polarizations coupled at the bridge, or parallel "secondary" resonators with different decay rates. In modal RT practice, Bank 2010 uses **one-way** coupling for structural stability: secondary resonator banks are driven by the hammer force for beating and double decay, and by summed main-string outputs through an 8×8 region gain matrix for sympathetic resonance. In DWG, bidirectional coupling via a shared bridge termination is straightforward.

### Cited Findings
- Most piano keys have three slightly mistuned strings, giving beating and two-stage decay: a fast early decay and a long "aftersound". The single string's horizontal and vertical polarizations are also coupled through the bridge. — [Bank et al. 2010](https://home.mit.bme.hu/~bank/publist/taslp10.pdf)
- "A two-stage decay is desired in piano strings." Summing detuned strings "can provide a realistic beating effect … [but] does not provide a true two-stage decay". In DWG, bridge motion launches identical waves on all attached strings; a rigid-bridge scattering formulation and coupling-matrix eigenanalysis are in JOS PASP §C.13 and §6.12.2 (horizontal/vertical). — [JOS PASP, Coupled Strings](https://ccrma.stanford.edu/~jos/pasp/Coupled_Strings.html)
- Weinreich's account: the vibration should be described by the normal modes of string doublets and triplets coupled through the soundboard; aftersound comes from small differences in excitation and mistuning. — [Baxandall et al., IOA paper](https://www.ioa.org.uk/system/files/publications/AR%20BAXANDALL%20IS%20BROWN%20CE%20GOUGH%20VIBRATIONS%20OF%20PIANO%20STRINGS%20AND%20COUPLING%20TO%20THE%20SOUNDBOARD.pdf); [KTH lecture notes](https://www.speech.kth.se/music/5_lectures/weinreic/weinreic.html) (secondary; Weinreich JASA 1977 not read directly)
- RT implementation options listed by Bank 2010: (a) two string models in parallel per note, with various coupling types (refs. Bensa et al., Aramaki et al.); (b) perceptual: 5–10 parallel resonators only for prominent partials (Bank 2000/2003); (c) the "beating equalizer", a peak EQ modulating partial envelopes (Rauhala et al.). — [Bank et al. 2010](https://home.mit.bme.hu/~bank/publist/taslp10.pdf)
- Bank 2010's choice: **secondary resonator banks** (full string models with fewer modes than the main string) driven by the hammer force. Only the bridge-force output is needed, so each resonator has one lumped coefficient. — [Bank et al. 2010](https://home.mit.bme.hu/~bank/publist/taslp10.pdf)
- **Sympathetic resonance:** stable bidirectional coupling is easy with DWG strings on a common termination (Garnett; Borin et al.) but "more complicated in modal synthesis". Bank therefore uses **unidirectional** coupling: main-string outputs → summed per region → R×R gain matrix (R = 8 keyboard regions, +64 multiplications) → secondary banks. It is "structurally stable" for any coupling gains. The secondary banks cannot be switched off by polyphony management because they carry sympathetic resonance. — [Bank et al. 2010](https://home.mit.bme.hu/~bank/publist/taslp10.pdf)
- Dampers are modeled only by shortening resonator decay times; a physical damper model is future work. — [Bank et al. 2010](https://home.mit.bme.hu/~bank/publist/taslp10.pdf)
- Bensa et al., "Synthesis of Coupled Piano String Vibrations Based on Physical Modeling", J. New Music Research 30(3):213–226 (2001): coupled DWG strings. — [ref. in Four Decades of DWG](https://arxiv.org/abs/2604.12878)
- Rauhala et al., "A Parametric Piano Synthesizer", Computer Music Journal 32(4):17–30 (2008): a DWG string plus simulations of beating, sympathetic resonance and hammer knock, with real-time control of inharmonicity and f0. — [Project MUSE](https://muse.jhu.edu/article/254428); [dblp](https://dblp.org/pid/35/1671)
- DDSP-Piano uses n_strings = 2 detuned additive "sub-strings" per note. The detuning is learned, through a tanh scaled to a semitone range, plus a global instrument detuning. Authors' observation: "the natural beating between simultaneous notes in harmony may be sufficient … and the mutual notes interaction is eventually negligible" for the tested samples. — [Renault et al. 2022](https://dafx.de/paper-archive/2022/papers/DAFx20in22_paper_48.pdf)

### Inferences
- For Rust: per note, one main modal or DWG string plus one or two secondary banks with slight detuning and a different decay gives beating and double decay with guaranteed stability (no feedback). Sympathetic resonance can go through a shared bridge bus that feeds all undamped secondary banks. This matches Bank 2010 and is proven RT.
- A passive, bidirectional bridge (port-Hamiltonian / energy-conserving coupling) is the physically correct alternative. Risse et al. note that a bridge can be "formulated considering a passive coupling under a port-Hamiltonian representation", but they did not implement it.

### Gaps
- No measured unison mistuning values (cents) or typical horizontal/vertical polarization frequency splits were found in the sources read.
- Weinreich 1977 original (JASA 62:1474) was not read directly; the date is reported inconsistently in secondary sources.

---

## 5. Duplex scaling and aliquot, terminations (agraffe, capo), wound bass strings

### Takeaway
None of the sources retrieved in this session give a synthesis-oriented model or measured data for duplex/aliquot sections, agraffe vs capo terminations, or wound-string specifics (core plus wrapping, effective κ and μ). This topic is a gap.

### Cited Findings
- Bank 2010 assumes rigid hinged terminations and moves soundboard "impedance effects" (frequency and decay changes) into the string parameters. The soundboard impedance is about 1000× the string impedance, so bidirectional coupling is mainly a frequency/decay perturbation. — [Bank et al. 2010](https://home.mit.bme.hu/~bank/publist/taslp10.pdf)
- String lengths for a full concert grand (Steinway D) are tabulated in Chabassier & Duruflé, INRIA technical report RT-0425 (2012), cited by the vendor forum calculation. — (vendor forum; secondary)

### Inferences
- Wound strings can be represented in the stiff-string PDE through effective μ (core plus winding mass) and an effective EI that comes mostly from the core. B then follows from the same formula. This is standard practice but no source for it was retrieved here.
- Duplex and aliquot segments could be extra undamped secondary modal banks tuned to harmonics, fed from the bridge bus, using the same mechanism as sympathetic resonance.

### Gaps
- Conklin 1996 "Piano strings and scale design" (JASA 100(3)) and Chabassier & Duruflé RT-0425 were not read.
- No source was found on modeling agraffe vs capo d'astro bar terminations, or on the measured effect of duplex scaling in synthesis.

---

## 6. Key groups and theses, 2023–2026 work, differentiable / ML-assisted estimation

### Takeaway
The lineage runs from Bank (Budapest/Helsinki: MSc 2000, PhD c. 2006, modal RT piano 2010, SPM review 2019) to Aalto (Rauhala, Lehtonen, Välimäki: dispersion and loss filter design, parametric DWG piano 2006–2008), Marseille (Bensa thesis 2003; FD-to-DWG link), Inria/ENSTA (Chabassier thesis 2012; energy-preserving full piano) and Edinburgh/Bologna (Bilbao, Ducceschi: quadratisation and explicit SAV 2019–2025). Recent RT-oriented string work is mostly at DAFx: van Walstijn et al. 2023 (modal hammer–string with time-variant parameters) and Risse–Hélie–Bilbao 2025 (drift-free explicit SAV, 60-voice demo). ML work (Renault et al. 2022; Renault PhD 2024; Simionato et al. 2024) uses inharmonic additive models whose inharmonicity and detuning parameters can be read physically. In listening tests, the physics-based commercial modelled piano still beat DDSP-Piano.

### Cited Findings
- Bank MSc thesis (BME, May 2000), published as Report 54 of the HUT Laboratory of Acoustics and Audio Signal Processing; RT version in 2002. — [Bank et al. 2010 refs](https://home.mit.bme.hu/~bank/publist/taslp10.pdf)
- Bensa PhD thesis (Université de la Méditerranée, Marseille, May 2003), on piano synthesis by physical and signal models. — [Bank et al. 2010 refs](https://home.mit.bme.hu/~bank/publist/taslp10.pdf)
- Bank's PhD on geometric nonlinearities in string synthesis. — [home.mit.bme.hu/~bank/phd](https://home.mit.bme.hu/~bank/phd)
- Rauhala, Lehtonen, Välimäki, "Toward next-generation digital keyboard instruments", IEEE Signal Processing Magazine 24(2) (2007), covering new signal-processing methods for parametric piano synthesis. — [Bank 2010 refs](https://home.mit.bme.hu/~bank/publist/taslp10.pdf); [search result](https://data.nationallibrary.fi/bib/me/W00522687100)
- **van Walstijn, Bhanuprakash, Chatziioannou, "Tunable collisions: hammer-string simulation with time-variant parameters", DAFx 2023.** It starts from a modal string expansion and builds an **explicit** scheme by quadratising the contact energy (one-sided power law). It allows online changes to all scaled-form parameters, including tension and contact stiffness. It has a numerical energy balance and a strictly non-adhesive contact force. Matlab on an Intel i7-6700 kept **RTF < 0.75 for block sizes ≥ 32 with up to 1000 string modes**. About **300 modes** cover the audio range for A0. Optimized C++/AVX is expected to be much faster. Parameters given for piano C2 (f0 = 65.4 Hz) and C4. — [DAFx23 paper 18](https://www.dafx.de/paper-archive/2023/DAFx23_paper_18.pdf)
- **Risse, Hélie, Bilbao, "Power-balanced drift regulation for SAV methods: application to real-time simulation of nonlinear string vibrations", DAFx 2025.** The model is a cubic nonlinear stiff string with (η0, η1) losses, discretized by structure-preserving FD (pentadiagonal operators). The scheme is explicit and time-interleaved, about 40% more expensive than the original explicit SAV. It is a C++/Eigen Max/MSP external with **60-note polyphony at 96 kHz using multithreading** on a laptop. Code is public. f0 and inharmonicity β are exposed as high-level parameters, with T and l0 derived; pitch bend is done by modulating l0, and hence h, subject to h ≥ h_min. — [DAFx25 paper 24](https://dafx25.dii.univpm.it/wp-content/uploads/2025/09/DAFx25_paper_24.pdf)
- Bilbao, Webb, Wang, Ducceschi, "Real-time gong synthesis", DAFx 2023: an explicit SAV method used in RT for a 2D nonlinear plate. Drift is a major problem there. — [refs in DAFx25](https://dafx25.dii.univpm.it/wp-content/uploads/2025/09/DAFx25_paper_24.pdf)
- Bilbao, Ducceschi, Webb, "Large-scale real-time modular physical modeling sound synthesis", DAFx 2019, with the linear stiff string as the building block. — [DAFx 2019 paper 22](https://dafx.de/paper-archive/2019/DAFx2019_paper_22.pdf) (search summary only)
- **Renault, Mignot, Roebel, "Differentiable Piano Model for MIDI-to-Audio Performance Synthesis", DAFx 2022 (DDSP-Piano).** A polyphonic (P = 16 channels) harmonic-plus-noise DDSP model: explicit Rigaud inharmonicity model, 2 detuned strings per note, K = 96 partials, 64 noise filter bands, learned 1.5 s reverb IRs at 16 kHz. Trained on MAESTRO v3.0.0 (about 200 h). MOS test with 52 listeners: better than a TTS-style neural baseline, still below the commercial modelled piano (physical modeling) and on par with sampling. — [DAFx 2022 paper](https://dafx.de/paper-archive/2022/papers/DAFx20in22_paper_48.pdf). About 281.5k parameters at P = 16, per [Simionato et al.](https://www.frontiersin.org/journals/signal-processing/articles/10.3389/frsip.2023.1276748/full)
- Renault PhD thesis, "Neural Audio Synthesis of Realistic Piano Performances", IRCAM/Sorbonne, defended July 2024 (advisor Roebel, co-advisor Mignot). — [HAL tel-04732963](https://hal.univ-lille.fr/IRCAM/tel-04732963v1)
- Simionato, Fasciani, Holm, "Physics-informed differentiable method for piano modeling", Frontiers in Signal Processing (2024). Small networks predict B(f0) and the detuning δF; 24 partials plus 2f phantoms. 3,428 parameters; **567 FLOPs/sample at polyphony 1, 33,057 FLOPs/sample at polyphony 88** (24-sample frames). No CPU benchmark and no listening test. — [Frontiers](https://www.frontiersin.org/journals/signal-processing/articles/10.3389/frsip.2023.1276748/full)
- "Differentiable modal synthesis for physical modeling of planar string sound and motion simulation" (arXiv 2407.05516, 2024) covers nonlinear stiff strings. — [arXiv](https://arxiv.org/html/2407.05516v1) (not read in detail)
- "Sines, Transient, Noise Neural Modeling of Piano Notes" (Frontiers Signal Proc. 2024/2025, arXiv 2409.06513) is a neural piano note model. It notes that accurate physical models (Chabassier et al. 2014) are computationally demanding. — [arXiv](https://arxiv.org/abs/2409.06513) (not read in detail)
- The 2026 waveguide review (Tablas de Paula, Smith, Välimäki, Reiss, JAES 74(7–8)) concludes that waveguides can now be optimized with ML/DDSP. — [arXiv 2604.12878](https://arxiv.org/abs/2604.12878)

### Inferences
- The most usable ML result for a no-sample Rust engine is offline estimation of physical parameters (B curve, unison detuning, per-mode decay) by differentiable fitting to recordings, followed by a modal or DWG runtime. DDSP-Piano's explicit-B ablation supports keeping physically structured parameters.

### Gaps
- No 2024–2026 DAFx/JASA paper presenting a **complete** RT physics-based piano with coupled unisons and a bridge (beyond Bank 2010) was found. A 2024–2025 DAFx archive sweep was not completed.
- Bank's 2010s follow-up modal-piano papers and the Viscount product details were not retrieved.

---

## 7. Reported real-time performance numbers

### Takeaway
The proven numbers are: a full modal piano (10,000 resonators plus 4 convolutions) at ~30% of a 2.4 GHz Core 2 Duo (2010); an FD+resonator piano with longitudinal modes at 5–10 voices on a 2004 PC; a modal hammer–string with 1000 modes at RTF < 0.75 in Matlab (2023); and an explicit SAV nonlinear FD string at 1–7% of one Apple M3 core per string at 44.1 kHz (2025), with 60 voices at 96 kHz multithreaded. A modern CPU with SIMD should comfortably run a Bank-style modal piano at full polyphony.

### Cited Findings
| Model (year) | Platform | Reported figure | Status | Source |
|---|---|---|---|---|
| Modal piano, transverse + longitudinal + secondary resonators + 4×20000-tap FFT convolution soundboard (Bank, Zambon, Fontana 2010) | Intel Core 2 Duo 2.4 GHz laptop, C++, SSE, soundboard on 2nd thread | **Full polyphony, ~10,000 2nd-order resonators, ~30% load**. Convolutions ≈ ¼ of string+hammer cost. Relative cost at full polyphony: transverse 40%, hammer 5%, secondary resonators 30%, longitudinal 25%. SSE ≈ 4× speedup | RT | [taslp10.pdf](https://home.mit.bme.hu/~bank/publist/taslp10.pdf) |
| Same: control rates | n/a | Audio rate for DSP; "MIDI rate" fs/N_a for hammer and polyphony; calibration rate 20–50 Hz for parameter changes. Polyphony queue limited by total cost, not note count | RT | [taslp10.pdf](https://home.mit.bme.hu/~bank/publist/taslp10.pdf) |
| Parallel 2nd-order soundboard filter alternative (Bank 2010) | n/a | 100 log-spaced poles (order 200), R = 0.98. Uses more ops than partitioned convolution for the same quality but less memory and zero latency | offline tests only | [taslp10.pdf](https://home.mit.bme.hu/~bank/publist/taslp10.pdf) |
| FD transverse + resonator longitudinal (Bank & Sujbert 2004) | "average PC" 2004, C++ | **5–10 voices** RT; 10–20× a DWG string; ~10% of the full FD model | RT | [DAFx-04 P_089](https://dafx.de/paper-archive/2004/P_089.PDF) |
| Full FD with longitudinal motion (2004 context) | 2004 PC | Needs fs ≈ 500 kHz, which means monophony | marginal | [DAFx-04 P_089](https://dafx.de/paper-archive/2004/P_089.PDF) |
| Modal hammer–string, explicit quadratised contact (van Walstijn et al. 2023) | Intel i7-6700, **Matlab** | **RTF < 0.75** for block ≥ 32 and up to 1000 modes; A0 needs ~300 modes | RT-viable (single string) | [DAFx23_paper_18](https://www.dafx.de/paper-archive/2023/DAFx23_paper_18.pdf) |
| Explicit SAV cubic nonlinear stiff FD string with drift control (Risse, Hélie, Bilbao 2025) | MacBook Pro **M3**, C++/Eigen, 44.1 kHz, single string | f0 = 20 Hz, N = 360: **3.6% (f32) / 7.3% (f64)**; 50 Hz, N = 215: 2.2% / 4.1%; 100 Hz, N = 140: 1.5% / 2.7%; 200 Hz, N = 84: 0.99% / 1.6%. Original explicit SAV (f64): 5.3 / 3.0 / 2.0 / 1.2% | RT | [DAFx25 paper 24](https://dafx25.dii.univpm.it/wp-content/uploads/2025/09/DAFx25_paper_24.pdf) |
| Same, Max/MSP external | laptop, multithreaded | **60-note polyphony at 96 kHz** (uncoupled strings, synthetic force excitation, no hammer or bridge) | RT | [DAFx25 paper 24](https://dafx25.dii.univpm.it/wp-content/uploads/2025/09/DAFx25_paper_24.pdf) |
| Geometrically exact string, quadratised, linearly implicit (Ducceschi & Bilbao 2019) | 3.6 GHz i5 (2011 iMac), Matlab | About 3× faster ("half an order of magnitude") than Newton (10 iterations); run at fs up to 5 × 480 kHz | offline | [ICA 2019](https://www.pub.dega-akustik.de/ICA2019/data/articles/001028.pdf) |
| Physics-informed DDSP piano (Simionato et al. 2024) | n/a | 567 FLOPs/sample (1 voice), 33,057 FLOPs/sample (88 voices); latency ~1 ms (24-sample frames) | estimated, not benchmarked | [Frontiers](https://www.frontiersin.org/journals/signal-processing/articles/10.3389/frsip.2023.1276748/full) |
| Full piano FEM/FD with energy-preserving schemes (Chabassier et al. 2013–2016) | HPC | Not retrieved | offline | [Part 2](https://www.numdam.org/articles/10.1051/m2an/2015007/) |

### Inferences
- Rough scaling estimate from Bank 2010's numbers (inference, not measured): about 10,000 resonators × ~2.5 MPOS × 44,100 Hz ≈ 1.1 GMAC/s for strings. That is a modest load for one modern AVX2/AVX-512 or NEON core in f32. A Rust modal piano with ~3 strings/note, secondary banks and a longitudinal block should fit well within one or two modern cores.
- Explicit SAV FD at ~1–4% of an M3 core per string (f32) suggests 88 × 3 nonlinear FD strings would need about 3–9 cores' worth without voice management, before adding hammer, coupling and bridge. In practice only sounding strings need simulating, but sympathetic resonance (pedal down) wants all strings live. This is an inference from the single-string table.

### Gaps
- No published per-voice CPU figures were found for DWG piano models on modern CPUs (Rauhala et al. 2008 CMJ was not read in full).
- No published RT figures for a fully coupled (unison plus bridge) energy-conserving piano string model were found.
