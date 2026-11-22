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
use crate::modal::Isa;

/// Samples processed at once (at most: also at most the shortest line, so
/// that a block reads every line before writing it).
const SEG: usize = 64;
/// Samples the early taps accumulate at once (one vector).
const W: usize = 16;
/// Reflection orders the taps can have (order 0 is the direct sound).
const ORDERS: usize = 8;
/// Order low-passes of all the mics, in whole groups of 16.
const CHAINS: usize = (MICS * (ORDERS - 1)).next_multiple_of(16);

#[derive(Clone, Copy, Default)]
struct Tap {
    delay: usize,
    gain: f32,
    order: usize,
    /// The probe facing the wall the sound leaves for.
    probe: usize,
}

/// A tap of [`Room::early`], its order given by its place.
#[derive(Clone, Copy, Default)]
struct EarlyTap {
    delay: usize,
    gain: f32,
    probe: usize,
}

/// Per-block work buffers (one block of at most [`SEG`] samples).
struct Scratch {
    /// The source, per probe.
    src: [[f32; SEG]; PROBES],
    /// Each mic's taps summed per reflection order (1, 2, …), then
    /// low-passed ([`Room::chain_start`]).
    order: [[f32; SEG]; CHAINS],
    /// Each mic's early reflections.
    early: [[f32; SEG]; MICS],
    /// Per line: its output, then attenuated (y).
    y: [[f32; SEG]; LINES],
    /// Per line: its output's low-pass.
    lp: [[f32; SEG]; LINES],
    /// Per line: its input, then what is written back into it.
    fb: [[f32; SEG]; LINES],
    /// The output mixes (common, then one per mic).
    mix: [[f32; SEG]; MICS + 1],
    /// The tail at each mic.
    tail: [[f32; SEG]; MICS],
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
    /// Source history (pressure at 1 m), one ring of `hist_mask + 1`
    /// samples per probe, its first [`SEG`] samples repeated after its end
    /// so that [`SEG`] samples from any start read in one piece.
    hist: [Vec<f32>; PROBES],
    hist_mask: usize,
    pos: usize,
    /// Microphones in use (the first `n_mics` of every per-mic array).
    n_mics: usize,
    /// The taps strongest first, as built (the reference the tests check
    /// [`Room::early`] against).
    #[cfg(test)]
    taps: [[Tap; MAX_TAPS]; MICS],
    #[cfg(test)]
    n_taps: [usize; MICS],
    /// Each mic's taps by reflection order (in their order within each):
    /// order o is `early[m][order_end[m][o − 1]..order_end[m][o]]`.
    early: [[EarlyTap; MAX_TAPS]; MICS],
    order_end: [[usize; ORDERS]; MICS],
    /// Orders 1..`n_orders[m]` have taps (the low-passes of the others stay
    /// at zero).
    n_orders: [usize; MICS],
    /// Mic m's order low-passes are chains `chain_start[m]..chain_start[m + 1]`
    /// of the block's filters, whose coefficients are `chain_a` (0 past
    /// the last).
    chain_start: [usize; MICS + 1],
    chain_a: [f32; CHAINS],
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
    /// Per output mix (row r + 1 of the Hadamard matrix), the lines it
    /// takes with a minus sign (bit i: (−1)^popcount((r + 1) & i) = −1).
    mix_neg: [u16; MICS + 1],
    sample_rate: f64,
    /// Samples per block: [`SEG`], or the shortest line if shorter.
    seg: usize,
    /// Longest delay a block reads the source history at.
    reach: usize,
    /// Longest line.
    max_line: usize,
    /// The coefficients keep an all-zero room at +0 ([`Room::new`]).
    can_rest: bool,
    /// Samples since the source was last other than +0 (saturating).
    src_quiet: usize,
    /// Samples since a line was last fed other than +0 (saturating).
    fdn_quiet: usize,
    /// Every low-pass and crossover state is +0.
    filters_zero: bool,
    isa: Isa,
    scratch: Box<Scratch>,
    #[cfg(test)]
    rested: usize,
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
        // The taps grouped by order (a stable sort: each order's taps in
        // the order they are summed in).
        let mut early = [[EarlyTap::default(); MAX_TAPS]; MICS];
        let mut order_end = [[0usize; ORDERS]; MICS];
        let mut n_orders = [1usize; MICS];
        for m in 0..MICS {
            let mut list: Vec<Tap> = taps[m][..n_taps[m]].iter().copied().filter(|t| t.order < ORDERS).collect();
            list.sort_by_key(|t| t.order);
            for (i, t) in list.iter().enumerate() {
                early[m][i] = EarlyTap { delay: t.delay, gain: t.gain, probe: t.probe };
            }
            for o in 0..ORDERS {
                order_end[m][o] = list.iter().filter(|t| t.order <= o).count();
            }
            n_orders[m] = list.iter().map(|t| t.order + 1).max().unwrap_or(1);
        }

