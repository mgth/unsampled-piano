# Excitation side of a physics-based real-time piano: hammer, action, dampers, pedals, mechanical noises (state of the art to 2026)

Scope: what drives the strings in a no-sample, MIDI-driven, real-time physical piano written in Rust. Verification status: items marked "(verified)" were read in the primary text (PDF or full page). Items attributed to search abstracts or forum posts are labelled as such. Anything not sourced is in the Gaps sections.

---

## 1. Hammer felt models and measured parameters across the keyboard

### Takeaway
The workable choice is a power-law felt spring, F = Q0·u^p, plus a hysteresis term: either Stulov's hereditary integral or the "simple" form, F = Q0[u^p + α·d(u^p)/dt]. Stulov fitted closed-form per-key formulas for m, p, Q0 and α over keys n = 1..88, so a full keyboard can be parameterised with five formulas. Measured p varies widely between sources: about 2.2–3.5 in static tests, and about 2 (bass) to 4 (treble) in dynamic tests on a voiced Steinway. Stulov's own fit gives 3.7 to about 5.0, so p should be exposed as a voicing control and not hard-coded.

### Cited Findings
- **Stulov four-parameter hereditary (hysteretic) model** (verified in text): F(u(t)) = F0·[u^p(t) − (ε/τ)∫0^t u^p(ξ)·exp((ξ−t)/τ) dξ]. F0 is the instantaneous stiffness and p the compliance-nonlinearity exponent; these are the elastic parameters. ε (hereditary amplitude, 0 ≤ ε < 1) and τ (relaxation time) are the hereditary parameters. The hammer equation of motion is m·ü + F(u) = 0, with u(0)=0 and u̇(0)=V. Felt force–compression curves are hysteretic, and the loading slope depends strongly on loading rate. — [Stulov & Kartofelev, "Piano hammer-string interaction: influence of elastic parameters of bass hammers on contact time", Acoustics High Tatras 2009](https://www.tud.ttu.ee/web/dmitri.kartofelev/publications/34IAC2009.pdf)
- **Slow-loading limit** (verified): F = F0(1−ε)·u^p = Q0·u^p, where Q0 is the static hammer stiffness. — [Stulov & Kartofelev 2009](https://www.tud.ttu.ee/web/dmitri.kartofelev/publications/34IAC2009.pdf)
- **Per-key parameter fits** for n = 1..88 (verified in Stulov 2009 and cross-checked in Smith's PASP):
  - m = 11.074 − 0.074n + 1e-4·n² [g]
  - p = 3.7 + 0.015n
  - Q0 = 183·exp(0.045n) [N/mm^p]
  - Stulov 2009 also gives ε ≈ 0.9894 (±) 8.8e-5·n² and τ ≈ 2.72 (±) 0.02n (±) 9e-5·n² [µs]. The PDF text extraction lost the signs of these two formulas, so they must be checked against Stulov, Acta Acustica 91:1086–1097 (2005).
  - Sources: [Stulov & Kartofelev 2009](https://www.tud.ttu.ee/web/dmitri.kartofelev/publications/34IAC2009.pdf); [J.O. Smith, PASP, "Piano Hammer Modeling"](https://www.dsprelated.com/freebooks/pasp/Piano_Hammer_Modeling.html)
- **Simple hysteresis model** (PASP, citing Stulov): f_h = Q0[x^p + α·d(x^p)/dt], with α = 248 + 1.83n − 5.5e-2·n² [µs]. PASP calls this "a good approximation under normal playing conditions" and notes Q0 = f0(1−ε). — [Smith, PASP](https://www.dsprelated.com/freebooks/pasp/Piano_Hammer_Modeling.html)
- **Analytic contact time and compression for a hammer striking a rigid surface** (verified; useful for calibration and sanity checks):
  - u_max = [((p+1)·m·V²)/(2·Q0)]^(1/(p+1))
  - t0 = (2·u_max/V)·∫0^1 dx/√(1−x^(p+1)) = (2√π·u_max/V)·Γ(1+a)/Γ(b), with a = 1/(p+1) and b = a + 1/2
  - For a linear hammer (p = 1), t0 = π√(m/Q0), independent of velocity.
  - Contact time falls as velocity rises. For bass hammers (n = 1, 5, 10) the curves span about 2.5 ms down to about 0.75 ms over 0.75–3 m/s (read from Fig. 2).
  - The hysteretic model and the rigid-surface compression model predict nearly the same durations.
  - Bass felt is about 15–17 mm thick. The inferred compression-wave speed in felt is 25–75 m/s.
  - Bass hammers are lighter than their strings and can rebound from felt elasticity alone, without help from the reflection off the agraffe.
  - Source: [Stulov & Kartofelev 2009](https://www.tud.ttu.ee/web/dmitri.kartofelev/publications/34IAC2009.pdf)
- **Simple static model (1995)**: in Stulov's earlier static felt model the felt Young's modulus is about 160 MPa for soft and medium hammers and about 260 MPa for hard ones (from a search summary of the 1995 Estonian Academy paper). — [Proc. Estonian Acad. Sci. Eng. 1995](https://kirj.ee/wp-content/plugins/kirj/pub/eng-2-1995-172-182_20230202004753.pdf)
- **Measured exponent ranges** (Russell's summary page, 1997):
  - Static tests: p = 2.2–3.5 for hammers taken from pianos, and 1.5–2.8 for unused hammers, with no definite bass–treble trend.
  - Dynamic tests on a voiced Steinway action: p is about 2 in the bass, rising to about 4 in the treble. Example D2 hammer: p = 2.2.
  - Hammers rejected as "too hard" had lower p than voiced ones.
  - Below p = 1 the hammer is "too linear". Above p = 3, ff becomes harsh and pp bland. Preferred range: 2 < p < 3.
  - Hard blows give shorter contact and a "dynamically harder" felt, so ff is brighter.
  - Treble hammers stiffen more with force than bass hammers.
  - Voicing changes the softness gradient (soft surface, hard core).
  - Source: [D. Russell, "The piano hammer as a nonlinear spring" (1997)](https://www.acs.psu.edu/drussell/piano/nonlinearhammer.html)
- **Chabassier hammer law with hysteresis** (verified): F = K·Φ(d) + R·dΦ(d)/dt, with Φ(d) = ((d0 − d)+)^p. This is the same structure as Stulov's "simple" model and the Hunt–Crossley-type damping. Measured hammer-head masses: 10.76 g for D#1 and 7.90 g for C#5. Head height H = 0.04 m, d0 = 0.02 m. — [Chabassier & Duruflé, "Energy based simulation of a Timoshenko beam in non-forced rotation. Influence of the piano hammer shank flexibility on the sound", JSV 2014](https://www.math.u-bordeaux.fr/~durufle/article/HammerJSVV2.pdf)
- **Hammer shank flexibility** (verified):
  - Chabassier & Duruflé model the shank as a rotating Timoshenko beam (non-forced rotation) with the head as a rigid end mass. The equations come from the virtual work principle, and head–shank coupling uses Lagrange multipliers.
  - Shank parameters (from Izadbakhsh/McPhee/Birkett): L = 0.086 m, E = 10.18 GPa, A = 32.38e-6 m², ρ = 560 kg/m³, I = 83.44e-12 m⁴, G = 0.64 GPa, κ = 0.85. The jack force acts 0.0155 m from the shank pivot.
  - Let-off is modelled abruptly: the jack force drops to zero once the head–string distance falls below the let-off distance. The authors say a smoother felt-roller law would be better but no data exist.
  - Result: with shank flexibility, equally loud notes played legato vs staccato have different spectra, with up to 20 dB differences in the 500–1000 Hz region (e.g. partial 16). Let-off distance (1.5 vs 3.0 mm) also changes the spectrum.
  - Source: [Chabassier & Duruflé 2014](https://www.math.u-bordeaux.fr/~durufle/article/HammerJSVV2.pdf)
- **McPhee group** (Waterloo):
  - Izadbakhsh, Birkett & McPhee (ASME IDETC 2007; J. Comput. Nonlinear Dyn. 2008): grand action with a flexible hammer shank.
  - Vyasarayani, Birkett & McPhee (2007 Kharagpur; JASA 123:3660, 2008): flexible action impacting a flexible string.
  - Izadbakhsh's 2007 thesis models the shank as a Rayleigh beam. It finds that shank flexibility changes the head's impact velocity and increases the head's "scuffing" motion along the string during contact.
  - Sources: [Waterloo MRG listing](https://uwaterloo.ca/motion-research-group/publications/considering-effect-hammer-shank-flexibility-using-multibody); [Izadbakhsh thesis, UWSpace](https://uwspace.uwaterloo.ca/handle/10012/3110)
- **IRCAM Modalys** exposes a "felt" connection with Stulov's parameters (F0, alpha exponent, epsilon hysteresis, tau time constant). This is a working software implementation of the hysteretic felt in a modal synthesizer. — [Modalys connection felt doc](https://support.ircam.fr/docs/Modalys/3.5.1/co/connection_felt.html)

### Inferences
- Values computed by the author from the verified Stulov formulas:
  - Key 1: m ≈ 11.0 g, p ≈ 3.72, Q0 ≈ 191 N/mm^p
  - Key 40 (C4): m ≈ 8.27 g, p ≈ 4.30, Q0 ≈ 1.1e3 N/mm^p
  - Key 88: m ≈ 5.34 g, p ≈ 5.02, Q0 ≈ 9.6e3 N/mm^p
  - For C4 at V = 3 m/s against a rigid surface, the closed form gives u_max ≈ 0.72 mm and t0 ≈ 0.60 ms. For key 1 at 1 m/s it gives u_max ≈ 0.65 mm and t0 ≈ 1.7 ms. The key-1 value agrees with Stulov's Fig. 2, about 2 ms.
  - Contact on a real (yielding) string is longer than these rigid-surface values.
- Units matter: Q0 is in N/mm^p. In SI (m), K = Q0·1000^p, e.g. about 1e15–1e18 for p = 4–5. The ill-conditioning this creates is a practical reason to integrate felt compression in mm or to non-dimensionalise.
- Stulov's p (3.7–5.0) is higher than Russell's dynamic range (about 2–4). Treat p and Q0 as voicing knobs. The power-law-plus-damping form (Hunt–Crossley/Chabassier: K·Φ + R·dΦ/dt) is the cheapest hysteretic choice for real time. The full hereditary integral needs only one extra state, because the exponential kernel can be computed recursively with a one-pole filter on u^p.
- Shank flexibility is a "touch" mechanism that only a full-action model reproduces. A real-time engine could approximate it by giving the hammer head one or two extra modal DOF (shank bending modes) fed by a touch-dependent jack force profile. This is speculative and not demonstrated in real time in the sources found.

### Gaps
- The exact signs of the ε(n) and τ(n) fits were not recoverable from the PDF text (see Stulov 2005, Acta Acustica united with Acustica 91:1086–1097).
- Chaigne & Askenfelt (JASA 95:1112 and 95:1631, 1994) hammer tables (mass, K, p for C2/C4/C7) were not retrieved.
- Giordano & Millis measurements of hammer–string force on real strings were not retrieved. Purdue's piano physics page mentions that work: [Purdue](https://www.physics.purdue.edu/piano/piano.html).
- Boutillon's (1988) dynamic hysteresis data, Hall's 1986–87 papers, and Conklin's JASA 1996 series were not accessed in this pass.

---

## 2. Numerical treatment of hammer–string contact and interface to waveguides or FD/modal strings

### Takeaway
There are three families:
- **Iterative energy-conserving implicit schemes** (Newton–Raphson on a scalar equation): robust but variable cost.
- **Non-iterative "energy quadratisation" (IEQ/SAV) schemes** (Ducceschi–Bilbao 2021+): fixed cost, explicit, provably stable, and already run in real time (C++/JUCE). This is the most attractive choice for a Rust real-time engine.
- **Waveguide-specific approaches**: the Van Duyne/Smith wave digital hammer, Bank's multi-rate hammer (5–10 voice real time on a ~2000s PC), and commuted synthesis, where the hammer becomes a velocity-dependent linear filter. Chabassier's fully energy-preserving whole-piano schemes are offline references.

### Cited Findings
- **Ducceschi, Bilbao, Willemsen & Serafin, "Linearly-implicit schemes for collisions in musical acoustics based on energy quadratisation", JASA 149(5):3502–3516 (2021)** (verified):
  - Collision potential: φ(η) = Kη/(α+1)·[η]+^(α+1), with α ≥ 1.
  - Write φ = ψ²/2, treat ψ as an independent state at half-steps, and discretise M·δtt·u = −K·u − (μt+ψ)·g together with δt+ψ = g·δt·u.
  - This gives a fully explicit update ("solved by simple division"). The discrete energy H = M(δt−u)²/2 + K·u^n·u^(n−1)/2 + ψ²/2 is conserved to machine precision.
  - Stability condition: k < 2√(M/K) for the linear part, independent of the nonlinearity.
  - The recommended gradient (their eq. 27) is g = κ·√(Kη(α+1)/2)·η^((α−1)/2) when η ≥ 0 (κ = sign ψ), and g = −2ψ/(η* − η^(n−1)) when η < 0. Here η* is the predicted compression without collision. The second form removes the spurious oscillations of the earlier choice. Guard: if g·η^(n−1) ≥ 4ψ, set g = 0 so that the force is always outward.
  - Iterative schemes (Chatziioannou–van Walstijn "IT-1"; Bilbao "IT-2") need more Newton iterations as stiffness grows, while N-IT cost is fixed. The authors call this "an important aspect in view of any real-time implementation".
  - Accuracy degrades to first order when Kη/(M·k) > 1. Very stiff hammers (Kη = 1e12) at 44.1 kHz show small differences from IT-2, which vanish at 5×44.1 kHz.
  - Hammer–string test: M = 10 g, α = 1.3, V0 = 0.5 m/s, string ρ = 6.3 g/m, T0 = 100 N, L = 0.7 m.
  - A non-iterative tromba marina model with α = 1.3 collisions runs in real time in C++/JUCE.
  - Source: [Ducceschi et al. 2021 (AAU accepted manuscript)](https://vbn.aau.dk/ws/files/475165562/DucceschiEtal2021JASALinearlyImplicitSchemes.pdf)
- **SAV follow-ups (from search abstracts):**
  - Modal SAV for snare-membrane collisions (Ducceschi, Hamilton & Russo, Forum Acusticum 2023). It notes that simple explicit or semi-implicit schemes are prone to instability, while fully implicit conservative schemes need Newton–Raphson. — [FA2023 paper](https://dael.euracoustics.org/confs/fa2023/data/articles/000995.pdf)
  - Russo, Webb, Ducceschi & Bilbao: convergence and relaxation for modal SAV on nonlinear strings (2025). — [Edinburgh research portal](https://www.research.ed.ac.uk/en/publications/convergence-analysis-and-relaxation-techniques-for-modal-scalar-a/)
  - Risse, Hélie & Bilbao: drift regulation for SAV on nonlinear strings (DAFx 2025). — [DAFx25 paper 24](https://dafx25.dii.univpm.it/wp-content/uploads/2025/09/DAFx25_paper_24.pdf)
  - Ducceschi & Bilbao: "Non-iterative simulation methods for virtual analog modelling", IEEE/ACM TASLP 2022 (listed in the NEMUS ERC project results). — [CORDIS 950084](https://cordis.europa.eu/project/id/950084/results)
- **Chabassier & Joly (2010)**: fully implicit energy-preserving schemes for nonlinear Hamiltonian wave systems, applied to the geometrically exact piano string (transverse–longitudinal coupling). — [IP Paris portal](https://researchportal.ip-paris.fr/en/publications/energy-preserving-schemes-for-nonlinear-hamiltonian-systems-of-wa/)
- **Chabassier, Chaigne & Joly**: "Time domain simulation of a piano", Part 2: numerical aspects (ESAIM M2AN 2016). — [numdam](https://numdam.org/articles/10.1051/m2an/2015007/)
- **Chabassier & Duruflé (JSV 2014)**: the shank scheme is coupled to the global energy-preserving piano solver through Lagrange multipliers (verified). — [HammerJSVV2.pdf](https://www.math.u-bordeaux.fr/~durufle/article/HammerJSVV2.pdf)
- **Castera & Chabassier (ESAIM M2AN, 2024)**: linearly implicit quadratisation of nonlinear kinetic energy for a rotating flexible hammer shank, with a discrete power balance (search abstract). — [ESAIM M2AN](https://www.esaim-m2an.org/articles/m2an/pdf/forth/m2an230209.pdf)
- **Wave digital hammer** (Van Duyne & Smith; JASA 96:3300, 1994, ASA abstract): the hammer is a nonlinear mass–spring (felt = nonlinear spring) attached as a traveling-wave element to any waveguide string. — [ASA 1994 abstract](https://auditory.org/asamtgs/asa94aus/4aMUa/4aMUa4.html)
- **Commuted piano synthesis** (Smith & Van Duyne, ICMC 1995, pp. 319–326; "Developments for the commuted piano", ICMC 1995, pp. 335–343):
  - The soundboard and body response is commuted into the excitation.
  - The nonlinear hammer–string interaction is replaced by a commutable, velocity-dependent linear filter, with a calibration scheme for the "linearised hammer".
  - Sources: [ASA 1995 abstract](https://auditory.org/asamtgs/asa95wsh/5aMU/5aMU5.html); [vds95.pdf](https://ccrma.stanford.edu/%7ejos/pdf/vds95.pdf)
- **PASP linear and state-dependent hammer** (verified):
  - Seen from the string, a mass + damped-spring hammer has driving-point impedance R_h(s) = (μs² + ks)/(s² + (μ/m)s + k/m). This gives biquad scattering filters in a waveguide.
  - Element values can be made state-dependent, i.e. coefficient formulas are explicit functions of state. State variables should be chosen so that nonlinearities stay memoryless after digitisation.
  - Source: [Smith, PASP](https://www.dsprelated.com/freebooks/pasp/Piano_Hammer_Modeling.html)
- **Bank (BME)**, from the thesis abstract and a DAFx 2002 abstract:
  - The nonlinear hammer is attached to a digital waveguide. The "discontinuity problem" at connection is addressed. The hammer's instability is cured by a multi-rate implementation, which also balances the cost of low vs high register strings.
  - Variant (Bank & Sujbert, DAFx 2002): the hammer force is computed with an auxiliary waveguide plus the nonlinear hammer, then used as the target for designing a low-order hammer filter in real time (nonlinear commuted synthesis).
  - Real time: "about 5–10 note polyphony on an average personal computer" (mid-2000s).
  - Sources: [Bank thesis abstract](https://home.mit.bme.hu/~bank/thesis/thesabs.html); [DAFx paper archive](https://dafx.de/paper-archive/details/yL8RbflysbijGmR3zFXxGg); [Bank DAFx04 sound examples](https://home.mit.bme.hu/~bank/publist/dafx04/index.html)
- **Chaigne & Askenfelt (JASA 95:1112, 1994)**: the reference explicit finite-difference stiff, damped string with a nonlinear hammer, derived from first principles, with parameters taken from real instruments. — [Giordano, JASA Reflections 154:R3 (2023)](https://par.nsf.gov/servlets/purl/10477206)

### Inferences
- For a Rust engine with modal or FD strings, the Ducceschi N-IT update is a good fit for the hammer:
  - one extra scalar state ψ per hammer;
  - a fixed number of FLOPs per sample;
  - no branches beyond the contact test;
  - energy stability, so it cannot blow up at ff.
- With a waveguide string, the delay-free loop between hammer and string must be broken. The options are the WDF-style reflection-free port (Van Duyne/Smith), a one-sample delay (unstable for stiff felt, hence Bank's multi-rate fix), or an oversampled local FD/modal patch around the contact point.
- Contact lasts only about 0.5–4 ms, so a per-voice oversampled hammer–string sub-solver (e.g. 4–8× only during contact) is affordable. This matches Ducceschi's observation that accuracy loss vanishes at 5×44.1 kHz.

### Gaps
- No verified CPU figures for Chabassier's full-piano simulations (expected to be offline-only; not confirmed here).
- Bilbao's "Numerical Sound Synthesis" (2009) chapter on hammer collisions was not consulted.

---

## 3. MIDI velocity → hammer velocity, strike position, contact duration vs velocity

### Takeaway
Hammer velocity is roughly logarithmic in MIDI velocity on reproducing pianos. Pressed touch rarely exceeds about 5 m/s, while struck touch reaches about 8 m/s (secondary source). Stulov treats V ≤ 5 m/s as the normal range. Contact time decreases with velocity because of felt nonlinearity: for bass hammers it goes from about 2.5 ms at 0.75 m/s to about 1 ms at 3 m/s. On real actions, key velocity just before let-off is the best linear predictor of hammer impact velocity. This matters for mapping key sensors to physics.

### Cited Findings
- "Usually the striking velocity of piano hammer V is not greater than 5 m/s." Contact time decreases with velocity; the compression-wave speed grows roughly as V^(2/(p+1)). — [Stulov & Kartofelev 2009](https://www.tud.ttu.ee/web/dmitri.kartofelev/publications/34IAC2009.pdf)
- Struck touch reaches hammer velocities near 8 m/s, while pressed touch rarely exceeds 5 m/s. Struck tones reach the string about 30–40 ms sooner than pressed tones at equal velocity. (Secondary summary of Goebl, Bresin & Galembo 2004/2005; not verified in the primary source.) — [composition.gregspero.com summary](https://composition.gregspero.com/lessons/a2/)
- On reproducing pianos the relation between hammer velocity (m/s) and MIDI velocity is "well matched by a logarithmic curve". Solenoid force saturates at MIDI ≈ 85 (low keys), about 90 (G2–C5), and about 100 (G6). Most performed velocities fall between 40 and 80. (Mailing-list reprint of a Goebl/Bresin-related passage; authorship unclear.) — [MMD 2003-08-25](https://www.mmdigest.com/Archives/Digests/200308/2003.08.25.01.html)
- Goebl & Bresin (JASA 2003): the Bösendorfer SE290 is more consistent over the whole dynamic range, while the Yamaha Disklavier is accurate only in a wide middle range. Key and hammer motion were measured with accelerometers. — [Goebl & Bresin 2003 PDF](https://iwk.mdw.ac.at/goebl/papers/Goebl-Bresin_JASA2003_reproAccuracy.pdf)
- **Fioretti, Bergamino, Gabrielli, Ciattaglia & Spinsante, DAFx 2025** (verified):
  - Accelerometers were placed on the key and hammer of an acoustic grand action. A linear key-to-hammer velocity model captures the trend but deviates near impact, which the authors attribute to escapement, felt compression and friction.
  - The minimum-MSE sampling point for predicting impact hammer velocity is slightly before let-off.
  - A tested digital keyboard has its three contacts at 5.6 mm (45%), 7.3 mm (58%) and 9.1 mm (73%) of 12.5 mm travel. Contact C coincides with the best-predictor point.
  - The usual A→C averaged velocity fits worse, especially at the extreme dynamic layers. Averaging from 10% of travel to C gives MSE = 3.4e-3.
  - The dataset is public.
  - Source: [DAFx25 paper 38](https://dafx25.dii.univpm.it/wp-content/uploads/2025/09/DAFx25_paper_38.pdf)
- KTH (Askenfelt & Jansson):
  - The jack loses contact with the roller less than 1 ms before string impact, and the hammer flies free during the last part of travel.
  - The key reaches key-bottom at about the same moment as impact.
  - Hammer–string contact lasts "a few thousandths of a second".
  - The timing and sequence of contact events depend on the key trajectory.
  - Source: [KTH lecture "From touch to string vibrations"](https://www.speech.kth.se/music/5_lectures/askenflt/askenflt.html) (from search snippet; page returned 502 on fetch)
- Hall (1986) showed the string's modal amplitudes depend entirely on the force pulse F(t), the contact time and the striking position α: A_n + jB_n ∝ (2·sin(nπα)/(μ·n·c))·∫F(t)·e^(jω_n t)dt. Strike position therefore imposes a sin(nπα) comb on the spectrum. — [Stulov & Kartofelev 2009, eq. 1](https://www.tud.ttu.ee/web/dmitri.kartofelev/publications/34IAC2009.pdf)
- In Chabassier & Duruflé the hammer-string contact uses a fixed contact interval on the string. — [Chabassier & Duruflé 2014](https://www.math.u-bordeaux.fr/~durufle/article/HammerJSVV2.pdf)

### Inferences
- A practical MIDI map would be V_h = V_min·(V_max/V_min)^((vel−1)/126), with V_min ≈ 0.3–0.5 m/s and V_max ≈ 5–7 m/s. An exponential in velocity is the inverse of the reported log relation; the ranges are an engineering choice informed by the 5 m/s pressed and 8 m/s struck figures.
- Expose the curve to the user, as all commercial instruments do.
- With 16-bit MIDI 2.0 velocity (Section 7), the map should take a float in [0,1].

### Gaps
- No primary table of hammer velocity vs MIDI velocity was retrieved; the Goebl–Bresin 2003 PDF could not be downloaded.
- No measured per-note striking-position table (the common rule of thumb is about 1/7–1/9 of speaking length, varying across the scale) was retrieved. Conklin 1996 Part I is the expected source.
- Goebl, Bresin & Galembo 2004/2005 primary numbers are unverified.

---

## 4. Piano action / escapement, repetition, touch, una corda

### Takeaway
Multibody action models exist, but they are research or offline tools:
- Hirschkorn (2004): 5 rigid bodies, 13 contacts, validated well for forte blows.
- Izadbakhsh and Vyasarayani/McPhee (2007–08): flexible shank and string contact.
- Thorin/Boutillon (2014–17): non-smooth dynamics aimed at efficiency.

A typical modern grand action model has about 5 DOF plus 1 for the damper. "Touch" affects sound through finger–key and key-bed noise (Goebl 2014: listeners discriminate only when those noises are present). Simulation shows that shank vibration can also change the spectrum (Chabassier 2014). For real time, a reduced action (key → hammer flight with a let-off event, drop, check, repetition-lever reset) is the realistic target.

### Cited Findings
- **Hirschkorn** (UWaterloo MASc thesis, 2004):
  - Key, whippen, jack, repetition lever and hammer are rigid bodies, with normal and friction contact forces at 13 locations.
  - The rig is a Boston grand keyboard with motor-driven key, load cell, optical encoders and high-speed video.
  - The model "very accurately" reproduces high-force blows and is less accurate at low force.
  - Recommended next steps: flexible key and hammer, and speed-independent felt compression.
  - Source: [UWSpace 10012/877](https://uwspace.uwaterloo.ca/handle/10012/877)
- **Izadbakhsh** (2007): the shank as a flexible Rayleigh beam changes impact velocity and increases scuffing during string contact. — [UWSpace 10012/3110](https://uwspace.uwaterloo.ca/handle/10012/3110)
- A 2024 Acta Acustica review states that most action models treat parts as rigid, except two that include shank flexibility, and that a modern grand action model has about 5 DOF plus 1 for the damper spoon. (Search snippet; full text blocked by a captcha.) — [Acta Acustica 2024, aacus240066](https://acta-acustica.edpsciences.org/articles/aacus/full_html/2024/01/aacus240066/aacus240066.html)
- **Thorin, Boutillon et al. (2014)**: non-smooth dynamics for efficient grand action simulation. The PDF exists on Boutillon's page but could not be fetched (TLS error). — [Thorin2014.pdf](https://ppi-lms.polytechnique.fr/xavier.boutillon/Annexes/Thorin2014.pdf)
- **Let-off in a full-piano simulation** (verified): the jack force is set to zero when the head–string distance falls below the let-off distance. Let-off distances of 1.5 mm and 3.0 mm with legato touch produce different spectra. Staccato vs legato at equal loudness gives differences up to 20 dB in the 500–1000 Hz region. The model has no key-bed shock, no impact-point change and no longitudinal rubbing, so the differences come from shank vibration alone. — [Chabassier & Duruflé 2014](https://www.math.u-bordeaux.fr/~durufle/article/HammerJSVV2.pdf)
- **Goebl, Bresin & Fujinaga (JASA 2014)**: listeners distinguished tones with identical hammer velocity but different touch only when finger–key sounds were present. Removing them eliminated the effect. A key-bottom effect may come from action mechanics. — [Goebl et al. 2014 (ResearchGate)](https://www.researchgate.net/profile/Werner-Goebl/publication/269274469_GoeblBresinFujinaga2014-JASA-PianoTouchQuality1/links/548576440cf24356db60f1e0/GoeblBresinFujinaga2014-JASA-PianoTouchQuality1.pdf)
- Chabassier & Duruflé quote the experimental literature: "the difference between two equally loud piano tones due to type of touch lies in the different noise components involved in the keystroke". They then show a structural (shank) mechanism in simulation. — [Chabassier & Duruflé 2014](https://www.math.u-bordeaux.fr/~durufle/article/HammerJSVV2.pdf)
- DAFx 2025: escapement nonlinearity and secondary hammer motion after jack release break the linear key→hammer mapping near impact. — [DAFx25 paper 38](https://dafx25.dii.univpm.it/wp-content/uploads/2025/09/DAFx25_paper_38.pdf)

### Inferences
- A real-time "action" layer for MIDI can be event-based:
  - note-on velocity → hammer free-flight velocity (plus touch-type metadata for noise synthesis);
  - repeated note-on before note-off → re-strike the still-moving string, possibly with a lower maximum velocity if the key has not fully returned (repetition-lever behaviour);
  - half-blow and una corda → per-note changes to hammer–string geometry (number of strings struck, felt zone with different Q0/p).
- A full multibody action is not needed for sound unless continuous key-position sensing is available.

### Gaps
- No verified numbers on una corda shift distance, the felt zone hit, or repetition minimum key rise. Thorin/Boutillon results were not read.
- No source was found that models upright action (half-blow soft pedal, slower repetition) quantitatively for synthesis.
- Askenfelt & Jansson's 1990–91 numerical timing tables were not retrieved (the KTH page returned 502).

---

## 5. Dampers, sustain / half-pedal, sostenuto, soft pedal

### Takeaway
There are almost no peer-reviewed physical models of damper felt for synthesis. Practical models use a time-varying viscous (or frequency-dependent) damping coefficient applied locally at the damper position. Half-pedal maps to an intermediate coefficient or contact pressure. Sustain-pedal sympathetic resonance is handled in the waveguide/filter literature (Lehtonen et al. JASA 2007; Zambon, Lehtonen & Bank DAFx 2008, parallel second-order filters). The commercial modelled piano exposes "Damping duration" and "Damper position" (which overtones are damped less efficiently). A vendor forum post claims Coulomb friction (F = μ·F_N) between felt and wire for pedal-release sound.

### Cited Findings
- US patents (8115092 / 8895831, Kawai-family): a "damper model calculating module" varies a viscous coefficient b_D(nΔt) over time to simulate continuous decay and string resonance. An intermediate value naturally represents partial (half) damping. — [US 8115092](https://image-ppubs.uspto.gov/dirsearch-public/print/downloadPdf/8115092); [US 8895831](https://image-ppubs.uspto.gov/dirsearch-public/print/downloadPdf/8895831)
- Older patent "Musical synthesizing apparatus for providing simulation of controlled damping". — [US 5304734](https://image-ppubs.uspto.gov/dirsearch-public/print/downloadPdf/5304734)
- **Zambon, Lehtonen & Bank (DAFx 2008)**: sustain-pedal effect simulation using parallel second-order filters calibrated on soundboard and string-register impulse responses. — [DAFx archive](https://dafx.de/paper-archive/details/5mJ31JDQWZTHNiqjmf19mQ)
- **Lehtonen et al. (JASA 2007)**, as cited secondhand: with the sustain pedal engaged, dampers lift, decay behaviour changes, and sympathetic vibration becomes more likely. A 2005 TKK thesis (supervisor Välimäki, advisor Rauhala) analysed recordings for a synthetic sustain-pedal algorithm. — [preprints.org 2026](https://www.preprints.org/manuscript/202608.1643); [Aalto thesis record](https://aaltodoc.aalto.fi/items/48d09ac6-59d3-4aee-94e9-3f047459d92b)
- "Piano forte pedal analysis and detection" (Télécom Paris/Eurecom): the sustain pedal increases partial decay times. — [IP Paris](https://researchportal.ip-paris.fr/en/publications/piano-forte-pedal-analysis-and-detection/)
- The commercial piano (forum reports, not official documentation):
  - The Action panel has "Damping duration" (damper efficiency) and "Damper position" (which overtones are damped less efficiently).
  - Half-pedal works with a continuous CC64; its velocity/pedal panel shows live CC64 values and a calibration.
  - Some pedals send a limited range (Yamaha FC3: 0–68; Roland DP-10: 24–127), and MIDI files often contain only 0/64/127.
  - Sources: vendor forum (four threads)
- A vendor forum post (vendor or user description, not peer-reviewed) describes pedal release computed as wool-fibre friction on steel wire, F_f = μ·F_N, while the felt slowly lifts. — (vendor forum)
- Chabassier & Duruflé note that a grand action model has an extra DOF for the damper spoon (via the Acta Acustica 2024 snippet). — [Acta Acustica 2024](https://acta-acustica.edpsciences.org/articles/aacus/full_html/2024/01/aacus240066/aacus240066.html)

### Inferences
- A physically motivated real-time damper can be:
  - a localised damping force −R_d(t)·v(x_d) at damper position x_d on each string (a modal or FD point damper), plus optionally a soft unilateral spring;
  - R_d(t) driven by damper height h(t) ∈ [0,1], computed from the key position (note-off) and CC64 (continuous);
  - damper position x_d producing a comb of less-damped partials (sin(nπ·x_d/L) ≈ 0 modes survive), which is the "Damper position" effect;
  - half-pedal as a partial h, i.e. a lighter contact force, giving slower and frequency-selective damping (high partials damp first if the felt contact acts as a lowpass).
- Sostenuto is pure logic: latch the "damper lifted" state of keys held at the moment CC66 goes on.
- Soft pedal (CC67) on a grand selects 2-of-3 strings plus an alternate felt zone (lower Q0, different p). On an upright, it reduces hammer travel ("half-blow"), i.e. lower attainable V_h.

### Gaps
- No peer-reviewed measured damper-felt contact law (stiffness, friction or damping values, damping time constants vs register) was found in this pass.
- No measured half-pedal (damper height vs CC64) curve was found.
- The full Lehtonen 2007 JASA paper was not retrieved.

---

## 6. Mechanical noises (hammer, key, key-bed, release, pedal, damper)

### Takeaway
The literature and products treat mechanical noises mimetically: impulse or event-triggered short excitations filtered by body or soundboard resonances, scaled with note-on or note-off velocity. They are not obtained from first-principles action models. The perceptual relevance of finger–key and key-bed noise is established (Goebl 2014). Physically, these are impacts on the key frame and keybed that radiate through the structure. An impact-modal approach (impulse → modal body filter) is the natural physics-based route.

### Cited Findings
- Goebl et al. (2014): finger–key noise drives listeners' ability to discriminate touch at equal hammer velocity. — [Goebl et al. 2014](https://www.researchgate.net/profile/Werner-Goebl/publication/269274469_GoeblBresinFujinaga2014-JASA-PianoTouchQuality1/links/548576440cf24356db60f1e0/GoeblBresinFujinaga2014-JASA-PianoTouchQuality1.pdf)
- Patent US 5286915 (an electronic instrument simulating hammer–string interaction): mechanical noise from the key striking the stopper is synthesised from the negative part of a key accelerometer signal, filtered to approximate frame or soundboard resonance, and mixed with the tone. — [US 5286915](https://image-ppubs.uspto.gov/dirsearch-public/print/downloadPdf/5286915)
- Patent US 11138961: keybed-hitting sound generation in electronic pianos. — [US 11138961](https://image-ppubs.uspto.gov/dirsearch-public/print/downloadPdf/11138961)
- The commercial modelled piano, per forum users:
  - Separate "Hammer noise", "Key release noise" and "Damper/Pedal noise" controls.
  - Key release noise depends on note-off velocity; in the Blüthner model its loudness range over note-off velocity is 12 dB.
  - Pedal noise can be raised to +25 dB.
  - Noise level depends on the microphone "Level compensation" setting.
  - Sources: vendor forum (four threads)
- Harpsichord precedent (EURASIP 2004): the key-release "thump" is reproduced by triggering a short sample extracted from a recording. This is mimetic, not physical. — [DOAJ, EURASIP JASP 2004](https://doaj.org/article/8724dc0c97f64d13ac03db29cfccb353)
- The Chabassier full-piano model explicitly excludes "shock excitation on the structure" (keybed, rim, plate). This confirms that the leading physics model does not include these noises. — [Chabassier & Duruflé 2014](https://www.math.u-bordeaux.fr/~durufle/article/HammerJSVV2.pdf)

### Inferences
- A no-sample implementation can model each noise as a contact impulse (half-sine or raised-cosine force whose duration and amplitude come from a Hertz or Hunt–Crossley contact of the key on felt) driving a small modal bank:
  - keybed/key-frame modes for key-bottom thump at note-on (struck touch: earlier and stronger);
  - key-front rail and back-check for release;
  - damper re-contact with the string (a short damped excitation of the string itself plus a felt-friction burst);
  - pedal trapwork/lyre modes for pedal noise (CC64 rate of change).
- The modal parameters must be tuned by ear or measured (impact hammer on a real keybed). No published modal tables were found.

### Gaps
- No peer-reviewed physical model or measured spectra of key-bed thump, key-release, damper-return or pedal noises for synthesis was found. The "string-ending/termination noise" item likewise had no hits.

---

## 7. MIDI conventions (CC64/66/67, high-res velocity, release velocity, aftertouch, MPE) and the commercial piano feature checklist

### Takeaway
- MIDI 1.0 gives 7-bit velocity, a continuous CC64 (0–127, despite the switch-oriented spec tradition), CC66 sostenuto and CC67 soft.
- MIDI 2.0 raises velocity to 16 bits (65,536 steps).
- The 2025 MIDI Association/AMEI Piano Profile standardises a default velocity-to-dynamics (pppp–ffff) curve via MIDI-CI.
- The commercial modelled piano is the de facto feature checklist: continuous half pedal, note-off velocity driving release noise, and separate hammer, key and pedal noises.

### Cited Findings
- The MIDI Association and AMEI released the Piano Profile and implementation guide (MIDI-CI negotiates a shared interpretation). It defines a default note-on velocity curve mapping intended dynamics (pppp–ffff) to velocity values in both MIDI 1.0 and 2.0. — [midi.org announcement](https://midi.org/midi-association-and-amei-release-the-piano-profile-and-implementation-guide); [midi.org ?p=7438](https://midi.org/?p=7438)
- MIDI 2.0 velocity: 65,536 levels vs 128. Synthogy Ivory (high-resolution velocity, Piano Profile demo) and the Roland A88 MKII (high-resolution velocity) are listed as implementations. — [midi.org Feb 2026 update](https://midi.org/the-state-of-midi-2-0-high-resolution-performance-and-the-rise-of-profiles-update-feb-2026); [midi.org open-source tag](https://midi.org/tag/open-source)
- CineGrand2 advertises high-resolution pedal messages over MIDI 2.0. — [midi.org ?p=5153](https://midi.org/?p=5153)
- CC64 practice: some argue CC64–69 were meant as switches, but CC64 is used continuously (0–127) in practice. Some DAWs (e.g. Cubase historically) display it as on/off. — [KVR thread](https://www.kvraudio.com/forum/viewtopic.php?p=6667754); (vendor forum)
- The commercial piano: continuous CC64 for half-pedal with pedal calibration, and CC remapping (e.g. CC11 → sustain); key release noise tied to note-off velocity; hammer noise, damper/pedal noise, "Damping duration" and "Damper position" parameters (forum sources above). The official manual pages (vendor site) returned HTTP 403 and could not be read.

### Inferences
- Engine input should be continuous and resolution-agnostic:
  - velocity as f32 in [0,1] (from 7-bit or 16-bit);
  - release velocity as f32 (drives damper-return speed and release noise);
  - CC64, CC66 and CC67 as continuous f32 with slew limiting (to avoid zipper noise in damping coefficients);
  - optional polyphonic aftertouch or MPE per-note pressure mapped to "key held down hard", e.g. a key-bed pressure noise or a partial damper-lift effect. These mappings are a design choice, not a documented acoustic piano behaviour.
- Implementing a MIDI-CI Piano Profile responder would give standardised velocity semantics.

### Gaps
- Official documentation of the commercial piano (current version's parameter list; MIDI 2.0, polyphonic aftertouch and MPE support; repetition modelling) could not be accessed (HTTP 403). Whether its versions 8/9 support MIDI 2.0 high-resolution velocity is unverified.
- Exact Piano Profile curve values were not retrieved.
- No verified source documents an "aftertouch-like" acoustic effect in real pianos beyond key-bottom pressure and noise.
