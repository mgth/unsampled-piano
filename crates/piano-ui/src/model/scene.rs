//! The instrument as the model describes it, in three dimensions and to
//! scale: one list of triangles and lines that the plan (egui) and the 3D
//! view (GPU) both draw. Built from the model alone (static and editor
//! views) or with the engine's state laid on it (live view).
//!
//! World axes: x across the keyboard (treble at 0, bass towards −x), y
//! away from the pianist, z up; metres. A grand's board lies in z = 0, its
//! plate coordinates (x along the strings, y across, z above) turned into
//! (−y, x, z). An upright's board stands in y = 0, its x pointing down
//! from its top and its z towards the pianist.
//!
//! What the model has no geometry for is drawn plainly: the board, the
//! case and the lid follow the model's outline (or its rectangle), its
//! polygon drawn as the curve it stands for ([`rounded`]); the strings run from their key's place across the agraffes to their bridge
//! point, which crosses the bass over the tenor as on overstrung pianos;
//! legs, keyboard and pedals only situate the instrument.

use egui::Color32;
use piano_core::params::PianoModel;
use piano_core::soundboard::{Outline, SoundboardParams, LID_OVERHANG};
use piano_core::observe::KeyState;
use piano_core::Observation;

use super::cam::{add, cross, dot, norm, scale, sub, V3};
use crate::sketches::{self, Part};

mod action;
mod lettering;
mod name;
mod plate;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct Vertex {
    pub pos: V3,
    pub normal: V3,
    /// sRGB, 0–1, straight alpha.
    pub color: [f32; 4],
    /// What it is made of ([`Material`] as a number), for the realistic
    /// view.
    pub mat: f32,
    /// A line's wire diameter (m), 0 for a stroke of the drawing.
    pub width: f32,
}

/// What a part is made of, for the 3D view's realistic look (the plain
/// views keep to its colour): the shader's numbering. Most parts' come
/// from the colour they are drawn in ([`material_of`]); a colour that
/// stands for no material (a part pointed at, a level) takes the one the
/// part being drawn sets ([`Builder::mat`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Material {
    /// A satin paint.
    #[default]
    Paint = 0,
    /// The case, lid, legs and lyre: polished black lacquer.
    Lacquer = 1,
    /// The soundboard: varnished spruce.
    Spruce = 2,
    /// Keys' sticks, the action, bridges: maple, hornbeam.
    Wood = 3,
    /// The plate: cast iron in gold bronze paint.
    Iron = 4,
    Steel = 5,
    /// The wound strings' copper.
    Copper = 6,
    Brass = 7,
    /// Felts and cloths.
    Felt = 8,
    Leather = 9,
    /// The white keys' tops, the black keys.
    KeyTop = 10,
    Ebony = 11,
    /// The keys' leads.
    Lead = 12,
    /// The name's gold leaf.
    Gilt = 13,
}

/// A named part: where its label goes, and the page that sets it.
#[derive(Clone, Debug, PartialEq)]
pub struct Label {
    pub at: V3,
    pub text: String,
    pub page: &'static str,
}

#[derive(Default)]
pub struct Scene {
    /// Opaque triangles, three vertices each.
    pub tris: Vec<Vertex>,
    /// Translucent ones (frame, lid), drawn last.
    pub glass: Vec<Vertex>,
    /// The lid: see-through in the drawing (drawn as [`Self::glass`]),
    /// solid lacquer in the realistic look.
    pub lid: Vec<Vertex>,
    /// Lines, two vertices each.
    pub lines: Vec<Vertex>,
    pub labels: Vec<Label>,
    pub min: V3,
    pub max: V3,
    /// The instrument's own extent, without the microphones (what the 3D
    /// view frames).
    pub body: (V3, V3),
    /// The board stands (an upright).
    pub vertical: bool,
    /// The floor's height (m).
    pub floor: f32,
    /// The soundboard's extent.
    pub board: (V3, V3),
    /// The engine's state laid on it: the board coloured by [`heat`].
    pub live: bool,
    /// The keys where they can be played (none under a shut fallboard):
    /// each its note and a box (min, max) round a part of it at rest, the
    /// part the pianist sees.
    pub keys: Vec<(u8, V3, V3)>,
}

impl Scene {
    /// The layers ([`Layer::PARTS`]) as one scene.
    pub fn merged(layers: &[&Scene]) -> Scene {
        let cat = |part: fn(&Scene) -> &[Vertex]| layers.iter().flat_map(|s| part(s).iter().copied()).collect::<Vec<_>>();
        let drawn = || layers.iter().filter(|s| !s.tris.is_empty() || !s.lines.is_empty());
        let union = |of: fn(&Scene) -> (V3, V3)| {
            drawn().map(|s| of(s)).reduce(|a, b| (std::array::from_fn(|i| a.0[i].min(b.0[i])), std::array::from_fn(|i| a.1[i].max(b.1[i])))).unwrap_or(of(layers[0]))
        };
        let (min, max) = union(|s| (s.min, s.max));
        Scene {
            tris: cat(|s| &s.tris),
            glass: cat(|s| &s.glass),
            lid: cat(|s| &s.lid),
            lines: cat(|s| &s.lines),
            labels: layers.iter().flat_map(|s| s.labels.iter().cloned()).collect(),
            min,
            max,
            body: union(|s| s.body),
            vertical: layers[0].vertical,
            floor: layers[0].floor,
            board: layers[0].board,
            live: layers.iter().any(|s| s.live),
            keys: layers.iter().flat_map(|s| s.keys.iter().copied()).collect(),
        }
    }

    /// The key a ray from `from` along `dir` meets first, if any, and
    /// where along it (0 at its front … 1 at the back of its visible part).
    pub fn key_at(&self, from: V3, dir: V3) -> Option<(u8, f32)> {
        let mut best: Option<(f32, u8)> = None;
        for &(note, lo, hi) in &self.keys {
            // The slabs' method: the ray's stretch within each pair of
            // faces, the box's where they overlap.
            let (mut t0, mut t1) = (0.0f32, f32::MAX);
            for i in 0..3 {
                let (a, b) = (lo[i].min(hi[i]), lo[i].max(hi[i]));
                if dir[i].abs() < 1e-9 {
                    if from[i] < a || from[i] > b {
                        t1 = -1.0;
                    }
                    continue;
                }
                let (ta, tb) = ((a - from[i]) / dir[i], (b - from[i]) / dir[i]);
                t0 = t0.max(ta.min(tb));
                t1 = t1.min(ta.max(tb));
            }
            if t0 <= t1 && best.is_none_or(|(t, _)| t0 < t) {
                best = Some((t0, note));
            }
        }
        let (t, note) = best?;
        let (front, back) = self.keys.iter().filter(|k| k.0 == note).fold((f32::MAX, f32::MIN), |(f, b), k| (f.min(k.1[1].min(k.2[1])), b.max(k.1[1].max(k.2[1]))));
        let y = from[1] + t * dir[1];
        Some((note, ((y - front) / (back - front).max(1e-6)).clamp(0.0, 1.0)))
    }
}

/// What the live view lays on the instrument: the engine's state, and the
/// levels the view derived from it (0 at rest … 1 loudest).
pub struct Live<'a> {
    pub obs: &'a Observation,
    /// Per key, its strings' level.
    pub keys: &'a [f32; 128],
    /// The board's motion on its grid ([`BOARD_GRID`]), row by row along y.
    pub board: Option<&'a [f32]>,
    /// Seconds, for the strings' (slowed, exaggerated) motion.
    pub time: f32,
}

#[derive(Clone, Copy)]
pub struct Options<'a> {
    pub key: u8,
    pub lit: &'a [Part],
    pub lid: bool,
    pub frame: bool,
    pub mics: bool,
    /// The case (rim, arms, belly rail) drawn, or left out to show the
    /// action.
    pub case: bool,
    /// A grand's fallboard shut over the keys.
    pub fallboard: bool,
    /// How solid a grand's front is above its fallboard, up to the case's
    /// top (0 left out … 1 solid).
    pub front: f32,
    /// Text colour of the theme (the key shown).
    pub ink: Color32,
}

/// Board grid: points along x (the strings) and across.
pub const BOARD_GRID: (usize, usize) = (36, 28);

/// Bridge height above the board, the strings' height on it, and the bass
/// strings' rise over the others (they cross over them).
const BRIDGE_H: f64 = 0.03;
const STRING_Z: f64 = 0.035;
const BASS_RISE: f64 = 0.02;
/// A built board's ([`SoundboardParams::structure`]): its bass strings' least
/// height on their bridge over the main bridge's highest strings (they
/// cross over them), on a cap that narrow over the bridge's body, which is
/// as thick as the engine has it.
const BASS_OVER: f64 = 0.02;
const BASS_CAP: f64 = 0.016;
/// The board's thickness where the model has none (no built board).
const BOARD_T: f64 = 0.012;
/// A built board's ribs: sections along each.
const RIB_SECTIONS: usize = 12;
/// More of them over each feathered end of a rib
/// ([`piano_core::fem::BoardStructure::rib_feather`]).
const RIB_FEATHER_SECTIONS: usize = 5;
/// The plate's riser under the hitch pins where their strings' bridge
/// stands high: its half-width, its top under the strings.
const HITCH_RISER: f64 = 0.009;
const HITCH_RISER_UNDER: f64 = 0.007;
/// A unison's strings across, centre to centre.
const UNISON_GAP: f64 = 0.0025;
/// A unison of thick strings (the bass's wound ones) stands wider: its
/// strings that far clear of each other, as long as the unison is no wider
/// than this from side to side ([`unison_gap`]).
const UNISON_CLEAR: f64 = 0.0012;
const UNISON_SPAN: f64 = 0.011;
/// A grand's hammer line in front of its board: room for its hammers,
/// leaning back at rest and forward on the strings, to swing past the
/// board's edge, and for the dampers' wires behind them ([`action`]); the
/// pin block's back edge in front of it.
const HAMMER_CLEAR: f64 = 0.045;
/// The least a grand's hammers' line stands in front of its board's front
/// edge where the strings' lengths ask for it that near: on a concert
/// grand's photograph the top hammers strike 1 cm in front of the strip
/// the board ends on.
const HAMMER_EDGE: f64 = 0.01;
/// The most a grand's hammers' line may slant against its board's axis
/// (along per metre across: 3°).
const HAMMER_SLANT: f64 = 0.0524;
const BLOCK_CLEAR: f64 = 0.015;
/// The tuning pins (7 mm), one per string in front of the agraffes (see
/// [`tuning_pins`]): the first row's distance, the rows' spacing.
const PIN_R: f64 = 0.0035;
const PIN_GAP: f64 = 0.025;
const PIN_ROW: f64 = 0.016;
/// A grand's, as a concert grand's are (read off a photograph of one from
/// above, within 1 cm): the first row behind the broad felt in front of the
/// agraffes, or that far from the capo bar where the strings bear on bars
/// in front of it (`frame.duplex_far`); the rows' spacing, each unison's pins
/// one behind the other two rows apart, each on its own string's line
/// carried straight on (the leftmost string's nearest), and every other
/// unison's one row further: unisons side by side in a quincunx, a row's
/// pins two keys apart.
const PIN_GAP_GRAND: f64 = 0.048;
const PIN_GAP_CAPO: f64 = 0.09;
const PIN_ROW_GRAND: f64 = 0.017;
/// A grand's unison's pins across where its strings cannot run straight
/// on to them: that much of an even share of the keys' pitch from one to
/// the next, the whole that much of a coil's radius to the right (the
/// values that leave the strings most room on the default grand, whose
/// bass unisons stand 10 mm apart square to their strings: there the
/// strings graze the pins they pass).
const PIN_SPREAD_GRAND: f64 = 0.75;
const PIN_SHIFT_GRAND: f64 = 0.7;
/// The room a string is left beside a pin it passes; and, of what a pin
/// gives way, the least that clears its neighbour's string.
const PIN_PASS: f64 = 0.0004;
const PIN_TURN: f64 = 0.25;
/// Under a capo bar ([`piano_core::structure::FrameParams::capo_bar`]) the
/// strings bear on short bars of the plate in front of it, one to each
/// group of that many unisons from their section's lowest, straight and
/// slanted (`frame.duplex_group`, `frame.duplex_far`, `frame.duplex_near`:
/// the engine's short lengths, [`piano_core::duplex`]), stepping back
/// from one group to the next; the bar's width, how far its top lifts the
/// strings over the capo's height; the felt in front of it, its distance
/// from the bar and its width.
const DUPLEX_BAR_W: f64 = 0.008;
const DUPLEX_RISE: f64 = 0.002;
const DUPLEX_FELT: f64 = 0.016;
const DUPLEX_FELT_W: f64 = 0.018;
/// A pin's lean away from its string's pull (rad, some 4°); its coil
/// (three turns of wire) below the strings' height, its height and the
/// wire's thickness; its square head's height, its top above the coil;
/// the bushing it stands in, radius and height.
const PIN_LEAN: f64 = 0.07;
const PIN_COIL_DROP: f64 = 0.007;
const PIN_COIL_H: f64 = 0.003;
const PIN_WIRE: f64 = 0.001;
/// The coil: its turns, and the straight lengths a turn is drawn in.
const PIN_COIL_TURNS: usize = 3;
const PIN_COIL_STEPS: usize = 16;
const PIN_HEAD: f64 = 0.011;
const PIN_TOP: f64 = 0.02;
const PIN_BUSH_R: f64 = 0.0055;
const PIN_BUSH_H: f64 = 0.0015;
/// An agraffe, as they are made: a hexagonal flange screwed into the
/// plate, a neck, and a head the unison's strings go through, its flats
/// facing along them: the head's half-width, and its growth per string
/// beyond the first; the flange's radius beyond the head's and height;
/// the head's extent below and above the strings; the holes' radius; the
/// longest neck, beyond which the agraffe stands on a boss of the plate
/// (the bass's, its strings higher), and the boss's radius beyond the
/// flange's.
const AGRAFFE_HEAD: f64 = 0.0028;
const AGRAFFE_HEAD_PER: f64 = 0.0009;
const AGRAFFE_FLANGE: f64 = 0.0012;
const AGRAFFE_FLANGE_H: f64 = 0.003;
const AGRAFFE_BELOW: f64 = 0.0045;
const AGRAFFE_ABOVE: f64 = 0.0035;
const AGRAFFE_HOLE_R: f64 = 0.0006;
const AGRAFFE_NECK: f64 = 0.004;
const AGRAFFE_BOSS: f64 = 0.0025;
/// The felt strip on the plate under the strings' dead lengths, between
/// the agraffes and the pins: its middle in front of the agraffes, width
/// and thickness.
const UNDER_FELT_AT: f64 = 0.0125;
const UNDER_FELT_W: f64 = 0.012;
const UNDER_FELT_T: f64 = 0.004;
/// A grand's: a broad band from just in front of the agraffes to the pins
/// (some 35 mm on a concert grand's photograph).
const UNDER_FELT_AT_GRAND: f64 = 0.026;
const UNDER_FELT_W_GRAND: f64 = 0.03;
/// The hitch pins: their radius, how far they stand inside the case's wall
/// along the string, the rear lengths (a few cm in the top treble, growing
/// with the speaking length to some 20 cm in the bass, as on grands) and
/// how much lower they hold the strings than the bridge.
const HITCH_R: f64 = 0.0025;
const HITCH_INSET: f64 = 0.05;
const HITCH_MIN: f64 = 0.03;
const HITCH_BASE: f64 = 0.04;
const HITCH_SHARE: f64 = 0.1;
const HITCH_DROP: f64 = 0.006;
/// The bridge pins: radius, how far before and after the bridge's line
/// along the string, and the string's side of each.
const BRIDGE_PIN_R: f64 = 0.0009;
const BRIDGE_PIN_ALONG: f64 = 0.006;
const BRIDGE_PIN_SIDE: f64 = 0.0012;
/// The felt braid across the rear lengths: its width along the strings,
/// its thickness, and how far above and below the strings it weaves.
const BRAID_W: f32 = 0.012;
const BRAID_T: f32 = 0.0016;
const BRAID_WEAVE: f64 = 0.0035;
/// The pin block's top (under the plate) and thickness; the case's front
/// panel and the belly rail's thickness.
const PIN_BLOCK_TOP: f64 = 0.02;
const PIN_BLOCK_T: f64 = 0.032;
const FRONT_T: f64 = 0.02;
const BELLY_RAIL: f64 = 0.05;
const BELLY_AT: f64 = 0.06;
/// The keyboard: depth of the white keys. A grand's keybed, its underside
/// flush with the case's, its thickness and the keys' bottom above it; the
/// keys' top above the floor; the key slip's thickness.
const KEY_DEPTH: f64 = 0.15;
const KEYBED_T: f64 = 0.035;
const KEY_FRAME: f64 = 0.04;
const KEY_HEIGHT: f64 = 0.72;
const KEY_SLIP: f64 = 0.018;
/// A grand's cheek blocks' least width, between the keys and the case's
/// arms; the bass's agraffes' least distance from the board's bass side.
const KEY_CHEEK: f64 = 0.03;
/// A real keyboard's white key (m): its 52 make 1.227 m, whatever the
/// instrument's size.
const WHITE_KEY: f64 = 0.0236;
/// A grand's hammers stand further apart than its keys where its strings
/// do (the keys slant to them): on a concert grand the dampers' heads of a
/// section stand 14.35 mm apart for keys 13.77 mm apart (a photograph from
/// above, 18 heads over 235 pixels at 1.1 mm a pixel), the sections' gaps
/// on top.
const HAMMER_FAN: f64 = 1.042;
/// How far back from the board's front (of its length a) the front edge's
/// ends may lie, and how much of its width b that edge spans at least.
const FRONT_WITHIN: f64 = 0.25;
const FRONT_SPAN: f64 = 0.5;
const AGRAFFE_EDGE: f64 = 0.025;
/// An upright's board's bottom above the floor (the room's); the back of
/// its keys' visible part in front of the board, under the fallboard.
pub const UPRIGHT_BASE: f64 = piano_core::room::VERTICAL_FOOT;
const UPRIGHT_KEYS_Y: f64 = -0.27;
/// An upright's case: its sides' and top's thickness, its cheek blocks'
/// least width beside the keys, its lower panel's front behind the
/// fallboard's, its plinth's height.
const U_SIDE_T: f64 = 0.025;
const U_TOP_T: f64 = 0.025;
const U_CHEEK: f64 = 0.04;
const U_LOWER_BACK: f64 = 0.03;
const U_PLINTH: f64 = 0.06;
/// An upright's rim's front, in front of its board.
const U_RIM_FRONT: f64 = 0.015;
/// A grand's outer rim, the case's wall up to the lid: of the rim's
/// thickness, the rest is the inner rim under the board's glued edge
/// (makers give 22–29 mm outer, 38–54 mm inner).
const OUTER_RIM: f64 = 0.025;
/// The name on a grand's fallboard: its middle's height above the keys.
/// The fallboard shut: its top's height above the keys at the name board
/// and over their front, its thickness, its front edge's radius, the lip's
/// thickness in front of the keys.
const NAME_UP: f64 = 0.05;
const FALL_BACK: f64 = 0.085;
const FALL_FRONT: f64 = 0.018;
const FALL_T: f64 = 0.014;
const FALL_ROUND: f64 = 0.006;
const FALL_LIP: f64 = 0.006;
/// The lid's thickness, and the angle from which its front flap lies
/// folded right back on it.
const LID_T: f64 = 0.02;
/// The moulding under the flap's front edge, which closes the top of the
/// case's front (the music desk's cavity) and goes with the flap: its
/// height, about 2 in on a grand (an owner's measure; makers publish none).
const LID_DROP: f64 = 0.05;
const LID_FLAP_DEG: f64 = 8.0;
/// The lid's prop: how far behind the board's front it stands on the
/// treble side's rim, its section, and the lid's angle from which it is up.
const PROP_AT: f64 = 0.35;
const PROP_W: f64 = 0.03;
const PROP_T: f64 = 0.016;
const PROP_FROM_DEG: f64 = 3.0;
/// The pedals' spacing across (m).
const PEDAL_PITCH: f32 = 0.07;

/// Bends up to this angle (degrees) are shaded as a curve, sharper ones as
/// corners.
const SMOOTH_DEG: f64 = 40.0;
/// An outline's bends sharper than this (degrees) stay corners in
/// [`rounded`].
const CORNER_DEG: f64 = 60.0;
/// [`rounded`]'s curves turn about this much (degrees) from one point to
/// the next.
const ROUND_STEP_DEG: f64 = 1.5;

const STEEL: Color32 = Color32::from_rgb(170, 172, 178);
const COPPER: Color32 = Color32::from_rgb(184, 115, 70);
const IRON: Color32 = Color32::from_rgb(190, 160, 90);
const BRASS: Color32 = Color32::from_rgb(222, 184, 98);
const PIN: Color32 = Color32::from_rgb(168, 170, 176);
const BUSHING: Color32 = Color32::from_rgb(150, 104, 72);
const HITCH: Color32 = Color32::from_rgb(150, 152, 160);
/// A built board's ribs, a little paler than the board.
const RIB_WOOD: Color32 = Color32::from_rgb(204, 150, 70);
const BRIDGE_PIN: Color32 = Color32::from_rgb(196, 198, 206);
const CASE: Color32 = Color32::from_rgb(40, 34, 32);
const IVORY: Color32 = Color32::from_rgb(236, 232, 222);
const EBONY: Color32 = Color32::from_rgb(28, 26, 26);
/// The strip of felt behind the keys.
const FELT: Color32 = Color32::from_rgb(122, 28, 36);
/// The one under the strings' dead lengths.
const UNDER_FELT: Color32 = Color32::from_rgb(176, 48, 36);
const DAMPER_WOOD: Color32 = Color32::from_rgb(24, 22, 23);
const DAMPER_FELT: Color32 = Color32::from_rgb(226, 222, 212);
const MIC: [Color32; piano_core::mics::MAX_MICS] = crate::mics::MIC_COLORS;
const LYRE_ROD: Color32 = Color32::from_rgb(64, 64, 70);

/// The material each of the scene's own colours stands for.
const MATERIALS: [(Color32, Material); 19] = [
    (name::GILT, Material::Gilt),
    (STEEL, Material::Steel),
    (PIN, Material::Steel),
    (HITCH, Material::Steel),
    (BRIDGE_PIN, Material::Steel),
    (LYRE_ROD, Material::Steel),
    (COPPER, Material::Copper),
    (IRON, Material::Iron),
    (BRASS, Material::Brass),
    (BUSHING, Material::Wood),
    (DAMPER_WOOD, Material::Lacquer),
    (sketches::BRIDGE, Material::Wood),
    (sketches::WOOD, Material::Spruce),
    (CASE, Material::Lacquer),
    (IVORY, Material::KeyTop),
    (EBONY, Material::Ebony),
    (FELT, Material::Felt),
    (UNDER_FELT, Material::Felt),
    (DAMPER_FELT, Material::Felt),
];

/// The material the colour `c` stands for, if any.
fn material_of(c: [f32; 4]) -> Option<Material> {
    let k = |x: f32| (x * 255.0).round() as u8;
    let c = Color32::from_rgb(k(c[0]), k(c[1]), k(c[2]));
    MATERIALS.iter().chain(&action::MATERIALS).find(|(k, _)| *k == c).map(|&(_, m)| m)
}

/// A string's diameter, from its mass per length as if all steel (a
/// wound string's a little under its winding's), and its plain wire's
/// beyond the bridge and round its tuning pin (a wound string's core).
fn string_d(model: &PianoModel, m: u8) -> (f32, f32) {
    let d = 2.0 * (model.note(m).mu / (action::STEEL_DENSITY * std::f64::consts::PI)).sqrt();
    (d as f32, d.min(0.0016) as f32)
}

/// Key `m`'s strings across, centre to centre: [`UNISON_GAP`], more where
/// its strings are thicker than that leaves room for.
pub fn unison_gap(model: &PianoModel, m: u8) -> f64 {
    let strings = model.note(m).strings;
    if strings < 2 {
        return UNISON_GAP;
    }
    let d = string_d(model, m).0 as f64;
    UNISON_GAP.max((d + UNISON_CLEAR).min((UNISON_SPAN - d) / (strings as f64 - 1.0)))
}

fn rgba(c: Color32, a: f32) -> [f32; 4] {
    [c.r() as f32 / 255.0, c.g() as f32 / 255.0, c.b() as f32 / 255.0, a]
}

fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()))
}

