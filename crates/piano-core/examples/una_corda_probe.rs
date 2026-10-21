//! Level and brightness of C4 with and without una corda.
use piano_core::{Engine, EngineConfig, Event, TimedEvent};
fn main() {
    for (soft, softening) in [(0u8, 0.0), (127, 0.35), (127, 0.6), (127, 0.8)] {
        let mut cfg = EngineConfig::default();
        cfg.model.una_corda_softening = softening;
        let mut e = Engine::new(cfg);
        let mut out = vec![0.0f32; 128];
        let mut all = Vec::new();
        for b in 0..200 {
            let mut ev = vec![];
            if b == 0 {
                ev.push(TimedEvent { frame: 0, event: Event::Control { cc: 67, value: soft } });
                ev.push(TimedEvent { frame: 1, event: Event::NoteOn { note: 60, velocity: 80 } });
            }
            e.process(&ev, &mut out);
            all.extend_from_slice(&out);
        }
        let x: Vec<f64> = all.iter().map(|v| *v as f64).collect();
        let rms = |a: usize, b: usize| 10.0 * (x[a..b].iter().map(|v| v * v).sum::<f64>() / (b - a) as f64).log10();
        // Spectral centroid of the first 100 ms.
        let n = 4800;
        let mut num = 0.0;
        let mut den = 0.0;
        for k in 1..n / 2 {
            let (mut re, mut im) = (0.0, 0.0);
            if k % 4 != 0 { continue; }
            for (i, v) in x[..n].iter().enumerate() {
                let ph = std::f64::consts::TAU * (k * i) as f64 / n as f64;
                re += v * ph.cos();
                im -= v * ph.sin();
            }
            let p = re * re + im * im;
            num += k as f64 * 10.0 * p;
            den += p;
        }
        println!("una corda {soft:3} softening {softening:.2}: level 0–0.1 s {:6.1} dB, 0.1–0.5 s {:6.1} dB, centroid {:5.0} Hz", rms(0, 4800), rms(4800, 24000), num / den);
    }
}
