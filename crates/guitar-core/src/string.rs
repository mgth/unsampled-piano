//! Modes of one string stopped at a given length, in its two polarisations,
//! on the bridge's admittance.
//!
//! - Frequencies: stiff string, ω_n = 2π·n·f₀·√(1 + B·n²) with
//!   B = π³·E·d⁴/(64·T·L²) (Fletcher 1964), d the core diameter.
//! - The string's own losses (Valette & Cuesta 1993): loss factor
//!   η_n = air + viscoelastic + internal, σ_n = η_n·ω_n/2. Air: R/(μω) with
//!   R = 2πη_a + 2πd·√(πη_a·ρ_a·f) per unit length (viscous drag on a
//!   cylinder). Viscoelastic: δ_ve times the bending share of the mode's
//!   energy, B·n²/(1 + B·n²).
//! - Bridge: an end of admittance Y (2×2 over the polarisations, Y = v/F)
//!   reflects with 1 − 2Z₀Y, which shifts each partial's complex frequency
//!   by i·Z₀·Y·ω_n/(nπ) (Z₀ = √(Tμ); weak coupling, Gough 1981). The 2×2
//!   eigenproblem per partial gives two modes, each a mix of both
//!   polarisations: the decay rate is Z₀·Re Y·ω_n/(nπ), and a cross term
//!   trades energy between the directions (two-stage decay, beats).
//!   Passive whenever Re Y is positive semi-definite: then every eigenvalue
//!   of Y has Re ≥ 0, so no mode grows.
//! - Pluck: the string held in a triangle of height h at x₀ from the bridge
//!   and released from rest, y(x, t) = Σ a_n·sin(nπx/L)·e^{−σt}·cos(ω_n t),
//!   a_n = 2hL²·sin(nπx₀/L)/(n²π²·x₀(L − x₀)), the corner rounded over the
//!   contact width w by sinc(nπw/2L).
//! - Outputs: the perpendicular force on the bridge, F = T·∂y/∂x at
//!   x = 0, weight T·nπ/L per partial; and the instrument's output
//!   ([`crate::params::Output`]): that force scaled (bridge, piezo), or the
//!   EMF of the selected pickups, S·∫W(x)·ẏ(x) dx over each coil's window
//!   (aperture a: weight sin(kx_p)·sinc(ka/2); a humbucker's two coils
//!   added). Pickups in parallel make the mean of their EMFs.
//! - Mute: a dashpot R spread over a fingertip of width w centred at x_m
//!   adds R·⟨sin²(nπx/L)⟩/(μL) to σ_n (modal mass μL/2), the bank's damper
//!   weight.
//!
//! State conventions: ζ (one per partial and polarisation) is the complex
//! amplitude of the uncoupled mode, displacement q = Re ζ, ζ ∝ e^{iωt}; the
//! bank runs the coupled amplitudes z = E⁻¹·ζ (index 2n + k).

use piano_core::complex::C64;
use piano_core::modal::{ModeSpec, ModeTable};

use crate::params::{Admittance, GuitarModel, Output, PickupKind, StringParams, SupportParams};

pub const PERP: usize = 0;
pub const PAR: usize = 1;

const AIR_VISCOSITY: f64 = 1.8e-5;
const AIR_DENSITY: f64 = 1.2;

pub fn midi_hz(note: f64) -> f64 {
    440.0 * 2f64.powf((note - 69.0) / 12.0)
}

/// Linear density (kg/m) of a string tuned to its open note.
pub fn linear_density(s: &StringParams, scale_length: f64) -> f64 {
    let f0 = midi_hz(s.open_note as f64);
    s.tension / (2.0 * scale_length * f0).powi(2)
}

/// Inharmonicity coefficient B at vibrating length `length`.
pub fn inharmonicity(s: &StringParams, length: f64) -> f64 {
    std::f64::consts::PI.powi(3) * s.young * s.core_diameter.powi(4) / (64.0 * s.tension * length * length)
}

