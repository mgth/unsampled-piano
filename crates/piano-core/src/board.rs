//! The soundboard as one multi-input, multi-output modal bank.
//!
//! Inputs: the bridge force of each of the [`REGIONS`] bridge regions.
//! Outputs: the pressure at the two microphones, and the bridge velocity at
//! each region point (its time derivative drives the strings
//! sympathetically: a moving end forces string mode k with −(μL/kπ)·ü).
//! Each board mode n is a complex one-pole driven by Σ_r Φ_n(x_r)·T·F_r and
//! read through complex weights: radiation for the mics, Φ_n(x_p) for the
//! velocity at point p (exact at all frequencies, unlike iω_n·Φ_n for the
//! acceleration, which only holds near each mode's resonance). One bank
//! replaces one bank per region.

// Dense per-lane numeric code reads better with explicit indices.
#![allow(clippy::needless_range_loop)]

use crate::modal::{Isa, LANES};
use crate::simd::V16;
#[cfg(target_arch = "x86_64")]
use crate::simd;

pub const REGIONS: usize = 16;
pub const MICS: usize = 2;
/// Outputs: the mics, then the bridge velocity at each region point.
pub const OUTPUTS: usize = MICS + REGIONS;

/// One board mode, in physical units.
#[derive(Clone, Copy, Debug)]
pub struct BoardMode {
    pub freq_hz: f64,
    pub sigma: f64,
    /// The damped strings' share of `sigma` (1/s), which the dampers'
    /// engagement scales while playing ([`BoardBank::set_load`]).
    pub load: f64,
    /// Input gain from each region (Φ_n(x_r)·T).
    pub inputs: [f64; REGIONS],
    /// Complex output weights: mics, then region displacements.
    pub outputs: [(f64, f64); OUTPUTS],
}

#[derive(Clone, Copy)]
#[repr(C, align(64))]
struct Chunk {
    pr: [f32; LANES],
    pi: [f32; LANES],
    phi: [[f32; LANES]; REGIONS],
    wr: [[f32; LANES]; OUTPUTS],
    wi: [[f32; LANES]; OUTPUTS],
    zr: [f32; LANES],
    zi: [f32; LANES],
}

impl Default for Chunk {
    fn default() -> Self {
        Self {
            pr: [0.0; LANES],
            pi: [0.0; LANES],
            phi: [[0.0; LANES]; REGIONS],
            wr: [[0.0; LANES]; OUTPUTS],
            wi: [[0.0; LANES]; OUTPUTS],
            zr: [0.0; LANES],
            zi: [0.0; LANES],
        }
    }
}

/// The modes are split in this many parts, computed one after the other or
/// on several threads ([`BoardBank::split`]): the same result either way.
pub const PARTS: usize = 2;

/// One part of a bank's modes, with its own working memory.
pub struct BoardPart {
    chunks: Vec<Chunk>,
    /// Index of the first chunk in the bank.
    c0: usize,
    /// Per output and sample: lane sums over the part's chunks, index
    /// `output·max_block + t` (the extra outputs after the [`OUTPUTS`] ones).
    lanes: Vec<[f32; LANES]>,
    /// State history of one block of [`BLOCK`] chunks over the block, index
    /// `chunk·max_block + t`.
    hist: Vec<Hist>,
    isa: Isa,
}

pub struct BoardBank {
    parts: [BoardPart; PARTS],
    n_modes: usize,
    /// Summed outputs, index `output·max_block + t`.
    sums: Vec<f32>,
    /// Direct feedthrough of one output over the block.
    feed: Vec<f32>,
    max_block: usize,
    /// This block: its length, the regions with input, outputs computed.
    block_len: usize,
    active: [usize; REGIONS],
    na: usize,
    no: usize,
    n_ex: usize,
    /// Inputs transposed per sample, only the active regions.
    xin: Vec<[f32; REGIONS]>,
    /// Direct feedthrough correcting impulse invariance (index output, region):
    /// a force impulse makes the velocity jump; the sampled response counts
    /// the jump fully at n = 0 instead of half.
    direct: [[f32; REGIONS]; OUTPUTS],
    /// Extra outputs computed on the first `extra_chunks` chunks only (the
    /// low modes): weights `[c·n_extra + e]` (re, im).
    /// They carry no direct term: meant for displacement-like outputs,
    /// which do not jump on an impulse.
    n_extra: usize,
    extra_chunks: usize,
    extra_w: Vec<[[f32; LANES]; 2]>,
    isa: Isa,
    /// fs², for [`Self::energy`].
    fs2: f32,
    /// Per mode: damping without the strings' load, the load (1/s), and
    /// cos/sin of ω·T, to move the poles with the dampers; sample period;
    /// the load scale the poles hold.
    poles: Vec<(f64, f64, f64, f64)>,
    t: f64,
    load_scale: f64,
}

impl BoardBank {
    pub fn new(sample_rate: f64, modes: &[BoardMode], max_block: usize) -> Self {
        Self::with_extra(sample_rate, modes, &[], max_block)
    }

