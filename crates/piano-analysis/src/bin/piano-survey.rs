//! Batch analysis of a sample set named like `A0v16.flac` (note + velocity
//! layer): one CSV row per (file, partial).

use clap::Parser;
use piano_analysis::{decay, distance, esprit, inharm, io};
use std::io::Write;
use std::path::PathBuf;

#[derive(Parser)]
#[command(about = "Analyse every <note>v<layer>.{flac,wav} of a directory into a CSV")]
struct Args {
    dir: PathBuf,
    #[arg(long, default_value = "survey.csv")]
    out: PathBuf,
    /// Partials per note whose decay is measured.
    #[arg(long, default_value_t = 12)]
    partials: usize,
    #[arg(long, default_value_t = 440.0)]
    a4: f64,
    /// Drop from the peak (dB) separating the early from the late decay.
    #[arg(long, default_value_t = 15.0)]
    knee_db: f64,
    /// Envelopes are fitted down to this far below their maximum at most
    /// (the recordings' usable range; a render falls to digital silence).
    #[arg(long, default_value_t = 70.0)]
    range_db: f64,
    /// Analyse mean channel power, preserving stereo energy (bench protocol v2).
    /// ESPRIT needs a phase-bearing signal and remains a mono diagnostic.
    #[arg(long, conflicts_with = "esprit")]
    channel_power: bool,
    /// Also resolve each partial into components (subband ESPRIT) into this CSV.
    #[arg(long)]
    esprit: Option<PathBuf>,
}

struct Row {
    name: String,
    note: u8,
    layer: u32,
    line: Result<(Vec<String>, Vec<String>), String>,
}

fn opt(v: Option<f64>) -> String {
    v.filter(|x| x.is_finite()).map_or(String::new(), |x| format!("{x:.4}"))
}

