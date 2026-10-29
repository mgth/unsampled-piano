//! Opt-in research radiator. Frozen 48 kHz geometry, per-key contact models,
//! and the complete 65,536-tap causal Rayleigh responses from the offline bench.
//! A common 128-sample delay keeps action/structure and radiation aligned.
//! Everything is prepared off the audio thread; processing allocates nothing.
use crate::{
    instrument::Instrument,
    note_model::{NoteModel, Resolution},
    params::PianoModel,
};
use realfft::{ComplexToReal, RealFftPlanner, RealToComplex, num_complex::Complex64};
use std::sync::Arc;

pub const LATENCY: usize = 128;
const REGIONS: usize = 16;
const RING: usize = 16384;
const TAPS: usize = 65536;

pub struct Experiment {
    pub model: PianoModel,
    pub contacts: Vec<(u8, PianoModel)>,
    pub label: String,
    prepared: Vec<Arc<Partition>>,
}
impl std::fmt::Debug for Experiment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Experiment").field("label", &self.label).finish()
    }
}
impl Experiment {
    /// `impulses`: region, ear, tap, including microphone gain, before the
    /// engine's pressure/output scale. Reject incompatible bundles explicitly.
    pub fn new(model: PianoModel, contacts: Vec<(u8, PianoModel)>, impulses: Vec<f64>, label: String) -> Result<Self, String> {
        let resolved = model.with_macros();
        let sb = resolved.soundboard.ok_or("Rayleigh test needs a soundboard")?;
        if !sb.radiate || sb.exact_below != 0.0 || sb.string_load_free != 1.0 || sb.case_open != 1.0 {
            return Err("Rayleigh test requires radiating board, exact_below=0, string_load_free=1, case_open=1".into());
        }
        if impulses.len() != REGIONS * 2 * TAPS || impulses.iter().any(|x| !x.is_finite()) {
            return Err("Rayleigh test: expected finite 16 × 2 × 65536 impulse samples".into());
        }
        let mut seen = [false; 128];
        for (note, contact) in &contacts {
            if !(21..=108).contains(note) || seen[*note as usize] {
                return Err("invalid/duplicate contact note".into());
            }
            seen[*note as usize] = true;
            // No per-key acoustic geometry: all voices drive the same board.
            if contact.soundboard != model.soundboard || contact.room != model.room {
                return Err("per-key contact cannot change the listening setup".into());
            }
        }
        let prepared = [(128, 0, 1024), (512, 1024, 8192), (2048, 8192, TAPS)]
            .into_iter()
            .map(|(b, start, end)| Arc::new(Partition::new(&impulses, TAPS, b, start, end)))
            .collect();
        Ok(Self {
            model,
            contacts,
            label,
            prepared,
        })
    }
    pub fn instrument(&self) -> Instrument {
        let mut inst = Instrument::new(self.model, 48000.0);
        let sb = inst.model.board().expect("validated board");
        let raw = crate::board::board_modes(&sb, 48000.0);
        for &(note, source) in &self.contacts {
            let model = source.with_macros();
            let model = PianoModel {
                soundboard: model.soundboard.map(|s| s.resolved()),
                ..model
            };
            inst.table.replace(
                note,
                NoteModel::build_with(&model, Some(&sb), Some(&raw), None, note, 48000.0, Resolution::DEFAULT),
            );
        }
        inst
    }
    pub fn radiator(&self) -> Radiator {
        Radiator::new(&self.prepared)
    }
}

