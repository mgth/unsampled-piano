//! Modes of one string stopped at a given length, on a rigid bridge.
//!
//! - Frequencies: stiff string, f_n = n·f₀·√(1 + B·n²) with
//!   B = π³·E·d⁴/(64·T·L²) (Fletcher 1964), d the core diameter.
//! - Losses (Valette & Cuesta 1993): loss factor η_n = air + viscoelastic +
//!   internal + bridge, σ_n = η_n·ω_n/2. Air: R/(μω) with
//!   R = 2πη_a + 2πd·√(πη_a·ρ_a·f) per unit length (viscous drag on a
//!   cylinder). Viscoelastic: δ_ve times the bending share of the mode's
//!   energy, B·n²/(1 + B·n²). The bridge term is a constant until the body
//!   gives the real admittance.
//! - Pluck: the string held in a triangle of height h at x₀ from the bridge
//!   and released from rest, y(x, t) = Σ a_n·sin(nπx/L)·e^{−σt}·cos(ω_n t),
//!   a_n = 2hL²·sin(nπx₀/L)/(n²π²·x₀(L − x₀)), the corner rounded over the
//!   contact width w by sinc(nπw/2L).
//! - Output: the transverse force on the bridge, F = T·∂y/∂x at x = 0,
//!   weight T·nπ/L per mode.
//! - Mute: a dashpot R at distance d from the far end adds
//!   R·sin²(nπd/L)/(μL) to σ_n (modal mass μL/2), the bank's damper weight.

use piano_core::modal::{ModeSpec, ModeTable};

use crate::params::{GuitarModel, StringParams};

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

/// Loss factor of a mode at frequency `f` with bending share `bend`.
fn loss_factor(s: &StringParams, mu: f64, f: f64, bend: f64, eta_bridge: f64) -> f64 {
    use std::f64::consts::PI;
    let r = 2.0 * PI * AIR_VISCOSITY + 2.0 * PI * s.diameter * (PI * AIR_VISCOSITY * AIR_DENSITY * f).sqrt();
    r / (mu * 2.0 * PI * f) + s.delta_ve * bend + s.eta_int + eta_bridge
}

/// One string at one fret, ready for [`piano_core::modal::ModalBank::load`].
pub struct StringTable {
    pub modes: ModeTable,
    /// Initial state of each mode per metre of pluck displacement.
    pub shape: Vec<f64>,
    pub f0: f64,
    pub b: f64,
    pub length: f64,
}

impl StringTable {
    pub fn new(model: &GuitarModel, string: usize, fret: u8, sample_rate: f64) -> Self {
        use std::f64::consts::PI;
        let s = &model.strings[string];
        let mu = linear_density(s, model.scale_length);
        let length = model.scale_length * 2f64.powf(-(fret as f64) / 12.0);
        let f0 = (s.tension / mu).sqrt() / (2.0 * length);
        let b = inharmonicity(s, length);
        let f_max = model.max_freq.min(0.45 * sample_rate);
        // Plucking beyond the fret is impossible; keep the corner on the string.
        let x0 = model.pluck.position.min(0.9 * length);
        let (d, r) = (model.mute.distance, model.mute.resistance);
        let mut specs = Vec::new();
        let mut shape = Vec::new();
        for n in 1.. {
            let nf = n as f64;
            let f = nf * f0 * (1.0 + b * nf * nf).sqrt();
            if f > f_max {
                break;
            }
            let bend = b * nf * nf / (1.0 + b * nf * nf);
            let sigma = loss_factor(s, mu, f, bend, model.eta_bridge) * PI * f;
            let k = nf * PI / length;
            let rounding = sinc(k * model.pluck.width / 2.0);
            shape.push(2.0 * length * length * (k * x0).sin() / (nf * nf * PI * PI * x0 * (length - x0)) * rounding);
            specs.push(ModeSpec {
                freq_hz: f,
                sigma,
                out_re: s.tension * k * model.output_gain,
                damp_w: r * (k * d).sin().powi(2) / (mu * length),
                ..Default::default()
            });
        }
        Self { modes: ModeTable::new(sample_rate, &specs), shape, f0, b, length }
    }
}

fn sinc(x: f64) -> f64 {
    if x.abs() < 1e-9 { 1.0 } else { x.sin() / x }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_strings_in_tune() {
        let m = GuitarModel::default();
        for (i, s) in m.strings.iter().enumerate() {
            let t = StringTable::new(&m, i, 0, 48_000.0);
            let cents = 1200.0 * (t.f0 / midi_hz(s.open_note as f64)).log2();
            assert!(cents.abs() < 1e-6, "string {i}: {cents} cents");
        }
    }

    #[test]
    fn twelfth_fret_is_the_octave() {
        let m = GuitarModel::default();
        let open = StringTable::new(&m, 0, 0, 48_000.0);
        let oct = StringTable::new(&m, 0, 12, 48_000.0);
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
            let b = StringTable::new(&m, i, 0, 48_000.0).b;
            assert!((5e-6..3e-4).contains(&b), "string {i}: B = {b:e}");
        }
    }

    #[test]
    fn mid_pluck_has_no_even_partials() {
        let mut m = GuitarModel::default();
        m.pluck.position = m.scale_length / 2.0;
        m.pluck.width = 0.0;
        let t = StringTable::new(&m, 0, 0, 48_000.0);
        for (n, a) in t.shape.iter().enumerate().take(20) {
            if n % 2 == 1 {
                assert!(a.abs() < 1e-12 * t.shape[0].abs(), "partial {}: {a}", n + 1);
            }
        }
        // Released from h = 1: the fundamental carries 8/π² of it.
        assert!((t.shape[0] - 8.0 / std::f64::consts::PI.powi(2)).abs() < 1e-12);
    }
}