/// Vibrating length at a fret (equal temperament, no compensation).
pub fn fret_length(scale_length: f64, fret: u8) -> f64 {
    scale_length * 2f64.powf(-(fret as f64) / 12.0)
}

/// Loss factor of the string itself at frequency `f` with bending share `bend`.
fn loss_factor(s: &StringParams, mu: f64, f: f64, bend: f64) -> f64 {
    use std::f64::consts::PI;
    let r = 2.0 * PI * AIR_VISCOSITY + 2.0 * PI * s.diameter * (PI * AIR_VISCOSITY * AIR_DENSITY * f).sqrt();
    r / (mu * 2.0 * PI * f) + s.delta_ve * bend + s.eta_int
}

fn admittance(a: &Admittance, f: f64) -> C64 {
    C64::new(a.conductance + a.conductance_per_khz * f / 1000.0, a.susceptance)
}

/// One partial in both polarisations.
#[derive(Clone, Copy, Debug)]
pub struct Partial {
    /// Wavenumber nπ/L (mode shape sin(kx), x from the bridge).
    pub k: f64,
    /// On rigid supports (rad/s), and the string's own decay rate (1/s).
    pub omega: f64,
    pub sigma: f64,
    /// Extra decay rate when muted (bank damper scale 1).
    pub damp_w: f64,
    /// Each polarisation on its own admittance, the cross term left out:
    /// (ω, σ). What the contact solver runs.
    pub polar: [(f64, f64); 2],
    /// Coupled modes: complex frequency shifts, E[p][k] (polarisation p of
    /// mode k) and E⁻¹.
    pub shift: [C64; 2],
    pub e: [[C64; 2]; 2],
    pub e_inv: [[C64; 2]; 2],
}

impl Partial {
    /// ζ (per polarisation) of coupled amplitudes `z`.
    pub fn polar_of(&self, z: [C64; 2]) -> [C64; 2] {
        let e = &self.e;
        [e[0][0] * z[0] + e[0][1] * z[1], e[1][0] * z[0] + e[1][1] * z[1]]
    }

    /// Coupled amplitudes of ζ.
    pub fn coupled_of(&self, zeta: [C64; 2]) -> [C64; 2] {
        let e = &self.e_inv;
        [e[0][0] * zeta[0] + e[0][1] * zeta[1], e[1][0] * zeta[0] + e[1][1] * zeta[1]]
    }
}

/// Eigenvalues and eigenvectors (columns of E) of a 2×2 complex matrix, and
/// E⁻¹. Diagonal, or too close to a defective matrix: the identity.
fn eig2(m: [[C64; 2]; 2]) -> ([C64; 2], [[C64; 2]; 2], [[C64; 2]; 2]) {
    let id = [[C64::ONE, C64::ZERO], [C64::ZERO, C64::ONE]];
    let diag = ([m[0][0], m[1][1]], id, id);
    let size = m[0][0].norm() + m[1][1].norm() + m[0][1].norm() + m[1][0].norm();
    if m[0][1].norm() <= 1e-12 * size && m[1][0].norm() <= 1e-12 * size {
        return diag;
    }
    let half = (m[0][0] - m[1][1]).scale(0.5);
    let s = (half * half + m[0][1] * m[1][0]).sqrt();
    let mean = (m[0][0] + m[1][1]).scale(0.5);
    let lam = [mean + s, mean - s];
    let vec = |l: C64| {
        let v = if m[0][1].norm() >= m[1][0].norm() { (m[0][1], l - m[0][0]) } else { (l - m[1][1], m[1][0]) };
        let n = (v.0.norm_sqr() + v.1.norm_sqr()).sqrt();
        (v.0.scale(1.0 / n), v.1.scale(1.0 / n))
    };
    let (v0, v1) = (vec(lam[0]), vec(lam[1]));
    let e = [[v0.0, v1.0], [v0.1, v1.1]];
    let det = e[0][0] * e[1][1] - e[0][1] * e[1][0];
    if det.norm() < 1e-9 {
        return diag;
    }
    let inv = [[e[1][1] / det, -e[0][1] / det], [-e[1][0] / det, e[0][0] / det]];
    (lam, e, inv)
}