struct Partition {
    block: usize,
    start: usize,
    count: usize,
    bins: usize,
    filters: Vec<Complex64>, // ear, input, partition, bin
    fft: Arc<dyn RealToComplex<f64>>,
    ifft: Arc<dyn ComplexToReal<f64>>,
}
impl Partition {
    fn new(ir: &[f64], taps: usize, block: usize, start: usize, end: usize) -> Self {
        let mut planner = RealFftPlanner::<f64>::new();
        let fft = planner.plan_fft_forward(2 * block);
        let ifft = planner.plan_fft_inverse(2 * block);
        let count = (end - start).div_ceil(block);
        let bins = block + 1;
        let mut filters = vec![Complex64::default(); 2 * REGIONS * count * bins];
        let mut tmp = fft.make_input_vec();
        let mut scratch = fft.make_scratch_vec();
        for ear in 0..2 {
            for r in 0..REGIONS {
                for p in 0..count {
                    tmp.fill(0.0);
                    for (i, x) in tmp[..block].iter_mut().enumerate() {
                        let tap = start + p * block + i;
                        if tap < end {
                            *x = ir[(r * 2 + ear) * taps + tap] / (2 * block) as f64;
                        }
                    }
                    let offset = ((ear * REGIONS + r) * count + p) * bins;
                    fft.process_with_scratch(&mut tmp, &mut filters[offset..offset + bins], &mut scratch).unwrap();
                }
            }
        }
        Self {
            block,
            start,
            count,
            bins,
            filters,
            fft,
            ifft,
        }
    }
}
struct Stage {
    p: Arc<Partition>,
    input: Vec<Vec<f64>>,
    history: Vec<Complex64>, // input, partition, bin
    sum: Vec<Complex64>,
    output: Vec<f64>,
    scratch: Vec<Complex64>,
    fill: usize,
    cursor: usize,
}
impl Stage {
    fn new(p: Arc<Partition>) -> Self {
        Self {
            input: vec![vec![0.; 2 * p.block]; REGIONS],
            history: vec![Complex64::default(); REGIONS * p.count * p.bins],
            sum: p.fft.make_output_vec(),
            output: p.ifft.make_output_vec(),
            scratch: vec![Complex64::default(); p.fft.get_scratch_len().max(p.ifft.get_scratch_len())],
            p,
            fill: 0,
            cursor: 0,
        }
    }
    fn push(&mut self, force: &[f32; REGIONS], clock: usize, ring: &mut [Vec<f64>; 2]) {
        for (r, &x) in force.iter().enumerate() {
            self.input[r][self.fill] = x as f64;
        }
        self.fill += 1;
        if self.fill < self.p.block {
            return;
        }
        let p = &self.p;
        for r in 0..REGIONS {
            self.input[r][p.block..].fill(0.);
            let offset = (r * p.count + self.cursor) * p.bins;
            p.fft
                .process_with_scratch(&mut self.input[r], &mut self.history[offset..offset + p.bins], &mut self.scratch)
                .unwrap();
        }
        for (ear, out) in ring.iter_mut().enumerate() {
            self.sum.fill(Complex64::default());
            for r in 0..REGIONS {
                for k in 0..p.count {
                    let h = ((ear * REGIONS + r) * p.count + k) * p.bins;
                    let x = (r * p.count + (self.cursor + p.count - k) % p.count) * p.bins;
                    for ((y, a), b) in self.sum.iter_mut().zip(&self.history[x..x + p.bins]).zip(&p.filters[h..h + p.bins]) {
                        y.re += a.re * b.re - a.im * b.im;
                        y.im += a.re * b.im + a.im * b.re;
                    }
                }
            }
            // Roundoff must not give the real-only DC/Nyquist bins imaginary parts.
            self.sum[0].im = 0.;
            self.sum[p.bins - 1].im = 0.;
            p.ifft.process_with_scratch(&mut self.sum, &mut self.output, &mut self.scratch).unwrap();
            let first = clock + 1 + p.start + LATENCY - p.block;
            for (i, &x) in self.output.iter().enumerate() {
                out[(first + i) & (RING - 1)] += x;
            }
        }
        self.fill = 0;
        self.cursor = (self.cursor + 1) % p.count;
    }
}