    /// Bank with extra outputs: `extra[i]` holds the complex weights of mode
    /// `i` on each extra output (all the same length); modes past
    /// `extra.len()` do not contribute to them.
    pub fn with_extra(sample_rate: f64, modes: &[BoardMode], extra: &[Vec<(f64, f64)>], max_block: usize) -> Self {
        let t = 1.0 / sample_rate;
        let n_extra = extra.first().map_or(0, |e| e.len());
        assert!(n_extra <= 64, "at most 64 extra outputs");
        let extra_chunks = if n_extra == 0 { 0 } else { extra.len().min(modes.len()).div_ceil(LANES) };
        let mut extra_w = vec![[[0.0f32; LANES]; 2]; extra_chunks * n_extra];
        for (i, ws) in extra.iter().take(modes.len()).enumerate() {
            for (e, w) in ws.iter().enumerate() {
                let cell = &mut extra_w[(i / LANES) * n_extra + e];
                cell[0][i % LANES] = w.0 as f32;
                cell[1][i % LANES] = w.1 as f32;
            }
        }
        let mut chunks = vec![Chunk::default(); modes.len().div_ceil(LANES)];
        let isa = Isa::detect();
        let mut direct = [[0.0f64; REGIONS]; OUTPUTS];
        for m in modes {
            for (o, row) in direct.iter_mut().enumerate() {
                for (r, d) in row.iter_mut().enumerate() {
                    *d -= 0.5 * m.outputs[o].0 * m.inputs[r];
                }
            }
        }
        for (i, m) in modes.iter().enumerate() {
            let c = &mut chunks[i / LANES];
            let l = i % LANES;
            let r = (-m.sigma * t).exp();
            let (s, co) = (std::f64::consts::TAU * m.freq_hz * t).sin_cos();
            c.pr[l] = (r * co) as f32;
            c.pi[l] = (r * s) as f32;
            for (k, g) in m.inputs.iter().enumerate() {
                c.phi[k][l] = *g as f32;
            }
            for (o, w) in m.outputs.iter().enumerate() {
                c.wr[o][l] = w.0 as f32;
                c.wi[o][l] = w.1 as f32;
            }
        }
        // Low modes first: the extra outputs live in the first part.
        let half = chunks.len().div_ceil(PARTS);
        let mut rest = chunks;
        let parts = std::array::from_fn(|p| {
            let tail = rest.split_off(half.min(rest.len()));
            BoardPart {
                chunks: std::mem::replace(&mut rest, tail),
                c0: p * half,
                lanes: vec![[0.0; LANES]; (OUTPUTS + n_extra) * max_block],
                hist: vec![([0.0; LANES], [0.0; LANES]); BLOCK * max_block],
                isa,
            }
        });
        Self {
            parts,
            n_modes: modes.len(),
            sums: vec![0.0; (OUTPUTS + n_extra) * max_block],
            feed: vec![0.0; max_block],
            max_block,
            block_len: 0,
            active: [0; REGIONS],
            na: 0,
            no: 0,
            n_ex: 0,
            xin: vec![[0.0; REGIONS]; max_block],
            direct: direct.map(|row| row.map(|d| d as f32)),
            n_extra,
            extra_chunks,
            extra_w,
            isa,
            fs2: (sample_rate * sample_rate) as f32,
            poles: modes
                .iter()
                .map(|m| {
                    let (s, c) = (std::f64::consts::TAU * m.freq_hz * t).sin_cos();
                    (m.sigma - m.load, m.load, c, s)
                })
                .collect(),
            t,
            load_scale: 1.0,
        }
    }

    /// Scales the damped strings' load on every mode: 1 with every damper
    /// on its strings (as built), 0 with all of them lifted.
    pub fn set_load(&mut self, scale: f64) {
        if scale == self.load_scale {
            return;
        }
        self.load_scale = scale;
        for part in &mut self.parts {
            for (j, c) in part.chunks.iter_mut().enumerate() {
                for l in 0..LANES {
                    let Some(&(own, load, co, s)) = self.poles.get((part.c0 + j) * LANES + l) else { break };
                    if load == 0.0 {
                        continue;
                    }
                    let r = (-(own + scale * load) * self.t).exp();
                    c.pr[l] = (r * co) as f32;
                    c.pi[l] = (r * s) as f32;
                }
            }
        }
    }

    pub fn len(&self) -> usize {
        self.n_modes
    }

    pub fn extra_outputs(&self) -> usize {
        self.n_extra
    }

    pub fn set_isa(&mut self, isa: Isa) {
        self.isa = isa;
        for p in &mut self.parts {
            p.isa = isa;
        }
    }

    pub fn is_empty(&self) -> bool {
        self.n_modes == 0
    }

    /// Output-weighted energy at the mics, to let the board sleep. The mic
    /// weights are acceleration weights ([`to_acceleration_weights`]): the
    /// differentiator's gain |1 − p|·fs at each mode restores the pressure.
    pub fn energy(&self) -> f32 {
        let mut e = 0.0;
        for c in self.parts.iter().flat_map(|p| &p.chunks) {
            for l in 0..LANES {
                let w = c.wr[0][l] * c.wr[0][l] + c.wi[0][l] * c.wi[0][l];
                let d = (1.0 - c.pr[l]) * (1.0 - c.pr[l]) + c.pi[l] * c.pi[l];
                e += w * d * (c.zr[l] * c.zr[l] + c.zi[l] * c.zi[l]);
            }
        }
        e * self.fs2
    }

