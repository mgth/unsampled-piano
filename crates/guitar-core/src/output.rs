//! The instrument's output stage, after the strings: the guitar's passive
//! circuit for magnetic pickups, a piezo's high-pass, or the bridge force
//! as it is. The strings hand it the pickups' EMF (V) or the force they
//! scale; no amplifier or effect here (the user's own chain follows).
//!
//! Magnetic pickups (after Lemme; Paiva et al. 2012): the coil is an EMF in
//! series with R + sL, its self-capacitance C across its terminals (node A).
//! The volume pot's track runs from A to ground, the jack (node B) on its
//! wiper: R_top from A to B, R_bot from B to ground. The tone pot and its
//! capacitor go from A to ground, and the cable capacitance and the input
//! it feeds from B to ground. Nodal analysis gives
//!
//! ```text
//! H = V_B/E = G_top·D_t / ([D_t + Z_s·(sC·D_t + sC_t + G_top·D_t)]·Y_B − Z_s·G_top²·D_t)
//! Z_s = R + sL,  D_t = 1 + s·R_t·C_t,  Y_B = G_top + G_bot + G_in + s·C_cab
//! ```
//!
//! a 1st-over-4th-order rational function: the coil's resonance (2–5 kHz,
//! damped by the pots and the load) and the tone control's low-pass. It is
//! discretised by the bilinear transform, prewarped at the resonance.
//! Pickups switched in parallel are taken as identical: their Thévenin
//! equivalent is the mean EMF (the string tables average them) behind
//! R/n, L/n, n·C. Pots follow an audio taper, 10 % at mid travel.

use crate::params::{MagneticOutput, Output};

/// Coefficients of a polynomial in s, lowest power first.
type Poly = Vec<f64>;

fn mul(a: &[f64], b: &[f64]) -> Poly {
    let mut r = vec![0.0; a.len() + b.len() - 1];
    for (i, x) in a.iter().enumerate() {
        for (j, y) in b.iter().enumerate() {
            r[i + j] += x * y;
        }
    }
    r
}

fn add(a: &[f64], b: &[f64]) -> Poly {
    (0..a.len().max(b.len())).map(|i| a.get(i).unwrap_or(&0.0) + b.get(i).unwrap_or(&0.0)).collect()
}

fn scale(a: &[f64], s: f64) -> Poly {
    a.iter().map(|x| x * s).collect()
}

/// Audio-taper pot: share of the track at knob position `v` (0 … 1).
fn taper(v: f64) -> f64 {
    (81f64.powf(v.clamp(0.0, 1.0)) - 1.0) / 80.0
}

/// N(s)/D(s) of the circuit (V at the jack per V of EMF), and the coil's
/// resonance estimate (Hz).
pub fn circuit(m: &MagneticOutput) -> (Poly, Poly, f64) {
    let picked: Vec<_> = m.selected.iter().filter_map(|&i| m.pickups.get(i)).collect();
    let n = picked.len().max(1) as f64;
    let mean = |f: fn(&crate::params::Pickup) -> f64| picked.iter().map(|p| f(p)).sum::<f64>() / n;
    let (l, r, c) = (mean(|p| p.inductance) / n, mean(|p| p.resistance) / n, mean(|p| p.capacitance) * n);
    let ci = &m.circuit;
    const R_MIN: f64 = 1.0;
    let g_top = 1.0 / ((1.0 - taper(m.volume)) * ci.volume_pot + R_MIN);
    let g_bot = 1.0 / (taper(m.volume) * ci.volume_pot + R_MIN);
    let g_in = 1.0 / ci.load_resistance;
    let r_t = taper(m.tone) * ci.tone_pot + R_MIN;
    let (ct, ccab) = (ci.tone_cap, ci.cable_capacitance);

    let zs = [r, l];
    let dt = [1.0, r_t * ct];
    let inner = add(&add(&mul(&[0.0, c], &dt), &[0.0, ct]), &scale(&dt, g_top));
    let a = add(&dt, &mul(&zs, &inner));
    let yb = [g_top + g_bot + g_in, ccab];
    let den = add(&mul(&a, &yb), &scale(&mul(&zs, &dt), -g_top * g_top));
    let num = scale(&dt, g_top);
    let f_res = 1.0 / (std::f64::consts::TAU * (l * (c + ccab)).sqrt());
    (num, den, f_res)
}

