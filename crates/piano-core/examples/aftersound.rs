//! Aftersound of the coupled unison modes, per key: each coupled mode's
//! amplitude at the bridge after a hammer blow (excitation Σ_j hit_j·V_jm
//! times output Σ_j w_j·V_jm), then the smoothed partial envelope
//! √Σ|a_m|²·e^{−2σ_m t} (beats ignored) at 1, 2 and 3 s re t = 0, for
//! partials 1–6, and the level of the slow modes (σ < σ_prompt/1.5) re the
//! prompt mode.
use piano_core::complex::C64;
use piano_core::params::PianoModel;
use piano_core::soundboard::Soundboard;
use piano_core::unison::MAX_OSC;

fn main() {
    let model = PianoModel::default();
    let sr = 48_000.0;
    let sb = Soundboard::new(model.soundboard.unwrap());
    let modes = piano_core::board::board_modes(&sb, sr);
    let pol = model.polarization;
    println!("note  partials 1–6: env dB at 1/2/3 s | after dB re prompt | T60 prompt / after (s)");
    let mut acc = [[0.0f64; 3]; 4];
    let mut cnt = [0usize; 4];
    for note in (21..=99).step_by(3) {
        let nm = piano_core::note_model::NoteModel::build(&model, Some(&sb), Some(&modes), note, sr, 1200);
        let ns = nm.params.strings;
        let hits = model.unison.hits(note);
        let mut h = [0.0; MAX_OSC];
        let mut w = [0.0; MAX_OSC];
        for j in 0..ns {
            h[j] = hits[j];
            w[j] = 1.0;
            if pol.enabled {
                h[ns + j] = pol.hammer_tilt * hits[j];
                w[ns + j] = pol.radiation;
            }
        }
        let mut line = String::new();
        let mut env_sum = [0.0f64; 3];
        for (ki, cl) in nm.clusters.iter().take(6).enumerate() {
            let a: Vec<(f64, f64)> = (0..cl.n)
                .map(|m| {
                    let a: C64 = cl.output_weight(m, &h) * cl.output_weight(m, &w);
                    (a.norm(), -cl.lam[m].re)
                })
                .collect();
            let e = |t: f64| a.iter().map(|(x, s)| x * x * (-2.0 * s * t).exp()).sum::<f64>();
            let e0 = e(0.0);
            let lv: Vec<f64> = [1.0, 2.0, 3.0].iter().map(|&t| 10.0 * (e(t) / e0).log10()).collect();
            for i in 0..3 {
                env_sum[i] += lv[i];
            }
            let p = a.iter().cloned().fold((0.0, 0.0), |b, x| if x.0 > b.0 { x } else { b });
            let slow: f64 = a.iter().filter(|x| x.1 < p.1 / 1.5).map(|x| x.0 * x.0).sum();
            let ts = a.iter().filter(|x| x.1 < p.1 / 1.5).map(|x| x.1).fold(0.0, f64::max);
            if ki < 3 {
                line += &format!(
                    " k{}:{:4.0}{:4.0}{:4.0}|{:4.0}|{:4.1}/{:4.1}",
                    ki + 1,
                    lv[0],
                    lv[1],
                    lv[2],
                    10.0 * (slow / (p.0 * p.0)).log10(),
                    6.91 / p.1,
                    6.91 / ts
                );
            }
        }
        let r = if note < 48 { 0 } else if note < 72 { 1 } else if note < 96 { 2 } else { 3 };
        for i in 0..3 {
            acc[r][i] += env_sum[i] / 6.0;
        }
        cnt[r] += 1;
        println!("{note:3} {line}");
    }
    for (r, lab) in ["bass", "middle", "treble", "top treble"].iter().enumerate() {
        if cnt[r] > 0 {
            println!("{lab:8} mean env k1–6 at 1/2/3 s: {:6.1} {:6.1} {:6.1}", acc[r][0] / cnt[r] as f64, acc[r][1] / cnt[r] as f64, acc[r][2] / cnt[r] as f64);
        }
    }
}
