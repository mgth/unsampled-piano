# Commercial (Vendor), commercial and open-source physically-modelled pianos, and calibration/evaluation data (state as of Oct. 2026)

Research notes. Every claim carries an inline source. Facts documented by primary sources (patents, manuals, company pages, peer-reviewed papers) are kept apart from secondary or forum claims, which are flagged. Note: vendor.com returned HTTP 403 to automated fetches, so the current (v9) online manual could not be read directly. The parameter checklist below therefore comes from the **Commercial 4 PDF manual**, extracted locally, plus third-party reviews of v8 and v9.

---

## Q1. Vendor / Commercial: founders, origins, technology, patents, papers, size/CPU, version history

### Takeaway
[founder] [founder] is a former piano tuner turned applied mathematician (PhD on the "parametrization of vibrating phenomena", INSA Toulouse / Institut de Mathématiques de Toulouse). Vendor was founded in 2006 under the French 1999 innovation law, and Commercial 1 shipped in August 2006. The only Vendor patent, US [vendor patent] B2 (priority 2006-07-28), describes an **offline modal ("presynthesis") computation plus real-time additive synthesis of exponentially damped partials**. The offline step solves a coupled string-soundboard FE eigenproblem, approximated by a precomputed interpolation/metamodel over the physical parameters. The real-time step uses excitation amplitudes and phases indexed by velocity. This is consistent with [founder]'s own description of the technique as "modal analysis + parametrization". The ~50 MB install size supports the claim that no sampled sustain is used. Small sampled noises (pedal, key release, hammer) are reported by Wikipedia but are not confirmed by Vendor in what I could read.

### Cited Findings