    /// Modulus of each mode's state, in the order of the modes given, into
    /// `out` (as many as fit); returns how many were written.
    pub fn amplitudes(&self, out: &mut [f32]) -> usize {
        let n = self.n_modes.min(out.len());
        for part in &self.parts {
            // Only the chunks holding the modes asked for.
            let used = n.div_ceil(LANES).saturating_sub(part.c0).min(part.chunks.len());
            for (ci, c) in part.chunks[..used].iter().enumerate() {
                for l in 0..LANES {
                    let i = (part.c0 + ci) * LANES + l;
                    if i < n {
                        out[i] = c.zr[l].hypot(c.zi[l]);
                    }
                }
            }
        }
        n
    }

    /// Advances by `len` samples. `inputs[r][..len]` are the region forces;
    /// the outputs are *added* to `mics[o][..len]` and, when given,
    /// `disp[r][..len]` (bridge velocity at the region points) and
    /// `extra[e][..len]`.
    pub fn process(
        &mut self,
        inputs: &[Vec<f32>],
        len: usize,
        mics: [&mut [f32]; MICS],
        disp: Option<&mut [Vec<f32>]>,
        extra: Option<&mut [Vec<f32>]>,
    ) {
        self.prepare(inputs, len, disp.is_some(), extra.is_some());
        let (job, parts) = self.split();
        for part in parts {
            part.run(&job);
        }
        self.finish(mics, disp, extra);
    }

    /// [`Self::process`] in three steps, to run the parts in parallel:
    /// `prepare`, then [`BoardPart::run`] on each part of [`Self::split`],
    /// then `finish`.
    pub fn prepare(&mut self, inputs: &[Vec<f32>], len: usize, disp: bool, extra: bool) {
        assert!(len <= self.max_block && inputs.len() == REGIONS);
        // Only regions with input this block take part in the projection.
        self.na = 0;
        for (r, x) in inputs.iter().enumerate() {
            if x[..len].iter().any(|&v| v != 0.0) {
                self.active[self.na] = r;
                self.na += 1;
            }
        }
        for (t, xi) in self.xin[..len].iter_mut().enumerate() {
            for (a, &r) in self.active[..self.na].iter().enumerate() {
                xi[a] = inputs[r][t];
            }
        }
        // Outputs computed: the mics, the bridge velocities when asked for
        // (then the extra outputs).
        self.block_len = len;
        self.no = if disp { OUTPUTS } else { MICS };
        self.n_ex = if extra { self.n_extra } else { 0 };
    }

    /// The block's work, shared by the parts, and the parts.
    pub fn split(&mut self) -> (BoardJob<'_>, &mut [BoardPart; PARTS]) {
        let job = BoardJob {
            xin: &self.xin[..self.block_len],
            act: &self.active[..self.na],
            mb: self.max_block,
            len: self.block_len,
            outs: &ALL_OUTPUTS[..self.no],
            n_ex: self.n_ex,
            ex_w: &self.extra_w[..self.extra_chunks * self.n_ex],
            ex_chunks: self.extra_chunks,
        };
        (job, &mut self.parts)
    }

    /// Adds the outputs of the parts, run since [`Self::prepare`].
    pub fn finish(&mut self, mics: [&mut [f32]; MICS], disp: Option<&mut [Vec<f32>]>, extra: Option<&mut [Vec<f32>]>) {
        let (mb, len) = (self.max_block, self.block_len);
        let outs = ALL_OUTPUTS[..self.no].iter().chain(&EXTRA[..self.n_ex]);
        // Lane by lane over the parts in order, then the lanes as a tree.
        for &o in outs {
            for t in 0..len {
                let mut a = [0.0f32; LANES];
                for p in self.parts.iter().filter(|p| !p.chunks.is_empty() && (o < OUTPUTS || p.c0 < self.extra_chunks)) {
                    let l = &p.lanes[o * mb + t];
                    for k in 0..LANES {
                        a[k] += l[k];
                    }
                }
                self.sums[o * mb + t] = hsum(&a);
            }
        }
        let [m0, m1] = mics;
        let act = &self.active[..self.na];
        // Output plus direct feedthrough Σ_k direct·x_k (k in order, for
        // every sample at once: vectorises).
        let (xin, direct, sums, feed) = (&self.xin[..len], &self.direct, &self.sums, &mut self.feed[..len]);
        let mut add = |o: usize, out: &mut [f32]| {
            feed.fill(0.0);
            for (k, &r) in act.iter().enumerate() {
                let d = direct[o][r];
                for (f, x) in feed.iter_mut().zip(xin) {
                    *f += d * x[k];
                }
            }
            for ((y, s), f) in out[..len].iter_mut().zip(&sums[o * mb..]).zip(feed.iter()) {
                *y += s + f;
            }
        };
        add(0, m0);
        add(1, m1);
        if let Some(disp) = disp {
            for (r, d) in disp.iter_mut().enumerate() {
                add(MICS + r, d);
            }
        }
        if let Some(extra) = extra {
            for (e, out) in extra.iter_mut().take(self.n_extra).enumerate() {
                for (x, s) in out[..len].iter_mut().zip(&self.sums[(OUTPUTS + e) * mb..]) {
                    *x += s;
                }
            }
        }
    }
}