        // Wall absorption grows with frequency: each reflection order passes
        // through a one-pole low-pass whose corner drops with the order.
        let order_lp: [f32; ORDERS] = std::array::from_fn(|n| {
            let fc = 12_000.0 / (1.0 + n as f64) * (1.0 - p.absorption_high).max(0.05);
            (-std::f64::consts::TAU * fc / sr).exp() as f32
        });
        // An order without taps keeps its low-pass at +0 when a·(+0) is
        // +0; otherwise every order is filtered.
        if !order_lp.iter().all(|&a: &f32| a >= 0.0 && a.is_finite()) {
            n_orders = [ORDERS; MICS];
        }
        let mut chain_start = [0usize; MICS + 1];
        let mut chain_a = [0.0f32; CHAINS];
        for m in 0..MICS {
            let j0 = chain_start[m];
            chain_a[j0..j0 + n_orders[m] - 1].copy_from_slice(&order_lp[1..n_orders[m]]);
            chain_start[m + 1] = j0 + n_orders[m] - 1;
        }

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
        let g_lo: [f32; LINES] = std::array::from_fn(|i| g(t_lo, line_len[i]));
        let g_hi: [f32; LINES] = std::array::from_fn(|i| g(t_hi, line_len[i]));
        let lp_a = (-std::f64::consts::TAU * 1000.0 / sr).exp() as f32;
        let pre = ((first.max(0.005)) * sr) as usize;
        // Two mics d apart in a diffuse field hear the same pressure only
        // while kd is small: their coherence, sinc²(kd), halves at
        // kd ≈ 1.4, f ≈ 76/d Hz. Each mic against its nearest neighbour
        // (alone: all of its tail its own).
        let xo_a: [f32; MICS] = std::array::from_fn(|m| {
            let d = (0..n_mics).filter(|&o| o != m && m < n_mics).map(|o| dist(mic_room[m], mic_room[o])).fold(f64::INFINITY, f64::min);
            if d.is_finite() { (-std::f64::consts::TAU * (76.0 / d.max(0.05)) / sr).exp() as f32 } else { 0.0 }
        });
        let hear = std::array::from_fn(|m| sb.mics.as_slice().get(m).map_or(0.0, |mic| mic.diffuse_efficiency().sqrt() as f32));
        // A block reads the history up to `reach` samples before its start
        // while writing its own samples: the ring holds both.
        let reach = max_delay.max(pre);
        let hist_len = (reach + SEG).next_power_of_two();
        let inject: [[f32; PROBES]; LINES] = std::array::from_fn(|i| {
            std::array::from_fn(|k| if (INJECT_ROWS[k] & i).count_ones().is_multiple_of(2) { 1.0 } else { -1.0 } / (PROBES as f32).sqrt())
        });
        // From all +0 and a +0 source, every product below meets a +0 or
        // a non-negative coefficient and every sum a +0 term: the room
        // stays at +0 and adds hear·(+0) to the mics.
        let fin = |x: &f32| x.is_finite();
        let can_rest = order_lp.iter().all(|&a: &f32| a >= 0.0 && (1.0 - a).is_finite())
            && g_hi.iter().all(|&g: &f32| g >= 0.0 && g.is_finite())
            && g_lo.iter().all(fin)
            && lp_a >= 0.0
            && lp_a.is_finite()
            && xo_a.iter().all(|&a: &f32| a >= 0.0 && a.is_finite())
            && inject.iter().flatten().all(fin)
            && taps.iter().flatten().all(|t| t.gain.is_finite());
        let scratch = Box::new(Scratch {
            src: [[0.0; SEG]; PROBES],
            order: [[0.0; SEG]; CHAINS],
            early: [[0.0; SEG]; MICS],
            y: [[0.0; SEG]; LINES],
            lp: [[0.0; SEG]; LINES],
            fb: [[0.0; SEG]; LINES],
            mix: [[0.0; SEG]; MICS + 1],
            tail: [[0.0; SEG]; MICS],
        });

        let mut room = Self {
            hist: std::array::from_fn(|_| vec![0.0; hist_len + SEG]),
            hist_mask: hist_len - 1,
            pos: 0,
            n_mics,
            #[cfg(test)]
            taps,
            #[cfg(test)]
            n_taps,
            early,
            order_end,
            n_orders,
            chain_start,
            chain_a,
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
            inject,
            mix_neg: std::array::from_fn(|row| (0..LINES).filter(|&i| ((row + 1) & i).count_ones() % 2 != 0).fold(0u16, |b, i| b | 1 << i)),
            sample_rate,
            seg: SEG.min(*line_len.iter().min().unwrap_or(&SEG)),
            reach,
            max_line: *line_len.iter().max().unwrap_or(&0),
            can_rest,
            src_quiet: usize::MAX,
            fdn_quiet: usize::MAX,
            filters_zero: true,
            isa: Self::runs_on(Isa::detect()),
            scratch,
            #[cfg(test)]
            rested: 0,
        };
        // Level of the tail: reverberant/direct energy = (r/r_c)². For a
        // unit impulse at 1 m the direct energy at the mic is 1/r², so the
        // tail's impulse response must carry 1/r_c².
        let t60_mid = 0.5 * (t_lo + t_hi);
        // r_c of the bare room: the source gain is applied to the input.
        let rc = 0.057 * (p.volume() / t60_mid).sqrt();
        let e = room.tail_energy(t60_mid);
        room.out_gain = (p.tail_gain / rc / e.sqrt()) as f32;
        room.can_rest &= room.out_gain.is_finite();
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

