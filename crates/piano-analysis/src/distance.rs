//! Multi-resolution STFT distance between two signals (spectral convergence
//! + log-magnitude L1), averaged over FFT sizes 512–4096.

use crate::spectrum::hann;
use realfft::RealFftPlanner;

fn stft_mag(x: &[f32], n: usize) -> Vec<Vec<f64>> {
    let hop = n / 4;
    let w = hann(n);
    let fft = RealFftPlanner::<f64>::new().plan_fft_forward(n);
    let mut buf = vec![0.0; n];
    let mut spec = fft.make_output_vec();
    let mut frames = Vec::new();
    let mut s = 0;
    while s + n <= x.len() {
        for i in 0..n {
            buf[i] = x[s + i] as f64 * w[i];
        }
        fft.process(&mut buf, &mut spec).unwrap();
        frames.push(spec.iter().map(|c| c.norm()).collect());
        s += hop;
    }
    frames
}

#[derive(Clone, Copy, Debug)]
pub struct MrStft {
    pub spectral_convergence: f64,
    pub log_mag_l1: f64,
}

pub fn mrstft(a: &[f32], b: &[f32]) -> MrStft {
    let len = a.len().min(b.len());
    let sizes = [512usize, 1024, 2048, 4096];
    let (mut sc, mut lm) = (0.0, 0.0);
    for &n in &sizes {
        let (sa, sb) = (stft_mag(&a[..len], n), stft_mag(&b[..len], n));
        let (mut num, mut den, mut l1, mut cnt) = (0.0, 0.0, 0.0, 0usize);
        for (fa, fb) in sa.iter().zip(&sb) {
            for (&x, &y) in fa.iter().zip(fb) {
                num += (x - y).powi(2);
                den += x * x;
                l1 += ((x + 1e-7).ln() - (y + 1e-7).ln()).abs();
                cnt += 1;
            }
        }
        sc += (num / den.max(1e-30)).sqrt();
        lm += l1 / cnt.max(1) as f64;
    }
    MrStft { spectral_convergence: sc / sizes.len() as f64, log_mag_l1: lm / sizes.len() as f64 }
}

/// Spectral centroid (Hz) of `x[start..start+len]`.
pub fn centroid(x: &[f32], sr: f64, start: usize, len: usize) -> f64 {
    let s = crate::spectrum::Spectrum::new(x, sr, start, len, 1);
    let (mut num, mut den) = (0.0, 0.0);
    for (i, d) in s.db.iter().enumerate() {
        let m = 10f64.powf(d / 20.0);
        num += i as f64 * s.bin_hz * m;
        den += m;
    }
    num / den.max(1e-30)
}
