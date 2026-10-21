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
//! integral (baffled plate), `p = iωρ₀/(2π)·∫ v·e^{−ikR}/R dS`, evaluated
//! once per mode and microphone at the mode frequency (narrow-band modes).

use crate::complex::C64;
use std::f64::consts::{PI, TAU};

#[derive(Clone, Copy, Debug, PartialEq)]
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
    /// Highest mode kept (Hz).
    pub f_max: f64,
    /// Bridge as a polyline in normalised plate coordinates, from C8 down to
    /// the last note of the main bridge; then the bass bridge.
    pub treble_bridge: [(f64, f64); 3],
    pub bass_bridge: [(f64, f64); 2],
    /// First note on the main bridge (lower notes are on the bass bridge).
    pub main_bridge_from: u8,
    /// Microphones (m, plate coordinates, z above the board).
    pub mics: [(f64, f64, f64); 2],
    /// Render the radiated pressure (false: output the bridge force, the
    /// board only sets the string coupling — for analysis).
    pub radiate: bool,
    /// Grid step of the radiation integral (m).
    pub grid_step: f64,
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
        loss_corner: f64::INFINITY,
        bridge_mass: 0.0,
        series_stiffness: 2e7,
        f_max: 20_000.0,
        treble_bridge: [(0.18, 0.22), (0.42, 0.60), (0.62, 0.86)],
        bass_bridge: [(0.72, 0.62), (0.86, 0.40)],
        main_bridge_from: 29,
        mics: [(0.45, 0.35, 1.5), (1.15, 0.85, 1.5)],
        radiate: true,
        grid_step: 0.015,
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
        let lerp = |p: (f64, f64), q: (f64, f64), t: f64| (p.0 + (q.0 - p.0) * t, p.1 + (q.1 - p.1) * t);
        let (u, v) = if note >= self.main_bridge_from {
            // 0 at C8, 1 at the lowest note of the main bridge.
            let t = (108.0 - note as f64) / (108.0 - self.main_bridge_from as f64);
            let tb = &self.treble_bridge;
            if t < 0.5 { lerp(tb[0], tb[1], 2.0 * t) } else { lerp(tb[1], tb[2], 2.0 * t - 1.0) }
        } else {
            let span = (self.main_bridge_from - 21).max(1) as f64;
            let t = (self.main_bridge_from as f64 - 1.0 - note as f64) / span.max(1.0);
            lerp(self.bass_bridge[0], self.bass_bridge[1], t.clamp(0.0, 1.0))
        };
        (u * self.a, v * self.b)
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
                let w = k * (p.dx * xm * xm + 2.0 * p.h() * xm * yn + p.dy * yn * yn).sqrt();
                if w > w_max {
                    break;
                }
                modes.push(PlateMode { m, n, omega: w });
                n += 1;
            }
            m += 1;
        }
        modes.sort_by(|x, y| x.omega.total_cmp(&y.omega));
        Self { params, modes }
    }

    /// Mass-normalised mode shape at (x, y) metres.
    pub fn shape(&self, mode: &PlateMode, x: f64, y: f64) -> f64 {
        let p = &self.params;
        2.0 / (p.rho_h * p.a * p.b).sqrt() * (mode.m as f64 * PI * x / p.a).sin() * (mode.n as f64 * PI * y / p.b).sin()
    }

    pub fn eta(&self, omega: f64) -> f64 {
        self.params.loss_factor * (1.0 + omega / (TAU * self.params.loss_corner))
    }

    pub fn sigma(&self, mode: &PlateMode) -> f64 {
        // η·ω_n is the 2σ of the velocity equation q̈ + η·ω_n·q̇ + ω_n²·q.
        0.5 * self.eta(mode.omega) * mode.omega
    }

    /// Mobility seen by the strings at (x, y): plate with the bridge mass,
    /// in series with the pin/cap compliance.
    pub fn bridge_admittance(&self, x: f64, y: f64, omega: f64) -> C64 {
        let yp = self.admittance(x, y, omega);
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
            let den = C64::new(md.omega * md.omega - omega * omega, self.eta(md.omega) * md.omega * omega);
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
        // Distances from every grid cell to every mic, computed once.
        let mut geo: Vec<[(f64, f64, f64, f64); 2]> = Vec::with_capacity(nx * ny);
        let mut cells = Vec::with_capacity(nx * ny);
        for i in 0..nx {
            for j in 0..ny {
                let (x, y) = ((i as f64 + 0.5) * dx, (j as f64 + 0.5) * dy);
                cells.push((x, y));
                geo.push(std::array::from_fn(|mic| {
                    let (mx, my, mz) = p.mics[mic];
                    let r = ((x - mx).powi(2) + (y - my).powi(2) + mz * mz).sqrt();
                    (r, 0.0, 0.0, 0.0)
                }));
            }
        }
        let ds = dx * dy;
        self.modes
            .iter()
            .map(|md| {
                let k = md.omega / C0;
                let pre = C64::new(0.0, md.omega * RHO0 / TAU);
                let mut acc = [C64::ZERO; 2];
                // Separable mode shape: tabulate the sines once per mode.
                let sx: Vec<f64> = (0..nx).map(|i| (md.m as f64 * PI * (i as f64 + 0.5) * dx / p.a).sin()).collect();
                let sy: Vec<f64> = (0..ny).map(|j| (md.n as f64 * PI * (j as f64 + 0.5) * dy / p.b).sin()).collect();
                let norm = 2.0 / (p.rho_h * p.a * p.b).sqrt();
                for (c, g) in geo.iter().enumerate() {
                    let phi = norm * sx[c / ny] * sy[c % ny];
                    for mic in 0..2 {
                        let r = g[mic].0;
                        acc[mic] += C64::polar(phi * ds / r, -k * r);
                    }
                }
                [pre * acc[0], pre * acc[1]]
            })
            .collect()
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