    /// The code the room runs when the engine runs `isa`: its vector code
    /// is compiled for AVX2 alone, which AVX-512 machines take too (measured
    /// faster on Zen 5: the taps' loads start anywhere, so a 64-byte one
    /// nearly always spans two cache lines, and the filters are serial),
    /// when AVX2 itself is detected ([`Isa::Avx512`] says AVX-512F only).
    fn runs_on(isa: Isa) -> Isa {
        match isa {
            #[cfg(target_arch = "x86_64")]
            Isa::Avx2 | Isa::Avx512 if is_x86_feature_detected!("avx2") => Isa::Avx2,
            _ => Isa::Generic,
        }
    }

    pub fn reset(&mut self) {
        for h in &mut self.hist {
            h.fill(0.0);
        }
        for l in &mut self.lines {
            l.fill(0.0);
        }
        self.lp_state = [0.0; LINES];
        self.xo_common = [0.0; MICS];
        self.xo_own = [0.0; MICS];
        self.order_state = [[0.0; 8]; MICS];
        self.src_quiet = usize::MAX;
        self.fdn_quiet = usize::MAX;
        self.filters_zero = true;
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
        let r: [f32; MICS] = std::array::from_fn(|m| self.r_direct[m] as f32 / n.max(1) as f32);
        let mut t0 = 0;
        while t0 < len {
            let l = (len - t0).min(self.seg);
            match self.isa {
                // SAFETY: `isa` comes from `Self::runs_on`: AVX2 detected.
                #[cfg(target_arch = "x86_64")]
                Isa::Avx2 => unsafe { block_avx2(self, mics, board.as_ref(), t0, l, n, &r) },
                _ => self.block(mics, board.as_ref(), t0, l, n, &r),
            }
            t0 += l;
        }
    }