impl BoardPart {
    /// Advances this part's modes over the block and computes its lane sums.
    pub fn run(&mut self, job: &BoardJob) {
        let mut st = State { chunks: &mut self.chunks[..], c0: self.c0, hist: &mut self.hist[..], lanes: &mut self.lanes[..] };
        match self.isa {
            // aarch64: NEON is the baseline (32 registers: three outputs a tile).
            #[cfg(target_arch = "aarch64")]
            Isa::Generic => kernel::<crate::simd::arm::N, 1, 3, 1>(job, &mut st),
            #[cfg(not(target_arch = "aarch64"))]
            Isa::Generic => kernel::<[f32; LANES], 1, 1, 1>(job, &mut st),
            // SAFETY: only selectable when detected.
            #[cfg(target_arch = "x86_64")]
            Isa::Avx2 => unsafe { kernel_avx2(job, &mut st) },
            #[cfg(target_arch = "x86_64")]
            Isa::Avx512 => unsafe { kernel_avx512(job, &mut st) },
        }
    }
}

/// Indices of the outputs.
const ALL_OUTPUTS: [usize; OUTPUTS] = {
    let mut o = [0; OUTPUTS];
    let mut i = 0;
    while i < OUTPUTS {
        o[i] = i;
        i += 1;
    }
    o
};

type Hist = ([f32; LANES], [f32; LANES]);

/// Extra outputs' weights of the low chunks, `[c·n_ex + e]`.
type ExtraW = [[f32; LANES]; 2];

/// What one [`BoardBank::process`] computes, shared by the parts.
pub struct BoardJob<'a> {
    xin: &'a [[f32; REGIONS]],
    act: &'a [usize],
    mb: usize,
    len: usize,
    /// Outputs to compute (indices into [`OUTPUTS`]).
    outs: &'a [usize],
    n_ex: usize,
    ex_w: &'a [ExtraW],
    ex_chunks: usize,
}

/// Indices of the extra outputs.
const EXTRA: [usize; 64] = {
    let mut e = [0; 64];
    let mut i = 0;
    while i < 64 {
        e[i] = OUTPUTS + i;
        i += 1;
    }
    e
};

/// Chunks per pass of the kernel: their state history over a sub-block
/// (4·64·128 B = 32 KB) stays in L1.
const BLOCK: usize = 4;

struct State<'a> {
    chunks: &'a mut [Chunk],
    /// Index of the first chunk in the bank (for the extra outputs).
    c0: usize,
    hist: &'a mut [Hist],
    lanes: &'a mut [[f32; LANES]],
}


#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
fn kernel_avx2(job: &BoardJob, st: &mut State) {
    kernel::<simd::x86::Y, 2, 1, 4>(job, st)
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx512f")]
fn kernel_avx512(job: &BoardJob, st: &mut State) {
    kernel::<simd::x86::Z, 4, 3, 4>(job, st)
}

/// Sum of the lanes as a tree (independent adds, not a serial chain).
#[inline(always)]
fn hsum(a: &[f32; LANES]) -> f32 {
    let mut v = *a;
    let mut w = LANES / 2;
    while w > 0 {
        for l in 0..w {
            v[l] += v[l + w];
        }
        w /= 2;
    }
    v[0]
}

/// Per block of [`BLOCK`] chunks: their recursion over the sub-block (`G`
/// chunks interleaved, to hide the recursion's latency; states kept in
/// `hist`), then the outputs, `OT` outputs × `TT` samples at a time with
/// their lane sums in registers across the block's chunks. Each output sums
/// its chunks in order, lane by lane, then its lanes as a tree: the same
/// result for any G, tile or ISA.
#[inline(always)]
fn kernel<V: V16, const G: usize, const OT: usize, const TT: usize>(job: &BoardJob, st: &mut State) {
    let (mb, len) = (job.mb, job.len);
    let (ex_w, n_ex) = (job.ex_w, job.n_ex);
    let ex = &EXTRA[..n_ex];
    for (bi, blk) in st.chunks.chunks_mut(BLOCK).enumerate() {
        let c0 = st.c0 + bi * BLOCK;
        let (grps, rest) = blk.as_chunks_mut::<G>();
        for (gi, grp) in grps.iter_mut().enumerate() {
            recurse::<V, G>(grp, job, &mut st.hist[gi * G * mb..]);
        }
        for (k, c) in rest.iter_mut().enumerate() {
            recurse::<V, 1>(std::array::from_mut(c), job, &mut st.hist[(grps.len() * G + k) * mb..]);
        }
        let (blk, hist) = (&*blk, &*st.hist);
        outputs::<V, OT, TT>(job.outs, blk.len(), bi == 0, mb, len, hist, st.lanes, |c, o| (&blk[c].wr[o], &blk[c].wi[o]));
        let n = job.ex_chunks.saturating_sub(c0).min(blk.len());
        if n > 0 {
            outputs::<V, OT, TT>(ex, n, bi == 0, mb, len, hist, st.lanes, |c, o| {
                let w = &ex_w[(c0 + c) * n_ex + o - OUTPUTS];
                (&w[0], &w[1])
            });
        }
    }
}

/// Advances `G` chunks over the sub-block, states into `hist[g·mb + t]`.
#[inline(always)]
fn recurse<V: V16, const G: usize>(grp: &mut [Chunk; G], job: &BoardJob, hist: &mut [Hist]) {
    let mb = job.mb;
    let (mut pr, mut pi, mut zr, mut zi) = ([V::zero(); G], [V::zero(); G], [V::zero(); G], [V::zero(); G]);
    for g in 0..G {
        pr[g] = V::ld(&grp[g].pr);
        pi[g] = V::ld(&grp[g].pi);
        zr[g] = V::ld(&grp[g].zr);
        zi[g] = V::ld(&grp[g].zi);
    }
    for (t, x) in job.xin.iter().enumerate() {
        let mut u = [V::zero(); G];
        for (k, &r) in job.act.iter().enumerate() {
            let xs = V::splat(x[k]);
            for g in 0..G {
                u[g] = u[g].add(V::ld(&grp[g].phi[r]).mul(xs));
            }
        }
        for g in 0..G {
            let r = pr[g].mul(zr[g]).sub(pi[g].mul(zi[g])).add(u[g]);
            let i = pr[g].mul(zi[g]).add(pi[g].mul(zr[g]));
            zr[g] = r;
            zi[g] = i;
            let h = &mut hist[g * mb + t];
            r.st(&mut h.0);
            i.st(&mut h.1);
        }
    }
    for g in 0..G {
        zr[g].st(&mut grp[g].zr);
        zi[g].st(&mut grp[g].zi);
    }
}

/// Outputs `outs` over the block's chunks `0..nc` into the lane sums
/// (starting from zero on the first block), by tiles.
#[allow(clippy::too_many_arguments)]
#[inline(always)]
fn outputs<'w, V: V16, const OT: usize, const TT: usize>(
    outs: &[usize],
    nc: usize,
    first: bool,
    mb: usize,
    len: usize,
    hist: &[Hist],
    lanes: &mut [[f32; LANES]],
    w: impl Fn(usize, usize) -> (&'w [f32; LANES], &'w [f32; LANES]) + Copy,
) {
    let mut t0 = 0;
    while t0 + TT <= len {
        out_tiles::<V, OT, TT>(outs, nc, first, mb, t0, hist, lanes, w);
        t0 += TT;
    }
    while t0 < len {
        out_tiles::<V, OT, 1>(outs, nc, first, mb, t0, hist, lanes, w);
        t0 += 1;
    }
}

