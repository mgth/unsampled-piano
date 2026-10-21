//! Zero-padded Hann spectrum with interpolated peak picking.

use realfft::RealFftPlanner;

pub struct Spectrum {
    pub sr: f64,
    pub bin_hz: f64,
    /// Magnitude in dB.
    pub db: Vec<f64>,
}

pub fn hann(n: usize) -> Vec<f64> {
    (0..n).map(|i| 0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / n as f64).cos()).collect()
}

impl Spectrum {
    /// Spectrum of `x[start..start+len]`, zero-padded by `pad` (power of two
    /// length at least `pad·len`).
    pub fn new(x: &[f32], sr: f64, start: usize, len: usize, pad: usize) -> Self {
        let seg = &x[start.min(x.len())..(start + len).min(x.len())];
        let n = (seg.len() * pad).next_power_of_two();
        let w = hann(seg.len());
        let mut buf = vec![0.0f64; n];
        for (i, (&s, &wi)) in seg.iter().zip(&w).enumerate() {
            buf[i] = s as f64 * wi;
        }
        let fft = RealFftPlanner::<f64>::new().plan_fft_forward(n);
        let mut spec = fft.make_output_vec();
        fft.process(&mut buf, &mut spec).unwrap();
        let db = spec.iter().map(|c| 20.0 * (c.norm() + 1e-300).log10()).collect();
        Self { sr, bin_hz: sr / n as f64, db }
    }

    /// Highest peak in [lo, hi] Hz, refined by parabolic interpolation on dB.
    /// Returns (frequency, level dB). Edges of the range are not peaks.
    pub fn peak_in(&self, lo: f64, hi: f64) -> Option<(f64, f64)> {
        let a = ((lo / self.bin_hz).ceil() as usize).max(1);
        let b = ((hi / self.bin_hz).floor() as usize).min(self.db.len() - 2);
        if a >= b {
            return None;
        }
        let (i, _) = (a..=b).map(|i| (i, self.db[i])).max_by(|x, y| x.1.total_cmp(&y.1))?;
        if i == a || i == b {
            return None;
        }
        let (l, c, r) = (self.db[i - 1], self.db[i], self.db[i + 1]);
        let d = 0.5 * (l - r) / (l - 2.0 * c + r);
        Some(((i as f64 + d) * self.bin_hz, c - 0.25 * (l - r) * d))
    }

    /// Median level in [lo, hi] Hz, a robust noise-floor estimate.
    pub fn median_in(&self, lo: f64, hi: f64) -> f64 {
        let a = (lo / self.bin_hz) as usize;
        let b = ((hi / self.bin_hz) as usize).min(self.db.len() - 1);
        let mut v: Vec<f64> = self.db[a..=b.max(a)].to_vec();
        v.sort_by(f64::total_cmp);
        v[v.len() / 2]
    }
}
