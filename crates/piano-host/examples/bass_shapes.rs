//! A preset's first board modes as its bridges see them: each mode's
//! frequency and loss factor, the mass it shows at a few keys' bridge points
//! (1/Φ², kg, the mode being mass-normalised), where it moves most, and the
//! driving-point mobility at those keys. With `MAP=dir`, each mode's shape on
//! a 2 cm grid as text (x, y, Φ; NaN outside the board), and the bridges'
//! points.
//!   cargo run --release -p piano-host --example bass_shapes -- preset.toml [modes]
use std::f64::consts::TAU;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let model = piano_host::load_preset(std::path::Path::new(&args[1])).unwrap();
    let count: usize = args.get(2).and_then(|v| v.parse().ok()).unwrap_or(10);
    let sb = model.board().unwrap();
    let p = sb.params;
    let keys: [u8; 12] = [21, 24, 28, 33, 40, 41, 45, 52, 60, 72, 84, 96];
    let shapes: Vec<Vec<f64>> = keys.iter().map(|&k| sb.bridge_shapes(k)).collect();
    let step = 0.02;
    let (nx, ny) = ((p.a / step) as usize, (p.b / step) as usize);
    let inside = |x: f64, y: f64| sb.outline.as_ref().is_none_or(|o| o.contains(x, y));
    if let Some(fem) = &sb.fem {
        println!("built board: {:.2} kg on {:.2} m²", fem.mass, fem.area);
    }
    println!("board {:.3} × {:.3} m; bridge points (x, y, distance to the rim, m):", p.a, p.b);
    for &k in &keys {
        let (x, y) = p.bridge_point(k);
        println!("  key {k:3}: {x:.3} {y:.3}  {:.3}", sb.outline.as_ref().map_or(f64::NAN, |o| o.edge_distance(x, y)));
    }
    if p.structure.strings {
        let (hold, grip) = (model.string_hold(&p.structure), model.string_grip(&p.structure));
        println!("the strings' hold (kN/m) and, along their length, E·A·(1/L + 1/l) (MN/m), per key:");
        for &k in &keys {
            println!("  key {k:3}: {:6.1} {:6.2}", hold.0[k as usize - 21] / 1e3, grip.0[k as usize - 21] / 1e6);
        }
    }
    println!("mode  Hz     η     peak at (x, y)  mass at the peak | mass at the keys' bridge points (kg): {}", keys.map(|k| format!("{k:7}")).join(""));
    for (i, md) in sb.modes.iter().take(count).enumerate() {
        let (mut peak, mut at) = (0.0f64, (0.0, 0.0));
        let mut map = String::new();
        for a in 0..=nx {
            for b in 0..=ny {
                let (x, y) = (a as f64 * step, b as f64 * step);
                let v = if inside(x, y) { sb.shape(md, x, y) } else { f64::NAN };
                if v.abs() > peak {
                    (peak, at) = (v.abs(), (x, y));
                }
                map.push_str(&format!("{x:.2} {y:.2} {v:.5}\n"));
            }
        }
        if let Ok(dir) = std::env::var("MAP") {
            std::fs::write(format!("{dir}/mode{i:02}_{:.0}Hz.txt", md.omega / TAU), map).unwrap();
        }
        let row: String = shapes.iter().map(|s| format!("{:7.1}", (1.0 / (s[i] * s[i])).min(9999.0))).collect();
        println!("{i:3} {:6.1} {:.3}  ({:.2}, {:.2})  {:6.2} | {row}", md.omega / TAU, sb.mode_eta(md), at.0, at.1, 1.0 / (peak * peak));
    }
    if let Ok(dir) = std::env::var("MAP") {
        let mut s = String::new();
        for k in 21u8..=108 {
            let (x, y) = p.bridge_point(k);
            s.push_str(&format!("{k} {x:.4} {y:.4}\n"));
        }
        std::fs::write(format!("{dir}/bridge.txt"), s).unwrap();
    }
    // Plath's fifteen driving points (2019, table 4.2): his keys 1, 10, 15, 20 on the bass bridge, 21 … 88 on the main one.
    let plath: [u8; 15] = [21, 30, 35, 40, 41, 43, 46, 50, 54, 59, 65, 73, 83, 94, 108];
    let mean = |k: u8, lo: f64, hi: f64| {
        let sh = sb.bridge_shapes(k);
        let n = 400;
        (0..n).map(|i| sb.bridge_admittance(&sh, TAU * (lo + (hi - lo) * (i as f64 + 0.5) / n as f64))).map(|y| y.re.hypot(y.im)).sum::<f64>() / n as f64
    };
    let all: Vec<f64> = plath.iter().map(|&k| mean(k, 50.0, 1000.0)).collect();
    println!(
        "mean |Y| over 50–1000 Hz at Plath's fifteen bridge points: {:.2e} s/kg (bass bridge {:.2e}, main {:.2e}); over 60–160 Hz on the bass bridge {:.2e}",
        all.iter().sum::<f64>() / 15.0,
        all[..4].iter().sum::<f64>() / 4.0,
        all[4..].iter().sum::<f64>() / 11.0,
        plath[..4].iter().map(|&k| mean(k, 60.0, 160.0)).sum::<f64>() / 4.0
    );
    println!("driving-point mobility at the keys' bridge points: Re Y, |Y| (10·log10, dB re 1 s/kg)");
    println!("   Hz | {}", keys.map(|k| format!("{k:13}")).join(""));
    let mut f = 25.0;
    while f < 420.0 {
        let row: String = shapes
            .iter()
            .map(|s| {
                let y = sb.bridge_admittance(s, TAU * f);
                format!(" {:6.1}{:6.1}", 10.0 * y.re.max(1e-12).log10(), 10.0 * y.re.hypot(y.im).log10())
            })
            .collect();
        println!("{f:5.1} | {row}");
        f *= 2f64.powf(1.0 / 12.0);
    }
}