fn analyse(path: &std::path::Path, note: u8, a: &Args) -> Result<(Vec<String>, Vec<String>), String> {
    let (audio, sr) = if a.channel_power {
        io::read_channels(path).map_err(|e| e.to_string())?
    } else {
        let (x, sr) = io::read_mono(path, None).map_err(|e| e.to_string())?;
        (vec![x], sr)
    };
    let channels: Vec<&[f32]> = audio.iter().map(Vec::as_slice).collect();
    let x = channels[0];
    let f_nom = a.a4 * 2f64.powf((note as f64 - 69.0) / 12.0);
    let fit = inharm::estimate_channels(&channels, sr, f_nom, inharm::Options::default()).ok_or("no partials")?;
    let peak = (0..x.len())
        .map(|i| (channels.iter().map(|c| (c[i] as f64).powi(2)).sum::<f64>() / channels.len() as f64).sqrt())
        .fold(0.0f64, f64::max);
    let on = fit.onset;
    let centroid = if a.channel_power {
        let spec = piano_analysis::spectrum::Spectrum::channel_power(&channels, sr, on, (0.05 * sr) as usize, 1);
        let (num, den) = spec.db.iter().enumerate().fold((0.0, 0.0), |(n, d), (i, db)| {
            let mag = 10f64.powf(db / 20.0);
            (n + i as f64 * spec.bin_hz * mag, d + mag)
        });
        num / den.max(1e-30)
    } else {
        distance::centroid(x, sr, on, (0.05 * sr) as usize)
    };
    let head = format!(
        "{:.4},{:.3},{:.4e},{},{},{:.2},{:.1}",
        fit.f0,
        1200.0 * (fit.f0 / f_nom).log2(),
        fit.b,
        fit.partials.len(),
        fit.partials.last().map_or(0, |p| p.k),
        20.0 * peak.max(1e-12).log10(),
        centroid
    );
    let tail = &x[on..];
    let tails: Vec<&[f32]> = channels.iter().map(|c| &c[on..]).collect();
    let mut lines = Vec::new();
    let mut comps = Vec::new();
    // Long segment for the high-resolution analysis, starting just after the attack.
    let seg_start = (0.05 * sr) as usize;
    let spec = a
        .esprit
        .as_ref()
        .map(|_| esprit::SegmentSpectrum::new(tail, sr, seg_start, (tail.len() - seg_start.min(tail.len())).min((12.0 * sr) as usize)));
    for p in fit.partials.iter().take(a.partials) {
        let spacing = fit.f0 * (1.0 + 2.0 * fit.b * (p.k * p.k) as f64);
        let win = (4.0 / spacing).clamp(0.02, 0.2);
        let env = decay::envelope_channels(&tails, sr, p.freq, win, win / 4.0);
        let pts = decay::usable_within(&env, 10.0, a.range_db);
        let single = decay::line_fit(pts);
        let (early, late) = match pts.first() {
            Some(&(_, top)) => {
                let j = pts.iter().position(|q| q.1 < top - a.knee_db).unwrap_or(pts.len());
                let e = if j >= 4 { decay::line_fit(&pts[..j]) } else { None };
                let l = if pts.len() - j >= 4 { decay::line_fit(&pts[j..]) } else { None };
                (e, l)
            }
            None => (None, None),
        };
        let dd = decay::fit_double(pts).filter(|d| {
            d.sigma1 > d.sigma2
                && d.sigma2 > 0.0
                && (-60.0..0.0).contains(&(20.0 * (d.a2 / d.a1).log10()))
                && single.is_some_and(|s| d.rms_err_db < 0.5 * s.rms_err_db)
        });
        let dur = pts.last().zip(pts.first()).map(|(l, f)| l.0 - f.0);
        if let (Some(spec), Some(&(t_end, _))) = (&spec, pts.last()) {
            let len = (t_end - 0.05 - 0.1).clamp(0.4, 10.0).min(spec.duration() - 0.2);
            let modes = esprit::analyse_partial(spec, p.freq, spacing, 0.1, len, esprit::Options::default());
            if let Some(main) = modes.first().copied() {
                for (rank, m) in modes.iter().enumerate() {
                    comps.push(format!(
                        "{},{:.3},{},{:.4},{:.3},{:.4},{:.2},{:.1}",
                        p.k,
                        p.freq,
                        rank,
                        m.freq,
                        1200.0 * (m.freq / main.freq).log2(),
                        m.t60(),
                        20.0 * (m.amp / main.amp).log10(),
                        20.0 * m.amp.max(1e-12).log10()
                    ));
                }
            }
        }
        lines.push(format!(
            "{head},{},{:.3},{:.1},{},{},{},{},{},{},{},{}",
            p.k,
            p.freq,
            p.level_db,
            opt(single.map(|s| s.t60())),
            opt(early.map(|s| s.t60())),
            opt(late.map(|s| s.t60())),
            opt(single.map(|s| s.rms_err_db)),
            opt(dd.map(|d| 6.9078 / d.sigma1)),
            opt(dd.map(|d| 6.9078 / d.sigma2)),
            opt(dd.map(|d| 20.0 * (d.a2 / d.a1).log10())),
            opt(dur),
        ));
    }
    Ok((lines, comps))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a = Args::parse();
    let mut jobs: Vec<(PathBuf, String, u8, u32)> = Vec::new();
    for e in std::fs::read_dir(&a.dir)? {
        let path = e?.path();
        if !path.extension().and_then(|s| s.to_str()).is_some_and(|s| s.eq_ignore_ascii_case("wav") || s.eq_ignore_ascii_case("flac")) {
            continue;
        }
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
        let Some((n, v)) = stem.rsplit_once('v') else {
            continue;
        };
        let (Some(note), Ok(layer)) = (io::parse_note_name(n), v.parse::<u32>()) else {
            continue;
        };
        jobs.push((path, stem, note, layer));
    }
    jobs.sort_by_key(|j| (j.2, j.3));
    eprintln!("{} files", jobs.len());

    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let next = std::sync::atomic::AtomicUsize::new(0);
    let mut rows: Vec<Row> = std::thread::scope(|s| {
        let hs: Vec<_> = (0..threads)
            .map(|_| {
                s.spawn(|| {
                    let mut out = Vec::new();
                    loop {
                        let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        let Some((path, name, note, layer)) = jobs.get(i) else {
                            break;
                        };
                        out.push(Row { name: name.clone(), note: *note, layer: *layer, line: analyse(path, *note, &a) });
                    }
                    out
                })
            })
            .collect();
        hs.into_iter().flat_map(|h| h.join().unwrap()).collect()
    });
    rows.sort_by_key(|r| (r.note, r.layer));

    let mut f = std::io::BufWriter::new(std::fs::File::create(&a.out)?);
    writeln!(
        f,
        "file,midi,layer,f0,cents_et,B,n_partials,k_max,peak_dbfs,centroid_hz,k,freq,level_db,t60,t60_early,t60_late,fit_err_db,dd_t60_fast,dd_t60_slow,dd_slow_db,dur_s"
    )?;
    let mut fe = match &a.esprit {
        Some(p) => {
            let mut w = std::io::BufWriter::new(std::fs::File::create(p)?);
            writeln!(w, "file,midi,layer,k,partial_freq,rank,freq,cents_rel_main,t60,amp_db_rel,amp_db")?;
            Some(w)
        }
        None => None,
    };
    let mut failed = 0;
    for r in &rows {
        match &r.line {
            Ok((lines, comps)) => {
                for l in lines {
                    writeln!(f, "{},{},{},{l}", r.name, r.note, r.layer)?;
                }
                if let Some(fe) = fe.as_mut() {
                    for c in comps {
                        writeln!(fe, "{},{},{},{c}", r.name, r.note, r.layer)?;
                    }
                }
            }
            Err(e) => {
                failed += 1;
                eprintln!("{}: {e}", r.name);
            }
        }
    }
    eprintln!("wrote {} ({} files failed)", a.out.display(), failed);
    Ok(())
}
