//! Soundboard: modal plate, bridge admittance, radiation to microphones.
//!
//! The board is an orthotropic plate (spruce + ribs) on a rectangle a × b,
//! clamped in the rim. Frequencies use Warburton's approximation for clamped
//! edges (index m → m + ½), shapes the sine approximation:
//!
//! ```text
//! ω_mn = π²/√(ρh)·√(Dx(m'/a)⁴ + 2H(m'/a)²(n'/b)² + Dy(n'/b)⁴),   H = √(Dx·Dy), m' = m + ½
//! Φ_mn(x, y) = 2/√(ρh·a·b)·sin(mπx/a)·sin(nπy/b)   (mass-normalised)
//! ```
//!
//! Simply supported edges would put the first mode near 27 Hz, on A0's
//! fundamental; clamping puts it near 60 Hz, as measured on grands.
//!
//! and loss factor η (Ege et al. 2013: ≈ 2 %). The driving-point mobility
//! at a bridge point is `Y(ω) = Σ iω·Φ²/(ω_n² − ω² + iη·ω_n·ω)` (e^{iωt}
//! convention, Re Y > 0); its mean over frequency is the infinite-plate
//! value 1/(8√(D·ρh)). ρh/D sets the modal density (≈ 1 mode / 20 Hz, Ege
//! 2013); ρh·D sets the mobility level, calibrated on the decay statistics
//! of Salamander (docs/phase4.md): 9.4e-4 s/kg, of the order of Ege's
//! measured 1.3e-3. The strings see the plate through a series compliance
//! (pins, bridge cap) and optionally a local bridge mass.
//!
//! Radiation: each mode's velocity field radiates through the Rayleigh
//! integral, `p = iωρ₀/(2π)·∫ v·(e^{−ikR}/R − β·e^{−ikR'}/R') dS`, where the
//! second term is the back face (antiphase, image at the case depth, R' its
//! distance): an unbaffled board short-circuits its low frequencies. A
//! wall behind the board (an upright's) reflects that back face's sound
//! once more towards the mics.
//! Evaluated once per mode and microphone at the mode frequency, and
//! rendered on the modal acceleration ([`crate::board::to_acceleration_weights`]),
//! so that a mode driven below its resonance radiates at the driving
//! frequency's rate.

use crate::complex::C64;
use std::f64::consts::{PI, TAU};

const RHO0: f64 = 1.2;
const C0: f64 = 343.0;

#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
pub struct SoundboardParams {
    /// Plate dimensions (m).
    pub a: f64,
    pub b: f64,
    /// Surface mass (kg/m²), bending stiffnesses (N·m) along a and b.
    pub rho_h: f64,
    pub dx: f64,
    pub dy: f64,
    pub loss_factor: f64,
    /// Loss factor growth with frequency: η(f) = η·(1 + f/loss_corner).
    pub loss_corner: f64,
    /// Extra loss of the low modes, which work the board's edge, the rim
    /// and the case: η(f) is multiplied by 1 + (loss_low_corner/f)² (Hz;
    /// 0: none).
    pub loss_low_corner: f64,
    /// Loss through the rim and the case, seen by the strings whose bridge
    /// runs near the board's edge (the bass bridge; the middle of the main
    /// bridge crosses the board's centre): a conductance edge_loss (s/kg)
    /// added to the bridge mobility, weighted by e^(−d/edge_depth) with d
    /// the bridge point's distance to the nearest edge (m), falling above
    /// edge_corner (Hz) as 1/(1 + (f/edge_corner)²). It damps the strings
    /// and radiates nothing. 0: none.
    pub edge_loss: f64,
    pub edge_depth: f64,
    pub edge_corner: f64,
    /// Effective bridge mass at the string's attachment point (kg): the
    /// strings see Y/(1 + iω·m·Y) and the board receives F/(1 + iω·m·Y).
    pub bridge_mass: f64,
    /// Stiffness (N/m) in series between string and bridge (pins, cap):
    /// adds iω/k to the admittance seen by the strings.
    pub series_stiffness: f64,
    /// The bridge is a stiff hardwood beam glued on the board: a string's
    /// force reaches the board spread along it, over `bridge_spread` times
    /// the bridge's bending wavelength 2π·(EI/(μω²))^¼ (capped at 0.5 m).
    /// Short board waves (high modes) then average out: this is what makes
    /// the bridge conductance rise slowly with frequency. EI (N·m²) and line
    /// mass μ (kg/m) of the bridge; EI = 0 drives the board at a point.
    pub bridge_stiffness: f64,
    pub bridge_line_mass: f64,
    pub bridge_spread: f64,
    /// The bridges as beams in the board's own modes (0: off): their
    /// bending stiffness `bridge_stiffness` along their curve and their
    /// mass `bridge_line_mass`, times this factor, glued along the main and
    /// the bass bridge's lines. Neighbouring points of a bridge then move
    /// together. They act on the outlined board's own modes, below
    /// `outline_below` (docs/bridge_beam.md).
    pub bridge_beam: f64,
    /// Boards are thicker and more densely ribbed under the treble: the
    /// impedance under the bridge grows as e^{slope·(key − 60)} (the uniform
    /// plate sets its value at C4). Applied to the shapes through which the
    /// strings drive the board, so the board also answers less to them.
    pub impedance_slope: f64,
    /// The same slope below C4 (the bass side of the board). A positive
    /// slope (bass side more mobile) made its low modes boom (+13 dB at
    /// 100–200 Hz over Salamander, "a barrel"); a slightly negative one
    /// (stiffer bass side, ×2.2 at A0) keeps the bass sustain once the
    /// crown has packed the low modes between 150 and 300 Hz
    /// (docs/bass_radiation.md). 0: the stiffer bass side left the bass's
    /// attack 7.6 dB under the middle's, against 3.8 on eight real grands
    /// (4.7 now, docs/bass_decay.md).
    pub impedance_slope_bass: f64,
    /// Highest mode kept (Hz).
    pub f_max: f64,
    /// The board is not held by an infinitely rigid rim: board, rim and
    /// case also move as a block on the case's compliance, a heavily damped
    /// "piston" mode (uniform shape) that absorbs and radiates the bass
    /// fundamentals, below the first clamped mode. Frequency (Hz, 0: none),
    /// loss factor and effective mass (kg).
    pub piston_freq: f64,
    pub piston_loss: f64,
    pub piston_mass: f64,
    /// Radiation of the piston mode relative to a bare board moving alone:
    /// when the whole case moves, the air pushed above is largely returned
    /// below (strong dipole cancellation).
    pub piston_radiation: f64,
    /// Crown and string down-bearing make the board a pre-stressed shell:
    /// its membrane stiffness raises the lowest modes, little the others.
    /// Modelled as an elastic foundation, ω² → ω² + (2π·crown_freq)²
    /// (Hz, 0: flat plate).
    pub crown_freq: f64,
    /// Main bridge: a smooth curve through five points in normalised plate
    /// coordinates, from C8 down to the last note of the main bridge; then
    /// the bass bridge, from its highest key to A0.
    pub treble_bridge: [(f64, f64); 5],
    pub bass_bridge: [(f64, f64); 2],
    /// The bass bridge's bow: an arc of a circle through its two ends,
    /// this far (m) from their chord at its middle, towards the tail when
    /// positive; its keys at equal steps along it. 0: straight, and what a
    /// preset saved before the bow existed reads, so that it keeps its bridge.
    #[cfg_attr(feature = "serde", serde(default))]
    pub bass_bridge_bow: f64,
    /// The main bridge's strings lean towards the bass side as they run
    /// back from the keys, as on real grands (the bass strings, crossing
    /// over them, lean the other way), and fan out: the tenor's are
    /// oblique, the treble's nearly upright. This angle (degrees, from the
    /// keyboard's normal) at the main bridge's lowest key, falling in
    /// equal steps of keys to 0 at C8 ([`Self::string_slant`]). Each
    /// string turns about its key's place on [`KEY_LINE_X`], keeping its
    /// length: `treble_bridge` gives the bridge of upright strings, and
    /// its points move towards the bass side by the string's length times
    /// its angle's sine. 0: upright strings, and what a preset saved
    /// before the slant existed reads, so that it keeps its bridge.
    #[cfg_attr(feature = "serde", serde(default))]
    pub main_string_slant: f64,
    /// First note on the main bridge (lower notes are on the bass bridge).
    pub main_bridge_from: u8,
    /// Microphones (m, plate coordinates, z above the board).
    pub mics: [(f64, f64, f64); 2],
    /// The microphones used instead of `mics` when the board is vertical
    /// (an upright): a listening set-up places them for both kinds of
    /// piano. The same coordinates: x down from the board's top, y across
    /// (the bass at y = b, on the listener's left), z in front of the board.
    pub mics_upright: [(f64, f64, f64); 2],
    /// `mics` x and y are fractions of the board's a and b (z stays in
    /// metres): a listening preset then fits every board size.
    pub mics_relative: bool,
    /// Gain of the listening set-up (dB), on top of `output_gain_db` (the
    /// instrument's own level): closer microphones hear the piano louder.
    pub mic_gain_db: f64,
    /// Microphone pattern a + (1 − a)·cos θ, aimed at the piano: 1 omni,
    /// 0.5 cardioid. The direct sound arrives on axis; the room's sound from
    /// every direction, which a cardioid hears with a third of the energy.
    pub mic_pattern: f64,
    /// Render the radiated pressure (false: output the bridge force, the
    /// board only sets the string coupling — for analysis).
    pub radiate: bool,
    /// Grid step of the radiation integral (m).
    pub grid_step: f64,
    /// The board radiates from both faces: the back face, in antiphase,
    /// reaches the mics from `back_depth` m below (around the case), with
    /// `back_gain` (partial shielding by rim and keybed). This acoustic short
    /// circuit makes the low modes radiate as dipoles, not as the monopoles
    /// of an infinite baffle. back_gain = 0 is the baffled plate.
    pub back_depth: f64,
    pub back_gain: f64,
    /// A reflecting wall parallel to the board, `wall_distance` m behind
    /// its back face (0: none), as behind an upright: the back face's sound
    /// reaches the mics a second time, after 2·wall_distance more and with
    /// the wall's pressure reflection coefficient `wall_reflection`.
    pub wall_distance: f64,
    pub wall_reflection: f64,
    /// An upright's board stands vertically (its x axis pointing down from
    /// the top, its front face towards the room): this places it and the
    /// microphones in the room, against the room's wall when
    /// `wall_distance` > 0. The radiation itself is the same.
    pub vertical: bool,
    /// The closed case in front of an upright's board (action, keys and
    /// panels): share of the front face's sound that leaves through its
    /// openings (1: no case, or panels off), the rest passing through the
    /// panels by the mass law, τ = 1/√(1 + (ω·m/2ρc)²), with
    /// `case_panel_mass` (kg/m²).
    pub case_open: f64,
    pub case_panel_mass: f64,
    /// Lid opening angle (degrees); 0: shut on the case. It is hinged
    /// along the board's long side y = b, `lid_hinge_height` m above the
    /// board, has the case's shape ([`Self::lid_outline`]), and opens
    /// towards y < 0 (the audience). Shut, the board's front face sounds
    /// into the box the case and the lid close, 16 cm high: below its
    /// height resonance c/2h (≈ 1070 Hz) board, air and lid move together
    /// and the sound gets out round the rim and through the lid; above, the
    /// box rings and its felts and wood take a growing share. Gain on the
    /// front face: `lid_leak` + (1 − `lid_leak`)/√(1 + (f·2h/c)²), mixed
    /// in as the lid closes over its last [`LID_CLOSING`] degrees
    /// ([`Self::lid_closure`]). Not on an upright (`case_open`).
    pub lid_angle: f64,
    /// No lid at all (taken off the case): `lid_angle` is then unused.
    /// Before 2026-10-31 an angle of 0 meant this; a preset saved then
    /// keeps its removed lid ([`deserialize_saved`]).
    pub lid_removed: bool,
    pub lid_hinge_height: f64,
    /// Pressure reflection coefficient of the lid, and transmission through
    /// it for a microphone behind it (no diffraction modelled).
    pub lid_reflection: f64,
    pub lid_transmission: f64,
    /// Share of the front face's sound that still leaves the shut case at
    /// high frequencies (the gaps, the opening behind the music desk).
    pub lid_leak: f64,
    /// The strings resting on the bridge with their dampers down absorb
    /// what reaches them: each loads the bridge, on average over frequency,
    /// with its characteristic impedance √(T·μ) (a lossy string's input
    /// resistance averages to it). Scale on that load (0: none). It damps
    /// the board's modes where the bridge works them, most below ≈ 1 kHz,
    /// which smooths its response from key to key (docs/evenness.md).
    pub string_load: f64,
    /// Share of that load the board keeps while the pedal lifts the
    /// dampers. A free string still loads the bridge: its input resistance
    /// Re(Z0·coth γL), averaged over frequency, is √(T·μ) as for a damped
    /// one, only gathered at its partials. The sympathy is one-way (the
    /// listening strings take nothing back from the board), so this load
    /// is the energy they sing back. 0: the pedal leaves the board bare, and
    /// its first mode droned (+9 dB at 100 Hz on Liang's pedalled Chopin
    /// op. 10 no. 3, the bass's pedal halo 8.7 dB over the Disklavier's;
    /// docs/pedal_drone.md).
    pub string_load_free: f64,
    /// Board modes below this frequency (Hz) are driven at each key's own
    /// bridge point instead of its region's ([`crate::board::LowDrive`]):
    /// a more even board from key to key, for a bank of ≈ 40–70 more modes.
    /// 0: off.
    pub exact_below: f64,
    /// The board's outline, a polygon in normalised plate coordinates (as
    /// the bridges) inside the a × b rectangle: a grand's board follows the
    /// bentside and narrows towards the tail. Unused vertices repeat the
    /// last one; no area (the default, all zeros): the full rectangle.
    pub outline: [(f64, f64); OUTLINE_POINTS],
    /// Below this frequency (Hz) the board has the outlined plate's own
    /// modes (Rayleigh–Ritz on the rectangle's, [`Soundboard::new`]);
    /// above, the rectangle's, radiating from inside the outline only.
    pub outline_below: f64,
    /// Ribs glued under the board, parallel to its short side and equally
    /// spaced along its length: their spacing (m; 0: a plain plate) and
    /// the share of `dy` they carry. The plate keeps its smeared stiffness
    /// `dy`; the ribs' bending stiffness is added where they lie and taken
    /// away between them, nothing on the long waves, which confines the
    /// modes above the frequency where half a bending wave fits between
    /// two ribs, as on a real board (docs/board_radiation.md). They act on
    /// the outlined board's own modes, below `outline_below`.
    pub rib_spacing: f64,
    pub rib_share: f64,
    /// Irregular ribs, as a real board's are (0: equally spaced and alike):
    /// each rib moves by up to ± half this share of the spacing and its
    /// stiffness varies by up to ± this share, from a fixed pseudo-random
    /// sequence (their mean stays `rib_share`). Periodic ribs make waves
    /// that spread over the whole board; irregular ones confine them.
    pub rib_jitter: f64,
    /// What lies over the board (strings, frame, dampers, bridges)
    /// scatters its sound in every direction, where the board itself, a
    /// plate, sends its high frequencies straight up and almost nothing
    /// along its surface, towards a listener at the keyboard. A layer of
    /// secondary sources `scatter_step` m apart, at heights spread about
    /// `scatter_height` m, each re-radiating the board's velocity under
    /// it with the efficiency (f/scatter_corner)²/(1 + (f/scatter_corner)²)
    /// of small obstacles, times `scatter` (0: none). Their spread in
    /// height makes them add in power rather than in phase above
    /// ≈ 2 kHz (docs/board_radiation.md).
    pub scatter: f64,
    pub scatter_corner: f64,
    pub scatter_step: f64,
    pub scatter_height: f64,
    /// What the layer scatters it first takes from the board's direct
    /// sound: the share of the direct field's power the layer intercepts
    /// at full scattering (the same frequency law), 0–1.
    pub scatter_shadow: f64,
}

