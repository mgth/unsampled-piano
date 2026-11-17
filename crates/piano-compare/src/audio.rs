//! Playback of one recording at a time, resampled to the output device.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct Playing {
    /// Interleaved stereo at the device rate.
    buf: Vec<f32>,
    pos: usize,
}

pub struct Audio {
    _stream: cpal::Stream,
    pub player: Player,
    pub device: String,
}

/// What the UI and its helper threads hold to play.
#[derive(Clone)]
pub struct Player {
    state: Arc<Mutex<Playing>>,
    pub rate: f64,
}

impl Audio {
    pub fn open() -> Result<Self, String> {
        let ids = cpal::available_hosts();
        let id = ids.iter().find(|id| format!("{id:?}").to_lowercase().contains("pipewire")).copied().unwrap_or_else(|| cpal::default_host().id());
        let host = cpal::host_from_id(id).map_err(|e| e.to_string())?;
        let device = host.default_output_device().ok_or("no audio output device")?;
        let config = device.default_output_config().map_err(|e| e.to_string())?;
        let rate = config.sample_rate() as f64;
        let channels = config.channels() as usize;
        let state = Arc::new(Mutex::new(Playing::default()));
        let sc = cpal::StreamConfig { channels: config.channels(), sample_rate: config.sample_rate(), buffer_size: cpal::BufferSize::Default };
        use cpal::SampleFormat as F;
        let stream = match config.sample_format() {
            F::F32 => stream::<f32>(&device, sc, channels, state.clone()),
            F::I16 => stream::<i16>(&device, sc, channels, state.clone()),
            F::I32 => stream::<i32>(&device, sc, channels, state.clone()),
            F::U16 => stream::<u16>(&device, sc, channels, state.clone()),
            f => return Err(format!("unsupported sample format {f:?}")),
        }
        .map_err(|e| e.to_string())?;
        stream.play().map_err(|e| e.to_string())?;
        let name = device.description().map(|d| d.to_string()).unwrap_or_default();
        Ok(Self { _stream: stream, player: Player { state, rate }, device: name })
    }
}

impl Player {
    /// Plays `channels` (1 or more, at `sr`), replacing what is playing.
    pub fn play(&self, channels: &[Vec<f32>], sr: f64, gain: f32) {
        let l = resample(&channels[0], sr, self.rate);
        let r = channels.get(1).map_or_else(|| l.clone(), |c| resample(c, sr, self.rate));
        let n = l.len().min(r.len());
        // Short fade-out so that a cut never clicks.
        let fade = ((0.03 * self.rate) as usize).min(n);
        let mut buf = Vec::with_capacity(2 * n);
        for i in 0..n {
            let g = gain * if i + fade >= n { (n - i) as f32 / fade as f32 } else { 1.0 };
            buf.push(l[i] * g);
            buf.push(r[i] * g);
        }
        *self.state.lock().unwrap() = Playing { buf, pos: 0 };
    }

    pub fn stop(&self) {
        *self.state.lock().unwrap() = Playing::default();
    }

    /// Seconds played, while something plays.
    pub fn position(&self) -> Option<f64> {
        let s = self.state.lock().unwrap();
        (s.pos < s.buf.len()).then(|| s.pos as f64 / 2.0 / self.rate)
    }
}

fn stream<T: cpal::SizedSample + cpal::FromSample<f32>>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    channels: usize,
    state: Arc<Mutex<Playing>>,
) -> Result<cpal::Stream, cpal::Error> {
    device.build_output_stream(
        config,
        move |data: &mut [T], _| {
            data.fill(T::EQUILIBRIUM);
            // Never wait on the UI: skip a buffer instead.
            let Ok(mut s) = state.try_lock() else { return };
            for f in data.chunks_mut(channels) {
                if s.pos + 1 >= s.buf.len() {
                    break;
                }
                f[0] = T::from_sample(s.buf[s.pos]);
                if let Some(x) = f.get_mut(1) {
                    *x = T::from_sample(s.buf[s.pos + 1]);
                }
                s.pos += 2;
            }
        },
        |e| eprintln!("audio error: {e}"),
        None,
    )
}

/// The clip of a recording to play: from 20 ms before its onset, `seconds`
/// long, with its gain (the same mean level over its first second when
/// `matched`, never above 0.98 at the peak).
pub fn prepare(path: &std::path::Path, seconds: f64, matched: bool, volume_db: f32) -> Option<(Vec<Vec<f32>>, f64, f32)> {
    let (ch, sr) = piano_analysis::io::read_channels(path).ok()?;
    let refs: Vec<&[f32]> = ch.iter().map(|c| c.as_slice()).collect();
    let on = piano_analysis::inharm::onset_channels(&refs);
    let start = on.saturating_sub((0.02 * sr) as usize);
    let end = (start + (seconds * sr) as usize).min(ch[0].len());
    let clip: Vec<Vec<f32>> = ch.iter().map(|c| c[start..end].to_vec()).collect();
    let mut gain = 10f32.powf(volume_db / 20.0);
    if matched {
        let a = on - start;
        let b = (a + sr as usize).min(clip[0].len());
        let p: f64 = clip.iter().map(|c| c[a..b].iter().map(|v| (*v as f64).powi(2)).sum::<f64>()).sum::<f64>() / ((b - a).max(1) * clip.len()) as f64;
        if p > 0.0 {
            gain *= (10f64.powf(-24.0 / 20.0) / p.sqrt()) as f32;
        }
    }
    let peak = clip.iter().flatten().fold(0.0f32, |m, v| m.max(v.abs()));
    if peak * gain > 0.98 {
        gain = 0.98 / peak;
    }
    Some((clip, sr, gain))
}

