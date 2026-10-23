//! Integer upsampling by 2, 4 or 8: a cascade of half-band interpolators.
//! It lets the engine run at 44.1/48 kHz behind an output at 88.2–384 kHz,
//! where the model has nothing to add above 20 kHz and would cost twice to
//! eight times as much. Kaiser-windowed, 100 dB down from the first image;
//! the band up to 20 kHz is kept flat. No allocation after `new`.

use core::f64::consts::PI;

/// Audio band kept flat (Hz).
const PASS_HZ: f64 = 20_000.0;
/// Rejection of the images (dB).
const ATTENUATION_DB: f64 = 100.0;

/// One 2× stage. With N = 4K − 1 taps centred on an odd index, the odd
/// output phase is the input delayed (the centre tap); the even phase is a
/// symmetric 2K-tap filter.
struct Halfband {
    /// Even-phase taps (×2 for the zero stuffing), oldest input first.
    taps: Vec<f32>,
    /// The last 2K − 1 inputs, then the block being processed.
    hist: Vec<f32>,
}

impl Halfband {
    /// A stage from `rate_in` to twice that, for blocks up to `max_in`.
    fn new(rate_in: f64, max_in: usize) -> Self {
        let rate_out = 2.0 * rate_in;
        let pass = PASS_HZ.min(0.45 * rate_in);
        // Transition from the band's edge to its first image.
        let width = (rate_in - 2.0 * pass) / rate_out;
        let n_est = (ATTENUATION_DB - 7.95) / (14.36 * width);
        let k = ((n_est + 1.0) / 4.0).ceil().max(2.0) as usize;
        let n = 4 * k - 1;
        let c = (n - 1) / 2;
        let beta = 0.1102 * (ATTENUATION_DB - 8.7);
        let i0b = bessel_i0(beta);
        let tap = |i: usize| {
            let x = i as f64 - c as f64;
            let sinc = if x == 0.0 {
                1.0
            } else {
                (PI * x / 2.0).sin() / (PI * x / 2.0)
            };
            let r = x / c as f64;
            0.5 * sinc * bessel_i0(beta * (1.0 - r * r).max(0.0).sqrt()) / i0b
        };
        // Even indices of the full filter; their sum is normalised to 1 (the
        // ×2 of the zero stuffing included) so the two phases match at DC.
        let mut taps: Vec<f64> = (0..2 * k).map(|q| tap(2 * q)).collect();
        let sum: f64 = taps.iter().sum();
        taps.iter_mut().for_each(|t| *t /= sum);
        // y[2m] = Σ taps[q]·x[m − q]: reversed so that it reads forwards.
        taps.reverse();
        Self {
            taps: taps.into_iter().map(|t| t as f32).collect(),
            hist: vec![0.0; 2 * k - 1 + max_in],
        }
    }

    /// Output samples by which the stage delays its input.
    fn latency(&self) -> usize {
        // Centre of the full filter (2K − 1 output samples).
        self.taps.len() - 1
    }

    /// `out` has twice the length of `input`.
    fn process(&mut self, input: &[f32], out: &mut [f32]) {
        let len = self.taps.len();
        let h = len - 1;
        self.hist[h..h + input.len()].copy_from_slice(input);
        for (m, o) in out.as_chunks_mut::<2>().0.iter_mut().enumerate() {
            let window = &self.hist[m..m + len];
            let mut acc = 0.0f32;
            for (t, x) in self.taps.iter().zip(window) {
                acc += t * x;
            }
            o[0] = acc;
            // The centre tap: the input K samples back.
            o[1] = self.hist[m + len / 2];
        }
        self.hist.copy_within(input.len()..input.len() + h, 0);
    }
}

/// Upsampler by a power of two (1: a copy).
pub struct Upsampler {
    stages: Vec<Halfband>,
    /// Between stages.
    scratch: [Vec<f32>; 2],
    factor: usize,
}

impl Upsampler {
    /// From `rate_in` by `factor` (1, 2, 4 or 8), for blocks of up to
    /// `max_in` input samples.
    pub fn new(rate_in: f64, factor: usize, max_in: usize) -> Self {
        assert!(
            matches!(factor, 1 | 2 | 4 | 8),
            "upsampling factor {factor}"
        );
        let stages: Vec<Halfband> = (0..factor.trailing_zeros())
            .map(|s| Halfband::new(rate_in * (1 << s) as f64, max_in << s))
            .collect();
        let scratch = [
            vec![0.0; max_in * factor / 2],
            vec![0.0; max_in * factor / 2],
        ];
        Self {
            stages,
            scratch,
            factor,
        }
    }

