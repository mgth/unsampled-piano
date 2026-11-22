//! The two stages of a key's partials as its coupled modes predict them.
//!
//! For each partial of a key in a band: the string's own rate, the
//! bridge's coupling eps_V (Re: one string's bridge loss), the strings'
//! frequency split, and the key's coupled modes (3 strings x 2
//! polarisations): rate, offset from the partial, their weight in the
//! hammer's blow and in the output. From them the line's envelope
//! |sum_m a_m e^(lambda_m t)|^2, read in Hann windows as the decay
//! measures do: early slope (0.1 s windows over 0.02-0.32 s), late slope
//! (0.2 s windows over 0.3-2.2 s) and the level at 0.3 s re the start;
//! and the share of the output the sideways motion carries, at the start
//! and (incoherently) at 0.3 s.
//! The blow is approximated: each string's force is the hit pattern
//! (1 + spread * uneven) times its strike shape, delayed by its height lag;
//! the horizontal oscillators take hammer_tilt of it.
//! Rates in 1/s (amplitude) unless marked dB/s.
//!   cargo run --release -p piano-host --example unison_stages -- preset.toml [velocity fmin fmax] [notes...]
use piano_core::complex::C64;
use std::f64::consts::{PI, TAU};

fn window_db(a: &[(C64, C64)], t0: f64, len: f64) -> f64 {
    // a: (amplitude, lambda relative to the partial's own angular frequency)
    let n = 200;
    let mut p = 0.0;
    let mut wsum = 0.0;
    for i in 0..n {
        let u = (i as f64 + 0.5) / n as f64;
        let w = (PI * u).sin().powi(2);
        let t = t0 + u * len;
        let mut y = C64::ZERO;
        for &(c, l) in a {
            y += c * l.scale(t).exp();
        }
        p += w * y.norm_sqr();
        wsum += w;
    }
    10.0 * (p / wsum + 1e-300).log10()
}

