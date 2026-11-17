//! Which board modes make a key's bridge move and its sound reach the
//! microphones at one frequency (analysis): for the key's own bridge point,
//! each mode's share of the bridge velocity and of each microphone's
//! pressure at `freq`, largest first.
//!   cargo run --release -p piano-render --example mode_contrib -- <preset.toml> <note> <freq Hz> [listening.toml …]
use piano_core::board::{board_modes_at, mic_differentiator, to_acceleration_weights, transfer_with};
use piano_core::instrument::MICS;
use piano_core::params::PianoModel;

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let mut model = piano_host::load_preset(std::path::Path::new(&a[0])).unwrap();
    for l in &a[3..] {
        model = piano_host::load_listening(std::path::Path::new(l), &model).unwrap();
    }
    let model = model.with_macros();
    let note: u8 = a[1].parse().unwrap();
    let f: f64 = a[2].parse().unwrap();
    let sr = 48_000.0;
    let model = PianoModel { soundboard: model.soundboard.map(|s| s.resolved()), ..model };
    let sb = model.board().expect("a soundboard");
    let p = sb.params.bridge_point(note);
    let mut modes = board_modes_at(&sb, &[p; piano_core::board::REGIONS], sr);
    to_acceleration_weights(&mut modes, sr);
    let w = std::f64::consts::TAU * f;
    let total = |o: usize, only: Option<usize>| transfer_with(&modes, |i, m| if only.is_none_or(|j| j == i) { m.inputs[0] } else { 0.0 }, o, w, sr);
    let v_all = total(piano_core::board::BRIDGE, None);
    let m_all: Vec<_> = (0..MICS).map(|o| total(o, None) * mic_differentiator(w, sr)).collect();
    println!("note {note}, bridge point ({:.3}, {:.3}) m, {f} Hz: |bridge| {:.3e}, |mic L| {:.3e}, |mic R| {:.3e}", p.0, p.1, v_all.norm(), m_all[0].norm(), m_all[1].norm());
    let mut rows: Vec<(usize, f64, f64, f64, f64, f64)> = (0..modes.len())
        .map(|i| {
            let v = total(piano_core::board::BRIDGE, Some(i));
            let l = total(0, Some(i)) * mic_differentiator(w, sr);
            let r = total(1, Some(i)) * mic_differentiator(w, sr);
            // Projections on the totals: the share each mode adds (negative: it cancels).
            let pv = (v.re * v_all.re + v.im * v_all.im) / v_all.norm().powi(2);
            let pl = (l.re * m_all[0].re + l.im * m_all[0].im) / m_all[0].norm().powi(2);
            let pr = (r.re * m_all[1].re + r.im * m_all[1].im) / m_all[1].norm().powi(2);
            (i, modes[i].freq_hz, modes[i].sigma, pv, pl, pr)
        })
        .collect();
    rows.sort_by(|x, y| (y.4.abs() + y.5.abs()).total_cmp(&(x.4.abs() + x.5.abs())));
    println!("mode  freq Hz  sigma 1/s  share of bridge velocity  share of mic L  share of mic R");
    for (i, fr, s, pv, pl, pr) in rows.iter().take(14) {
        println!("{i:5} {fr:8.1} {s:9.1} {pv:10.2} {pl:16.2} {pr:14.2}");
    }
}
