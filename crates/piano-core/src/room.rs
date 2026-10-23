//! The room around the piano: early reflections by image sources and a
//! late reverberant tail by a feedback delay network (FDN), both driven by
//! the piano's radiated sound.
//!
//! The direct sound at each microphone is the soundboard's Rayleigh integral
//! (exact); the room adds what comes back from the walls. The piano is taken
//! as a point source at the board centre for the reflections, with source
//! signal s(t) = pressure at 1 m, estimated from the mics as p·r.
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
const MICS: usize = 2;

#[derive(Clone, Copy, Default)]
struct Tap {
    delay: usize,
    gain: f32,
    order: usize,
}

/// Early reflections and late tail, real-time part.
pub struct Room {
    // Source history (pressure at 1 m) for the early taps.
    hist: Vec<f32>,
    pos: usize,
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
    /// Source pre-delay into the FDN (first reflection time).
    pre: usize,
    out_gain: f32,
    /// Distances mic → source, to estimate the source from the mics.
    r_direct: [f64; MICS],
    /// √(diffuse-field efficiency of the mics / Q): the room is driven by
    /// the power the piano radiates in every direction, not by its level
    /// towards the mics, and heard through the mics' pattern.
    source_gain: f32,
    sample_rate: f64,
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
///   (directivity index ≈ 4–6 dB, Q ≈ 3–4, at mid and high frequencies);
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
    } else if sb.lid_angle > 0.0 && sb.lid_reflection > 0.0 {
        3.0
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
/// takes √(a² + (1 − a)²/3) of the diffuse sound.
pub fn source_gain(sb: &SoundboardParams, directivity: f64) -> f64 {
    let a = sb.mic_pattern.clamp(0.0, 1.0);
    ((a * a + (1.0 - a) * (1.0 - a) / 3.0) / directivity.max(1.0)).sqrt()
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
        let mic_room: [[f64; 3]; MICS] = std::array::from_fn(|m| to_room(p, sb, sb.mics[m]));
        // An upright against the wall y = 0: the board's radiation already
        // has that wall's first reflection.
        let own_wall = sb.vertical && sb.wall_distance > 0.0;
        let dist = |a: [f64; 3], b: [f64; 3]| ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt();
        let r_direct: [f64; MICS] = std::array::from_fn(|m| dist(src, mic_room[m]).max(0.3));

        // Image sources of a shoebox: per axis, image coordinate and number
        // of reflections for index n ∈ [−order, order].
        let beta = |alpha: f64| (1.0 - alpha).max(0.0).sqrt();
        let b_mid = beta(0.5 * (p.absorption_low + p.absorption_high));
        let mut taps = [[Tap::default(); MAX_TAPS]; MICS];
        let mut n_taps = [0usize; MICS];
        let o = p.order as i64;
        let mut first = f64::INFINITY;
        for m in 0..MICS {
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
                        list.push(Tap { delay, gain: gain as f32, order: refl });
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

        let mut room = Self {
            hist: vec![0.0; hist_len.max(pre + 1).next_power_of_two()],
            pos: 0,
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
            pre,
            out_gain: 1.0,
            source_gain: source_gain(sb, directivity) as f32,
            r_direct,
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
        for i in 0..n {
            let (l, _) = self.fdn_step(if i == 0 { 1.0 } else { 0.0 });
            e += (l as f64).powi(2);
        }
        e
    }

    pub fn reset(&mut self) {
        self.hist.fill(0.0);
        for l in &mut self.lines {
            l.fill(0.0);
        }
        self.lp_state = [0.0; LINES];
        self.order_state = [[0.0; 8]; MICS];
    }

    #[inline]
    fn fdn_step(&mut self, x: f32) -> (f32, f32) {
        // Read the line outputs, attenuated (two bands).
        let mut y = [0.0f32; LINES];
        for i in 0..LINES {
            let v = self.lines[i][self.line_pos[i]];
            self.lp_state[i] = self.lp_a * self.lp_state[i] + (1.0 - self.lp_a) * v;
            y[i] = self.g_hi[i] * v + (self.g_lo[i] - self.g_hi[i]) * self.lp_state[i];
        }
        // Householder feedback: y − (2/N)·Σy, plus the input with signs.
        let s = y.iter().sum::<f32>() * (2.0 / LINES as f32);
        let (mut l, mut r) = (0.0f32, 0.0f32);
        for i in 0..LINES {
            let sign = if i % 2 == 0 { 1.0 } else { -1.0 };
            let fb = y[i] - s + sign * x;
            let p = self.line_pos[i];
            self.lines[i][p] = fb;
            self.line_pos[i] = if p + 1 == self.line_len[i] { 0 } else { p + 1 };
            // Two decorrelated output mixes.
            l += if i % 4 < 2 { y[i] } else { -y[i] };
            r += if i % 3 == 0 { -y[i] } else { y[i] };
        }
        (l, r)
    }

    /// Adds the room's response to the microphone signals (in place).
    /// Real-time safe.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        let mask = self.hist.len() - 1;
        let r = [self.r_direct[0] as f32, self.r_direct[1] as f32];
        for (lo, ro) in left.iter_mut().zip(right.iter_mut()) {
            // Equivalent omnidirectional source: pressure at 1 m from the
            // mics, less the piano's directivity towards them.
            let s = 0.5 * (*lo * r[0] + *ro * r[1]) * self.source_gain;
            self.hist[self.pos & mask] = s;
            let mut early = [0.0f32; MICS];
            for (m, e) in early.iter_mut().enumerate() {
                let mut by_order = [0.0f32; 8];
                for t in &self.taps[m][..self.n_taps[m]] {
                    by_order[t.order] += t.gain * self.hist[(self.pos.wrapping_sub(t.delay)) & mask];
                }
                for (n, &x) in by_order.iter().enumerate().skip(1) {
                    let st = &mut self.order_state[m][n];
                    *st = self.order_lp[n] * *st + (1.0 - self.order_lp[n]) * x;
                    *e += *st;
                }
            }
            let x = self.hist[(self.pos.wrapping_sub(self.pre)) & mask];
            let (tl, tr) = self.fdn_step(x);
            *lo += early[0] + self.out_gain * tl;
            *ro += early[1] + self.out_gain * tr;
            self.pos = self.pos.wrapping_add(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tail_decays_at_the_sabine_rate() {
        for p in [RoomParams::HALL, RoomParams::CONCERT_HALL] {
            check_t60(p);
        }
    }

    #[test]
    fn an_upright_stands_against_its_wall() {
        let p = RoomParams::HALL;
        let sb = SoundboardParams { vertical: true, wall_distance: 0.2, a: 1.05, mics: [(0.3, 0.4, 0.5), (0.8, 1.0, 0.5)], ..SoundboardParams::GRAND_2M };
        let m = to_room(&p, &sb, sb.mics[0]);
        assert!((m[1] - 0.7).abs() < 1e-12 && (m[2] - (VERTICAL_FOOT + 1.05 - 0.3)).abs() < 1e-12, "{m:?}");
        assert_eq!(piano_directivity(&p, &sb), 2.0);
        assert_eq!(piano_directivity(&p, &SoundboardParams { wall_distance: 0.0, ..sb }), 1.5);
        let grand = SoundboardParams { lid_angle: 35.0, ..SoundboardParams::GRAND_2M };
        assert_eq!(piano_directivity(&p, &grand), 3.0);
        assert_eq!(piano_directivity(&p, &SoundboardParams { lid_angle: 0.0, ..grand }), 2.0);
        assert_eq!(piano_directivity(&RoomParams { directivity: 4.0, ..p }, &grand), 4.0);
        let mut room = Room::new(&p, &sb, 48_000.0);
        let (mut l, mut r) = (vec![0.0f32; 48_000], vec![0.0f32; 48_000]);
        l[0] = 1.0;
        room.process(&mut l, &mut r);
        assert!(l.iter().chain(&r).all(|v| v.is_finite()));
    }

    fn check_t60(p: RoomParams) {
        let sr = 48_000.0;
        let sb = SoundboardParams { mics: [(0.5, 0.4, 1.5), (1.2, 0.9, 1.5)], ..SoundboardParams::GRAND_2M };
        let mut room = Room::new(&p, &sb, sr);
        let n = (3.0 * sr) as usize;
        let (mut l, mut r) = (vec![0.0f32; n], vec![0.0f32; n]);
        l[0] = 1.0;
        r[0] = 1.0;
        room.process(&mut l, &mut r);
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