/// Windowed-sinc resampling (32 zero crossings, Blackman window, cut below
/// the lower Nyquist frequency). Between integer rates the kernel is
/// computed once per phase (polyphase): a few milliseconds per second of
/// sound, where computing it per sample took longer than the clip.
pub fn resample(x: &[f32], from: f64, to: f64) -> Vec<f32> {
    if (from - to).abs() < 1e-9 {
        return x.to_vec();
    }
    const HALF: f64 = 32.0;
    let ratio = to / from;
    let cut = ratio.min(1.0) * 0.97;
    let span = (HALF / cut).ceil() as isize;
    let taps = (2 * span) as usize;
    // Tap j of an output whose position falls `frac` past input sample c
    // reads x[c - span + 1 + j].
    let kernel = |frac: f64| -> Vec<f64> {
        (0..taps)
            .map(|j| {
                let d = (j as isize - span + 1) as f64 - frac;
                let w = d / span as f64;
                if w.abs() >= 1.0 {
                    return 0.0;
                }
                let u = d * cut;
                let sinc = if u.abs() < 1e-12 { 1.0 } else { (std::f64::consts::PI * u).sin() / (std::f64::consts::PI * u) };
                let p = std::f64::consts::PI * (w + 1.0);
                cut * sinc * (0.42 - 0.5 * p.cos() + 0.08 * (2.0 * p).cos())
            })
            .collect()
    };
    let n = (x.len() as f64 * ratio) as usize;
    let dot = |c: isize, k: &[f64]| -> f32 {
        let first = c - span + 1;
        let mut acc = 0.0f64;
        if first >= 0 && (first as usize + taps) <= x.len() {
            for (v, w) in x[first as usize..first as usize + taps].iter().zip(k) {
                acc += *v as f64 * w;
            }
        } else {
            for (j, w) in k.iter().enumerate() {
                let i = first + j as isize;
                if i >= 0 && (i as usize) < x.len() {
                    acc += x[i as usize] as f64 * w;
                }
            }
        }
        acc as f32
    };
    let (fi, ti) = (from.round() as u64, to.round() as u64);
    let integer = (from - fi as f64).abs() < 1e-9 && (to - ti as f64).abs() < 1e-9;
    let g = if integer { gcd(fi, ti) } else { 0 };
    if g > 0 && ti / g <= 4096 {
        let (up, down) = (ti / g, fi / g);
        let table: Vec<Vec<f64>> = (0..up).map(|ph| kernel(ph as f64 / up as f64)).collect();
        (0..n as u64).map(|i| dot(((i * down) / up) as isize, &table[((i * down) % up) as usize])).collect()
    } else {
        (0..n)
            .map(|i| {
                let t = i as f64 / ratio;
                dot(t.floor() as isize, &kernel(t - t.floor()))
            })
            .collect()
    }
}

fn gcd(a: u64, b: u64) -> u64 {
    if b == 0 { a } else { gcd(b, a % b) }
}

#[cfg(test)]
mod tests {
    /// Time from the click to the first sample, on the bench's sets
    /// (`cargo test --release -p piano-compare -- --ignored --nocapture`).
    #[test]
    #[ignore]
    fn playback_latency() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let main = crate::sources::main_checkout(&root.canonicalize().unwrap()).join("out");
        for f in ["bench/sets/bivib_grand/C4v8.wav", "bench/sets/headroom/C4v8.flac", "bench/sets/helgesson/C4v8.flac", "compare/iowa/C4v8.wav", "compare/salamander/C4v8.flac", "bench/sets/liang/C4v13.wav", "catalog_clean/steinway_d/C4v8.wav"] {
            let p = main.join(f);
            if !p.is_file() {
                continue;
            }
            let t = std::time::Instant::now();
            let (ch, sr) = piano_analysis::io::read_channels(&p).unwrap();
            let read = t.elapsed().as_secs_f64();
            let refs: Vec<&[f32]> = ch.iter().map(|c| c.as_slice()).collect();
            let on = piano_analysis::inharm::onset_channels(&refs) as f64 / sr;
            let t = std::time::Instant::now();
            let (clip, sr, _) = super::prepare(&p, 3.0, true, -6.0).unwrap();
            let prep = t.elapsed().as_secs_f64();
            let t = std::time::Instant::now();
            for c in &clip {
                super::resample(c, sr, 48_000.0);
            }
            let rs = t.elapsed().as_secs_f64();
            println!("{f}: {:.1} s at {sr} Hz, onset {on:.3} s; read {:.0} ms, prepare {:.0} ms, resample {:.0} ms", ch[0].len() as f64 / sr, read * 1e3, prep * 1e3, rs * 1e3);
        }
    }

    #[test]
    fn resampled_sine_keeps_level_and_pitch() {
        for (from, to) in [(44_100.0, 48_000.0), (96_000.0, 48_000.0), (50_000.0, 48_000.0), (44_100.5, 48_000.0)] {
            sine(from, to);
        }
    }

    fn sine(from: f64, to: f64) {
        let f = 1000.0;
        let x: Vec<f32> = (0..from as usize).map(|i| (std::f64::consts::TAU * f * i as f64 / from).sin() as f32).collect();
        let y = super::resample(&x, from, to);
        assert!((y.len() as f64 - 48_000.0).abs() < 2.0, "{from}: {} samples", y.len());
        let mid = &y[10_000..38_000];
        let rms = (mid.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / mid.len() as f64).sqrt();
        assert!((rms - 0.5f64.sqrt()).abs() < 0.01, "rms {rms}");
        let crossings = mid.windows(2).filter(|w| w[0] <= 0.0 && w[1] > 0.0).count() as f64;
        assert!((crossings / (mid.len() as f64 / to) - f).abs() < 2.0);
    }
}