/// Vertices of [`SoundboardParams::outline`].
pub const OUTLINE_POINTS: usize = 10;

/// How far the lid reaches past the board's edge (m): over the rim, as
/// thick as [`crate::structure::RimParams`]' default.
pub const LID_OVERHANG: f64 = 0.075;

/// The last degrees of the lid's travel (a 13 cm gap at the front edge of
/// a 1.5 m wide case), over which it goes from open to shut: its image and
/// shadow fade into the shut box's gain, so that closing it is continuous.
pub const LID_CLOSING: f64 = 5.0;

impl Default for SoundboardParams {
    fn default() -> Self {
        Self::GRAND_2M
    }
}

/// A saved soundboard, its lid read as presets wrote it before 2026-10-31
/// too: without `lid_removed`, an angle of 0 was a removed lid and
/// `lid_shut` (0–1) a shut one.
#[cfg(feature = "serde")]
pub fn deserialize_saved<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<SoundboardParams>, D::Error> {
    #[derive(serde::Deserialize)]
    struct Saved {
        #[serde(flatten)]
        p: SoundboardParams,
        lid_removed: Option<bool>,
        lid_shut: Option<f64>,
    }
    let saved: Option<Saved> = serde::Deserialize::deserialize(d)?;
    Ok(saved.map(|s| {
        let mut p = s.p;
        match (s.lid_removed, s.lid_shut) {
            (Some(removed), _) => p.lid_removed = removed,
            (None, Some(shut)) if shut >= 0.5 => (p.lid_removed, p.lid_angle) = (false, 0.0),
            (None, _) => p.lid_removed = p.lid_angle <= 0.0,
        }
        p
    }))
}

impl SoundboardParams {
    /// Scales the mobility by `s` at constant modal density (ρh/D kept).
    pub fn with_mobility_scale(mut self, s: f64) -> Self {
        self.rho_h /= s;
        self.dx /= s;
        self.dy /= s;
        self
    }

    /// A 2 m grand: ≈ 1.7 × 1.3 m board, ribbed spruce (surface mass and
    /// stiffness include ribs and bridges). Coupling parameters calibrated
    /// on Salamander (tools/coupling_grid.py, tools/board_grid.py).
    pub const GRAND_2M: Self = Self {
        a: 1.7,
        b: 1.3,
        // Surface mass and stiffnesses × 1.2 since the board has its grand
        // outline (82 % of the rectangle): the modes above `outline_below`
        // are the rectangle's, mass-normalised on it, so a fifth of their
        // mass lies outside the outline; the same D/ρh keeps the modal
        // frequencies and the calibrated mobility (docs/outline.md). Then
        // × 1.25 more, the same D/ρh: the board's conductance at the
        // fundamentals C3–C6 averaged 1.48 × the strings' measured decays
        // (tools/bridge_conductance.py), 1.17 × now; the middle's and the
        // treble's sustain at 1–2 s come onto the real grands
        // (docs/board_hump.md).
        rho_h: 10.5,
        dx: 5737.5,
        dy: 2550.0,
        loss_factor: 0.02,
        // Wood and radiation damping grow with frequency: η = 0.02·(1 +
        // f/2500 Hz). The share of its energy the board radiates falls as
        // 1/η; the earlier corner at 700 Hz (13 % at 4 kHz) muffled the
        // 2–8 kHz bands (docs/brightness.md).
        loss_corner: 2500.0,
        // No extra loss on the modes themselves (it sped the middle's
        // fundamentals too, −21 dB/s against −12, docs/bass_decay.md): the
        // rim and the case take their energy where the bridge runs near the
        // edge. 1e-3 s/kg within ≈ 15 cm of it, fading above 400 Hz: on
        // every key of A0–D#4, partials 1–3 decay over the first 1.5 s
        // within 1–4 dB/s of eight real grands (the middle's partial 1 at
        // −12.3 dB/s against −12.2). Only the lowest modes get more: at
        // 200 Hz, η × 3.7 at 110 Hz and × 1.6 at 260 Hz. A1's second partial
        // lies on the first mode (108.8 Hz) and kept ringing between the
        // left hand's notes in Beethoven (docs/bass_metal.md).
        loss_low_corner: 200.0,
        edge_loss: 1e-3,
        edge_depth: 0.15,
        edge_corner: 400.0,
        bridge_mass: 0.0,
        series_stiffness: 2e7,
        // Maple, ≈ 3 × 2.5 cm; spread over 0.3 bending wavelength (set with
        // the ESPRIT decays of the upper partials, docs/sustain.md).
        bridge_stiffness: 470.0,
        bridge_line_mass: 0.6,
        bridge_spread: 0.3,
        bridge_beam: 0.0,
        // ×2.4 at C8, ×0.4 at A0 relative to C4. The treble side was set on
        // Salamander's decay envelopes alone (0.035, ×4.6 at C8,
        // docs/sustain.md); against eight real grands 0.02 lets the top
        // strings lose their prompt sound faster into the bridge (bench
        // 1.53 → 1.51 with the top losses lowered, docs/top_treble.md).
        // Re-set with the outline, which gathers the low modes on the bass
        // side (the rectangle had 0.02 and 0): bench 1.11 against 1.15 with
        // the rectangle's slopes, docs/bass_metal.md.
        impedance_slope: 0.0024,
        impedance_slope_bass: 0.0076,
        f_max: 20_000.0,
        // A broad mode near 40 Hz (docs/bass.md). Mass set on the late
        // sustain of the bass (docs/long_sustain.md): with 1.5 kg it took
        // 37 dB from the 63 Hz band between 0.3 and 3 s (references 6–21).
        // With the edge loss, 60 Hz and 3 kg made the bass fundamentals
        // decay too fast (docs/bass_decay.md).
        piston_freq: 40.0,
        piston_loss: 0.7,
        piston_mass: 6.0,
        // The case moving as a block barely radiates (the board's two faces
        // and the open bottom short-circuit it): 0.3 put the bass
        // fundamentals 25–30 dB over the references below 40 Hz, and 0.03
        // still left the fundamentals of C1–F#1, which drive this mode
        // near its resonance, 7 dB over eight real grands at the attack
        // (relative to the strongest partial; 3 dB with 0.003,
        // docs/bass_decay.md).
        piston_radiation: 0.003,
        // Set on the third-octave balance of the bass keys (commercial piano, Iowa,
        // Salamander, docs/bass_radiation.md): first mode 61 → 109 Hz.
        crown_freq: 90.0,
        // Main bridge at equal steps of the main range (C8, ≈ G#6, ≈ E5,
        // ≈ C#4, A2): x follows the speaking length of the 2 m scale from
        // agraffes 3 cm in front of the board's front edge (C8 kept 6 cm in),
        // y the key's place across the 1.2 m wide string band. The treble
        // end sits in the board's clamped corner, as on real grands.
        treble_bridge: [(0.035, 0.077), (0.051, 0.245), (0.136, 0.413), (0.327, 0.581), (0.666, 0.749)],
        // The bass strings cross over the tenor towards the middle of the
        // board but keep their keys' order, as on real grands: A0, the
        // longest, ends furthest back and nearest the spine. The bridge
        // lies where the IRCAM Steinway D's does, in proportion to the
        // board (Chabassier 2012): A0 at 87 % of the length and 25 % of the
        // width from the spine, G#2 at 55 % and 44 %, bowed towards the tail
        // as the D's (its 4 cm on a 0.69 m chord). From agraffes 3 cm in
        // front of the board it would want A0 9 cm longer and G#2 17 cm
        // shorter than the scale: the board and the agraffes are not yet a
        // 2 m grand's (docs/bridges.md).
        bass_bridge: [(0.553, 0.562), (0.867, 0.753)],
        // 24 keys on the bass bridge, as on grands of 2–2.25 m: the long
        // bridge starts at G#2 on a 225 cm grand, near A#2 on a 200 cm one,
        // at F2 on 227 and 274 cm concert grands (docs/bridges.md).
        main_bridge_from: 45,
        bass_bridge_bow: 0.034,
        // The tenor's strings at 8° towards the spine, fanning to upright at
        // C8 (A2's bridge point 16 cm towards the bass side, 16 cm from the
        // spine; the bass strings cross over them at 12–15° the other way).
        // No measured angle was found; the bench scores 1.04 with and
        // without it, and 10° and 12° worse (docs/bridges.md).
        main_string_slant: 8.0,
        mics: [(0.45, 0.35, 1.5), (1.15, 0.85, 1.5)],
        // 1.5 m in front of an upright's board, the left one on the bass side.
        mics_upright: [(0.7, 1.0, 1.5), (0.3, 0.45, 1.5)],
        mics_relative: false,
        mic_gain_db: 0.0,
        mic_pattern: 1.0,
        radiate: true,
        grid_step: 0.015,
        // Calibrated on the spectral tilt of Salamander (low bands).
        back_depth: 0.6,
        back_gain: 0.6,
        wall_distance: 0.0,
        wall_reflection: 0.9,
        vertical: false,
        case_open: 1.0,
        case_panel_mass: 6.0,
        // Removed lid: the default mics hang above the board (calibrated
        // set-up); see presets/listening/Microphones/concert.toml for an open lid.
        lid_angle: 0.0,
        lid_removed: true,
        // On the rim's top (`RimParams::top`), as on a real grand.
        lid_hinge_height: 0.16,
        lid_reflection: 0.85,
        lid_transmission: 0.1,
        // −12 dB at high frequencies, no measurement behind it (docs/lid.md).
        lid_leak: 0.25,
        string_load: 1.0,
        string_load_free: 1.0,
        exact_below: 0.0,
        // The grand's outline (docs/outline.md), on since 2026-10-28: with
        // the rectangle, the modes at 260–335 Hz reached the two close
        // microphones in antiphase and a bass attack arrived 3.7 ms apart
        // with no correlation between the channels (real pairs: 0.2–1.7 ms,
        // +0.2 to +0.5), a flam the user heard as a "tube" in the bass; the
        // outline makes the attack coherent in every band (docs/bass_metal.md).
        outline: Self::GRAND_OUTLINE,
        outline_below: 1500.0,
        rib_spacing: 0.0,
        rib_share: 0.0,
        rib_jitter: 0.0,
        scatter: 0.0,
        scatter_corner: 1500.0,
        scatter_step: 0.1,
        scatter_height: 0.05,
        scatter_shadow: 0.0,
    };