    pub fn factor(&self) -> usize {
        self.factor
    }

    /// Output samples by which the sound is delayed.
    pub fn latency(&self) -> usize {
        let n = self.stages.len();
        self.stages
            .iter()
            .enumerate()
            .map(|(s, st)| st.latency() << (n - 1 - s))
            .sum()
    }

    /// `out` has `factor` times the length of `input`.
    pub fn process(&mut self, input: &[f32], out: &mut [f32]) {
        debug_assert_eq!(out.len(), input.len() * self.factor);
        let n = self.stages.len();
        if n == 0 {
            out.copy_from_slice(input);
            return;
        }
        let mut len = input.len();
        for (s, stage) in self.stages.iter_mut().enumerate() {
            // Each stage reads a and writes b, then they swap; the last one
            // writes `out`.
            let [a, b] = &mut self.scratch;
            let src: &[f32] = if s == 0 { input } else { &a[..len] };
            if s == n - 1 {
                stage.process(src, &mut out[..2 * len]);
            } else {
                stage.process(src, &mut b[..2 * len]);
                core::mem::swap(a, b);
            }
            len *= 2;
        }
    }
}

/// Modified Bessel function of the first kind, order 0 (Kaiser window).
fn bessel_i0(x: f64) -> f64 {
    let (mut sum, mut term, q) = (1.0, 1.0, x * x / 4.0);
    for k in 1..50 {
        term *= q / (k * k) as f64;
        sum += term;
        if term < sum * 1e-17 {
            break;
        }
    }
    sum
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Amplitude of `f` Hz in `x` at `rate` (single-bin DFT, Hann window).
    fn level(x: &[f32], f: f64, rate: f64) -> f64 {
        let n = x.len();
        let (mut re, mut im, mut wsum) = (0.0, 0.0, 0.0);
        for (i, &v) in x.iter().enumerate() {
            let w = 0.5 - 0.5 * (2.0 * PI * i as f64 / n as f64).cos();
            let ph = 2.0 * PI * f * i as f64 / rate;
            re += w * v as f64 * ph.cos();
            im += w * v as f64 * ph.sin();
            wsum += w;
        }
        2.0 * (re * re + im * im).sqrt() / wsum
    }

    fn run(factor: usize, f: f64) -> (Vec<f32>, f64) {
        let rate = 48_000.0;
        let input: Vec<f32> = (0..9600)
            .map(|i| (2.0 * PI * f * i as f64 / rate).sin() as f32)
            .collect();
        let mut up = Upsampler::new(rate, factor, 512);
        let mut out = vec![0.0; input.len() * factor];
        // In uneven blocks, as a host would call.
        let mut i = 0;
        for size in [512, 37, 1, 300].iter().cycle() {
            if i >= input.len() {
                break;
            }
            let n = (*size).min(input.len() - i);
            up.process(&input[i..i + n], &mut out[i * factor..(i + n) * factor]);
            i += n;
        }
        (out, rate * factor as f64)
    }

    #[test]
    fn keeps_the_band_and_removes_the_images() {
        for factor in [2, 4, 8] {
            for f in [100.0, 1000.0, 10_000.0, 19_000.0, 20_000.0] {
                let (out, rate) = run(factor, f);
                let settled = &out[out.len() / 4..];
                let pass = 20.0 * level(settled, f, rate).log10();
                assert!(pass.abs() < 0.01, "×{factor} {f} Hz: {pass:.4} dB");
                // The images of the first stage (48 kHz ± f) and beyond.
                for image in [48_000.0 - f, 48_000.0 + f, 96_000.0 - f] {
                    if image < rate / 2.0 {
                        let db = 20.0 * level(settled, image, rate).log10();
                        assert!(
                            db < -95.0,
                            "×{factor} {f} Hz: image at {image} Hz {db:.1} dB"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn delays_by_its_latency() {
        for factor in [1, 2, 4, 8] {
            let mut up = Upsampler::new(48_000.0, factor, 256);
            let mut input = vec![0.0f32; 256];
            input[10] = 1.0;
            let mut out = vec![0.0; 256 * factor];
            up.process(&input, &mut out);
            let peak = out
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
                .unwrap()
                .0;
            assert_eq!(peak, 10 * factor + up.latency(), "×{factor}");
        }
    }
}