**People and origins**
- Vendor was created in 2006 "to sell Commercial", under the French law on innovation and research (1999), with support from INSA Toulouse and the Institute of Mathematics of Toulouse. Commercial was then "the first fully modeled piano instrument". (Interview published 31 May 2010.) — [CDM, "Makers of Commercial Talk Piano Modeling"](https://cdm.link/makers-of-commercial-talk-piano-modeling-developing-for-linux/)
- Team as of 2010:
  - **[founder]** ([founder]): former piano tuner who started university mathematics at 31 and prepared a PhD on the "parametrization of vibrating phenomena".
  - **[engineer]** ([engineer] per other sources): engineer at the Institut de Mathématiques de Toulouse, in charge of the real-time engine.
  - **[staff]** ([staff]): marketing and product development, later sales and support.
  - — [CDM 2010](https://cdm.link/makers-of-commercial-talk-piano-modeling-developing-for-linux/). Surnames [engineer] and [staff] are not in the CDM text. Search snippets name [engineer] [engineer] as co-founder, and Vendor's history page credits [founder] with the model design ([search summary of vendor.com/about](https://www.vendor.com/about); page returned 403 to direct fetch, so unverified).
- [founder] described the technique as "standard techniques issued from mechanics theory", including **modal analysis** (calculating vibration frequencies and the corresponding modes) and parametrization techniques developed at the university, plus his practical tuner/restorer knowledge. The model covers the interaction of soundboard, strings, bridge and air. — [CDM 2010](https://cdm.link/makers-of-commercial-talk-piano-modeling-developing-for-linux/)
- A German interview with "Dr. [founder] [founder], Chef von Vendor" exists (not read in full). — [digital-notes.de]((link removed)]-dem-chef-von-vendor/)
- First release: August 2006. — [Wikipedia: Commercial](https://en.wikipedia.org/wiki/Commercial)
- Wikipedia says sampled pedal noise, key-release noise and hammer noise supplement the modelled sound. The article carries "primary sources/promotional" maintenance tags. — [Wikipedia](https://en.wikipedia.org/wiki/Commercial) (treat as unverified)
- Some historical instruments come from the **[restoration project]** (Keyboard Instruments Virtual Restoration) project, which makes playable models of museum instruments (e.g. Händel-Haus Halle). — [Wikipedia](https://en.wikipedia.org/wiki/Commercial), citing commercial.com/[restoration project]

**Patent: US [vendor patent] B2** ("Device for producing signals representative of sounds of a keyboard and stringed instrument")
- Bibliographic data:
  - Inventor: [founder] [founder]. Assignee: Vendor.
  - Priority 2006-07-28; filed 2007-07-13; published as US[vendor patent]A1 (2009-10-01); granted 2011-03-29.
  - Status listed active, estimated expiry 2027-10-20.
  - — [Google Patents US[vendor patent]B2](https://patents.google.com/patent/US[vendor patent]B2/en)
- Family members: [vendor patent]B1 (FR title "Dispositif de production de signaux représentatifs de sons d'un instrument à clavier et à cordes"). Vendor's documentation also lists Japan [vendor patent] and China [vendor patent].2. — [Google Patents [vendor patent]B1](https://patents.google.com/patent/[vendor patent]B1/fr); [Justia, patents assigned to Vendor](https://patents.justia.com/assignee/vendor); JP/CN numbers from a search snippet of the [Vendor manual](https://www.vendor.com/user_manual?product=commercial&lang=en)
- Problem statement: the patent says earlier waveguide approaches needed manual filter tuning or recordings of one specific piano, and full physical models were too heavy for real time. The aim is to let the user set physical parameters of the soundboard and strings. — [Google Patents](https://patents.google.com/patent/US[vendor patent]B2/en)
- **Mechanical model** — [Google Patents](https://patents.google.com/patent/US[vendor patent]B2/en):
  - Each string is an elastic **beam**, which gives stiffness inharmonicity, plus a "quadratic effect".
  - The soundboard is modelled by **finite elements**. Options: shell, first-order laminated model with fibre orientation and ribs, isotropic model with ribs, or full 3D.
  - Strings and soundboard are solved as **one coupled system**. This includes sympathetic resonance between notes, interaction between unison strings, and explicit unison mistuning δ_p.
  - The FE mass, stiffness and damping matrices give **complex eigenfrequencies and mode shapes**, computed with a nonlinear eigenproblem method.
- **Synthesis** — [Google Patents](https://patents.google.com/patent/US[vendor patent]B2/en):
  - Signal model: s(p,t) = Σ_n a_n(p)·e^(−d_n(p)t)·sin(2π f_n(p) t + θ_n(p)) + b(p,t), where b is a percussive noise term.
  - f_n and d_n come from the eigenvalues.
  - a_n and θ_n come from a modal decomposition of **excitation signals** (recorded from a piano or from a hammer/string model), stored per strike intensity.
  - The real-time engine does additive synthesis and picks the excitation set closest to the incoming velocity.
- **"Presynthesis" module** — [Google Patents](https://patents.google.com/patent/US[vendor patent]B2/en):
  - Runs in the background, not in real time.
  - Maps the user's physical parameters to modal data through a **precomputed interpolation function (metamodel)**, built on designs of experiments (Latin hypercube, space-filling, sparse grids).
  - Candidate interpolators: multivariate polynomials, RBF, kriging, neural nets, SVM, regression, and Taylor/Padé approximants from model derivatives.
- **Parameters named in the patent** — [Google Patents](https://patents.google.com/patent/US[vendor patent]B2/en):
  - Soundboard: impedance Z_np per partial, Hooke-tensor weighting, dimensions, curvature, moisture.
  - Strings: unison detuning, tension, number of strings per note, position relative to the soundboard.
  - User-facing impedance weighting per harmonic, defined by three parameters (a constant, a **cut-off index**, a **slope**). These correspond to the product's "soundboard impedance / cutoff / Q factor" sliders.
  - The patent also mentions harpsichord, clavichord and fortepiano as targets.
- Citations:
  - Cites Yamaha US5264658A and Interval Research US6647359B1.
  - Cited by Yamaha (US8530736B2 / US20120137857A1, "Musical tone signal synthesis method…") and **Viscount WO2013135627A1** ("A system to reproduce the sound of a stringed instrument", i.e. the Physis patent).
  - The family is cited by Yamaha US8115092B2, Yamaha JP6819309B2 ("Resonance signal generator…"), and Casio JP6801443B2 / JP6540681B2 / JP7230870B2.
  - — [Google Patents citation lists](https://patents.google.com/patent/US[vendor patent]B2/en)

**Academic papers by [founder] on piano/string modelling**
- I found none. HAL/search queries for [founder] [founder] (Toulouse) combined with Padé / vibration / parametrization returned unrelated authors. — [search attempt, no hit]. See Gaps.

**Size and CPU**
- SOS review of v9 (published April 2026): ~**54 MB** macOS install; "mathematically models sound in real time, rather than playing back samples". — [Sound On Sound, Commercial 9](https://www.soundonsound.com/reviews/vendor-commercial-9?amp)
- CDM (2010): a whole grand piano "fits in just a few megs", compared with sampled pianos of up to ~40 GB. [engineer] said Commercial "needs quite a bit of CPU power" and a netbook runs at 80% or more CPU load. — [CDM 2010](https://cdm.link/makers-of-commercial-talk-piano-modeling-developing-for-linux/)
- Commercial 4 manual — [Commercial 4 manual PDF](https://www.strumentimusicali.net/manuali/VENDOR_COMMERCIAL4_ENG.pdf):
  - All notes are computed in real time.
  - Minimum 1.5 GHz CPU; **bass notes require more computation**.
  - Options exposed: **Internal Sample Rate** (lower means less CPU), **Polyphony** (counting notes *and sympathetic resonances*; manual or "Auto Pessimistic/Optimistic"), and CPU-overload detection.
  - Pro edition allows sample rates up to **192 kHz**; Standard is limited to 48 kHz.

**Version history (documented points only)**
- v1: Aug 2006. — [Wikipedia](https://en.wikipedia.org/wiki/Commercial)
- Reviews: v6 (SOS, March 2018), v7 (SOS, April 2021), v8 (SOS, Sept 2023), v9 (SOS, April 2026). — [SOS Commercial 9 review](https://www.soundonsound.com/reviews/vendor-commercial-9?amp)
- **v8 (released 16 Nov 2022)** — [Gearspace, "Vendor releases Commercial 8"](https://gearspace.com/board/new-product-alert-2-older-threads/1393236-vendor-releases-commercial-8-a.html); [Vendor forum thread](https://forum.vendor.com/viewtopic.php?id=9900):
  - Classical Guitar pack.
  - Revoiced instruments and refreshed GUI.
  - **Sympathetic resonance fed by external audio input**.
  - **User-defined stretch points** for tuning.
  - Per-note vibrato and tremolo.
- SOS also cites v8 improvements to all modern acoustic pianos, reed EPs and concert harp. — [SOS Commercial 8](https://www.soundonsound.com/reviews/vendor-commercial-8?amp)
- **v9** — [SOS Commercial 9](https://www.soundonsound.com/reviews/vendor-commercial-9?amp):
  - Changelog builds date from **9.0.1 on 15 Oct 2025** — [FileHorse old versions](https://mac.filehorse.com/download-commercial/old-versions/).
  - **Improved soundboard model** ("enhanced stereo and spatial qualities").
  - **Hammer Tone** parameter (EQ of the first milliseconds of impact).
  - **Thunder pedal** (drives the top of the dynamic range).
  - "Sombre" presets for all grands and felt-hammer presets for historical pianos.
  - Up to **8 virtual mics positioned in 3D** with directivity and per-mic convolution reverb, on 8 busses (Standard and above).
  - EP line-out option.
  - **[synth pack]** synth pack built on the same physical engine.
  - Ping-pong delay, resizable windows.
- 9.2.0 (8 June 2026): performance improvements, new Grand K2 presets, reversible Thunder pedal. — [KVR news](https://www.kvraudio.com/news-print.php?id=67262)
- 9.2.1 (11 June 2026) is the current stable release per Wikipedia. — [Wikipedia](https://en.wikipedia.org/wiki/Commercial)
- **MIDI 2.0**: a Vendor-forum user stated in Aug 2024 (v8.3) that Commercial does **not** support MIDI 2.0, only 14-bit HiRes velocity (CC#88 prefix). This is a forum claim, not a Vendor statement. — [Vendor forum "MIDI 2.0 setup?"](https://forum.vendor.com/viewtopic.php?id=11644)
- [modelled organ] (2020) and [modelled organ] 2 (Dec 2023) are sister products. — [SOS Commercial 9](https://www.soundonsound.com/reviews/vendor-commercial-9?amp)
- Modelled instruments include Steinway B/D, Petrof 275/284, Bechstein DG, Steingraeber E-272, Grotrian Concert Royal, Blüthner Model 1, historical pianos, harpsichord, EPs, mallets, harps and guitar. — [Wikipedia](https://en.wikipedia.org/wiki/Commercial)

**User-exposed parameter checklist (Commercial 4 manual, plus v8/v9 additions)** — [Commercial 4 manual](https://www.strumentimusicali.net/manuali/VENDOR_COMMERCIAL4_ENG.pdf) unless noted

*Tuning*
- Diapason (A4).
- Temperament. Temperament tables take string inharmonicity into account, except "flat".
- Microtuning / Scala import.
- **Unison width**: difference between the lowest and highest frequency of the 3 strings of a note.
- **Octave stretching**: 1 = stretching that follows natural inharmonicity (which depends on string length).
- **Direct sound duration**: the fast "direct" decay versus slow "remanent" decay, i.e. the double decay that results from unison coupling.
- v8 adds user stretch points — [Gearspace](https://gearspace.com/board/new-product-alert-2-older-threads/1393236-vendor-releases-commercial-8-a.html).

*Voicing*
- **Hammer hardness** set at three velocities (piano / mezzo / forte).
- **Spectrum profile**: per-partial gain sliders, applied to the model before synthesis, not as an EQ.
- **Hammer noise** level.
- **Strike point**, with a "humanize" random variation.
- **Soft (una corda) pedal smoothing**.
- v9 adds Hammer Tone — [SOS v9](https://www.soundonsound.com/reviews/vendor-commercial-9?amp).

*Design*
- **Soundboard impedance**: greater impedance gives longer sound.
- **Cutoff**: more long-lived high partials.
- **Q factor**: steeper impedance slope above cutoff, so high partials decay faster.
- **String length**: controls inharmonicity, adjustable up to a "10 m piano".
- **Sympathetic resonance**: depends on each damper position and the sustain pedal.
- **Duplex scale** resonance (front and rear undamped string segments).
- (The design panel also listed "quadratic effect" in later versions per the [search summary of the current manual](https://www.vendor.com/user_manual?product=commercial&lang=en); not verified because of the 403.)

*Action*
- **Damper position**: each overtone decays according to where the damper lands; noticeable with partial pedalling and release velocity.
- **Damping duration** (damper efficiency).
- **Last damper** (key number above which there are no dampers).
- **Mute** (damper pressure).
- **Key release noise**.
- **Sustain pedal noise** (velocity-dependent "whoosh").
- Re-pedalling after release.
- Repeated-note timbre change because the hammer strikes still-vibrating strings.

*Pedals*
- Progressive sustain (half or partial pedal), sostenuto, **harmonic pedal**, una corda.
- v9 adds the Thunder pedal.

*Radiation / mics*
- "Physical model of the soundboard and its radiation": the sound pressure is computed at the mic position (omni).
- Up to 5 mics in v4 and **8 mics with 3D position and directivity in v9** ([SOS v9](https://www.soundonsound.com/reviews/vendor-commercial-9?amp)).
- **Lid position**, part of the radiation model.
- Level and delay compensation.
- Stereo width.
- **Sound speed** in air (exposed as "time unit").
- **Binaural head model**: listener position, head rotation and head size.

*Dynamics and keyboard*
- Velocity curve, with a keyboard calibration assistant.
- Separate pedal and note-off velocity curves.
- Dynamics range in dB. The manual says measured acoustic pianos have about **50 dB** of dynamic range, and suggests 20–60 dB.
- Volume.

*Pre-processing EQ*
- The EQ acts on the model (per-partial), with automatic loudness compensation across the keyboard.

*Effects*
- Tremolo, wah, chorus, flanger, delay, amp, compressor, and reverb.

*Mallet bounce* (percussion and cimbalom)
- Initial delay, velocity sensitivity, delay loss, velocity loss, humanization.

*Pro "Note Edit"* (per-note curves)
- Tuning, unison width, hammer hardness, strike point, string length, spectrum profile **per partial per note**, soundboard impedance, damper position, direct sound duration.
- Includes interpolation between "control notes".

*Options*
- Internal sample rate (up to 192 kHz in Pro), polyphony, CPU-overload handling.

*v8 additions*
- External audio input feeding sympathetic resonance.
- Per-note vibrato and tremolo.
- — [Gearspace](https://gearspace.com/board/new-product-alert-2-older-threads/1393236-vendor-releases-commercial-8-a.html)

### Inferences
- The patent and the parameter set together suggest the following structure (inference, not confirmed by Vendor for current versions):
  1. Per note, a bank of damped partials whose frequencies come from a stiff-string model (string length → inharmonicity B).
  2. Decay rates come from a frequency-dependent bridge/soundboard impedance (impedance level, cutoff, Q-slope). This is the patent's "weighting constant, cut-off index, slope".
  3. Unison detuning gives beating and double decay ("direct sound duration").
  4. Velocity-indexed excitation spectra (hammer hardness per velocity, strike-point comb, spectrum profile) are applied as initial amplitudes.
  5. The damper state sets per-partial damping.
  6. A shared resonance network handles sympathetic and duplex resonance.
  7. A radiation/microphone model with lid and air propagation produces the output.
- The "polyphony includes sympathetic resonances" wording ([manual](https://www.strumentimusicali.net/manuali/VENDOR_COMMERCIAL4_ENG.pdf)) suggests sympathetic string excitations are rendered as extra voices.
- "Bass notes require more computation" fits additive or modal synthesis, where cost scales with the number of partials below Nyquist (more partials in the bass).
- The spectrum profile and EQ acting "before computing sound" are only possible in a parametric (modal/additive) engine, not a waveguide-filter engine. This supports the patent-described architecture still being in use.
- The patent's main independent claims expire around Oct 2027 (Google estimate). For a project distributed after that date this constraint disappears; before then, a metamodel-plus-additive architecture closely copying the claims should be checked by counsel.

### Gaps
- Could not read vendor.com (HTTP 403): the current v9 manual, the official changelog ("commercial#changes"), the FAQ and the history page. Exact version-by-version feature additions for v2–v7 (e.g. when "string length", "Hammer tap", morphing/layering, condition/"aging" controls, MIDI 2.0 claims arrived) remain **unverified**. Morphing and layering appear in a retailer listing without a version attribution.
- I found no peer-reviewed paper by [founder] (or Vendor staff) on piano modelling, and no thesis reference. His PhD title is known only paraphrased from CDM.
- I found no official polyphony or CPU benchmark figures beyond the 2010 anecdotes and manual guidance.
- Whether hammer, pedal and release noises are sampled is reported by Wikipedia only.

---

## Q2. Other commercial and open-source physically-modelled pianos and documented techniques

### Takeaway
Few commercial products are fully modelled: Commercial, Roland V-Piano (2009), Viscount Physis (2012–2014) and, by its own marketing, Arturia Piano V. Most digital pianos (Yamaha VRM, Roland SuperNATURAL, Kawai) are **samples plus modelled resonances**: modelled damper and sympathetic resonance, plus modelled sustain or decay. Open-source options range from commuted-waveguide synthesis (STK/Faust piano.dsp) to finite-difference engines (OpenPiano, alpha). A newer Faust "coopiano" models hammer, unison strings and a modal soundboard.

### Cited Findings

**Commercial products**
- **Roland V-Piano** (2009) — [Roland Engineering: V-Piano](https://articles.roland.com/roland-engineering-v-piano/); [SOS V-Piano review](https://www.soundonsound.com/reviews/roland-v-piano):
  - No samples ("number-crunching"), per SOS.
  - Technique per Tadao Kikumoto:
    - The three strings of a note are separated and modelled as separate "partials" (components).
    - Frame and body thump is isolated in the HF range and resynthesized.
    - Other partials are modelled individually, then combined with a supplemental virtual soundboard.
    - Built on COSM (as in V-Guitar), plus "Aesthetical Modeling" of subjective responses.
    - Multichannel output separates the player and audience perspectives.
  - Used later in FANTOM, RD-2000 and the V-Piano Grand.
- **Roland SuperNATURAL / Piano Reality**: samples combined with modelling. A modelling algorithm replaces sample looping for sustain and decay. Damper Modelling covers pedal noise and string/soundboard resonance. — [Roland Engineering article (search summary)](https://articles.roland.com/roland-engineering-v-piano/); [Pianist Magazine, "The Roland reality"](https://www.pianistmagazine.com/news/the-roland-reality/) (secondary)
- **Yamaha VRM (Virtual Resonance Modeling)**: simulates string and soundboard resonance interaction.
  - Original VRM computes the string state for all 88 keys, including damper pedal timing and depth.
  - Enhanced VRM adds aliquot resonance and soundboard/rim/frame resonance.
  - Developed with Yamaha's acoustic grand department.
  - The tone base is sampled (CFX/Bösendorfer).
  - — [Yamaha FAQ "What is VRM"](https://faq.yamaha.com/usa/s/article/000001306) (page did not render for fetch; content from search summary); [Sand, software and sound: Yamaha VRM technology](http://sandsoftwaresound.net/yamaha-vrm-technology/) (secondary blog)
- **Viscount Physis Piano**: fully physical model ("more than 100 elements of the acoustic grand piano sound").
  - Unveiled at Musikmesse 2012; products from 2014 (V100 hybrid upright, H-series).
  - Research is credited to three Italian universities.
  - — [MMR, "Physical modeling and the Physis Piano"](https://mmrmagazine.com/physical-modeling-and-the-physis-piano/); [PianoBuyer](https://www.pianobuyer.com/physis); [Nord user forum](https://www.norduserforum.com/viewtopic.php?p=14877) (forum)
  - Viscount's patent WO2013135627A1, "A system to reproduce the sound of a stringed instrument", cites Vendor's patent. — [Google Patents citations](https://patents.google.com/patent/US[vendor patent]B2/en)
- **Arturia Piano V**: marketed as a real-time simulation of the physics and acoustics of a piano.
  - 12 models.
  - Exposed parameters: hammer action, **string age**, **unison detuning**, **stretch tuning**, lid position, 4 macros.
  - No technical paper found. — [Arturia Piano V overview](https://www.arturia.com/products/software-instruments/piano-v/overview)
- **Kawai**: no documented physical-modelling engine found (see Gaps). Two Kawai patents (US4387622A, JPH10247084A) appear only as citations in the Vendor patent family. — [Google Patents](https://patents.google.com/patent/US[vendor patent]B2/en)
- **OneTrick KEYS** (Faust-powered) uses Stanford's 1995 commuted piano synthesis. — [Faust "Powered by Faust"](https://faust.grame.fr/community/powered-by-faust/)

**Open source and research**
- **STK / faust-stk piano.dsp** — [faust-stk DAFx 2011 paper](https://ccrma.stanford.edu/~rmichon/publications/doc/DAFx11-Faust-STK.pdf); [grame-cncm/faust faust-stk examples](https://github.com/grame-cncm/faust/tree/master-dev/examples/physicalModeling/faust-stk):
  - Commuted waveguide synthesis below E6; modal (biquad bank) synthesis at E6 and above.
  - Derived from a late-1990s CCRMA SynthBuilder patch, partly ported to STK by Stephen Sinclair (2006).
  - Monophonic per instance.
- **sletz/faust-coopiano**: Faust port of the "coopiano" physical piano.
  - Hammers, unison strings, **modal soundboard**, reverb.
  - Polyphonic, web version, checked against a C++ reference, MIT license.
  - — [GitHub sletz/faust-coopiano](https://github.com/sletz/faust-coopiano)
- **OpenPiano** (michele-perrone): JUCE plugin, **AGPL-3.0**, alpha.
  - **Finite-difference** lossy stiff string with hammer.
  - One string per note.
  - Soundboard, multi-string, double decay and sympathetic resonance only planned.
  - — [GitHub OpenPiano](https://github.com/michele-perrone/OpenPiano)
- **Felucca** project issue: a comparison of the Faust waveguide piano against a sample piano (size, RAM, CPU). — [GitHub kmplngj/Felucca #231](https://github.com/kmplngj/Felucca/issues/231)
- **Bank's physics-based piano** (BME Budapest thesis) — [Bank thesis abstract](https://home.mit.bme.hu/~bank/thesis/thesabs.html):
  - Digital-waveguide strings.
  - Nonlinear hammer stabilized by a **multi-rate** implementation.
  - Soundboard filters that cost 10–100× the string and hammer models.
  - Built from measured piano parts.
- **Bank, Zambon, Fontana**, "A modal-based real-time piano synthesizer" (2010), is listed as similar art. — [Google Patents "similar documents"](https://patents.google.com/patent/US[vendor patent]B2/en)
- **Bensa et al.**:
  - Hybrid source (subtractive signal model) plus waveguide resonator — [EURASIP JASP](https://asp-eurasipjournals.springeropen.com/articles/10.1155/S1110865704402121).
  - Power-normalized nonlinear lossy hammer (Bensa, Bilbao, Kronland-Martinet, Smith) — [search result](https://ccrma.stanford.edu/~bilbao/booktop/node232.html).
- **Chabassier, Chaigne, Joly**, "Modeling and simulation of a grand piano", JASA 134:648–665 (2013) — [HAL hal-00873089](https://hal-ensta-paris.archives-ouvertes.fr/hal-00873089); [ESAIM M2AN 2014](https://numdam.org/articles/10.1051/m2an/2013136/):
  - Full time-domain model: nonlinear strings with phantom partials and precursors, hammer, bridge, soundboard and radiation.
  - Schur complement and Lagrange decoupling.
  - Compared with **Steinway D recordings**.
- Newer **neural/differentiable** piano models:
  - DDSP-Piano (Renault, Mignot, Roebel, DAFx 2022): differentiable polyphonic model with inharmonicity and tuning priors. — [DAFx paper](https://dafx2020.mdw.ac.at/Proceedings/papers/DAFx20in22_paper_48.pdf)
  - Simionato et al., "Sines, transient, noise neural modeling of piano notes" (2024/25). — [arXiv 2409.06513](https://arxiv.org/pdf/2409.06513)
  - Simionato et al., "Physics-informed differentiable method for piano modeling" (2024). — [Frontiers in Signal Processing](https://www.frontiersin.org/journals/signal-processing/articles/10.3389/frsip.2024.1494864/full)

**Patents relevant to resonance and damper modelling**
- **Yamaha** (cite or are cited with the Vendor family):
  - US8530736B2 / US20120137857A1, "Musical tone signal synthesis method…".
  - US8115092B2, "Method for synthesizing tone signal…".
  - JP6819309B2, "Resonance signal generator…".
  - — [Google Patents](https://patents.google.com/patent/US[vendor patent]B2/en)
- A blog also attributes US 8,878,045 and US 8,729,376 to the VRM context (unverified). — [Sand, software and sound](http://sandsoftwaresound.net/yamaha-vrm-technology/)
- **Casio** resonance and tone-generation patents: JP6801443B2, JP6540681B2, JP7230870B2. — [Google Patents](https://patents.google.com/patent/US[vendor patent]B2/en)
- **US 5,744,743** (1998), "Pedal resonance effect simulation device for digital pianos": sympathetic resonance via delay lines. Assignee not confirmed in my sources. — [Justia](https://patents.justia.com/patent/5744743)

### Inferences
- Two families appear among commercial engines:
  - "Analytic or component" models: Roland COSM separates unison strings, body thump and soundboard.
  - "Modal or parametric" models: Commercial's patent, and plausibly Physis.
- Mass-market digital pianos are converging on sampled attack plus modelled resonance and damping. This suggests the hardest parts to sample convincingly are pedal and sympathetic resonance and partial damping. Those are where a physics engine adds most value.
- Arturia's exposed parameters (string age, unison detuning, stretch) and Commercial's checklist overlap strongly. That gives a de-facto "feature parity" list for a new synthesizer.

### Gaps
- Kawai: no documented physical-modelling engine or patent retrieved. Its marketing names "Harmonic Imaging" (sampling) and resonance features, but none were confirmed in this session.
- No Roland patent numbers retrieved for V-Piano or damper modelling.
- Native Instruments, AAS ("Physical Audio"), "Pianoid", "PianoSynth" (Stanford) and Rust crates were not covered.
- MDA Piano was not checked. My understanding is that it is sample-based, but this is unverified.
- Physis technical details are only available from marketing.

---

## Q3. Publicly available data for calibration

### Takeaway
For **performance-level** evaluation (MIDI plus audio), MAESTRO (~200 h, CC BY-NC-SA 4.0), MAPS (65 h, CC BY-NC-SA 2.0 FR) and SMD (CC BY-NC-SA 3.0) are standard, all Disklavier-based and non-commercial. For **single notes**, the Salamander Grand (Yamaha C5, 16 velocity layers, 48 kHz/24-bit, CC-BY 3.0, but see the license conflict) and Iowa MIS (Steinway, stereo, "without restriction", with a caveat) are the most permissive. Measured physical data (bridge admittance, hammer curves, inharmonicity) exist mainly as **figures in papers** (Conklin 1996, Giordano, Wogram 1980, Chabassier 2013), not as downloadable datasets.

### Cited Findings
- **MAESTRO** — [Magenta MAESTRO page](https://magenta.tensorflow.org/datasets/maestro); [Hawthorne et al. 2019, arXiv 1810.12247](https://arxiv.org/pdf/1810.12247):
  - Yamaha Disklavier recordings from the International Piano-e-Competition, with aligned MIDI including pedals.
  - CC BY-NC-SA 4.0 (Google LLC).
  - v1.0.0 = 172.3 h; current versions ~200 h.
- **MAPS** (Emiya, Badeau, David, 2010) — [Telecom Paris ADASP MAPS page](https://adasp.telecom-paris.fr/resources/2010-07-08-maps-database/); [content summary](https://transactions.ismir.net/articles/162):
  - 65 h of aligned audio and MIDI.
  - Mostly software-synthesized pianos plus Yamaha Disklavier recordings.
  - Includes isolated notes and chords.
  - License **CC BY-NC-SA 2.0 France**.
- **SMD (Saarland Music Data), MIDI-Audio Piano Music** — [SMD page](https://resources.mpi-inf.mpg.de/SMD/SMD_MIDI-Audio-Piano-Music.html); [AudioLabs SMD MIDI](https://www.audiolabs-erlangen.de/resources/MIR/SMD/midi); [ISMIR 2011 LBD](https://ismir2011.ismir.net/latebreaking/LB-5.pdf):
  - Yamaha Disklavier DCFIIISM4PRO, Hochschule für Musik Saar.
  - **CC BY-NC-SA 3.0**.
  - WAV on Zenodo.
- **SMD-synth** (Fraunhofer IDMT): synthesized variant, CC BY-NC-SA 3.0. — [Zenodo 4637908](https://zenodo.org/records/4637908)
- **Salamander Grand Piano V3** (Alexander Holm) — [sfzinstruments/SalamanderGrandPiano](https://github.com/sfzinstruments/SalamanderGrandPiano); [Internet Archive](https://archive.org/details/SalamanderGrandPianoV3); [SFZ Instruments pianos](https://sfzinstruments.github.io/pianos/):
  - Yamaha C5, two AKG C414 in AB about 12 cm above the strings.
  - 48 kHz/24-bit, 16 velocity layers.
  - CC-BY 3.0 per GitHub and Internet Archive, but a Purgatory Creek page says **CC BY-SA 3.0 US**. — [Purgatory Creek](https://www.purgatorycreek.com/index.php/acoustic-piano-collection/). Check the LICENSE in the specific download.
- **University of Iowa MIS** — [Iowa MIS](https://theremin.music.uiowa.edu/MIS.html); [Korg forum quoting a 2008 email](https://korgpahelper.yolasite.com/piano-discussion.php) (secondary):
  - Free "for any projects, without restrictions" since 1997.
  - The piano is the only non-anechoic instrument, recorded in a small teaching studio, in stereo, 16-bit/44.1 kHz AIFF.
  - Caveat: a 2008 email from L. Fritts reportedly restricted the *Steinway* set to non-commercial use on Korg Pa keyboards.
- **Chabassier et al. 2013**: model compared with recorded **Steinway D** waveforms. I did not establish whether the recordings or parameters are distributed. — [HAL hal-00873089](https://hal-ensta-paris.archives-ouvertes.fr/hal-00873089)
- **Soundboard and bridge mobility**:
  - Conklin (1996) bridge mobility of a grand.
  - Giordano's upright point mobility, described as the only reliable, well-documented point mobility measurements. A mean impedance of about 1500 kg/s at C4 is cited.
  - Wogram (1980) uprights, around 10^3 kg/s.
  - All available as **published curves**, not data files.
  - — [arXiv 1210.5109 (Ege, Boutillon, Rébillat)](https://arxiv.org/html/1210.5109); [arXiv 1210.5688](https://arxiv.org/pdf/1210.5688); [ResearchGate figure](https://www.researchgate.net/figure/Bridge-mobility-direction-normal-to-the-soundboard-of-a-grand-piano-at-the-terminating_fig5_301850959)
- **Acta Acustica 2022** (Elie et al.), "Physically-based sound synthesis software for Computer-Aided-Design of piano soundboards": computes bridge mobility for a reference piano and compares it with Giordano. The PDF was 403 to fetch, so software or data availability is unknown. — [Acta Acustica aacus210096](https://acta-acustica.edpsciences.org/articles/aacus/pdf/2022/01/aacus210096.pdf); [ResearchGate](https://www.researchgate.net/publication/362255835_Physically-based_sound_synthesis_software_for_Computer-Aided-Design_of_piano_soundboards)
- **Commercial manual**: measured acoustic pianos show about **50 dB** dynamic range. This is a useful target for velocity-to-loudness calibration. — [Commercial 4 manual](https://www.strumentimusicali.net/manuali/VENDOR_COMMERCIAL4_ENG.pdf)

### Inferences
- A practical calibration pipeline:
  1. Fit per-note inharmonicity B, partial decay rates and unison detuning from **Salamander** (16 velocities, ideal for hammer-hardness-vs-velocity fitting) and **Iowa MIS** (stereo Steinway).
  2. Check bridge-impedance magnitudes against Giordano and Conklin curves digitized from papers.
  3. Validate whole performances (pedalling, sympathetic resonance) on **MAESTRO, SMD or MAPS** via MIDI rendering and comparison.
  4. Note that the non-commercial licences restrict their use to evaluation or research, not to bundled assets.
- MAESTRO's Disklavier MIDI includes continuous sustain-pedal data. That makes it a good stress test for half-pedal and damper-position modelling, which are distinctive Commercial features.

### Gaps
- **OMAPS** dataset: not found or verified in this session.
- **Conklin inharmonicity tables** and **Giordano hammer/soundboard data**: no downloadable files found. The original papers would need to be digitized.
- **Bank's measured data**: no public download located. Only the thesis abstract was found.
- Hammer force/compression datasets (e.g. Stulov, Giordano–Millis) were not covered.
- Chabassier's HAL entry was not checked for supplementary data.
- License of the Chabassier Steinway D recordings is unknown.

---

## Q4. Evaluation methods (objective and perceptual), including Commercial versus sampled pianos

### Takeaway
Recent piano-synthesis papers evaluate with **MUSHRA or MUSHRA-like tests** (about 10–20 expert listeners, low-passed anchor, Friedman or Wilcoxon statistics) plus objective metrics such as **FAD** (VGGish, CLAP or EnCodec embeddings, so scores are not comparable across papers), spectrogram and chroma distances, and partial-tracking metrics. **Commercial is widely used as the "high-quality reference renderer"** in MIDI-to-audio research and is usually rated above neural systems. I found no independent perceptual study of Commercial versus sampled pianos.

### Cited Findings
- **Simionato et al.** ("Sines, transient, noise neural modeling of piano notes"):
  - MUSHRA with 20 trained musicians, professional headphones, quiet room.
  - Single anchor = reference low-passed at 1 kHz.
  - Friedman test and Cliff's delta.
  - Also tests trichord (unison) behaviour.
  - — [arXiv 2409.06513](https://arxiv.org/pdf/2409.06513)
- **DDSP-Piano** (DAFx 2022): a listening test found it beats a neural benchmark with fewer parameters, but **physical-modelling systems still scored higher on quality**. — [DAFx 2022 paper](https://dafx2020.mdw.ac.at/Proceedings/papers/DAFx20in22_paper_48.pdf)
- **MIDI velocity in-filling** (2025): MUSHRA-like test, 11 experts, all stimuli rendered with **Commercial 8 Steinway D**. — [arXiv 2508.07751](https://arxiv.org/pdf/2508.07751)
- **Resynthesis of transcriptions** (2022): MUSHRA-like test with candidates rendered with **Commercial Grand Steinway**. — [PMC9007253](https://pmc.ncbi.nlm.nih.gov/articles/PMC9007253/)
- **Integrated expressive piano rendering** (2025): Commercial used as the reference. It beat the authors' fine-tuned MIDI-to-audio system on chroma and spectrogram distances. Pedal and key noises of the neural system still trail Commercial. — [arXiv 2501.10222](https://arxiv.org/pdf/2501.10222); [Emergent Mind summary](https://www.emergentmind.com/topics/atepp-dataset) (secondary)
- **MIDI-VALLE** (ISMIR 2025): FAD improved by over 75% versus a baseline on ATEPP and MAESTRO, plus a listening test. A review notes it lacks a Commercial baseline. — [ISMIR 2025 program](https://ismir2025program.ismir.net/poster_208.html); [Zenodo](https://zenodo.org/records/17811447)
- **FAD tooling**: the `frechet-audio-distance` PyPI package supports VGGish, PANN, CLAP and EnCodec embeddings. — [PyPI](https://pypi.org/project/frechet-audio-distance/)
- Embedding choice changes FAD values, so scores across piano papers are not directly comparable. — [arXiv 2609.18216](https://arxiv.org/pdf/2609.18216); [arXiv 2608.01920](https://arxiv.org/pdf/2608.01920)
- A survey of generative-audio metrics uses MUSHRA rating bands and Wilcoxon tests. — [arXiv 2209.00130](https://arxiv.org/pdf/2209.00130)
- **Chabassier et al.** validated a physical model by waveform and spectral comparison with Steinway D recordings in low, mid and high registers, including phantom partials and precursors. — [HAL hal-00873089](https://hal-ensta-paris.archives-ouvertes.fr/hal-00873089)

### Inferences
- A suggested two-tier protocol for a physics-based synthesizer:
  - **Objective, per note**:
    - Partial frequencies, hence fitted B and stretch.
    - Per-partial T60 and double-decay (two-slope) parameters.
    - Beating from unison detuning.
    - Attack spectral centroid versus velocity.
    - Precursor and phantom-partial presence.
    - These are compared against Salamander or Iowa notes.
  - **Objective, per performance**: FAD (report the embedding), multi-resolution STFT and chroma distances on MAESTRO, SMD or MAPS MIDI rendered through the synth, with **Commercial renders as a strong baseline**.
  - **Perceptual**: MUSHRA (≥15 trained listeners, 1 kHz LP anchor, hidden reference = recording) with Friedman and post-hoc tests, following Simionato et al.
- Because Commercial is the de-facto reference in the MIR literature, including "Commercial render" as a MUSHRA condition makes results comparable with that literature.

### Gaps
- No published perceptual study comparing Commercial against sampled pianos (or against recordings) was found.
- No published partial-decay perceptual-threshold study specific to physical piano models was retrieved this session. JNDs for inharmonicity or decay (e.g. Järveläinen et al.) were not checked.
- Exact listening-test sizes for DDSP-Piano were not extracted.