/// The board's motion on its wood: lighter and warmer where it moves, a
/// glow rather than a flare (the low levels barely tint it).
pub fn heat(level: f32) -> Color32 {
    let l = level.clamp(0.0, 1.0);
    mix(sketches::WOOD.gamma_multiply(0.8), Color32::from_rgb(244, 200, 128), 0.75 * l * l)
}

/// Plate coordinates to world.
#[derive(Clone, Copy)]
struct Place {
    vertical: bool,
    top: f64,
}

impl Place {
    /// A plate direction in world coordinates.
    fn dir(&self, x: f64, y: f64, z: f64) -> V3 {
        norm(sub(self.at(x, y, z), self.at(0.0, 0.0, 0.0)))
    }

    fn at(&self, x: f64, y: f64, z: f64) -> V3 {
        if self.vertical { [-y as f32, -z as f32, (self.top - x) as f32] } else { [-y as f32, x as f32, z as f32] }
    }
}

/// Which of the instrument's parts a scene holds: all of them, or one of
/// three layers, each built again only when what it shows changes: what
/// stays still while the instrument plays (built once), what the pianist
/// moves (keys, action, dampers, pedals: at each key or pedal), and what
/// sounds (the strings' speaking lengths, the board: at each look while
/// they do).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layer {
    All,
    Still,
    Played,
    Sounding,
}

impl Layer {
    /// The layers a scene is drawn in, in their order.
    pub const PARTS: [Layer; 3] = [Layer::Still, Layer::Played, Layer::Sounding];

    /// Whether the parts of layer `of` are in it.
    fn has(self, of: Layer) -> bool {
        self == Layer::All || self == of
    }
}

struct Builder {
    s: Scene,
    place: Place,
    layer: Layer,
    /// What is being drawn goes into the scene (is of its layer): else
    /// the drawing calls do nothing.
    draw: bool,
    /// What the part being drawn is made of where its colour says
    /// nothing ([`material_of`]).
    mat: Material,
    /// The last colour drawn, the material it had then and the one it
    /// gave (a colour is drawn many times over).
    last: ([f32; 4], Material, Material),
}

impl Builder {
    fn new(place: Place, layer: Layer) -> Self {
        Self { s: Scene::default(), place, layer, draw: layer.has(Layer::Still), mat: Material::Paint, last: ([-1.0; 4], Material::Paint, Material::Paint) }
    }

    /// What follows is of layer `of`.
    fn layer_of(&mut self, of: Layer) {
        self.draw = self.layer.has(of);
    }

    #[inline]
    fn vertex(&mut self, pos: V3, normal: V3, color: [f32; 4]) -> Vertex {
        if self.last.0 != color || self.last.1 != self.mat {
            self.miss(color);
        }
        Vertex { pos, normal, color, mat: self.last.2 as u8 as f32, width: 0.0 }
    }

    #[cold]
    fn miss(&mut self, color: [f32; 4]) {
        self.last = (color, self.mat, material_of(color).unwrap_or(self.mat));
    }

    fn tri(&mut self, glass: bool, p: [V3; 3], c: [[f32; 4]; 3]) {
        if !self.draw {
            return;
        }
        let n = norm(cross(sub(p[1], p[0]), sub(p[2], p[0])));
        for (pos, color) in p.into_iter().zip(c) {
            let v = self.vertex(pos, n, color);
            if glass { &mut self.s.glass } else { &mut self.s.tris }.push(v);
        }
    }

    /// A quad with a normal at each corner (smooth shading).
    fn quad_n(&mut self, p: [V3; 4], n: [V3; 4], c: [f32; 4]) {
        if !self.draw {
            return;
        }
        for k in [0, 1, 2, 0, 2, 3] {
            let v = self.vertex(p[k], n[k], c);
            self.s.tris.push(v);
        }
    }

    /// A wall of constant section along the plate polyline `line`, between
    /// `a` and `b` metres to its right (outwards on a counter-clockwise
    /// outline), from `z0` up to each segment's `top`: one piece, mitred at
    /// the joins, shaded smooth where the line bends by less than
    /// [`SMOOTH_DEG`], with a step where two segments' tops differ.
    #[allow(clippy::too_many_arguments)]
    fn wall(&mut self, line: &[(f64, f64)], closed: bool, a: f64, b: f64, z0: f64, top: &dyn Fn(usize) -> f64, c: Color32) {
        if !self.draw {
            return;
        }
        let n = line.len();
        if n < 2 {
            return;
        }
        let segs = if closed { n } else { n - 1 };
        let c = rgba(c, 1.0);
        let right = |k: usize| {
            let (p, q) = (line[k], line[(k + 1) % n]);
            let (dx, dy) = (q.0 - p.0, q.1 - p.1);
            let l = dx.hypot(dy).max(1e-12);
            (dy / l, -dx / l)
        };
        // Per vertex: the segments on either side (None at an open end),
        // the mitre direction and its length factor, and whether the bend
        // is smooth.
        let joint = |i: usize| -> (Option<usize>, Option<usize>) {
            let prev = if i > 0 { Some(i - 1) } else if closed { Some(n - 1) } else { None };
            let next = if i < segs { Some(i) } else { None };
            (prev, next)
        };
        let mut mitre = Vec::with_capacity(n);
        for i in 0..n {
            let (prev, next) = joint(i);
            let (m, smooth) = match (prev.map(right), next.map(right)) {
                (Some(u), Some(v)) => {
                    let (mx, my) = (u.0 + v.0, u.1 + v.1);
                    let l = mx.hypot(my).max(1e-12);
                    let m = (mx / l, my / l);
                    let k = 1.0 / (m.0 * v.0 + m.1 * v.1).max(0.3);
                    ((m.0 * k, m.1 * k), u.0 * v.0 + u.1 * v.1 > SMOOTH_DEG.to_radians().cos())
                }
                (Some(u), None) | (None, Some(u)) => (u, false),
                (None, None) => ((0.0, 0.0), false),
            };
            mitre.push((m, smooth));
        }
        let at = |i: usize, off: f64| (line[i].0 + mitre[i].0.0 * off, line[i].1 + mitre[i].0.1 * off);
        let pl = self.place;
        let w = |p: (f64, f64), z: f64| pl.at(p.0, p.1, z);
        let (up, down) = (pl.dir(0.0, 0.0, 1.0), pl.dir(0.0, 0.0, -1.0));
        for k in 0..segs {
            let (i, j) = (k, (k + 1) % n);
            let h = top(k);
            let (li, lj, ri, rj) = (at(i, a), at(j, a), at(i, b), at(j, b));
            // The faces' own normal, or the mitre's at a smooth bend.
            let face = right(k);
            let side = |v: usize| if mitre[v].1 { mitre[v].0 } else { face };
            let (si, sj) = (side(i), side(j));
            let out = |s: (f64, f64)| pl.dir(s.0, s.1, 0.0);
            let inn = |s: (f64, f64)| pl.dir(-s.0, -s.1, 0.0);
            self.quad_n([w(li, h), w(lj, h), w(rj, h), w(ri, h)], [up; 4], c);
            self.quad_n([w(li, z0), w(ri, z0), w(rj, z0), w(lj, z0)], [down; 4], c);
            self.quad_n([w(ri, z0), w(ri, h), w(rj, h), w(rj, z0)], [out(si), out(si), out(sj), out(sj)], c);
            self.quad_n([w(li, z0), w(lj, z0), w(lj, h), w(li, h)], [inn(si), inn(si), inn(sj), inn(sj)], c);
        }
        // Steps between segments of different heights, and open ends.
        for i in 0..n {
            let (prev, next) = joint(i);
            let (lo, hi, along) = match (prev, next) {
                (Some(p), Some(q)) if (top(p) - top(q)).abs() > 1e-9 => {
                    let lower = if top(p) < top(q) { p } else { q };
                    let r = right(lower);
                    (top(p).min(top(q)), top(p).max(top(q)), (-r.1, r.0))
                }
                (Some(p), None) => (z0, top(p), (-right(p).1, right(p).0)),
                (None, Some(q)) => (z0, top(q), (-right(q).1, right(q).0)),
                _ => continue,
            };
            let d = pl.dir(along.0, along.1, 0.0);
            let (l, r) = (at(i, a), at(i, b));
            self.quad_n([w(l, lo), w(r, lo), w(r, hi), w(l, hi)], [d; 4], c);
        }
    }

    /// A flat solid from `points` (u, v), its `loops` (first point,
    /// count; outlines counter-clockwise, holes clockwise) and its face's
    /// counter-clockwise `tris`, between the heights `h.0` and `h.1`;
    /// `at(u, v, h)` places it. Its bottom face is left out on the
    /// surface it stands on (h 0).
    #[allow(clippy::too_many_arguments)]
    fn prism(&mut self, points: &[(f64, f64)], loops: &[(usize, usize)], tris: &[[usize; 3]], h: (f64, f64), at: &dyn Fn(f64, f64, f64) -> V3, c: Color32) {
        if !self.draw {
            return;
        }
        let c = rgba(c, 1.0);
        let up = sub(at(0.0, 0.0, 1.0), at(0.0, 0.0, 0.0));
        let faces = if h.0 > 0.0 { vec![(h.1, 1.0f32), (h.0, -1.0)] } else { vec![(h.1, 1.0)] };
        for t in tris {
            let [p, q, r] = t.map(|i| points[i]);
            // Its slivers' rounding aside.
            if ((q.0 - p.0) * (r.1 - p.1) - (r.0 - p.0) * (q.1 - p.1)).abs() < 1e-14 {
                continue;
            }
            for &(z, side) in &faces {
                let mut w = [p, q, r].map(|p| at(p.0, p.1, z));
                if dot(cross(sub(w[1], w[0]), sub(w[2], w[0])), up) * side < 0.0 {
                    w.swap(1, 2);
                }
                self.tri(false, w, [c; 3]);
            }
        }
        for &(first, count) in loops {
            for k in 0..count {
                let (p, q) = (points[first + k], points[first + (k + 1) % count]);
                let (du, dv) = (q.0 - p.0, q.1 - p.1);
                let l = du.hypot(dv).max(1e-12);
                // Outwards: to the right of the way round.
                let n = norm(sub(at(p.0 + dv / l, p.1 - du / l, h.0), at(p.0, p.1, h.0)));
                self.quad_n([at(p.0, p.1, h.0), at(q.0, q.1, h.0), at(q.0, q.1, h.1), at(p.0, p.1, h.1)], [n; 4], c);
            }
        }
    }

    /// Makes what was drawn solid since the `from`th vertex see-through,
    /// as solid as `alpha` (left out at 0).
    fn see_through(&mut self, from: usize, alpha: f32) {
        if alpha >= 1.0 {
            return;
        }
        let drawn = self.s.tris.split_off(from);
        if alpha > 0.0 {
            self.s.glass.extend(drawn.into_iter().map(|v| Vertex { color: [v.color[0], v.color[1], v.color[2], v.color[3] * alpha], ..v }));
        }
    }

    fn quad(&mut self, glass: bool, p: [V3; 4], c: [f32; 4]) {
        self.tri(glass, [p[0], p[1], p[2]], [c; 3]);
        self.tri(glass, [p[0], p[2], p[3]], [c; 3]);
    }

    /// A box from corner `o` along the edges `u`, `v`, `w` (world), its
    /// faces turned outwards.
    fn cuboid(&mut self, o: V3, u: V3, v: V3, w: V3, c: Color32) {
        if !self.draw {
            return;
        }
        let c = rgba(c, 1.0);
        let centre = add(o, scale(add(add(u, v), w), 0.5));
        for (a, b, d) in [(u, v, w), (v, w, u), (w, u, v)] {
            for far in [false, true] {
                let base = if far { add(o, d) } else { o };
                let mut q = [base, add(base, a), add(add(base, a), b), add(base, b)];
                let mid = add(base, scale(add(a, b), 0.5));
                if dot(cross(a, b), sub(mid, centre)) < 0.0 {
                    q.reverse();
                }
                self.quad(false, q, c);
            }
        }
    }

    /// A prism standing on the plate point `c`, from plate z `z.0` to
    /// `z.1`: `sides` flat faces, its corners `r` from the axis, the first
    /// at the plate angle `turn`; capped at both ends.
    fn plate_prism(&mut self, c: (f64, f64), r: f64, sides: usize, turn: f64, z: (f64, f64), col: Color32) {
        if !self.draw {
            return;
        }
        let p = self.place;
        let corner = |k: usize, z: f64| {
            let a = turn + std::f64::consts::TAU * k as f64 / sides as f64;
            p.at(c.0 + r * a.cos(), c.1 + r * a.sin(), z)
        };
        let col = rgba(col, 1.0);
        for k in 0..sides {
            self.quad(false, [corner(k, z.0), corner(k + 1, z.0), corner(k + 1, z.1), corner(k, z.1)], col);
        }
        for h in [z.0, z.1] {
            for k in 1..sides - 1 {
                self.tri(false, [corner(0, h), corner(k, h), corner(k + 1, h)], [col; 3]);
            }
        }
    }

    /// A turned part on the world axis from `base` along the unit `axis`,
    /// its square sections' flats facing `side`: each ring its height
    /// along the axis, its radius (a square's half-width) and whether it is
    /// square (its corners rounded) rather than round; each band between
    /// two rings its own colour, shaded smooth around a round one and flat
    /// on a square one or a step; the top capped.
    fn lathe(&mut self, base: V3, axis: V3, side: V3, rings: &[(f32, f32, bool)], colours: &[Color32]) {
        if !self.draw {
            return;
        }
        const SIDES: usize = 12;
        let u = norm(sub(side, scale(axis, dot(side, axis))));
        let v = cross(axis, u);
        // Around, at unit radius: a round section's points (its normals),
        // and a square's (a superellipse).
        let round: [V3; SIDES] = std::array::from_fn(|k| {
            let (s, c) = (std::f32::consts::TAU * (k as f32 + 0.5) / SIDES as f32).sin_cos();
            add(scale(u, c), scale(v, s))
        });
        let square: [V3; SIDES] = std::array::from_fn(|k| {
            let (s, c) = (std::f32::consts::TAU * (k as f32 + 0.5) / SIDES as f32).sin_cos();
            scale(round[k], 1.0 / (c.powi(6) + s.powi(6)).powf(1.0 / 6.0))
        });
        let points: Vec<[V3; SIDES]> = rings
            .iter()
            .map(|&(h, r, sq)| {
                let centre = add(base, scale(axis, h));
                std::array::from_fn(|k| add(centre, scale(if sq { square[k] } else { round[k] }, r)))
            })
            .collect();
        for i in 0..rings.len() - 1 {
            let c = rgba(colours[i.min(colours.len() - 1)], 1.0);
            let ((h0, r0, sq0), (h1, r1, sq1)) = (rings[i], rings[i + 1]);
            let step = (h1 - h0).abs() < 0.2 * (r1 - r0).abs();
            // The profile's slope tilts a round band's normals off the radius.
            let slope = |k: usize| norm(sub(scale(round[k % SIDES], h1 - h0), scale(axis, r1 - r0)));
            for k in 0..SIDES {
                let l = (k + 1) % SIDES;
                let p = [points[i][k], points[i][l], points[i + 1][l], points[i + 1][k]];
                if step || sq0 || sq1 {
                    let n = norm(cross(sub(p[1], p[0]), sub(p[3], p[0])));
                    let n = if dot(n, n) > 0.5 { n } else { norm(cross(sub(p[2], p[3]), sub(p[0], p[3]))) };
                    self.quad_n(p, [n; 4], c);
                } else {
                    self.quad_n(p, [slope(k), slope(l), slope(l), slope(k)], c);
                }
            }
        }
        let last = rings.len() - 1;
        let top = add(base, scale(axis, rings[last].0));
        let c = rgba(colours[last.min(colours.len()) - 1], 1.0);
        for k in 0..SIDES {
            self.tri(false, [top, points[last][k], points[last][(k + 1) % SIDES]], [c; 3]);
        }
    }

    /// A tube along the world path through the rings' centres, capped at
    /// both ends: each ring's section a rectangle with rounded corners,
    /// its half-widths along `side` and across, and its corner radius (as
    /// large as the smaller half-width throughout: an ellipse). Smooth
    /// around; along, smooth where the surface bends by less than
    /// [`SMOOTH_DEG`] (a ring repeated at the same centre makes a step).
    fn sweep(&mut self, rings: &[(V3, f32, f32, f32)], side: V3, c: Color32) {
        if !self.draw {
            return;
        }
        const ARC: usize = 4;
        let n = rings.len();
        if n < 2 {
            return;
        }
        let c = rgba(c, 1.0);
        let round = rings.iter().all(|&(_, u, v, r)| r >= u.min(v) - 1e-6);
        // The section: per point, its place and the tangent around (in the
        // ring's side and across axes) for half-widths (u, v), radius r.
        let count = if round { 4 * ARC } else { 4 * (ARC + 1) };
        let section = |u: f32, v: f32, r: f32, k: usize| -> ((f32, f32), (f32, f32)) {
            if round {
                let a = std::f32::consts::TAU * k as f32 / count as f32;
                ((u * a.cos(), v * a.sin()), (-u * a.sin(), v * a.cos()))
            } else {
                let r = r.min(0.98 * u.min(v));
                let (corner, j) = (k / (ARC + 1), k % (ARC + 1));
                let a = std::f32::consts::FRAC_PI_2 * (corner as f32 + j as f32 / ARC as f32);
                let (sx, sy) = ([1.0, -1.0, -1.0, 1.0][corner], [1.0, 1.0, -1.0, -1.0][corner]);
                ((sx * (u - r) + r * a.cos(), sy * (v - r) + r * a.sin()), (-a.sin(), a.cos()))
            }
        };
        // Each ring's frame: the path's tangent, `side` square to it, and
        // the third axis across.
        let frames: Vec<(V3, V3, V3)> = (0..n)
            .map(|i| {
                let mut t = sub(rings[(i + 1).min(n - 1)].0, rings[i.saturating_sub(1)].0);
                // At a step (a repeated centre), the next ring's own tangent.
                let mut j = i;
                while dot(t, t) < 1e-12 && j + 1 < n {
                    j += 1;
                    t = sub(rings[(j + 1).min(n - 1)].0, rings[j.saturating_sub(1)].0);
                }
                let t = norm(t);
                let u = norm(sub(side, scale(t, dot(side, t))));
                (t, u, cross(t, u))
            })
            .collect();
        let point = |i: usize, k: usize| {
            let (p, u, v, r) = rings[i];
            let ((x, y), _) = section(u, v, r, k);
            let (_, a, b) = frames[i];
            add(p, add(scale(a, x), scale(b, y)))
        };
        let around = |i: usize, k: usize| {
            let (_, u, v, r) = rings[i];
            let (_, (x, y)) = section(u, v, r, k);
            let (_, a, b) = frames[i];
            add(scale(a, x), scale(b, y))
        };
        // Along segment `s`, at its ring `i` and point `k`: the outward
        // normal, smoothed with the neighbouring segment's at a gentle bend.
        let along = |s: usize, k: usize| sub(point(s + 1, k), point(s, k));
        let normal = |s: usize, i: usize, k: usize| {
            let d = along(s, k);
            let other = if i == s { s.checked_sub(1) } else { (s + 1 < n - 1).then_some(s + 1) };
            let d = match other.map(|o| along(o, k)) {
                Some(e) if dot(norm(d), norm(e)) > SMOOTH_DEG.to_radians().cos() as f32 => add(norm(d), norm(e)),
                _ => d,
            };
            norm(cross(around(i, k), d))
        };
        for s in 0..n - 1 {
            for k in 0..count {
                let l = (k + 1) % count;
                let p = [point(s, k), point(s + 1, k), point(s + 1, l), point(s, l)];
                let nn = [normal(s, s, k), normal(s, s + 1, k), normal(s, s + 1, l), normal(s, s, l)];
                self.quad_n(p, nn, c);
            }
        }
        for (i, out) in [(0, -1.0), (n - 1, 1.0)] {
            let centre = rings[i].0;
            for k in 0..count {
                let mut t = [centre, point(i, k), point(i, (k + 1) % count)];
                if out < 0.0 {
                    t.swap(1, 2);
                }
                self.tri(false, t, [c; 3]);
            }
        }
    }

    /// A slab across the keyboard (world x) from `x0` to `x1`, its side
    /// the polygon `profile` in (y, z), capped by it; its edges shaded
    /// smooth where the profile bends by less than [`SMOOTH_DEG`].
    fn slab(&mut self, profile: &[(f32, f32)], x0: f32, x1: f32, c: Color32) {
        if !self.draw {
            return;
        }
        let mut p: Vec<(f64, f64)> = profile.iter().map(|&(y, z)| (y as f64, z as f64)).collect();
        if signed_area(&p) < 0.0 {
            p.reverse();
        }
        let n = p.len();
        let col = rgba(c, 1.0);
        for [i, j, k] in triangulate(&p) {
            for x in [x0, x1] {
                self.tri(false, [p[i], p[j], p[k]].map(|(y, z)| [x, y as f32, z as f32]), [col; 3]);
            }
        }
        // Counter-clockwise in (y, z): an edge's outward normal is its
        // direction turned clockwise.
        let out = |k: usize| {
            let (a, b) = (p[k], p[(k + 1) % n]);
            let l = (b.0 - a.0).hypot(b.1 - a.1).max(1e-12);
            ((b.1 - a.1) / l, -(b.0 - a.0) / l)
        };
        let at = |k: usize, e: usize| {
            let (o, m) = (out(e), out(if k == e { (e + n - 1) % n } else { (e + 1) % n }));
            let s = if o.0 * m.0 + o.1 * m.1 > SMOOTH_DEG.to_radians().cos() { (o.0 + m.0, o.1 + m.1) } else { o };
            norm([0.0, s.0 as f32, s.1 as f32])
        };
        for e in 0..n {
            let f = (e + 1) % n;
            let w = |k: usize, x: f32| [x, p[k].0 as f32, p[k].1 as f32];
            self.quad_n([w(e, x0), w(f, x0), w(f, x1), w(e, x1)], [at(e, e), at(f, e), at(f, e), at(e, e)], col);
        }
    }

    fn line(&mut self, a: V3, b: V3, c: [f32; 4]) {
        self.wire(a, b, c, 0.0);
    }

    /// A line that is a wire `d` metres thick.
    fn wire(&mut self, a: V3, b: V3, c: [f32; 4], d: f32) {
        if !self.draw {
            return;
        }
        for pos in [a, b] {
            let v = Vertex { width: d, ..self.vertex(pos, [0.0, 0.0, 1.0], c) };
            self.s.lines.push(v);
        }
    }

    fn label(&mut self, at: V3, text: impl Into<String>, page: &'static str) {
        if !self.layer.has(Layer::Still) {
            return;
        }
        self.s.labels.push(Label { at, text: text.into(), page });
    }

    /// The extent of everything drawn so far.
    fn bounds(&self) -> (V3, V3) {
        let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
        for v in self.s.tris.iter().chain(&self.s.glass).chain(&self.s.lid).chain(&self.s.lines) {
            for i in 0..3 {
                lo[i] = lo[i].min(v.pos[i]);
                hi[i] = hi[i].max(v.pos[i]);
            }
        }
        if lo[0] <= hi[0] { (lo, hi) } else { ([-1.0; 3], [1.0; 3]) }
    }

    fn finish(mut self) -> Scene {
        (self.s.min, self.s.max) = self.bounds();
        self.s.vertical = self.place.vertical;
        self.s
    }
}

/// The corner `c` of a profile from `a` on to `z`, rounded to radius `r`:
/// the arc's points from the one side to the other.
fn fillet(a: (f64, f64), c: (f64, f64), z: (f64, f64), r: f64) -> Vec<(f64, f64)> {
    let unit = |p: (f64, f64)| {
        let (x, y) = (p.0 - c.0, p.1 - c.1);
        let l = x.hypot(y).max(1e-12);
        (x / l, y / l)
    };
    let (u1, u2) = (unit(a), unit(z));
    let half = (u1.0 * u2.0 + u1.1 * u2.1).clamp(-1.0, 1.0).acos() / 2.0;
    let (t, d) = (r / half.tan(), r / half.sin());
    let bis = unit((c.0 + u1.0 + u2.0, c.1 + u1.1 + u2.1));
    let centre = (c.0 + bis.0 * d, c.1 + bis.1 * d);
    let angle = |p: (f64, f64)| (p.1 - centre.1).atan2(p.0 - centre.0);
    let (a0, mut a1) = (angle((c.0 + u1.0 * t, c.1 + u1.1 * t)), angle((c.0 + u2.0 * t, c.1 + u2.1 * t)));
    if a1 - a0 > std::f64::consts::PI {
        a1 -= std::f64::consts::TAU;
    } else if a0 - a1 > std::f64::consts::PI {
        a1 += std::f64::consts::TAU;
    }
    (0..=6).map(|k| a0 + (a1 - a0) * k as f64 / 6.0).map(|a| (centre.0 + r * a.cos(), centre.1 + r * a.sin())).collect()
}