pub struct Radiator {
    stages: Vec<Stage>,
    ring: [Vec<f64>; 2],
    residual: [[f64; LATENCY]; 2],
    old: [[f32; crate::engine::SUB_BLOCK]; 2],
    last: [f32; 2],
    clock: usize,
}
impl Radiator {
    fn new(prepared: &[Arc<Partition>]) -> Self {
        Self {
            stages: prepared.iter().cloned().map(Stage::new).collect(),
            ring: std::array::from_fn(|_| vec![0.; RING]),
            residual: [[0.; LATENCY]; 2],
            old: [[0.; crate::engine::SUB_BLOCK]; 2],
            last: [0.; 2],
            clock: 0,
        }
    }
    pub(crate) fn clear_capture(&mut self) {
        self.old.iter_mut().for_each(|x| x.fill(0.));
    }
    pub(crate) fn capture(&mut self, l: &[f32], r: &[f32]) {
        self.old[0][..l.len()].copy_from_slice(l);
        self.old[1][..r.len()].copy_from_slice(r);
    }
    pub(crate) fn add_capture(&mut self, l: &[f32], r: &[f32]) {
        for (old, ch) in self.old.iter_mut().zip([l, r]) {
            for (a, b) in old.iter_mut().zip(ch) {
                *a += b;
            }
        }
    }
    pub(crate) fn process(&mut self, primary: &[Vec<f32>], sympathetic: &[Vec<f32>], l: &mut [f32], r: &mut [f32], scale: f32) {
        for i in 0..l.len() {
            let force = std::array::from_fn(|k| primary[k][i] + sympathetic[k][i]);
            let idx = self.clock & (RING - 1);
            let delay = self.clock % LATENCY;
            for (ear, ch) in [&mut *l, &mut *r].into_iter().enumerate() {
                let old = (self.old[ear][i] - self.last[ear]) * 48000. * scale;
                self.last[ear] = self.old[ear][i];
                let dry = ch[i] as f64 - old as f64;
                ch[i] = (self.residual[ear][delay] + self.ring[ear][idx] * scale as f64) as f32;
                self.residual[ear][delay] = dry;
                self.ring[ear][idx] = 0.;
            }
            for stage in &mut self.stages {
                stage.push(&force, self.clock, &mut self.ring);
            }
            self.clock += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn partitioned_response_matches_direct_with_common_delay() {
        // Impulses spanning all stage boundaries, all regions, both ears.
        let mut ir = vec![0.; REGIONS * 2 * TAPS];
        let taps = [0, 1, 127, 128, 1023, 1024, 8191, 8192, 10001, 65535];
        for r in 0..REGIONS {
            for e in 0..2 {
                for &t in &taps {
                    ir[(r * 2 + e) * TAPS + t] = (r + 1) as f64 * (e + 1) as f64 * 0.001;
                }
            }
        }
        let ps = [(128, 0, 1024), (512, 1024, 8192), (2048, 8192, TAPS)].map(|(b, s, e)| Arc::new(Partition::new(&ir, TAPS, b, s, e)));
        let mut rad = Radiator::new(&ps);
        let mut maximum = 0f32;
        let mut primary = vec![vec![0.; 16]; 16];
        let sym = primary.clone();
        for start in (0..TAPS + 256).step_by(16) {
            for r in 0..16 {
                primary[r].fill(0.);
                if start == 0 {
                    primary[r][0] = (r + 1) as f32 * 0.01;
                }
            }
            let (mut l, mut r) = ([0.; 16], [0.; 16]);
            rad.process(&primary, &sym, &mut l, &mut r, 1.);
            for (e, ch) in [l, r].into_iter().enumerate() {
                for (i, x) in ch.into_iter().enumerate() {
                    let t = start + i;
                    let expected = if t >= LATENCY && taps.contains(&(t - LATENCY)) {
                        (1..=16).map(|v| (v * v) as f32 * (e + 1) as f32 * 0.00001).sum()
                    } else {
                        0.
                    };
                    maximum = maximum.max((x - expected).abs());
                }
            }
        }
        assert!(maximum < 1e-7, "{maximum}");
    }
    #[test]
    fn streaming_overlap_and_residual_keep_phase_across_arbitrary_blocks() {
        let mut ir = vec![0.; REGIONS * 2 * TAPS];
        let taps = [0, 143, 1023, 1024, 8191, 8192, 28000, 65535];
        for e in 0..2 {
            for &t in &taps {
                ir[(3 * 2 + e) * TAPS + t] = (e + 1) as f64 * 0.002;
            }
        }
        let ps = [(128, 0, 1024), (512, 1024, 8192), (2048, 8192, TAPS)].map(|(b, s, e)| Arc::new(Partition::new(&ir, TAPS, b, s, e)));
        let mut rad = Radiator::new(&ps);
        let n = TAPS + 4096;
        let input: Vec<f32> = (0..n).map(|t| if t < 3000 { ((t * 37 % 101) as f32 - 50.) * 0.01 } else { 0. }).collect();
        let residual: Vec<f32> = (0..n).map(|t| ((t * 13 % 97) as f32 - 48.) * 0.0001).collect();
        let mut primary = vec![vec![0.; 16]; 16];
        let mut sym = primary.clone();
        let mut start = 0;
        let mut maximum = 0f32;
        while start < n {
            let len = (1 + start % 16).min(n - start);
            for i in 0..len {
                primary[3][i] = input[start + i] * 0.75;
                sym[3][i] = input[start + i] * 0.25;
            }
            let mut l = residual[start..start + len].to_vec();
            let mut r = l.clone();
            rad.process(&primary, &sym, &mut l, &mut r, 1.);
            for (e, ch) in [l, r].into_iter().enumerate() {
                for (i, x) in ch.into_iter().enumerate() {
                    let time = start + i;
                    let mut expected = if time >= LATENCY { residual[time - LATENCY] } else { 0. };
                    for &tap in &taps {
                        if time >= LATENCY + tap {
                            expected += input[time - LATENCY - tap] * (e + 1) as f32 * 0.002;
                        }
                    }
                    maximum = maximum.max((expected - x).abs());
                }
            }
            start += len;
        }
        assert!(maximum < 1e-7, "streaming error {maximum}");
    }
}
