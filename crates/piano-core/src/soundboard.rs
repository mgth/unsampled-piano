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
    /// Bridge as a polyline in normalised plate coordinates, from C8 down to
    /// the last note of the main bridge; then the bass bridge.
    pub treble_bridge: [(f64, f64); 5],
    pub bass_bridge: [(f64, f64); 2],
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
    /// Lid opening angle (degrees); 0 = lid removed. It is hinged along the
    /// board's long side y = b, `lid_hinge_height` m above the board, as long
    /// as the board is wide, and opens towards y < 0 (the audience).
    pub lid_angle: f64,
    pub lid_hinge_height: f64,
    /// Pressure reflection coefficient of the lid, and transmission through
    /// it for a microphone behind it (no diffraction modelled).
    pub lid_reflection: f64,
    pub lid_transmission: f64,
}

impl Default for SoundboardParams {
    fn default() -> Self {
        Self::GRAND_2M
    }
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
        rho_h: 7.0,
        dx: 3825.0,
        dy: 1700.0,
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
        // −12.3 dB/s against −12.2).
        loss_low_corner: 0.0,
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
        // ×2.4 at C8, ×0.4 at A0 relative to C4. The treble side was set on
        // Salamander's decay envelopes alone (0.035, ×4.6 at C8,
        // docs/sustain.md); against eight real grands 0.02 lets the top
        // strings lose their prompt sound faster into the bridge (bench
        // 1.53 → 1.51 with the top losses lowered, docs/top_treble.md).
        impedance_slope: 0.02,
        impedance_slope_bass: 0.0,
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
        // Main bridge at equal steps of the main range (C8, ≈ F6, ≈ G#4,
        // ≈ C#3, F1): x follows the speaking length of the 2 m scale from
        // agraffes 3 cm in front of the board's front edge (C8 kept 6 cm in),
        // y the key's place across the 1.2 m wide string band. The treble
        // end sits in the board's clamped corner, as on real grands.
        treble_bridge: [(0.035, 0.077), (0.07, 0.28), (0.21, 0.50), (0.62, 0.71), (0.78, 0.92)],
        bass_bridge: [(0.72, 0.62), (0.86, 0.40)],
        main_bridge_from: 29,
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
        lid_hinge_height: 0.25,
        lid_reflection: 0.85,
        lid_transmission: 0.1,
    };

    fn h(&self) -> f64 {
        (self.dx * self.dy).sqrt()
    }

    /// Gain of the front face's sound through the case at ω.
    pub fn case_gain(&self, omega: f64) -> f64 {
        let tau = 1.0 / (1.0 + (omega * self.case_panel_mass / (2.0 * RHO0 * C0)).powi(2)).sqrt();
        self.case_open + (1.0 - self.case_open) * tau
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
        let lerp = |p: (f64, f64), q: (f64, f64), t: f64| (p.0 + (q.0 - p.0) * t, p.1 + (q.1 - p.1) * t);
        let (u, v) = if main {
            // 0 at C8, 1 at the lowest note of the main bridge.
            let t = ((108.0 - note) / (108.0 - self.main_bridge_from as f64)).clamp(0.0, 1.0);
            let tb = &self.treble_bridge;
            let seg = ((t * 4.0) as usize).min(3);
            lerp(tb[seg], tb[seg + 1], t * 4.0 - seg as f64)
        } else {
            let span = (self.main_bridge_from - 21).max(1) as f64;
            let t = (self.main_bridge_from as f64 - 1.0 - note) / span.max(1.0);
            lerp(self.bass_bridge[0], self.bass_bridge[1], t.clamp(0.0, 1.0))
        };
        (u * self.a, v * self.b)
    }
}

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

/// The open lid as a reflecting rectangle.
struct Lid {
    p0: [f64; 3],
    up: [f64; 3],
    n: [f64; 3],
    width: f64,
    length: f64,
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
    fn new(p: &SoundboardParams) -> Option<Self> {
        (p.lid_angle > 0.0).then(|| {
            let th = p.lid_angle.to_radians();
            Self {
                p0: [0.0, p.b, p.lid_hinge_height],
                up: [0.0, -th.cos(), th.sin()],
                // Normal pointing to the board side.
                n: [0.0, -th.sin(), -th.cos()],
                width: p.a,
                length: 1.05 * p.b,
                reflection: p.lid_reflection,
                transmission: p.lid_transmission,
            }
        })
    }