#[allow(clippy::too_many_arguments)]
#[inline(always)]
fn out_tiles<'w, V: V16, const OT: usize, const TT: usize>(
    outs: &[usize],
    nc: usize,
    first: bool,
    mb: usize,
    t0: usize,
    hist: &[Hist],
    lanes: &mut [[f32; LANES]],
    w: impl Fn(usize, usize) -> (&'w [f32; LANES], &'w [f32; LANES]) + Copy,
) {
    let mut i = 0;
    while i + OT <= outs.len() {
        tile::<V, OT, TT>(&outs[i..i + OT], nc, first, mb, t0, hist, lanes, w);
        i += OT;
    }
    while i < outs.len() {
        tile::<V, 1, TT>(&outs[i..i + 1], nc, first, mb, t0, hist, lanes, w);
        i += 1;
    }
}

#[allow(clippy::too_many_arguments)]
#[inline(always)]
fn tile<'w, V: V16, const OT: usize, const TT: usize>(
    outs: &[usize],
    nc: usize,
    first: bool,
    mb: usize,
    t0: usize,
    hist: &[Hist],
    lanes: &mut [[f32; LANES]],
    w: impl Fn(usize, usize) -> (&'w [f32; LANES], &'w [f32; LANES]),
) {
    // Plain loops: `array::from_fn` is not always inlined here.
    let mut acc = [[V::zero(); TT]; OT];
    if !first {
        for j in 0..OT {
            for k in 0..TT {
                acc[j][k] = V::ld(&lanes[outs[j] * mb + t0 + k]);
            }
        }
    }
    for c in 0..nc {
        let h = &hist[c * mb + t0..c * mb + t0 + TT];
        let (mut hr, mut hi) = ([V::zero(); TT], [V::zero(); TT]);
        for k in 0..TT {
            hr[k] = V::ld(&h[k].0);
            hi[k] = V::ld(&h[k].1);
        }
        for j in 0..OT {
            let (wr, wi) = w(c, outs[j]);
            let (wr, wi) = (V::ld(wr), V::ld(wi));
            for k in 0..TT {
                acc[j][k] = acc[j][k].add(wr.mul(hr[k]).sub(wi.mul(hi[k])));
            }
        }
    }
    for j in 0..OT {
        for k in 0..TT {
            acc[j][k].st(&mut lanes[outs[j] * mb + t0 + k]);
        }
    }
}

/// Transfer at angular frequency ω (rad/s) from region input `r` to output
/// `o`, as the bank renders a real sinusoid: x(t) = cos ωt gives
/// y(t) = Re(H·e^{iωt}). Includes the counter-rotating part of each pole.
pub fn transfer(modes: &[BoardMode], r: usize, o: usize, omega: f64, sample_rate: f64) -> crate::complex::C64 {
    transfer_with(modes, |_, m| m.inputs[r], o, omega, sample_rate)
}

