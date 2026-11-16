//! What the frame takes from a key's partials at the strings' far end, in a
//! preset: each partial's decay rate from the string's own losses, with the
//! board at the bridge, and with the frame as well (the unison's coupled
//! modes, mean and slowest), and the two conductances (analysis,
//! docs/capo_duplex.md).
//!   cargo run --release -p piano-render --example capo_probe -- <preset.toml> [key …]
//! Prints CSV: key, partial, f, the string's own rate (dB/s), the coupled
//! modes' mean and slowest rates without the frame, the same with it, the
//! board's and the frame's conductances (s/kg) and the frame's share of the
//! mean rate (%).
use piano_core::note_model::{NoteModel, Resolution};
use piano_core::structure::FrameAdmittance;
use std::f64::consts::TAU;

const DB: f64 = 8.685_889_638;

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let model = piano_host::load_preset(std::path::Path::new(&a[0])).unwrap().with_macros();
    let model = piano_core::params::PianoModel { soundboard: model.soundboard.map(|s| s.resolved()), ..model };
    let board = model.board().expect("a soundboard");
    let frame = FrameAdmittance::new(&model.frame, &board.params).expect("a frame, two way");
    let keys: Vec<u8> = a[1..].iter().filter_map(|k| k.parse().ok()).collect();
    let keys = if keys.is_empty() { (72..=108).step_by(2).collect() } else { keys };
    println!("key,partial,f,own,mean_off,slow_off,mean_on,slow_on,board_g,frame_g,frame_share");
    for note in keys {
        let np = model.note(note);
        let build = |f: Option<&FrameAdmittance>| NoteModel::build_with(&model, Some(&board), None, None, f, note, 48_000.0, Resolution::DEFAULT);
        let (off, on) = (build(None), build(Some(&frame)));
        let shapes = board.bridge_shapes(note);
        let n = on.n_osc;
        // The modes of the length in front of the capo (`frame.duplex`):
        // frequency, the string's own rate there, the modes' mean rate.
        for (i, c) in (on.n_partials..on.clusters.len()).enumerate() {
            let f = on.phys[c * n].freq;
            let mean = on.modes[c * n..(c + 1) * n].iter().map(|x| x.sigma).sum::<f64>() / n as f64 * DB;
            let sh = on.phys[c * n].shape.unwrap();
            println!("{note},d{},{f:.0},{:.1},,,{mean:.1},,nu {:.2},bridge {:.3},strike {:.3}", i + 1, model.losses.sigma(note, f) * DB, sh.nu, sh.bridge, sh.strike);
        }
        for k in 1..=(on.n_partials).min(8) {
            let rates = |m: &NoteModel| {
                let s: Vec<f64> = m.modes[(k - 1) * n..k * n].iter().map(|x| x.sigma).collect();
                (s.iter().sum::<f64>() / n as f64 * DB, s.iter().cloned().fold(f64::MAX, f64::min) * DB)
            };
            let f = piano_core::params::partial_freq(np.f0, np.b, k as u32);
            let (m0, s0) = rates(&off);
            let (m1, s1) = rates(&on);
            let gb = board.bridge_admittance(&shapes, TAU * f).re;
            let gf = frame.at(board.params.key_across(note), TAU * f).re;
            println!(
                "{note},{k},{f:.0},{:.1},{m0:.1},{s0:.1},{m1:.1},{s1:.1},{gb:.3e},{gf:.3e},{:.1}",
                model.losses.sigma(note, f) * DB,
                100.0 * (m1 - m0) / m1
            );
        }
    }
}
