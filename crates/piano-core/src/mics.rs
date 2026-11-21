//! The microphones of a listening set-up: up to [`MAX_MICS`], each with its
//! position (on a grand and on an upright), pattern, aim, gain and pan.
//!
//! Every microphone is one output of the board's bank and one channel of
//! the room; the engine then mixes them down to stereo (gain and pan per
//! microphone) or hands them out one channel each (gain only), as
//! [`crate::soundboard::SoundboardParams::outputs_per_mic`] says.
//!
//! In a preset, `mics` is an array of tables:
//!
//! ```toml
//! [[soundboard.mics]]
//! at = [0.29, 0.35, 0.4]
//! upright = [0.71, 0.65, 0.4]
//! pattern = 0.5
//! pan = -1.0
//! ```
//!
//! Presets saved before 2026-11-17 wrote `mics = [[x, y, z], [x, y, z]]`
//! with `mics_upright` and one `mic_pattern`: they read as those two
//! microphones panned hard left and right
//! ([`crate::soundboard::deserialize_saved`]).

/// Microphones a set-up holds at most (outputs of the board's bank and
/// channels of the room, reserved whether used or not).
pub const MAX_MICS: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
pub struct Mic {
    /// Position (m, board coordinates, z above the board; x and y as
    /// fractions of the board's a and b when the set-up is relative).
    pub at: (f64, f64, f64),
    /// Position when the board is vertical (an upright): x down from the
    /// board's top, y across (the bass at y = b), z in front of the board.
    pub upright: (f64, f64, f64),
    /// Pattern a + (1 − a)·cos θ: 1 omni, 0.5 cardioid, 0 figure of eight.
    /// It weighs the direct sound by its angle to the aim, and the room's
    /// diffuse sound by √(a² + (1 − a)²/3).
    pub pattern: f64,
    /// Point the microphone is aimed at (the same coordinates as `at`);
    /// none: the board's centre.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub aim: Option<(f64, f64, f64)>,
    /// The same on a vertical board (the coordinates of `upright`).
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub aim_upright: Option<(f64, f64, f64)>,
    /// Gain (dB) of this microphone in the mix (or on its own channel).
    pub gain_db: f64,
    /// Place in the stereo mix, −1 left to 1 right (constant power).
    pub pan: f64,
}

impl Mic {
    pub const DEFAULT: Self = Self { at: (0.5, 0.5, 1.5), upright: (0.5, 0.5, 1.5), pattern: 1.0, aim: None, aim_upright: None, gain_db: 0.0, pan: 0.0 };

    /// An omni at `at` (also its upright place), centred.
    pub const fn omni(at: (f64, f64, f64)) -> Self {
        Self { at, upright: at, ..Self::DEFAULT }
    }

    /// Stereo gains (left, right) of the pan: cos and sin of (1 + pan)·π/4,
    /// so hard left and right give (1, 0) and (0, 1), the centre −3 dB each.
    pub fn pan_gains(&self) -> (f64, f64) {
        let t = (1.0 + self.pan.clamp(-1.0, 1.0)) * std::f64::consts::FRAC_PI_4;
        let (s, c) = t.sin_cos();
        // Exact at the ends (cos π/2 is 6e-17).
        let snap = |x: f64| if x.abs() < 1e-12 { 0.0 } else if (x - 1.0).abs() < 1e-12 { 1.0 } else { x };
        (snap(c), snap(s))
    }

    /// Share of a diffuse field's power the pattern takes, a² + (1 − a)²/3.
    pub fn diffuse_efficiency(&self) -> f64 {
        let a = self.pattern.clamp(0.0, 1.0);
        a * a + (1.0 - a) * (1.0 - a) / 3.0
    }
}

impl Default for Mic {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// The set-up's microphones: the first `len` of [`MAX_MICS`] slots (a fixed
/// array keeps the model `Copy`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mics {
    slots: [Mic; MAX_MICS],
    len: usize,
}

impl Mics {
    /// The microphones of `list` (at most [`MAX_MICS`], the rest dropped).
    pub const fn new(list: &[Mic]) -> Self {
        let mut slots = [Mic::DEFAULT; MAX_MICS];
        let mut i = 0;
        while i < list.len() && i < MAX_MICS {
            slots[i] = list[i];
            i += 1;
        }
        Self { slots, len: i }
    }