    /// Samples `t0..t0 + l` (l ≤ `seg`). Every sample's source depends on
    /// the mics before the room adds to them, never on the room's output;
    /// the taps read the source at least one sample back, the tail's input
    /// at the pre-delay, and each line what was written into it a whole
    /// line (≥ l samples) before: the whole block's source first, then its
    /// taps, then the tail, each over the block, give what one sample at a
    /// time would, in the same operations.
    #[inline(always)]
    fn block(&mut self, mics: &mut [&mut [f32]], board: Option<&BoardFeed>, t0: usize, l: usize, n: usize, r: &[f32; MICS]) {
        let mask = self.hist_mask;
        let h = mask + 1;
        let sc = &mut *self.scratch;
        let (g, pg) = (self.source_gain, self.probe_gain);

        // Equivalent omnidirectional source: pressure at 1 m, less the
        // piano's directivity towards the mics (their mean). The board's
        // own part from the probes, the rest from the mics (summed from
        // −0, as `Iterator::sum`).
        let mut other = [-0.0f32; SEG];
        let other = &mut other[..l];
        match board {
            Some(b) => {
                for m in 0..n {
                    let (x, y, rm) = (&mics[m][t0..t0 + l], &b.mics[m][t0..t0 + l], r[m]);
                    for ((o, x), y) in other.iter_mut().zip(x).zip(y) {
                        *o += (x - y) * rm;
                    }
                }
                for k in 0..PROBES {
                    let p = &b.probes[k][t0..t0 + l];
                    for ((s, o), p) in sc.src[k][..l].iter_mut().zip(&*other).zip(p) {
                        *s = (o + pg * p) * g;
                    }
                }
            }
            None => {
                for m in 0..n {
                    let (x, rm) = (&mics[m][t0..t0 + l], r[m]);
                    for (o, x) in other.iter_mut().zip(x) {
                        *o += x * rm;
                    }
                }
                for k in 0..PROBES {
                    for (s, o) in sc.src[k][..l].iter_mut().zip(&*other) {
                        *s = o * g;
                    }
                }
            }
        }
        let j0 = self.pos & mask;
        let first = (h - j0).min(l);
        let mut bits = 0u32;
        for (hk, s) in self.hist.iter_mut().zip(&sc.src) {
            let s = &s[..l];
            hk[j0..j0 + first].copy_from_slice(&s[..first]);
            hk[..l - first].copy_from_slice(&s[first..]);
            if j0 < SEG || first < l {
                hk.copy_within(0..SEG, h);
            }
            bits |= s.iter().fold(0, |a, v| a | v.to_bits());
        }

        // At rest (every state and every source sample it reads +0), the
        // block would leave the room at +0 and add hear·(+0) to the mics.
        let quiet = bits == 0;
        let rest = quiet && self.can_rest && self.filters_zero && self.src_quiet >= self.reach && self.fdn_quiet >= self.max_line;
        self.src_quiet = if quiet { self.src_quiet.saturating_add(l) } else { 0 };
        if rest {
            for m in 0..n {
                let z = self.hear[m] * 0.0;
                for y in &mut mics[m][t0..t0 + l] {
                    *y += z;
                }
            }
            self.advance(l);
            self.fdn_quiet = self.fdn_quiet.saturating_add(l);
            #[cfg(test)]
            {
                self.rested += l;
            }
            return;
        }

        // Early reflections: per mic and order, the taps summed in their
        // order from +0 (W samples at once), then the order's low-pass,
        // every mic's and order's filter at once (chain j of mic m and
        // order o: `chain_start[m] + o − 1`).
        for m in 0..n {
            let j0 = self.chain_start[m];
            for o in 1..self.n_orders[m] {
                let taps = &self.early[m][self.order_end[m][o - 1]..self.order_end[m][o]];
                let out = &mut sc.order[j0 + o - 1];
                let chunks = l.div_ceil(W);
                let mut c = 0;
                while c < chunks {
                    let (base, out) = (self.pos.wrapping_add(c * W), &mut out[c * W..]);
                    c += if chunks - c >= 4 {
                        tap_sums::<4>(taps, &self.hist, base, mask, out)
                    } else {
                        tap_sums::<1>(taps, &self.hist, base, mask, out)
                    };
                }
                let c1 = 1.0 - self.order_lp[o];
                for v in &mut out[..l] {
                    *v *= c1;
                }
            }
        }
        let mut st = [0.0f32; CHAINS];
        for m in 0..n {
            let (j0, no) = (self.chain_start[m], self.n_orders[m]);
            st[j0..j0 + no - 1].copy_from_slice(&self.order_state[m][1..no]);
        }
        for g in (0..self.chain_start[n]).step_by(16) {
            let u = (&mut sc.order[g..g + 16]).try_into().unwrap();
            one_poles::<16>(u, self.chain_a[g..g + 16].try_into().unwrap(), (&mut st[g..g + 16]).try_into().unwrap(), l);
        }
        for m in 0..n {
            let (j0, no) = (self.chain_start[m], self.n_orders[m]);
            self.order_state[m][1..no].copy_from_slice(&st[j0..j0 + no - 1]);
            let e = &mut sc.early[m][..l];
            e.fill(0.0);
            for x in &sc.order[j0..j0 + no - 1] {
                for (e, x) in e.iter_mut().zip(x) {
                    *e += *x;
                }
            }
            // The orders without taps, at +0: adding them once is adding
            // them all (it only turns −0 into +0).
            if no < ORDERS {
                for e in e.iter_mut() {
                    *e += 0.0;
                }
            }
        }

        // Late tail. A block is no longer than any line: it reads each
        // line's outputs before any of its inputs is written.
        for i in 0..LINES {
            let (line, p) = (&self.lines[i], self.line_pos[i]);
            let first = (self.line_len[i] - p).min(l);
            sc.y[i][..first].copy_from_slice(&line[p..p + first]);
            sc.y[i][first..l].copy_from_slice(&line[..l - first]);
        }
        let xb = self.pos.wrapping_sub(self.pre) & mask;
        let x: [&[f32]; PROBES] = std::array::from_fn(|k| &self.hist[k][xb..xb + l]);
        for i in 0..LINES {
            let c = self.inject[i];
            for (t, f) in sc.fb[i][..l].iter_mut().enumerate() {
                *f = -0.0 + c[0] * x[0][t] + c[1] * x[1][t] + c[2] * x[2][t] + c[3] * x[3][t];
            }
        }
        // Two-band attenuation.
        let c = 1.0 - self.lp_a;
        for (lp, y) in sc.lp.iter_mut().zip(&sc.y) {
            for (lp, v) in lp[..l].iter_mut().zip(&y[..l]) {
                *lp = c * v;
            }
        }
        one_poles(&mut sc.lp, &[self.lp_a; LINES], &mut self.lp_state, l);
        for i in 0..LINES {
            let (gh, gd) = (self.g_hi[i], self.g_lo[i] - self.g_hi[i]);
            for (y, lp) in sc.y[i][..l].iter_mut().zip(&sc.lp[i][..l]) {
                *y = gh * *y + gd * lp;
            }
        }
        // Householder feedback: y − (2/N)·Σy, plus the input.
        let mut s = [-0.0f32; SEG];
        for y in &sc.y {
            for (s, y) in s[..l].iter_mut().zip(&y[..l]) {
                *s += y;
            }
        }
        for s in &mut s[..l] {
            *s *= 2.0 / LINES as f32;
        }
        let mut fbits = 0u32;
        for i in 0..LINES {
            let f = &mut sc.fb[i][..l];
            for ((f, y), s) in f.iter_mut().zip(&sc.y[i][..l]).zip(&s[..l]) {
                *f += y - s;
                fbits |= f.to_bits();
            }
            let (line, p) = (&mut self.lines[i], self.line_pos[i]);
            let first = (self.line_len[i] - p).min(l);
            line[p..p + first].copy_from_slice(&f[..first]);
            line[..l - first].copy_from_slice(&f[first..]);
        }
        // Mutually orthogonal output mixes: the common one (row 1 of the
        // 16 × 16 Hadamard matrix: signs (−1)^popcount(row & i)), then one
        // per mic (rows 2, 3, …), summed over the lines in order from +0.
        let nf = self.n_mics.max(1);
        for (mix, &neg) in sc.mix[..=nf].iter_mut().zip(&self.mix_neg) {
            let chunks = l.div_ceil(W);
            let mut c = 0;
            while c < chunks {
                c += if chunks - c >= 4 { signed_sums::<4>(&sc.y, neg, c * W, mix) } else { signed_sums::<1>(&sc.y, neg, c * W, mix) };
            }
        }
        crossovers_dyn(&sc.mix, &self.xo_a[..nf], &mut self.xo_common[..nf], &mut self.xo_own[..nf], &mut sc.tail, l);
        for m in 0..n {
            let (hear, og) = (self.hear[m], self.out_gain);
            for ((y, e), tail) in mics[m][t0..t0 + l].iter_mut().zip(&sc.early[m][..l]).zip(&sc.tail[m][..l]) {
                *y += hear * (e + og * tail);
            }
        }
        self.advance(l);

        let zero = |s: &[f32]| s.iter().all(|v| v.to_bits() == 0);
        self.fdn_quiet = if fbits == 0 { self.fdn_quiet.saturating_add(l) } else { 0 };
        self.filters_zero = zero(&self.lp_state)
            && zero(&self.xo_common[..nf])
            && zero(&self.xo_own[..nf])
            && self.order_state[..self.n_mics].iter().all(|s| zero(s));
    }