    /// A grand's outline for the 2 m board: straight front edge (x = 0) and
    /// spine (the bass side, y = b); the bentside leaves the treble cheek
    /// 20 cm behind the front edge and curves in towards the tail, which
    /// keeps the bass side's 60 %: 82 % of the rectangle. The bass bridge's
    /// ends lie 42 cm (G#2) and 23 cm (A0, from the tail) inside it.
    pub const GRAND_OUTLINE: [(f64, f64); OUTLINE_POINTS] = [
        (0.0, 0.0),
        (0.12, 0.0),
        (0.30, 0.10),
        (0.50, 0.21),
        (0.70, 0.28),
        (0.85, 0.30),
        (0.95, 0.33),
        (1.0, 0.40),
        (1.0, 1.0),
        (0.0, 1.0),
    ];

    fn h(&self) -> f64 {
        (self.dx * self.dy).sqrt()
    }

    /// The lid folded down on the case, in metres: the board's outline (or
    /// the rectangle) grown by [`LID_OVERHANG`] over the rim, its bentside
    /// following the board's. Open, it turns about the hinge y = b; the
    /// part past the hinge (y > b) is the rim's top under it.
    pub fn lid_outline(&self) -> Outline {
        let board = Outline::new(self).unwrap_or_else(|| Outline { points: vec![(0.0, 0.0), (self.a, 0.0), (self.a, self.b), (0.0, self.b)] });
        board.grown(LID_OVERHANG)
    }

    /// How shut the lid is: 1 at an angle of 0, 0 from [`LID_CLOSING`]
    /// degrees open, and with no lid or on an upright (`case_open`).
    pub fn lid_closure(&self) -> f64 {
        if self.vertical || self.lid_removed {
            return 0.0;
        }
        (1.0 - self.lid_angle / LID_CLOSING).clamp(0.0, 1.0)
    }

    /// Gain of the front face's sound through the case at ω, and through
    /// the shut lid ([`Self::lid_closure`]).
    pub fn case_gain(&self, omega: f64) -> f64 {
        let tau = 1.0 / (1.0 + (omega * self.case_panel_mass / (2.0 * RHO0 * C0)).powi(2)).sqrt();
        let case = self.case_open + (1.0 - self.case_open) * tau;
        let s = self.lid_closure();
        if self.vertical || s == 0.0 {
            return case;
        }
        let r = omega * 2.0 * self.lid_hinge_height.max(0.01) / (TAU * C0);
        let lid = self.lid_leak + (1.0 - self.lid_leak) / (1.0 + r * r).sqrt();
        case * (1.0 - s + s * lid)
    }

    /// Radiation paths from the board cell `c` to the point `m`.
    fn path(&self, lid: Option<&Lid>, c: [f64; 3], m: [f64; 3]) -> Path {
        let d2 = (c[0] - m[0]).powi(2) + (c[1] - m[1]).powi(2);
        let at = |z: f64| (d2 + z * z).sqrt();
        let z = m[2];
        if z < 0.0 {
            // Below (behind) the board: the back face directly, the front
            // one around the case.
            return Path { front: (at(-z + self.back_depth), self.back_gain), back: (at(-z), 1.0), wall: (1.0, 0.0), image: (1.0, 0.0) };
        }
        let (direct, image) = match lid {
            Some(l) => (l.direct_gain(c, m), l.image(c, m)),
            None => (1.0, None),
        };
        let wall = if self.wall_distance > 0.0 {
            (at(z + self.back_depth + 2.0 * self.wall_distance), self.wall_reflection * self.back_gain)
        } else {
            (1.0, 0.0)
        };
        Path { front: (at(z), direct), back: (at(z + self.back_depth), self.back_gain), wall, image: image.unwrap_or((1.0, 0.0)) }
    }

    /// Mean (infinite-plate) mobility 1/(8√(D·ρh)) with D = H.
    pub fn mean_mobility(&self) -> f64 {
        1.0 / (8.0 * (self.h() * self.rho_h).sqrt())
    }

    /// Bridge point of a key, in metres.
    pub fn bridge_point(&self, note: u8) -> (f64, f64) {
        self.bridge_point_at(note as f64, note >= self.main_bridge_from)
    }

    /// Point at a fractional key position along the main or the bass bridge.
    pub fn bridge_point_at(&self, note: f64, main: bool) -> (f64, f64) {
        let (x, y) = self.upright_point_at(note, main);
        let a = if main { self.string_slant(note) } else { 0.0 };
        if a == 0.0 {
            return (x, y);
        }
        let (s, c) = a.sin_cos();
        (KEY_LINE_X + (x - KEY_LINE_X) * c, y + (x - KEY_LINE_X) * s)
    }

    /// A main bridge string's slant (rad, towards the bass side going
    /// back): [`Self::main_string_slant`] at the main bridge's lowest key,
    /// 0 at C8, but no more than keeps its bridge point [`SLANT_EDGE`]
    /// inside the board's bass side (the spine for a grand) and its front
    /// edge, whatever the angle asked. The strings lean only towards the
    /// bass: a negative angle reads as 0.
    pub fn string_slant(&self, note: f64) -> f64 {
        let t = ((108.0 - note) / (108.0 - self.main_bridge_from as f64).max(1.0)).clamp(0.0, 1.0);
        let a = self.main_string_slant.to_radians() * t;
        let (x, y) = self.upright_point_at(note, true);
        let d = (x - KEY_LINE_X).max(1e-9);
        let side = ((self.b - SLANT_EDGE - y).max(0.0) / d).min(1.0).asin();
        let front = ((SLANT_EDGE - KEY_LINE_X) / d).clamp(-1.0, 1.0).acos();
        a.clamp(0.0, side.min(front))
    }

    /// Where a key's strings cross the keys' line ([`KEY_LINE_X`]), across
    /// the board (m): the main bridge's strings come from there whatever
    /// their slant. The bass strings are taken as running along x from
    /// their bridge point, as the frame's agraffes always have.
    pub fn key_across(&self, note: u8) -> f64 {
        let main = note >= self.main_bridge_from;
        if main { self.upright_point_at(note as f64, true).1 } else { self.bridge_point(note).1 }
    }

    /// The bridge point before the main strings' slant: the curve through
    /// `treble_bridge`, or the bass bridge.
    fn upright_point_at(&self, note: f64, main: bool) -> (f64, f64) {
        let lerp = |p: (f64, f64), q: (f64, f64), t: f64| (p.0 + (q.0 - p.0) * t, p.1 + (q.1 - p.1) * t);
        let (u, v) = if main {
            // 0 at C8, 1 at the lowest note of the main bridge.
            let t = ((108.0 - note) / (108.0 - self.main_bridge_from as f64)).clamp(0.0, 1.0);
            // A real bridge is one bent beam: a Catmull-Rom curve through
            // the five points, so that the keys' spacing along it changes
            // smoothly instead of jumping at the points. The ends' outer
            // neighbours are mirrored.
            let tb = &self.treble_bridge;
            let seg = ((t * 4.0) as usize).min(3);
            let u = t * 4.0 - seg as f64;
            let mirror = |p: (f64, f64), q: (f64, f64)| (2.0 * p.0 - q.0, 2.0 * p.1 - q.1);
            let p0 = if seg > 0 { tb[seg - 1] } else { mirror(tb[0], tb[1]) };
            let p3 = if seg < 3 { tb[seg + 2] } else { mirror(tb[4], tb[3]) };
            let (p1, p2) = (tb[seg], tb[seg + 1]);
            let cr = |a: f64, b: f64, c: f64, d: f64| {
                0.5 * (2.0 * b + (c - a) * u + (2.0 * a - 5.0 * b + 4.0 * c - d) * u * u + (3.0 * (b - c) + d - a) * u * u * u)
            };
            (cr(p0.0, p1.0, p2.0, p3.0), cr(p0.1, p1.1, p2.1, p3.1))
        } else {
            // 0 at the highest bass key, 1 at A0.
            let span = (self.main_bridge_from as f64 - 22.0).max(1.0);
            let t = ((self.main_bridge_from as f64 - 1.0 - note) / span).clamp(0.0, 1.0);
            if self.bass_bridge_bow == 0.0 {
                lerp(self.bass_bridge[0], self.bass_bridge[1], t)
            } else {
                let (x, y) = self.bass_arc(t);
                (x / self.a, y / self.b)
            }
        };
        (u * self.a, v * self.b)
    }

    /// Point at `t` (0 at the highest bass key, 1 at A0) along the bowed
    /// bass bridge, in metres: equal steps of `t` are equal arc lengths.
    fn bass_arc(&self, t: f64) -> (f64, f64) {
        let [(u0, v0), (u1, v1)] = self.bass_bridge;
        let (p, q) = ((u0 * self.a, v0 * self.b), (u1 * self.a, v1 * self.b));
        let c = (q.0 - p.0).hypot(q.1 - p.1).max(1e-9);
        let d = ((q.0 - p.0) / c, (q.1 - p.1) / c);
        // The chord's normal towards the tail (+x), and the bow along it.
        let n = if d.1 >= 0.0 { (d.1, -d.0) } else { (-d.1, d.0) };
        let s = self.bass_bridge_bow.clamp(-c / 2.0, c / 2.0);
        let n = if s < 0.0 { (-n.0, -n.1) } else { n };
        let s = s.abs();
        // Radius and half angle of the arc; its centre behind the chord.
        let r = (c * c / 4.0 + s * s) / (2.0 * s);
        let half = (c / 2.0).atan2(r - s);
        let mid = ((p.0 + q.0) / 2.0, (p.1 + q.1) / 2.0);
        let centre = (mid.0 - n.0 * (r - s), mid.1 - n.1 * (r - s));
        let phi = -half + 2.0 * half * t;
        (centre.0 + r * (phi.sin() * d.0 + phi.cos() * n.0), centre.1 + r * (phi.sin() * d.1 + phi.cos() * n.1))
    }

