//! The room around the piano: early reflections by image sources and a
//! late reverberant tail by a feedback delay network (FDN), both driven by
//! the piano's radiated sound.
//!
//! The direct sound at each microphone is the soundboard's Rayleigh integral
//! (exact); the room adds what comes back from the walls. The piano is taken
//! as a point source at the board centre for the reflections, with source
//! signals s_k(t) = pressure at 1 m towards [`PROBES`] directions: the
//! board's own radiation at the probes ([`crate::board::probe_points`]),
//! scaled to 1 m, plus what else the mics hear (action, body, rim, frame)
//! estimated from them as p·r. Each reflection takes the probe facing its
//! wall; the tail takes the probes on orthogonal inputs, so their powers
//! add as in a diffuse field (two close mics alone would bring the
//! interference of their two points into the whole room).
//!
//! - Early reflections: image sources of a shoebox room up to `order`, delay
//!   (r_image − r_direct)/c, gain Π√(1 − α)/r_image, wall absorption growing
//!   with frequency as a one-pole low-pass per reflection order.
//! - Late tail: 16-line FDN with a Householder matrix, delays spread over the
//!   room's mean free path, per-line two-band attenuation giving the Sabine
//!   T60 = 0.161·V/(S·α) at low and high frequency (air absorption included
//!   in the high band). Its level makes the reverberant energy at the mics
//!   (r/r_c)² times the direct one, with r_c = 0.057·√(V/T60) the critical
//!   distance.

// Small fixed-size DSP loops read better with explicit indices.
#![allow(clippy::needless_range_loop)]

use crate::board::{PROBES, PROBE_DISTANCE};
use crate::soundboard::SoundboardParams;

#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
pub struct RoomParams {
    pub enabled: bool,
    /// Room dimensions (m).
    pub size: [f64; 3],
    /// Position of the soundboard's corner (x = 0, y = 0) in the room (m);
    /// the board lies in the horizontal plane at that height. A vertical
    /// (upright) board stands at x, y with its top edge at the board's
    /// length above its foot ([`VERTICAL_FOOT`]), against the wall y = 0
    /// when it has one ([`crate::soundboard::SoundboardParams::wall_distance`]).
    pub piano: [f64; 3],
    /// Mean absorption coefficient of the walls at low (≈ 125 Hz) and high
    /// (≈ 4 kHz) frequency.
    pub absorption_low: f64,
    pub absorption_high: f64,
    /// Image-source order of the early reflections.
    pub order: usize,
    /// Scales the late tail (1 = physical level).
    pub tail_gain: f64,
    /// Directivity factor Q of the piano: the mics sit in its main lobe
    /// (lid, open side), so the power it sends into the room is that of an
    /// omnidirectional source 1/Q as strong as the mics suggest. 0: from
    /// the instrument and its set-up ([`piano_directivity`]); > 0: this
    /// value (1 = omnidirectional).
    pub directivity: f64,
}

impl RoomParams {
    /// A concert hall: 40 × 28 × 16 m (≈ 18 000 m³), piano on stage near
    /// one end; Sabine T60 ≈ 2.0 s low, 1.6 s high. Seen from mics 1.5 m
    /// away (critical distance ≈ 5.7 m), the tail is ≈ 12 dB below the
    /// direct sound and the first reflections come after 30–60 ms.
    pub const CONCERT_HALL: Self = Self {
        enabled: true,
        size: [40.0, 28.0, 16.0],
        piano: [6.0, 13.0, 0.9],
        absorption_low: 0.327,
        absorption_high: 0.37,
        order: 3,
        tail_gain: 1.0,
        directivity: 0.0,
    };

    /// A small recording hall: 12 × 8 × 5 m, T60 ≈ 1.1 s low, 0.75 s high.
    pub const HALL: Self = Self {
        enabled: true,
        size: [12.0, 8.0, 5.0],
        piano: [4.5, 3.0, 0.9],
        absorption_low: 0.18,
        absorption_high: 0.25,
        order: 3,
        tail_gain: 1.0,
        directivity: 0.0,
    };

    pub fn volume(&self) -> f64 {
        self.size.iter().product()
    }

    pub fn surface(&self) -> f64 {
        let [x, y, z] = self.size;
        2.0 * (x * y + x * z + y * z)
    }

