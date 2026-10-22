//! Bridge conductance seen by the bass partials versus the fit measured on
//! Salamander's decays (tools/bridge_conductance.py):
//! log G = −8.836 + 0.289·log f − 0.0146·midi.
use piano_core::params::{partial_freq, PianoModel};
use piano_core::soundboard::Soundboard;

fn main() {
    let model = PianoModel::default();
    let sb = Soundboard::new(model.soundboard.unwrap());
    for (lo, hi, klo, khi) in [(21u8, 47u8, 1u32, 4u32), (21, 47, 5, 12), (48, 71, 1, 4), (48, 71, 5, 12)] {
        let mut r = Vec::new();
        for note in lo..=hi {
            let np = model.note(note);
            let shapes = sb.bridge_shapes(note);
            for k in klo..=khi {
                let f = partial_freq(np.f0, np.b, k);
                let g = sb.mobility(&shapes, std::f64::consts::TAU * f).re.max(1e-9);
                let fit = (-8.836 + 0.289 * f.ln() - 0.0146 * note as f64).exp();
                r.push((g / fit).log10() * 10.0);
            }
        }
        r.sort_by(f64::total_cmp);
        let q = |p: f64| r[((r.len() - 1) as f64 * p) as usize];
        // Decay rates average the conductance (not its log): mean ratio too.
        let mean_lin = 10.0 * (r.iter().map(|d| 10f64.powf(d / 10.0)).sum::<f64>() / r.len() as f64).log10();
        println!(
            "keys {lo}–{hi}, partials {klo}–{khi}: model/measured G (dB) p10 {:+.1} median {:+.1} p90 {:+.1}, mean of ratio {:+.1}",
            q(0.1),
            q(0.5),
            q(0.9),
            mean_lin
        );
    }
}