    fn side(&self, q: [f64; 3]) -> f64 {
        dot(sub(q, self.p0), self.n)
    }

    fn on_lid(&self, x: [f64; 3]) -> bool {
        let t = dot(sub(x, self.p0), self.up);
        (0.0..=self.length).contains(&t) && (-0.05..=self.width + 0.05).contains(&x[0])
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
    pub m: u32,
    pub n: u32,
    pub omega: f64,
}

pub struct Soundboard {
    pub params: SoundboardParams,
    pub modes: Vec<PlateMode>,
}

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
    pub fn new(params: SoundboardParams) -> Self {
        let p = params;
        let w_max = TAU * p.f_max;
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
                modes.push(PlateMode { m, n, omega: w });
                n += 1;
            }
            m += 1;
        }
        if p.piston_freq > 0.0 && p.piston_mass > 0.0 {
            modes.push(PlateMode { m: 0, n: 0, omega: TAU * p.piston_freq });
        }
        modes.sort_by(|x, y| x.omega.total_cmp(&y.omega));
        Self { params, modes }
    }

    /// Mass-normalised mode shape at (x, y) metres (uniform for the piston
    /// mode, m = 0).
    pub fn shape(&self, mode: &PlateMode, x: f64, y: f64) -> f64 {
        let p = &self.params;
        if mode.m == 0 {
            return 1.0 / p.piston_mass.sqrt();
        }
        2.0 / (p.rho_h * p.a * p.b).sqrt() * (mode.m as f64 * PI * x / p.a).sin() * (mode.n as f64 * PI * y / p.b).sin()
    }

    /// Loss factor of a mode.
    pub fn mode_eta(&self, mode: &PlateMode) -> f64 {
        if mode.m == 0 { self.params.piston_loss } else { self.eta(mode.omega) }
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
        let d = x.min(p.a - x).min(y).min(p.b - y).max(0.0);
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

    /// Pressure per unit modal velocity of the first `n_modes` modes at
    /// `points` (board coordinates, z < 0 below or behind the board), on
    /// a radiation grid of `step` m.
    fn field(&self, points: &[[f64; 3]], step: f64, n_modes: usize) -> Vec<Vec<C64>> {
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
        crate::par::map(n_modes, |i| {
            let md = &self.modes[i];
            let k = md.omega / C0;
            let pre = C64::new(0.0, md.omega * RHO0 / TAU);
            let case = p.case_gain(md.omega);
            // Separable mode shape: tabulate the sines once per mode.
            let piston = md.m == 0;
            let sx: Vec<f64> = (0..nx)
                .map(|i| if piston { 1.0 } else { (md.m as f64 * PI * (i as f64 + 0.5) * dx / p.a).sin() })
                .collect();
            let sy: Vec<f64> = (0..ny)
                .map(|j| if piston { 1.0 } else { (md.n as f64 * PI * (j as f64 + 0.5) * dy / p.b).sin() })
                .collect();
            let norm = if piston { p.piston_radiation / p.piston_mass.sqrt() } else { 2.0 / (p.rho_h * p.a * p.b).sqrt() };
            let mut acc = vec![C64::ZERO; np];
            for (c, g) in geo.chunks(np).enumerate() {
                let phi = norm * sx[c / ny] * sy[c % ny] * ds;
                for (a, pt) in acc.iter_mut().zip(g) {
                    // Front face (through the case) and its lid image in
                    // phase, back face and its wall image in antiphase.
                    *a += C64::polar(case * pt.front.1 * phi / pt.front.0, -k * pt.front.0)
                        - C64::polar(pt.back.1 * phi / pt.back.0, -k * pt.back.0);
                    if pt.wall.1 != 0.0 {
                        *a += C64::polar(-pt.wall.1 * phi / pt.wall.0, -k * pt.wall.0);
                    }
                    if pt.image.1 != 0.0 {
                        *a += C64::polar(case * pt.image.1 * phi / pt.image.0, -k * pt.image.0);
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

    #[test]
    fn modal_density_and_mean_mobility_match_measurements() {
        let sb = Soundboard::new(SoundboardParams::GRAND_2M);
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