/// The board's outline in plate metres, counter-clockwise: the model's
/// own (`soundboard.outline`), [`rounded`], or the full a × b rectangle.
pub fn board_outline(sb: &SoundboardParams) -> Vec<(f64, f64)> {
    let mut p = Outline::new(sb).map_or_else(|| vec![(0.0, 0.0), (sb.a, 0.0), (sb.a, sb.b), (0.0, sb.b)], |o| o.points);
    if signed_area(&p) < 0.0 {
        p.reverse();
    }
    rounded(&p)
}

/// The closed polygon `p` drawn as the curve it stands for (a grand's
/// bentside and tail are a few vertices in the model): through every
/// vertex, its corners (bends over [`CORNER_DEG`]) kept and its edges along
/// its bounding box straight; every other edge a cubic, tangent at each end
/// to the straight edge beside it, else to its two edges' bisector (along
/// the box's side at a vertex on it), split every [`ROUND_STEP_DEG`] or so
/// of turn. It stays within the box: a board's outline within its a × b.
pub fn rounded(p: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let n = p.len();
    if n < 3 {
        return p.to_vec();
    }
    let (lo, hi) = p.iter().fold(((f64::MAX, f64::MAX), (f64::MIN, f64::MIN)), |(l, h), q| ((l.0.min(q.0), l.1.min(q.1)), (h.0.max(q.0), h.1.max(q.1))));
    let edge = |k: usize| {
        let (a, b) = (p[k], p[(k + 1) % n]);
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let l = dx.hypot(dy);
        ((dx / l.max(1e-12), dy / l.max(1e-12)), l)
    };
    let prev = |i: usize| (i + n - 1) % n;
    let corner: Vec<bool> = (0..n)
        .map(|i| {
            let (u, v) = (edge(prev(i)).0, edge(i).0);
            u.0 * v.0 + u.1 * v.1 < CORNER_DEG.to_radians().cos()
        })
        .collect();
    let side = |a: f64, b: f64, lo: f64, hi: f64| (a - b).abs() < 1e-9 && ((a - lo).abs() < 1e-9 || (a - hi).abs() < 1e-9);
    let straight: Vec<bool> = (0..n)
        .map(|k| {
            let (a, b) = (p[k], p[(k + 1) % n]);
            (corner[k] && corner[(k + 1) % n]) || side(a.0, b.0, lo.0, hi.0) || side(a.1, b.1, lo.1, hi.1) || edge(k).1 < 1e-9
        })
        .collect();
    // The tangent at a vertex that is not a corner.
    let tangent = |i: usize| -> Option<(f64, f64)> {
        let (u, v) = (edge(prev(i)).0, edge(i).0);
        let on = |a: f64, lo: f64, hi: f64| (a - lo).abs() < 1e-9 || (a - hi).abs() < 1e-9;
        match (corner[i], straight[prev(i)], straight[i]) {
            (true, _, _) => None,
            (_, true, false) => Some(u),
            (_, false, true) => Some(v),
            _ => {
                let (mx, my) = (u.0 + v.0, u.1 + v.1);
                Some(match (on(p[i].0, lo.0, hi.0), on(p[i].1, lo.1, hi.1)) {
                    (true, false) => (0.0, my.signum()),
                    (false, true) => (mx.signum(), 0.0),
                    _ => {
                        let l = mx.hypot(my).max(1e-12);
                        (mx / l, my / l)
                    }
                })
            }
        }
    };
    let mut out = Vec::with_capacity(4 * n);
    for k in 0..n {
        out.push(p[k]);
        if straight[k] {
            continue;
        }
        let (j, (c, l)) = ((k + 1) % n, edge(k));
        let (t0, t1) = (tangent(k).unwrap_or(c), tangent(j).unwrap_or(c));
        let off = |t: (f64, f64)| (t.0 * c.0 + t.1 * c.1).clamp(-1.0, 1.0).acos();
        let pieces = ((off(t0) + off(t1)).to_degrees() / ROUND_STEP_DEG).ceil().clamp(1.0, 48.0) as usize;
        for s in 1..pieces {
            let t = s as f64 / pieces as f64;
            let (t2, t3) = (t * t, t * t * t);
            let (h0, h1, h2, h3) = (2.0 * t3 - 3.0 * t2 + 1.0, t3 - 2.0 * t2 + t, 3.0 * t2 - 2.0 * t3, t3 - t2);
            let at = |a: f64, ta: f64, b: f64, tb: f64| h0 * a + h1 * l * ta + h2 * b + h3 * l * tb;
            out.push((at(p[k].0, t0.0, p[j].0, t1.0).clamp(lo.0, hi.0), at(p[k].1, t0.1, p[j].1, t1.1).clamp(lo.1, hi.1)));
        }
    }
    out
}

/// The iron frame's outline in plate coordinates, as the engine has it: the
/// case's, carried forward over the pin block (`FrameParams::outline`).
pub fn frame_outline(model: &PianoModel, sb: &SoundboardParams) -> Vec<(f64, f64)> {
    let f = &model.frame;
    let plate = f.plate(sb);
    let mut p = Outline::new(&plate).map_or_else(|| vec![(0.0, 0.0), (f.a, 0.0), (f.a, f.b), (0.0, f.b)], |o| o.points);
    for q in &mut p {
        q.0 += f.offset_x;
    }
    if signed_area(&p) < 0.0 {
        p.reverse();
    }
    rounded(&p)
}

/// The board's front edge, along the keyboard: the index of its first
/// point in the counter-clockwise `outline` (the bass corner; the treble
/// one follows). The widest edge across within the board's front, square
/// to the spine or not (a concert grand's runs further forward at the
/// treble); none on an outline with no such edge.
pub fn front_edge(outline: &[(f64, f64)], sb: &SoundboardParams) -> Option<usize> {
    let n = outline.len();
    let across = |k: usize| outline[k].1 - outline[(k + 1) % n].1;
    (0..n)
        .filter(|&k| outline[k].0.max(outline[(k + 1) % n].0) <= FRONT_WITHIN * sb.a && across(k) >= FRONT_SPAN * sb.b)
        .max_by(|&i, &j| across(i).total_cmp(&across(j)))
}

pub fn signed_area(p: &[(f64, f64)]) -> f64 {
    0.5 * p.iter().zip(p.iter().cycle().skip(1)).map(|(a, b)| a.0 * b.1 - b.0 * a.1).sum::<f64>()
}

fn cross2(a: (f64, f64), b: (f64, f64), c: (f64, f64)) -> f64 {
    (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
}

/// The part of the polygon `subject` inside the convex counter-clockwise
/// `window` (Sutherland–Hodgman).
pub fn clip(subject: &[(f64, f64)], window: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let mut out = subject.to_vec();
    for k in 0..window.len() {
        let (a, b) = (window[k], window[(k + 1) % window.len()]);
        let input = std::mem::take(&mut out);
        let cut = |p: (f64, f64), q: (f64, f64)| {
            let (dp, dq) = (cross2(a, b, p), cross2(a, b, q));
            let t = dp / (dp - dq);
            (p.0 + t * (q.0 - p.0), p.1 + t * (q.1 - p.1))
        };
        for (i, &p) in input.iter().enumerate() {
            let q = input[(i + 1) % input.len()];
            match (cross2(a, b, p) >= 0.0, cross2(a, b, q) >= 0.0) {
                (true, true) => out.push(q),
                (true, false) => out.push(cut(p, q)),
                (false, true) => out.extend([cut(p, q), q]),
                (false, false) => {}
            }
        }
        if out.is_empty() {
            break;
        }
    }
    out
}

/// Triangles covering the simple counter-clockwise polygon `p` (ear
/// clipping), as indices into it.
pub fn triangulate(p: &[(f64, f64)]) -> Vec<[usize; 3]> {
    let mut idx: Vec<usize> = (0..p.len()).collect();
    let mut tris = Vec::new();
    while idx.len() > 3 {
        let n = idx.len();
        let ear = (0..n).find(|&i| {
            let (a, b, c) = (p[idx[(i + n - 1) % n]], p[idx[i]], p[idx[(i + 1) % n]]);
            cross2(a, b, c) > 0.0
                && idx.iter().all(|&j| {
                    let q = p[j];
                    q == a || q == b || q == c || !(cross2(a, b, q) >= 0.0 && cross2(b, c, q) >= 0.0 && cross2(c, a, q) >= 0.0)
                })
        });
        let Some(i) = ear else { break };
        tris.push([idx[(i + n - 1) % n], idx[i], idx[(i + 1) % n]]);
        idx.remove(i);
    }
    if idx.len() == 3 {
        tris.push([idx[0], idx[1], idx[2]]);
    }
    tris
}

/// Where each key's hammer strikes its strings across (m, plate y): evenly
/// from one semitone to the next, as on a grand (whose keys are not: the
/// action's shanks fan out to the strings), C8 and the main bridge's
/// lowest key in line with where their strings cross the keys' line
/// ([`SoundboardParams::key_across`]), the bass beyond at the same pitch,
/// each section of strings (`soundboard.string_sections`) its gap further
/// from the one below, where a strut of the frame passes,
/// but closer together if that would bring A0's agraffe within
/// [`AGRAFFE_EDGE`] of the board's bass side (the case's wall): its
/// string runs on from its strike, away from its bridge, to its agraffe.
/// Never wider than a real keyboard's ([`hammer_line`]).
/// The engine has no place across for the bass's hammers (it drives the
/// frame in line with their bridge points).
pub fn key_across(model: &PianoModel, sb: &SoundboardParams, m: u8) -> f64 {
    let lo_key = sb.main_bridge_from.min(107);
    let (c8, pitch) = hammer_line(sb);
    // The sections' gaps (`soundboard.string_sections`), where the frame's
    // struts pass: taken out of the main bridge's hammers' pitch, so that
    // its two end keys stay in line with their strings; the bass's whole
    // section stands its own gap further.
    let gaps = |from: u8| (from + 1..=108).map(|b| sb.section_gap(b)).sum::<f64>();
    let span = pitch * (108 - lo_key) as f64;
    let all = gaps(lo_key).min(0.5 * span);
    if m >= lo_key || pitch <= 0.0 {
        if all <= 0.0 {
            return c8 + pitch * (108 - m) as f64;
        }
        return c8 + (span - all) / (108 - lo_key) as f64 * (108 - m) as f64 + gaps(m).min(all);
    }
    let lowest = c8 + span + sb.section_gap(lo_key);
    let s = model.note(21).strike_pos;
    let a0 = (1.0 - s) * (case_sides(sb).1 - AGRAFFE_EDGE) + s * sb.bridge_point(21).1;
    let bass = ((a0 - lowest) / (lo_key - 21) as f64).clamp(0.5 * pitch, pitch);
    lowest + bass * (lo_key - m) as f64
}

/// The main bridge's hammers across (plate y): C8's place and the pitch
/// from one semitone to the next. In line with where the strings cross the
/// keys' line ([`SoundboardParams::key_across`]), unless that would make a
/// keyboard wider than a real one ([`WHITE_KEY`]; a board whose bridge
/// points are the strings' real, slanted ends, with no slant of the model's
/// on top): then, in the middle of the case, at a real keyboard's pitch on
/// an upright; on a grand at a concert grand's action's, a little wider
/// than its keys ([`HAMMER_FAN`]) with the gaps between its sections of
/// strings on top, the keys slanted to it; the strings fanning out from
/// their hammers to their bridge as a grand's do.
fn hammer_line(sb: &SoundboardParams) -> (f64, f64) {
    let y = |m: u8| sb.key_across(m);
    let lo_key = sb.main_bridge_from.min(107);
    let pitch = (y(lo_key) - y(108)) / (108 - lo_key) as f64;
    let real = WHITE_KEY * 7.0 / 12.0;
    // (Within 1 %: the default's keyboard is a real one's already.)
    if pitch <= 1.01 * real {
        return (y(108), pitch);
    }
    if sb.vertical {
        return (y(108).max((sb.b - 87.0 * real) / 2.0), real);
    }
    let keys = (108 - lo_key) as f64;
    let gaps: f64 = (lo_key + 1..=108).map(|m| sb.section_gap(m)).sum();
    let wide = (HAMMER_FAN * real + gaps / keys).min(pitch);
    // (A measured bridge has C8's strings where they are: its hammer in
    // line with them, the short top strings square to the keyboard.)
    if sb.bridge_measured(true) {
        return (y(108), wide);
    }
    // (C8's strings clear of the treble side's wall.)
    let (treble, bass) = case_sides(sb);
    (y(108).max((treble + bass - 87.0 * wide - sb.section_gap(lo_key)) / 2.0).max(treble + AGRAFFE_EDGE), wide)
}

/// The case's sides across (plate y): the board's outline's, treble and
/// bass; the rectangle's without one.
fn case_sides(sb: &SoundboardParams) -> (f64, f64) {
    let (lo, hi) = sb.outline.iter().filter(|p| p.0 != 0.0 || p.1 != 0.0).fold((f64::MAX, f64::MIN), |(lo, hi), p| (lo.min(p.1 * sb.b), hi.max(p.1 * sb.b)));
    if lo < hi { (lo, hi) } else { (0.0, sb.b) }
}

/// The keyboard's extent across (plate y): C8 and the main bridge's lowest
/// key in line with their strings, the bass keys beyond, but moved
/// towards the treble (or the bass) as far as leaves room for cheek
/// blocks [`KEY_CHEEK`] wide inside the case's walls (centred if there is
/// not that much room); the keys slant to their hammers behind it.
fn keyboard_span(model: &PianoModel, sb: &SoundboardParams) -> (f64, f64) {
    let mid = |m: u8| {
        let (x, w) = sketches::key_span(m);
        (x + w / 2.0) as f64
    };
    let lo_key = sb.main_bridge_from.min(107);
    let (c8, pitch) = hammer_line(sb);
    // Never wider than a real one: under an action that is (a grand's,
    // [`HAMMER_FAN`]), in the middle of its hammers from A0 to C8.
    let real = WHITE_KEY * 7.0 / 12.0;
    let white = if pitch > 1.01 * real { real } else { pitch } * (108 - lo_key) as f64 / (mid(108) - mid(lo_key));
    let hi = if pitch > 1.01 * real {
        (c8 + key_across(model, sb, 21)) / 2.0 + (mid(108) + mid(21)) / 2.0 * white
    } else {
        c8 + mid(108) * white
    };
    let lo = hi - 52.0 * white;
    let (lo, hi) = (lo.min(hi), lo.max(hi));
    let room = (KEY_CHEEK, sb.b - KEY_CHEEK);
    let shift = if hi - lo > room.1 - room.0 {
        (sb.b - lo - hi) / 2.0
    } else {
        (room.0 - lo).max(0.0) + (room.1 - hi).min(0.0)
    };
    (lo + shift, hi + shift)
}

/// The rear length (m) key `m`'s strings would have, room allowing: a
/// built board's own ([`piano_core::fem::BoardStructure::back_length`],
/// what holds its bridge), or [`HITCH_BASE`] and [`HITCH_SHARE`] of the
/// speaking length as drawn.
pub fn rear_wanted(sb: &SoundboardParams, m: u8, speaking: f64) -> f64 {
    if sb.structure.enabled { sb.structure.back_length_at(m) } else { HITCH_BASE + HITCH_SHARE * speaking }
}

/// A string's rear length (m), from its bridge point `p` (plate) on along
/// its direction `u` to its hitch pin: `wanted` ([`rear_wanted`]), but
/// [`HITCH_INSET`] short of the board's outline (the case's wall), and at
/// least [`HITCH_MIN`].
pub fn hitch_length(outline: &[(f64, f64)], p: (f64, f64), u: (f64, f64), wanted: f64) -> f64 {
    let n = outline.len();
    let mut best = wanted;
    for i in 0..n {
        let (a, b) = (outline[i], outline[(i + 1) % n]);
        let e = (b.0 - a.0, b.1 - a.1);
        let denom = u.0 * e.1 - u.1 * e.0;
        let len = e.0.hypot(e.1);
        if denom.abs() < 1e-12 || len < 1e-12 {
            continue;
        }
        let w = (a.0 - p.0, a.1 - p.1);
        let (t, s) = ((w.0 * e.1 - w.1 * e.0) / denom, (w.0 * u.1 - w.1 * u.0) / denom);
        if t > 0.0 && (0.0..=1.0).contains(&s) {
            // The inset measured square to the wall, along the string.
            let cos = (denom / len).abs().max(0.25);
            best = best.min(t - HITCH_INSET / cos);
        }
    }
    best.max(HITCH_MIN)
}

/// The agraffes' line (plate x), as the plan sketch draws it.
pub fn agraffe_x(model: &PianoModel) -> f64 {
    let f = &model.frame;
    let fx = if f.enabled { f.offset_x } else { -0.2 };
    fx + if f.enabled { f.agraffe_x } else { 0.25 }
}

/// The hammers' line (plate x): straight across the keyboard, as the
/// action's rail holds them. A grand's stands where its main strings'
/// lengths and their bridge points meet, as far as the board's front edge
/// leaves room (below); an upright's as follows. Each key's agraffe stands in front of it by
/// its strike point's share of the string, so that the agraffes run on a
/// diagonal, the bass's (the longest strings) frontmost, on the agraffe
/// line, the treble's just in front of the hammers. A grand's hammers
/// rise in front of its board, though: there the line stops short of the
/// board's edge, and the bass's agraffes come forward of the agraffe line.
pub fn hammer_x(model: &PianoModel, sb: &SoundboardParams) -> f64 {
    hammers(model, sb).x
}

/// The hammers' line: its place along (plate x) at `mid` across, and how
/// much further back it stands per metre towards the bass ([`hammers`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hammers {
    pub x: f64,
    pub slope: f64,
    pub mid: f64,
}

impl Hammers {
    /// The line's place along at `y` across.
    pub fn at(&self, y: f64) -> f64 {
        self.x + self.slope * (y - self.mid)
    }
}

/// The hammers' line of an instrument ([`hammer_x`] is its place at the
/// middle). An upright's and a grand's whose bridge is the drawing's own
/// are square to the strings' axis. A grand's whose bridge is a measured
/// one ([`SoundboardParams::bridge_keys`]) stands where its main strings'
/// own lengths, from their bridge points, put their hammers: each key's
/// where its speaking length behind the strike point reaches its bridge
/// point from its hammer's place across, and the straight line that fits
/// them best, which may slant a little against the board's axis
/// ([`HAMMER_SLANT`] at the most; a concert grand's does, by a degree or
/// two, the treble's hammers nearer the keyboard), so that the strings are
/// drawn as long as the engine has them; [`HAMMER_EDGE`] in front of the
/// board's front edge at the least. Kept from one call to the next for the
/// same instrument.
pub fn hammers(model: &PianoModel, sb: &SoundboardParams) -> Hammers {
    thread_local! {
        static LAST: std::cell::RefCell<Option<(PianoModel, SoundboardParams, Hammers)>> = const { std::cell::RefCell::new(None) };
    }
    if let Some(h) = LAST.with(|l| l.borrow().as_ref().filter(|(m, s, _)| m == model && s == sb).map(|l| l.2)) {
        return h;
    }
    let h = hammers_of(model, sb);
    LAST.with(|l| *l.borrow_mut() = Some((model.clone(), *sb, h)));
    h
}

fn hammers_of(model: &PianoModel, sb: &SoundboardParams) -> Hammers {
    let ag = agraffe_x(model);
    let x = (21..=108u8).map(|m| ag + model.note(m).strike_pos * (sb.bridge_point(m).0 - ag)).fold(f64::MIN, f64::max);
    if sb.vertical {
        return Hammers { x, slope: 0.0, mid: sb.b / 2.0 };
    }
    // Each main key's hammer's place across, and along where its string's
    // length asks for it.
    let places: Vec<(f64, f64)> = (sb.main_bridge_from.min(108)..=108)
        .map(|m| {
            let n = model.note(m);
            let (bx, by) = sb.bridge_point(m);
            let hy = key_across(model, sb, m);
            let (back, dy) = ((1.0 - n.strike_pos) * n.length, by - hy);
            (hy, bx - (back * back - dy * dy).max(0.0).sqrt())
        })
        .collect();
    let outline = board_outline(sb);
    let front = front_edge(&outline, sb).map(|k| (outline[k], outline[(k + 1) % outline.len()]));
    // (Without a measured bridge the board's layout is the drawing's own:
    // its hammers keep their room in front of the board, square to it.)
    if !sb.bridge_measured(true) {
        let mut along: Vec<f64> = places.iter().map(|p| p.1).collect();
        along.sort_by(f64::total_cmp);
        let edge = front.map_or(0.0, |(p, q)| p.0.min(q.0));
        return Hammers { x: along[along.len() / 2].min(edge - HAMMER_CLEAR), slope: 0.0, mid: sb.b / 2.0 };
    }
    // The line that fits them best, and no nearer the board's front edge
    // than [`HAMMER_EDGE`] at any key.
    let n = places.len() as f64;
    let (my, mx) = (places.iter().map(|p| p.0).sum::<f64>() / n, places.iter().map(|p| p.1).sum::<f64>() / n);
    let (sxy, syy) = places.iter().fold((0.0, 0.0), |(a, b), p| (a + (p.0 - my) * (p.1 - mx), b + (p.0 - my) * (p.0 - my)));
    let slope = if syy > 0.0 { (sxy / syy).clamp(-HAMMER_SLANT, HAMMER_SLANT) } else { 0.0 };
    let mut h = Hammers { x: mx, slope, mid: my };
    if let Some((p, q)) = front {
        let edge = |y: f64| p.0 + (q.0 - p.0) * ((y - p.1) / (q.1 - p.1)).clamp(0.0, 1.0);
        let over = places.iter().map(|&(y, _)| h.at(y) - (edge(y) - HAMMER_EDGE)).fold(0.0, f64::max);
        h.x -= over;
    }
    h
}

/// A key's string's ends in plate coordinates (its agraffe, its bridge
/// point) and its height at its agraffe ([`bridge_z`] on its bridge).
pub type StringEnds = ((f64, f64), (f64, f64), f64);

/// The top of the bridge under key `m` above the board, where the board is
/// a built one: the engine's thickness there, less the plate's.
fn bridge_top(sb: &SoundboardParams, m: u8) -> Option<f64> {
    let s = &sb.structure;
    s.enabled.then(|| s.bridge_thickness_at(m as f64, m >= sb.main_bridge_from) - s.plate_center)
}

/// Key `m`'s strings' height on their bridge: [`STRING_Z`] − [`BRIDGE_H`]
/// over its top. The agraffe's height where the model has no bridge of its
/// own; over a built board's, as high as the engine has the bridge there,
/// the strings sloping from their agraffe to it, and the bass strings at
/// least [`BASS_OVER`] over the main bridge's highest.
pub fn bridge_z(sb: &SoundboardParams, m: u8) -> f64 {
    let main = m >= sb.main_bridge_from;
    let flat = STRING_Z + if main { 0.0 } else { BASS_RISE };
    let Some(top) = bridge_top(sb, m) else { return flat };
    let over = STRING_Z - BRIDGE_H;
    if main {
        return top + over;
    }
    let highest = (sb.main_bridge_from..=108).filter_map(|k| bridge_top(sb, k)).fold(0.0, f64::max);
    (top + over).max(highest + over + BASS_OVER).max(flat)
}

/// Key `m`'s strings' height at `t` of their speaking length from their
/// agraffe.
pub fn string_z(sb: &SoundboardParams, ends: StringEnds, m: u8, t: f64) -> f64 {
    ends.2 + (bridge_z(sb, m) - ends.2) * t
}

