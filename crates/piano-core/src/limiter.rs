//! Output limiter without latency (`limiter`, `limiter_ceiling_db`): no
//! sample leaves above the ceiling. The gain falls at once to what the
//! sample needs and comes back with a 100 ms time constant, so a loud
//! attack only dips the notes under it briefly; without a look-ahead the
//! piano is not delayed. Loud treble attacks reach full scale long before
//! the music sounds loud (a peaky attack: docs/limiter.md).

/// Release time constant (s).
const RELEASE_S: f64 = 0.1;

pub struct Limiter {
    gain: f32,
    release: f32,
}

impl Limiter {
    pub fn new(sample_rate: f64) -> Self {
        Self { gain: 1.0, release: (1.0 - (-1.0 / (RELEASE_S * sample_rate)).exp()) as f32 }
    }

    /// Limits a stereo block in place to `ceiling` (linear), both channels
    /// alike (the image stays); returns the lowest gain applied.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32], ceiling: f32) -> f32 {
        let mut lowest = 1.0f32;
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            let peak = l.abs().max(r.abs());
            let want = if peak > ceiling { ceiling / peak } else { 1.0 };
            self.gain = if want < self.gain { want } else { self.gain + (want - self.gain) * self.release };
            *l *= self.gain;
            *r *= self.gain;
            lowest = lowest.min(self.gain);
        }
        lowest
    }

    /// Off: the next block starts at unity gain.
    pub fn reset(&mut self) {
        self.gain = 1.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_leaves_above_the_ceiling_and_the_gain_comes_back() {
        let sr = 48_000.0;
        let mut lim = Limiter::new(sr);
        let ceiling = 10f32.powf(-1.0 / 20.0);
        // A loud burst (peaks at 4) then a quiet tone.
        let n = 48_000;
        let tone = |i: usize, a: f32| a * (std::f32::consts::TAU * 440.0 * i as f32 / sr as f32).sin();
        let mut l: Vec<f32> = (0..n).map(|i| tone(i, if i < 4800 { 4.0 } else { 0.3 })).collect();
        let mut r = l.clone();
        let lowest = lim.process(&mut l, &mut r, ceiling);
        assert!(l.iter().chain(&r).all(|x| x.abs() <= ceiling * (1.0 + 1e-6)));
        assert!(lowest < 0.25 * 1.01 && lowest > 0.2);
        // Half a second after the burst the quiet tone is back to full level.
        let tail = &l[n - 4800..];
        let peak = tail.iter().fold(0.0f32, |m, x| m.max(x.abs()));
        assert!((peak - 0.3).abs() < 0.003, "{peak}");
        // Below the ceiling it passes untouched.
        let mut a: Vec<f32> = (0..1000).map(|i| tone(i, 0.5)).collect();
        let b = a.clone();
        let mut c = a.clone();
        let mut fresh = Limiter::new(sr);
        fresh.process(&mut a, &mut c, ceiling);
        assert_eq!(a, b);
    }
}