fn slope(a: &[(C64, C64)], from: f64, to: f64, win: f64, hop: f64) -> f64 {
    let mut ts = Vec::new();
    let mut ys = Vec::new();
    let mut t = from;
    while t + win <= to + 1e-9 {
        ts.push(t + win / 2.0);
        ys.push(window_db(a, t, win));
        t += hop;
    }
    let n = ts.len() as f64;
    let (mt, my) = (ts.iter().sum::<f64>() / n, ys.iter().sum::<f64>() / n);
    let (mut sxy, mut sxx) = (0.0, 0.0);
    for (x, y) in ts.iter().zip(&ys) {
        sxy += (x - mt) * (y - my);
        sxx += (x - mt) * (x - mt);
    }
    sxy / sxx
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let model = piano_host::load_preset(std::path::Path::new(&args[1])).unwrap();
    let vel: u8 = args.get(2).and_then(|v| v.parse().ok()).unwrap_or(100);
    let fmin: f64 = args.get(3).and_then(|v| v.parse().ok()).unwrap_or(1000.0);
    let fmax: f64 = args.get(4).and_then(|v| v.parse().ok()).unwrap_or(8000.0);
    let notes: Vec<u8> = if args.len() > 5 { args[5..].iter().map(|v| v.parse().unwrap()).collect() } else { vec![60, 72, 84, 96] };
    let sr = 48_000.0;
    let inst = piano_core::instrument::Instrument::new(model, sr);
    let model = inst.model;
    let v = model.velocity.hammer_speed(vel);
    let spread = model.unison.spread_at(v);
    println!("# velocity {vel}: hammer {v:.2} m/s, hit spread {spread:.3}");
    println!(
        "note,k,f,sigma_s,re_eps,im_eps,split_hz,modes(df_hz:sigma:hit_db:out_db),early_dbs,late_dbs,level03_db,fast_sigma,slow_sigma,knee_db,sideways0_db,sideways03_db"
    );
    for note in notes {
        let nm = inst.table.get(note);
        let np = nm.params;
        let ns = np.strings;
        let n = nm.n_osc;
        let pol = model.polarization.at(note);
        let heights = model.unison.heights(note);
        println!("# note {note}: T {:.1} N, L {:.4} m, T/L {:.0} 1/s per s/kg, strings {ns}, detune {:?} cents, B {:.3e}", np.tension, np.length, np.tension / np.length, &np.detune[..ns], np.b);
        let lag: Vec<f64> = (0..ns).map(|j| heights[j] * model.unison.height_spread / v.max(0.05)).collect();
        for ki in 0..nm.n_partials {
            let cl = &nm.clusters[ki];
            let p0 = nm.phys[ki * n];
            let (k, f) = (p0.k, p0.freq);
            if f < fmin || f > fmax {
                continue;
            }
            // eps_V from the trace: sum(lambda) = sum(d) - ns*(eps_V + eps_H).
            let mut sd = C64::ZERO;
            let mut sl = C64::ZERO;
            for j in 0..n {
                let p = nm.phys[ki * n + j];
                sd += C64::new(-p.sigma, TAU * p.freq);
                sl += cl.lam[j];
            }
            let h = if pol.enabled {
                let top = if pol.slope_to > 0.0 { pol.slope_to } else { f64::INFINITY };
                if pol.slope == 0.0 { pol.h_coupling } else { pol.h_coupling * (f.min(top).max(pol.slope_from).max(1.0) / 261.63).powf(pol.slope) }
            } else {
                0.0
            };
            let eps = (sd - sl).scale(1.0 / (ns as f64 * (1.0 + h)));
            let fr: Vec<f64> = (0..ns).map(|j| nm.phys[ki * n + j].freq).collect();
            let split = fr.iter().cloned().fold(f64::MIN, f64::max) - fr.iter().cloned().fold(f64::MAX, f64::min);
            // The blow.
            let mut zeta = [C64::ZERO; 6];
            for j in 0..n {
                let s = j % ns;
                let p = nm.phys[ki * n + j];
                let x = np.strike_pos + np.strike_offset[s];
                let shape = (k as f64 * PI * x).sin() / (k as f64 * PI * np.strike_pos).sin().abs().max(1e-6);
                let hit = p.hit * (1.0 + spread * p.uneven) * shape;
                zeta[j] = C64::polar(hit, -TAU * f * lag[s]);
            }
            let c = cl.to_modes(&zeta);
            let w0 = TAU * f;
            let mut a = Vec::new();
            let mut cells = Vec::new();
            for m in 0..n {
                let ms = &nm.modes[ki * n + m];
                let out = C64::new(ms.out_re, ms.out_im);
                let amp = c[m] * out;
                a.push((amp, C64::new(-ms.sigma, TAU * ms.freq_hz - w0)));
                cells.push((ms.freq_hz - f, ms.sigma, c[m].norm_sqr(), amp.norm_sqr()));
            }
            // Share of the output carried by the sideways motion: each mode's
            // power weighted by its horizontal part (sum over j >= ns of |V_jm|^2),
            // at the start and, incoherently, at 0.3 s.
            let mut hs = [0.0f64; 4];
            for m in 0..n {
                let (mut h2, mut a2) = (0.0, 0.0);
                for j in 0..n {
                    let q = cl.v[j][m].norm_sqr();
                    a2 += q;
                    if j >= ns {
                        h2 += q;
                    }
                }
                let share = if a2 > 0.0 { h2 / a2 } else { 0.0 };
                let p0 = cells[m].3;
                let p3 = p0 * (-2.0 * cells[m].1 * 0.3).exp();
                hs[0] += share * p0;
                hs[1] += p0;
                hs[2] += share * p3;
                hs[3] += p3;
            }
            let tot: f64 = cells.iter().map(|c| c.3).sum::<f64>().max(1e-300);
            let ctot: f64 = cells.iter().map(|c| c.2).sum::<f64>().max(1e-300);
            let modes: Vec<String> = cells
                .iter()
                .map(|(df, s, ch, co)| format!("{df:+.2}:{s:.2}:{:.1}:{:.1}", 10.0 * (ch / ctot).log10(), 10.0 * (co / tot).log10()))
                .collect();
            let early = slope(&a, 0.02, 0.32, 0.1, 0.025);
            let late = slope(&a, 0.3, 2.2, 0.2, 0.1);
            let lvl = window_db(&a, 0.3, 0.1) - window_db(&a, 0.02, 0.1);
            // Incoherent two-exponential reading: the loudest mode, and the
            // slowest within 40 dB of it.
            let big = cells.iter().cloned().fold((0.0, 0.0, 0.0, 0.0), |b, c| if c.3 > b.3 { c } else { b });
            let slow = cells.iter().filter(|c| c.3 > 1e-4 * big.3).cloned().fold((0.0, f64::MAX, 0.0, 0.0), |b, c| if c.1 < b.1 { c } else { b });
            println!(
                "{note},{k},{f:.1},{:.3},{:.3},{:.3},{split:.3},{},{early:.1},{late:.1},{lvl:.1},{:.3},{:.3},{:.1},{:.1},{:.1}",
                p0.sigma,
                eps.re,
                eps.im,
                modes.join(" "),
                big.1,
                slow.1,
                10.0 * (slow.3 / tot).log10(),
                10.0 * (hs[0] / hs[1].max(1e-300) + 1e-30).log10(),
                10.0 * (hs[2] / hs[3].max(1e-300) + 1e-30).log10()
            );
        }
    }
}