/// A key's string in plate coordinates: from its agraffe to its bridge
/// point, at its height. Its hammer strikes it on the hammers' line `hx`
/// ([`hammer_x`]), in line with its key; the agraffe lies on the string's
/// line beyond, the more to the side the more the string slants (the
/// overstrung bass's, towards the bass end).
pub fn string_ends(model: &PianoModel, sb: &SoundboardParams, hx: f64, m: u8) -> ((f64, f64), (f64, f64), f64) {
    let z = STRING_Z + if m < sb.main_bridge_from { BASS_RISE } else { 0.0 };
    let (bx, by) = sb.bridge_point(m);
    let n = model.note(m);
    let s = n.strike_pos;
    let hy = key_across(model, sb, m);
    // (`hx` is the hammers' line at its middle: the key's own place on it.)
    let line = hammers(model, sb);
    let hx = hx + line.slope * (hy - line.mid);
    // A grand's: the engine's own length from the hammer to the agraffe
    // (its strike point's share of the scale's speaking length), so that
    // what lies in front of the hammers stands where it does on the
    // instrument, whatever the board's layout makes of the length behind.
    let (dx, dy) = (bx - hx, by - hy);
    let l = dx.hypot(dy);
    if !sb.vertical && l > 1e-9 {
        let front = s * n.length;
        return ((hx - dx / l * front, hy - dy / l * front), (bx, by), z);
    }
    let back = |hit: f64, bridge: f64| (hit - s * bridge) / (1.0 - s);
    ((back(hx, bx), back(hy, by)), (bx, by), z)
}

/// Where the engine's share `t` of key `m`'s speaking length (from the
/// agraffe) lies on its string as drawn, as a share of that: a grand's is
/// drawn with the engine's lengths from the agraffe on ([`string_ends`]),
/// whatever its length to the bridge.
pub fn drawn_share(model: &PianoModel, sb: &SoundboardParams, ends: StringEnds, m: u8, t: f64) -> f64 {
    if sb.vertical {
        return t;
    }
    let ((ax, ay), (bx, by), _) = ends;
    (t * model.note(m).length / (bx - ax).hypot(by - ay).max(1e-9)).min(1.0)
}

/// The keys that bear on one bar with key `m` in front of the capo bar
/// (first, last), if `m`'s strings end under it: `frame.duplex_group` unisons
/// to a bar from the lowest key of its section of strings.
pub fn duplex_group(model: &PianoModel, sb: &SoundboardParams, m: u8) -> Option<(u8, u8)> {
    model.frame.duplex_keys(sb, m)
}

/// How far in front of the capo bar key `m`'s strings bear on their bar
/// (`frame.duplex_far` to `frame.duplex_near` within a group), if they end
/// under it.
pub fn duplex_at(model: &PianoModel, sb: &SoundboardParams, m: u8) -> Option<f64> {
    model.frame.duplex_length(sb, m)
}

/// A string's tuning pin: where it stands on the plate, the agraffe hole
/// its string runs to from it, and the key's string's direction there.
#[derive(Clone, Copy, Debug)]
pub struct TuningPin {
    pub at: (f64, f64),
    pub hole: (f64, f64),
    pub dir: (f64, f64),
    /// How far over the strings' height at their agraffe the string comes
    /// to `hole`: 0 at an agraffe, [`DUPLEX_RISE`] where `hole` is its place
    /// on the bearing bar in front of a capo bar.
    pub rise: f64,
}

/// The tuning pins, one per string from the bass up (each key's strings
/// across in turn), as grands have them: in the pin block in front of the
/// agraffes, spread evenly across (each key's share of an even pitch,
/// whatever the keys' own spacing), a unison's on a diagonal one row
/// further from the agraffes per string (single strings alternating
/// between two rows, the one next to the first unison in front, clear of
/// that unison's nearer pin a row back), so that the farther pins'
/// strings pass between the nearer pins. `ends` are the keys' [`string_ends`] from A0.
pub fn tuning_pins(model: &PianoModel, sb: &SoundboardParams, ends: &[StringEnds]) -> Vec<TuningPin> {
    let ay = |m: u8| ends[(m - 21) as usize].0.1;
    let bass = |m: u8| m < sb.main_bridge_from;
    let unison = (21..=108u8).find(|&m| model.note(m).strings > 1).unwrap_or(109);
    let grand = !sb.vertical;
    // A grand's main strings' pins stand in level rows across each section
    // of strings: the first row's place (plate x), as near the keyboard as
    // the section's rearmost string end asks ([`PIN_GAP_GRAND`] in front of
    // an agraffe, [`PIN_GAP_CAPO`] in front of the capo bar).
    let first_gap = |k: u8| if duplex_at(model, sb, k).is_some() { PIN_GAP_CAPO } else { PIN_GAP_GRAND };
    let level = |m: u8| {
        if m < sb.main_bridge_from {
            return None;
        }
        let starts = |k: u8| k == sb.main_bridge_from || sb.section_gap(k) > 0.0 || (model.frame.capo_bar && k == model.frame.capo_from);
        let first = (21..=m).rev().find(|&k| starts(k)).unwrap_or(21);
        let last = (m + 1..=108).find(|&k| starts(k)).map_or(108, |k| k - 1);
        Some((first..=last).map(|k| ends[(k - 21) as usize].0 .0 - first_gap(k)).fold(f64::MIN, f64::max))
    };
    let mut pins = Vec::new();
    // The key's rank among its section's keys of as many strings.
    let mut rank = 0usize;
    for m in 21..=108u8 {
        if m == 21 || m == sb.main_bridge_from || sb.section_gap(m) > 0.0 || model.note(m).strings != model.note(m - 1).strings {
            rank = 0;
        } else {
            rank += 1;
        }
        let ((ax, _), (bx, by), _) = ends[(m - 21) as usize];
        // The keys' places across over an octave around it on its bridge,
        // fitted with a line: its even place and the pitch there.
        // (A grand's: the keys of its own section of strings, whose gap
        // from the next the pins keep.)
        let apart = |k: u8| grand && (k.min(m) + 1..=k.max(m)).any(|j| sb.section_gap(j) > 0.0);
        let keys: Vec<f64> = (m.saturating_sub(6).max(21)..=(m + 6).min(108)).filter(|&k| bass(k) == bass(m) && !apart(k)).map(f64::from).collect();
        let n = keys.len() as f64;
        let (mk, my) = (keys.iter().sum::<f64>() / n, keys.iter().map(|&k| ay(k as u8)).sum::<f64>() / n);
        let (sxy, sxx) = keys.iter().fold((0.0, 0.0), |(a, b), &k| (a + (k - mk) * (ay(k as u8) - my), b + (k - mk) * (k - mk)));
        let slope = if sxx > 0.0 { sxy / sxx } else { 0.0 };
        let (even, pitch) = (my + slope * (m as f64 - mk), slope.abs());
        // Along the key's strings and across them (the unison's 2.5 mm
        // apart at the agraffe, as drawn).
        let (dx, dy) = (bx - ax, by - ay(m));
        let len = dx.hypot(dy).max(1e-9);
        let (ux, uy) = (dx / len, dy / len);
        let (nx, ny) = (-uy, ux);
        let strings = model.note(m).strings;
        for s in 0..strings {
            let mid = s as f64 - (strings as f64 - 1.0) / 2.0;
            let row = if strings == 1 {
                (unison as i32 - 1 - m as i32).rem_euclid(2) as f64
            } else if grand {
                // A grand's: each unison's pins one behind the other, two
                // rows apart, and every other unison's a row further, so
                // that neighbours stand side by side in a quincunx.
                (2 * s + rank % 2) as f64
            } else {
                s as f64
            };
            let duplex = duplex_at(model, sb, m);
            let (gap, rows) = match (grand, duplex) {
                (_, Some(_)) => (PIN_GAP_CAPO, PIN_ROW_GRAND),
                (true, None) => (PIN_GAP_GRAND, PIN_ROW_GRAND),
                (false, None) => (PIN_GAP, PIN_ROW),
            };
            let g = unison_gap(model, m);
            let (back, across) = (gap + rows * row, mid * pitch / strings as f64);
            // To its agraffe's hole, or to its place on the bar it bears
            // on in front of the capo bar.
            let to = duplex.unwrap_or(0.0);
            let hole = (ax - ux * to + nx * mid * g, ay(m) - uy * to + ny * mid * g);
            let rise = if duplex.is_some() { DUPLEX_RISE } else { 0.0 };
            if !grand {
                pins.push(TuningPin { at: (ax - ux * back + nx * across, even - uy * back + ny * across), hole, dir: (ux, uy), rise });
                continue;
            }
            // A grand's: on its own string's line carried straight on
            // through the agraffe, the string coming to the pin's right
            // side (the treble's). The unison's leftmost string has the
            // nearest pin, so that the others pass on its right. Where the
            // unisons stand too close for the next one's strings to pass on
            // its left, the pins give way to the right by what lacks (the
            // strings then turn at the agraffe by that little).
            let row = if strings == 1 { row } else { (2 * (strings - 1 - s) + rank % 2) as f64 };
            // The main strings' rows are level across their section: each
            // unison takes the first rows of its own (every other one, a
            // quincunx) that lie far enough in front of its strings' end,
            // so that the pins fill the plate from the felt's slanted line
            // down, as on a grand.
            let back = match level(m) {
                Some(x) => {
                    let (parity, nth) = (row as usize % 2, row as usize / 2);
                    let from = ((x - (ax - gap)) / rows - 1e-9).ceil().max(0.0) as usize;
                    let r0 = from + (from + parity) % 2;
                    (ax - (x - rows * (r0 + 2 * nth) as f64)) / ux.max(0.5)
                }
                None => gap + rows * row,
            };
            let coil = PIN_R + PIN_WIRE + 0.0001;
            // (The unisons' spacing square to the strings: less than
            // across the keyboard where the agraffes' line slants. A string
            // that turns to a pin given way has turned by less where it
            // passes a nearer pin: by 0.4 to 0.8 of it, [`PIN_TURN`] the
            // least it is still short of it.)
            let square = [m.checked_sub(1).filter(|&k| k >= 21), Some(m + 1).filter(|&k| k <= 108)]
                .into_iter()
                .flatten()
                .filter(|&k| bass(k) == bass(m) && !apart(k))
                .map(|k| {
                    // (At the agraffes and where the far pins stand: the
                    // strings of two keys may close up on the way.)
                    let ((kx, ky), (qx, qy), _) = ends[(k - 21) as usize];
                    let l = (qx - kx).hypot(qy - ky).max(1e-9);
                    let far = gap + 5.0 * rows;
                    let at = |d: f64| ((kx - (qx - kx) / l * d - (ax - ux * d)) * nx + (ky - (qy - ky) / l * d - (ay(m) - uy * d)) * ny).abs();
                    at(0.0).min(at(far))
                })
                .fold(pitch, f64::min);
            let lacks = (strings as f64 - 1.0) * g + 2.0 * coil + PIN_PASS - square;
            let most = if strings > 1 { ((g - PIN_PASS) / 0.45).max(0.0) } else { 0.0 };
            // No room for that: spread across its unison's share of the
            // keys' pitch, the strings turning at the agraffe.
            if lacks / PIN_TURN > most {
let across = mid * (pitch * PIN_SPREAD_GRAND / strings as f64).max(if g > UNISON_GAP { g } else { 0.0 }) - coil * PIN_SHIFT_GRAND;
                pins.push(TuningPin { at: (ax - ux * back + nx * across, even - uy * back + ny * across), hole, dir: (ux, uy), rise });
                continue;
            }
            let aside = coil - (lacks / PIN_TURN).max(0.0);
            let line = (ax + nx * mid * g, ay(m) + ny * mid * g);
            pins.push(TuningPin { at: (line.0 - ux * back + nx * aside, line.1 - uy * back + ny * aside), hole, dir: (ux, uy), rise });
        }
    }
    // A grand's: whatever pin still stands in a string's way (strings that
    // fan, sections that meet) steps aside from it, a few times over.
    if grand {
        let coil = PIN_R + PIN_WIRE + 0.0001;
        let need = PIN_R + PIN_PASS;
        for _ in 0..12 {
            let mut moved = false;
            for i in 0..pins.len() {
                let p = pins[i];
                let a = (p.at.0 + p.dir.1 * coil, p.at.1 - p.dir.0 * coil);
                let (ex, ey) = (p.hole.0 - a.0, p.hole.1 - a.1);
                let l2 = ex * ex + ey * ey;
                if l2 < 1e-12 {
                    continue;
                }
                for j in (0..pins.len()).filter(|&j| j != i) {
                    let q = pins[j].at;
                    let t = (((q.0 - a.0) * ex + (q.1 - a.1) * ey) / l2).clamp(0.0, 1.0);
                    let (dx, dy) = (q.0 - (a.0 + ex * t), q.1 - (a.1 + ey * t));
                    let d = dx.hypot(dy);
                    if d < need && d > 1e-9 {
                        let k = (need - d + 0.0001) / d;
                        pins[j].at = (q.0 + dx * k, q.1 + dy * k);
                        moved = true;
                    }
                }
            }
            if !moved {
                break;
            }
        }
    }
    pins
}

/// The `i`th string's tuning pin `p`, for a string at height `z` drawn in
/// `c`, as they are made: a steel pin in a bushing, leaning a little away
/// from its string's pull, three turns of the string coiled round it a
/// little below the strings' height (the string comes to the coil's side,
/// the treble's, and leaves it at a tangent), a square head for the tuning hammer
/// under a rounded top, its flats turned any way; and the string's dead
/// length from the coil to its agraffe.
fn tuning_pin(b: &mut Builder, p: &TuningPin, i: usize, z: f64, c: Color32, wire: f32) {
    let place = b.place;
    let hole = place.at(p.hole.0, p.hole.1, z + p.rise);
    let (ux, uy) = p.dir;
    let lean = PIN_LEAN.tan();
    let base = place.at(p.at.0, p.at.1, PIN_BLOCK_TOP);
    let axis = place.dir(-ux * lean, -uy * lean, 1.0);
    // Turned by the golden angle from one pin to the next: no two alike.
    let turn = i as f64 * 2.399_963;
    let side = place.dir(turn.cos(), turn.sin(), 0.0);
    let (r, rb, w) = (PIN_R as f32, PIN_BUSH_R as f32, PIN_WIRE as f32);
    let (hb, ch) = (PIN_BUSH_H as f32, (PIN_COIL_H / 2.0) as f32);
    let coil = (z - PIN_COIL_DROP - PIN_BLOCK_TOP) as f32;
    let top = coil + PIN_TOP as f32;
    let head = top - PIN_HEAD as f32;
    #[rustfmt::skip]
    let rings = [
        (0.0, rb, false), (hb, rb, false), (hb, r, false),
        (head, r, false), (head + 0.0008, 0.8 * r, true), (top - 0.0012, 0.72 * r, true),
        (top - 0.0003, 0.55 * r, false), (top, 0.3 * r, false),
    ];
    b.lathe(base, axis, side, &rings, &[BUSHING, BUSHING, PIN, PIN, PIN, PIN, PIN]);
    // The coil's turns, up to where the string leaves it towards its
    // agraffe.
    let at = |h: f32| add(base, scale(axis, h));
    // The string comes to the pin's side, not to its axis: tangent to the
    // coil on its right as seen from the keyboard (the treble's side), and
    // winds on round the front of the pin, down from the eye, so that it
    // leaves from the lowest turn.
    let to = sub(hole, at(coil));
    let across = sub(to, scale(axis, dot(to, axis)));
    let e1 = norm(across);
    let e2 = cross(axis, e1);
    let e2 = if dot(e2, place.dir(0.0, -1.0, 0.0)) < 0.0 { scale(e2, -1.0) } else { e2 };
    // The coil itself: the wire wound as a spring, turn against turn, in
    // short lengths ([`PIN_COIL_STEPS`] a turn), about the coil's height.
    let _ = (w, ch);
    let rr = r + wire / 2.0;
    let touch = (rr / dot(across, across).sqrt().max(rr)).acos();
    let turns = PIN_COIL_TURNS as f32;
    let steps = PIN_COIL_TURNS * PIN_COIL_STEPS;
    let helix = |k: usize| {
        let t = k as f32 / steps as f32;
        let a = touch + std::f32::consts::TAU * turns * (1.0 - t);
        let h = coil + wire * turns * (0.5 - t);
        add(at(h), add(scale(e1, rr * a.cos()), scale(e2, rr * a.sin())))
    };
    for k in 0..steps {
        b.wire(helix(k), helix(k + 1), rgba(c, 1.0), wire);
    }
    b.wire(helix(steps), hole, rgba(c, 1.0), wire);
}

/// An agraffe's head's half-width for a unison of `strings`, `gap` apart
/// (wider with them, [`AGRAFFE_HEAD_PER`] a string at [`UNISON_GAP`]).
fn agraffe_head(strings: usize, gap: f64) -> f64 {
    AGRAFFE_HEAD + (AGRAFFE_HEAD_PER + (gap - UNISON_GAP) / 2.0) * (strings as f64 - 1.0)
}

/// The agraffe at `at` (plate) for a unison of `strings` along `u` at
/// height `z`, standing on `foot` (see the [`AGRAFFE_HEAD`] constants):
/// brass `c`, its boss the plate's `plate`; the strings' holes through
/// its head `gap` apart, as they are drawn.
#[allow(clippy::too_many_arguments)]
fn agraffe(b: &mut Builder, at: (f64, f64), u: (f64, f64), z: f64, foot: f64, strings: usize, gap: f64, c: Color32, plate: Color32) {
    let place = b.place;
    let hw = agraffe_head(strings, gap);
    let flange = hw + AGRAFFE_FLANGE;
    let head = z - AGRAFFE_BELOW;
    let foot = if head - AGRAFFE_NECK - AGRAFFE_FLANGE_H > foot {
        let top = head - AGRAFFE_NECK - AGRAFFE_FLANGE_H;
        b.plate_prism(at, flange + AGRAFFE_BOSS, 8, 0.0, (foot, top), plate);
        top
    } else {
        foot
    };
    b.plate_prism(at, flange, 6, u.1.atan2(u.0), (foot, foot + AGRAFFE_FLANGE_H), c);
    let base = place.at(at.0, at.1, foot + AGRAFFE_FLANGE_H);
    let (h, top, w) = ((head - foot - AGRAFFE_FLANGE_H) as f32, (z + AGRAFFE_ABOVE - foot - AGRAFFE_FLANGE_H) as f32, hw as f32);
    #[rustfmt::skip]
    let rings = [
        (0.0, 0.6 * w, false), (h, 0.6 * w, false), (h, w, true),
        (top - 0.0012, w, true), (top - 0.0003, 0.8 * w, false), (top, 0.45 * w, false),
    ];
    b.lathe(base, place.dir(0.0, 0.0, 1.0), place.dir(u.0, u.1, 0.0), &rings, &[c]);
    // The holes, on the faces the strings go in and out by.
    let (n, hole) = ((-u.1, u.0), rgba(Color32::from_rgb(40, 32, 20), 1.0));
    for s in 0..strings {
        let off = (s as f64 - (strings as f64 - 1.0) / 2.0) * gap;
        for side in [-1.0, 1.0] {
            let d = side * (hw + 0.0001);
            let (cx, cy) = (at.0 + u.0 * d + n.0 * off, at.1 + u.1 * d + n.1 * off);
            let rim = |k: usize| {
                let (sn, cs) = (std::f64::consts::TAU * k as f64 / 6.0).sin_cos();
                let r = AGRAFFE_HOLE_R;
                place.at(cx + n.0 * r * cs, cy + n.1 * r * cs, z + r * sn)
            };
            let centre = place.at(cx, cy, z);
            for k in 0..6 {
                let t = if side > 0.0 { [centre, rim(k), rim(k + 1)] } else { [centre, rim(k + 1), rim(k)] };
                b.tri(false, t, [hole; 3]);
            }
        }
    }
}

/// A grand's leg under `top` (world, the case's underside), as most are
/// made: a moulded block under the case, a square shaft tapering down with
/// rounded corners and a bead under its neck, a collar and a brass ferrule
/// at its foot, on a caster of two brass wheels side by side.
fn leg(b: &mut Builder, top: V3, floor: f32) {
    let [x, y, zt] = top;
    let at = |z: f32| [x, y, z];
    // The wheels roll along y, on an axle across; the fork's cheeks
    // outside them, its housing above.
    let (wr, ww) = (0.028, 0.012);
    let hub = floor + wr;
    for s in [-1.0, 1.0] {
        let (x0, x1) = (x + s * 0.003, x + s * (0.003 + ww));
        let (x0, x1) = (x0.min(x1), x0.max(x1));
        let wheel = |x: f32, r: f32| ([x, y, hub], r, r, r);
        b.sweep(&[wheel(x0, wr - 0.004), wheel(x0 + 0.002, wr), wheel(x1 - 0.002, wr), wheel(x1, wr - 0.004)], [0.0, 1.0, 0.0], BRASS);
        b.cuboid([x + s * 0.019 - 0.0015, y - 0.012, hub - 0.006], [0.003, 0.0, 0.0], [0.0, 0.024, 0.0], [0.0, 0.0, wr + 0.02], BRASS);
    }
    let axle = |x: f32| ([x, y, hub], 0.005, 0.005, 0.005);
    b.sweep(&[axle(x - 0.022), axle(x + 0.022)], [0.0, 1.0, 0.0], STEEL);
    let zc = hub + wr + 0.014;
    let ring = |z: f32, w: f32, r: f32| (at(z), w, w, r);
    b.sweep(&[ring(hub + wr - 0.006, 0.02, 0.019), ring(zc - 0.004, 0.026, 0.025), ring(zc, 0.028, 0.027)], [1.0, 0.0, 0.0], BRASS);
    // The ferrule, then the shaft up to the block under the case.
    let f = zc + 0.022;
    b.sweep(&[ring(zc, 0.03, 0.008), ring(zc + 0.003, 0.033, 0.009), ring(f - 0.003, 0.033, 0.009), ring(f, 0.03, 0.008)], [1.0, 0.0, 0.0], BRASS);
    let neck = zt - 0.12;
    let wood = [
        ring(f, 0.031, 0.008),
        ring(f, 0.036, 0.01),
        ring(f + 0.004, 0.04, 0.012),
        ring(f + 0.018, 0.04, 0.012),
        ring(f + 0.03, 0.034, 0.01),
        ring(f + 0.045, 0.034, 0.01),
        ring(neck, 0.05, 0.014),
        ring(neck + 0.004, 0.056, 0.016),
        ring(neck + 0.014, 0.056, 0.016),
        ring(neck + 0.018, 0.05, 0.014),
        ring(zt - 0.075, 0.05, 0.014),
        ring(zt - 0.055, 0.054, 0.014),
        ring(zt - 0.042, 0.062, 0.014),
        ring(zt - 0.036, 0.066, 0.012),
        ring(zt - 0.036, 0.074, 0.01),
        ring(zt, 0.074, 0.01),
    ];
    b.sweep(&wood, [1.0, 0.0, 0.0], CASE);
}

/// A pedal from the face it comes out of at `y` (world), `x` across,
/// `z` high, towards the pianist: narrow where it leaves the box, widening
/// into a foot pad that rises a little to its rounded tip; turned about its
/// pivot inside the box by `press` (0 at rest … 1 down).
fn pedal(b: &mut Builder, x: f32, y: f32, z: f32, press: f32, c: Color32) {
    let (py, pz) = (y + 0.06, z);
    let (s, co) = (0.1 * press.clamp(0.0, 1.0)).sin_cos();
    let ring = |d: f32, w: f32, t: f32| {
        // Out of the face by d, then turned down (towards −z) about the pivot.
        let (dy, dz) = (y - d - py, 0.006 * (d / 0.13).max(0.0).powi(2));
        ([x, py + dy * co - dz * s, pz + dy * s + dz * co], w, t, 0.004)
    };
    let rings = [
        ring(-0.01, 0.009, 0.006),
        ring(0.0, 0.009, 0.006),
        ring(0.035, 0.011, 0.0056),
        ring(0.07, 0.017, 0.0052),
        ring(0.09, 0.02, 0.005),
        ring(0.112, 0.021, 0.005),
        ring(0.124, 0.019, 0.0048),
        ring(0.131, 0.013, 0.0043),
    ];
    b.sweep(&rings, [1.0, 0.0, 0.0], c);
}