    /// The main or the bass bridge as a line to draw, in metres.
    pub fn bridge_line(&self, main: bool) -> Vec<(f64, f64)> {
        if main {
            let lo = self.main_bridge_from as f64;
            (0..=64).map(|i| self.bridge_point_at(108.0 - (108.0 - lo) * i as f64 / 64.0, true)).collect()
        } else {
            (0..=32).map(|i| self.bridge_point_at(self.main_bridge_from as f64 - 1.0 - (self.main_bridge_from as f64 - 22.0) * i as f64 / 32.0, false)).collect()
        }
    }
}

/// The keys' line (m, plate x): where the bridges' layout puts the strings'
/// front ends, 3 cm in front of the board's front edge; the main strings
/// turn about it when they slant ([`SoundboardParams::main_string_slant`]).
pub const KEY_LINE_X: f64 = -0.03;

/// The slanting main strings' bridge points stay this far (m) inside the
/// board's sides ([`SoundboardParams::string_slant`]).
pub const SLANT_EDGE: f64 = 0.05;

/// The radiation paths from a board cell to a point: (length, gain).
#[derive(Clone, Copy)]
struct Path {
    /// Front face: direct (shadowed by the lid or the case), or around the
    /// case for a point below the board.
    front: (f64, f64),
    /// Back face: around the case, or direct for a point below.
    back: (f64, f64),
    /// Back face reflected by the wall (gain 0: no wall).
    wall: (f64, f64),
    /// Front face reflected by the lid (gain 0 when the reflection misses).
    image: (f64, f64),
}

/// The open lid, a reflecting plate of the case's shape.
struct Lid {
    p0: [f64; 3],
    up: [f64; 3],
    n: [f64; 3],
    /// [`SoundboardParams::lid_outline`] and the hinge's y.
    outline: Outline,
    hinge: f64,
    reflection: f64,
    transmission: f64,
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn lerp3(a: [f64; 3], b: [f64; 3], u: f64) -> [f64; 3] {
    [a[0] + u * (b[0] - a[0]), a[1] + u * (b[1] - a[1]), a[2] + u * (b[2] - a[2])]
}

impl Lid {
    /// As the lid shuts, its image and its shadow fade into the shut
    /// lid's gain on every path ([`SoundboardParams::case_gain`]): none
    /// left at 1, so that closing it is continuous.
    fn new(p: &SoundboardParams) -> Option<Self> {
        let s = p.lid_closure();
        (!p.lid_removed && p.lid_angle > 0.0 && s < 1.0).then(|| {
            let th = p.lid_angle.to_radians();
            Self {
                p0: [0.0, p.b, p.lid_hinge_height],
                up: [0.0, -th.cos(), th.sin()],
                // Normal pointing to the board side.
                n: [0.0, -th.sin(), -th.cos()],
                outline: p.lid_outline(),
                hinge: p.b,
                reflection: p.lid_reflection * (1.0 - s),
                transmission: p.lid_transmission + s * (1.0 - p.lid_transmission),
            }
        })
    }

    fn side(&self, q: [f64; 3]) -> f64 {
        dot(sub(q, self.p0), self.n)
    }

    /// Whether the point of the lid's plane lies on the lid: folded down
    /// about the hinge, (x, hinge − t) inside its outline.
    fn on_lid(&self, x: [f64; 3]) -> bool {
        let t = dot(sub(x, self.p0), self.up);
        t >= 0.0 && self.outline.contains(x[0], self.hinge - t)
    }

    /// Direct path gain: shadowed when the mic is behind the lid.
    fn direct_gain(&self, c: [f64; 3], m: [f64; 3]) -> f64 {
        let (fc, fm) = (self.side(c), self.side(m));
        if fm >= 0.0 || fc <= 0.0 {
            return 1.0;
        }
        if self.on_lid(lerp3(c, m, fc / (fc - fm))) { self.transmission } else { 1.0 }
    }

