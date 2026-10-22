//! The model's bridge conductance Re Y at the partials of every key, fitted
//! like tools/bridge_conductance.py fits the reference:
//! log G = a + b·log f + c·midi.
use piano_core::params::{partial_freq, PianoModel};
use piano_core::soundboard::Soundboard;

fn main() {
    let model = PianoModel::default();
    let sb = Soundboard::new(model.soundboard.unwrap());
    let (mut rows, mut ys) = (Vec::new(), Vec::new());
    for note in (21..=108).step_by(3) {
        let np = model.note(note);
        if np.strings < 2 {
            continue;
        }
        let shapes = sb.bridge_shapes(note);
        for k in 1..=12 {
            let f = partial_freq(np.f0, np.b, k);
            if f > 10_000.0 {
                break;
            }
            let g = sb.mobility(&shapes, std::f64::consts::TAU * f).re;
            if g > 0.0 {
                rows.push([1.0, f.ln(), note as f64]);
                ys.push(g.ln());
            }
        }
    }
    // Least squares by normal equations (3×3).
    let mut a = [[0.0f64; 3]; 3];
    let mut b = [0.0f64; 3];
    for (r, y) in rows.iter().zip(&ys) {
        for i in 0..3 {
            b[i] += r[i] * y;
            for j in 0..3 {
                a[i][j] += r[i] * r[j];
            }
        }
    }
    let det = |m: [[f64; 3]; 3]| {
        m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1]) - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
    };
    let d = det(a);
    let sol: Vec<f64> = (0..3)
        .map(|c| {
            let mut m = a;
            for i in 0..3 {
                m[i][c] = b[i];
            }
            det(m) / d
        })
        .collect();
    let rms = (rows.iter().zip(&ys).map(|(r, y)| (y - (sol[0] + sol[1] * r[1] + sol[2] * r[2])).powi(2)).sum::<f64>() / ys.len() as f64).sqrt();
    println!("{} partials: log G = {:.3} + {:.3}·log f + {:.4}·midi   rms {:.3}", ys.len(), sol[0], sol[1], sol[2], rms);
    for (f, m) in [(300.0, 48.0), (1000.0, 60.0), (1000.0, 84.0), (3000.0, 96.0)] {
        let g = (sol[0] + sol[1] * f64::ln(f) + sol[2] * m).exp();
        let r = (-8.836 + 0.289 * f64::ln(f) - 0.0146 * m).exp();
        println!("  f {f:5.0} Hz midi {m:3.0}: model {g:.2e}  reference {r:.2e}  ratio {:.2}", g / r);
    }
}
