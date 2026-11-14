//! The board's driving-point mobility |Y| (s/kg) at a key's bridge point, per third octave
//! (mean of |Y| over the band), from a preset (analysis).
//!   cargo run --release -p piano-render --example bridge_mobility -- <preset.toml> note [note …]

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let model = piano_host::load_preset(std::path::Path::new(&a[0])).unwrap();
    let sb = model.board().expect("a soundboard");
    let bands: Vec<f64> = (0..11).map(|i| 500.0 * 2f64.powf(i as f64 / 3.0)).collect();
    println!("| key | {} |", bands.iter().map(|b| format!("{b:.0}")).collect::<Vec<_>>().join(" | "));
    for n in &a[1..] {
        let note: u8 = n.parse().unwrap();
        let (x, y) = sb.params.bridge_point(note);
        let row: Vec<String> = bands
            .iter()
            .map(|&c| {
                let (lo, hi) = (c / 2f64.powf(1.0 / 6.0), c * 2f64.powf(1.0 / 6.0));
                let m = 60;
                let s: f64 = (0..m).map(|i| { let v = sb.admittance(x, y, std::f64::consts::TAU * (lo + (hi - lo) * (i as f64 + 0.5) / m as f64)); (v.re * v.re + v.im * v.im).sqrt() }).sum();
                format!("{:.2e}", s / m as f64)
            })
            .collect();
        println!("| {note} | {} |", row.join(" | "));
    }
}