    /// Two omnis at `l` and `r` (also on an upright), panned hard left and right.
    pub const fn pair(l: (f64, f64, f64), r: (f64, f64, f64)) -> Self {
        Self::new(&[Mic { pan: -1.0, ..Mic::omni(l) }, Mic { pan: 1.0, ..Mic::omni(r) }])
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn as_slice(&self) -> &[Mic] {
        &self.slots[..self.len]
    }

    pub fn as_mut_slice(&mut self) -> &mut [Mic] {
        &mut self.slots[..self.len]
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Mic> {
        self.as_slice().iter()
    }

    /// Adds a microphone (false: already [`MAX_MICS`]).
    pub fn push(&mut self, m: Mic) -> bool {
        if self.len == MAX_MICS {
            return false;
        }
        self.slots[self.len] = m;
        self.len += 1;
        true
    }

    /// Removes microphone `i` (the later ones move down).
    pub fn remove(&mut self, i: usize) {
        if i < self.len {
            self.slots.copy_within(i + 1..self.len, i);
            self.len -= 1;
        }
    }

    pub fn map(mut self, f: impl Fn(Mic) -> Mic) -> Self {
        for m in self.as_mut_slice() {
            *m = f(*m);
        }
        self
    }
}

impl std::ops::Index<usize> for Mics {
    type Output = Mic;
    fn index(&self, i: usize) -> &Mic {
        &self.as_slice()[i]
    }
}

impl std::ops::IndexMut<usize> for Mics {
    fn index_mut(&mut self, i: usize) -> &mut Mic {
        &mut self.as_mut_slice()[i]
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Mics {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_seq(self.as_slice())
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Mics {
    /// A table per microphone, or (presets before 2026-11-17) a position
    /// per microphone: omnis, panned from left to right in order.
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(serde::Deserialize)]
        #[serde(untagged)]
        enum Entry {
            At((f64, f64, f64)),
            Mic(Mic),
        }
        let list: Vec<Entry> = serde::Deserialize::deserialize(d)?;
        if list.len() > MAX_MICS {
            return Err(serde::de::Error::custom(format!("{} microphones, at most {MAX_MICS}", list.len())));
        }
        let n = list.len();
        let mics: Vec<Mic> = list
            .into_iter()
            .enumerate()
            .map(|(i, e)| match e {
                Entry::Mic(m) => m,
                Entry::At(at) => Mic { pan: if n > 1 { -1.0 + 2.0 * i as f64 / (n - 1) as f64 } else { 0.0 }, ..Mic::omni(at) },
            })
            .collect();
        Ok(Self::new(&mics))
    }
}

/// Where the noises heard directly were set (the action's knock,
/// [`crate::action`]; the body's halo, [`crate::body`]): at the close pair of
/// `close.toml` (x and y as fractions of the board), the bench's
/// microphones, through that set-up's gain.
pub const NOISE_REFERENCE: [(f64, f64, f64); 2] = [(0.294118, 0.346154, 0.4), (0.705882, 0.653846, 0.4)];
pub const NOISE_REFERENCE_GAIN_DB: f64 = -7.4;

/// Per microphone of a set-up: the gain and the delay (samples) with which
/// it hears the noises heard directly, against [`NOISE_REFERENCE`].
///
/// Those noises are rendered once, on two sides, and were given to every
/// microphone alike: a pair 2 m away heard them as loud and as early as the
/// close pair, and a set-up mixing both (`mixed.toml`, the default) summed
/// two identical copies in phase, the far one raised by its own gain: on
/// E1 the action and the halo stood 13 dB higher against the strings than
/// through either pair alone.
///
/// The halo is the board and the case ringing, and the action stands
/// against them: each microphone hears them as it hears the board. Gain:
/// the board's radiation at the microphone against at the reference pair
/// (the built modes below 4 kHz, omni, mean |p|² per octave averaged in dB
/// over 125 Hz–4 kHz), times the set-up's gain against the reference's
/// (the board's weights carry `mic_gain_db`, the noises did not), both as
/// the listening set-ups give them, before the musician's controls move
/// them alike (`setup_gain_db`). Delay:
/// the mean path from the board and the action along the keys, against
/// the reference's. A set-up at the reference hears them as before, bit for
/// bit. Patterns and the lid are not counted.
pub fn noise_paths(sb: &crate::soundboard::Soundboard, setup_gain_db: f64, sample_rate: f64) -> Vec<(f64, usize)> {
    let p = &sb.params;
    let (a, b) = (p.a, p.b);
    let refs: Vec<[f64; 3]> = NOISE_REFERENCE.iter().map(|&(u, v, z)| [u * a, v * b, z]).collect();
    let mics: Vec<[f64; 3]> = p.mics.iter().map(|m| [m.at.0, m.at.1, m.at.2]).collect();
    let points: Vec<[f64; 3]> = mics.iter().chain(&refs).copied().collect();
    // The board's level at each point.
    let n = sb.modes.iter().take_while(|m| m.omega < std::f64::consts::TAU * 4000.0).count();
    let w = sb.radiation_points(&points, n);
    let bands: Vec<f64> = (0..6).map(|k| 125.0 * 2f64.powi(k)).collect();
    let level = |pt: usize| {
        let mut db = Vec::new();
        for &c in &bands {
            let (mut e, mut k) = (0.0, 0);
            for (md, row) in sb.modes[..n].iter().zip(&w) {
                let f = md.omega / std::f64::consts::TAU;
                if f >= c / 2f64.sqrt() && f < c * 2f64.sqrt() {
                    e += row[pt].norm_sqr();
                    k += 1;
                }
            }
            if k > 0 && e > 0.0 {
                db.push(10.0 * (e / k as f64).log10());
            }
        }
        if db.is_empty() { 0.0 } else { db.iter().sum::<f64>() / db.len() as f64 }
    };
    let nm = mics.len();
    let ref_db = 10.0 * (0.5 * (10f64.powf(level(nm) / 10.0) + 10f64.powf(level(nm + 1) / 10.0))).log10();
    // The mean path from the sources: the board and the action in front of it.
    let mut sources = Vec::new();
    for i in 0..8 {
        for j in 0..6 {
            sources.push([(i as f64 + 0.5) / 8.0 * a, (j as f64 + 0.5) / 6.0 * b, 0.0]);
        }
    }
    for j in 0..9 {
        sources.push([crate::soundboard::KEY_LINE_X - 0.12, (0.1 + 0.8 * j as f64 / 8.0) * b, 0.0]);
    }
    let path = |m: &[f64; 3]| sources.iter().map(|s| ((m[0] - s[0]).powi(2) + (m[1] - s[1]).powi(2) + (m[2] - s[2]).powi(2)).sqrt()).sum::<f64>() / sources.len() as f64;
    let ref_path = 0.5 * (path(&refs[0]) + path(&refs[1]));
    let setup = setup_gain_db - NOISE_REFERENCE_GAIN_DB;
    mics.iter()
        .enumerate()
        .map(|(i, m)| {
            // A reference microphone hears them as it did.
            if refs.contains(m) {
                return (if setup == 0.0 { 1.0 } else { 10f64.powf(setup / 20.0) }, 0);
            }
            let gain = 10f64.powf((level(i) - ref_db + setup) / 20.0);
            (gain, ((path(m) - ref_path).max(0.0) / crate::soundboard::C0 * sample_rate).round() as usize)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pans_keep_the_power() {
        for p in [-1.0, -0.5, 0.0, 0.3, 1.0] {
            let (l, r) = Mic { pan: p, ..Mic::DEFAULT }.pan_gains();
            assert!((l * l + r * r - 1.0).abs() < 1e-12, "{p}");
        }
        assert_eq!(Mic { pan: -1.0, ..Mic::DEFAULT }.pan_gains(), (1.0, 0.0));
        assert_eq!(Mic { pan: 1.0, ..Mic::DEFAULT }.pan_gains(), (0.0, 1.0));
    }

    #[test]
    fn a_list_grows_and_shrinks() {
        let mut m = Mics::pair((0.0, 0.0, 1.0), (1.0, 0.0, 1.0));
        assert_eq!(m.len(), 2);
        assert!(m.push(Mic::omni((2.0, 0.0, 1.0))));
        m.remove(0);
        assert_eq!(m[0].at, (1.0, 0.0, 1.0));
        assert_eq!(m[1].at, (2.0, 0.0, 1.0));
        assert_eq!(m.len(), 2);
    }
}