/// One string at one length, ready for [`piano_core::modal::ModalBank::load`].
pub struct StringTable {
    /// Two coupled modes per partial, index 2n + k.
    pub modes: ModeTable,
    pub partials: Vec<Partial>,
    /// Ideal pluck: ζ of each partial per metre of displacement.
    pub shape: Vec<f64>,
    pub f0: f64,
    pub b: f64,
    pub length: f64,
    pub tension: f64,
    /// Modal mass μL/2 (kg).
    pub modal_mass: f64,
    /// Per partial: weight of each polarisation in the instrument's
    /// output, on displacement, or on velocity when `velocity_out` (the
    /// pickups' EMF); output gain included.
    pub win: Vec<[f64; 2]>,
    pub velocity_out: bool,
    /// Built to run muted ([`Self::muted`]).
    pub muted: bool,
    /// Frets under the vibrating part, with their gaps, and the slope of the
    /// rest line from the saddle (set by the instrument: the geometry is the
    /// fingerboard's).
    pub frets: Vec<crate::frets::FretGap>,
    pub rest_slope: f64,
    /// Per partial: perpendicular bridge force per metre (N/m), T·nπ/L.
    pub force: Vec<f64>,
    /// What a tension change needs: the string, the bridge, the density,
    /// the highest frequency a mode may take.
    pub params: StringParams,
    pub support: SupportParams,
    pub mu: f64,
    pub f_cap: f64,
}

/// A partial of the string at another tension: frequency, own decay, the
/// coupled modes' shifts, each polarisation alone.
#[derive(Clone, Copy, Debug, Default)]
pub struct Bent {
    pub omega: f64,
    pub sigma: f64,
    pub shift: [C64; 2],
    pub polar: [(f64, f64); 2],
}

/// Output weight of a partial of wavenumber `k` per polarisation, and
/// whether it applies to the velocity.
fn output_weight(model: &GuitarModel, tension: f64, k: f64) -> ([f64; 2], bool) {
    let g = model.output_gain;
    match &model.output {
        Output::BridgeForce { gain } => ([tension * k * gain * g, 0.0], false),
        Output::Piezo { sensitivity, .. } => ([tension * k * sensitivity * g, 0.0], false),
        Output::Magnetic(m) => {
            let window = |x: f64, a: f64| (k * x).sin() * sinc(k * a / 2.0);
            let picked: Vec<_> = m.selected.iter().filter_map(|&i| m.pickups.get(i)).collect();
            let emf: f64 = picked
                .iter()
                .map(|p| {
                    p.sensitivity
                        * match p.kind {
                            PickupKind::SingleCoil => window(p.position, p.aperture),
                            PickupKind::Humbucker => {
                                let h = p.coil_spacing / 2.0;
                                (window(p.position - h, p.aperture) + window(p.position + h, p.aperture)) / 2.0
                            }
                        }
                })
                .sum::<f64>()
                / picked.len().max(1) as f64;
            // The poles see motion toward them: the perpendicular direction.
            ([emf * g, 0.0], true)
        }
    }
}

impl StringTable {
    /// `string` stopped at `length`, ringing freely; a fingertip centred
    /// `mute_at` from the bridge would mute it (damper scale 1).
    pub fn new(model: &GuitarModel, string: usize, length: f64, mute_at: f64, sample_rate: f64) -> Self {
        Self::build(model, string, length, mute_at, sample_rate, false)
    }