/// [`transfer`] with each mode's input gain given by `input` (index, mode).
pub fn transfer_with(modes: &[BoardMode], input: impl Fn(usize, &BoardMode) -> f64, o: usize, omega: f64, sample_rate: f64) -> crate::complex::C64 {
    use crate::complex::C64;
    let t = 1.0 / sample_rate;
    let e_neg = C64::polar(1.0, -omega * t);
    let e_pos = C64::polar(1.0, omega * t);
    let mut h = C64::ZERO;
    for (i, m) in modes.iter().enumerate() {
        let p = C64::polar((-m.sigma * t).exp(), std::f64::consts::TAU * m.freq_hz * t);
        let g = input(i, m);
        let b = C64::new(g, 0.0);
        let w = C64::new(m.outputs[o].0, m.outputs[o].1);
        let a = b / (C64::ONE - p * e_neg).scale(2.0);
        let bb = b / (C64::ONE - p * e_pos).scale(2.0);
        h += w * a + (w * bb).conj();
        // Same direct-term correction as the bank.
        h += C64::new(-0.5 * m.outputs[o].0 * g, 0.0);
    }
    h
}

/// Every key driving the board's low modes at its own bridge point, not
/// at its region's: one region per ≈ 5.5 keys made the board's answer jump
/// from key to key (docs/evenness.md). Below 1 kHz the keys' drive shapes
/// span few dimensions: their first 16 principal profiles keep 99.98 % of
/// them (examples/board_basis.rs). The low modes then run in a bank of
/// their own whose 16 inputs are these profiles, each key feeding them
/// with its own weights; the other modes keep the regions.
/// Profiles kept (of at most [`REGIONS`] inputs).
pub const PROFILES: usize = REGIONS;

#[derive(Clone, Debug)]
pub struct LowDrive {
    /// The first `n` board modes (those below the soundboard's
    /// `exact_below`).
    pub n: usize,
    /// Per low mode: its gain from each profile (profile · T).
    pub inputs: Vec<[f64; REGIONS]>,
    /// Per key A0..C8: its weight on each profile.
    pub keys: Vec<[f64; REGIONS]>,
}

impl LowDrive {
    /// From the soundboard and its rendered modes (sorted by frequency,
    /// [`board_modes`]); None when no mode lies below `f_max`.
    pub fn new(sb: &crate::soundboard::Soundboard, modes: &[BoardMode], f_max: f64, sample_rate: f64) -> Option<Self> {
        let n = modes.iter().take_while(|m| m.freq_hz < f_max).count();
        if n == 0 {
            return None;
        }
        let shapes: Vec<Vec<f64>> = crate::par::map(88, |k| sb.bridge_shapes(21 + k as u8)[..n].to_vec());
        let mut gram = vec![0.0; n * n];
        for s in &shapes {
            for i in 0..n {
                for j in 0..n {
                    gram[i * n + j] += s[i] * s[j];
                }
            }
        }
        let (_, mut profiles) = crate::eigen::symmetric(&gram, n);
        profiles.truncate(PROFILES);
        let t = 1.0 / sample_rate;
        let inputs = (0..n).map(|i| std::array::from_fn(|j| profiles.get(j).map_or(0.0, |v| v[i] * t))).collect();
        let keys = shapes
            .iter()
            .map(|s| std::array::from_fn(|j| profiles.get(j).map_or(0.0, |v| v.iter().zip(s).map(|(a, b)| a * b).sum())))
            .collect();
        Some(Self { n, inputs, keys })
    }

    /// The low modes as the bank of profiles runs them.
    pub fn modes(&self, modes: &[BoardMode]) -> Vec<BoardMode> {
        modes[..self.n].iter().zip(&self.inputs).map(|(m, i)| BoardMode { inputs: *i, ..*m }).collect()
    }

    /// A key's weights on the profiles.
    pub fn key(&self, note: u8) -> [f64; REGIONS] {
        self.keys[(note.clamp(21, 108) - 21) as usize]
    }
}

/// Response of the differentiator the engine applies to the summed mic
/// outputs, y[n] = (x[n] − x[n−1])·fs, at ω (rad/s).
pub fn mic_differentiator(omega: f64, sample_rate: f64) -> crate::complex::C64 {
    (crate::complex::C64::ONE - crate::complex::C64::polar(1.0, -omega / sample_rate)).scale(sample_rate)
}

/// The radiated pressure follows the surface *acceleration* (Rayleigh
/// integral, p = ρ/2π·∫ v̇(t − R/c)/R dS), while the mic weights are
/// evaluated once per mode at its own frequency. Rendering them on the
/// modal velocity made every mode radiate at its resonance's rate even when
/// driven far below it: under a bass fundamental, the quasi-static motion
/// of the modes above radiated ×ω_n/ω too much (+20–30 dB below 50 Hz,
/// docs/bass_radiation.md). The weights are divided by the engine's
/// differentiator at the mode frequency: unchanged at resonance, ∝ ω below.
pub fn to_acceleration_weights(modes: &mut [BoardMode], sample_rate: f64) {
    for m in modes {
        let d = mic_differentiator(std::f64::consts::TAU * m.freq_hz, sample_rate);
        for o in &mut m.outputs[..MICS] {
            let w = crate::complex::C64::new(o.0, o.1) / d;
            *o = (w.re, w.im);
        }
    }
}

/// Transfer to a mic output as rendered (acceleration weights followed by
/// the engine's differentiator), in pressure per unit force.
pub fn mic_transfer(modes: &[BoardMode], r: usize, mic: usize, omega: f64, sample_rate: f64) -> crate::complex::C64 {
    transfer(modes, r, mic, omega, sample_rate) * mic_differentiator(omega, sample_rate)
}

