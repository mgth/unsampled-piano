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
//! distance): an unbaffled board short-circuits its low frequencies.
//! Evaluated once per mode and microphone at the mode frequency, and
//! rendered on the modal acceleration ([`crate::board::to_acceleration_weights`]),
//! so that a mode driven below its resonance radiates at the driving
//! frequency's rate.

use crate::complex::C64;
use std::f64::consts::{PI, TAU};

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
    /// (docs/bass_radiation.md).
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
        bridge_mass: 0.0,
        series_stiffness: 2e7,
        // Maple, ≈ 3 × 2.5 cm; spread over 0.3 bending wavelength (set with
        // the ESPRIT decays of the upper partials, docs/sustain.md).
        bridge_stiffness: 470.0,
        bridge_line_mass: 0.6,
        bridge_spread: 0.3,
        // Calibrated on the decay envelopes of Salamander (docs/sustain.md):
        // ×4.6 at C8, ×0.4 at A0 relative to C4.
        impedance_slope: 0.035,
        impedance_slope_bass: -0.03,
        f_max: 20_000.0,
        // A broad mode near 40 Hz (docs/bass.md). Mass set on the late
        // sustain of the bass (docs/long_sustain.md): with 1.5 kg it took
        // 37 dB from the 63 Hz band between 0.3 and 3 s (references 6–21).
        piston_freq: 40.0,
        piston_loss: 0.7,
        piston_mass: 6.0,
        // The case moving as a block barely radiates (the board's two faces
        // and the open bottom short-circuit it): 0.3 put the bass
        // fundamentals 25–30 dB over the references below 40 Hz.
        piston_radiation: 0.03,
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
        mics_relative: false,
        mic_gain_db: 0.0,
        mic_pattern: 1.0,
        radiate: true,
        grid_step: 0.015,
        // Calibrated on the spectral tilt of Salamander (low bands).
        back_depth: 0.6,
        back_gain: 0.6,
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

/// One radiation path from a board cell to a mic.
#[derive(Clone, Copy)]
struct Path {
    r: f64,
    /// 1, or the lid's transmission when it shadows the mic.
    direct: f64,
    /// Distance of the back-face image.
    rb: f64,
    /// Lid image: (distance, gain), gain 0 when the reflection misses.
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
    /// The same set-up with the microphones in metres.
    pub fn resolved(self) -> Self {
        if !self.mics_relative {
            return self;
        }
        Self { mics: self.mics.map(|(x, y, z)| (x * self.a, y * self.b, z)), mics_relative: false, ..self }
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
        self.params.loss_factor * (1.0 + omega / (TAU * self.params.loss_corner))
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
        const RHO0: f64 = 1.2;
        const C0: f64 = 343.0;
        let p = &self.params;
        let nx = (p.a / p.grid_step).ceil() as usize;
        let ny = (p.b / p.grid_step).ceil() as usize;
        let (dx, dy) = (p.a / nx as f64, p.b / ny as f64);
        // Paths from every grid cell to every mic, computed once: direct
        // (possibly shadowed by the lid), back face, and lid reflection.
        let lid = Lid::new(p);
        let mut geo: Vec<[Path; 2]> = Vec::with_capacity(nx * ny);
        for i in 0..nx {
            for j in 0..ny {
                let (x, y) = ((i as f64 + 0.5) * dx, (j as f64 + 0.5) * dy);
                geo.push(std::array::from_fn(|mic| {
                    let (mx, my, mz) = p.mics[mic];
                    let (c, m) = ([x, y, 0.0], [mx, my, mz]);
                    let d2 = (x - mx).powi(2) + (y - my).powi(2);
                    let (direct, image) = match &lid {
                        Some(l) => (l.direct_gain(c, m), l.image(c, m)),
                        None => (1.0, None),
                    };
                    Path {
                        r: (d2 + mz * mz).sqrt(),
                        direct,
                        rb: (d2 + (mz + p.back_depth).powi(2)).sqrt(),
                        image: image.unwrap_or((0.0, 0.0)),
                    }
                }));
            }
        }
        let ds = dx * dy;
        crate::par::map(self.modes.len(), |i| {
                let md = &self.modes[i];
                let k = md.omega / C0;
                let pre = C64::new(0.0, md.omega * RHO0 / TAU);
                let mut acc = [C64::ZERO; 2];
                // Separable mode shape: tabulate the sines once per mode.
                let piston = md.m == 0;

                let sx: Vec<f64> = (0..nx)
                    .map(|i| if piston { 1.0 } else { (md.m as f64 * PI * (i as f64 + 0.5) * dx / p.a).sin() })
                    .collect();
                let sy: Vec<f64> = (0..ny)
                    .map(|j| if piston { 1.0 } else { (md.n as f64 * PI * (j as f64 + 0.5) * dy / p.b).sin() })
                    .collect();
                let norm = if piston {
                    p.piston_radiation / p.piston_mass.sqrt()
                } else {
                    2.0 / (p.rho_h * p.a * p.b).sqrt()
                };
                for (c, g) in geo.iter().enumerate() {
                    let phi = norm * sx[c / ny] * sy[c % ny];
                    for (mic, pt) in g.iter().enumerate() {
                        acc[mic] += C64::polar(pt.direct * phi * ds / pt.r, -k * pt.r)
                            - C64::polar(p.back_gain * phi * ds / pt.rb, -k * pt.rb);
                        if pt.image.1 != 0.0 {
                            acc[mic] += C64::polar(pt.image.1 * phi * ds / pt.image.0, -k * pt.image.0);
                        }
                    }
                }
                [pre * acc[0], pre * acc[1]]
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