/// A grand's pedal lyre centred at `cx` across and `y` along, from the
/// floor up to the keybed's underside at `top`, as on most grands: a block
/// screwed under the keybed, two posts bowed outwards like a lyre's arms
/// with slats between them, the box the pedals come out of at its foot,
/// and two braces from the box's back up to the belly rail at `brace`
/// (y, z).
fn lyre(b: &mut Builder, cx: f32, y: f32, top: f32, floor: f32, brace: (f32, f32), pedals: [(f32, Color32); 3]) {
    let across = |x0: f32, x1: f32, z: f32, half: (f32, f32), r: f32, ease: f32| {
        let ring = |x: f32, k: f32| ([x, y, z], half.0 - k, half.1 - k, r);
        [ring(x0, ease), ring(x0 + ease, 0.0), ring(x1 - ease, 0.0), ring(x1, ease)]
    };
    // The box, on a plinth, under a moulded top.
    let (bz0, bz1) = (floor + 0.04, floor + 0.115);
    b.sweep(&across(cx - 0.2, cx + 0.2, floor + 0.035, (0.062, 0.008), 0.004, 0.004), [0.0, 1.0, 0.0], CASE);
    b.sweep(&across(cx - 0.19, cx + 0.19, (bz0 + bz1) / 2.0, (0.055, (bz1 - bz0) / 2.0), 0.012, 0.004), [0.0, 1.0, 0.0], CASE);
    b.sweep(&across(cx - 0.2, cx + 0.2, bz1 + 0.007, (0.062, 0.008), 0.005, 0.004), [0.0, 1.0, 0.0], CASE);
    // The block under the keybed, a moulding under it.
    b.sweep(&across(cx - 0.21, cx + 0.21, top - 0.018, (0.048, 0.018), 0.008, 0.004), [0.0, 1.0, 0.0], CASE);
    b.sweep(&across(cx - 0.17, cx + 0.17, top - 0.042, (0.038, 0.006), 0.004, 0.003), [0.0, 1.0, 0.0], CASE);
    // The posts, between the box's top and the moulding.
    let (p0, p1) = (bz1 + 0.015, top - 0.048);
    for s in [-1.0f32, 1.0] {
        let rings: Vec<_> = (0..=16)
            .map(|i| {
                let t = i as f32 / 16.0;
                let bow = 0.045 * (std::f32::consts::PI * t).sin() * (1.0 - 0.35 * t);
                ([cx + s * (0.1 + bow), y, p0 + (p1 - p0) * t], 0.02, 0.013, 0.006)
            })
            .collect();
        b.sweep(&rings, [0.0, 1.0, 0.0], CASE);
    }
    for (x, w) in [(-0.045, 0.006), (0.0, 0.009), (0.045, 0.006)] {
        let ring = |z: f32| ([cx + x, y, z], 0.01, w, 0.004);
        b.sweep(&[ring(p0), ring(p1)], [0.0, 1.0, 0.0], CASE);
    }
    // The braces: steel rods with brass ends.
    for s in [-1.0f32, 1.0] {
        let (a, z) = ([cx + s * 0.15, y + 0.05, bz1 - 0.01], [cx + s * 0.2, brace.0, brace.1]);
        let d = norm(sub(z, a));
        let side = norm(cross(d, [1.0, 0.0, 0.0]));
        let rod = |p: V3, r: f32| (p, r, r, r);
        b.sweep(&[rod(a, 0.006), rod(z, 0.006)], side, LYRE_ROD);
        for (p, q) in [(a, add(a, scale(d, 0.03))), (sub(z, scale(d, 0.03)), z)] {
            b.sweep(&[rod(p, 0.009), rod(q, 0.009)], side, BRASS);
        }
    }
    // The pedals through brass plates on the box's face.
    let face = y - 0.055;
    for (i, &(press, c)) in pedals.iter().enumerate() {
        let x = cx + (i as f32 - 1.0) * PEDAL_PITCH;
        b.cuboid([x - 0.017, face - 0.002, floor + 0.06], [0.034, 0.0, 0.0], [0.0, 0.002, 0.0], [0.0, 0.0, 0.026], BRASS);
        b.layer_of(Layer::Played);
        pedal(b, x, face, floor + 0.073, press, c);
        b.layer_of(Layer::Still);
    }
}

/// The fallboard over the keys from `x.0` to `x.1` across (world), the
/// keys' front and their visible part's back `y`, the keys' bottom `kz`:
/// `shut`, a board over the keys from the name board to their front, the
/// name on it; else the name on the name board above the keys, centred
/// at `cx`.
fn fallboard(b: &mut Builder, shut: bool, x: (f64, f64), cx: f64, y: (f64, f64), kz: f64, case: Color32) {
    let (key_front, y_back) = y;
    let kt = kz + 0.022;
    if !shut {
        name::name(b, &|u, v, h| [(cx + u) as f32, (y_back - h) as f32, (kt + NAME_UP + v) as f32]);
        return;
    }
    // Its top from high on the name board down to just in front of the
    // keys, its front edge rounded, a lip down in front of them onto the
    // key slip; under it, clear of the black keys.
    let (back, front) = ((y_back, kt + FALL_BACK), (key_front - FALL_LIP - 0.003, kt + FALL_FRONT));
    let mut profile = vec![back];
    profile.extend(fillet(back, front, (front.0, kz), FALL_ROUND));
    profile.extend([(front.0, kz), (front.0 + FALL_LIP, kz), (front.0 + FALL_LIP, kt + 0.004), (back.0, back.1 - FALL_T)]);
    let profile: Vec<(f32, f32)> = profile.into_iter().map(|(y, z)| (y as f32, z as f32)).collect();
    b.slab(&profile, x.0 as f32, x.1 as f32, case);
    // The name on its top, a little behind the middle.
    let (dy, dz) = (back.0 - front.0, back.1 - front.1);
    let l = dy.hypot(dz);
    let (dy, dz) = (dy / l, dz / l);
    let (py, pz) = (front.0 + dy * 0.55 * l, front.1 + dz * 0.55 * l);
    name::name(b, &|u, v, h| [(cx + u) as f32, (py + dy * v - dz * h) as f32, (pz + dz * v + dy * h) as f32]);
}

/// An upright's case round what is drawn so far, as most are made: two
/// sides from the floor up past the tuning pins, coming forward beside
/// the keys as arms; the top over them; behind the keys the fallboard,
/// open, and the upper panel in front of the action up to the top; the
/// keybed under the keys, the key slip in front of them, cheek blocks at
/// their ends; under the keybed the lower panel down to the plinth. The
/// panels and the top as solid as `o.front`. Across (plate y), the board
/// and rim's `across` or the keys' `keys` with their cheeks, the wider.
#[allow(clippy::too_many_arguments)]
/// One piece of the lid: the plate polygon `plan` (counter-clockwise),
/// between the heights `w` above the lid's underside, each point placed
/// by `at` (its place when shut, its height).
fn lid_leaf(b: &mut Builder, plan: &[(f64, f64)], w: (f64, f64), at: &dyn Fn((f64, f64), f64) -> V3, c: [f32; 4]) {
    if plan.len() < 3 {
        return;
    }
    for [i, j, k] in triangulate(plan) {
        let t = [plan[i], plan[j], plan[k]];
        b.tri(true, t.map(|p| at(p, w.1)), [c; 3]);
        b.tri(true, [t[2], t[1], t[0]].map(|p| at(p, w.0)), [c; 3]);
    }
    for i in 0..plan.len() {
        let (p, q) = (plan[i], plan[(i + 1) % plan.len()]);
        b.quad(true, [at(p, w.0), at(q, w.0), at(q, w.1), at(p, w.1)], c);
        b.line(at(p, w.1), at(q, w.1), rgba(CASE, 0.9));
    }
}

/// A grand's inner rim `inner` wide outside the board's `edge`, from `z0`
/// up to the board's underside, and the board's edge glued over it, out
/// to the outer rim.
#[allow(clippy::too_many_arguments)]
fn inner_rim(b: &mut Builder, edge: &[(f64, f64)], closed: bool, inner: f64, z0: f64, board_t: f64, case: Color32, wood: Color32) {
    b.wall(edge, closed, 0.0, inner, z0, &|_| -board_t, case);
    let from = b.s.tris.len();
    b.mat = Material::Spruce;
    b.wall(edge, closed, 0.0, inner, -board_t, &|_| 0.0, wood);
    b.mat = Material::Paint;
    // The board's reflection out to the outer rim's wall.
    let (lo, hi) = &mut b.s.board;
    for v in &b.s.tris[from..] {
        for i in 0..3 {
            lo[i] = lo[i].min(v.pos[i]);
            hi[i] = hi[i].max(v.pos[i]);
        }
    }
}

/// A built board's ribs under it, as the engine has them
/// ([`piano_core::fem::BoardStructure::ribs`]): each from end to end, as
/// wide as it is, its depth under the board's top the engine's own
/// ([`piano_core::fem::BoardStructure::rib_thickness_at`]): a parabola from
/// its middle to its ends, or its full height cut down to a tip over its
/// feathered ends; `thick` is the board's thickness at a point.
fn ribs(b: &mut Builder, s: &piano_core::fem::BoardStructure, sb: &SoundboardParams, thick: &dyn Fn(f64, f64) -> f64, c: Color32) {
    if !b.draw {
        return;
    }
    let place = b.place;
    let c = rgba(c, 1.0);
    b.mat = Material::Spruce;
    let mut label = None;
    for (r, width) in s.ribs_in_use() {
        let (p, q) = ((r.0 * sb.a, r.1 * sb.b), (r.2 * sb.a, r.3 * sb.b));
        let len = (q.0 - p.0).hypot(q.1 - p.1).max(1e-9);
        let (u, n) = (((q.0 - p.0) / len, (q.1 - p.1) / len), (-(q.1 - p.1) / len * width / 2.0, (q.0 - p.0) / len * width / 2.0));
        // Where its sections are cut, 0 to 1 along it: evenly, and closer
        // over a feathered end, where its height falls within centimetres.
        let feather = if s.rib_feather > 0.0 { s.rib_feather.min(0.25 * len) / len } else { 0.0 };
        let mut cuts: Vec<f64> = Vec::new();
        if feather > 0.0 {
            cuts.extend((0..RIB_FEATHER_SECTIONS).map(|k| feather * k as f64 / RIB_FEATHER_SECTIONS as f64));
        }
        cuts.extend((0..=RIB_SECTIONS).map(|k| feather + (1.0 - 2.0 * feather) * k as f64 / RIB_SECTIONS as f64));
        if feather > 0.0 {
            cuts.extend((1..=RIB_FEATHER_SECTIONS).map(|k| 1.0 - feather + feather * k as f64 / RIB_FEATHER_SECTIONS as f64));
        }
        let sections = cuts.len() - 1;
        // Each section: its two top corners against the board, its two
        // bottom ones.
        let section = |k: usize| {
            let t = cuts[k];
            let (x, y) = (p.0 + u.0 * len * t, p.1 + u.1 * len * t);
            let plate = thick(x, y);
            let top = -plate;
            let bottom = top.min(-s.rib_thickness_at(&r, len * t, len, plate));
            [place.at(x - n.0, y - n.1, top), place.at(x + n.0, y + n.1, top), place.at(x + n.0, y + n.1, bottom), place.at(x - n.0, y - n.1, bottom)]
        };
        for k in 0..sections {
            let (a, d) = (section(k), section(k + 1));
            b.quad(false, [a[0], a[3], d[3], d[0]], c);
            b.quad(false, [a[1], d[1], d[2], a[2]], c);
            b.quad(false, [a[3], a[2], d[2], d[3]], c);
        }
        let (a, d) = (section(0), section(sections));
        b.quad(false, [a[0], a[1], a[2], a[3]], c);
        b.quad(false, [d[0], d[3], d[2], d[1]], c);
        label.get_or_insert(section(sections / 2)[2]);
    }
    if let Some(at) = label {
        b.label(at, "Ribs", "Soundboard");
    }
}

fn upright_case(b: &mut Builder, o: &Options, across: (f64, f64), keys: (f64, f64), kz: f64, key_front: f64, y_back: f64, case: Color32) {
    let (_, hi) = b.bounds();
    let (top, back) = (hi[2] as f64 + 0.01, hi[1] as f64 + 0.005);
    let (klo, khi) = keys;
    let (x0, x1) = (-across.1.max(khi + U_CHEEK), -across.0.min(klo - U_CHEEK));
    let key_top = kz + 0.022;
    let (bed_top, bed_bot) = (kz - KEY_FRAME, kz - KEY_FRAME - KEYBED_T);
    let yf = key_front - KEY_SLIP - 0.002;
    let low = y_back + U_LOWER_BACK;
    // The sides: down from the top to the arms, which curve down to a
    // little above the keys and on to the key slip, rounded at their
    // front; under the keybed back to the lower panel.
    let arm = key_top + 0.035;
    let mut profile = vec![(low, 0.0), (back, 0.0), (back, top + U_TOP_T), (y_back, top + U_TOP_T), (y_back, arm + 0.12)];
    for i in 1..=10 {
        let s = i as f64 / 10.0;
        profile.push((y_back - 0.09 * s, arm + 0.06 * (1.0 + (std::f64::consts::PI * s).cos())));
    }
    profile.extend(fillet((y_back - 0.09, arm), (yf, arm), (yf, bed_bot), 0.022));
    profile.extend([(yf, bed_bot), (low, bed_bot)]);
    let profile: Vec<(f32, f32)> = profile.into_iter().map(|(y, z)| (y as f32, z as f32)).collect();
    for (a, z) in [(x0 - U_SIDE_T, x0), (x1, x1 + U_SIDE_T)] {
        b.slab(&profile, a as f32, z as f32, case);
    }
    let box_ = |b: &mut Builder, x: (f64, f64), y: (f64, f64), z: (f64, f64), c: Color32| {
        b.cuboid([x.0 as f32, y.0 as f32, z.0 as f32], [(x.1 - x.0) as f32, 0.0, 0.0], [0.0, (y.1 - y.0) as f32, 0.0], [0.0, 0.0, (z.1 - z.0) as f32], c);
    };
    // The keybed from the key slip back to in front of the plate's
    // struts, the key slip, the cheek blocks.
    let bed_back = -(plate::U_CLEAR + plate::U_DEPTH) - 0.005;
    box_(b, (x0, x1), (yf + 0.004, bed_back), (bed_bot, bed_top), case);
    box_(b, (x0, x1), (yf, yf + KEY_SLIP), (bed_bot, kz - 0.002), case);
    for x in [(x0, -(khi + 0.002)), (-(klo - 0.002), x1)] {
        box_(b, x, (yf + KEY_SLIP, y_back), (bed_top, key_top + 0.012), case);
    }
    // The felt strip behind the keys, the fallboard and the name.
    box_(b, (-(khi + 0.002), -(klo - 0.002)), (y_back - 0.006, y_back), (key_top - 0.004, key_top + 0.008), FELT);
    fallboard(b, o.fallboard, (x0, x1), -(klo + khi) / 2.0, (key_front, y_back), kz, case);
    // The panels, the top and the plinth, as solid as asked.
    let from = b.s.tris.len();
    box_(b, (x0, x1), (y_back, y_back + FRONT_T), (key_top + 0.008, top), case);
    box_(b, (x0 - U_SIDE_T, x1 + U_SIDE_T), (y_back - 0.012, back), (top, top + U_TOP_T), case);
    box_(b, (x0, x1), (low, low + FRONT_T), (U_PLINTH, bed_bot), case);
    box_(b, (x0, x1), (low - 0.01, low + FRONT_T), (0.0, U_PLINTH), case);
    b.see_through(from, o.front);
}

#[cfg(test)]
pub fn build(model: &PianoModel, live: Option<&Live>, o: &Options) -> Scene {
    build_layer(model, live, o, Layer::All)
}