    /// Sabine T60 (s) at low and high frequency (air absorption ≈ 0.01/m at
    /// 4 kHz adds 4·m·V to the absorption area in the high band).
    pub fn t60(&self) -> (f64, f64) {
        let (v, s) = (self.volume(), self.surface());
        (0.161 * v / (s * self.absorption_low), 0.161 * v / (s * self.absorption_high + 4.0 * 0.0025 * v))
    }
}

impl Default for RoomParams {
    fn default() -> Self {
        Self::CONCERT_HALL
    }
}

const C0: f64 = 343.0;
/// Height of a vertical board's lower edge above the floor (m).
pub const VERTICAL_FOOT: f64 = 0.25;
const LINES: usize = 16;
const MAX_TAPS: usize = 128;
use crate::mics::MAX_MICS as MICS;

#[derive(Clone, Copy, Default)]
struct Tap {
    delay: usize,
    gain: f32,
    order: usize,
    /// The probe facing the wall the sound leaves for.
    probe: usize,
}

/// Hadamard rows (of the 16 × 16 matrix) the probes enter the tail by:
/// orthogonal to each other and to the all-ones row the Householder
/// feedback reflects.
const INJECT_ROWS: [usize; PROBES] = [1, 4, 8, 15];

/// What the board radiates over a block, for the room: its part of the
/// mic signals (the first as many as the room's mics) and its pressure at
/// the probes, both in the units of the mic signals (after the
/// differentiator and the output gain).
pub struct BoardFeed<'a> {
    pub mics: [&'a [f32]; MICS],
    pub probes: [&'a [f32]; PROBES],
}

/// Early reflections and late tail, real-time part.
pub struct Room {
    // Source history (pressure at 1 m, per probe) for the early taps.
    hist: Vec<[f32; PROBES]>,
    pos: usize,
    /// Microphones in use (the first `n_mics` of every per-mic array).
    n_mics: usize,
    taps: [[Tap; MAX_TAPS]; MICS],
    n_taps: [usize; MICS],
    /// Per reflection order: one-pole low-pass coefficient (wall absorption).
    order_lp: [f32; 8],
    order_state: [[f32; 8]; MICS],
    // FDN.
    lines: Vec<Vec<f32>>,
    line_pos: [usize; LINES],
    line_len: [usize; LINES],
    /// Two-band attenuation per line: y = g_hi·x + (g_lo − g_hi)·LP(x).
    g_lo: [f32; LINES],
    g_hi: [f32; LINES],
    lp_a: f32,
    lp_state: [f32; LINES],
    /// The tail at each mic: a part common to all below the diffuse-field
    /// corner of its spacing to the nearest other mic, its own independent
    /// part above (one-pole crossover coefficient per mic, and the low-pass
    /// states of the common mix and of its own mix as that mic hears them).
    xo_a: [f32; MICS],
    xo_common: [f32; MICS],
    xo_own: [f32; MICS],
    /// Source pre-delay into the FDN (first reflection time).
    pre: usize,
    out_gain: f32,
    /// Distances mic → source, to estimate the source from the mics.
    r_direct: [f64; MICS],
    /// 1/√Q: the room is driven by the power the piano radiates in every
    /// direction, not by its level towards the mics.
    source_gain: f32,
    /// √(diffuse-field efficiency) of each mic's pattern: how much of the
    /// room it hears.
    hear: [f32; MICS],
    /// Probe pressure → pressure at 1 m (the board's probe outputs are
    /// levelled with the mics' estimate, [`crate::board::PROBES_BELOW`]).
    probe_gain: f32,
    /// Per line, each probe's sign into it (over √PROBES).
    inject: [[f32; PROBES]; LINES],
    sample_rate: f64,
}

/// The probes' directions from the board's centre, in room coordinates.
fn probe_dirs(p: &RoomParams, sb: &SoundboardParams) -> [[f64; 3]; PROBES] {
    let c = to_room(p, sb, (0.5 * sb.a, 0.5 * sb.b, 0.0));
    crate::board::probe_points(sb).map(|q| {
        let r = to_room(p, sb, (q[0], q[1], q[2]));
        let d = [r[0] - c[0], r[1] - c[1], r[2] - c[2]];
        let n = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt().max(1e-9);
        d.map(|x| x / n)
    })
}

/// A point in board coordinates (m, z towards the mics) in the room.
pub fn to_room(p: &RoomParams, sb: &SoundboardParams, (x, y, z): (f64, f64, f64)) -> [f64; 3] {
    if sb.vertical {
        let wall_y = if sb.wall_distance > 0.0 { sb.wall_distance } else { p.piano[1] };
        [p.piano[0] + y, wall_y + z, VERTICAL_FOOT + sb.a - x]
    } else {
        [p.piano[0] + x, p.piano[1] + y, p.piano[2] + z]
    }
}

/// Directivity factor Q of the piano towards its microphones, when
/// [`RoomParams::directivity`] is 0:
/// - a grand with its lid open beams towards the lid's opening: Q = 3
///   (directivity index ≈ 4–6 dB, Q ≈ 3–4, at mid and high frequencies),
///   down to 2 as it shuts ([`SoundboardParams::lid_closure`]) and as its
///   reflection fades below a grand's (the lid control fading a removed
///   lid in);
/// - a grand with the lid closed or removed still radiates mostly up and
///   down, its two faces in antiphase: Q = 2;
/// - an upright against a wall radiates into the half space in front of it:
///   Q = 2; standing free, 1.5.
///
/// Computing Q from the board's own radiation in every direction gave 0.1
/// to 8 from one octave to the next (interference of the few modes per band
/// at the mics), too erratic to drive the room (docs/upright.md).
pub fn piano_directivity(p: &RoomParams, sb: &SoundboardParams) -> f64 {
    if p.directivity > 0.0 {
        return p.directivity;
    }
    if sb.vertical {
        if sb.wall_distance > 0.0 { 2.0 } else { 1.5 }
    } else if !sb.lid_removed {
        let reflects = (sb.lid_reflection / SoundboardParams::GRAND_2M.lid_reflection).clamp(0.0, 1.0);
        2.0 + (1.0 - sb.lid_closure()) * reflects
    } else {
        2.0
    }
}

/// The point the room takes as the source, in board coordinates: 10 cm in
/// front of the board's centre.
pub fn source_point(sb: &SoundboardParams) -> (f64, f64, f64) {
    (sb.a / 2.0, sb.b / 2.0, 0.1)
}

/// √(diffuse-field efficiency of the mics / Q): the room is driven by the
/// power the piano radiates in every direction, not by its level towards
/// the mics, and heard through the mics' pattern a + (1 − a)·cos θ, which
/// takes √(a² + (1 − a)²/3) of the diffuse sound (the mics' mean power).
pub fn source_gain(sb: &SoundboardParams, directivity: f64) -> f64 {
    let n = sb.mics.len().max(1) as f64;
    let eff = sb.mics.iter().map(|m| m.diffuse_efficiency()).sum::<f64>() / n;
    (eff / directivity.max(1.0)).sqrt()
}

/// Distance (m, at least 0.3) of each microphone of `sb` (in metres, as
/// [`SoundboardParams::resolved`] places them) to the room's source point:
/// times its pressure, the source at 1 m.
pub fn mic_distances(sb: &SoundboardParams) -> Vec<f64> {
    let s = source_point(sb);
    sb.mics.iter().map(|m| ((m.at.0 - s.0).powi(2) + (m.at.1 - s.1).powi(2) + (m.at.2 - s.2).powi(2)).sqrt().max(0.3)).collect()
}

/// Critical distance (m) at which the room's sound equals the direct one,
/// as heard through the mics and with the piano's directivity.
pub fn critical_distance(p: &RoomParams, sb: &SoundboardParams) -> f64 {
    let (t_lo, t_hi) = p.t60();
    0.057 * (p.volume() / (0.5 * (t_lo + t_hi))).sqrt() / (p.tail_gain * source_gain(sb, piano_directivity(p, sb))).max(1e-3)
}

impl Room {
    /// `sb`: the soundboard with its microphones in metres.
    pub fn new(p: &RoomParams, sb: &SoundboardParams, sample_rate: f64) -> Self {
        let directivity = piano_directivity(p, sb);
        let sr = sample_rate;
        let src = to_room(p, sb, source_point(sb));
        let n_mics = sb.mics.len();
        let mic_room: [[f64; 3]; MICS] = std::array::from_fn(|m| sb.mics.as_slice().get(m).map_or(src, |mic| to_room(p, sb, mic.at)));
        // An upright against the wall y = 0: the board's radiation already
        // has that wall's first reflection.
        let own_wall = sb.vertical && sb.wall_distance > 0.0;
        let dist = |a: [f64; 3], b: [f64; 3]| ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt();
        let r_direct: [f64; MICS] = std::array::from_fn(|m| dist(src, mic_room[m]).max(0.3));
        let dirs = probe_dirs(p, sb);

        // Image sources of a shoebox: per axis, image coordinate and number
        // of reflections for index n ∈ [−order, order].
        let beta = |alpha: f64| (1.0 - alpha).max(0.0).sqrt();
        let b_mid = beta(0.5 * (p.absorption_low + p.absorption_high));
        let mut taps = [[Tap::default(); MAX_TAPS]; MICS];
        let mut n_taps = [0usize; MICS];
        let o = p.order as i64;
        let mut first = f64::INFINITY;
        for m in 0..n_mics {
            let mut list = Vec::new();
            for nx in -o..=o {
                for ny in -o..=o {
                    for nz in -o..=o {
                        let refl = (nx.abs() + ny.abs() + nz.abs()) as usize;
                        if refl == 0 || refl > p.order || (own_wall && (nx, ny, nz) == (0, -1, 0)) {
                            continue;
                        }
                        // n even: n·L + s; n odd: (n + 1)·L − s (mirrored).
                        let img = |n: i64, s: f64, l: f64| {
                            if n.rem_euclid(2) == 0 { n as f64 * l + s } else { (n + 1) as f64 * l - s }
                        };
                        // Odd images mirror the source in the wall.
                        let pi = [img(nx, src[0], p.size[0]), img(ny, src[1], p.size[1]), img(nz, src[2], p.size[2])];
                        let r = dist(pi, mic_room[m]);
                        let delay = ((r - r_direct[m]) / C0 * sr).round().max(1.0) as usize;
                        let gain = b_mid.powi(refl as i32) / r;
                        first = first.min((r - r_direct[m]) / C0);
                        // The sound leaves the source towards the mic's
                        // image in the same walls.
                        let mi = [img(nx, mic_room[m][0], p.size[0]), img(ny, mic_room[m][1], p.size[1]), img(nz, mic_room[m][2], p.size[2])];
                        let go = [mi[0] - src[0], mi[1] - src[1], mi[2] - src[2]];
                        let probe = (0..PROBES)
                            .max_by(|&a, &b| {
                                let dot = |k: usize| dirs[k].iter().zip(&go).map(|(x, y)| x * y).sum::<f64>();
                                dot(a).total_cmp(&dot(b))
                            })
                            .unwrap_or(0);
                        list.push(Tap { delay, gain: gain as f32, order: refl, probe });
                    }
                }
            }
            // Keep the strongest taps if the order produced too many.
            list.sort_by(|a, b| b.gain.total_cmp(&a.gain));
            list.truncate(MAX_TAPS);
            n_taps[m] = list.len();
            for (i, t) in list.into_iter().enumerate() {
                taps[m][i] = t;
            }
        }
        let max_delay = taps.iter().flat_map(|t| t.iter()).map(|t| t.delay).max().unwrap_or(1);
        let hist_len = (max_delay + 1).next_power_of_two();

        // Wall absorption grows with frequency: each reflection order passes
        // through a one-pole low-pass whose corner drops with the order.
        let order_lp = std::array::from_fn(|n| {
            let fc = 12_000.0 / (1.0 + n as f64) * (1.0 - p.absorption_high).max(0.05);
            (-std::f64::consts::TAU * fc / sr).exp() as f32
        });

        // FDN delays: spread around the mean free path 4V/S, mutually prime.
        let mfp = 4.0 * p.volume() / p.surface();
        let mut line_len = [0usize; LINES];
        for (i, l) in line_len.iter_mut().enumerate() {
            let t = mfp / C0 * (0.6 + 1.2 * i as f64 / (LINES - 1) as f64);
            let mut n = (t * sr) as usize | 1;
            while (2..n).take_while(|d| d * d <= n).any(|d| n.is_multiple_of(d)) {
                n += 2;
            }
            *l = n;
        }
        let (t_lo, t_hi) = p.t60();
        let g = |t60: f64, n: usize| 10f64.powf(-3.0 * n as f64 / (t60 * sr)) as f32;
        let g_lo = std::array::from_fn(|i| g(t_lo, line_len[i]));
        let g_hi = std::array::from_fn(|i| g(t_hi, line_len[i]));
        let lp_a = (-std::f64::consts::TAU * 1000.0 / sr).exp() as f32;
        let pre = ((first.max(0.005)) * sr) as usize;
        // Two mics d apart in a diffuse field hear the same pressure only
        // while kd is small: their coherence, sinc²(kd), halves at
        // kd ≈ 1.4, f ≈ 76/d Hz. Each mic against its nearest neighbour
        // (alone: all of its tail its own).
        let xo_a = std::array::from_fn(|m| {
            let d = (0..n_mics).filter(|&o| o != m && m < n_mics).map(|o| dist(mic_room[m], mic_room[o])).fold(f64::INFINITY, f64::min);
            if d.is_finite() { (-std::f64::consts::TAU * (76.0 / d.max(0.05)) / sr).exp() as f32 } else { 0.0 }
        });
        let hear = std::array::from_fn(|m| sb.mics.as_slice().get(m).map_or(0.0, |mic| mic.diffuse_efficiency().sqrt() as f32));

        let mut room = Self {
            hist: vec![[0.0; PROBES]; hist_len.max(pre + 1).next_power_of_two()],
            pos: 0,
            n_mics,
            taps,
            n_taps,
            order_lp,
            order_state: [[0.0; 8]; MICS],
            lines: line_len.iter().map(|&n| vec![0.0; n]).collect(),
            line_pos: [0; LINES],
            line_len,
            g_lo,
            g_hi,
            lp_a,
            lp_state: [0.0; LINES],
            xo_a,
            xo_common: [0.0; MICS],
            xo_own: [0.0; MICS],
            pre,
            out_gain: 1.0,
            source_gain: (1.0 / directivity.max(1.0)).sqrt() as f32,
            hear,
            r_direct,
            probe_gain: PROBE_DISTANCE as f32,
            inject: std::array::from_fn(|i| {
                std::array::from_fn(|k| if (INJECT_ROWS[k] & i).count_ones().is_multiple_of(2) { 1.0 } else { -1.0 } / (PROBES as f32).sqrt())
            }),
            sample_rate,
        };
        // Level of the tail: reverberant/direct energy = (r/r_c)². For a
        // unit impulse at 1 m the direct energy at the mic is 1/r², so the
        // tail's impulse response must carry 1/r_c².
        let t60_mid = 0.5 * (t_lo + t_hi);
        // r_c of the bare room: the source gain is applied to the input.
        let rc = 0.057 * (p.volume() / t60_mid).sqrt();
        let e = room.tail_energy(t60_mid);
        room.out_gain = (p.tail_gain / rc / e.sqrt()) as f32;
        room.reset();
        room
    }

    /// Energy of the FDN's impulse response (unit gain), one channel.
    fn tail_energy(&mut self, t60: f64) -> f64 {
        self.reset();
        let n = (1.5 * t60 * self.sample_rate) as usize;
        let mut e = 0.0f64;
        let mut out = [0.0f32; MICS];
        for i in 0..n {
            let x = if i == 0 { 1.0 } else { 0.0 };
            self.fdn_step(&std::array::from_fn(|j| if j % 2 == 0 { x } else { -x }), &mut out);
            e += (out[0] as f64).powi(2);
        }
        e
    }

    pub fn reset(&mut self) {
        self.hist.fill([0.0; PROBES]);
        for l in &mut self.lines {
            l.fill(0.0);
        }
        self.lp_state = [0.0; LINES];
        self.xo_common = [0.0; MICS];
        self.xo_own = [0.0; MICS];
        self.order_state = [[0.0; 8]; MICS];
    }

    /// One step of the tail, `x[i]` entering line i; the tail at each mic
    /// into `out` (unit gain, the first `n_mics`; mic 0 alone if none).
    #[inline]
    fn fdn_step(&mut self, x: &[f32; LINES], out: &mut [f32; MICS]) {
        // Read the line outputs, attenuated (two bands).
        let mut y = [0.0f32; LINES];
        for i in 0..LINES {
            let v = self.lines[i][self.line_pos[i]];
            self.lp_state[i] = self.lp_a * self.lp_state[i] + (1.0 - self.lp_a) * v;
            y[i] = self.g_hi[i] * v + (self.g_lo[i] - self.g_hi[i]) * self.lp_state[i];
        }
        // Householder feedback: y − (2/N)·Σy, plus the input.
        let s = y.iter().sum::<f32>() * (2.0 / LINES as f32);
        // Mutually orthogonal output mixes: the common one (row 1 of the
        // 16 × 16 Hadamard matrix: signs (−1)^popcount(row & i)), then one
        // per mic (rows 2, 3, …).
        let n = self.n_mics.max(1);
        let mut mix = [0.0f32; MICS + 1];
        for i in 0..LINES {
            let fb = y[i] - s + x[i];
            let p = self.line_pos[i];
            self.lines[i][p] = fb;
            self.line_pos[i] = if p + 1 == self.line_len[i] { 0 } else { p + 1 };
            for (row, m) in mix[..=n].iter_mut().enumerate() {
                *m += if ((row + 1) & i).count_ones() % 2 == 0 { y[i] } else { -y[i] };
            }
        }
        // Common below the corner, independent above (the one-pole low- and
        // high-pass pair keeps the power: |LP|² + |HP|² = 1).
        for m in 0..n {
            let a = self.xo_a[m];
            self.xo_common[m] = a * self.xo_common[m] + (1.0 - a) * mix[0];
            self.xo_own[m] = a * self.xo_own[m] + (1.0 - a) * mix[m + 1];
            out[m] = self.xo_common[m] + (mix[m + 1] - self.xo_own[m]);
        }
    }

    /// Microphones the room hears with.
    pub fn mics(&self) -> usize {
        self.n_mics
    }

    /// Adds the room's response to the microphone signals (in place),
    /// estimating the source from them alone. Real-time safe.
    pub fn process(&mut self, mics: &mut [&mut [f32]]) {
        self.process_with(mics, None);
    }

    /// Adds the room's response to the microphone signals `mics` (in place,
    /// one per microphone, the same length), the board's part of the source
    /// taken at the probes when given (`board` as long as the mics).
    /// Real-time safe.
    pub fn process_with(&mut self, mics: &mut [&mut [f32]], board: Option<BoardFeed>) {
        let n = self.n_mics.min(mics.len());
        let Some(len) = mics.first().map(|m| m.len()) else { return };
        let mask = self.hist.len() - 1;
        let r: [f32; MICS] = std::array::from_fn(|m| self.r_direct[m] as f32 / n.max(1) as f32);
        let (g, pg) = (self.source_gain, self.probe_gain);
        let mut tail = [0.0f32; MICS];
        for t in 0..len {
            // Equivalent omnidirectional source: pressure at 1 m, less the
            // piano's directivity towards the mics (their mean). The
            // board's own part from the probes, the rest from the mics.
            let s: [f32; PROBES] = match &board {
                Some(b) => {
                    let other: f32 = (0..n).map(|m| (mics[m][t] - b.mics[m][t]) * r[m]).sum();
                    std::array::from_fn(|k| (other + pg * b.probes[k][t]) * g)
                }
                None => [(0..n).map(|m| mics[m][t] * r[m]).sum::<f32>() * g; PROBES],
            };
            self.hist[self.pos & mask] = s;
            let mut early = [0.0f32; MICS];
            for (m, e) in early[..n].iter_mut().enumerate() {
                let mut by_order = [0.0f32; 8];
                for t in &self.taps[m][..self.n_taps[m]] {
                    by_order[t.order] += t.gain * self.hist[(self.pos.wrapping_sub(t.delay)) & mask][t.probe];
                }
                for (n, &x) in by_order.iter().enumerate().skip(1) {
                    let st = &mut self.order_state[m][n];
                    *st = self.order_lp[n] * *st + (1.0 - self.order_lp[n]) * x;
                    *e += *st;
                }
            }
            let x = self.hist[(self.pos.wrapping_sub(self.pre)) & mask];
            let inj: [f32; LINES] = std::array::from_fn(|i| self.inject[i].iter().zip(&x).map(|(a, b)| a * b).sum());
            self.fdn_step(&inj, &mut tail);
            for m in 0..n {
                mics[m][t] += self.hear[m] * (early[m] + self.out_gain * tail[m]);
            }
            self.pos = self.pos.wrapping_add(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mics::{Mic, Mics};

    #[test]
    fn tail_decays_at_the_sabine_rate() {
        for p in [RoomParams::HALL, RoomParams::CONCERT_HALL] {
            check_t60(p);
        }
    }

    #[test]
    fn an_upright_stands_against_its_wall() {
        let p = RoomParams::HALL;
        let sb = SoundboardParams { vertical: true, wall_distance: 0.2, a: 1.05, mics: Mics::pair((0.3, 0.4, 0.5), (0.8, 1.0, 0.5)), ..SoundboardParams::GRAND_2M };
        let m = to_room(&p, &sb, sb.mics[0].at);
        assert!((m[1] - 0.7).abs() < 1e-12 && (m[2] - (VERTICAL_FOOT + 1.05 - 0.3)).abs() < 1e-12, "{m:?}");
        assert_eq!(piano_directivity(&p, &sb), 2.0);
        assert_eq!(piano_directivity(&p, &SoundboardParams { wall_distance: 0.0, ..sb }), 1.5);
        let grand = SoundboardParams { lid_angle: 35.0, lid_removed: false, ..SoundboardParams::GRAND_2M };
        assert_eq!(piano_directivity(&p, &grand), 3.0);
        assert_eq!(piano_directivity(&p, &SoundboardParams { lid_removed: true, ..grand }), 2.0);
        assert_eq!(piano_directivity(&p, &SoundboardParams { lid_angle: 0.0, ..grand }), 2.0);
        assert_eq!(piano_directivity(&p, &SoundboardParams { lid_angle: 0.5 * crate::soundboard::LID_CLOSING, ..grand }), 2.5);
        assert_eq!(piano_directivity(&RoomParams { directivity: 4.0, ..p }, &grand), 4.0);
        let mut room = Room::new(&p, &sb, 48_000.0);
        let (mut l, mut r) = (vec![0.0f32; 48_000], vec![0.0f32; 48_000]);
        l[0] = 1.0;
        room.process(&mut [&mut l, &mut r]);
        assert!(l.iter().chain(&r).all(|v| v.is_finite()));
    }

    #[test]
    fn every_mic_hears_its_own_tail_through_its_pattern() {
        // Two omnis and two cardioids far apart: four tails of the same
        // level apart from the pattern (a third of the energy for a
        // cardioid), each its own above the spacing's corner.
        let p = RoomParams::HALL;
        let c = |at, a| Mic { pattern: a, ..Mic::omni(at) };
        let mics = Mics::new(&[c((0.5, 0.4, 1.5), 1.0), (c((1.2, 0.9, 1.5), 1.0)), c((0.5, -2.0, 2.0), 0.5), c((1.2, -2.0, 2.0), 0.5)]);
        let sb = SoundboardParams { mics, ..SoundboardParams::GRAND_2M };
        let mut room = Room::new(&p, &sb, 48_000.0);
        assert_eq!(room.mics(), 4);
        let n = 48_000;
        let mut ch: Vec<Vec<f32>> = vec![vec![0.0; n]; 4];
        // The same source estimate from every mic.
        let r = room.r_direct;
        for (m, c) in ch.iter_mut().enumerate() {
            c[0] = (1.0 / r[m]) as f32;
        }
        let mut out: Vec<&mut [f32]> = ch.iter_mut().map(|c| &mut c[..]).collect();
        room.process(&mut out);
        let tail = |c: &[f32]| c[24_000..].iter().map(|v| (*v as f64).powi(2)).sum::<f64>();
        let e: Vec<f64> = ch.iter().map(|c| tail(c)).collect();
        assert!((e[1] / e[0] - 1.0).abs() < 0.5 && (e[3] / e[2] - 1.0).abs() < 0.5, "{e:?}");
        let card = (e[2] + e[3]) / (e[0] + e[1]);
        assert!((card - 1.0 / 3.0).abs() < 0.12, "cardioids hear {card:.2} of the omnis' tail");
        // Far apart, the tails differ.
        let dot: f64 = ch[0][24_000..].iter().zip(&ch[2][24_000..]).map(|(a, b)| (*a as f64) * (*b as f64)).sum();
        assert!(dot.abs() < 0.3 * (e[0] * e[2]).sqrt(), "correlation {}", dot / (e[0] * e[2]).sqrt());
    }

    #[test]
    fn the_board_reaches_the_room_through_its_probes() {
        let p = RoomParams::HALL;
        let sb = SoundboardParams { mics: Mics::pair((0.5, 0.4, 1.5), (1.2, 0.9, 1.5)), ..SoundboardParams::GRAND_2M };
        let n = 24_000;
        let energy = |x: &[f32]| x.iter().map(|v| (*v as f64).powi(2)).sum::<f64>();
        let run = |board: [f32; 2], probes: [f32; PROBES]| {
            let mut room = Room::new(&p, &sb, 48_000.0);
            let (mut l, mut r) = (vec![0.0f32; n], vec![0.0f32; n]);
            l[0] = 1.0;
            r[0] = 1.0;
            let (mut bm, mut pr) = ([vec![0.0f32; n], vec![0.0f32; n]], std::array::from_fn::<_, PROBES, _>(|_| vec![0.0f32; n]));
            bm[0][0] = board[0];
            bm[1][0] = board[1];
            for (q, v) in pr.iter_mut().zip(probes) {
                q[0] = v;
            }
            let feed = BoardFeed { mics: std::array::from_fn(|m| bm.get(m).map_or(&[][..], |b| &b[..])), probes: std::array::from_fn(|k| &pr[k][..]) };
            room.process_with(&mut [&mut l, &mut r], Some(feed));
            energy(&l[1..]) + energy(&r[1..])
        };
        // All the mics hear is the board, whose probes are silent: no room.
        assert_eq!(run([1.0, 1.0], [0.0; PROBES]), 0.0);
        // The same board heard at the probes as the mics' estimate: the
        // room as from the mics alone (the tail by other inputs).
        let mut alone = Room::new(&p, &sb, 48_000.0);
        let (mut l, mut r) = (vec![0.0f32; n], vec![0.0f32; n]);
        l[0] = 1.0;
        r[0] = 1.0;
        alone.process(&mut [&mut l, &mut r]);
        let est = 0.5 * (alone.r_direct[0] + alone.r_direct[1]) as f32 / PROBE_DISTANCE as f32;
        let probes = run([1.0, 1.0], [est; PROBES]);
        let mics = energy(&l[1..]) + energy(&r[1..]);
        assert!((probes / mics - 1.0).abs() < 0.2, "probes {probes:.3e}, mics {mics:.3e}");
    }

    fn check_t60(p: RoomParams) {
        let sr = 48_000.0;
        let sb = SoundboardParams { mics: Mics::pair((0.5, 0.4, 1.5), (1.2, 0.9, 1.5)), ..SoundboardParams::GRAND_2M };
        let mut room = Room::new(&p, &sb, sr);
        let n = (3.0 * sr) as usize;
        let (mut l, mut r) = (vec![0.0f32; n], vec![0.0f32; n]);
        l[0] = 1.0;
        r[0] = 1.0;
        room.process(&mut [&mut l, &mut r]);
        let level = |a: f64, b: f64| {
            let s = &l[(a * sr) as usize..(b * sr) as usize];
            10.0 * (s.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / s.len() as f64).log10()
        };
        // Broadband decay over 0.5 s after the build-up, versus Sabine.
        let slope = (level(0.4, 0.5) - level(0.9, 1.0)) / 0.5;
        let t60 = 60.0 / slope;
        let (lo, hi) = p.t60();
        assert!(t60 > 0.8 * hi && t60 < 1.2 * lo, "T60 {t60:.2} s vs Sabine {hi:.2}–{lo:.2} s");
        assert!(l.iter().all(|v| v.is_finite()));
    }
}
