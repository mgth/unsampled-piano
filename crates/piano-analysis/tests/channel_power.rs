use piano_analysis::{decay, inharm, io, spectrum::Spectrum};

fn tone(sigma: f64) -> Vec<f32> {
    let f0 = 261.625565;
    (0..96000)
        .map(|i| {
            let t = i as f64 / 48000.0;
            (1..=8)
                .map(|k| {
                    let k = k as f64;
                    ((std::f64::consts::TAU * f0 * k * (1.0 + 0.0003 * k * k).sqrt() * t).sin() * (-sigma * t).exp() / k) as f32
                })
                .sum()
        })
        .collect()
}

#[test]
fn opposite_phase_preserves_partials_and_decay() {
    let x = tone(1.2);
    let opposite: Vec<f32> = x.iter().map(|v| -v).collect();
    let m = inharm::estimate(&x, 48000.0, 261.625565, Default::default()).unwrap();
    let s = inharm::estimate_channels(&[&x, &opposite], 48000.0, 261.625565, Default::default()).unwrap();
    assert!((m.f0 - s.f0).abs() < 1e-6);
    assert!((m.b - s.b).abs() < 1e-9);
    assert_eq!(m.partials.len(), s.partials.len());
    let env = decay::envelope_channels(&[&x, &opposite], 48000.0, s.partials[0].freq, 0.05, 0.01);
    let fit = decay::line_fit(&env).unwrap();
    assert!((fit.sigma() - 1.2).abs() < 0.01);
}

#[test]
fn power_spectrum_weights_energy_not_channel_count_or_pressure() {
    let x = tone(1.0);
    let silent = vec![0.0; x.len()];
    let mono = Spectrum::new(&x, 48000.0, 0, 48000, 1);
    let stereo = Spectrum::channel_power(&[&x, &silent], 48000.0, 0, 48000, 1);
    let swapped = Spectrum::channel_power(&[&silent, &x], 48000.0, 0, 48000, 1);
    let a = mono.peak_in(250.0, 275.0).unwrap();
    let b = stereo.peak_in(250.0, 275.0).unwrap();
    assert!((b.1 - a.1 + 10.0 * 2f64.log10()).abs() < 1e-8);
    assert_eq!(stereo.db, swapped.db);
}

#[test]
fn distinct_channel_decays_are_combined_before_fitting() {
    let sr = 48000.0;
    let make = |sigma: f64| -> Vec<f32> {
        (0..96000)
            .map(|i| {
                let t = i as f64 / sr;
                ((std::f64::consts::TAU * 440.0 * t).sin() * (-sigma * t).exp()) as f32
            })
            .collect()
    };
    let fast = make(3.0);
    let slow = make(0.3);
    let e = decay::envelope_channels(&[&fast, &slow], sr, 440.0, 0.02, 0.01);
    for &(t, db) in e.iter().step_by(20) {
        let expected = 10.0 * (((-6.0 * t).exp() + (-0.6 * t).exp()) / 2.0).log10();
        assert!((db - expected).abs() < 0.01, "{t}: {db} vs {expected}");
    }
}

#[test]
fn wav_reader_retains_channel_polarity() {
    let path = std::env::temp_dir().join(format!("piano-channels-{}.wav", std::process::id()));
    let spec = hound::WavSpec { channels: 2, sample_rate: 48000, bits_per_sample: 32, sample_format: hound::SampleFormat::Float };
    let mut w = hound::WavWriter::create(&path, spec).unwrap();
    for v in [0.25f32, -0.25, 0.5, -0.5] {
        w.write_sample(v).unwrap();
    }
    w.finalize().unwrap();
    let (x, sr) = io::read_channels(&path).unwrap();
    std::fs::remove_file(path).unwrap();
    assert_eq!(sr, 48000.0);
    assert_eq!(x, vec![vec![0.25, 0.5], vec![-0.25, -0.5]]);
}
