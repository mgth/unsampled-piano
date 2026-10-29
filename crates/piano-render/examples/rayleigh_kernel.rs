//! Causal propagation histogram per mode, research only.
//! rayleigh_kernel PRESET OUTDIR SPATIAL_STEP DELAY_OVERSAMPLING
use piano_core::{board, complex::C64, params::PianoModel};
use std::{f64::consts::TAU, io::Write, path::Path};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<String> = std::env::args().collect();
    if a.len() != 5 {
        return Err("rayleigh_kernel PRESET OUTDIR STEP OVERSAMPLING".into());
    }
    let raw = piano_host::load_preset(Path::new(&a[1]))?.with_macros();
    let model = PianoModel {
        soundboard: raw.soundboard.map(|p| p.resolved()),
        ..raw
    };
    let sb = model.board().ok_or("no board")?;
    let p = &sb.params;
    // The current grand is open; a dispersive case-panel filter needs a
    // separate causal implementation before this prototype can support it.
    assert_eq!(p.case_open, 1.);
    assert_eq!(p.exact_below, 0.);
    assert_eq!(p.scatter, 0., "causal scattering is not implemented by this exporter");
    let step: f64 = a[3].parse()?;
    let os: usize = a[4].parse()?;
    assert!(step > 0. && step <= 0.02 && (8..=64).contains(&os));
    let sr = 48000.;
    let fs = sr * os as f64;
    let out = Path::new(&a[2]);
    std::fs::create_dir_all(out)?;
    let modes = board::board_modes(&sb, sr);
    let mut current = modes.clone();
    board::to_acceleration_weights(&mut current, sr);
    let mut csv = std::fs::File::create(out.join("modes.csv"))?;
    write!(csv, "frequency,sigma,pl_re,pl_im,pr_re,pr_im")?;
    for r in 0..16 {
        write!(csv, ",input{r}")?;
    }
    writeln!(csv)?;
    for m in &current {
        write!(
            csv,
            "{},{},{},{},{},{}",
            m.freq_hz, m.sigma, m.outputs[0].0, m.outputs[0].1, m.outputs[1].0, m.outputs[1].1
        )?;
        for v in m.inputs {
            write!(csv, ",{v}")?;
        }
        writeln!(csv)?;
    }
    let nx = (p.a / step).ceil() as usize;
    let ny = (p.b / step).ceil() as usize;
    let dx = p.a / nx as f64;
    let dy = p.b / ny as f64;
    let mut cells = Vec::new();
    let mut maxdelay: f64 = 0.;
    let mut mindelay: f64 = 1e9;
    for i in 0..nx {
        for j in 0..ny {
            let x = (i as f64 + 0.5) * dx;
            let y = (j as f64 + 0.5) * dy;
            let ds = dx * dy * sb.outline.as_ref().map_or(1., |o| o.coverage(x, y, dx, dy));
            if ds == 0. {
                continue;
            }
            let paths = p
                .mics
                .map(|(mx, my, mz)| p.radiation_paths([x, y, 0.], [mx, my, mz]));
            for ear in paths {
                for (d, g, _) in ear {
                    if g != 0. {
                        maxdelay = maxdelay.max(d / 343. * fs);
                        mindelay = mindelay.min(d / 343. * sr);
                    }
                }
            }
            cells.push((x, y, ds, paths));
        }
    }
    // The resampling filter extends 32 base-rate samples either side.
    assert!(
        mindelay > 32.,
        "not enough physical delay for causal interpolation"
    );
    let len = maxdelay.ceil() as usize + 2 + 32 * os;
    let hist: Vec<Vec<f64>> = piano_core::par::map(modes.len(), |i| {
        let md = &sb.modes[i];
        let mut h = vec![0.; 2 * len];
        for (x, y, ds, paths) in &cells {
            let phi = sb.shape(md, *x, *y) * if md.m == 0 { p.piston_radiation } else { 1. };
            let weight = 1.2 / TAU * phi * ds;
            for (ear, paths) in paths.iter().enumerate() {
                for (d, g, _) in paths {
                    if *g == 0. {
                        continue;
                    }
                    let delay = d / 343. * fs;
                    let ix = delay.floor() as usize;
                    let frac = delay - ix as f64;
                    let v = weight * g / d;
                    h[ear * len + ix] += v * (1. - frac);
                    h[ear * len + ix + 1] += v * frac;
                }
            }
        }
        h
    });
    let mut file = std::io::BufWriter::new(std::fs::File::create(out.join("kernels.f64"))?);
    for h in &hist {
        for v in h {
            file.write_all(&v.to_le_bytes())?;
        }
    }
    file.flush()?;
    let rad = sb.radiation();
    let mut verify = std::fs::File::create(out.join("quadrature.csv"))?;
    writeln!(
        verify,
        "mode,frequency,ear,exact_re,exact_im,hist_re,hist_im"
    )?;
    for i in [0, 1, modes.len() / 4, modes.len() - 1] {
        for ear in 0..2 {
            let w = TAU * modes[i].freq_hz;
            let h = hist[i][ear * len..(ear + 1) * len]
                .iter()
                .enumerate()
                .fold(C64::ZERO, |a, (j, v)| {
                    a + C64::polar(*v, -w * j as f64 / fs)
                });
            let exact = rad[i][ear] / C64::new(0., w);
            writeln!(
                verify,
                "{i},{},{ear},{},{},{},{}",
                modes[i].freq_hz, exact.re, exact.im, h.re, h.im
            )?;
        }
    }
    std::fs::write(
        out.join("shape.txt"),
        format!("{} 2 {len} {os}\n", modes.len()),
    )?;
    std::fs::write(out.join("geometry.txt"),format!("step={step}\ncells={}\nmin_delay_samples={mindelay}\nmax_delay_samples={}\nmic_gain={}\n",cells.len(),maxdelay/os as f64,10f64.powf(p.mic_gain_db/20.)))?;
    eprintln!(
        "{} modes, {} cells, {len} histogram slots at {fs} Hz",
        modes.len(),
        cells.len()
    );
    Ok(())
}
