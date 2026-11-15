//! The board's driving-point mobility at every key's bridge point, in
//! octave bands from 125 Hz to 8 kHz (20·log10 of the mean |Y|, dB re
//! 1 s/kg), as CSV: to compare two presets key by key (analysis).
//!   cargo run --release -p piano-render --example bridge_mobility_keys -- <preset.toml>

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let model = piano_host::load_preset(std::path::Path::new(&a[0])).unwrap().with_macros();
    let sb = model.board().expect("a soundboard");
    let bands: Vec<f64> = (0..7).map(|i| 125.0 * 2f64.powi(i)).collect();
    println!("midi,{}", bands.iter().map(|b| format!("{b:.0}")).collect::<Vec<_>>().join(","));
    for note in 21..=108u8 {
        let (x, y) = sb.params.bridge_point(note);
        let row: Vec<String> = bands
            .iter()
            .map(|&c| {
                let (lo, hi) = (c / 2f64.sqrt(), c * 2f64.sqrt());
                let m = 200;
                let s: f64 = (0..m)
                    .map(|i| {
                        let v = sb.admittance(x, y, std::f64::consts::TAU * lo * (hi / lo).powf((i as f64 + 0.5) / m as f64));
                        (v.re * v.re + v.im * v.im).sqrt()
                    })
                    .sum();
                format!("{:.2}", 20.0 * (s / m as f64).log10())
            })
            .collect();
        println!("{note},{}", row.join(","));
    }
}