/// The scene, or one of its layers ([`Layer`]): the still one does not
/// depend on `live`.
pub fn build_layer(model: &PianoModel, live: Option<&Live>, o: &Options, layer: Layer) -> Scene {
    let sb = model.soundboard.map(|s| s.resolved()).unwrap_or_default();
    let vertical = sb.vertical;
    let place = Place { vertical, top: UPRIGHT_BASE + sb.a };
    let mut b = Builder::new(place, layer);
    let lit = |p: Part| o.lit.contains(&p);
    let pick = |p: Part, c: Color32| if lit(p) { sketches::HIGHLIGHT } else { c };
    let (f, rim) = (&model.frame, &model.rim);
    let obs = live.map(|l| l.obs);
    let key_level = |m: u8| live.map_or(0.0, |l| l.keys[m as usize]);

    // The soundboard within its outline, on a grid so that its motion can
    // colour it: each grid cell clipped to the outline.
    let (gx, gy) = BOARD_GRID;
    let wood = pick(Part::Board, sketches::WOOD);
    let outline = board_outline(&sb);
    let field = live.and_then(|l| l.board);
    let colour = |x: f64, y: f64| -> [f32; 4] {
        let Some(f) = field else { return rgba(wood, 1.0) };
        let (u, v) = ((x / sb.a).clamp(0.0, 1.0) * (gx - 1) as f64, (y / sb.b).clamp(0.0, 1.0) * (gy - 1) as f64);
        let (i, j) = ((u as usize).min(gx - 2), (v as usize).min(gy - 2));
        let (fu, fv) = ((u - i as f64) as f32, (v - j as f64) as f32);
        let at = |i: usize, j: usize| f[j * gx + i];
        let level = (at(i, j) * (1.0 - fu) + at(i + 1, j) * fu) * (1.0 - fv) + (at(i, j + 1) * (1.0 - fu) + at(i + 1, j + 1) * fu) * fv;
        rgba(heat(level), 1.0)
    };
    b.mat = Material::Spruce;
    b.layer_of(Layer::Sounding);
    for j in 0..gy - 1 {
        for i in 0..gx - 1 {
            let (x0, x1) = (sb.a * i as f64 / (gx - 1) as f64, sb.a * (i + 1) as f64 / (gx - 1) as f64);
            let (y0, y1) = (sb.b * j as f64 / (gy - 1) as f64, sb.b * (j + 1) as f64 / (gy - 1) as f64);
            let piece = clip(&outline, &[(x0, y0), (x1, y0), (x1, y1), (x0, y1)]);
            for k in 1..piece.len().saturating_sub(1) {
                let t = [piece[0], piece[k], piece[k + 1]];
                // The slivers where a cell's edge passes by a rounded
                // outline's point aside.
                if cross2(t[0], t[1], t[2]).abs() < 1e-7 {
                    continue;
                }
                b.tri(false, t.map(|(x, y)| place.at(x, y, 0.0)), t.map(|(x, y)| colour(x, y)));
            }
        }
    }
    // Its underside (its edge is against the rim's wall): flat where the
    // model has no thickness; a built board's as thick as the engine has
    // it, thinner towards the rim, with its ribs under it.
    b.layer_of(Layer::Still);
    let under = rgba(wood.gamma_multiply(0.7), 1.0);
    let built = sb.structure.enabled.then_some(&sb.structure);
    let edge_t = built.map_or(BOARD_T, |s| s.plate_edge);
    let rim_line = Outline { points: outline.clone() };
    let thick = |x: f64, y: f64| built.map_or(BOARD_T, |s| s.plate_thickness_at(rim_line.edge_distance(x, y)));
    if !b.draw {
        // (Long to work out: not for another layer.)
    } else if built.is_some() {
        for j in 0..gy - 1 {
            for i in 0..gx - 1 {
                let (x0, x1) = (sb.a * i as f64 / (gx - 1) as f64, sb.a * (i + 1) as f64 / (gx - 1) as f64);
                let (y0, y1) = (sb.b * j as f64 / (gy - 1) as f64, sb.b * (j + 1) as f64 / (gy - 1) as f64);
                let piece = clip(&outline, &[(x0, y0), (x1, y0), (x1, y1), (x0, y1)]);
                for k in 1..piece.len().saturating_sub(1) {
                    let t = [piece[0], piece[k + 1], piece[k]];
                    if cross2(t[0], t[1], t[2]).abs() < 1e-7 {
                        continue;
                    }
                    b.tri(false, t.map(|(x, y)| place.at(x, y, -thick(x, y))), [under; 3]);
                }
            }
        }
    } else {
        for [i, j, k] in triangulate(&outline) {
            b.tri(false, [outline[i], outline[k], outline[j]].map(|(x, y)| place.at(x, y, -BOARD_T)), [under; 3]);
        }
    }
    if let Some(s) = built {
        ribs(&mut b, s, &sb, &thick, pick(Part::Board, RIB_WOOD));
    }
    b.mat = Material::Paint;
    b.s.board = outline.iter().flat_map(|&(x, y)| [place.at(x, y, 0.0), place.at(x, y, -edge_t)]).fold(([f32::MAX; 3], [f32::MIN; 3]), |(lo, hi), p| {
        (std::array::from_fn(|i| lo[i].min(p[i])), std::array::from_fn(|i| hi[i].max(p[i])))
    });
    b.label(place.at(sb.a * 0.45, sb.b * 0.7, 0.0), "Soundboard", "Soundboard");

    // The strings' ends: their agraffes on a diagonal in front of the
    // hammers ([`hammer_x`]), their tuning pins in front of those.
    let hx = hammer_x(model, &sb);
    let ends: Vec<_> = (21..=108u8).map(|m| string_ends(model, &sb, hx, m)).collect();
    let pins = tuning_pins(model, &sb, &ends);
    let case_front = pins.iter().map(|p| p.at.0).fold(f64::MAX, f64::min) - PIN_BUSH_R - 0.008;

    // The case: the rim's walls along the board's outline, carried forward
    // over the pin block to just in front of the tuning pins (as the frame
    // is), and the belly rail under the board's front edge.
    let t = if rim.enabled { rim.thickness } else { 0.02 };
    let (z0, z1) = if rim.enabled { (rim.top - rim.height, rim.top) } else { (-0.24, 0.16) };
    // An upright's stands behind its board, as its back's frame, clear of
    // the strings and the action in front.
    let (z0, z1) = if vertical { (U_RIM_FRONT - (z1 - z0), U_RIM_FRONT) } else { (z0, z1) };
    let case = pick(Part::Rim, CASE);
    // A grand's case goes on above the rim up to the lid's hinge: the box
    // the shut lid closes ([`SoundboardParams::lid_closure`]), the lid resting
    // on it. Only the rim below is in the structure's model.
    let lid_top = if vertical { z1 } else { sb.lid_hinge_height.max(z1) };
    // A grand's outer rim alone rises to the lid; the inner one inside it
    // stops under the board, whose glued edge covers it, and under the pin
    // block's ends in front of the board.
    let inner = if vertical { 0.0 } else { t - t.min(OUTER_RIM) };
    let n = outline.len();
    let front = front_edge(&outline, &sb);
    let (mut across_lo, mut across_hi) = (0.0, sb.b);
    if let Some(k) = front {
        let (p, q) = (outline[k], outline[(k + 1) % n]);
        // The sides, each carried straight forward from its front corner
        // (wherever the board's front edge puts it) to the case's front.
        let edge: Vec<_> = (1..=n).map(|i| outline[(k + i) % n]).collect();
        let mut sides = vec![(case_front, q.1)];
        sides.extend(edge.iter().copied().filter(|e| e.0 > case_front + 1e-6));
        sides.push((case_front, p.1));
        (across_lo, across_hi) = (p.1.min(q.1), p.1.max(q.1));
        if o.case {
            b.wall(&sides, false, inner, t, z0, &|_| z1, case);
            if lid_top > z1 {
                b.wall(&sides, false, inner, t, z1, &|_| lid_top, CASE);
            }
            if inner > 0.0 {
                inner_rim(&mut b, &edge, false, inner, z0, edge_t, case, wood);
                for arm in [[sides[0], edge[0]], [edge[n - 1], sides[sides.len() - 1]]] {
                    b.wall(&arm, false, 0.0, inner, z0, &|_| PIN_BLOCK_TOP, case);
                }
            }
            // A grand's under the board, behind the hammers resting under
            // its edge and the keys' tails.
            let at = if vertical { -BELLY_RAIL } else { BELLY_AT };
            b.wall(&[p, q], false, at, at + BELLY_RAIL, z0, &|_| -edge_t, case);
        }
    } else if o.case {
        b.wall(&outline, true, inner, t, z0, &|_| z1, case);
        if lid_top > z1 {
            b.wall(&outline, true, inner, t, z1, &|_| lid_top, CASE);
        }
        if inner > 0.0 {
            inner_rim(&mut b, &outline, true, inner, z0, edge_t, case, wood);
        }
    }

    // The pin block under the plate, from the case's front to just behind
    // the agraffes, short of the hammers.
    let line = hammers(model, &sb);
    let block_back = |ax: f64, ay: f64| (ax + 0.008).min(line.at(ay) - BLOCK_CLEAR);
    let mut back: Vec<(f64, f64)> = ends.iter().map(|&((ax, ay), _, _)| (block_back(ax, ay), ay.clamp(across_lo, across_hi))).collect();
    back.sort_by(|p, q| q.1.total_cmp(&p.1));
    let mut block = vec![(case_front, across_lo), (case_front, across_hi), (back[0].0, across_hi)];
    block.extend(&back);
    block.push((back[back.len() - 1].0, across_lo));
    // A key past the board's edge (A0, the keyboard being wider than the
    // board) lands on a corner once clamped: one point, not two.
    block.dedup_by(|p, q| (p.0 - q.0).hypot(p.1 - q.1) < 1e-9);
    if signed_area(&block) < 0.0 {
        block.reverse();
    }
    let plate = rgba(pick(Part::Frame, IRON), 1.0);
    let (bz0, bz1) = (PIN_BLOCK_TOP - PIN_BLOCK_T, PIN_BLOCK_TOP);
    for [i, j, k] in triangulate(&block) {
        for z in [bz0, bz1] {
            b.tri(false, [block[i], block[j], block[k]].map(|(x, y)| place.at(x, y, z)), [plate; 3]);
        }
    }
    for i in 0..block.len() {
        let (p, q) = (block[i], block[(i + 1) % block.len()]);
        b.quad(false, [place.at(p.0, p.1, bz0), place.at(q.0, q.1, bz0), place.at(q.0, q.1, bz1), place.at(p.0, p.1, bz1)], plate);
    }
    // The felt strips on it under the strings' dead lengths, along the
    // agraffes just in front of them: one per bridge's strings.
    // A grand's: a broad band, one per section of strings; and under a
    // capo bar, in its place, the bars the strings bear on in front of it,
    // one to each group of unisons, each with its felt in front
    // (`frame.duplex_far`, `frame.duplex_near`).
    let (felt_at, felt_w) = if vertical { (UNDER_FELT_AT, UNDER_FELT_W) } else { (UNDER_FELT_AT_GRAND, UNDER_FELT_W_GRAND) };
    let in_front = |m: u8, d: f64| {
        let ((ax, ay), (bx, by), _) = ends[(m - 21) as usize];
        let l = (bx - ax).hypot(by - ay).max(1e-9);
        (ax - (bx - ax) / l * d, ay - (by - ay) / l * d)
    };
    let mut m = 21u8;
    while m <= 108 {
        // The keys that share a strip with `m`: its group on a bearing
        // bar, or its section's agraffes (an upright's: its bridge's).
        let group = duplex_group(model, &sb, m);
        let last = group.map_or_else(
            || (m + 1..=108).find(|&k| k == sb.main_bridge_from || model.frame.capo(k) != model.frame.capo(m) || (!vertical && sb.section_gap(k) > 0.0)).map_or(108, |k| k - 1),
            |g| g.1,
        );
        // Carried half a key further at both ends.
        let run = |d: &dyn Fn(u8) -> f64| {
            let mut p: Vec<(f64, f64)> = (m..=last).map(|k| in_front(k, d(k))).collect();
            if p.len() > 1 {
                let n = p.len();
                let (a, z) = ((p[0].0 - p[1].0, p[0].1 - p[1].1), (p[n - 1].0 - p[n - 2].0, p[n - 1].1 - p[n - 2].1));
                p[0] = (p[0].0 + a.0 / 2.0, p[0].1 + a.1 / 2.0);
                p[n - 1] = (p[n - 1].0 + z.0 / 2.0, p[n - 1].1 + z.1 / 2.0);
            }
            p.sort_by(|p, q| q.1.total_cmp(&p.1));
            p
        };
        if group.is_some() {
            let z = ends[(m - 21) as usize].2;
            let bar = run(&|k| duplex_at(model, &sb, k).unwrap_or(0.0));
            b.mat = Material::Iron;
            b.wall(&bar, false, -DUPLEX_BAR_W / 2.0, DUPLEX_BAR_W / 2.0, PIN_BLOCK_TOP, &|_| z + DUPLEX_RISE - 0.0005, pick(Part::Agraffes, IRON));
            b.mat = Material::Paint;
            let felt = run(&|k| duplex_at(model, &sb, k).unwrap_or(0.0) + DUPLEX_FELT);
            b.wall(&felt, false, -DUPLEX_FELT_W / 2.0, DUPLEX_FELT_W / 2.0, PIN_BLOCK_TOP, &|_| PIN_BLOCK_TOP + UNDER_FELT_T, UNDER_FELT);
        } else {
            let strip = run(&|_| felt_at);
            b.wall(&strip, false, -felt_w / 2.0, felt_w / 2.0, PIN_BLOCK_TOP, &|_| PIN_BLOCK_TOP + UNDER_FELT_T, UNDER_FELT);
        }
        m = last + 1;
    }
    let rim_label = outline.iter().cloned().fold((0.0, 0.0), |m, p| if p.0 > m.0 { p } else { m });
    if o.case {
        b.label(place.at(rim_label.0 + t, rim_label.1, z1), if rim.enabled { "Rim" } else { "Case (not modelled)" }, "Frame & rim");
    }

    // Bridges on the board.
    let main = pick(Part::Bridges, sketches::BRIDGE);
    let bass = if lit(Part::BassBridge) { sketches::HIGHLIGHT } else { main };
    // As high and as wide as a built board's are along them (the keys
    // under [`SoundboardParams::bridge_line`]'s points); the bass bridge's
    // body then as thick as the engine's, a narrower cap on it carrying
    // its strings over the others.
    let tb = sb.bridge_line(true);
    let bb = sb.bridge_line(false);
    let from = sb.main_bridge_from as f64;
    let main_key = |i: usize| 108.0 - (108.0 - from) * i as f64 / (tb.len() - 1).max(1) as f64;
    let bass_key = |i: usize| from - 1.0 - (from - 22.0) * i as f64 / (bb.len() - 1).max(1) as f64;
    let over = STRING_Z - BRIDGE_H;
    let bass_z = bridge_z(&sb, sb.main_bridge_from.saturating_sub(1).max(21)) - over;
    let (main_top, bass_top) = (|i: usize| built.map_or(BRIDGE_H, |s| s.bridge_thickness_at(main_key(i), true) - s.plate_center), |i: usize| {
        built.map_or(BRIDGE_H + BASS_RISE, |s| (s.bridge_thickness_at(bass_key(i), false) - s.plate_center).min(bass_z))
    });
    let (main_w, bass_w) = built.map_or((0.0125, 0.015), |s| (s.bridge_width / 2.0, s.bridge_width / 2.0));
    b.wall(&tb, false, -main_w, main_w, 0.0, &main_top, main);
    b.wall(&bb, false, -bass_w, bass_w, 0.0, &bass_top, bass);
    // The bend that makes one bridge of the two
    // ([`piano_core::fem::BoardStructure::bridge_ring`]), from the main
    // bridge's height at its lowest key to the bass bridge's body.
    let ring = if built.is_some() { piano_core::fem::ring_line(&sb) } else { Vec::new() };
    if ring.len() > 1 {
        let (h0, h1) = (main_top(tb.len() - 1), bass_top(bb.len() - 1));
        b.wall(&ring, false, -main_w, main_w, 0.0, &|i| h0 + (h1 - h0) * i as f64 / (ring.len() - 1) as f64, main);
    }
    if built.is_some() {
        b.wall(&bb, false, -BASS_CAP / 2.0, BASS_CAP / 2.0, 0.0, &|_| bass_z, bass);
    }
    let mid = tb.len() / 2;
    b.label(place.at(tb[mid].0, tb[mid].1, main_top(mid)), "Bridge", "Soundboard");
    b.label(place.at(bb[0].0, bb[0].1, bass_z), "Bass bridge", "Soundboard");

    // Strings, from their tuning pins across their agraffes, hammers and
    // dampers.
    let d = &model.damper;
    // Strings counted from the bass, as their tuning pins are.
    let mut count = 0usize;
    // Where the felt braid crosses each unison's rear length.
    struct Crossing {
        bass: bool,
        /// Plate place, the strings' height there and their direction.
        at: (f64, f64),
        z: f64,
        dir: (f64, f64),
    }
    let mut braid: Vec<Crossing> = Vec::new();
    // Under each key's hitch pins: which bridge, where, how high.
    let mut risers: Vec<(bool, (f64, f64), f64)> = Vec::new();
    for m in 21..=108u8 {
        let n = model.note(m);
        let ((ax, ay), (bx, by), z) = ends[(m - 21) as usize];
        // On its bridge, as high as that stands.
        let zb = bridge_z(&sb, m);
        let level = key_level(m);
        let wound = m < sb.main_bridge_from || n.strings == 1;
        let base = if wound { COPPER } else { STEEL };
        b.mat = if wound { Material::Copper } else { Material::Steel };
        let (wire, plain) = string_d(model, m);
        let base = if lit(Part::Length) || lit(Part::Unison) { sketches::HIGHLIGHT } else { base };
        let c = if m == o.key { o.ink } else { mix(base, sketches::HIGHLIGHT, level * 0.8) };
        // What is not its speaking length does not move.
        let still = if m == o.key { o.ink } else { base };
        // Across the unison, 2.5 mm apart (thick strings wider).
        let gap = unison_gap(model, m);
        let (dx, dy) = (bx - ax, by - ay);
        let len = dx.hypot(dy).max(1e-9);
        let (nx, ny) = (-dy / len, dx / len);
        let segments = if level > 0.02 { 16 } else { 1 };
        // Beyond the bridge, the strings' rear lengths to their hitch pins,
        // a little lower on the plate.
        let (ux, uy) = (dx / len, dy / len);
        let rear = hitch_length(&outline, (bx, by), (ux, uy), rear_wanted(&sb, m, len));
        let tail = if m == o.key { o.ink } else { pick(Part::Frame, HITCH) };
        // Over the bridge, between its two pins: the speaking length ends
        // against the front pin, to one side, and the string crosses to
        // the other side of the back pin (side bearing).
        let (pa, pd) = (BRIDGE_PIN_ALONG, BRIDGE_PIN_SIDE);
        for s in 0..n.strings {
            let off = (s as f64 - (n.strings as f64 - 1.0) / 2.0) * gap;
            let point = |t: f64| {
                let amp = live.map_or(0.0, |l| {
                    let phase = l.time * std::f32::consts::TAU * (5.0 + 0.07 * m as f32) + s as f32;
                    0.012 * level * phase.sin() * (std::f32::consts::PI * t as f32).sin()
                });
                let z = z + (zb - z) * t + amp as f64;
                let (cx, cy) = (-ux * pa + nx * pd, -uy * pa + ny * pd);
                place.at(ax + (dx + cx) * t + nx * off, ay + (dy + cy) * t + ny * off, z)
            };
            // On over the bridge to its back pin.
            let (gx, gy) = (bx + ux * pa + nx * (off - pd), by + uy * pa + ny * (off - pd));
            b.layer_of(Layer::Sounding);
            for k in 0..segments {
                let (t0, t1) = (k as f64 / segments as f64, (k + 1) as f64 / segments as f64);
                b.wire(point(t0), point(t1), rgba(c, 1.0), wire);
            }
            b.wire(point(1.0), place.at(gx, gy, zb), rgba(c, 1.0), wire);
            b.layer_of(Layer::Still);
            // Its tuning pin, and the string's dead length from the pin's
            // coil to the agraffe.
            // (A wound string's winding starts past its agraffe: bare
            // steel from the pin to it.)
            let dead = if still == COPPER { STEEL } else { still };
            let mat = b.mat;
            b.mat = Material::Steel;
            tuning_pin(&mut b, &pins[count], count, z, dead, plain);
            // From the bar it bears on down under the capo bar.
            if pins[count].rise > 0.0 {
                let hole = pins[count].hole;
                b.wire(place.at(hole.0, hole.1, z + pins[count].rise), point(0.0), rgba(dead, 1.0), plain);
            }
            b.mat = mat;
            // The bridge pins, and the string between them.
            let foot = zb - (STRING_Z - BRIDGE_H);
            let front = (bx - ux * pa + nx * (off + pd + BRIDGE_PIN_R), by - uy * pa + ny * (off + pd + BRIDGE_PIN_R));
            b.plate_prism(front, BRIDGE_PIN_R, 6, 0.0, (foot, zb + 0.004), BRIDGE_PIN);
            b.plate_prism((gx - nx * BRIDGE_PIN_R, gy - ny * BRIDGE_PIN_R), BRIDGE_PIN_R, 6, 0.0, (foot, zb + 0.004), BRIDGE_PIN);
            // Its rear length, plain wire, round its hitch pin.
            let (hx, hy) = (bx + ux * rear + nx * off, by + uy * rear + ny * off);
            b.wire(place.at(gx, gy, zb), place.at(hx - ux * HITCH_R, hy - uy * HITCH_R, zb - HITCH_DROP), rgba(tail, 1.0), plain);
            // Standing on the plate's rail, or on its riser where the
            // bridge holds the string high above it.
            let stand = if built.is_some() { (zb - HITCH_DROP - HITCH_RISER_UNDER).max(plate::RAIL_TOP) } else { plate::RAIL_TOP };
            b.plate_prism((hx, hy), HITCH_R, 6, 0.0, (stand - 0.001, zb - HITCH_DROP + 0.004), HITCH);
            if s == 0 {
                let mid = (n.strings as f64 - 1.0) / 2.0 * gap;
                risers.push((m < sb.main_bridge_from, (hx + nx * mid, hy + ny * mid), stand));
            }
            count += 1;
        }
        b.mat = Material::Paint;
        // The felt woven through the rear lengths of the strings that have
        // dampers (the treble's ring free, a duplex).
        if m <= d.highest_note {
            braid.push(Crossing { bass: m < sb.main_bridge_from, at: (bx + ux * rear * 0.45, by + uy * rear * 0.45), z: zb - HITCH_DROP * 0.45, dir: (ux, uy) });
        }
        // The agraffe the unison goes through, standing on the pin block
        // (the highest treble's overhang it).
        // (None under a capo bar: the plate's bar holds the strings.)
        let r = agraffe_head(n.strings, gap) + AGRAFFE_FLANGE;
        let foot = if ax + r <= block_back(ax, ay) { PIN_BLOCK_TOP } else { z - 0.008 };
        if vertical || !f.capo(m) {
            agraffe(&mut b, (ax, ay), (ux, uy), z, foot, n.strings, gap, pick(Part::Agraffes, BRASS), pick(Part::Frame, IRON));
        }
    }
    // The plate's risers under the hitch pins of a bridge that holds its
    // strings high (a built board's): along the pins, up to under them.
    if f.enabled && o.frame {
        b.mat = Material::Iron;
        for bass in [true, false] {
            let set: Vec<_> = risers.iter().filter(|r| r.0 == bass).collect();
            if set.iter().any(|r| r.2 > plate::RAIL_TOP + 0.002) {
                let line: Vec<_> = set.iter().map(|r| r.1).collect();
                b.wall(&line, false, -HITCH_RISER, HITCH_RISER, plate::RAIL_TOP - 0.001, &|i| set[i].2, pick(Part::Frame, IRON));
            }
        }
        b.mat = Material::Paint;
    }
    // The braid: a band of felt across each bridge's rear lengths, over
    // one unison and under the next.
    for bass in [true, false] {
        let set: Vec<_> = braid.iter().filter(|e| e.bass == bass).collect();
        if set.len() < 2 {
            continue;
        }
        let steps = 4;
        let mut rings = Vec::new();
        for i in 0..set.len() - 1 {
            let (p, q) = (set[i], set[i + 1]);
            for k in 0..steps {
                let t = k as f64 / steps as f64;
                let weave = BRAID_WEAVE * (std::f64::consts::PI * (i as f64 + t)).cos();
                let (x, y) = (p.at.0 + (q.at.0 - p.at.0) * t, p.at.1 + (q.at.1 - p.at.1) * t);
                rings.push((place.at(x, y, p.z + (q.z - p.z) * t + weave), BRAID_W / 2.0, BRAID_T / 2.0, BRAID_T / 2.0 - 0.0001));
            }
        }
        let last = set[set.len() - 1];
        let weave = BRAID_WEAVE * (std::f64::consts::PI * (set.len() - 1) as f64).cos();
        rings.push((place.at(last.at.0, last.at.1, last.z + weave), BRAID_W / 2.0, BRAID_T / 2.0, BRAID_T / 2.0 - 0.0001));
        let u = set.iter().fold((0.0, 0.0), |a, e| (a.0 + e.dir.0, a.1 + e.dir.1));
        b.sweep(&rings, place.dir(u.0, u.1, 0.0), FELT);
    }
    let mid = string_ends(model, &sb, hx, 60);
    let n60 = model.note(60);
    let s60 = drawn_share(model, &sb, mid, 60, n60.strike_pos);
    let strike = (mid.0.0 + (mid.1.0 - mid.0.0) * s60, mid.0.1 + (mid.1.1 - mid.0.1) * s60);
    b.label(place.at(strike.0, strike.1 - 0.12, if vertical { mid.2 + 0.04 } else { mid.2 - 0.04 }), "Hammers", "Hammer");
    let damp = action::damper_at(model, mid, 60);
    b.label(place.at(damp.0, damp.1 + 0.12, mid.2 + 0.03), "Dampers", "Dampers");
    b.label(place.at((mid.0.0 + mid.1.0) / 2.0, mid.0.1 - 0.25, mid.2), "Strings", "Strings");
    let a0 = string_ends(model, &sb, hx, 21);
    b.label(place.at(a0.0.0, a0.0.1, a0.2 + 0.01), "Agraffes", "Frame & rim");
    let c8 = string_ends(model, &sb, hx, 108);
    b.label(place.at(c8.0.0 - PIN_GAP, c8.0.1, c8.2 + 0.015), "Tuning pins", "Frame & rim");

    // The iron frame over the strings, as a plate is cast ([`plate`]).
    if f.enabled && o.frame {
        let lit_frame = lit(Part::Frame) || lit(Part::FrameHeight);
        b.mat = Material::Iron;
        // (Long to work out: not when it is not drawn.)
        if b.draw {
            plate::plate(&mut b, model, &sb, &ends, case_front, if lit_frame { sketches::HIGHLIGHT } else { IRON });
        }
        b.mat = Material::Paint;
        // Labelled at its tail, the farthest point from the keyboard.
        let outline = frame_outline(model, &sb);
        let tail = outline.iter().copied().max_by(|p, q| p.0.total_cmp(&q.0)).unwrap_or((0.0, 0.0));
        b.label(place.at(tail.0, tail.1, f.height), "Frame", "Frame & rim");
    }

    // The lid, hinged along the bass side, of the case's shape as the
    // radiation has it ([`SoundboardParams::lid_outline`]): over the rim,
    // its bentside the board's, rounded as the board is, and carried
    // forward over the case's front. Two leaves [`LID_T`] thick: the main
    // one from the hammers' line back, and the front flap, which folds
    // back on it as the lid opens (flat when shut, right over from
    // [`LID_FLAP_DEG`]). Drawn hinged on the outer rim, the wall it stands
    // on: the board's glued edge on the inner rim lies between that and
    // y = b, the radiation's hinge, and a lid hinged there would leave it
    // open to the sky.
    let lid = Outline { points: outline.clone() }.grown(LID_OVERHANG).points;
    let lid_from = b.s.glass.len();
    if !vertical && !sb.lid_removed && o.lid {
        let open = sb.lid_angle > 0.0;
        let (th, h) = (sb.lid_angle.max(0.0).to_radians(), sb.lid_hinge_height);
        let hinge = sb.b + inner.min(LID_OVERHANG);
        let mut plan = lid.clone();
        if let Some(k) = front {
            // Over the case's front, which its moulding closes.
            for i in [k, (k + 1) % n] {
                plan[i].0 = case_front - FRONT_T;
            }
        }
        if signed_area(&plan) < 0.0 {
            plan.reverse();
        }
        let far = 10.0 * (sb.a + sb.b);
        // The flap's hinge: over the hammers' line, the flap alone over
        // the pin block in front of it. No flap on a case with no front.
        let fold_x = if front.is_some() { hx } else { -far };
        let leaf = clip(&plan, &[(fold_x, -far), (far, -far), (far, hinge), (fold_x, hinge)]);
        let flap = clip(&plan, &[(-far, -far), (fold_x, -far), (fold_x, hinge), (-far, hinge)]);
        // A point of the lid from its place when shut and its height
        // in the lid's thickness: turned about the hinge.
        let turned = move |(x, y): (f64, f64), w: f64| {
            let d = hinge - y;
            place.at(x, hinge - d * th.cos() + w * th.sin(), h + d * th.sin() + w * th.cos())
        };
        // The flap's, turned first about its own hinge on the lid's top.
        let s = (sb.lid_angle / LID_FLAP_DEG).clamp(0.0, 1.0);
        let fold = std::f64::consts::PI * s * s * (3.0 - 2.0 * s);
        let folded = move |(x, y): (f64, f64), w: f64| {
            let (vx, vw) = (x - fold_x, w - LID_T);
            turned((fold_x + vx * fold.cos() + vw * fold.sin(), y), LID_T - vx * fold.sin() + vw * fold.cos())
        };
        let c = rgba(CASE, if open { 0.35 } else { 0.25 });
        lid_leaf(&mut b, &leaf, (0.0, LID_T), &turned, c);
        lid_leaf(&mut b, &flap, (0.0, LID_T), &folded, c);
        // The moulding under the flap's front edge, between the arms: the
        // top of the case's front, which goes with the flap.
        if front.is_some() {
            let (klo, khi) = keyboard_span(model, &sb);
            let (y0, y1) = (across_lo.min(klo - KEY_CHEEK), across_hi.max(khi + KEY_CHEEK).min(hinge));
            let (x0, x1) = (case_front - FRONT_T, case_front);
            lid_leaf(&mut b, &[(x0, y0), (x1, y0), (x1, y1), (x0, y1)], (-LID_DROP, 0.0), &folded, c);
        }
        // Labelled at its edge farthest from the hinge.
        let edge = leaf.iter().copied().min_by(|p, q| p.1.total_cmp(&q.1)).unwrap_or((sb.a * 0.6, 0.0));
        if open {
            b.label(turned(edge, LID_T), format!("Lid, {:.0}°", sb.lid_angle), "Microphones");
        } else {
            b.label(place.at(sb.a * 0.6, sb.b * 0.5, h + LID_T), "Lid, shut", "Microphones");
        }
        // The prop that holds it open: a stick from the treble side's rim
        // up to the lid's underside, square to the lid (as long as the
        // angle asks: a real one has two or three lengths).
        let prop_x = PROP_AT.min(0.5 * sb.a);
        let side = (0..n)
            .filter_map(|i| {
                let (p, q) = (outline[i], outline[(i + 1) % n]);
                ((p.0 - prop_x) * (q.0 - prop_x) <= 0.0 && (p.0 - q.0).abs() > 1e-9).then(|| p.1 + (q.1 - p.1) * (prop_x - p.0) / (q.0 - p.0))
            })
            .fold(f64::MAX, f64::min);
        if sb.lid_angle >= PROP_FROM_DEG && sb.lid_angle < 90.0 && side < hinge {
            let foot = side - (inner + t) / 2.0;
            let d = (hinge - foot) * th.cos();
            let tip = turned((prop_x, hinge - d), 0.0);
            let ring = |p: V3| (p, PROP_W as f32 / 2.0, PROP_T as f32 / 2.0, PROP_T as f32 / 3.0);
            b.sweep(&[ring(place.at(prop_x, foot, lid_top)), ring(tip)], place.dir(1.0, 0.0, 0.0), CASE);
        }
    }
    b.s.lid = b.s.glass.split_off(lid_from);

    // The keyboard, the keys down pressed in, its back against the case's
    // front: a grand's on its keybed, at the usual height above the floor.
    let keys_back = case_front - FRONT_T;
    let (klo, khi) = keyboard_span(model, &sb);
    let (y_back, kz) = if vertical { (UPRIGHT_KEYS_Y, KEY_HEIGHT - 0.022) } else { (keys_back, z0 + KEYBED_T + KEY_FRAME) };
    let key_top = kz + 0.022;
    let floor = if vertical { 0.0 } else { key_top - KEY_HEIGHT };
    let key_front = y_back - KEY_DEPTH;
    let ctx = action::Ctx {
        model,
        sb: &sb,
        ends: &ends,
        hx,
        obs,
        key: o.key,
        lit: &lit,
        level: &key_level,
        span: (klo, khi),
        key_front,
        key_back: y_back,
        kz,
        vertical,
    };
    b.layer_of(Layer::Played);
    if b.draw {
        action::keys(&mut b, &ctx);
    }
    b.layer_of(Layer::Still);
    let cx = -((klo + khi) / 2.0) as f32;
    // Under a grand's front legs, their centres (world x, y).
    let mut front_legs = [[0.0f32; 2]; 2];
    if vertical {
        if o.case {
            let across = (across_lo.min(outline.iter().map(|p| p.1).fold(f64::MAX, f64::min)) - t, across_hi.max(outline.iter().map(|p| p.1).fold(f64::MIN, f64::max)) + t);
            upright_case(&mut b, o, across, (klo, khi), kz, key_front, y_back, case);
        }
    } else {
        // Its case, as most grands': the keybed under the case's front
        // and the action, its sides carried forward as arms that come down in a curve from
        // the case's top to a little above the keys, rounded at their
        // front; the cheek blocks between them and the keys' ends, the key
        // slip along the keys' front, and behind them the fallboard (open)
        // over a strip of felt. Across (plate y): the case's sides, wider
        // if the keys are.
        let (lo, hi) = (across_lo.min(klo - KEY_CHEEK), across_hi.max(khi + KEY_CHEEK));
        let (x_lo, x_hi) = (-(hi + t) as f32, -(lo - t) as f32);
        let (yf, yb) = ((key_front - KEY_SLIP - 0.002) as f32, (y_back + FRONT_T) as f32);
        let (z0f, top, bed) = (z0 as f32, key_top as f32, (z0 + KEYBED_T) as f32);
        b.cuboid([x_lo, yf + 0.004, z0f], [x_hi - x_lo, 0.0, 0.0], [0.0, BELLY_AT as f32 - yf - 0.004, 0.0], [0.0, 0.0, KEYBED_T as f32], case);
        // The front legs under the arms, flush with their outsides.
        let y = yf + 0.1;
        front_legs = [[x_lo + 0.074, y], [x_hi - 0.074, y]];
        let arm = top + 0.035;
        let mut profile = vec![(yf, z0f), (yb, z0f), (yb, lid_top as f32)];
        for i in 1..=10 {
            let s = i as f32 / 10.0;
            profile.push((yb - 0.09 * s, arm + (lid_top as f32 - arm) * 0.5 * (1.0 + (std::f32::consts::PI * s).cos())));
        }
        let r = 0.022;
        for i in 0..=6 {
            let a = std::f32::consts::FRAC_PI_2 * i as f32 / 6.0;
            profile.push((yf + r - r * a.sin(), arm - r + r * a.cos()));
        }
        for (x0, x1) in [(x_lo, -hi as f32), (-lo as f32, x_hi)].into_iter().filter(|_| o.case) {
            b.slab(&profile, x0, x1, case);
        }
        for (x0, x1) in [(-hi as f32, -(khi + 0.002) as f32), (-(klo - 0.002) as f32, -lo as f32)].into_iter().filter(|_| o.case) {
            let block = |y: f32| ([(x0 + x1) / 2.0, y, (bed + top + 0.012) / 2.0], (x1 - x0) / 2.0, (top + 0.012 - bed) / 2.0, 0.008);
            b.sweep(&[block(yf + KEY_SLIP as f32), block(y_back as f32)], [1.0, 0.0, 0.0], case);
        }
        let along = |x0: f32, x1: f32, y: f32, z: (f32, f32), d: f32, r: f32| {
            let ring = |x: f32| ([x, y, (z.0 + z.1) / 2.0], d / 2.0, (z.1 - z.0) / 2.0, r);
            [ring(x0), ring(x1)]
        };
        // The fallboard over the keys, which run on under it, the name
        // on it; shut, a board over the keys from it to their front, the
        // name on that.
        if o.case {
            b.sweep(&along(-hi as f32, -lo as f32, yf + KEY_SLIP as f32 / 2.0, (z0f, kz as f32 - 0.002), KEY_SLIP as f32, 0.005), [0.0, 1.0, 0.0], case);
            b.sweep(&along(-hi as f32, -lo as f32, (y_back + FRONT_T / 2.0) as f32, (top + 0.014, PIN_BLOCK_TOP as f32), FRONT_T as f32, 0.009), [0.0, 1.0, 0.0], case);
            b.cuboid([-(khi + 0.002) as f32, y_back as f32 - 0.006, top - 0.004], [(khi - klo + 0.004) as f32, 0.0, 0.0], [0.0, 0.006, 0.0], [0.0, 0.0, 0.012], FELT);
            // Above it, the front closed up to the moulding under the
            // lid's flap, which closes the rest ([`LID_DROP`]), as solid as
            // asked. Behind it the pin block lies open under the flap,
            // which covers it when shut.
            let from = b.s.tris.len();
            let shut = (lid_top - LID_DROP) as f32;
            b.sweep(&along(-hi as f32, -lo as f32, (y_back + FRONT_T / 2.0) as f32, (PIN_BLOCK_TOP as f32, shut), FRONT_T as f32, 0.009), [0.0, 1.0, 0.0], case);
            b.see_through(from, o.front);
            fallboard(&mut b, o.fallboard, (-hi, -lo), cx as f64, (key_front, y_back), kz, case);
        }
    }
    if o.case && o.fallboard {
        b.s.keys.clear();
    }
    b.label([-((klo + khi) / 2.0) as f32, key_front as f32, (kz + 0.03) as f32], "Keyboard", "Player");

    // Pedals: soft, sostenuto, sustain from left to right.
    let values = obs.map_or([0.0; 3], |o| [o.soft, if o.sostenuto { 1.0 } else { 0.0 }, o.sustain]);
    let parts = [Part::UnaCorda, Part::UnaCorda, Part::SustainPedal];
    let pedals: [(f32, Color32); 3] = std::array::from_fn(|i| {
        let v = values[i];
        (v, if v > 0.05 || (i != 1 && lit(parts[i])) { sketches::HIGHLIGHT } else { BRASS })
    });
    b.mat = Material::Brass;
    if vertical {
        // Out of the plinth under the lower panel, through brass plates.
        let face = (y_back + U_LOWER_BACK - 0.01) as f32;
        for (i, &(v, c)) in pedals.iter().enumerate() {
            let x = cx + (i as f32 - 1.0) * PEDAL_PITCH;
            b.cuboid([x - 0.017, face - 0.002, floor as f32 + 0.012], [0.034, 0.0, 0.0], [0.0, 0.002, 0.0], [0.0, 0.0, 0.026], BRASS);
            b.layer_of(Layer::Played);
            pedal(&mut b, x, face, floor as f32 + 0.025, v, c);
            b.layer_of(Layer::Still);
        }
    } else {
        // The lyre under the keybed, in front of the fallboard, braced to
        // the belly rail; the third leg under the case's tail.
        lyre(&mut b, cx, y_back as f32 - 0.05, z0 as f32, floor as f32, ((BELLY_AT + BELLY_RAIL / 2.0) as f32, z0 as f32), pedals);
        let tail = place.at(sb.a - 0.12, sb.b * 0.5, z0);
        let legs = if b.draw { 3 } else { 0 };
        for [x, y] in front_legs.into_iter().chain([[tail[0], tail[1]]]).take(legs) {
            leg(&mut b, [x, y, z0 as f32], floor as f32);
        }
    }
    b.mat = Material::Paint;
    b.label([cx, key_front as f32 - 0.1, floor as f32 + 0.08], "Pedals", "Player");
    b.s.body = b.bounds();
    b.s.floor = floor as f32;
    b.s.live = live.is_some() && layer.has(Layer::Sounding);

    // The microphones and their stands.
    if o.mics {
        for (i, &piano_core::mics::Mic { at: (x, y, z), .. }) in sb.mics.iter().enumerate() {
            let c = MIC[i];
            let p = place.at(x, y, z);
            let r = 0.025f32;
            let ax = [[r, 0.0, 0.0], [0.0, r, 0.0], [0.0, 0.0, r]];
            // An octahedron.
            for sx in [-1.0f32, 1.0] {
                for sy in [-1.0f32, 1.0] {
                    for sz in [-1.0f32, 1.0] {
                        let (a, bb, cc) = (scale(ax[0], sx), scale(ax[1], sy), scale(ax[2], sz));
                        let mut t = [add(p, a), add(p, bb), add(p, cc)];
                        if sx * sy * sz < 0.0 {
                            t.swap(1, 2);
                        }
                        b.tri(false, t, [rgba(c, 1.0); 3]);
                    }
                }
            }
            let foot = [p[0], p[1], floor as f32];
            b.wire(p, foot, rgba(c, 0.7), 0.012);
            b.label(p, if i == 0 { "Mic L" } else { "Mic R" }, "Microphones");
        }
    }

    b.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options() -> Options<'static> {
        Options { key: 60, lit: &[], lid: true, frame: true, mics: true, case: true, fallboard: false, front: 1.0, ink: Color32::WHITE }
    }

    #[test]
    fn the_layers_make_the_whole() {
        let mut obs = Observation { sustain: 1.0, soft: 0.5, ..Default::default() };
        obs.keys[60].down = true;
        obs.keys[60].hammer = true;
        let levels: [f32; 128] = std::array::from_fn(|m| if m % 3 == 0 { 0.6 } else { 0.0 });
        let board = vec![0.4f32; BOARD_GRID.0 * BOARD_GRID.1];
        let live = Live { obs: &obs, keys: &levels, board: Some(&board), time: 0.3 };
        let key = |v: &Vertex| [v.pos, v.normal].concat().into_iter().chain(v.color).chain([v.mat, v.width]).map(f32::to_bits).collect::<Vec<_>>();
        let sorted = |a: &[&[Vertex]]| {
            let mut v: Vec<_> = a.iter().flat_map(|s| s.iter().map(key)).collect();
            v.sort();
            v
        };
        for vertical in [false, true] {
            let m = PianoModel::default();
            let m = PianoModel { soundboard: m.soundboard.map(|sb| SoundboardParams { vertical, ..sb }), ..m };
            for o in [options(), Options { case: false, fallboard: true, ..options() }] {
                let all = build(&m, Some(&live), &o);
                let [still, played, sounding] = Layer::PARTS.map(|l| build_layer(&m, if l == Layer::Still { None } else { Some(&live) }, &o, l));
                let parts = [&still, &played, &sounding];
                assert_eq!(sorted(&[&all.tris]), sorted(&parts.map(|s| &s.tris[..])));
                assert_eq!(sorted(&[&all.lines]), sorted(&parts.map(|s| &s.lines[..])));
                assert_eq!(sorted(&[&all.glass, &all.lid]), sorted(&[&still.glass, &still.lid, &played.glass, &played.lid, &sounding.glass, &sounding.lid]));
                let merged = Scene::merged(&parts);
                assert_eq!(merged.tris.len(), all.tris.len());
                assert_eq!((merged.body, merged.labels.len()), (all.body, all.labels.len()));
                assert_eq!(all.labels, still.labels);
                assert!(played.labels.is_empty() && sounding.labels.is_empty() && still.keys.is_empty() && all.keys == played.keys);
                assert_eq!(all.board, still.board);
                assert!(all.live && sounding.live && !played.live && !still.live);
                // Most of it stands still; what sounds is little.
                assert!(played.tris.len() < still.tris.len() && sounding.tris.len() * 10 < played.tris.len(), "{} {} {}", still.tris.len(), played.tris.len(), sounding.tris.len());
            }
        }
    }

    #[test]
    fn keys_are_found_where_they_are_drawn() {
        let m = PianoModel::default();
        let s = build(&m, None, &options());
        // Straight down on the middle of each key's part in front: that key.
        for &(note, lo, hi) in &s.keys {
            let c = scale(add(lo, hi), 0.5);
            assert_eq!(s.key_at([c[0], c[1], 3.0], [0.0, 0.0, -1.0]).map(|k| k.0), Some(note), "{note}");
        }
        assert!((21..=108).all(|n| s.keys.iter().any(|k| k.0 == n)));
        // From the pianist's seat, through the front of the keyboard, a
        // white key; over its back, a black one.
        let (_, lo, hi) = *s.keys.iter().find(|k| k.0 == 61).unwrap();
        let x = (lo[0] + hi[0]) / 2.0;
        assert_eq!(s.key_at([x, lo[1] - 1.0, hi[2] + 0.1], norm([0.0, 1.0, -0.1])).map(|k| k.0), Some(61));
        // Where along: its front, its back.
        let along = |y: f32| s.key_at([x, y, 3.0], [0.0, 0.0, -1.0]).unwrap().1;
        assert!(along(lo[1] + 0.001) < 0.02 && along(hi[1] - 0.001) > 0.98, "{} {}", along(lo[1] + 0.001), along(hi[1] - 0.001));
        assert_eq!(s.key_at([x, lo[1] - 1.0, hi[2] - 0.035], [0.0, 1.0, 0.0]), None);
        // The fallboard shut, none.
        let shut = build(&m, None, &Options { fallboard: true, ..options() });
        assert!(shut.keys.is_empty());
        let up = PianoModel { soundboard: m.soundboard.map(|sb| SoundboardParams { vertical: true, ..sb }), ..m };
        assert_eq!(build(&up, None, &options()).keys.len(), s.keys.len());
    }

    #[test]
    fn the_default_grand_is_whole_and_to_scale() {
        let m = PianoModel::default();
        let s = build(&m, None, &options());
        assert_eq!(s.tris.len() % 3, 0);
        assert_eq!(s.lines.len() % 2, 0);
        assert!(s.tris.iter().chain(&s.lines).all(|v| v.pos.iter().chain(&v.normal).all(|x| x.is_finite())));
        // No flattened box: every face has an area (but the name's
        // letters, whose curves leave thin slivers).
        for t in s.tris.as_chunks::<3>().0.iter().filter(|t| t[0].mat != Material::Gilt as u8 as f32) {
            let area = dot(cross(sub(t[1].pos, t[0].pos), sub(t[2].pos, t[0].pos)), cross(sub(t[1].pos, t[0].pos), sub(t[2].pos, t[0].pos)));
            assert!(area > 1e-14, "flat triangle {t:?}");
        }
        let sb = m.soundboard.unwrap();
        // The board spans its a × b, the rim around it.
        assert!((s.max[1] - s.min[1]) as f64 >= sb.a, "{:?} {:?}", s.min, s.max);
        assert!((s.max[0] - s.min[0]) as f64 >= sb.b);
        assert!(s.labels.iter().any(|l| l.text == "Soundboard"));
        // Strings cross: the bass ones start across the keyboard's bass end.
        let ((_, a0), (_, b0), _) = string_ends(&m, &sb, hammer_x(&m, &sb), 21);
        assert!(a0 > b0, "A0 from y = {a0} to {b0}");
    }

    #[test]
    fn the_strings_of_a_bridge_do_not_cross_each_other() {
        let m = PianoModel::default();
        let sb = m.soundboard.unwrap().resolved();
        // Two segments cross when each one's ends lie on either side of the other.
        let crosses = |(p, q): ((f64, f64), (f64, f64)), (r, s): ((f64, f64), (f64, f64))| {
            cross2(p, q, r) * cross2(p, q, s) < 0.0 && cross2(r, s, p) * cross2(r, s, q) < 0.0
        };
        let hx = hammer_x(&m, &sb);
        for (lo, hi) in [(21, sb.main_bridge_from), (sb.main_bridge_from, 109)] {
            for i in lo..hi {
                for j in i + 1..hi {
                    let (a, b) = (string_ends(&m, &sb, hx, i), string_ends(&m, &sb, hx, j));
                    assert!(!crosses((a.0, a.1), (b.0, b.1)), "keys {i} and {j} cross");
                }
            }
        }
    }

    #[test]
    fn the_hammers_are_in_line_the_agraffes_on_a_diagonal() {
        let m = PianoModel::default();
        let sb = m.soundboard.unwrap().resolved();
        let hx = hammer_x(&m, &sb);
        let ends: Vec<_> = (21..=108u8).map(|k| (k, string_ends(&m, &sb, hx, k))).collect();
        for &(k, ((ax, ay), (bx, by), _)) in &ends {
            // (Struck at the engine's distance from the agraffe, whatever
            // the string's length as drawn.)
            let n = m.note(k);
            let s = drawn_share(&m, &sb, ((ax, ay), (bx, by), 0.0), k, n.strike_pos);
            assert!((ax + s * (bx - ax) - hx).abs() < 1e-9, "key {k} struck off the line");
            assert!((ay + s * (by - ay) - key_across(&m, &sb, k)).abs() < 1e-9, "key {k} struck off its key");
            assert!(((hx - ax).hypot(key_across(&m, &sb, k) - ay) - n.strike_pos * n.length).abs() < 1e-9, "key {k}'s agraffe");
            assert!(ax < hx, "key {k}'s agraffe at {ax}");
        }
        // The hammers in front of the board; the bass's agraffe in front,
        // the treble's well behind it, just in front of the hammers.
        let (a0, c8) = (ends[0].1.0.0, ends[87].1.0.0);
        assert!(hx <= -HAMMER_CLEAR, "{hx}");
        assert!(ends[0].1.0.1 > key_across(&m, &sb, 21) + 0.02, "A0's agraffe not to the bass side: {} from its key", ends[0].1.0.1 - key_across(&m, &sb, 21));
        assert!(c8 - a0 > 0.1 && hx - c8 < 0.01, "A0 at {a0}, C8 at {c8}, hammers at {hx}");
        // A tuning pin per string, in front of its agraffe.
        let strings: usize = (21..=108u8).map(|k| m.note(k).strings).sum();
        let pins = build(&m, None, &options()).tris.iter().filter(|v| v.color == rgba(PIN, 1.0)).count();
        assert_eq!(pins, strings * (5 * 12 * 2 + 12) * 3);
        let tuning = tuning_pins(&m, &sb, &ends.iter().map(|e| e.1).collect::<Vec<_>>());
        assert!(tuning.iter().all(|p| p.at.0 < p.hole.0 - PIN_GAP + 0.001), "a pin behind its agraffe");
    }

    #[test]
    fn an_outlined_board_is_drawn_within_its_outline() {
        let mut m = PianoModel::default();
        let sb = m.soundboard.as_mut().unwrap();
        sb.outline = SoundboardParams::GRAND_OUTLINE;
        let sb = *sb;
        let outline = board_outline(&sb);
        let area = signed_area(&outline);
        assert!(area > 0.0 && area < 0.9 * sb.a * sb.b, "{area}");
        // The grid's pieces cover the outline, the underside too.
        let pieces: f64 = (0..10)
            .flat_map(|i| (0..10).map(move |j| (i, j)))
            .map(|(i, j)| {
                let (x0, y0, w, h) = (sb.a * i as f64 / 10.0, sb.b * j as f64 / 10.0, sb.a / 10.0, sb.b / 10.0);
                signed_area(&clip(&outline, &[(x0, y0), (x0 + w, y0), (x0 + w, y0 + h), (x0, y0 + h)]))
            })
            .sum();
        assert!((pieces - area).abs() < 1e-9 * area.max(1.0), "{pieces} vs {area}");
        let under: f64 = triangulate(&outline).iter().map(|t| signed_area(&t.map(|i| outline[i]))).sum();
        assert!((under - area).abs() < 1e-9, "{under} vs {area}");
        // No board drawn outside it: the board's top lies within the outline.
        let s = build(&m, None, &options());
        let corner = Place { vertical: false, top: 0.0 }.at(sb.a * 0.95, sb.b * 0.05, 0.0);
        assert!(!s.tris.iter().any(|v| v.pos[2] == 0.0 && (v.pos[0] - corner[0]).abs() < 0.02 && (v.pos[1] - corner[1]).abs() < 0.02));
    }

    #[test]
    fn a_rounded_board_is_covered_by_its_top() {
        // The grand's, and one edited: its spine's back end 5 cm forward,
        // which would bend the tail out past the board's length.
        let mut edited = SoundboardParams::GRAND_2M;
        edited.outline[8].0 = 1.0 - 0.05 / edited.a;
        for sb in [SoundboardParams::GRAND_2M, edited] {
            let m = PianoModel { soundboard: Some(sb), ..PianoModel::default() };
            let sb = m.soundboard.unwrap().resolved();
            let outline = board_outline(&sb);
            assert!(outline.iter().all(|&(x, y)| (0.0..=sb.a).contains(&x) && (0.0..=sb.b).contains(&y)));
            // Without the case, whose inner rim the board's glued edge
            // covers beyond the outline.
            let s = build(&m, None, &Options { case: false, ..options() });
            let top: f64 = s
                .tris
                .as_chunks::<3>()
                .0
                .iter()
                .filter(|t| t.iter().all(|v| v.pos[2] == 0.0 && v.mat == Material::Spruce as u8 as f32))
                .map(|t| 0.5 * (dot(cross(sub(t[1].pos, t[0].pos), sub(t[2].pos, t[0].pos)), cross(sub(t[1].pos, t[0].pos), sub(t[2].pos, t[0].pos))) as f64).sqrt())
                .sum();
            let area = signed_area(&outline);
            assert!((top / area - 1.0).abs() < 1e-4, "{top} vs {area}");
        }
    }

    #[test]
    fn a_rounded_outline_is_a_curve_through_the_models() {
        let sb = SoundboardParams { outline: SoundboardParams::GRAND_OUTLINE, ..SoundboardParams::GRAND_2M };
        let mut model = Outline::new(&sb).unwrap().points;
        if signed_area(&model) < 0.0 {
            model.reverse();
        }
        let round = rounded(&model);
        assert!(round.len() > 3 * model.len(), "{}", round.len());
        // Every vertex of the model's on it, in order.
        let mut at = 0;
        for p in &model {
            at += round[at..].iter().position(|q| q == p).expect("a vertex left out");
        }
        // Close to the polygon: within 2.5 cm of it, its area within 1 %.
        let near = Outline { points: model.clone() };
        assert!(round.iter().all(|&(x, y)| near.edge_distance(x, y) < 0.025));
        assert!((signed_area(&round) / signed_area(&model) - 1.0).abs() < 0.01);
        // Smooth but at the three corners (front, spine): no bend over a
        // few degrees elsewhere.
        let n = round.len();
        let bends = (0..n).filter(|&i| {
            let (a, b, c) = (round[(i + n - 1) % n], round[i], round[(i + 1) % n]);
            let (u, v) = ((b.0 - a.0, b.1 - a.1), (c.0 - b.0, c.1 - b.1));
            (u.0 * v.0 + u.1 * v.1) / (u.0.hypot(u.1) * v.0.hypot(v.1)) < 8f64.to_radians().cos()
        });
        assert_eq!(bends.count(), 3);
        // A rectangle stays one.
        let rect = [(0.0, 0.0), (1.7, 0.0), (1.7, 1.3), (0.0, 1.3)];
        assert_eq!(rounded(&rect), rect);
    }

    #[test]
    fn the_rim_is_one_wall_without_overlaps() {
        let mut m = PianoModel::default();
        m.soundboard.as_mut().unwrap().outline = SoundboardParams::GRAND_OUTLINE;
        let sb = m.soundboard.unwrap();
        let outline = board_outline(&sb);
        let mut b = Builder::new(Place { vertical: false, top: 0.0 }, Layer::All);
        let (t, z0, z1) = (0.075, -0.25, 0.05);
        b.wall(&outline, true, 0.0, t, z0, &|_| z1, CASE);
        // Its top is the ring between the outline and its offset: its area
        // is the offset polygon's minus the outline's, no more (no overlap).
        let top: f64 = b.s.tris.as_chunks::<3>().0.iter().filter(|t| t.iter().all(|v| (v.pos[2] - z1 as f32).abs() < 1e-6)).map(|t| {
            let p = t.map(|v| (v.pos[0] as f64, v.pos[1] as f64));
            signed_area(&p).abs()
        }).sum();
        let perimeter: f64 = outline.iter().zip(outline.iter().cycle().skip(1)).map(|(p, q)| (q.0 - p.0).hypot(q.1 - p.1)).sum();
        // A ring t wide around a convex-ish outline: perimeter·t plus the
        // corners' mitres, well under the overlapping blocks' sum.
        assert!(top > perimeter * t && top < perimeter * t + 8.0 * t * t, "{top} vs {}", perimeter * t);
        // Every normal is a unit vector.
        assert!(b.s.tris.iter().all(|v| (dot(v.normal, v.normal) - 1.0).abs() < 1e-4));
    }

    #[test]
    fn any_bass_bridge_builds_grand_or_upright() {
        // Down to none: the main bridge carrying every string from A0.
        for vertical in [false, true] {
            for from in [21, 22, 28, 45, 60, 107] {
                for case in [true, false] {
                    let mut m = PianoModel::default();
                    let sb = m.soundboard.as_mut().unwrap();
                    (sb.vertical, sb.main_bridge_from) = (vertical, from);
                    let s = build(&m, None, &Options { case, ..options() });
                    assert!(!s.tris.is_empty(), "vertical {vertical}, main bridge from {from}");
                    assert!(s.tris.iter().chain(&s.lines).all(|v| v.pos.iter().all(|x| x.is_finite())), "vertical {vertical}, main bridge from {from}, case {case}");
                }
            }
        }
    }

    #[test]
    fn the_keys_and_the_strings_fit_inside_the_case() {
        let m = PianoModel::default();
        let sb = m.soundboard.unwrap().resolved();
        // The keyboard and its cheek blocks between the rim's walls, its
        // arms in line with them.
        let (lo, hi) = keyboard_span(&m, &sb);
        assert!(lo >= KEY_CHEEK - 1e-9 && hi <= sb.b - KEY_CHEEK + 1e-9, "keys from {lo} to {hi}, board {}", sb.b);
        // Every string's front end and tuning pin inside the bass side.
        let hx = hammer_x(&m, &sb);
        let ends: Vec<_> = (21..=108u8).map(|k| string_ends(&m, &sb, hx, k)).collect();
        assert!(ends.iter().all(|e| e.0.1 <= sb.b - AGRAFFE_EDGE + 1e-9), "an agraffe in the wall");
        let pins = tuning_pins(&m, &sb, &ends);
        let far = pins.iter().map(|p| p.at.1).fold(f64::MIN, f64::max);
        assert!(far + PIN_BUSH_R < sb.b, "a tuning pin at {far}");
        // The bass's hammers closer together, but no closer than half the
        // others' pitch, in order.
        let main = key_across(&m, &sb, 107) - key_across(&m, &sb, 108);
        for k in 21..sb.main_bridge_from {
            let gap = key_across(&m, &sb, k) - key_across(&m, &sb, k + 1);
            assert!(gap >= 0.5 * main - 1e-12 && gap <= main + 1e-12, "key {k}: {gap} vs {main}");
        }
    }

    /// The strings' sections part the hammers: a gap below each section's
    /// lowest key, the bass's included, on top of the pitch a concert
    /// grand's hammers have within a section (its action wider than its
    /// keys, [`HAMMER_FAN`]), the whole within the case's walls, each key
    /// still struck on its strings; none, and nothing moves.
    #[test]
    fn the_sections_part_the_hammers() {
        let preset = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../presets/instruments/Concert grand/concert_grand.toml");
        let mut even = piano_host::load_preset(&preset).unwrap();
        even.soundboard.as_mut().unwrap().string_sections = Default::default();
        let sb0 = even.soundboard.unwrap().resolved();
        let lo = sb0.main_bridge_from;
        let mut m = even.clone();
        let breaks = [lo, 56, 74, 92];
        m.soundboard.as_mut().unwrap().string_sections = breaks.map(|k| (k as f64, 0.014));
        let sb = m.soundboard.unwrap().resolved();
        let (treble, bass) = case_sides(&sb);
        // (C8's hammer in line with its strings on the measured bridge.)
        assert!((key_across(&m, &sb, 108) - sb.bridge_point(108).1).abs() < 1e-12 && key_across(&m, &sb, 108) > treble && key_across(&m, &sb, 21) <= bass - AGRAFFE_EDGE + 1e-12, "hammers from {} to {}", key_across(&m, &sb, 108), key_across(&m, &sb, 21));
        let step = |k: u8| key_across(&m, &sb, k - 1) - key_across(&m, &sb, k);
        let pitch = step(108);
        for k in 22..=108u8 {
            let gap = if breaks.contains(&k) { 0.014 } else { 0.0 };
            let want = if k <= lo { step(22) } else { pitch };
            assert!((step(k) - want - gap).abs() < 1e-9, "key {k}: {} for {}", step(k), want + gap);
        }
        assert!((pitch - (key_across(&even, &sb0, 107) - key_across(&even, &sb0, 108))).abs() < 1e-9);
        assert!((pitch - HAMMER_FAN * WHITE_KEY * 7.0 / 12.0).abs() < 1e-9, "{pitch}");
        let hx = hammer_x(&m, &sb);
        for k in 21..=108u8 {
            let ((ax, ay), (bx, by), _) = string_ends(&m, &sb, hx, k);
            // (On its own place of the hammers' line, which slants a little.)
            let s = (hammers(&m, &sb).at(key_across(&m, &sb, k)) - ax) / (bx - ax);
            assert!((ay + s * (by - ay) - key_across(&m, &sb, k)).abs() < 1e-9, "key {k} struck off its key");
        }
        // The Concert grand's hammers' line slants by a degree or two, the
        // treble's nearer the keyboard, and clears the board's front edge.
        let line = hammers(&m, &sb);
        assert!(line.slope > 0.015 && line.slope <= HAMMER_SLANT, "{line:?}");
        let outline = board_outline(&sb);
        let f = front_edge(&outline, &sb).unwrap();
        let edge = outline[f].0.min(outline[(f + 1) % outline.len()].0);
        assert!(line.at(key_across(&m, &sb, 108)) <= edge - HAMMER_EDGE + 1e-9, "{line:?}, edge at {edge}");
    }

    /// The Concert grand's board: its front edge runs forward towards the
    /// treble, and its bridge points are the strings' real ends, wider
    /// apart than its keys.
    #[test]
    fn a_slanted_front_edge_is_the_front_and_the_keyboard_a_real_one() {
        let preset = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../presets/instruments/Concert grand/concert_grand.toml");
        let m = piano_host::load_preset(&preset).unwrap();
        let sb = m.soundboard.unwrap().resolved();
        let outline = board_outline(&sb);
        let k = front_edge(&outline, &sb).expect("a front edge");
        let (p, q) = (outline[k], outline[(k + 1) % outline.len()]);
        assert!(p.1 > 0.95 * sb.b && q.1 < 0.05 * sb.b && (p.0 - q.0).abs() > 0.05, "front edge from {p:?} to {q:?}");
        let (lo, hi) = keyboard_span(&m, &sb);
        // (To a few millimetres: the semitones are even, the white keys not quite.)
        assert!((hi - lo - 52.0 * WHITE_KEY).abs() < 0.005, "keys {} m wide", hi - lo);
        assert!(lo >= KEY_CHEEK && hi <= sb.b - KEY_CHEEK, "keys from {lo} to {hi}, board {}", sb.b);
        // The default's is the rectangle's, or its outline's, at x = 0.
        let sb = PianoModel::default().soundboard.unwrap().resolved();
        let outline = board_outline(&sb);
        let k = front_edge(&outline, &sb).unwrap();
        assert!(outline[k].0.abs() < 1e-9 && outline[(k + 1) % outline.len()].0.abs() < 1e-9);
    }

    #[test]
    fn the_strings_pass_between_the_tuning_pins() {
        // The default grand, whose unisons are too close in places for
        // the strings to clear every pin (they graze them, 0.27 mm into a
        // pin 7 mm across at the worst), and the Concert grand, whose
        // tenor is as close (0.09 mm into a pin at the worst).
        let preset = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../presets/instruments/Concert grand/concert_grand.toml");
        for (m, least, apart) in [(PianoModel::default(), -0.0003, -0.0005), (piano_host::load_preset(&preset).unwrap(), -0.0003, -0.0005)] {
        let sb = m.soundboard.unwrap().resolved();
        let hx = hammer_x(&m, &sb);
        let ends: Vec<_> = (21..=108u8).map(|k| string_ends(&m, &sb, hx, k)).collect();
        let pins = tuning_pins(&m, &sb, &ends);
        assert_eq!(pins.len(), (21..=108u8).map(|k| m.note(k).strings).sum::<usize>());
        let coil = PIN_R + PIN_WIRE;
        let (mut wire_gap, mut pin_gap) = (f64::MAX, f64::MAX);
        for (i, p) in pins.iter().enumerate() {
            // From where it leaves its coil to its agraffe.
            let (dx, dy) = (p.hole.0 - p.at.0, p.hole.1 - p.at.1);
            let l = dx.hypot(dy);
            // (A grand's string leaves the coil on its right, at a tangent.)
            let side = (p.dir.1, -p.dir.0);
            let turn = (coil / l).min(1.0).acos();
            let e = (dx / l, dy / l);
            let s2 = if e.0 * side.1 - e.1 * side.0 > 0.0 { 1.0 } else { -1.0 };
            let a = (p.at.0 + coil * (e.0 * turn.cos() - s2 * e.1 * turn.sin()), p.at.1 + coil * (e.1 * turn.cos() + s2 * e.0 * turn.sin()));
            for (j, q) in pins.iter().enumerate() {
                if i == j {
                    continue;
                }
                pin_gap = pin_gap.min((q.at.0 - p.at.0).hypot(q.at.1 - p.at.1) - 2.0 * coil);
                let (ex, ey) = (p.hole.0 - a.0, p.hole.1 - a.1);
                let t = (((q.at.0 - a.0) * ex + (q.at.1 - a.1) * ey) / (ex * ex + ey * ey)).clamp(0.0, 1.0);
                let gp = (a.0 + ex * t - q.at.0).hypot(a.1 + ey * t - q.at.1) - PIN_R;
                wire_gap = wire_gap.min(gp);
            }
        }
        assert!(wire_gap > least, "a string touches a pin: {wire_gap}");
        // (Where the strings fan and the unisons are close, two coils of
        // neighbouring unisons overlap by 0.3 mm at the worst.)
        assert!(pin_gap > apart, "two coils touch: {pin_gap}");
        }
    }

    #[test]
    fn the_parts_are_made_of_something() {
        // Only the microphones and the agraffes' holes are plain paint;
        // the strings are wires as thick as they are.
        let mut model = PianoModel::default();
        for (case, vertical) in [(true, false), (false, false), (true, true)] {
            model.soundboard.as_mut().unwrap().vertical = vertical;
            let s = build(&model, None, &Options { key: 0, case, ..options() });
            let paint = Material::Paint as u8 as f32;
            let mut colours: Vec<_> = s.tris.iter().filter(|v| v.mat == paint).map(|v| v.color.map(|c| (c * 255.0).round() as u8)).collect();
            colours.sort();
            colours.dedup();
            let allowed: Vec<_> = MIC.iter().copied().chain([Color32::from_rgb(40, 32, 20)]).map(|c| rgba(c, 1.0).map(|c| (c * 255.0).round() as u8)).collect();
            assert!(colours.iter().all(|c| allowed.contains(c)), "unmade colours {colours:?} (case {case}, upright {vertical})");
            let strings = s.lines.iter().filter(|v| [Material::Steel, Material::Copper].map(|m| m as u8 as f32).contains(&v.mat));
            assert!(strings.clone().count() > 400 && strings.clone().all(|v| v.width > 0.0005 && v.width < 0.007));
        }
    }

    #[test]
    fn under_a_capo_bar_the_strings_bear_on_bars_six_to_each() {
        // A concert grand's sections, the capo bar from the fourth.
        let mut m = PianoModel::default();
        m.soundboard.as_mut().unwrap().string_sections = [41u8, 56, 74, 92].map(|k| (k as f64, 0.014));
        let sb = m.soundboard.unwrap().resolved();
        let hx = hammer_x(&m, &sb);
        let ends: Vec<_> = (21..=108u8).map(|k| string_ends(&m, &sb, hx, k)).collect();
        // Off: agraffes on every key, no bar.
        assert!((21..=108u8).all(|k| duplex_group(&m, &sb, k).is_none()));
        let brass = |m: &PianoModel| build(m, None, &options()).tris.iter().filter(|v| v.color == rgba(BRASS, 1.0)).count();
        let all = brass(&m);
        m.frame.capo_bar = true;
        let groups: Vec<_> = (74..=108u8).filter_map(|k| duplex_group(&m, &sb, k)).collect::<std::collections::BTreeSet<_>>().into_iter().collect();
        assert_eq!(groups, [(74, 79), (80, 85), (86, 91), (92, 97), (98, 103), (104, 108)]);
        assert!(duplex_group(&m, &sb, 73).is_none());
        // Each bar slanted, nearer the capo at its treble end, stepping
        // back to the next; the pins further than the bars and their felts.
        assert!((duplex_at(&m, &sb, 74).unwrap() - m.frame.duplex_far).abs() < 1e-12 && (duplex_at(&m, &sb, 79).unwrap() - m.frame.duplex_near).abs() < 1e-12);
        assert!(duplex_at(&m, &sb, 80).unwrap() > duplex_at(&m, &sb, 79).unwrap());
        let pins = tuning_pins(&m, &sb, &ends);
        let top: Vec<_> = pins.iter().rev().take(3).collect();
        assert!(top.iter().all(|p| p.rise > 0.0 && (p.at.0 - ends[87].0 .0).abs() > m.frame.duplex_far + DUPLEX_FELT + DUPLEX_FELT_W / 2.0));
        assert!(pins[0].rise == 0.0);
        // No agraffes under the bar.
        assert!(brass(&m) < all, "{} of {all}", brass(&m));
        // An upright has none.
        m.soundboard.as_mut().unwrap().vertical = true;
        let sb = m.soundboard.unwrap().resolved();
        assert!(duplex_group(&m, &sb, 100).is_none());
    }

    #[test]
    fn a_dampers_set_forward_rest_nearer_the_hammers() {
        let mut m = PianoModel::default();
        let sb = m.soundboard.unwrap().resolved();
        let hx = hammer_x(&m, &sb);
        let at = |m: &PianoModel, k: u8| action::damper_at(m, string_ends(m, &sb, hx, k), k);
        // In a line behind the hammers', as far as the engine has them.
        for k in [21u8, 40, 41, 60, 76] {
            let (x, y) = at(&m, k);
            let d = (x - hx).hypot(y - key_across(&m, &sb, k));
            assert!((d - m.damper.behind_strike).abs() < 2e-3, "key {k}'s damper {d} behind its hammer");
        }
        let line = at(&m, 40).0;
        m.damper.dodge = true;
        assert!((line - at(&m, 40).0 - m.damper.dodge_by).abs() < 2e-3);
        assert!((at(&m, 38).0 - line).abs() < 5e-3);
    }

    #[test]
    fn the_plate_stands_over_the_dampers_lifted() {
        // Every damper up (the pedal down): their heads under the plate's
        // struts, and behind the hammers' line (the bar in front of it).
        let m = PianoModel::default();
        let mut obs = Observation::default();
        for k in obs.keys.iter_mut() {
            k.damper = 0.0;
        }
        let levels = [0.0f32; 128];
        let live = Live { obs: &obs, keys: &levels, board: None, time: 0.0 };
        let s = build(&m, Some(&live), &options());
        let heads = [rgba(DAMPER_FELT, 1.0), rgba(DAMPER_WOOD, 1.0)];
        let (top, front) = s.tris.iter().filter(|v| heads.contains(&v.color)).fold((f32::MIN, f32::MAX), |(z, y), v| (z.max(v.pos[2]), y.min(v.pos[1])));
        assert!(top > 0.05 && (top as f64) < plate::CLEAR - 0.002, "heads up to {top}");
        // World y is plate x on a grand.
        let sb = m.soundboard.unwrap().resolved();
        assert!((front as f64) > hammer_x(&m, &sb) + 0.01, "heads from {front}");
    }

    /// The default grand on a built board (the D's structure).
    fn built() -> PianoModel {
        let mut m = PianoModel::default();
        m.soundboard.as_mut().unwrap().structure.enabled = true;
        m
    }

    /// The settings of the board as the D is drawn: feathered ribs and the
    /// bridge in one piece show as the engine has them.
    #[test]
    fn feathered_ribs_and_the_ring_show() {
        let ribs = |s: &Scene| s.tris.chunks(3).filter(|t| t[0].color == rgba(RIB_WOOD, 1.0)).count();
        let bridge = |s: &Scene| s.tris.chunks(3).filter(|t| t[0].color == rgba(sketches::BRIDGE, 1.0)).count();
        let lowest = |s: &Scene| s.tris.iter().filter(|v| v.color == rgba(RIB_WOOD, 1.0)).map(|v| v.pos[2]).fold(f32::MAX, f32::min);
        let plain = build(&built(), None, &options());
        let mut m = built();
        let st = &mut m.soundboard.as_mut().unwrap().structure;
        st.rib_feather = 0.15;
        // Every rib as high at its ends as in its middle: only the feather thins it.
        for r in &mut st.ribs {
            r.5 = r.4;
        }
        let feathered = build(&m, None, &options());
        assert_eq!(ribs(&feathered), 16 * (3 * (RIB_SECTIONS + 2 * RIB_FEATHER_SECTIONS) + 2) * 2);
        // As deep in the middle as the parabola's ribs; an end face 4 mm high under the board, not the rib's whole height.
        assert!((lowest(&feathered) - lowest(&plain)).abs() < 1e-4);
        let st = m.soundboard.unwrap().structure;
        let (rib, plate) = (st.ribs[8], st.plate_edge);
        let len = ((rib.2 - rib.0) * m.soundboard.unwrap().a).hypot((rib.3 - rib.1) * m.soundboard.unwrap().b);
        assert!((st.rib_thickness_at(&rib, 0.0, len, plate) - plate - st.rib_tip).abs() < 1e-3 && st.rib_thickness_at(&rib, 0.5 * len, len, plate) == rib.4);
        // The ring: more of the bridge, and none without the setting.
        assert_eq!(bridge(&feathered), bridge(&plain));
        m.soundboard.as_mut().unwrap().structure.bridge_ring = 0.1;
        let ringed = build(&m, None, &options());
        assert!(bridge(&ringed) > bridge(&plain) + 20, "{} against {}", bridge(&ringed), bridge(&plain));
        assert_eq!(piano_core::fem::ring_line(&built().soundboard.unwrap()).len(), 0);
    }

    #[test]
    fn a_built_board_shows_its_ribs_and_its_thickness() {
        let (flat, scene) = (build(&PianoModel::default(), None, &options()), build(&built(), None, &options()));
        let ribs = |s: &Scene| s.tris.chunks(3).filter(|t| t[0].color == rgba(RIB_WOOD, 1.0)).count();
        assert_eq!(ribs(&flat), 0);
        // 16 ribs: three faces a section and two ends, two triangles each.
        assert_eq!(ribs(&scene), 16 * (3 * RIB_SECTIONS + 2) * 2);
        assert!(scene.labels.iter().any(|l| l.text == "Ribs") && !flat.labels.iter().any(|l| l.text == "Ribs"));
        // Under the board, as deep as the deepest rib (3.42 cm); their
        // faces turned away from their middles.
        let s = built().soundboard.unwrap().structure;
        let rib: Vec<_> = scene.tris.chunks(3).filter(|t| t[0].color == rgba(RIB_WOOD, 1.0)).collect();
        let (lo, hi) = rib.iter().flat_map(|t| t.iter()).fold((f32::MAX, f32::MIN), |(lo, hi), v| (lo.min(v.pos[2]), hi.max(v.pos[2])));
        assert!((lo as f64 + 0.0342).abs() < 1e-4 && (hi as f64) < -s.plate_edge + 1e-6 && (hi as f64) > -s.plate_center - 1e-6, "{lo} {hi}");
        let per = (3 * RIB_SECTIONS + 2) * 2;
        for one in rib.chunks(per) {
            let c = scale(one.iter().flat_map(|t| t.iter()).fold([0.0; 3], |a, v| add(a, v.pos)), 1.0 / (3 * per) as f32);
            // (The ends' and the middle's sections face it squarely enough.)
            let out = one.iter().filter(|t| dot(t[0].normal, sub(scale(add(add(t[0].pos, t[1].pos), t[2].pos), 1.0 / 3.0), c)) > 0.0).count();
            assert!(out >= per - 4, "{out} of {per}");
        }
    }

    #[test]
    fn a_built_board_holds_its_strings_as_high_as_its_bridges() {
        let m = built();
        let sb = m.soundboard.unwrap().resolved();
        let s = &sb.structure;
        // Flat where the board is not built.
        let plain = PianoModel::default().soundboard.unwrap().resolved();
        for k in 21..=108u8 {
            assert_eq!(bridge_z(&plain, k), STRING_Z + if k < plain.main_bridge_from { BASS_RISE } else { 0.0 });
        }
        // The main bridge's strings on the engine's bridge, 7 cm thick at
        // F3 and 2.9 cm at C8, less the plate's 9 mm.
        let over = STRING_Z - BRIDGE_H;
        for (k, th) in [(53u8, 0.07), (108, 0.029)] {
            assert!((bridge_z(&sb, k) - (th - s.plate_center + over)).abs() < 1e-9, "{k}");
        }
        // The bass strings over every other string they cross, their
        // hammers and dampers at their height there.
        let hx = hammer_x(&m, &sb);
        let ends: Vec<_> = (21..=108u8).map(|k| string_ends(&m, &sb, hx, k)).collect();
        let mut crossings = 0;
        for i in 21..sb.main_bridge_from {
            let (a, b, _) = ends[(i - 21) as usize];
            for j in sb.main_bridge_from..=108 {
                let (c, d, _) = ends[(j - 21) as usize];
                let (den, e) = ((b.0 - a.0) * (d.1 - c.1) - (b.1 - a.1) * (d.0 - c.0), (c.0 - a.0, c.1 - a.1));
                let (t, u) = ((e.0 * (d.1 - c.1) - e.1 * (d.0 - c.0)) / den, (e.0 * (b.1 - a.1) - e.1 * (b.0 - a.0)) / den);
                if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) {
                    crossings += 1;
                    let clear = string_z(&sb, ends[(i - 21) as usize], i, t) - string_z(&sb, ends[(j - 21) as usize], j, u);
                    // (Centre to centre: the thickest wound string is 6 mm.)
                    assert!(clear > 0.007, "keys {i} over {j}: {clear}");
                }
            }
        }
        assert!(crossings > 100, "{crossings}");
        // The plate over them all, and the scene whole.
        let scene = build(&m, None, &options());
        let highest = (21..=108u8).map(|k| bridge_z(&sb, k)).fold(0.0, f64::max);
        assert!(highest > 0.07 && highest < 0.1, "{highest}");
        let bar = scene.tris.iter().filter(|v| v.color == rgba(IRON, 1.0) && (v.pos[2] as f64) > highest + 0.012).count();
        assert!(bar > 0);
    }

    #[test]
    fn a_built_board_has_its_own_rear_lengths() {
        let m = built();
        let sb = m.soundboard.unwrap().resolved();
        let outline = board_outline(&sb);
        let hx = hammer_x(&m, &sb);
        let mut own = 0;
        for k in 21..=108u8 {
            let ((ax, ay), (bx, by), _) = string_ends(&m, &sb, hx, k);
            let l = (bx - ax).hypot(by - ay);
            let u = ((bx - ax) / l, (by - ay) / l);
            let (engine, drawn) = (sb.structure.back_length_at(k), hitch_length(&outline, (bx, by), u, rear_wanted(&sb, k, l)));
            // The engine's, or what room the case leaves.
            assert!(drawn <= engine + 1e-9 && drawn >= HITCH_MIN, "{k}: {drawn} for {engine}");
            own += (drawn == engine) as usize;
        }
        assert!(own > 70, "{own}");
    }

    #[test]
    fn faces_point_outwards() {
        let mut b = Builder::new(Place { vertical: false, top: 0.0 }, Layer::All);
        b.cuboid([0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0], Color32::WHITE);
        for t in b.s.tris.chunks(3) {
            let c = scale(add(add(t[0].pos, t[1].pos), t[2].pos), 1.0 / 3.0);
            assert!(dot(t[0].normal, sub(c, [0.5; 3])) > 0.0, "{t:?}");
        }
    }

    #[test]
    fn live_state_shows() {
        let m = PianoModel::default();
        let mut obs = Observation::default();
        obs.keys[60].down = true;
        obs.keys[60].damper = 0.0;
        let mut levels = [0.0; 128];
        levels[60] = 1.0;
        let field = vec![0.5; BOARD_GRID.0 * BOARD_GRID.1];
        let live = Live { obs: &obs, keys: &levels, board: Some(&field), time: 0.1 };
        let o = Options { key: 40, ..options() };
        let still = build(&m, None, &o);
        let playing = build(&m, Some(&live), &o);
        // The struck key's strings drawn in segments, its damper lifted.
        assert!(playing.lines.len() > still.lines.len());
        assert_ne!(playing.tris, still.tris);
    }
}