/// Bilinear transform s = k·(1 − z⁻¹)/(1 + z⁻¹) of N/D: coefficients in
/// z⁻¹, normalised so that a[0] = 1.
fn bilinear(num: &[f64], den: &[f64], k: f64) -> (Vec<f64>, Vec<f64>) {
    let order = num.len().max(den.len()) - 1;
    let map = |p: &[f64]| {
        let mut out = vec![0.0; order + 1];
        for (j, &cj) in p.iter().enumerate() {
            // (1 − z⁻¹)^j·(1 + z⁻¹)^(order − j)
            let mut t = vec![1.0];
            for _ in 0..j {
                t = mul(&t, &[1.0, -1.0]);
            }
            for _ in j..order {
                t = mul(&t, &[1.0, 1.0]);
            }
            for (o, x) in out.iter_mut().zip(&t) {
                *o += cj * k.powi(j as i32) * x;
            }
        }
        out
    };
    let (b, a) = (map(num), map(den));
    let a0 = a[0];
    (b.iter().map(|x| x / a0).collect(), a.iter().map(|x| x / a0).collect())
}

/// |N(jω)/D(jω)| of a polynomial ratio.
pub fn analog_gain(num: &[f64], den: &[f64], f: f64) -> f64 {
    let w = std::f64::consts::TAU * f;
    let eval = |p: &[f64]| {
        let (mut re, mut im) = (0.0, 0.0);
        for (i, c) in p.iter().enumerate() {
            // (jω)^i
            let m = c * w.powi(i as i32);
            match i % 4 {
                0 => re += m,
                1 => im += m,
                2 => re -= m,
                _ => im -= m,
            }
        }
        re.hypot(im)
    };
    eval(num) / eval(den)
}

const MAX_ORDER: usize = 4;

pub struct OutputStage {
    b: [f64; MAX_ORDER + 1],
    a: [f64; MAX_ORDER + 1],
    order: usize,
    z: [f64; MAX_ORDER],
    sample_rate: f64,
}

impl OutputStage {
    pub fn new(output: &Output, sample_rate: f64) -> Self {
        let (num, den, k) = match output {
            Output::BridgeForce { .. } => (vec![1.0], vec![1.0], 1.0),
            Output::Piezo { highpass_hz, .. } => (vec![0.0, 1.0], vec![std::f64::consts::TAU * highpass_hz, 1.0], 2.0 * sample_rate),
            Output::Magnetic(m) => {
                let (num, den, f_res) = circuit(m);
                let w = std::f64::consts::TAU * f_res.min(0.4 * sample_rate);
                (num, den, w / (w / (2.0 * sample_rate)).tan())
            }
        };
        let (bv, av) = bilinear(&num, &den, k);
        let mut b = [0.0; MAX_ORDER + 1];
        let mut a = [0.0; MAX_ORDER + 1];
        b[..bv.len()].copy_from_slice(&bv);
        a[..av.len()].copy_from_slice(&av);
        Self { b, a, order: av.len() - 1, z: [0.0; MAX_ORDER], sample_rate }
    }

    /// Filters `buf` in place (transposed direct form II, f64). Real-time safe.
    pub fn process(&mut self, buf: &mut [f32]) {
        let (b, a, n) = (&self.b, &self.a, self.order);
        if n == 0 {
            for x in buf.iter_mut() {
                *x = (*x as f64 * b[0]) as f32;
            }
            return;
        }
        for x in buf.iter_mut() {
            let xi = *x as f64;
            let y = b[0] * xi + self.z[0];
            for i in 0..n - 1 {
                self.z[i] = b[i + 1] * xi - a[i + 1] * y + self.z[i + 1];
            }
            self.z[n - 1] = b[n] * xi - a[n] * y;
            *x = y as f32;
        }
    }

    pub fn reset(&mut self) {
        self.z = [0.0; MAX_ORDER];
    }

