// Lists the strongest spectral peaks of a file in a band (debug helper).
use piano_analysis::{inharm::onset, io, spectrum::Spectrum};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let (x, sr) = io::read_mono(a[1].as_ref(), None).unwrap();
    let (lo, hi): (f64, f64) = (a[2].parse().unwrap(), a[3].parse().unwrap());
    let on = onset(&x);
    for (skip, len) in [(0.02, 0.2), (0.3, 1.0)] {
        let s = Spectrum::new(&x, sr, on + (skip * sr) as usize, (len * sr) as usize, 8);
        let mut peaks = vec![];
        let (i0, i1) = ((lo / s.bin_hz) as usize, (hi / s.bin_hz) as usize);
        for i in i0.max(1)..i1 {
            if s.db[i] > s.db[i - 1] && s.db[i] >= s.db[i + 1] { peaks.push((s.db[i], i as f64 * s.bin_hz)); }
        }
        peaks.sort_by(|a, b| b.0.total_cmp(&a.0));
        println!("segment {skip}s+{len}s:");
        for (d, f) in peaks.iter().take(6) { println!("  {f:8.1} Hz {d:6.1} dB"); }
    }
}