    /// The same, to be run muted (the bank at damper scale 1): a velocity
    /// output (the pickups' EMF) then follows the extra decay. The bank's
    /// output weights cannot change while it plays, so a muted string has
    /// its own table.
    pub fn muted(model: &GuitarModel, string: usize, length: f64, mute_at: f64, sample_rate: f64) -> Self {
        Self::build(model, string, length, mute_at, sample_rate, true)
    }

    fn build(model: &GuitarModel, string: usize, length: f64, mute_at: f64, sample_rate: f64, muted: bool) -> Self {
        use std::f64::consts::PI;
        let s = &model.strings[string];
        let mu = linear_density(s, model.scale_length);
        let z0 = (s.tension * mu).sqrt();
        let f0 = (s.tension / mu).sqrt() / (2.0 * length);
        let b = inharmonicity(s, length);
        let f_max = model.max_freq.min(0.45 * sample_rate);
        // Plucking beyond the fret is impossible; keep the corner on the string.
        let x0 = model.pluck.position.min(0.9 * length);
        let (mw, r) = (model.mute.width.max(1e-6), model.mute.resistance);
        let sup = &model.support;
        let mut specs = Vec::new();
        let mut partials = Vec::new();
        let mut shape = Vec::new();
        let (mut win, mut force) = (Vec::new(), Vec::new());
        let mut velocity_out = false;
        for n in 1.. {
            let nf = n as f64;
            let f = nf * f0 * (1.0 + b * nf * nf).sqrt();
            if f > f_max {
                break;
            }
            let omega = 2.0 * PI * f;
            let bend = b * nf * nf / (1.0 + b * nf * nf);
            let sigma = loss_factor(s, mu, f, bend) * PI * f;
            let k = nf * PI / length;
            let rounding = sinc(k * model.pluck.width / 2.0);
            shape.push(2.0 * length * length * (k * x0).sin() / (nf * nf * PI * PI * x0 * (length - x0)) * rounding);
            // ⟨sin²(kx)⟩ over the fingertip.
            let mean_sin2 = 0.5 - (2.0 * k * mute_at).cos() * sinc(k * mw) / 2.0;
            let damp_w = r * mean_sin2 / (mu * length);
            let scale = C64::new(0.0, omega / (nf * PI) * z0);
            let y = [
                [admittance(&sup.perpendicular, f), admittance(&sup.cross, f)],
                [admittance(&sup.cross, f), admittance(&sup.parallel, f)],
            ];
            let m = [[scale * y[0][0], scale * y[0][1]], [scale * y[1][0], scale * y[1][1]]];
            let (shift, e, e_inv) = eig2(m);
            let polar = [(omega + m[0][0].re, sigma + m[0][0].im), (omega + m[1][1].re, sigma + m[1][1].im)];
            partials.push(Partial { k, omega, sigma, damp_w, polar, shift, e, e_inv });
            let (wp, vel) = output_weight(model, s.tension, k);
            velocity_out = vel;
            win.push(wp);
            force.push(s.tension * k);
            for kk in 0..2 {
                let (om, sg) = (omega + shift[kk].re, sigma + shift[kk].im);
                // Velocity of a mode z·e^{(−σ+iω)t}: (−σ + iω)·z, σ with the
                // mute when the table runs muted.
                let d = if vel { C64::new(-(sg + if muted { damp_w } else { 0.0 }), om) } else { C64::ONE };
                let w = (e[PERP][kk].scale(wp[PERP]) + e[PAR][kk].scale(wp[PAR])) * d;
                let w2 = e[PERP][kk].scale(s.tension * k);
                specs.push(ModeSpec {
                    freq_hz: om / (2.0 * PI),
                    sigma: sg,
                    out_re: w.re,
                    out_im: w.im,
                    out2_re: w2.re,
                    out2_im: w2.im,
                    damp_w,
                    ..Default::default()
                });
            }
        }
        Self {
            modes: ModeTable::new(sample_rate, &specs),
            partials,
            shape,
            f0,
            b,
            length,
            tension: s.tension,
            modal_mass: mu * length / 2.0,
            win,
            velocity_out,
            muted,
            force,
            frets: Vec::new(),
            rest_slope: 0.0,
            params: *s,
            support: *sup,
            mu,
            f_cap: 0.48 * sample_rate,
        }
    }