/// Board modes for a soundboard: inputs through the bridge at the region
/// keys, outputs at the two mics (Rayleigh) and bridge velocity there.
pub fn board_modes(sb: &crate::soundboard::Soundboard, sample_rate: f64) -> Vec<BoardMode> {
    let shapes: Vec<Vec<f64>> = (0..REGIONS).map(|r| sb.bridge_shapes(region_center_note(r))).collect();
    board_modes_with(sb, &shapes, sample_rate)
}

/// Same as [`board_modes`] with point inputs at `points` (m).
pub fn board_modes_at(sb: &crate::soundboard::Soundboard, points: &[(f64, f64)], sample_rate: f64) -> Vec<BoardMode> {
    let shapes: Vec<Vec<f64>> =
        points.iter().map(|&(x, y)| sb.modes.iter().map(|md| sb.shape(md, x, y)).collect()).collect();
    board_modes_with(sb, &shapes, sample_rate)
}

/// Board modes with per-region input shapes `shapes[r][mode]`.
fn board_modes_with(sb: &crate::soundboard::Soundboard, shapes: &[Vec<f64>], sample_rate: f64) -> Vec<BoardMode> {
    use crate::complex::C64;
    let rad = sb.radiation();
    let nyq = 0.45 * sample_rate;
    let t = 1.0 / sample_rate;
    sb.modes
        .iter()
        .zip(&rad)
        .enumerate()
        .filter(|(_, (md, _))| md.omega / std::f64::consts::TAU < nyq)
        .map(|(i, (md, w))| {
            let mut outputs = [(0.0, 0.0); OUTPUTS];
            outputs[0] = (w[0].re, w[0].im);
            outputs[1] = (w[1].re, w[1].im);
            for (r, sh) in shapes.iter().enumerate() {
                outputs[MICS + r] = (sh[i], 0.0);
            }
            // Exact velocity impulse response of a damped mode,
            // e^{−σt}(cos ω_d t − σ/ω_d·sin ω_d t) = Re((1 + iσ/ω_d)·e^{(−σ+iω_d)t}):
            // the factor matters for the strongly damped high modes.
            let sigma = sb.sigma(md);
            let shape = C64::new(1.0, sigma / md.omega);
            for o in outputs.iter_mut() {
                let w = C64::new(o.0, o.1) * shape;
                *o = (w.re, w.im);
            }
            BoardMode {
                freq_hz: md.omega / std::f64::consts::TAU,
                sigma,
                load: md.load,
                inputs: std::array::from_fn(|r| shapes[r][i] * t),
                outputs,
            }
        })
        .collect()
}

/// Bridge region of a key.
pub fn region_of(note: u8) -> usize {
    ((note.clamp(21, 108) - 21) as usize * REGIONS / 88).min(REGIONS - 1)
}

/// Below this frequency a key's bridge forces are shared between the two
/// regions around it, above it the nearest region takes them. Neighbouring
/// regions drive the board alike at low frequencies (their drive shapes
/// correlate by 0.9 at 100–200 Hz, 0.6–0.7 at 200–400 Hz, 0.2 at 400–800 Hz
/// in the middle: examples/region_coherence.rs), where one region per key
/// made the board's answer jump at the regions' ends (E4 to F4: 13 dB on
/// the fundamental). Above, they no longer add coherently: sharing lost
/// ≈ 3 dB of the treble's upper partials.
pub const SHARE_HZ: f64 = 400.0;

/// The two regions a key's low-frequency forces are shared between, and
/// their weights (linear between the regions' centres).
pub fn region_weights(note: u8) -> [(usize, f32); 2] {
    let x = ((note.clamp(21, 108) - 21) as f64 + 0.5) * REGIONS as f64 / 88.0 - 0.5;
    let r0 = (x.max(0.0).floor() as usize).min(REGIONS - 1);
    let r1 = (r0 + 1).min(REGIONS - 1);
    let w1 = if r1 == r0 { 0.0 } else { (x - r0 as f64).clamp(0.0, 1.0) as f32 };
    [(r0, 1.0 - w1), (r1, w1)]
}