    /// Gain of the digital filter at `f` (Hz).
    pub fn gain(&self, f: f64) -> f64 {
        let w = std::f64::consts::TAU * f / self.sample_rate;
        let eval = |c: &[f64]| {
            let (mut re, mut im) = (0.0, 0.0);
            for (i, x) in c.iter().enumerate() {
                re += x * (w * i as f64).cos();
                im -= x * (w * i as f64).sin();
            }
            re.hypot(im)
        };
        eval(&self.b[..=self.order]) / eval(&self.a[..=self.order])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::{Pickup, PickupKind};

    const SR: f64 = 48_000.0;

    fn single() -> MagneticOutput {
        MagneticOutput { pickups: vec![Pickup::single_coil(0.1)], selected: vec![0], ..MagneticOutput::default() }
    }

    fn peak(gain: impl Fn(f64) -> f64) -> (f64, f64) {
        (200..12_000).step_by(10).map(|f| (f as f64, gain(f as f64))).fold((0.0, 0.0), |m, x| if x.1 > m.1 { x } else { m })
    }

    fn db(x: f64) -> f64 {
        20.0 * x.log10()
    }

    #[test]
    fn single_coil_resonates_where_its_lc_says() {
        let m = single();
        let (num, den, f_res) = circuit(&m);
        let (f_peak, g_peak) = peak(|f| analog_gain(&num, &den, f));
        // L = 2.5 H with 100 + 500 pF: ≈ 4.1 kHz, pulled down a little by
        // the tone capacitor through its pot.
        assert!((f_peak / f_res - 1.0).abs() < 0.15, "peak {f_peak} Hz, LC {f_res} Hz");
        assert!(db(g_peak) > 3.0, "resonance only {} dB", db(g_peak));
        // Low frequencies pass (coil resistance against the pots and load).
        assert!(db(analog_gain(&num, &den, 100.0)).abs() < 1.0);
        // The digital filter keeps the peak where it is.
        let st = OutputStage::new(&Output::Magnetic(m), SR);
        let (fd, gd) = peak(|f| st.gain(f));
        assert!((fd / f_peak - 1.0).abs() < 0.02 && (db(gd) - db(g_peak)).abs() < 0.5, "digital {fd} Hz {gd}, analog {f_peak} Hz {g_peak}");
    }

    #[test]
    fn humbucker_resonates_lower() {
        let hb = MagneticOutput { pickups: vec![Pickup::humbucker(0.1)], ..single() };
        assert_eq!(hb.pickups[0].kind, PickupKind::Humbucker);
        let (n1, d1, _) = circuit(&single());
        let (n2, d2, _) = circuit(&hb);
        let (f1, _) = peak(|f| analog_gain(&n1, &d1, f));
        let (f2, _) = peak(|f| analog_gain(&n2, &d2, f));
        assert!(f2 < 0.8 * f1, "humbucker {f2} Hz vs single coil {f1} Hz");
    }

    #[test]
    fn tone_and_volume_knobs() {
        let open = single();
        let dark = MagneticOutput { tone: 0.0, ..single() };
        let off = MagneticOutput { volume: 0.0, ..single() };
        let g = |m: &MagneticOutput, f| {
            let (n, d, _) = circuit(m);
            analog_gain(&n, &d, f)
        };
        assert!(db(g(&open, 4000.0)) - db(g(&dark, 4000.0)) > 12.0);
        assert!(db(g(&off, 1000.0)) < -60.0);
        // Halfway down, an audio taper leaves ≈ −20 dB (10 % of the track).
        let half = MagneticOutput { volume: 0.5, ..single() };
        let d = db(g(&half, 300.0));
        assert!((-24.0..-14.0).contains(&d), "volume at 0.5: {d} dB");
    }

    #[test]
    fn piezo_highpass() {
        let st = OutputStage::new(&Output::Piezo { sensitivity: 0.05, highpass_hz: 20.0 }, SR);
        assert!((db(st.gain(20.0)) + 3.0).abs() < 0.2);
        assert!(db(st.gain(1000.0)).abs() < 0.01);
    }

    #[test]
    fn filter_is_stable_on_an_impulse() {
        let mut st = OutputStage::new(&Output::Magnetic(MagneticOutput { tone: 0.0, ..single() }), SR);
        let mut buf = vec![0.0f32; 48_000];
        buf[0] = 1.0;
        st.process(&mut buf);
        assert!(buf.iter().all(|v| v.is_finite()));
        assert!(buf[40_000..].iter().all(|v| v.abs() < 1e-6));
    }
}
