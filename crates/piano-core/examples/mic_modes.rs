//! Board modes 200–400 Hz: (m, n), frequency, shape at D2's bridge point and
//! under each close microphone (the default pair and a candidate), to see
//! which modes reach the two microphones in antiphase.
use piano_core::params::PianoModel;
fn main() {
    let model = PianoModel::default();
    let sb = model.board().unwrap();
    let (a, b) = (sb.params.a, sb.params.b);
    let pairs = [("P0", (0.294118, 0.346154), (0.705882, 0.653846)), ("P7", (0.33, 0.33), (0.60, 0.52))];
    let (xb, yb) = sb.params.bridge_point(38);
    println!("D2 bridge point ({:.2}, {:.2}) of ({a}, {b})", xb, yb);
    println!("(m,n)   Hz   bridge   P0 L     P0 R   | P7 L     P7 R");
    for md in sb.modes.iter().filter(|m| (200.0..400.0).contains(&(m.omega / std::f64::consts::TAU))) {
        let f = md.omega / std::f64::consts::TAU;
        let s = |p: (f64, f64)| sb.shape(md, p.0 * a, p.1 * b);
        let sbp = sb.shape(md, xb, yb);
        print!("({},{}) {:6.1} {:+6.2}", md.m, md.n, f, sbp);
        for (_, l, r) in &pairs {
            print!("   {:+6.2} {:+6.2} |", s(*l), s(*r));
        }
        println!();
    }
}
