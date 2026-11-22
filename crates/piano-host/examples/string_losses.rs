//! The strings' own losses of a preset, partial by partial, beside the
//! decay the engine's coupled modes get: the loss law in use, the three
//! terms of the physical law (losses.physical: air, viscoelastic at
//! delta_ve = 1, constant at q_te = 1, so that they can be scaled), each
//! taken from the key's fundamental up, and the slowest of the key's
//! coupled modes that carries the partial (within 10 dB of its loudest).
//! One CSV row per partial, rates in 1/s (amplitude).
//!   cargo run --release -p piano-host --example string_losses -- preset.toml [first last step]
use piano_core::params::{partial_freq, LossLaw, StringGeometry};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let model = piano_host::load_preset(std::path::Path::new(&args[1])).unwrap().with_macros();
    let lo: u8 = args.get(2).and_then(|v| v.parse().ok()).unwrap_or(48);
    let hi: u8 = args.get(3).and_then(|v| v.parse().ok()).unwrap_or(96);
    let step: usize = args.get(4).and_then(|v| v.parse().ok()).unwrap_or(3);
    let sr = 48_000.0;
    let sb = model.board().unwrap();
    let modes = piano_core::board::board_modes(&sb, sr);
    let unit = LossLaw { physical: true, air_scale: 1.0, delta_ve: 1.0, q_te: 1.0, ..model.losses };
    println!("note,k,f,d_outer_mm,mu,law,air,ve1,te1,slowest,b1");
    for note in (lo..=hi).step_by(step) {
        let np = model.note(note);
        let g = StringGeometry::of(&np, model.nonlinear.young);
        let nm = piano_core::note_model::NoteModel::build(&model, Some(&sb), Some(&modes), note, sr, 1200);
        let [a0, v0, t0] = unit.string_parts(np.f0, &g);
        let b1 = LossLaw { b2: 0.0, b2_top: 0.0, b2_high: 0.0, physical: false, ..model.losses }.sigma(note, np.f0);
        for k in 1..=80u32 {
            let f = partial_freq(np.f0, np.b, k);
            if f > 8000.0 {
                break;
            }
            let ms: Vec<_> = nm.modes.iter().zip(&nm.phys).filter(|(_, p)| p.k == k).map(|(m, _)| m).collect();
            if ms.is_empty() {
                continue;
            }
            let wmax = ms.iter().map(|m| m.out_re.hypot(m.out_im)).fold(1e-300, f64::max);
            let slow = ms.iter().filter(|m| m.out_re.hypot(m.out_im) > 0.316 * wmax).map(|m| m.sigma).fold(f64::INFINITY, f64::min);
            let [a, v, t] = unit.string_parts(f, &g);
            println!(
                "{note},{k},{f:.1},{:.3},{:.4e},{:.4},{:.4},{:.4},{:.4},{:.4},{:.4}",
                g.d_outer * 1e3,
                np.mu,
                model.losses.sigma_on(note, f, &g),
                a - a0,
                v - v0,
                t - t0,
                slow,
                b1
            );
        }
    }
}
