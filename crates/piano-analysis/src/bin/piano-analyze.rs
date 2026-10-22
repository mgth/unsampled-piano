//! Analyse one recorded or rendered note: f0, B, per-partial decays.

use clap::Parser;
use piano_analysis::{decay, distance, esprit, inharm, io};
use std::path::PathBuf;

#[derive(Parser)]
#[command(about = "Analyse a single piano note (WAV): inharmonicity and partial decays")]
struct Args {
    wav: PathBuf,
    /// MIDI note number of the recording.
    #[arg(long)]
    note: u8,
    #[arg(long, default_value_t = 440.0)]
    a4: f64,
    /// Channel to analyse (default: mix of all channels).
    #[arg(long)]
    channel: Option<usize>,
    /// Number of partials whose decay is fitted.
    #[arg(long, default_value_t = 12)]
    decays: usize,
    /// Ignore everything after this time (s), e.g. the key release.
    #[arg(long)]
    until: Option<f64>,
    /// Resolve the first N partials into components (subband ESPRIT).
    #[arg(long, default_value_t = 0)]
    esprit: usize,
    /// Optional second file to compare with (multi-resolution STFT distance).
    #[arg(long)]
    compare: Option<PathBuf>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a = Args::parse();
    let (mut x, sr) = io::read_mono(&a.wav, a.channel).map_err(|e| e.to_string())?;
    if let Some(u) = a.until {
        x.truncate(((u * sr) as usize).min(x.len()));
    }
    let f_nom = a.a4 * 2f64.powf((a.note as f64 - 69.0) / 12.0);
    let fit = inharm::estimate(&x, sr, f_nom, inharm::Options::default()).ok_or("no partials found")?;
    println!("file      {}", a.wav.display());
    println!("sr        {sr} Hz, {:.2} s", x.len() as f64 / sr);
    println!("f0        {:.3} Hz ({:+.2} cents vs nominal)", fit.f0, 1200.0 * (fit.f0 / f_nom).log2());
    println!("B         {:.3e}", fit.b);
    println!("partials  {} tracked (highest k = {})", fit.partials.len(), fit.partials.last().map_or(0, |p| p.k));
    let on = fit.onset;
    println!("centroid  {:.0} Hz (first 50 ms)", distance::centroid(&x, sr, on, (0.05 * sr) as usize));
    println!();
    println!("  k   freq Hz   level dB  T60 s   double: T60fast  T60slow  slow/fast dB");
    let tail = &x[on..];
    for p in fit.partials.iter().take(a.decays) {
        let spacing = fit.f0 * (1.0 + 2.0 * fit.b * (p.k * p.k) as f64);
        let win = (4.0 / spacing).clamp(0.02, 0.2);
        let env = decay::envelope(tail, sr, p.freq, win, win / 4.0);
        let pts = decay::usable(&env, 10.0);
        let single = decay::line_fit(pts);
        let double = decay::fit_double(pts);
        print!("{:3}  {:9.2}  {:8.1}", p.k, p.freq, p.level_db);
        match single {
            Some(s) => print!("  {:6.2}", s.t60()),
            None => print!("  {:>6}", "-"),
        }
        match double {
            // Only when two slopes explain the envelope clearly better than one.
            Some(d)
                if d.sigma1 > d.sigma2
                    && d.sigma2 > 0.0
                    && (-60.0..0.0).contains(&(20.0 * (d.a2 / d.a1).log10()))
                    && single.is_some_and(|s| d.rms_err_db < 0.5 * s.rms_err_db) =>
            {
                println!(
                "           {:6.2}   {:6.2}   {:6.1}",
                6.9078 / d.sigma1,
                6.9078 / d.sigma2,
                20.0 * (d.a2 / d.a1).log10()
            )
            }
            _ => println!(),
        }
    }
    if a.esprit > 0 {
        println!("\nComponents (sub-band ESPRIT, amplitudes extrapolated to the start of the segment)");
        println!("  k  rank    freq Hz    cents/main   T60 s    dB/main");
        let seg = (0.05 * sr) as usize;
        let spec = esprit::SegmentSpectrum::new(tail, sr, seg, (tail.len() - seg.min(tail.len())).min((12.0 * sr) as usize));
        let dur = (spec.duration() - 0.3).clamp(0.4, 10.0);
        for p in fit.partials.iter().take(a.esprit) {
            let spacing = fit.f0 * (1.0 + 2.0 * fit.b * (p.k * p.k) as f64);
            let modes = esprit::analyse_partial(&spec, p.freq, spacing, 0.1, dur, esprit::Options::default());
            for (r, m) in modes.iter().enumerate() {
                println!(
                    "{:3}  {r:4}  {:9.3}  {:+12.2}  {:6.2}  {:+9.1}",
                    p.k,
                    m.freq,
                    1200.0 * (m.freq / modes[0].freq).log2(),
                    m.t60(),
                    20.0 * (m.amp / modes[0].amp).log10()
                );
            }
        }
    }
    if let Some(other) = a.compare {
        let (y, sr2) = io::read_mono(&other, a.channel).map_err(|e| e.to_string())?;
        if sr2 != sr {
            return Err("sample rates differ".into());
        }
        let d = distance::mrstft(&x, &y);
        println!("\nMR-STFT vs {}: SC {:.3}, log-mag L1 {:.3}", other.display(), d.spectral_convergence, d.log_mag_l1);
    }
    Ok(())
}
