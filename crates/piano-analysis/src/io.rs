use std::path::Path;

/// Reads a WAV file as f32 mono: one channel, or the mean of all channels.
pub fn read_wav_mono(path: &Path, channel: Option<usize>) -> Result<(Vec<f32>, f64), hound::Error> {
    let mut r = hound::WavReader::open(path)?;
    let spec = r.spec();
    let ch = spec.channels as usize;
    let all: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => r.samples::<f32>().collect::<Result<_, _>>()?,
        hound::SampleFormat::Int => {
            let scale = 1.0 / (1u64 << (spec.bits_per_sample - 1)) as f32;
            r.samples::<i32>().map(|s| s.map(|v| v as f32 * scale)).collect::<Result<_, _>>()?
        }
    };
    let mono = all
        .chunks_exact(ch)
        .map(|f| match channel {
            Some(c) => f[c.min(ch - 1)],
            None => f.iter().sum::<f32>() / ch as f32,
        })
        .collect();
    Ok((mono, spec.sample_rate as f64))
}

/// Reads a FLAC file as f32 mono: one channel, or the mean of all channels.
pub fn read_flac_mono(path: &Path, channel: Option<usize>) -> Result<(Vec<f32>, f64), claxon::Error> {
    let mut r = claxon::FlacReader::open(path)?;
    let info = r.streaminfo();
    let ch = info.channels as usize;
    let scale = 1.0 / (1u64 << (info.bits_per_sample - 1)) as f32;
    let all: Vec<f32> = r.samples().map(|s| s.map(|v| v as f32 * scale)).collect::<Result<_, _>>()?;
    let mono = all
        .chunks_exact(ch)
        .map(|f| match channel {
            Some(c) => f[c.min(ch - 1)],
            None => f.iter().sum::<f32>() / ch as f32,
        })
        .collect();
    Ok((mono, info.sample_rate as f64))
}

/// WAV or FLAC, by extension.
pub fn read_mono(path: &Path, channel: Option<usize>) -> Result<(Vec<f32>, f64), Box<dyn std::error::Error + Send + Sync>> {
    match path.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).as_deref() {
        Some("flac") => Ok(read_flac_mono(path, channel)?),
        _ => Ok(read_wav_mono(path, channel)?),
    }
}

/// Every channel of a recording, and its sample rate.
pub type Channels = (Vec<Vec<f32>>, f64);

/// WAV or FLAC channels, without summing their pressures.
pub fn read_channels(path: &Path) -> Result<Channels, Box<dyn std::error::Error + Send + Sync>> {
    let (all, ch, sr) = if path.extension().and_then(|s| s.to_str()).is_some_and(|s| s.eq_ignore_ascii_case("flac")) {
        let mut r = claxon::FlacReader::open(path)?;
        let info = r.streaminfo();
        let scale = 1.0 / (1u64 << (info.bits_per_sample - 1)) as f32;
        let all: Vec<f32> = r.samples().map(|s| s.map(|v| v as f32 * scale)).collect::<Result<_, _>>()?;
        (all, info.channels as usize, info.sample_rate as f64)
    } else {
        let mut r = hound::WavReader::open(path)?;
        let spec = r.spec();
        let all: Vec<f32> = match spec.sample_format {
            hound::SampleFormat::Float => r.samples::<f32>().collect::<Result<_, _>>()?,
            hound::SampleFormat::Int => {
                let scale = 1.0 / (1u64 << (spec.bits_per_sample - 1)) as f32;
                r.samples::<i32>().map(|s| s.map(|v| v as f32 * scale)).collect::<Result<_, _>>()?
            }
        };
        (all, spec.channels as usize, spec.sample_rate as f64)
    };
    if ch == 0 || all.is_empty() || all.len() % ch != 0 {
        return Err("empty audio or incomplete channel frames".into());
    }
    let mut channels = vec![Vec::with_capacity(all.len() / ch); ch];
    for frame in all.chunks_exact(ch) {
        for (out, &sample) in channels.iter_mut().zip(frame) {
            out.push(sample);
        }
    }
    Ok((channels, sr))
}

/// MIDI number of a note name such as "A0", "D#1", "Bb3" (C4 = 60).
pub fn parse_note_name(s: &str) -> Option<u8> {
    let mut chars = s.chars();
    let pc = match chars.next()?.to_ascii_uppercase() {
        'C' => 0,
        'D' => 2,
        'E' => 4,
        'F' => 5,
        'G' => 7,
        'A' => 9,
        'B' => 11,
        _ => return None,
    };
    let rest: String = chars.collect();
    let (acc, oct) = if let Some(r) = rest.strip_prefix('#') {
        (1, r)
    } else if let Some(r) = rest.strip_prefix('b') {
        (-1, r)
    } else {
        (0, rest.as_str())
    };
    let oct: i32 = oct.parse().ok()?;
    u8::try_from(12 * (oct + 1) + pc + acc).ok()
}
