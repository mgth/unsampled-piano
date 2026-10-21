//! Synthetic test signals with known ground truth, to validate the analysers.

#[derive(Clone, Copy, Debug)]
pub struct Partial {
    pub freq: f64,
    pub amp: f64,
    /// Decay rate (1/s) of the main component.
    pub sigma: f64,
    /// Optional slow second component (amplitude, decay rate): double decay.
    pub slow: Option<(f64, f64)>,
    pub phase: f64,
}

/// Sum of exponentially decaying sinusoids plus white noise of RMS `noise`.
pub fn render(partials: &[Partial], sr: f64, dur: f64, noise: f64, seed: u64) -> Vec<f32> {
    let n = (dur * sr) as usize;
    let mut out = vec![0.0f64; n];
    for p in partials {
        let w = std::f64::consts::TAU * p.freq / sr;
        let r1 = (-p.sigma / sr).exp();
        let mut a1 = p.amp;
        let (mut a2, r2) = match p.slow {
            Some((a, s)) => (a, (-s / sr).exp()),
            None => (0.0, 1.0),
        };
        for (i, o) in out.iter_mut().enumerate() {
            *o += (a1 + a2) * (w * i as f64 + p.phase).sin();
            a1 *= r1;
            a2 *= r2;
        }
    }
    let mut rng = Lcg(seed);
    out.iter().map(|&v| (v + noise * rng.gauss()) as f32).collect()
}

/// Small deterministic PRNG (no dependency, reproducible tests).
pub struct Lcg(pub u64);

impl Lcg {
    pub fn uniform(&mut self) -> f64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.0 >> 11) as f64 + 0.5) / (1u64 << 53) as f64
    }

    pub fn gauss(&mut self) -> f64 {
        let (u, v) = (self.uniform(), self.uniform());
        (-2.0 * u.ln()).sqrt() * (std::f64::consts::TAU * v).cos()
    }
}
