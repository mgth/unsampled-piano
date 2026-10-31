//! ASCII map of one board mode (by index in frequency order among the
//! outlined board's own modes), with the main bridge's keys marked.
//! usage: mode_map <ritz index>
use piano_core::params::PianoModel;

fn main() {
    let i: u32 = std::env::args().nth(1).unwrap().parse().unwrap();
    let model = PianoModel::default();
    let sb = model.board().unwrap();
    let p = sb.params;
    let md = sb.modes.iter().find(|m| m.ritz == Some(i)).unwrap();
    println!("mode {i}: {:.1} Hz", md.omega / std::f64::consts::TAU);
    let (nx, ny) = (34, 26);
    let mut keys = std::collections::HashMap::new();
    for n in (21u8..=108).step_by(3) {
        let (x, y) = p.bridge_point(n);
        keys.insert(((x / p.a * nx as f64) as i32, (y / p.b * ny as f64) as i32), n);
    }
    let peak = (0..nx * ny).map(|c| sb.shape(md, (c % nx) as f64 / nx as f64 * p.a, (c / nx) as f64 / ny as f64 * p.b).abs()).fold(0.0, f64::max);
    for j in 0..ny {
        let row: String = (0..nx)
            .map(|k| {
                let (x, y) = ((k as f64 + 0.5) / nx as f64 * p.a, (j as f64 + 0.5) / ny as f64 * p.b);
                if keys.contains_key(&(k, j)) {
                    return '#';
                }
                if let Some(o) = &sb.outline {
                    if !o.contains(x, y) {
                        return ' ';
                    }
                }
                let v = sb.shape(md, x, y) / peak;
                [' ', '-', '=', '+', '*', '@'][0.max(((v.abs() * 5.0) as usize).min(5))].to_string().chars().next().map(|c| if v < 0.0 && c != ' ' { c.to_ascii_lowercase() } else { c }).unwrap()
            })
            .collect();
        println!("|{row}|");
    }
    for n in [57u8, 60, 62, 63, 64, 65, 69] {
        let (x, y) = p.bridge_point(n);
        println!("key {n}: ({x:.2}, {y:.2}) Φ {:.3}", sb.shape(md, x, y));
    }
}