    pub fn len(&self) -> usize {
        self.partials.len()
    }

    /// Tension ratio T/T₀ that sets the fundamental's frequency ratio to
    /// `r`: (f/f₀)² = ρ·(1 + B₀/ρ)/(1 + B₀), B falling as 1/T.
    pub fn ratio_for_pitch(&self, r: f64) -> f64 {
        r * r * (1.0 + self.b) - self.b
    }

    /// Partial `n` at the table's own tension.
    pub fn unbent(&self, n: usize) -> Bent {
        let p = &self.partials[n];
        Bent { omega: p.omega, sigma: p.sigma, shift: p.shift, polar: p.polar }
    }

    /// Partial `n` at tension ratio ρ = T/T₀: frequency (stiffness falls as
    /// 1/T), the string's own losses at the new frequency, the bridge's
    /// shifts with the new Z₀ and Y(f). The mode shapes, and the coupled
    /// modes' eigenvectors (first order: λ_k = (E⁻¹·M·E)_kk), are kept, so
    /// the bank's state carries over unchanged.
    pub fn bent(&self, n: usize, rho: f64) -> Bent {
        use std::f64::consts::PI;
        let rho = rho.max(1e-3);
        let p = &self.partials[n];
        let nf = (n + 1) as f64;
        let b = self.b / rho;
        let f = nf * self.f0 * rho.sqrt() * (1.0 + b * nf * nf).sqrt();
        let omega = 2.0 * PI * f;
        let bend = b * nf * nf / (1.0 + b * nf * nf);
        let sigma = loss_factor(&self.params, self.mu, f, bend) * PI * f;
        let z0 = (rho * self.tension * self.mu).sqrt();
        let scale = C64::new(0.0, omega / (nf * PI) * z0);
        let sup = &self.support;
        let y = [[admittance(&sup.perpendicular, f), admittance(&sup.cross, f)], [admittance(&sup.cross, f), admittance(&sup.parallel, f)]];
        let m = [[scale * y[0][0], scale * y[0][1]], [scale * y[1][0], scale * y[1][1]]];
        let (e, ei) = (&p.e, &p.e_inv);
        let shift = [0, 1].map(|k| {
            // (E⁻¹·M·E)_kk
            let me = [m[0][0] * e[0][k] + m[0][1] * e[1][k], m[1][0] * e[0][k] + m[1][1] * e[1][k]];
            ei[k][0] * me[0] + ei[k][1] * me[1]
        });
        let polar = [(omega + m[0][0].re, sigma + m[0][0].im), (omega + m[1][1].re, sigma + m[1][1].im)];
        Bent { omega, sigma, shift, polar }
    }

    /// The bank's modes for the partials `bent` at tension ratio `rho`
    /// (output weights follow: velocity, bridge force ∝ T). Modes pushed
    /// past `f_cap` are silenced rather than aliased.
    pub fn bent_specs<'a>(&'a self, bent: &'a [Bent], rho: f64) -> impl Iterator<Item = ModeSpec> + 'a {
        use std::f64::consts::PI;
        self.partials.iter().zip(bent).enumerate().flat_map(move |(n, (p, b))| {
            [0, 1].map(|kk| {
                let (om, sg) = (b.omega + b.shift[kk].re, b.sigma + b.shift[kk].im);
                let audible = om / (2.0 * PI) < self.f_cap;
                let d = if self.velocity_out { C64::new(-(sg + if self.muted { p.damp_w } else { 0.0 }), om) } else { C64::ONE };
                let w = if audible { (p.e[PERP][kk].scale(self.win[n][PERP]) + p.e[PAR][kk].scale(self.win[n][PAR])) * d } else { C64::ZERO };
                let w2 = if audible { p.e[PERP][kk].scale(rho * self.force[n]) } else { C64::ZERO };
                ModeSpec {
                    freq_hz: om.min(2.0 * PI * self.f_cap) / (2.0 * PI),
                    sigma: sg,
                    out_re: w.re,
                    out_im: w.im,
                    out2_re: w2.re,
                    out2_im: w2.im,
                    damp_w: p.damp_w,
                    ..Default::default()
                }
            })
        })
    }

