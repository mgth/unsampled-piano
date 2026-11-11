//! The built board's modes (docs/real_board.md): its frequencies against the
//! thesis's finite elements, its mass and its build time.
//!   cargo run --release -p piano-core --example board_fem [thesis|d] [step] [below]
use piano_core::fem::{BoardStructure, MAIN_BRIDGE_POINTS, BASS_BRIDGE_POINTS};
use piano_core::soundboard::{Soundboard, SoundboardParams};

/// The thesis's 3D elastodynamics of the same board (read off its Figure II.12).
const THESIS_3D: [f64; 20] = [29.0, 43.0, 64.0, 67.0, 94.0, 104.0, 120.0, 134.0, 144.0, 163.0, 180.0, 183.0, 199.0, 213.0, 229.0, 233.0, 241.0, 259.0, 277.0, 280.0];
/// Mode number against frequency at 200, 400, ... of the same figure.
/// Chladni figures on a real 2.74 m board without strings or bridge (Conklin 1996, in the thesis's Figure I.42).
const CONKLIN: [(usize, f64); 4] = [(1, 49.0), (2, 67.0), (3, 89.0), (4, 112.0)];
const THESIS_3D_HIGH: [(usize, f64); 6] = [(200, 1750.0), (400, 2700.0), (600, 3350.0), (800, 4050.0), (1000, 4750.0), (1200, 5350.0)];
/// The same figure's Reissner-Mindlin plate, the model used here.
const THESIS_RM_HIGH: [(usize, f64); 6] = [(200, 1900.0), (400, 3100.0), (600, 4000.0), (800, 4750.0), (1000, 5500.0), (1200, 6100.0)];

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let which = args.get(1).map_or("d", |s| s.as_str());
    let mut s = BoardStructure { enabled: true, ..BoardStructure::STEINWAY_D };
    if let Some(v) = args.get(2).and_then(|v| v.parse().ok()) {
        s.step = v;
    }
    if let Some(v) = args.get(3).and_then(|v| v.parse().ok()) {
        s.below = v;
    }
    if which == "thesis" {
        s.clamped = false;
        // A plate of 9 mm, ribs of 25 mm under it, a bridge of 20 mm over it.
        (s.plate_center, s.plate_edge) = (0.009, 0.009);
        for r in &mut s.ribs {
            (r.4, r.5) = (0.034, 0.034);
        }
        s.main_bridge_thickness = [(41.0, 0.029); MAIN_BRIDGE_POINTS];
        s.bass_bridge_thickness = [(21.0, 0.029); BASS_BRIDGE_POINTS];
    }
    let p = SoundboardParams {
        a: 2.124,
        b: 1.46,
        outline: [(0.0, 0.0096), (0.1149, 0.0068), (0.2373, 0.1781), (0.3362, 0.3014), (0.5621, 0.363), (0.8682, 0.411), (0.9976, 0.5753), (0.9976, 0.9315), (0.9341, 1.0), (0.0584, 1.0)],
        main_bridge_from: 41,
        treble_bridge: [(0.0443, 0.048), (0.0809, 0.202), (0.1596, 0.38), (0.3441, 0.579), (0.8023, 0.842)],
        bass_bridge: [(0.5765, 0.562), (0.8707, 0.753)],
        main_string_slant: 0.0,
        crown_freq: 0.0,
        structure: s,
        ..SoundboardParams::GRAND_2M
    };
    let t = std::time::Instant::now();
    let sb = Soundboard::new(p);
    let fem = sb.fem.as_ref().unwrap();
    println!("{} modes below {} Hz, grid {}×{}, {:.1} s; mass {:.1} kg on {:.2} m² ({:.1} kg/m²)", fem.omega.len(), s.below, fem.nx, fem.ny, t.elapsed().as_secs_f64(), fem.mass, fem.area, fem.mass / fem.area);
    let f: Vec<f64> = fem.omega.iter().map(|w| w / std::f64::consts::TAU).collect();
    // The air: flat and without crown, the frequency it takes and its share of the losses.
    let t = std::time::Instant::now();
    let air = piano_core::fem::air_on(fem, &p, 1.2, 343.0);
    println!("air in {:.1} s; mode: Hz, added mass, η of radiation; baffled board, the same", t.elapsed().as_secs_f64());
    let baffled = piano_core::fem::air_on(fem, &SoundboardParams { back_gain: 0.0, ..p }, 1.2, 343.0);
    for &i in [0usize, 1, 2, 3, 5, 10, 20, 40, 80, 120, 160, 200, 250, 300, 350].iter().filter(|&&i| i < f.len()) {
        let eta = |(m, r): (f64, f64)| r / ((1.0 + m).sqrt() * fem.omega[i]);
        println!("{:4}: {:6.0} Hz  +{:4.1} %  {:.4} | +{:4.1} %  {:.4}", i + 1, f[i], 100.0 * air[i].0, eta(air[i]), 100.0 * baffled[i].0, eta(baffled[i]));
    }
    println!("mode  ours   thesis 3D");
    for (i, t3) in THESIS_3D.iter().enumerate() {
        println!("{:4} {:6.1} {:6.0}", i + 1, f.get(i).copied().unwrap_or(0.0), t3);
    }
    let rms = (THESIS_3D.iter().zip(&f).map(|(t, o)| (o / t).ln().powi(2)).sum::<f64>() / 20.0).sqrt();
    println!("rms against 3D, 20 modes: {:.1} %", 100.0 * rms);
    for (n, c) in CONKLIN {
        println!("mode {n}: {:.1} Hz, Conklin {c:.0}", f[n - 1]);
    }
    for ((n, t3), (_, rm)) in THESIS_3D_HIGH.into_iter().zip(THESIS_RM_HIGH) {
        if let Some(o) = f.get(n - 1) {
            println!("mode {n}: {o:.0} Hz, thesis 3D {t3:.0}, Reissner-Mindlin {rm:.0}");
        }
    }
    // What a key's strings see: the board's mean conductance per third of an
    // octave, built board against the fitted plate, dB re 1 s/kg.
    let fitted = Soundboard::new(SoundboardParams {
        rho_h: 14.7,
        dx: 5737.5,
        dy: 2550.0,
        bass_bridge_bow: 0.034,
        rib_spacing: 0.11,
        rib_share: 0.5,
        rib_jitter: 0.3,
        impedance_slope_bass: -0.02,
        crown_freq: 55.0,
        structure: BoardStructure { enabled: false, ..s },
        ..p
    });
    let bands = [100.0, 200.0, 400.0, 800.0, 1600.0, 2500.0, 3500.0, 5000.0, 8000.0];
    println!("conductance, dB: key | {}", bands.map(|f| format!("{f:6.0}")).join(" "));
    for key in [24u8, 33, 40, 41, 48, 55, 60, 67, 72, 79, 84, 91, 96, 103, 108] {
        for (name, b) in [("built", &sb), ("fitted", &fitted)] {
            let sh = b.bridge_shapes(key);
            let row: Vec<String> = bands
                .iter()
                .map(|&f| {
                    let n = 64;
                    let g: f64 = (0..n)
                        .map(|i| {
                            let w = std::f64::consts::TAU * f * 2f64.powf((i as f64 / (n - 1) as f64 - 0.5) / 3.0);
                            b.mobility(&sh, w).re
                        })
                        .sum::<f64>()
                        / n as f64;
                    format!("{:6.1}", 10.0 * g.max(1e-12).log10())
                })
                .collect();
            println!("{key:3} {name:6} | {}", row.join(" "));
        }
    }
}