    fn advance(&mut self, l: usize) {
        self.pos = self.pos.wrapping_add(l);
        for (p, &len) in self.line_pos.iter_mut().zip(&self.line_len) {
            *p += l;
            if *p >= len {
                *p -= len;
            }
        }
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
fn block_avx2(room: &mut Room, mics: &mut [&mut [f32]], board: Option<&BoardFeed>, t0: usize, l: usize, n: usize, r: &[f32; MICS]) {
    room.block(mics, board, t0, l, n, r)
}

/// The taps' sum (in their order, from +0) over K·W samples from `base`
/// into `out`; returns K. Each sample its own accumulator.
#[inline(always)]
fn tap_sums<const K: usize>(taps: &[EarlyTap], hist: &[Vec<f32>; PROBES], base: usize, mask: usize, out: &mut [f32]) -> usize {
    let mut a = [[0.0f32; W]; K];
    for tap in taps {
        let b = base.wrapping_sub(tap.delay) & mask;
        let s = &hist[tap.probe][b..b + K * W];
        for (a, s) in a.iter_mut().zip(s.as_chunks::<W>().0) {
            for j in 0..W {
                a[j] += tap.gain * s[j];
            }
        }
    }
    out[..K * W].copy_from_slice(a.as_flattened());
    K
}

/// Σ ±y_i over the lines in order from +0 (minus where bit i of `neg` is
/// set) over K·W samples from `t`, into `out`; returns K.
#[inline(always)]
fn signed_sums<const K: usize>(y: &[[f32; SEG]; LINES], neg: u16, t: usize, out: &mut [f32; SEG]) -> usize {
    let mut a = [[0.0f32; W]; K];
    for (i, y) in y.iter().enumerate() {
        let y = &y[t..t + K * W];
        for (a, y) in a.iter_mut().zip(y.as_chunks::<W>().0) {
            if neg >> i & 1 != 0 {
                for j in 0..W {
                    a[j] += -y[j];
                }
            } else {
                for j in 0..W {
                    a[j] += y[j];
                }
            }
        }
    }
    out[t..t + K * W].copy_from_slice(a.as_flattened());
    K
}

/// N one-pole low-passes over the first `l` samples, in place: `u[j][t]`
/// holds (1 − a_j)·x_j(t) and becomes s_j(t) = a_j·s_j(t − 1) + u[j][t].
#[inline(always)]
fn one_poles<const N: usize>(u: &mut [[f32; SEG]; N], a: &[f32; N], s: &mut [f32; N], l: usize) {
    let mut st = *s;
    for t in 0..l {
        for j in 0..N {
            st[j] = a[j] * st[j] + u[j][t];
            u[j][t] = st[j];
        }
    }
    *s = st;
}

/// The tail at each of N mics: its common part below the crossover, its
/// own above (`mix[0]` the common mix, `mix[m + 1]` mic m's own).
#[inline(always)]
fn crossovers<const N: usize>(mix: &[[f32; SEG]; MICS + 1], a: &[f32; N], common: &mut [f32; N], own: &mut [f32; N], tail: &mut [[f32; SEG]; MICS], l: usize) {
    let (mut xc, mut xo) = (*common, *own);
    let c = a.map(|a| 1.0 - a);
    for t in 0..l {
        let m0 = mix[0][t];
        for m in 0..N {
            let mm = mix[m + 1][t];
            xc[m] = a[m] * xc[m] + c[m] * m0;
            xo[m] = a[m] * xo[m] + c[m] * mm;
            tail[m][t] = xc[m] + (mm - xo[m]);
        }
    }
    (*common, *own) = (xc, xo);
}

/// [`crossovers`] for as many mics as `a` has (1 to [`MICS`]).
#[inline(always)]
fn crossovers_dyn(mix: &[[f32; SEG]; MICS + 1], a: &[f32], common: &mut [f32], own: &mut [f32], tail: &mut [[f32; SEG]; MICS], l: usize) {
    macro_rules! go {
        ($($n:literal)*) => {
            match a.len() {
                $($n => crossovers::<$n>(mix, a.try_into().unwrap(), common.try_into().unwrap(), own.try_into().unwrap(), tail, l),)*
                _ => {}
            }
        };
    }
    go!(1 2 3 4 5 6 7 8);
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

    /// The room as first written (one sample at a time, taps in their
    /// original order): the reference the block processing must match bit
    /// for bit. It takes its constants from a fresh [`Room`].
    struct Frozen {
        hist: Vec<[f32; PROBES]>,
        pos: usize,
        n_mics: usize,
        taps: [[Tap; MAX_TAPS]; MICS],
        n_taps: [usize; MICS],
        order_lp: [f32; 8],
        order_state: [[f32; 8]; MICS],
        lines: Vec<Vec<f32>>,
        line_pos: [usize; LINES],
        line_len: [usize; LINES],
        g_lo: [f32; LINES],
        g_hi: [f32; LINES],
        lp_a: f32,
        lp_state: [f32; LINES],
        xo_a: [f32; MICS],
        xo_common: [f32; MICS],
        xo_own: [f32; MICS],
        pre: usize,
        out_gain: f32,
        r_direct: [f64; MICS],
        source_gain: f32,
        hear: [f32; MICS],
        probe_gain: f32,
        inject: [[f32; PROBES]; LINES],
    }

    impl Frozen {
        fn new(r: &Room) -> Self {
            let max_delay = r.taps.iter().flat_map(|t| t.iter()).map(|t| t.delay).max().unwrap_or(1);
            let hist_len = (max_delay + 1).next_power_of_two();
            Self {
                hist: vec![[0.0; PROBES]; hist_len.max(r.pre + 1).next_power_of_two()],
                pos: 0,
                n_mics: r.n_mics,
                taps: r.taps,
                n_taps: r.n_taps,
                order_lp: r.order_lp,
                order_state: [[0.0; 8]; MICS],
                lines: r.line_len.iter().map(|&n| vec![0.0; n]).collect(),
                line_pos: [0; LINES],
                line_len: r.line_len,
                g_lo: r.g_lo,
                g_hi: r.g_hi,
                lp_a: r.lp_a,
                lp_state: [0.0; LINES],
                xo_a: r.xo_a,
                xo_common: [0.0; MICS],
                xo_own: [0.0; MICS],
                pre: r.pre,
                out_gain: r.out_gain,
                r_direct: r.r_direct,
                source_gain: r.source_gain,
                hear: r.hear,
                probe_gain: r.probe_gain,
                inject: r.inject,
            }
        }

        fn fdn_step(&mut self, x: &[f32; LINES], out: &mut [f32; MICS]) {
            let mut y = [0.0f32; LINES];
            for i in 0..LINES {
                let v = self.lines[i][self.line_pos[i]];
                self.lp_state[i] = self.lp_a * self.lp_state[i] + (1.0 - self.lp_a) * v;
                y[i] = self.g_hi[i] * v + (self.g_lo[i] - self.g_hi[i]) * self.lp_state[i];
            }
            let s = y.iter().sum::<f32>() * (2.0 / LINES as f32);
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
            for m in 0..n {
                let a = self.xo_a[m];
                self.xo_common[m] = a * self.xo_common[m] + (1.0 - a) * mix[0];
                self.xo_own[m] = a * self.xo_own[m] + (1.0 - a) * mix[m + 1];
                out[m] = self.xo_common[m] + (mix[m + 1] - self.xo_own[m]);
            }
        }

        fn process_with(&mut self, mics: &mut [&mut [f32]], board: Option<BoardFeed>) {
            let n = self.n_mics.min(mics.len());
            let Some(len) = mics.first().map(|m| m.len()) else { return };
            let mask = self.hist.len() - 1;
            let r: [f32; MICS] = std::array::from_fn(|m| self.r_direct[m] as f32 / n.max(1) as f32);
            let (g, pg) = (self.source_gain, self.probe_gain);
            let mut tail = [0.0f32; MICS];
            for t in 0..len {
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

    /// Deterministic pseudo-random numbers (xorshift64*).
    struct Rng(u64);

    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 >> 12;
            self.0 ^= self.0 << 25;
            self.0 ^= self.0 >> 27;
            self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
        }

        /// Uniform in [−1, 1).
        fn signal(&mut self) -> f32 {
            (self.next() >> 40) as f32 / (1u64 << 23) as f32 - 1.0
        }

        fn below(&mut self, n: usize) -> usize {
            (self.next() % n as u64) as usize
        }
    }

    /// What the mics and the board carry over a stretch of blocks.
    #[derive(Clone, Copy, PartialEq)]
    enum Stretch {
        Noise,
        /// Every input +0.
        Silence,
        /// Every input −0.
        NegativeZero,
        /// One impulse, then silence.
        Impulse,
    }

    fn bit_identity_configs() -> Vec<(&'static str, RoomParams, SoundboardParams, f64)> {
        let pair = Mics::pair((0.5, 0.4, 1.5), (1.2, 0.9, 1.5));
        let grand = SoundboardParams { mics: pair, ..SoundboardParams::GRAND_2M };
        let upright = SoundboardParams { vertical: true, wall_distance: 0.2, a: 1.05, mics: Mics::pair((0.3, 0.4, 0.5), (0.8, 1.0, 0.5)), ..SoundboardParams::GRAND_2M };
        let one = SoundboardParams { mics: Mics::new(&[Mic::omni((0.6, 0.5, 1.2))]), ..SoundboardParams::GRAND_2M };
        let c = |at, a| Mic { pattern: a, ..Mic::omni(at) };
        let eight = SoundboardParams {
            mics: Mics::new(&[
                c((0.5, 0.4, 1.5), 1.0),
                c((1.2, 0.9, 1.5), 1.0),
                c((0.5, -2.0, 2.0), 0.5),
                c((1.2, -2.0, 2.0), 0.5),
                c((0.3, 0.3, 0.4), 0.5),
                c((1.6, 1.0, 0.4), 0.7),
                c((0.8, 3.0, 2.5), 0.0),
                c((0.8, 6.0, 3.0), 1.0),
            ]),
            ..SoundboardParams::GRAND_2M
        };
        let dead = RoomParams { size: [3.0, 2.5, 2.4], piano: [0.4, 0.3, 0.9], absorption_low: 0.9, absorption_high: 0.95, directivity: 4.0, ..RoomParams::HALL };
        // Lines shorter than a block, taps of every order up to 7 (more
        // than MAX_TAPS of them).
        let tiny = RoomParams { size: [1.2, 1.0, 1.0], piano: [0.1, 0.1, 0.5], order: 7, absorption_low: 0.6, absorption_high: 0.8, ..RoomParams::HALL };
        vec![
            ("shipped", RoomParams::default(), SoundboardParams::DEFAULT.resolved(), 48_000.0),
            ("hall pair", RoomParams::HALL, grand, 48_000.0),
            ("upright", RoomParams::HALL, upright, 44_100.0),
            ("one mic", RoomParams::CONCERT_HALL, one, 48_000.0),
            ("eight mics", RoomParams { order: 4, ..RoomParams::HALL }, eight, 48_000.0),
            ("dead", dead, SoundboardParams::DEFAULT.resolved(), 48_000.0),
            ("tiny", tiny, grand, 48_000.0),
        ]
    }

    /// Drives a room and its frozen reference with the same signals and
    /// checks their mic buffers bit for bit after every block. `board`:
    /// 0 none, 1 always, 2 every other block. `bufs`: how many mic buffers
    /// are passed (the room's mics, fewer or more).
    #[allow(clippy::too_many_arguments)]
    fn check_bit_identity(name: &str, p: &RoomParams, sb: &SoundboardParams, sr: f64, board: usize, bufs: usize, seed: u64, isa: Isa) -> Room {
        let mut room = Room::new(p, sb, sr);
        room.isa = Room::runs_on(isa);
        let mut frozen = Frozen::new(&room);
        let mut rng = Rng(seed);
        let lens = [1usize, 7, 63, 64, 64, 64, 32, 17, 2, 64];
        // Long enough for the dead room to decay to exact zeros.
        let decay = (1.2 * sr) as usize;
        let plan = [
            (Stretch::Silence, 1_000),
            (Stretch::NegativeZero, 500),
            (Stretch::Noise, 12_000),
            (Stretch::Silence, 3_000),
            (Stretch::Noise, 2_000),
            (Stretch::NegativeZero, 3_000),
            (Stretch::Noise, 500),
            (Stretch::Impulse, decay),
            (Stretch::NegativeZero, 1_000),
            (Stretch::Noise, 2_000),
            (Stretch::Silence, decay / 2),
            (Stretch::Noise, 2_000),
        ];
        let max_len = 1_000;
        let mut a: Vec<Vec<f32>> = vec![vec![0.0; max_len]; bufs];
        let mut b: Vec<Vec<f32>> = vec![vec![0.0; max_len]; bufs];
        let mut bm: Vec<Vec<f32>> = vec![vec![0.0; max_len]; MICS];
        let mut pr: Vec<Vec<f32>> = vec![vec![0.0; max_len]; PROBES];
        let mut block = 0usize;
        for (stretch, samples) in plan {
            let mut done = 0;
            let mut first = true;
            while done < samples {
                // Mostly engine-sized blocks, now and then a longer one.
                let len = match rng.below(40) {
                    0 => 65 + rng.below(max_len - 65),
                    1..=10 => 1 + rng.below(64),
                    _ => lens[rng.below(lens.len())],
                }
                .min(samples - done);
                let scale = [1.0f32, 1e-3, 1e-20, 3e-38][rng.below(4)];
                for t in 0..len {
                    let (zero, imp) = match stretch {
                        Stretch::Noise => (None, false),
                        Stretch::Silence => (Some(0.0f32), false),
                        Stretch::NegativeZero => (Some(-0.0f32), false),
                        Stretch::Impulse => (Some(0.0f32), first && t == 0),
                    };
                    for m in 0..bufs {
                        let v = match zero {
                            Some(z) if !imp => z,
                            Some(_) => 1.0,
                            None => rng.signal() * scale,
                        };
                        a[m][t] = v;
                        b[m][t] = v;
                    }
                    for m in 0..MICS {
                        bm[m][t] = match zero {
                            Some(z) if !imp => z,
                            Some(_) => 0.5,
                            None => a.get(m).map_or(0.0, |x| x[t]) * (0.3 + 0.5 * rng.signal()),
                        };
                    }
                    for k in 0..PROBES {
                        pr[k][t] = match zero {
                            Some(z) if !imp => z,
                            Some(_) => 0.25 * (k as f32 - 1.5),
                            None => rng.signal() * scale,
                        };
                    }
                }
                first = false;
                let with_board = match board {
                    0 => false,
                    1 => true,
                    _ => block.is_multiple_of(2),
                };
                let feed = || BoardFeed { mics: std::array::from_fn(|m| &bm[m][..len]), probes: std::array::from_fn(|k| &pr[k][..len]) };
                {
                    let mut ma: Vec<&mut [f32]> = a.iter_mut().map(|c| &mut c[..len]).collect();
                    room.process_with(&mut ma, with_board.then(feed));
                }
                {
                    let mut mb: Vec<&mut [f32]> = b.iter_mut().map(|c| &mut c[..len]).collect();
                    frozen.process_with(&mut mb, with_board.then(feed));
                }
                for m in 0..bufs {
                    for t in 0..len {
                        assert_eq!(a[m][t].to_bits(), b[m][t].to_bits(), "{name}: block {block} (len {len}), mic {m}, sample {t}: {} vs {}", a[m][t], b[m][t]);
                    }
                }
                done += len;
                block += 1;
            }
        }
        room
    }

    fn bit_identity(ftz: bool) {
        let _guard = ftz.then(crate::denormal::DenormalGuard::new);
        for isa in Isa::available() {
            for (c, (name, p, sb, sr)) in bit_identity_configs().into_iter().enumerate() {
                let n = sb.mics.len();
                for board in 0..3 {
                    let bufs = [n, n.saturating_sub(1).max(1), (n + 1).min(MICS)][(c + board) % 3];
                    let room = check_bit_identity(name, &p, &sb, sr, board, bufs, 0x9E37_79B9_7F4A_7C15 ^ (c * 3 + board) as u64, isa);
                    println!("{isa:?} {name}, board {board}, {bufs} buffers: {} samples at rest", room.rested);
                    // The dead room decays to exact zeros when flushing
                    // denormals, and rests.
                    if ftz && name == "dead" {
                        assert!(room.rested > 1_000, "{name} never rests");
                    }
                    // A fresh room rests in silence.
                    assert!(room.rested >= 1_000 - 64, "{name}: {} samples at rest", room.rested);
                }
            }
        }
    }

    #[test]
    fn blocks_match_the_sample_by_sample_room_bit_for_bit() {
        bit_identity(false);
    }

    #[test]
    fn blocks_match_the_sample_by_sample_room_bit_for_bit_flushing_denormals() {
        bit_identity(true);
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