/// Key at the middle of a region (its bridge point stands for the region).
pub fn region_center_note(r: usize) -> u8 {
    21 + ((2 * r + 1) * 88 / (2 * REGIONS)) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::soundboard::{Soundboard, SoundboardParams};

    /// The profiles rebuild every key's drive of the modes below 1 kHz.
    #[test]
    fn low_drive_rebuilds_each_key() {
        let sr = 48_000.0;
        let sb = crate::params::PianoModel::default().board().unwrap();
        let modes = board_modes(&sb, sr);
        let ld = LowDrive::new(&sb, &modes, 1000.0, sr).unwrap();
        assert!(ld.n > 30 && modes[ld.n].freq_hz >= 1000.0);
        let (mut err, mut sum) = (0.0, 0.0);
        for note in 21..=108 {
            let exact = sb.bridge_shapes(note);
            for i in 0..ld.n {
                let rebuilt: f64 = ld.inputs[i].iter().zip(ld.key(note)).map(|(a, b)| a * b).sum::<f64>() * sr;
                err += (rebuilt - exact[i]).powi(2);
                sum += exact[i].powi(2);
            }
        }
        assert!((err / sum).sqrt() < 0.02, "{}", (err / sum).sqrt());
    }

    /// With every damper lifted, the board has the poles it has without the
    /// strings' load.
    #[test]
    fn lifted_dampers_remove_the_string_load() {
        let sr = 48_000.0;
        let model = crate::params::PianoModel::default();
        let loaded = board_modes(&model.board().unwrap(), sr);
        let bare = board_modes(&Soundboard::new(model.soundboard.unwrap()), sr);
        // The load is a fair share of every low mode's damping (≥ 0.197 since
        // the board's mobility × 0.8, docs/board_hump.md: the (2,2) mode at
        // 247 Hz has its nodal line along the bridge).
        assert!(loaded.iter().filter(|m| m.freq_hz < 400.0).all(|m| m.load > 0.15 * m.sigma));
        let mut a = BoardBank::new(sr, &loaded, 64);
        let b = BoardBank::new(sr, &bare, 64);
        a.set_load(0.0);
        for (pa, pb) in a.parts.iter().zip(&b.parts) {
            for (ca, cb) in pa.chunks.iter().zip(&pb.chunks) {
                for l in 0..LANES {
                    assert!((ca.pr[l] - cb.pr[l]).abs() < 1e-6 && (ca.pi[l] - cb.pi[l]).abs() < 1e-6);
                }
            }
        }
    }

    /// The rendered velocity transfer matches the plate's own mobility Y at
    /// the driving point, and a rendered sine matches `transfer`.
    #[test]
    fn transfer_matches_mobility_and_rendering() {
        let sr = 48_000.0;
        let sb = Soundboard::new(SoundboardParams::GRAND_2M);
        let modes = board_modes(&sb, sr);
        let r = 7;
        let shapes = sb.bridge_shapes(region_center_note(r));
        for f in [80.0, 440.0, 2500.0] {
            let w = std::f64::consts::TAU * f;
            let h = transfer(&modes, r, MICS + r, w, sr);
            let want = sb.mobility(&shapes, w);
            // Impulse invariance departs from the continuous mobility for the
            // strongly damped high modes (η up to ≈ 0.6): in the 80 Hz
            // conductance valley the small real part is off by ≈ 2.4×, i.e.
            // ≈ 27 % of |Y| (the reactive part matches to 0.3 %).
            let tol = if f < 100.0 { 0.3 } else { 0.2 };
            assert!((h - want).norm() < tol * want.norm(), "f {f}: {h:?} vs {want:?}");
        }
        // Render a sine through the bank and compare its steady state.
        let f = 440.0;
        let mut bank = BoardBank::new(sr, &modes, 256);
        let mut inputs = vec![vec![0.0f32; 256]; REGIONS];
        let mut disp = vec![vec![0.0f32; 256]; REGIONS];
        let (mut m0, mut m1) = (vec![0.0f32; 256], vec![0.0f32; 256]);
        let h = transfer(&modes, r, MICS + r, std::f64::consts::TAU * f, sr);
        let mut err = 0.0f64;
        let mut refp = 0.0f64;
        for blk in 0..400 {
            for (t, x) in inputs[r].iter_mut().enumerate() {
                *x = (std::f64::consts::TAU * f * (blk * 256 + t) as f64 / sr).cos() as f32;
            }
            for d in disp.iter_mut() {
                d.fill(0.0);
            }
            bank.process(&inputs, 256, [&mut m0, &mut m1], Some(&mut disp), None);
            if blk >= 380 {
                for t in 0..256 {
                    let ph = std::f64::consts::TAU * f * (blk * 256 + t) as f64 / sr;
                    let want = (h * crate::complex::C64::polar(1.0, ph)).re;
                    err += (disp[r][t] as f64 - want).powi(2);
                    refp += want * want;
                }
            }
        }
        assert!(err / refp < 1e-3, "relative error {}", err / refp);
    }

    #[test]
    fn all_isas_are_bit_identical() {
        let sr = 48_000.0;
        let modes = board_modes(&Soundboard::new(SoundboardParams::GRAND_2M), sr);
        let mut inputs = vec![vec![0.0f32; 64]; REGIONS];
        for (r, x) in inputs.iter_mut().enumerate().filter(|(r, _)| r % 3 != 1) {
            for (t, v) in x.iter_mut().enumerate() {
                *v = (((t * 31 + r * 7) % 13) as f32 - 6.0) * 0.1;
            }
        }
        let mut reference: Option<Vec<f32>> = None;
        for isa in crate::modal::Isa::available() {
            let mut bank = BoardBank::new(sr, &modes, 64);
            bank.set_isa(isa);
            let mut out = Vec::new();
            for _ in 0..4 {
                let (mut m0, mut m1) = (vec![0.0f32; 64], vec![0.0f32; 64]);
                let mut disp = vec![vec![0.0f32; 64]; REGIONS];
                bank.process(&inputs, 64, [&mut m0, &mut m1], Some(&mut disp), None);
                out.extend(m0.into_iter().chain(m1).chain(disp.into_iter().flatten()));
            }
            match &reference {
                None => reference = Some(out),
                Some(r) => assert!(r.iter().zip(&out).all(|(a, b)| a.to_bits() == b.to_bits()), "{isa:?}"),
            }
        }
    }
}