    /// Mirror image of the cell in the lid, if the reflection hits the lid
    /// and the mic sees it: (path length, gain).
    fn image(&self, c: [f64; 3], m: [f64; 3]) -> Option<(f64, f64)> {
        let (fc, fm) = (self.side(c), self.side(m));
        if fc <= 0.0 || fm <= 0.0 {
            return None;
        }
        let img = [c[0] - 2.0 * fc * self.n[0], c[1] - 2.0 * fc * self.n[1], c[2] - 2.0 * fc * self.n[2]];
        self.on_lid(lerp3(img, m, fc / (fc + fm))).then(|| {
            let d = sub(m, img);
            (dot(d, d).sqrt(), self.reflection)
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PlateMode {
    /// The rectangle's mode (m, n); for an outlined board's own mode, its
    /// largest component.
    pub m: u32,
    pub n: u32,
    pub omega: f64,
    /// Damping rate (1/s) the damped strings add ([`Soundboard::load_strings`]).
    pub load: f64,
    /// An outlined board's own mode: its index in [`Soundboard::ritz`].
    pub ritz: Option<u32>,
}

pub struct Soundboard {
    pub params: SoundboardParams,
    pub modes: Vec<PlateMode>,
    /// The outline, when the board is not the full rectangle.
    pub outline: Option<Outline>,
    /// The outlined board's own modes, on the rectangle's modes `basis`:
    /// Φ = Σ_k ritz[i][k]·Φ_basis[k] (both mass-normalised); basis modes
    /// as (m, n, ω).
    pub basis: Vec<(u32, u32, f64)>,
    pub ritz: Vec<Vec<f64>>,
    /// The stiff ground that holds the board outside the outline: its
    /// stiffness (N/m³) and the cells it acts on, (x, y, area outside the
    /// outline), with the basis shapes there (`cells[c]`, `held[c·basis + k]`).
    pub ground: f64,
    pub cells: Vec<(f64, f64, f64)>,
    held: Vec<f64>,
}

/// The board's outline in metres (a closed polygon).
#[derive(Clone, Debug)]
pub struct Outline {
    pub points: Vec<(f64, f64)>,
}

impl Outline {
    /// None when the polygon has no area (the full rectangle).
    pub fn new(p: &SoundboardParams) -> Option<Self> {
        let mut points: Vec<(f64, f64)> = Vec::new();
        for &(u, v) in &p.outline {
            let q = (u.clamp(0.0, 1.0) * p.a, v.clamp(0.0, 1.0) * p.b);
            if points.last() != Some(&q) {
                points.push(q);
            }
        }
        while points.len() > 1 && points.first() == points.last() {
            points.pop();
        }
        let o = Self { points };
        (o.area() > 1e-6 * p.a * p.b).then_some(o)
    }

    fn edges(&self) -> impl Iterator<Item = ((f64, f64), (f64, f64))> + '_ {
        self.points.iter().zip(self.points.iter().cycle().skip(1)).map(|(&p, &q)| (p, q))
    }

    pub fn area(&self) -> f64 {
        0.5 * self.edges().map(|(p, q)| p.0 * q.1 - q.0 * p.1).sum::<f64>().abs()
    }

    pub fn contains(&self, x: f64, y: f64) -> bool {
        self.edges().filter(|&(p, q)| (p.1 > y) != (q.1 > y) && x < p.0 + (y - p.1) / (q.1 - p.1) * (q.0 - p.0)).count() % 2 == 1
    }

    /// Distance (m) to the nearest edge.
    pub fn edge_distance(&self, x: f64, y: f64) -> f64 {
        self.edges()
            .map(|(p, q)| {
                let (dx, dy) = (q.0 - p.0, q.1 - p.1);
                let t = (((x - p.0) * dx + (y - p.1) * dy) / (dx * dx + dy * dy).max(1e-30)).clamp(0.0, 1.0);
                (x - p.0 - t * dx).hypot(y - p.1 - t * dy)
            })
            .fold(f64::INFINITY, f64::min)
    }

    /// The outline moved `d` m outwards, edge by edge, mitred at the
    /// vertices (each vertex keeps its index).
    pub fn grown(&self, d: f64) -> Self {
        let n = self.points.len();
        let signed = 0.5 * self.edges().map(|(p, q)| p.0 * q.1 - q.0 * p.1).sum::<f64>();
        let side = if signed < 0.0 { -1.0 } else { 1.0 };
        // Outwards: right of each edge on a counter-clockwise polygon.
        let normal = |k: usize| {
            let (p, q) = (self.points[k], self.points[(k + 1) % n]);
            let (dx, dy) = (q.0 - p.0, q.1 - p.1);
            let l = dx.hypot(dy).max(1e-12);
            (side * dy / l, -side * dx / l)
        };
        let points = (0..n)
            .map(|i| {
                let (u, v) = (normal((i + n - 1) % n), normal(i));
                let (mx, my) = (u.0 + v.0, u.1 + v.1);
                let l = mx.hypot(my).max(1e-12);
                let k = d / ((mx * v.0 + my * v.1) / l).max(0.3);
                let p = self.points[i];
                (p.0 + mx / l * k, p.1 + my / l * k)
            })
            .collect();
        Self { points }
    }

    /// Share of the cell centred on (x, y), dx × dy, inside the outline.
    pub fn coverage(&self, x: f64, y: f64, dx: f64, dy: f64) -> f64 {
        const S: usize = 4;
        let off = |i: usize| (i as f64 + 0.5) / S as f64 - 0.5;
        let inside = (0..S * S).filter(|&i| self.contains(x + off(i % S) * dx, y + off(i / S) * dy)).count();
        inside as f64 / (S * S) as f64
    }
}

/// The rectangle's modes below w_max (rad/s), unsorted: (m, n, ω).
fn rectangle_modes(p: &SoundboardParams, w_max: f64) -> Vec<(u32, u32, f64)> {
    let k = PI * PI / p.rho_h.sqrt();
    let mut modes = Vec::new();
    let mut m = 1;
    loop {
        let xm = ((m as f64 + 0.5) / p.a).powi(2);
        if k * (p.dx * xm * xm).sqrt() > w_max {
            break;
        }
        let mut n = 1;
        loop {
            let yn = ((n as f64 + 0.5) / p.b).powi(2);
            let w = (k * k * (p.dx * xm * xm + 2.0 * p.h() * xm * yn + p.dy * yn * yn) + (TAU * p.crown_freq).powi(2)).sqrt();
            if w > w_max {
                break;
            }
            modes.push((m, n, w));
            n += 1;
        }
        m += 1;
    }
    modes
}

/// sin(k·θ) for k = 0..n (Chebyshev recurrence).
fn sines(theta: f64, n: usize, out: &mut Vec<f64>) {
    out.clear();
    out.push(0.0);
    if n == 0 {
        return;
    }
    let (s1, c2) = (theta.sin(), 2.0 * theta.cos());
    out.push(s1);
    for k in 2..=n {
        out.push(c2 * out[k - 1] - out[k - 2]);
    }
}

/// The outlined board's modes are sought among the rectangle's below
/// `BASIS` × the outline's corner frequency. A shorter rectangle taken as
/// an outline gets its own analytic modes within 1 % rms; on a grand's
/// outline the lowest twelve move by < 1 % from 4× to 8× (examples/outline_modes.rs).
const BASIS: f64 = 4.0;
/// The ground holding the board outside the outline, as the frequency
/// (relative to the basis's highest) it would give the board on its own:
/// softer, the modes leak out of the outline (−3 % at 0.5×); much stiffer,
/// the truncated basis locks (+0.5 % at 4×).
const GROUND: f64 = 2.0;

impl SoundboardParams {
    /// The same set-up with the microphones in use (`mics_upright` on a
    /// vertical board) in metres.
    /// Idempotent.
    pub fn resolved(self) -> Self {
        let metres = |m: [(f64, f64, f64); 2]| if self.mics_relative { m.map(|(x, y, z)| (x * self.a, y * self.b, z)) } else { m };
        let (grand, upright) = (metres(self.mics), metres(self.mics_upright));
        Self { mics: if self.vertical { upright } else { grand }, mics_upright: upright, mics_relative: false, ..self }
    }
}

impl Soundboard {
    /// All modes below `f_max` (and below `nyquist`), sorted by frequency.
    ///
    /// With an outline, the modes below `outline_below` are the outlined
    /// board's own, by Rayleigh–Ritz on the rectangle's modes below
    /// [`BASIS`] times that frequency: a stiff elastic ground holds the
    /// board outside the outline (the rim, clamping it there), adding
    /// ω_g²·∫_outside Φ_i·Φ_j·ρh to the rectangle's diagonal ω_i². The
    /// lowest eigenvectors are the board inside the outline. Above, the
    /// rectangle's modes remain, radiating from inside the outline only.
    pub fn new(params: SoundboardParams) -> Self {
        let p = params;
        let w_max = TAU * p.f_max;
        let mut modes: Vec<PlateMode> =
            rectangle_modes(&p, w_max).into_iter().map(|(m, n, omega)| PlateMode { m, n, omega, load: 0.0, ritz: None }).collect();
        let mut sb = Self { params, modes: Vec::new(), outline: Outline::new(&p), basis: Vec::new(), ritz: Vec::new(), ground: 0.0, cells: Vec::new(), held: Vec::new() };
        if sb.outline.is_some() && p.outline_below > 0.0 {
            let wc = TAU * p.outline_below;
            let own = sb.outlined_modes(wc);
            modes.retain(|m| m.omega >= wc);
            modes.extend(own.into_iter().filter(|m| m.omega <= w_max));
        }
        if p.piston_freq > 0.0 && p.piston_mass > 0.0 {
            modes.push(PlateMode { m: 0, n: 0, omega: TAU * p.piston_freq, load: 0.0, ritz: None });
        }
        modes.sort_by(|x, y| x.omega.total_cmp(&y.omega));
        sb.modes = modes;
        sb
    }

    /// The outlined board's modes below `wc` (rad/s); fills the basis, the
    /// ground and the Ritz vectors.
    fn outlined_modes(&mut self, wc: f64) -> Vec<PlateMode> {
        let p = self.params;
        let outline = self.outline.as_ref().expect("an outline");
        let mut basis = rectangle_modes(&p, BASIS * wc);
        basis.sort_by(|x, y| x.2.total_cmp(&y.2));
        let nb = basis.len();
        let w_top = basis.last().map_or(wc, |b| b.2);
        self.ground = p.rho_h * (GROUND * w_top).powi(2);
        // Cells outside the outline, on a grid of ≥ 8 points per wavelength
        // of the basis.
        let (mm, nm) = basis.iter().fold((1, 1), |(a, b), &(m, n, _)| (a.max(m), b.max(n)));
        let step = (p.a / mm as f64).min(p.b / nm as f64) / 4.0;
        let (nx, ny) = ((p.a / step).ceil() as usize, (p.b / step).ceil() as usize);
        let (dx, dy) = (p.a / nx as f64, p.b / ny as f64);
        self.cells = (0..nx * ny)
            .filter_map(|c| {
                let (x, y) = ((c / ny) as f64 * dx + 0.5 * dx, (c % ny) as f64 * dy + 0.5 * dy);
                let out = 1.0 - outline.coverage(x, y, dx, dy);
                (out > 0.0).then_some((x, y, out * dx * dy))
            })
            .collect();
        self.basis = basis.clone();
        let norm = 2.0 / (p.rho_h * p.a * p.b).sqrt();
        let held: Vec<Vec<f64>> = crate::par::map(self.cells.len(), |c| {
            let (x, y, _) = self.cells[c];
            let (mut sx, mut sy) = (Vec::new(), Vec::new());
            sines(PI * x / p.a, mm as usize, &mut sx);
            sines(PI * y / p.b, nm as usize, &mut sy);
            self.basis.iter().map(|&(m, n, _)| norm * sx[m as usize] * sy[n as usize]).collect()
        });
        self.held = held.into_iter().flatten().collect();
        // The ribs: a beam of stiffness EI along y at x_r adds
        // EI·∫Φ_i,yy·Φ_j,yy dy = EI·(nπ/b)⁴·(b/2)·norm²·sin(m_i·πx_r/a)·
        // sin(m_j·πx_r/a) for n_i = n_j; the same stiffness smeared over the
        // plate, EI/s on dy, is already in `dy`: it is taken out again,
        // (a/2s)·δ_ij, so that the long waves see nothing and the short ones
        // the ribs' period.
        // Irregular ribs: each moved and stiffened by its own pseudo-random
        // draws in [−½, ½] (the same at every build), the stiffnesses
        // normalised to a mean of 1.
        let draw = |r: usize, k: f64| ((r as f64 * 12.9898 + k * 78.233).sin() * 43_758.545_3).rem_euclid(1.0) - 0.5;
        let ribs: Vec<(f64, Vec<f64>)> = if p.rib_spacing > 0.0 && p.rib_share > 0.0 {
            let n_ribs = (p.a / p.rib_spacing).floor().max(1.0) as usize;
            let margin = 0.5 * (p.a - (n_ribs as f64 - 1.0) * p.rib_spacing);
            let j = p.rib_jitter.clamp(0.0, 1.0);
            let weights: Vec<f64> = (0..n_ribs).map(|r| 1.0 + 2.0 * j * draw(r, 2.0)).collect();
            let mean = weights.iter().sum::<f64>() / n_ribs as f64;
            (0..n_ribs)
                .map(|r| {
                    let x = (margin + (r as f64 + j * draw(r, 1.0)) * p.rib_spacing).clamp(0.0, p.a);
                    let mut sx = Vec::new();
                    sines(PI * x / p.a, mm as usize, &mut sx);
                    (weights[r] / mean, sx)
                })
                .collect()
        } else {
            Vec::new()
        };
        let ei = p.rib_share * p.dy * p.rib_spacing;
        let rib_gain = 2.0 * ei / (p.rho_h * p.a);
        let smeared = p.a / (2.0 * p.rib_spacing.max(1e-9));
        // A = diag(ω²) + ω_g²·G, G_ij = ρh·Σ_cells Φ_i·Φ_j·area.
        let rows: Vec<Vec<f64>> = crate::par::map(nb, |i| {
            let mut row = vec![0.0; nb];
            for (c, &(_, _, area)) in self.cells.iter().enumerate() {
                let h = &self.held[c * nb..(c + 1) * nb];
                let wi = self.ground * area * h[i];
                for (r, hj) in row.iter_mut().zip(h) {
                    *r += wi * hj;
                }
            }
            row[i] += basis[i].2 * basis[i].2;
            if !ribs.is_empty() {
                let (mi, ni, _) = basis[i];
                let k4 = (ni as f64 * PI / p.b).powi(4);
                for (j, &(mj, nj, _)) in basis.iter().enumerate() {
                    if nj != ni {
                        continue;
                    }
                    let s: f64 = ribs.iter().map(|(w, sx)| w * sx[mi as usize] * sx[mj as usize]).sum();
                    row[j] += rib_gain * k4 * (s - if mi == mj { smeared } else { 0.0 });
                }
            }
            row
        });
        let mut a: Vec<f64> = rows.into_iter().flatten().collect();
        let (vals, vecs) = if p.bridge_beam > 0.0 {
            let mut m = vec![0.0; nb * nb];
            for i in 0..nb {
                m[i * nb + i] = 1.0;
            }
            self.add_bridge_beams(&mut a, &mut m, mm as usize, nm as usize);
            crate::eigen::generalized(&a, &m, nb)
        } else {
            crate::eigen::symmetric_ql(&a, nb)
        };
        let mut modes = Vec::new();
        for (lam, v) in vals.into_iter().zip(vecs) {
            let omega = lam.max(0.0).sqrt();
            if omega >= wc {
                break;
            }
            let k = (0..nb).max_by(|&i, &j| v[i].abs().total_cmp(&v[j].abs())).unwrap_or(0);
            let (m, n, _) = self.basis[k];
            modes.push(PlateMode { m, n, omega, load: 0.0, ritz: Some(self.ritz.len() as u32) });
            self.ritz.push(v);
        }
        modes
    }

    /// Adds the bridges as beams to the basis's stiffness `k` and mass `m`
    /// (row-major, the basis's size): EI·∫Φ_i,ss·Φ_j,ss ds and μ·∫Φ_i·Φ_j ds
    /// along each bridge's curve, s its arc length, Φ_ss its second
    /// derivative along the tangent (the curvature's term left out), by the
    /// midpoint rule on ≥ 8 points per wavelength of the basis.
    fn add_bridge_beams(&self, k: &mut [f64], m: &mut [f64], mm: usize, nm: usize) {
        let p = &self.params;
        let nb = self.basis.len();
        let (ei, mu) = (p.bridge_beam * p.bridge_stiffness, p.bridge_beam * p.bridge_line_mass);
        let norm = 2.0 / (p.rho_h * p.a * p.b).sqrt();
        let step = (p.a / mm.max(1) as f64).min(p.b / nm.max(1) as f64) / 8.0;
        let main = p.main_bridge_from as f64;
        let mut segments: Vec<((f64, f64), (f64, f64))> = Vec::new();
        for (lo, hi, on_main) in [(main, 108.0, true), (21.0, main - 1.0, false)] {
            if hi <= lo {
                continue;
            }
            let (a0, a1) = (p.bridge_point_at(lo, on_main), p.bridge_point_at(hi, on_main));
            // Fine enough whatever the curve: refine until each piece is
            // shorter than the step.
            let n = 2000.max(((a1.0 - a0.0).hypot(a1.1 - a0.1) / step * 4.0) as usize);
            let pts: Vec<(f64, f64)> = (0..=n).map(|i| p.bridge_point_at(lo + (hi - lo) * i as f64 / n as f64, on_main)).collect();
            for w in pts.windows(2) {
                segments.push((w[0], w[1]));
            }
        }
        let (mut g, mut h) = (vec![0.0; nb], vec![0.0; nb]);
        let (mut sx, mut sy) = (Vec::new(), Vec::new());
        for &((x0, y0), (x1, y1)) in &segments {
            let ds = (x1 - x0).hypot(y1 - y0);
            if ds <= 0.0 {
                continue;
            }
            let (tx, ty) = ((x1 - x0) / ds, (y1 - y0) / ds);
            let (x, y) = (0.5 * (x0 + x1), 0.5 * (y0 + y1));
            sines(PI * x / p.a, mm, &mut sx);
            sines(PI * y / p.b, nm, &mut sy);
            for (i, &(mi, ni, _)) in self.basis.iter().enumerate() {
                let (kx, ky) = (mi as f64 * PI / p.a, ni as f64 * PI / p.b);
                let phi = norm * sx[mi as usize] * sy[ni as usize];
                let phi_xy = norm * kx * ky * (kx * x).cos() * (ky * y).cos();
                h[i] = phi;
                g[i] = -(tx * tx * kx * kx + ty * ty * ky * ky) * phi + 2.0 * tx * ty * phi_xy;
            }
            for i in 0..nb {
                let (gi, hi) = (ei * ds * g[i], mu * ds * h[i]);
                for j in 0..nb {
                    k[i * nb + j] += gi * g[j];
                    m[i * nb + j] += hi * h[j];
                }
            }
        }
    }

    /// Mass-normalised mode shape at (x, y) metres (uniform for the piston
    /// mode, m = 0).
    pub fn shape(&self, mode: &PlateMode, x: f64, y: f64) -> f64 {
        let p = &self.params;
        if mode.m == 0 {
            return 1.0 / p.piston_mass.sqrt();
        }
        if let Some(r) = mode.ritz {
            return self.ritz_shape(&self.ritz[r as usize], x, y);
        }
        2.0 / (p.rho_h * p.a * p.b).sqrt() * (mode.m as f64 * PI * x / p.a).sin() * (mode.n as f64 * PI * y / p.b).sin()
    }

    fn ritz_shape(&self, coefs: &[f64], x: f64, y: f64) -> f64 {
        let p = &self.params;
        let (mm, nm) = self.basis.iter().fold((1, 1), |(a, b), &(m, n, _)| (a.max(m), b.max(n)));
        let (mut sx, mut sy) = (Vec::new(), Vec::new());
        sines(PI * x / p.a, mm as usize, &mut sx);
        sines(PI * y / p.b, nm as usize, &mut sy);
        let sum: f64 = self.basis.iter().zip(coefs).map(|(&(m, n, _), c)| c * sx[m as usize] * sy[n as usize]).sum();
        2.0 / (p.rho_h * p.a * p.b).sqrt() * sum
    }

    /// The forces (N per unit modal displacement) with which an outlined
    /// board's own mode pushes on the ground that holds it outside the
    /// outline, at (x, y): its reaction on the rim where the outline leaves
    /// the rectangle (where they share an edge, its basis modes' own edge
    /// reactions, [`Self::ritz`]). None for the others.
    pub fn held_forces(&self, mode: &PlateMode) -> Option<Vec<(f64, f64, f64)>> {
        let coefs = &self.ritz[mode.ritz? as usize];
        let nb = self.basis.len();
        Some(
            self.cells
                .iter()
                .enumerate()
                .map(|(c, &(x, y, area))| {
                    let phi: f64 = self.held[c * nb..(c + 1) * nb].iter().zip(coefs).map(|(h, k)| h * k).sum();
                    (x, y, self.ground * area * phi)
                })
                .collect(),
        )
    }

    /// Loss factor of a mode.
    pub fn mode_eta(&self, mode: &PlateMode) -> f64 {
        let own = if mode.m == 0 { self.params.piston_loss } else { self.eta(mode.omega) };
        own + 2.0 * mode.load / mode.omega
    }

    /// Adds the load of the damped strings to every mode: a dashpot c at
    /// the bridge adds c·Φ_n²/2 to mode n's damping rate (Φ mass-normalised,
    /// spread along the bridge as the key drives it). `strings`: (key, sum
    /// of √(T·μ) over its strings, kg/s), for the keys whose dampers rest
    /// on them.
    pub fn load_strings(&mut self, strings: &[(u8, f64)]) {
        let scale = self.params.string_load;
        if scale <= 0.0 {
            return;
        }
        // Summed in key order whatever the thread count.
        let shapes = crate::par::map(strings.len(), |i| self.bridge_shapes(strings[i].0));
        let mut load = vec![0.0; self.modes.len()];
        for (&(_, z), sh) in strings.iter().zip(&shapes) {
            for (l, phi) in load.iter_mut().zip(sh) {
                *l += 0.5 * scale * z * phi * phi;
            }
        }
        for (md, l) in self.modes.iter_mut().zip(load) {
            md.load = l;
        }
    }

    pub fn eta(&self, omega: f64) -> f64 {
        let f = omega / TAU;
        let low = if self.params.loss_low_corner > 0.0 { 1.0 + (self.params.loss_low_corner / f.max(1.0)).powi(2) } else { 1.0 };
        self.params.loss_factor * (1.0 + f / self.params.loss_corner) * low
    }

    pub fn sigma(&self, mode: &PlateMode) -> f64 {
        // η·ω_n is the 2σ of the velocity equation q̈ + η·ω_n·q̇ + ω_n²·q.
        0.5 * self.mode_eta(mode) * mode.omega
    }

    /// Length of bridge (m) over which a force at ω reaches the board.
    pub fn drive_length(&self, omega: f64) -> f64 {
        let p = &self.params;
        if p.bridge_stiffness <= 0.0 || omega <= 0.0 {
            return 0.0;
        }
        (p.bridge_spread * TAU * (p.bridge_stiffness / (p.bridge_line_mass * omega * omega)).powf(0.25)).min(0.5)
    }

    /// Shape of every mode as a key's string drives it through the bridge:
    /// Φ averaged (Hann window) over the drive length at the mode's
    /// frequency, along the key's bridge.
    pub fn bridge_shapes(&self, note: u8) -> Vec<f64> {
        let p = &self.params;
        let main = note >= p.main_bridge_from;
        let (lo, hi) = if main { (p.main_bridge_from as f64, 108.0) } else { (21.0, p.main_bridge_from as f64 - 1.0) };
        let n = note as f64;
        let (a, b) = (p.bridge_point_at(n - 0.5, main), p.bridge_point_at(n + 0.5, main));
        let per_key = ((a.0 - b.0).hypot(a.1 - b.1)).max(1e-3);
        const K: usize = 9;
        let slope = if n < 60.0 { p.impedance_slope_bass } else { p.impedance_slope };
        let taper = (-0.5 * slope * (n - 60.0)).exp();
        self.modes
            .iter()
            .map(|md| {
                let taper = taper * self.beam_drive(md);
                let half = 0.5 * self.drive_length(md.omega) / per_key;
                if half < 1e-3 {
                    let (x, y) = p.bridge_point(note);
                    return taper * self.shape(md, x, y);
                }
                let (mut acc, mut wsum) = (0.0, 0.0);
                for i in 0..K {
                    let u = -1.0 + 2.0 * (i as f64 + 0.5) / K as f64;
                    let w = (0.5 * PI * u).cos().powi(2);
                    let (x, y) = p.bridge_point_at((n + half * u).clamp(lo, hi), main);
                    acc += w * self.shape(md, x, y);
                    wsum += w;
                }
                taper * acc / wsum
            })
            .collect()
    }

    /// The bridge beam's share of the drive for a mode that does not have
    /// the beam in its shape (the rectangle's, above `outline_below`, or a
    /// board without an outline): there the modes overlap and the board is
    /// an infinite plate under an infinite beam, whose impedances add,
    /// Z = 8·√(D·ρh) + 2(1 + i)·(EI·μ³)^¼·√ω. Its conductance against the
    /// plate's alone, √ of it on the shape (1 without the beam). The own
    /// modes with the beam give the same within 0.5 dB at 1 kHz.
    fn beam_drive(&self, md: &PlateMode) -> f64 {
        let p = &self.params;
        if p.bridge_beam <= 0.0 || md.ritz.is_some() || md.m == 0 {
            return 1.0;
        }
        let zp = 1.0 / p.mean_mobility();
        let zb = p.bridge_beam * 2.0 * (p.bridge_stiffness * p.bridge_line_mass.powi(3)).powf(0.25) * md.omega.sqrt();
        let (re, im) = (zp + zb, zb);
        (zp * re / (re * re + im * im)).sqrt()
    }

    /// Plate mobility for a drive with the given per-mode shapes.
    pub fn mobility(&self, shapes: &[f64], omega: f64) -> C64 {
        let mut y_sum = C64::ZERO;
        for (md, phi) in self.modes.iter().zip(shapes) {
            let den = C64::new(md.omega * md.omega - omega * omega, self.mode_eta(md) * md.omega * omega);
            y_sum += C64::new(0.0, omega * phi * phi) / den;
        }
        y_sum
    }

    /// Mobility seen by a key's strings: plate driven through the bridge
    /// (shapes from [`Self::bridge_shapes`]), with the bridge mass, in
    /// series with the pin/cap compliance.
    pub fn bridge_admittance(&self, shapes: &[f64], omega: f64) -> C64 {
        let yp = self.mobility(shapes, omega);
        let with_mass = yp / (C64::ONE + C64::new(0.0, omega * self.params.bridge_mass) * yp);
        with_mass + C64::new(0.0, omega / self.params.series_stiffness)
    }

    /// Conductance (s/kg) of the edge loss seen by `note`'s strings at ω
    /// (see [`SoundboardParams::edge_loss`]).
    pub fn edge_conductance(&self, note: u8, omega: f64) -> f64 {
        let p = &self.params;
        if p.edge_loss <= 0.0 {
            return 0.0;
        }
        let (x, y) = p.bridge_point_at(note as f64, note >= p.main_bridge_from);
        let d = match &self.outline {
            Some(o) => o.edge_distance(x, y),
            None => x.min(p.a - x).min(y).min(p.b - y).max(0.0),
        };
        let f = omega / TAU;
        p.edge_loss * (-d / p.edge_depth.max(1e-6)).exp() / (1.0 + (f / p.edge_corner.max(1.0)).powi(2))
    }

    /// Corner (rad/s) of the smooth low-pass F/(1 + iω·m·Y∞) through which
    /// the bridge forces reach the board.
    pub fn bridge_mass_corner(&self) -> Option<f64> {
        (self.params.bridge_mass > 0.0).then(|| 1.0 / (self.params.bridge_mass * self.params.mean_mobility()))
    }

    /// Driving-point mobility of the bare plate (m/s per N) at (x, y).
    pub fn admittance(&self, x: f64, y: f64, omega: f64) -> C64 {
        let mut y_sum = C64::ZERO;
        for md in &self.modes {
            let phi = self.shape(md, x, y);
            let den = C64::new(md.omega * md.omega - omega * omega, self.mode_eta(md) * md.omega * omega);
            y_sum += C64::new(0.0, omega * phi * phi) / den;
        }
        y_sum
    }

    /// Complex radiation weight of each mode at each microphone: pressure
    /// (Pa) per unit modal velocity, by the Rayleigh integral at ω_n.
    pub fn radiation(&self) -> Vec<[C64; 2]> {
        let p = &self.params;
        let mics = p.mics.map(|(x, y, z)| [x, y, z]);
        self.field(&mics, p.grid_step, self.modes.len()).into_iter().map(|v| [v[0], v[1]]).collect()
    }

    /// The radiation weights of every mode evaluated at one driving
    /// frequency `omega` instead of each mode's own: what the modes' sum
    /// radiates when driven there (the direct field of a point force is a
    /// sum of modes off resonance). Diagnostics.
    pub fn radiation_at(&self, omega: f64) -> Vec<[C64; 2]> {
        let p = &self.params;
        let mics = p.mics.map(|(x, y, z)| [x, y, z]);
        self.field_with(&mics, p.grid_step, self.modes.len(), Some(omega)).into_iter().map(|v| [v[0], v[1]]).collect()
    }

    /// Pressure per unit modal velocity of the first `n_modes` modes at
    /// `points` (board coordinates, z < 0 below or behind the board), on
    /// a radiation grid of `step` m.
    fn field(&self, points: &[[f64; 3]], step: f64, n_modes: usize) -> Vec<Vec<C64>> {
        self.field_with(points, step, n_modes, None)
    }

    /// [`Self::field`] with the Rayleigh integral taken at `omega` for
    /// every mode when given, else at each mode's own frequency.
    fn field_with(&self, points: &[[f64; 3]], step: f64, n_modes: usize, omega: Option<f64>) -> Vec<Vec<C64>> {
        let p = &self.params;
        let nx = (p.a / step).ceil() as usize;
        let ny = (p.b / step).ceil() as usize;
        let (dx, dy) = (p.a / nx as f64, p.b / ny as f64);
        // Paths from every grid cell to every point, computed once.
        let lid = Lid::new(p);
        let np = points.len();
        let mut geo: Vec<Path> = Vec::with_capacity(nx * ny * np);
        for i in 0..nx {
            for j in 0..ny {
                let c = [(i as f64 + 0.5) * dx, (j as f64 + 0.5) * dy, 0.0];
                geo.extend(points.iter().map(|&m| p.path(lid.as_ref(), c, m)));
            }
        }
        let ds = dx * dy;
        // With an outline, only the cells inside it radiate.
        let cover: Option<Vec<f64>> = self.outline.as_ref().map(|o| {
            (0..nx * ny).map(|c| o.coverage((c / ny) as f64 * dx + 0.5 * dx, (c % ny) as f64 * dy + 0.5 * dy, dx, dy)).collect()
        });
        // The outlined board's own modes, on the basis's sines tabulated once.
        let (mm, nm) = self.basis.iter().fold((0, 0), |(a, b), &(m, n, _)| (a.max(m as usize), b.max(n as usize)));
        let tab = |count: usize, cells: usize, d: f64, len: f64| -> Vec<Vec<f64>> {
            (0..=count).map(|m| (0..cells).map(|i| (m as f64 * PI * (i as f64 + 0.5) * d / len).sin()).collect()).collect()
        };
        let (bx, by) = if self.ritz.is_empty() { (Vec::new(), Vec::new()) } else { (tab(mm, nx, dx, p.a), tab(nm, ny, dy, p.b)) };
        // The scattering layer: (x, y, z, area) of its sources, on a grid
        // inside the outline, their heights spread by a fixed pseudo-random
        // sequence (the same layer for every build).
        let scat: Vec<(f64, f64, f64, f64, f64)> = if p.scatter > 0.0 && p.scatter_step > 0.0 {
            let (sx, sy) = ((p.a / p.scatter_step).ceil().max(1.0) as usize, (p.b / p.scatter_step).ceil().max(1.0) as usize);
            let (ex, ey) = (p.a / sx as f64, p.b / sy as f64);
            let mut seed = 0x9E37_79B9u32;
            let mut out = Vec::new();
            for i in 0..sx {
                for j in 0..sy {
                    seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                    let u = (seed >> 8) as f64 / (1u32 << 24) as f64;
                    seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                    let v = (seed >> 8) as f64 / (1u32 << 24) as f64;
                    let (x, y) = ((i as f64 + 0.5) * ex, (j as f64 + 0.5) * ey);
                    let inside = self.outline.as_ref().map_or(1.0, |o| o.coverage(x, y, ex, ey));
                    if inside > 0.5 {
                        // Each obstacle scatters with a phase of its own
                        // (its shape, its height, its stiffness): the layer
                        // adds in power, not in phase.
                        out.push((x, y, p.scatter_height * (0.4 + 1.2 * u), ex * ey, TAU * v));
                    }
                }
            }
            out
        } else {
            Vec::new()
        };
        crate::par::map(n_modes, |i| {
            let md = &self.modes[i];
            let w = omega.unwrap_or(md.omega);
            let k = w / C0;
            let pre = C64::new(0.0, w * RHO0 / TAU);
            let case = p.case_gain(w);
            // Separable mode shape: tabulate the sines once per mode.
            let piston = md.m == 0;
            let sx: Vec<f64> = (0..nx)
                .map(|i| if piston { 1.0 } else { (md.m as f64 * PI * (i as f64 + 0.5) * dx / p.a).sin() })
                .collect();
            let sy: Vec<f64> = (0..ny)
                .map(|j| if piston { 1.0 } else { (md.n as f64 * PI * (j as f64 + 0.5) * dy / p.b).sin() })
                .collect();
            let norm = if piston { p.piston_radiation / p.piston_mass.sqrt() } else { 2.0 / (p.rho_h * p.a * p.b).sqrt() };
            // Σ_k c_k·sin(m_k·x)·sin(n_k·y) on the grid: rows m first.
            let own: Option<Vec<f64>> = md.ritz.filter(|_| !piston).map(|r| {
                let mut rows = vec![vec![0.0; ny]; mm + 1];
                for (&(m, n, _), c) in self.basis.iter().zip(&self.ritz[r as usize]) {
                    for (v, s) in rows[m as usize].iter_mut().zip(&by[n as usize]) {
                        *v += c * s;
                    }
                }
                let mut grid = vec![0.0; nx * ny];
                for (m, row) in rows.iter().enumerate().filter(|(_, r)| r.iter().any(|&v| v != 0.0)) {
                    for (i, sxi) in bx[m].iter().enumerate() {
                        for (g, v) in grid[i * ny..(i + 1) * ny].iter_mut().zip(row) {
                            *g += sxi * v;
                        }
                    }
                }
                grid
            });
            // The scattering layer shadows the direct sound of a point above
            // the board by the share of power it intercepts.
            let shade = if p.scatter > 0.0 && p.scatter_shadow > 0.0 {
                let r = (w / (TAU * p.scatter_corner.max(1.0))).powi(2);
                (1.0 - p.scatter_shadow.clamp(0.0, 1.0) * r / (1.0 + r)).sqrt()
            } else {
                1.0
            };
            let mut acc = vec![C64::ZERO; np];
            for (c, g) in geo.chunks(np).enumerate() {
                // (Same operation order as without an outline: bit-identical.)
                let mut phi = own.as_ref().map_or_else(|| norm * sx[c / ny] * sy[c % ny], |o| norm * o[c]) * ds;
                if let Some(w) = &cover {
                    phi *= w[c];
                }
                for ((a, pt), m) in acc.iter_mut().zip(g).zip(points) {
                    // Front face (through the case) and its lid image in
                    // phase, back face and its wall image in antiphase.
                    let front = if m[2] >= 0.0 { shade } else { 1.0 };
                    *a += C64::polar(front * case * pt.front.1 * phi / pt.front.0, -k * pt.front.0)
                        - C64::polar(pt.back.1 * phi / pt.back.0, -k * pt.back.0);
                    if pt.wall.1 != 0.0 {
                        *a += C64::polar(-pt.wall.1 * phi / pt.wall.0, -k * pt.wall.0);
                    }
                    if pt.image.1 != 0.0 {
                        *a += C64::polar(front * case * pt.image.1 * phi / pt.image.0, -k * pt.image.0);
                    }
                }
            }
            if !scat.is_empty() {
                // Rayleigh scattering below the corner (∝ f² in amplitude),
                // geometric above (the scattered pressure follows the
                // obstacle's incident pressure, ∝ the velocity, not its
                // time derivative as the direct field does: ω_c/ω takes
                // the prefactor's ω out).
                let wc = TAU * p.scatter_corner.max(1.0);
                let r = (w / wc).powi(2);
                let sigma = p.scatter * r / (1.0 + r) * (wc / w);
                for &(x, y, z, area, phase) in &scat {
                    let phi = self.shape(md, x, y) * area * sigma;
                    for (a, m) in acc.iter_mut().zip(points) {
                        if m[2] < 0.0 {
                            continue;
                        }
                        let d = ((m[0] - x).powi(2) + (m[1] - y).powi(2) + (m[2] - z).powi(2)).sqrt().max(0.05);
                        *a += C64::polar(phi / d, phase - k * d);
                    }
                }
            }
            acc.into_iter().map(|a| pre * a).collect()
        })
    }

    /// Modal density (modes per Hz) between f1 and f2.
    pub fn modal_density(&self, f1: f64, f2: f64) -> f64 {
        let c = self.modes.iter().filter(|m| (TAU * f1..TAU * f2).contains(&m.omega)).count();
        c as f64 / (f2 - f1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The 2 m grand as the uniform rectangle it was calibrated as before
    /// its outline went on (the outline's recalibration undone).
    fn rectangle() -> SoundboardParams {
        SoundboardParams {
            outline: [(0.0, 0.0); OUTLINE_POINTS],
            rho_h: 7.0,
            dx: 3825.0,
            dy: 1700.0,
            impedance_slope: 0.02,
            impedance_slope_bass: 0.0,
            ..SoundboardParams::GRAND_2M
        }
    }

    #[test]
    fn the_bowed_bass_bridge_is_an_arc_through_its_ends() {
        let p = SoundboardParams { bass_bridge_bow: 0.05, ..SoundboardParams::GRAND_2M };
        let hi = p.main_bridge_from - 1;
        let end = |u: (f64, f64)| (u.0 * p.a, u.1 * p.b);
        let (top, a0) = (p.bridge_point(hi), p.bridge_point(21));
        let (e0, e1) = (end(p.bass_bridge[0]), end(p.bass_bridge[1]));
        assert!((top.0 - e0.0).hypot(top.1 - e0.1) < 1e-9 && (a0.0 - e1.0).hypot(a0.1 - e1.1) < 1e-9);
        // The middle lies the bow away from the chord, towards the tail.
        let m = p.bridge_point_at((hi as f64 + 21.0) / 2.0, false);
        let mid = ((e0.0 + e1.0) / 2.0, (e0.1 + e1.1) / 2.0);
        assert!(((m.0 - mid.0).hypot(m.1 - mid.1) - 0.05).abs() < 1e-9 && m.0 > mid.0);
        // Equal steps between keys.
        let gap = |n: u8| {
            let (a, b) = (p.bridge_point(n), p.bridge_point(n + 1));
            (a.0 - b.0).hypot(a.1 - b.1)
        };
        for n in 21..hi {
            assert!((gap(n) - gap(21)).abs() < 1e-9, "key {n}");
        }
    }

    #[test]
    fn slanting_main_strings_turn_about_their_keys() {
        let upright = SoundboardParams { main_string_slant: 0.0, ..SoundboardParams::GRAND_2M };
        let p = SoundboardParams { main_string_slant: 8.0, ..upright };
        for n in p.main_bridge_from..=108 {
            let tan = p.string_slant(n as f64).tan();
            let (key, (x0, y0), (x, y)) = (p.key_across(n), upright.bridge_point(n), p.bridge_point(n));
            // The key stays where it was, its string keeps its length...
            assert!((key - upright.key_across(n)).abs() < 1e-12 && (key - y0).abs() < 1e-12, "key {n}");
            let len = |x: f64, y: f64| (x - KEY_LINE_X).hypot(y - key);
            assert!((len(x, y) - len(x0, y0)).abs() < 1e-9, "key {n}");
            // ... and leans towards the bass side (+y) by the angle.
            assert!(((y - key) / (x - KEY_LINE_X) - tan).abs() < 1e-9, "key {n}");
        }
        // Whatever the angle asked, the bridge stays on the board, at least
        // SLANT_EDGE inside its sides; the editor's range keeps it inside
        // the outline untouched.
        let outline = Outline::new(&p).unwrap();
        for a in [12.0, 20.0, 45.0, 90.0] {
            let q = SoundboardParams { main_string_slant: a, ..upright };
            for n in q.main_bridge_from..=108 {
                let (x, y) = q.bridge_point(n);
                assert!(outline.contains(x, y) && y <= q.b - SLANT_EDGE + 1e-9 && x >= SLANT_EDGE - 1e-9, "{a}°, key {n}: ({x}, {y})");
            }
        }
        // Towards the treble: upright.
        let back = SoundboardParams { main_string_slant: -5.0, ..upright };
        assert!((back.main_bridge_from..=108).all(|n| back.bridge_point(n) == upright.bridge_point(n)));
        let max = SoundboardParams { main_string_slant: 12.0, ..upright };
        for n in max.main_bridge_from..=108 {
            let fan = 12f64.to_radians() * (108.0 - n as f64) / (108.0 - max.main_bridge_from as f64);
            assert!((max.string_slant(n as f64) - fan).abs() < 1e-12, "key {n}");
        }
        // They fan: the lowest key at the full angle, C8 upright.
        assert!((p.string_slant(p.main_bridge_from as f64) - 8f64.to_radians()).abs() < 1e-12 && p.string_slant(108.0) == 0.0);
        // The tenor moves most, C8 hardly; the bass bridge not at all.
        let shift = |n: u8| p.bridge_point(n).1 - upright.bridge_point(n).1;
        assert!(shift(p.main_bridge_from) > 0.15 && shift(108) == 0.0);
        for n in 21..p.main_bridge_from {
            assert_eq!(p.bridge_point(n), upright.bridge_point(n), "key {n}");
            assert_eq!(p.key_across(n), upright.key_across(n), "key {n}");
        }
    }

    #[test]
    fn the_main_bridge_is_smooth_through_its_points() {
        let p = SoundboardParams { main_string_slant: 0.0, ..SoundboardParams::GRAND_2M };
        let lo = p.main_bridge_from as f64;
        for (i, &(u, v)) in p.treble_bridge.iter().enumerate() {
            let (x, y) = p.bridge_point_at(108.0 - (108.0 - lo) * i as f64 / 4.0, true);
            assert!((x - u * p.a).abs() < 1e-9 && (y - v * p.b).abs() < 1e-9, "point {i}");
        }
        // The keys' spacing along it changes little from key to key (the
        // straight segments jumped from 19 to 38 mm at G#4 and C#3).
        let gap = |n: f64| {
            let (a, b) = (p.bridge_point_at(n - 0.5, true), p.bridge_point_at(n + 0.5, true));
            (a.0 - b.0).hypot(a.1 - b.1)
        };
        for n in p.main_bridge_from + 1..107 {
            let (g0, g1) = (gap(n as f64), gap(n as f64 + 1.0));
            assert!((g1 - g0).abs() < 4e-3, "key {n}: {g0} → {g1}");
        }
    }

    #[test]
    fn modal_density_and_mean_mobility_match_measurements() {
        let sb = Soundboard::new(rectangle());
        let p = sb.params;
        // Ege et al. 2013: ≈ 1 mode per 19.5 Hz.
        let dens = sb.modal_density(200.0, 1000.0);
        assert!((1.0 / dens - 20.0).abs() < 6.0, "1 mode per {:.1} Hz", 1.0 / dens);
        // Calibrated level, of the order of Ege's measured 1.3e-3 s/kg.
        assert!((p.mean_mobility() - 9.4e-4).abs() < 1e-4, "{}", p.mean_mobility());
        // Frequency average of Re Y at a bridge point ≈ infinite-plate value.
        let (x, y) = p.bridge_point(60);
        let n = 4000;
        let mean: f64 = (0..n)
            .map(|i| {
                let f = 300.0 + 2700.0 * i as f64 / n as f64;
                sb.admittance(x, y, TAU * f).re
            })
            .sum::<f64>()
            / n as f64;
        assert!(mean > 0.3 * p.mean_mobility() && mean < 3.0 * p.mean_mobility(), "mean Re Y {mean:e}");
    }

    #[test]
    fn an_upright_uses_its_own_microphones() {
        let p = SoundboardParams { mics_relative: true, mics: [(0.5, -1.0, 2.0); 2], mics_upright: [(-0.2, 0.7, 2.0), (-0.2, 0.3, 2.0)], ..SoundboardParams::GRAND_2M };
        assert_eq!(p.resolved().mics[0], (0.5 * p.a, -p.b, 2.0));
        let up = SoundboardParams { vertical: true, ..p }.resolved();
        assert_eq!(up.mics[0], (-0.2 * p.a, 0.7 * p.b, 2.0));
        assert_eq!(up.resolved().mics, up.mics, "resolving twice changes nothing");
    }

    #[test]
    fn wall_reflects_only_the_back_face() {
        let base = SoundboardParams { f_max: 400.0, grid_step: 0.05, ..SoundboardParams::GRAND_2M };
        let low = |p: SoundboardParams| Soundboard::new(p).radiation()[0][0].norm();
        let walled = SoundboardParams { wall_distance: 0.3, ..base };
        // The lowest mode is a dipole: the wall's image of the back face
        // adds to it in phase with the back face.
        assert!((low(walled) - low(base)).abs() > 0.05 * low(base));
        let baffled = SoundboardParams { back_gain: 0.0, ..base };
        assert_eq!(low(SoundboardParams { wall_distance: 0.3, ..baffled }), low(baffled));
    }

    /// The open lid has the case's shape: over the rim along the spine and
    /// the front, none over the bentside's missing corner at the tail.
    #[test]
    fn the_lid_has_the_case_shape() {
        let p = SoundboardParams { lid_angle: 35.0, lid_removed: false, ..SoundboardParams::GRAND_2M };
        let lid = Lid::new(&p).unwrap();
        let th = p.lid_angle.to_radians();
        // The point of the open lid over (x, y) of the case.
        let at = |x: f64, y: f64| [x, p.b - (p.b - y) * th.cos(), p.lid_hinge_height + (p.b - y) * th.sin()];
        assert!(lid.on_lid(at(0.5 * p.a, 0.5 * p.b)));
        assert!(lid.on_lid(at(-0.05, 0.5 * p.b)));
        assert!(lid.on_lid(at(p.a + 0.05, 0.9 * p.b)));
        assert!(lid.on_lid(at(0.05 * p.a, -0.05)));
        assert!(!lid.on_lid(at(0.95 * p.a, 0.1 * p.b)));
        assert!(!lid.on_lid(at(p.a + 0.1, 0.9 * p.b)));
        // The rectangle's lid without an outline.
        let rect = Lid::new(&SoundboardParams { outline: [(0.0, 0.0); OUTLINE_POINTS], ..p }).unwrap();
        assert!(rect.on_lid(at(0.95 * p.a, 0.1 * p.b)));
    }

    /// Shutting the lid, or opening it from shut, is continuous: its image
    /// and shadow fade out over its last degrees (close pair under a 35°
    /// lid; the audience pair, shut, opened by a hair as the lid macro does).
    #[test]
    fn shutting_the_lid_is_continuous() {
        let close = SoundboardParams {
            f_max: 1000.0,
            grid_step: 0.05,
            lid_angle: 35.0,
            lid_removed: false,
            mics_relative: true,
            mics: [(0.294118, 0.346154, 0.4), (0.705882, 0.653846, 0.4)],
            ..SoundboardParams::GRAND_2M
        }
        .resolved();
        let audience = SoundboardParams { lid_angle: 0.0, mics: [(0.55, -1.6, 1.0), (1.25, -1.6, 1.0)], ..close };
        let gap = |a: SoundboardParams, b: SoundboardParams| {
            let (a, b) = (Soundboard::new(a).radiation(), Soundboard::new(b).radiation());
            let d: f64 = a.iter().zip(&b).flat_map(|(x, y)| (0..2).map(move |m| (x[m] - y[m]).norm_sqr())).sum();
            let e: f64 = a.iter().flat_map(|x| x.iter().map(|c| c.norm_sqr())).sum();
            (d / e).sqrt()
        };
        let eps = 1e-6;
        let at = |angle: f64, p: SoundboardParams| SoundboardParams { lid_angle: angle, ..p };
        assert!(gap(at(LID_CLOSING * eps, close), at(0.0, close)) < 1e-4);
        assert!(gap(audience, at(LID_CLOSING * eps, audience)) < 1e-4);
        assert!(gap(at(LID_CLOSING, close), at(LID_CLOSING * (1.0 - eps), close)) < 1e-4);
        // An open lid still shapes the close pair's sound.
        assert!(gap(close, at(0.0, close)) > 0.1);
    }

    /// No outline, or one without area: the rectangle, bit for bit.
    #[test]
    fn an_outline_without_area_is_the_rectangle() {
        let base = SoundboardParams { f_max: 3000.0, grid_step: 0.05, ..rectangle() };
        let bits = |p: SoundboardParams| {
            let sb = Soundboard::new(p);
            assert!(sb.outline.is_none() && sb.ritz.is_empty());
            let mut v: Vec<u64> = sb.modes.iter().map(|m| m.omega.to_bits()).collect();
            v.extend(sb.bridge_shapes(40).iter().map(|x| x.to_bits()));
            v.extend(sb.radiation().iter().flat_map(|r| r.iter().flat_map(|c| [c.re.to_bits(), c.im.to_bits()])));
            v.push(sb.edge_conductance(21, 600.0).to_bits());
            v
        };
        let want = bits(base);
        assert_eq!(bits(SoundboardParams { outline: [(0.3, 0.6); OUTLINE_POINTS], ..base }), want);
        let mut line = [(1.0, 1.0); OUTLINE_POINTS];
        line[0] = (0.0, 0.0);
        assert_eq!(bits(SoundboardParams { outline: line, ..base }), want);
    }

    /// A shorter rectangle taken as an outline gets the modes of a board
    /// that short.
    #[test]
    fn a_shorter_outline_has_the_shorter_boards_modes() {
        let base = rectangle();
        let r = 0.75;
        let mut outline = [(0.0, 1.0); OUTLINE_POINTS];
        outline[..4].copy_from_slice(&[(0.0, 0.0), (r, 0.0), (r, 1.0), (0.0, 1.0)]);
        let sb = Soundboard::new(SoundboardParams { outline, ..base });
        let short = Soundboard::new(SoundboardParams { a: r * base.a, ..base });
        let own: Vec<f64> = sb.modes.iter().filter(|m| m.ritz.is_some()).map(|m| m.omega).collect();
        let want: Vec<f64> = short.modes.iter().filter(|m| m.m > 0).map(|m| m.omega).collect();
        assert!(own.len().abs_diff(want.iter().filter(|&&w| w < TAU * base.outline_below).count()) <= 2);
        let err = (own.iter().zip(&want).map(|(a, b)| (a / b - 1.0).powi(2)).sum::<f64>() / own.len() as f64).sqrt();
        assert!(err < 0.015, "rms {:.2} %", 100.0 * err);
        // Above the corner, the rectangle's modes.
        assert!(sb.modes.iter().filter(|m| m.omega >= TAU * base.outline_below).all(|m| m.ritz.is_none()));
    }

    /// The grand's outline: the bridges inside, the board's own modes at
    /// rest outside it, the mean mobility kept (fewer modes, each larger).
    #[test]
    fn a_grand_outline_holds_its_board() {
        let p = SoundboardParams { outline: SoundboardParams::GRAND_OUTLINE, ..SoundboardParams::GRAND_2M };
        let sb = Soundboard::new(p);
        let o = sb.outline.as_ref().unwrap();
        for note in 21..=108 {
            let (x, y) = p.bridge_point(note);
            assert!(o.contains(x, y) && o.edge_distance(x, y) > 0.05, "key {note}");
        }
        let cells = 60;
        let (dx, dy) = (p.a / cells as f64, p.b / cells as f64);
        for md in sb.modes.iter().filter(|m| m.ritz.is_some()) {
            let (mut inside, mut outside) = (0.0, 0.0);
            for c in 0..cells * cells {
                let (x, y) = ((c / cells) as f64 * dx + 0.5 * dx, (c % cells) as f64 * dy + 0.5 * dy);
                let phi2 = sb.shape(md, x, y).powi(2);
                if o.edge_distance(x, y) < 0.05 {
                    continue;
                }
                if o.contains(x, y) { inside += phi2 } else { outside += phi2 }
            }
            assert!(outside < 0.01 * inside, "{:.0} Hz: {:.3}", md.omega / TAU, outside / inside);
        }
        let (x, y) = p.bridge_point(60);
        let n = 4000;
        let mean = (0..n).map(|i| sb.admittance(x, y, TAU * (300.0 + 2700.0 * i as f64 / n as f64)).re).sum::<f64>() / n as f64;
        assert!(mean > 0.3 * p.mean_mobility() && mean < 3.0 * p.mean_mobility(), "mean Re Y {mean:e}");
    }

    #[test]
    fn conductance_is_positive_everywhere() {
        let sb = Soundboard::new(SoundboardParams::GRAND_2M);
        for note in [21u8, 28, 29, 60, 108] {
            let (x, y) = sb.params.bridge_point(note);
            assert!(x > 0.0 && x < sb.params.a && y > 0.0 && y < sb.params.b);
            for f in [30.0, 100.0, 440.0, 2000.0, 9000.0] {
                assert!(sb.admittance(x, y, TAU * f).re > 0.0, "note {note} f {f}");
            }
        }
    }
}