    pub fn is_empty(&self) -> bool {
        self.partials.is_empty()
    }
}

pub fn sinc(x: f64) -> f64 {
    if x.abs() < 1e-9 { 1.0 } else { x.sin() / x }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::SupportParams;

    const SR: f64 = 48_000.0;

    fn table(m: &GuitarModel, s: usize, fret: u8) -> StringTable {
        let l = fret_length(m.scale_length, fret);
        StringTable::new(m, s, l, l - m.mute.distance, SR)
    }

    #[test]
    fn open_strings_in_tune() {
        let m = GuitarModel::default();
        for (i, s) in m.strings.iter().enumerate() {
            let t = table(&m, i, 0);
            let cents = 1200.0 * (t.f0 / midi_hz(s.open_note as f64)).log2();
            assert!(cents.abs() < 1e-6, "string {i}: {cents} cents");
        }
    }

    #[test]
    fn twelfth_fret_is_the_octave() {
        let m = GuitarModel::default();
        let open = table(&m, 0, 0);
        let oct = table(&m, 0, 12);
        assert!((oct.f0 / open.f0 - 2.0).abs() < 1e-12);
        // Shorter string, stiffer relative to its tension: B grows 4×.
        assert!((oct.b / open.b - 4.0).abs() < 1e-9);
    }

    #[test]
    fn inharmonicity_in_measured_range() {
        // Measured on steel-string guitars: B ≈ 1e-5 … 2e-4 (Järveläinen
        // & Karjalainen; Fletcher & Rossing ch. 9).
        let m = GuitarModel::default();
        for i in 0..m.strings.len() {
            let b = table(&m, i, 0).b;
            assert!((5e-6..3e-4).contains(&b), "string {i}: B = {b:e}");
        }
    }

    #[test]
    fn mid_pluck_has_no_even_partials() {
        let mut m = GuitarModel::default();
        m.pluck.position = m.scale_length / 2.0;
        m.pluck.width = 0.0;
        let t = table(&m, 0, 0);
        for (n, a) in t.shape.iter().enumerate().take(20) {
            if n % 2 == 1 {
                assert!(a.abs() < 1e-12 * t.shape[0].abs(), "partial {}: {a}", n + 1);
            }
        }
        // Released from h = 1: the fundamental carries 8/π² of it.
        assert!((t.shape[0] - 8.0 / std::f64::consts::PI.powi(2)).abs() < 1e-12);
    }

    #[test]
    fn resistive_bridge_decay_matches_gough() {
        // One polarisation, pure conductance G: σ = σ_string + Z₀·G·ω/(nπ),
        // i.e. 2f₀·Z₀·G on a harmonic string, whatever the partial.
        let mut m = GuitarModel::default();
        let g = 0.01;
        m.support = SupportParams::default();
        m.support.perpendicular = Admittance { conductance: g, ..Admittance::ZERO };
        m.support.parallel = Admittance::ZERO;
        m.support.cross = Admittance::ZERO;
        let t = table(&m, 5, 0);
        let mu = linear_density(&m.strings[5], m.scale_length);
        let z0 = (m.strings[5].tension * mu).sqrt();
        for (n, p) in t.partials.iter().enumerate().take(5) {
            let expect = z0 * g * p.omega / ((n + 1) as f64 * std::f64::consts::PI);
            assert!((p.polar[PERP].1 - p.sigma - expect).abs() < 1e-12, "partial {}", n + 1);
            assert_eq!(p.polar[PAR].1, p.sigma);
            // No cross term: the modes are the polarisations.
            assert!((p.e[0][1].norm() + p.e[1][0].norm()) < 1e-12);
        }
    }

    #[test]
    fn spring_like_bridge_lowers_the_partials() {
        let mut m = GuitarModel::default();
        m.support.perpendicular.susceptance = 0.05;
        let t = table(&m, 0, 0);
        assert!(t.partials.iter().all(|p| p.polar[PERP].0 < p.omega));
    }

    #[test]
    fn coupled_modes_are_passive_and_invertible() {
        let m = GuitarModel::default();
        for s in 0..m.strings.len() {
            for fret in [0, 5, 12, 20] {
                let t = table(&m, s, fret);
                for p in &t.partials {
                    for l in p.shift {
                        assert!(l.im >= 0.0, "string {s} fret {fret}: growing mode {l:?}");
                    }
                    let z = [C64::new(0.3, -0.2), C64::new(-0.1, 0.7)];
                    let back = p.coupled_of(p.polar_of(z));
                    assert!((back[0] - z[0]).norm() + (back[1] - z[1]).norm() < 1e-9);
                }
            }
        }
    }

    #[test]
    fn pickup_at_the_middle_hears_no_even_partials() {
        use crate::params::{MagneticOutput, Output, Pickup};
        let mut m = GuitarModel::electric_solid();
        let half = m.scale_length / 2.0;
        m.output = Output::Magnetic(MagneticOutput { pickups: vec![Pickup { aperture: 0.0, ..Pickup::single_coil(half) }], selected: vec![0], ..MagneticOutput::default() });
        let t = table(&m, 0, 0);
        assert!(t.velocity_out);
        for (n, w) in t.win.iter().enumerate().take(20) {
            if n % 2 == 1 {
                assert!(w[PERP].abs() < 1e-9 * t.win[0][PERP].abs(), "partial {}: {w:?}", n + 1);
            }
            assert_eq!(w[PAR], 0.0);
        }
    }

    #[test]
    fn bending_hits_its_pitch_and_unbent_is_the_table() {
        let m = GuitarModel::electric_solid();
        for s in [0, 5] {
            let t = table(&m, s, 7);
            // ρ = 1 reproduces the table's own partials.
            for n in [0, 3, 20] {
                let (b, u) = (t.bent(n, 1.0), t.unbent(n));
                assert!((b.omega - u.omega).abs() < 1e-9 * u.omega && (b.sigma - u.sigma).abs() < 1e-9 * u.sigma.max(1.0));
                for k in 0..2 {
                    assert!((b.shift[k] - u.shift[k]).norm() < 1e-9 * u.shift[k].norm().max(1e-12), "shift {k}: {:?} vs {:?}", b.shift[k], u.shift[k]);
                }
            }
            // A whole-tone bend: the fundamental two semitones up.
            let r = 2f64.powf(2.0 / 12.0);
            let rho = t.ratio_for_pitch(r);
            let f = t.bent(0, rho).omega / t.partials[0].omega;
            assert!((1200.0 * (f / r).log2()).abs() < 0.01, "string {s}: {} cents off", 1200.0 * (f / r).log2());
            // Tighter string: relatively less stiff, higher partials closer to harmonic.
            assert!(rho > 1.0);
        }
    }

    #[test]
    fn bent_modes_never_pass_nyquist() {
        let m = GuitarModel::electric_solid();
        let t = table(&m, 5, 12);
        let rho = t.ratio_for_pitch(2f64.powf(4.0 / 12.0));
        let bent: Vec<Bent> = (0..t.len()).map(|n| t.bent(n, rho)).collect();
        for sp in t.bent_specs(&bent, rho) {
            assert!(sp.freq_hz <= t.f_cap);
            if sp.freq_hz >= t.f_cap {
                assert_eq!((sp.out_re, sp.out_im), (0.0, 0.0));
            }
        }
    }
}
