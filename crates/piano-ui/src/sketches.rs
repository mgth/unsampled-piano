//! Sketches locating the parameters on the instrument, drawn from the model
//! (felt depth, pedal curve, strike point, mistuning… follow the settings).
//!
//! The action: one key seen from the side, the player on the left. On a
//! grand the key lifts the hammer onto the string from below and, at its
//! back end, the damper off the string; the sustain pedal lifts every
//! damper through the damper tray. On an upright (standing board) the
//! strings stand behind the action: the wippen's jack throws the hammer
//! against them horizontally, the dampers press on them below the strike
//! point, the soft pedal's hammer rail brings the hammers closer and the
//! sustain pedal's lift rail pulls every damper back. Not to scale.
//!
//! The string: one key's string from the agraffe to the bridge, with the
//! strike and damper points (to scale along the string), its unison seen
//! from above, its two directions of vibration, its decay, its partials,
//! and a keyboard choosing the key shown.
//!
//! The plan: the case from above, to scale (soundboard, bridges, every
//! key's string, frame, rim, keyboard choosing the key shown), and its
//! side section for the heights.
//!
//! The room: from above and from the side, to scale, with the piano, the
//! microphones, the first reflections and the piano's directivity; and the
//! reverberation's decay at low and high frequency.

use egui::{Align2, Color32, FontId, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Vec2};
use piano_core::params::PianoModel;
use piano_core::soundboard::KEY_LINE_X;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sketch {
    Action,
    String,
    Plan,
    Room,
    /// The circle of fifths ([`crate::wheel`]).
    Temperament,
}

/// A part of a sketch a parameter acts on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    /// The key press setting the hammer's speed.
    Key,
    /// The hammer's top felt.
    Felt,
    /// The hammer's hard core.
    Core,
    /// The damper on the string, and its motion.
    Damper,
    SustainPedal,
    /// The soft pedal: una corda shift (grand) or half blow (upright).
    UnaCorda,
    /// The practice mute an upright's middle pedal lowers.
    MuteFelt,
    /// How soon a key can strike again: a grand's repetition lever, an
    /// upright's jack that must reset.
    Repetition,
    /// The key bottoming on the keybed.
    KeybedKnock,
    /// The damper landing on the ringing string.
    DamperLanding,
    /// The action's own knock, heard directly.
    ActionKnock,
    /// The hammer's shank, flexible under the head.
    Shank,
    /// The damper tray knocking when the pedal moves.
    PedalKnock,
    // The string.
    /// Where the hammer strikes it.
    Strike,
    /// Where the damper rests on it.
    DamperPoint,
    /// Its length and tension.
    Length,
    /// The strings of the key and their mistuning.
    Unison,
    /// Its up-and-down and sideways vibration.
    Polarization,
    /// Its stretching at large amplitude.
    Nonlinear,
    /// Its own losses (decay).
    Losses,
    /// The bridge it drives.
    Bridge,
    /// Its partials (stiffness).
    Partials,
    /// The other strings answering through the bridge.
    Sympathy,
    /// A range of keys (the parameter is a key number).
    Keys,
    // The case.
    /// The soundboard as a whole.
    Board,
    /// Its treble side, and its bass side (impedance slopes).
    BoardTreble,
    BoardBass,
    /// The main bridge, and the bass bridge.
    Bridges,
    BassBridge,
    /// How far along the bridge a string's force spreads.
    BridgeSpread,
    /// The whole case moving as a block.
    Piston,
    /// The board's underside radiating in antiphase.
    BackFace,
    /// The losses of the bridge points near the board's edges.
    BoardEdge,
    /// The wall behind an upright's board.
    Wall,
    Frame,
    Agraffes,
    Rim,
    /// Heights, in the side section.
    FrameHeight,
    RimSection,
    // The room.
    /// Its size (walls, floor, ceiling).
    RoomSize,
    /// Where the piano stands in it.
    PianoPlace,
    /// Its absorption at low and at high frequency (decay curves).
    AbsorbLow,
    AbsorbHigh,
    /// The early reflections computed one by one.
    Reflections,
    // The temperament wheel.
    /// The fifths and thirds a temperament tunes.
    Fifths,
    /// The note it is laid on.
    Tonic,
    /// The reverberation's level.
    Tail,
    /// How much the piano beams towards the mics.
    Directivity,
}

/// The sketch locating a parameter and the parts it acts on, if any.
pub fn locate(path: &str) -> Option<(Sketch, &'static [Part])> {
    use Part::*;
    // The first match wins: specific paths before their prefixes.
    const STRING: &[(&str, &[Part])] = &[
        ("strike_ratio", &[Strike, Partials]),
        ("damper.position", &[DamperPoint]),
        ("damper.behind_strike", &[DamperPoint]),
        ("damper.width", &[DamperPoint]),
        ("damper.node_floor", &[DamperPoint]),
        ("damper.horizontal", &[DamperPoint, Polarization]),
        ("damper.highest_note", &[DamperPoint, Keys]),
        ("scale.break_note", &[Length, Keys]),
        ("scale.", &[Length]),
        ("unison.bichord_from", &[Unison, Keys]),
        ("unison.trichord_from", &[Unison, Keys]),
        ("unison.", &[Unison]),
        ("polarization.", &[Polarization]),
        ("nonlinear.max_note", &[Nonlinear, Keys]),
        ("nonlinear.", &[Nonlinear]),
        ("losses.wound_until", &[Losses, Keys]),
        ("losses.top_from", &[Losses, Keys]),
        ("losses.", &[Losses]),
        ("inharmonicity.", &[Partials]),
        ("bridge.", &[Bridge]),
        ("soundboard.bridge_mass", &[Bridge]),
        ("soundboard.series_stiffness", &[Bridge]),
        ("sympathetic.", &[Sympathy]),
    ];
    // Empty: on the Microphones page's own drawing.
    const PLAN: &[(&str, &[Part])] = &[
        ("soundboard.mic", &[]),
        ("soundboard.lid_", &[]),
        ("soundboard.impedance_slope_bass", &[BoardBass]),
        ("soundboard.impedance_slope", &[BoardTreble]),
        ("soundboard.treble_bridge", &[Bridges]),
        ("soundboard.bass_bridge", &[BassBridge]),
        ("soundboard.main_bridge_from", &[BassBridge]),
        ("soundboard.main_string_slant", &[Bridges]),
        ("soundboard.bridge_", &[Bridges, BridgeSpread]),
        ("soundboard.piston_", &[Piston]),
        ("soundboard.back_", &[BackFace]),
        ("soundboard.edge_", &[BoardEdge, Bridges]),
        ("soundboard.wall_", &[Wall]),
        ("soundboard.", &[Board]),
        ("frame.agraffe_x", &[Agraffes]),
        ("frame.height", &[FrameHeight]),
        ("frame.back_", &[FrameHeight]),
        ("frame.", &[Frame]),
        ("rim.height", &[RimSection]),
        ("rim.top", &[RimSection]),
        ("rim.thickness", &[Rim, RimSection]),
        ("rim.inner_gain", &[RimSection]),
        ("rim.", &[Rim]),
    ];
    const ROOM: &[(&str, &[Part])] = &[
        ("room.enabled", &[RoomSize]),
        ("room.size", &[RoomSize]),
        ("room.piano", &[PianoPlace]),
        ("room.absorption_low", &[AbsorbLow]),
        ("room.absorption_high", &[AbsorbHigh]),
        ("room.order", &[Reflections]),
        ("room.tail_gain", &[Tail]),
        ("room.directivity", &[Directivity]),
    ];
    const ACTION: &[(&str, &[Part])] = &[
        ("velocity.", &[Key]),
        ("hardness", &[Felt]),
        ("p_bass", &[Felt]),
        ("p_offset", &[Felt]),
        ("hysteresis", &[Felt]),
        ("contact_width_scale", &[Felt]),
        ("hammer_mass_scale", &[Felt]),
        ("core_", &[Core]),
        ("una_corda_", &[UnaCorda]),
        ("half_blow", &[UnaCorda]),
        ("mute_felt", &[MuteFelt]),
        ("repetition_", &[Repetition]),
        ("damper.pedal_", &[SustainPedal]),
        ("damper.", &[Damper]),
        ("noise.keybed", &[KeybedKnock]),
        ("noise.action", &[ActionKnock]),
        ("shank_", &[Shank]),
        ("noise.damper_", &[DamperLanding]),
        ("noise.pedal", &[PedalKnock]),
        ("noise.knock_time", &[KeybedKnock, PedalKnock]),
    ];
    const TEMPERAMENT: &[(&str, &[Part])] = &[("temperament.key", &[Tonic]), ("temperament.", &[Fifths])];
    let find = |table: &'static [(&str, &'static [Part])]| table.iter().find(|(p, _)| path.starts_with(p)).map(|(_, parts)| *parts);
    let found = find(STRING)
        .map(|parts| (Sketch::String, parts))
        .or_else(|| find(PLAN).map(|parts| (Sketch::Plan, parts)))
        .or_else(|| find(ROOM).map(|parts| (Sketch::Room, parts)))
        .or_else(|| find(ACTION).map(|parts| (Sketch::Action, parts)))
        .or_else(|| find(TEMPERAMENT).map(|parts| (Sketch::Temperament, parts)));
    found.filter(|(_, parts)| !parts.is_empty())
}

/// The parts of `sketch` that `path` acts on (empty when it acts elsewhere).
pub fn lit_in(sketch: Sketch, path: &str) -> &'static [Part] {
    match locate(path) {
        Some((s, parts)) if s == sketch => parts,
        _ => &[],
    }
}

/// Draws `sketch`; the string sketch shows `key`, which its keyboard sets.
pub fn draw(ui: &mut Ui, sketch: Sketch, model: &PianoModel, path: &str, key: &mut u8, width: f32) -> Response {
    let lit = lit_in(sketch, path);
    match sketch {
        Sketch::Action => action(ui, model, lit, path, width),
        Sketch::String => string(ui, model, lit, path, key, width),
        Sketch::Plan => plan(ui, model, lit, key, width),
        Sketch::Room => room(ui, model, lit, width),
        Sketch::Temperament => crate::wheel::draw(ui, model, lit, width),
    }
}

/// Design space of the action drawing.
const W: f32 = 440.0;
const H: f32 = 254.0;

const STRING_Y: f32 = 30.0;
const STRIKE_X: f32 = 150.0;
const DAMPER_X: f32 = 300.0;
const KEY_Y: f32 = 130.0;
const KEYBED_Y: f32 = 150.0;
const TRAY_Y: f32 = 116.0;
const ROD_X: f32 = 355.0;
const PEDAL_Y: f32 = 176.0;

pub(crate) const HIGHLIGHT: Color32 = Color32::from_rgb(239, 159, 39);
pub(crate) const FELT: Color32 = Color32::from_rgb(222, 212, 188);
const CORE: Color32 = Color32::from_rgb(140, 120, 100);
pub(crate) const WOOD: Color32 = Color32::from_rgb(186, 117, 23);
pub(crate) const BRIDGE: Color32 = Color32::from_rgb(205, 95, 50);

/// The action at `width`, `lit` parts highlighted; `path` picks the key
/// whose felt depth is drawn (bass, middle or treble).
pub fn action(ui: &mut Ui, model: &PianoModel, lit: &[Part], path: &str, width: f32) -> Response {
    let k = width / W;
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(W, H) * k, Sense::hover());
    if !ui.is_rect_visible(rect) {
        return resp;
    }
    let p = ui.painter_at(rect);
    let at = |x: f32, y: f32| rect.min + Vec2::new(x, y) * k;
    let r = |x0: f32, y0: f32, x1: f32, y1: f32| Rect::from_two_pos(at(x0, y0), at(x1, y1));
    let vis = ui.visuals();
    let (ink, weak) = (vis.text_color(), vis.weak_text_color());
    let font = FontId::proportional((10.0 * k).max(9.0));
    let lit = |part: Part| lit.contains(&part);
    let line = |w: f32, c: Color32| Stroke::new(w * k.max(0.6), c);
    let outline = |part: Part| if lit(part) { line(2.0, HIGHLIGHT) } else { line(1.0, weak) };
    let fill = |part: Part, base: Color32| if lit(part) { HIGHLIGHT.gamma_multiply(0.55) } else { base };
    let label = |x: f32, y: f32, align: Align2, text: &str, part: Option<Part>| {
        let c = if part.is_some_and(lit) { ink } else { weak };
        p.text(at(x, y), align, text, font.clone(), c);
    };

    let felt = felt_shade(model.hardness);
    // An upright when the board stands, a grand otherwise.
    if model.soundboard.is_some_and(|b| b.vertical) {
        // The string, standing behind the action.
        let sx = 376.0;
        let sy = 46.0;
        p.line_segment([at(sx, 4.0), at(sx, 160.0)], line(2.0, ink));
        label(sx + 4.0, 4.0, Align2::LEFT_TOP, "String", None);

        // Keybed, key, balance point, capstan.
        let (key_y, keybed_y) = (150.0, 168.0);
        p.rect_filled(r(10.0, keybed_y, W - 10.0, keybed_y + 10.0), 1.0, vis.widgets.noninteractive.bg_fill);
        p.rect_stroke(r(10.0, keybed_y, W - 10.0, keybed_y + 10.0), 1.0, line(1.0, weak), egui::StrokeKind::Inside);
        label(12.0, keybed_y + 12.0, Align2::LEFT_TOP, "Keybed", Some(Part::KeybedKnock));
        p.rect_filled(r(20.0, key_y, sx - 60.0, key_y + 12.0), 1.0, WOOD.gamma_multiply(0.5));
        p.rect_filled(r(20.0, key_y - 3.0, 75.0, key_y + 12.0), 1.0, fill(Part::Key, Color32::from_gray(235)));
        p.rect_stroke(r(20.0, key_y - 3.0, 75.0, key_y + 12.0), 1.0, outline(Part::Key), egui::StrokeKind::Inside);
        p.add(Shape::convex_polygon(vec![at(170.0, key_y + 12.0), at(178.0, keybed_y), at(162.0, keybed_y)], weak, Stroke::NONE));
        label(110.0, key_y - 2.0, Align2::LEFT_BOTTOM, "Key", Some(Part::Key));
        p.rect_filled(r(sx - 96.0, 142.0, sx - 90.0, key_y), 1.0, weak);
        arrow(&p, at(47.0, 118.0), at(47.0, key_y - 6.0), outline(Part::Key));
        label(56.0, 120.0, Align2::LEFT_TOP, "Velocity", Some(Part::Key));

        // Wippen on the capstan, its jack under the hammer butt.
        p.line_segment([at(sx - 130.0, 140.0), at(sx - 28.0, 140.0)], line(2.0, weak));
        p.circle_stroke(at(sx - 130.0, 140.0), 3.0 * k, line(1.0, weak));
        let jack = if lit(Part::Repetition) { line(2.5, HIGHLIGHT) } else { line(2.0, weak) };
        p.line_segment([at(sx - 50.0, 140.0), at(sx - 50.0, 118.0)], jack);
        if lit(Part::Repetition) {
            let first = if model.repetition_time > 0.0 { "No repetition lever: the jack must reset" } else { "Repetition at once" };
            label(sx - 60.0, 92.0, Align2::RIGHT_CENTER, first, Some(Part::Repetition));
            label(sx - 60.0, 104.0, Align2::RIGHT_CENTER, &repetition_text(model), Some(Part::Repetition));
        }

        // Hammer: butt, shank, molding, core and top felt facing the string.
        let (cx, rx, ry) = (sx - 14.0, 12.0, 14.0);
        p.circle_filled(at(sx - 44.0, 112.0), 7.0 * k, weak.gamma_multiply(0.6));
        p.line_segment([at(sx - 42.0, 106.0), at(cx - 16.0, sy + 12.0)], line(3.0, if lit(Part::Shank) { HIGHLIGHT } else { WOOD }));
        if lit(Part::Shank) {
            label(cx - 24.0, sy + 14.0, Align2::RIGHT_CENTER, &shank_text(model), Some(Part::Shank));
        }
        p.rect_filled(r(cx - 20.0, sy - 12.0, cx - 12.0, sy + 12.0), 1.0, WOOD);
        let (depth, key) = felt_depth(model, path);
        let t = (depth as f32 * 8.0).clamp(1.5, 10.0);
        p.add(ellipse(at(cx, sy), rx * k, ry * k, fill(Part::Felt, felt), outline(Part::Felt)));
        if depth > 0.0 {
            let (core_l, core_r) = (cx - rx + 1.0, cx + rx - t);
            p.add(ellipse(
                at((core_l + core_r) / 2.0, sy),
                (core_r - core_l) / 2.0 * k,
                (ry - 0.45 * t) * k,
                fill(Part::Core, CORE),
                outline(Part::Core),
            ));
        } else {
            // Graded felt: denser towards the molding, in rings.
            let left = cx - rx + 1.0;
            for (f, c) in graded_rings(felt, lit(Part::Core)) {
                let right = cx + rx - f * 2.0 * rx * 0.85;
                p.add(ellipse(at((left + right) / 2.0, sy), (right - left) / 2.0 * k, ry * (1.0 - 0.45 * f) * k, c, Stroke::NONE));
            }
        }
        arrow(&p, at(cx - 26.0, sy - 22.0), at(cx + 2.0, sy - 22.0), outline(Part::Key));
        label(cx - 24.0, sy, Align2::RIGHT_CENTER, "Hammer", Some(Part::Felt));
        if lit(Part::Key) {
            let v = &model.velocity;
            let text = format!("{:.2}–{:.1} m/s, curve {:.2}", v.vmin, v.vmax, v.curve);
            label(cx - 30.0, sy - 22.0, Align2::RIGHT_CENTER, &text, Some(Part::Key));
        }
        if lit(Part::Felt) || lit(Part::Core) {
            let text = felt_text(model, depth, key);
            label(cx - 24.0, sy + 14.0, Align2::RIGHT_CENTER, &text, Some(Part::Core));
        }

        // Hammer rail: the soft pedal brings it, and the hammers resting on
        // it, closer to the strings (half blow).
        let rail = r(cx - 34.0, 76.0, cx - 24.0, 84.0);
        p.rect_filled(rail, 1.0, fill(Part::UnaCorda, weak));
        if lit(Part::UnaCorda) {
            arrow(&p, at(cx - 40.0, 92.0), at(cx - 20.0, 92.0), line(1.5, HIGHLIGHT));
            label(cx - 38.0, 80.0, Align2::RIGHT_CENTER, "Hammer rail (soft pedal)", Some(Part::UnaCorda));
        }

        // Practice mute: a felt strip lowered between hammers and strings.
        if model.mute_felt > 0.0 || lit(Part::MuteFelt) {
            let t = (model.mute_felt as f32 * 1.2).clamp(1.5, 6.0);
            if model.mute_felt > 0.0 {
                p.rect_filled(r(sx - 1.0 - t, sy - 22.0, sx - 1.0, sy + 22.0), 1.0, fill(Part::MuteFelt, FELT.gamma_multiply(0.8)));
            }
            if lit(Part::MuteFelt) {
                let text = if model.mute_felt > 0.0 {
                    format!("Practice mute, {:.1} mm (middle pedal)", model.mute_felt)
                } else {
                    "Middle pedal: sostenuto (no mute)".to_string()
                };
                label(cx - 24.0, sy + 14.0, Align2::RIGHT_CENTER, &text, Some(Part::MuteFelt));
            }
        }

        // Damper below the strike point, its lever pushed back by the
        // wippen's spoon and by the sustain pedal's lift rail.
        let damper = r(sx - 10.0, 96.0, sx, 116.0);
        p.rect_filled(r(sx - 18.0, 98.0, sx - 10.0, 114.0), 1.0, WOOD);
        p.rect_filled(damper, 1.0, fill(Part::Damper, FELT));
        p.rect_stroke(damper, 1.0, outline(Part::Damper), egui::StrokeKind::Inside);
        p.line_segment([at(sx - 14.0, 114.0), at(sx - 14.0, 136.0)], line(1.5, weak));
        p.circle_stroke(at(sx - 14.0, 136.0), 3.0 * k, line(1.0, weak));
        p.line_segment([at(sx - 28.0, 140.0), at(sx - 16.0, 128.0)], line(1.5, weak));
        label(sx + 4.0, 84.0, Align2::LEFT_CENTER, "Damper", Some(Part::Damper));
        if lit(Part::Damper) {
            let d = &model.damper;
            let text = format!("damper falls in {:.0} ms, lifts in {:.0} ms", 1e3 * d.fall_time, 1e3 * d.lift_time);
            label(sx - 60.0, 100.0, Align2::RIGHT_CENTER, &text, Some(Part::Damper));
            arrow(&p, at(sx - 4.0, 122.0), at(sx - 24.0, 122.0), line(1.5, HIGHLIGHT));
        }
        let lift = r(sx - 22.0, 124.0, 402.0, 128.0);
        p.rect_filled(lift, 1.0, fill(Part::SustainPedal, weak));
        p.line_segment([at(400.0, 128.0), at(400.0, 184.0)], outline(Part::SustainPedal));
        let pedal = r(356.0, 184.0, 426.0, 190.0);
        p.rect_filled(pedal, 2.0, fill(Part::SustainPedal, weak));
        label(352.0, 187.0, Align2::RIGHT_CENTER, "Sustain pedal (lift rail)", Some(Part::SustainPedal));

        // Noises.
        knock(&p, at(36.0, keybed_y), 7.0 * k, lit(Part::KeybedKnock), weak);
        knock(&p, at(sx, 106.0), 7.0 * k, lit(Part::DamperLanding), weak);
        knock(&p, at(sx + 12.0, 126.0), 7.0 * k, lit(Part::PedalKnock), weak);
        knock(&p, at(sx - 58.0, 122.0), 7.0 * k, lit(Part::ActionKnock), weak);
        if lit(Part::ActionKnock) {
            label(sx - 60.0, 92.0, Align2::RIGHT_CENTER, "The action's knock, heard directly", Some(Part::ActionKnock));
        }
    } else {
        // Frame and string, from the agraffe towards the bridge.
        p.add(Shape::convex_polygon(vec![at(54.0, 18.0), at(66.0, 18.0), at(60.0, STRING_Y)], weak, Stroke::NONE));
        p.line_segment([at(60.0, STRING_Y), at(W - 4.0, STRING_Y)], line(2.0, ink));
        label(W - 4.0, STRING_Y + 3.0, Align2::RIGHT_TOP, "String, to the bridge", None);
        label(62.0, 16.0, Align2::CENTER_BOTTOM, "Agraffe", None);

        // Keybed, key, balance point.
        p.rect_filled(r(10.0, KEYBED_Y, W - 10.0, KEYBED_Y + 10.0), 1.0, vis.widgets.noninteractive.bg_fill);
        p.rect_stroke(r(10.0, KEYBED_Y, W - 10.0, KEYBED_Y + 10.0), 1.0, line(1.0, weak), egui::StrokeKind::Inside);
        label(12.0, KEYBED_Y + 12.0, Align2::LEFT_TOP, "Keybed", Some(Part::KeybedKnock));
        p.rect_filled(r(20.0, KEY_Y, 330.0, KEY_Y + 12.0), 1.0, WOOD.gamma_multiply(0.5));
        p.rect_filled(r(20.0, KEY_Y - 3.0, 75.0, KEY_Y + 12.0), 1.0, fill(Part::Key, Color32::from_gray(235)));
        p.rect_stroke(r(20.0, KEY_Y - 3.0, 75.0, KEY_Y + 12.0), 1.0, outline(Part::Key), egui::StrokeKind::Inside);
        p.add(Shape::convex_polygon(vec![at(185.0, KEY_Y + 12.0), at(193.0, KEYBED_Y), at(177.0, KEYBED_Y)], weak, Stroke::NONE));
        label(110.0, KEY_Y - 2.0, Align2::LEFT_BOTTOM, "Key", Some(Part::Key));
        // The finger, and the hammer's flight it sets.
        arrow(&p, at(47.0, 98.0), at(47.0, KEY_Y - 6.0), outline(Part::Key));
        label(56.0, 100.0, Align2::LEFT_TOP, "Velocity", Some(Part::Key));
        arrow(&p, at(STRIKE_X - 24.0, 84.0), at(STRIKE_X - 24.0, 46.0), outline(Part::Key));
        if lit(Part::Key) {
            let v = &model.velocity;
            let text = format!("{:.2}–{:.1} m/s", v.vmin, v.vmax);
            label(STRIKE_X - 28.0, 60.0, Align2::RIGHT_CENTER, &text, Some(Part::Key));
            label(STRIKE_X - 28.0, 72.0, Align2::RIGHT_CENTER, &format!("curve {:.2}", v.curve), Some(Part::Key));
        }

        // Hammer: shank from its flange, wooden molding, core, top felt (its
        // depth exaggerated, from the model).
        p.line_segment([at(STRIKE_X, 80.0), at(255.0, 88.0)], line(3.0, if lit(Part::Shank) { HIGHLIGHT } else { WOOD }));
        if lit(Part::Shank) {
            label(STRIKE_X + 20.0, 60.0, Align2::LEFT_CENTER, &shank_text(model), Some(Part::Shank));
        }
        p.circle_stroke(at(255.0, 88.0), 3.5 * k, line(1.0, weak));
        p.line_segment([at(212.0, 85.0), at(212.0, KEY_Y)], line(2.0, weak));
        // The repetition lever (double escapement) holds the hammer halfway
        // down, ready to be thrown again before the key is fully up.
        let lever = if lit(Part::Repetition) { line(2.5, HIGHLIGHT) } else { line(1.5, weak) };
        p.line_segment([at(194.0, 100.0), at(232.0, 92.0)], lever);
        p.circle_stroke(at(194.0, 100.0), 2.5 * k, line(1.0, weak));
        if lit(Part::Repetition) {
            let text = if model.repetition_time > 0.0 { "Repetition lever (limited)" } else { "Repetition lever" };
            label(190.0, 100.0, Align2::RIGHT_CENTER, text, Some(Part::Repetition));
        }
        let (depth, key) = felt_depth(model, path);
        let t = (depth as f32 * 8.0).clamp(1.5, 10.0);
        let (top, bottom, rx) = (STRING_Y + 2.0, 76.0, 15.0);
        p.rect_filled(r(STRIKE_X - 11.0, 72.0, STRIKE_X + 11.0, 84.0), 1.0, WOOD);
        p.add(ellipse(at(STRIKE_X, (top + bottom) / 2.0), rx * k, (bottom - top) / 2.0 * k, fill(Part::Felt, felt), outline(Part::Felt)));
        if depth > 0.0 {
            let core_top = top + t;
            p.add(ellipse(
                at(STRIKE_X, (core_top + bottom) / 2.0),
                (rx - 0.45 * t) * k,
                (bottom - core_top) / 2.0 * k,
                fill(Part::Core, CORE),
                outline(Part::Core),
            ));
        } else {
            // Graded felt: denser towards the molding, in rings.
            for (f, c) in graded_rings(felt, lit(Part::Core)) {
                let ring_top = top + f * (bottom - top) * 0.85;
                p.add(ellipse(at(STRIKE_X, (ring_top + bottom) / 2.0), rx * (1.0 - 0.5 * f) * k, (bottom - ring_top) / 2.0 * k, c, Stroke::NONE));
            }
        }
        label(STRIKE_X + 20.0, 46.0, Align2::LEFT_CENTER, "Hammer", Some(Part::Felt));
        if lit(Part::Felt) || lit(Part::Core) {
            let text = felt_text(model, depth, key);
            label(STRIKE_X + 20.0, 60.0, Align2::LEFT_CENTER, &text, Some(Part::Core));
        }

        // Practice mute: a felt strip between hammers and strings while the
        // middle pedal is down (drawn lowered when it exists).
        if model.mute_felt > 0.0 || lit(Part::MuteFelt) {
            let t = (model.mute_felt as f32 * 1.2).clamp(1.5, 6.0);
            if model.mute_felt > 0.0 {
                p.rect_filled(r(STRIKE_X - 30.0, STRING_Y + 0.5, STRIKE_X + 30.0, STRING_Y + 0.5 + t), 1.0, fill(Part::MuteFelt, FELT.gamma_multiply(0.8)));
            }
            if lit(Part::MuteFelt) {
                let text = if model.mute_felt > 0.0 {
                    format!("Practice mute, {:.1} mm (middle pedal)", model.mute_felt)
                } else {
                    "Middle pedal: sostenuto (no mute)".to_string()
                };
                label(STRIKE_X + 20.0, 60.0, Align2::LEFT_CENTER, &text, Some(Part::MuteFelt));
            }
        }

        // Damper: felt on the string, wire, lever lifted by the back of the key,
        // tray lifted by the pedal rod.
        let damper = r(DAMPER_X - 11.0, 12.0, DAMPER_X + 11.0, STRING_Y);
        p.rect_filled(r(DAMPER_X - 9.0, 6.0, DAMPER_X + 9.0, 12.0), 1.0, WOOD);
        p.rect_filled(damper, 1.0, fill(Part::Damper, FELT));
        p.rect_stroke(damper, 1.0, outline(Part::Damper), egui::StrokeKind::Inside);
        p.line_segment([at(DAMPER_X, STRING_Y), at(DAMPER_X, 112.0)], line(1.0, weak));
        p.line_segment([at(268.0, 112.0), at(345.0, 112.0)], line(2.0, weak));
        p.circle_stroke(at(345.0, 112.0), 3.0 * k, line(1.0, weak));
        p.line_segment([at(320.0, KEY_Y), at(320.0, 114.0)], line(1.5, weak));
        label(DAMPER_X + 14.0, 20.0, Align2::LEFT_CENTER, "Damper", Some(Part::Damper));
        if lit(Part::Damper) {
            let d = &model.damper;
            let text = format!("falls in {:.0} ms, lifts in {:.0} ms", 1e3 * d.fall_time, 1e3 * d.lift_time);
            label(DAMPER_X + 14.0, 44.0, Align2::LEFT_CENTER, &text, Some(Part::Damper));
            arrow(&p, at(DAMPER_X - 18.0, 2.0), at(DAMPER_X - 18.0, 24.0), line(1.5, HIGHLIGHT));
        }
        let tray = r(262.0, TRAY_Y, ROD_X + 4.0, TRAY_Y + 4.0);
        p.rect_filled(tray, 1.0, fill(Part::SustainPedal, weak));
        p.line_segment([at(ROD_X, TRAY_Y + 4.0), at(ROD_X, PEDAL_Y)], outline(Part::SustainPedal));
        let pedal = r(ROD_X - 40.0, PEDAL_Y, ROD_X + 30.0, PEDAL_Y + 6.0);
        p.rect_filled(pedal, 2.0, fill(Part::SustainPedal, weak));
        label(ROD_X - 44.0, PEDAL_Y + 3.0, Align2::RIGHT_CENTER, "Sustain pedal", Some(Part::SustainPedal));
        label(ROD_X + 8.0, TRAY_Y - 6.0, Align2::LEFT_CENTER, "Damper tray", Some(Part::SustainPedal));

        // Noises.
        knock(&p, at(36.0, KEYBED_Y), 7.0 * k, lit(Part::KeybedKnock), weak);
        knock(&p, at(DAMPER_X, STRING_Y), 7.0 * k, lit(Part::DamperLanding), weak);
        knock(&p, at(290.0, TRAY_Y + 2.0), 7.0 * k, lit(Part::PedalKnock), weak);
        knock(&p, at(240.0, 100.0), 7.0 * k, lit(Part::ActionKnock), weak);
        if lit(Part::ActionKnock) {
            label(STRIKE_X + 20.0, 60.0, Align2::LEFT_CENTER, "Action knock (direct)", Some(Part::ActionKnock));
        }

    }

    // Below: the soft pedal, and the sustain pedal's travel.
    let y0 = 196.0;
    soft_pedal(&p, &at, model, lit(Part::UnaCorda), felt, ink, weak, &font, k);
    pedal_travel(&p, r(240.0, 213.0, W - 14.0, 237.0), model, lit(Part::SustainPedal), ink, weak, &font);
    label(240.0, y0, Align2::LEFT_TOP, "Damper contact by pedal travel", Some(Part::SustainPedal));
    resp
}

/// Design space of the string drawing.
const SH: f32 = 272.0;
const AGRAFFE_X: f32 = 24.0;
const BRIDGE_X: f32 = 404.0;
const KEYS_Y: f32 = 228.0;

/// One key's string, its unison, vibration, decay and partials, and the
/// keyboard choosing the key (clicked or dragged).
pub fn string(ui: &mut Ui, model: &PianoModel, lit: &[Part], path: &str, key: &mut u8, width: f32) -> Response {
    let k = width / W;
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(W, SH) * k, Sense::hover());
    let at = |x: f32, y: f32| rect.min + Vec2::new(x, y) * k;
    let r = |x0: f32, y0: f32, x1: f32, y1: f32| Rect::from_two_pos(at(x0, y0), at(x1, y1));
    // The keyboard: 52 white keys, black keys over their upper part.
    let ww = (W - 20.0) / 52.0;
    let keys = r(10.0, KEYS_Y, W - 10.0, KEYS_Y + KEYS_H);
    let pick = ui.interact(keys, ui.scope_id().with("piano-string-key"), Sense::click_and_drag());
    if let Some(pos) = pick.interact_pointer_pos().filter(|_| pick.clicked() || pick.dragged()) {
        let u = (pos.x - keys.left()) / (ww * k);
        let upper = (pos.y - keys.top()) < 0.6 * keys.height();
        *key = key_at(u, upper);
    }
    pick.on_hover_text("Click or drag along the keyboard to show another key's string");
    if !ui.is_rect_visible(rect) {
        return resp;
    }
    let n = model.note(*key);
    let p = ui.painter_at(rect);
    let vis = ui.visuals();
    let (ink, weak) = (vis.text_color(), vis.weak_text_color());
    let font = FontId::proportional((10.0 * k).max(9.0));
    let lit = |part: Part| lit.contains(&part);
    let line = |w: f32, c: Color32| Stroke::new(w * k.max(0.6), c);
    let color = |part: Part, base: Color32| if lit(part) { HIGHLIGHT } else { base };
    let label = |x: f32, y: f32, align: Align2, text: &str, part: Option<Part>| {
        let c = if part.is_some_and(lit) { ink } else { weak };
        p.text(at(x, y), align, text, font.clone(), c);
    };
    let name = note_name(*key);

    // Side view: agraffe, string and its vibration, hammer, damper, bridge.
    let sy = 40.0;
    let len_px = BRIDGE_X - AGRAFFE_X;
    label(4.0, 2.0, Align2::LEFT_TOP, &format!("The {name} string, from the agraffe to the bridge (to scale along it)"), None);
    p.add(Shape::convex_polygon(vec![at(AGRAFFE_X - 6.0, sy - 12.0), at(AGRAFFE_X + 6.0, sy - 12.0), at(AGRAFFE_X, sy)], weak, Stroke::NONE));
    let envelope = |sign: f32| -> Vec<Pos2> {
        (0..=40).map(|i| i as f32 / 40.0).map(|t| at(AGRAFFE_X + t * len_px, sy + sign * 7.0 * (t * std::f32::consts::PI).sin())).collect()
    };
    p.extend(Shape::dashed_line(&envelope(-1.0), line(0.8, weak.gamma_multiply(0.6)), 3.0, 3.0));
    p.extend(Shape::dashed_line(&envelope(1.0), line(0.8, weak.gamma_multiply(0.6)), 3.0, 3.0));
    p.line_segment([at(AGRAFFE_X, sy), at(BRIDGE_X, sy)], line(2.0, color(Part::Length, ink)));
    let bridge = r(BRIDGE_X - 6.0, sy, BRIDGE_X + 8.0, sy + 13.0);
    p.rect_filled(bridge, 1.0, color(Part::Bridge, WOOD));
    p.rect_filled(r(330.0, sy + 13.0, W - 2.0, sy + 17.0), 0.0, WOOD.gamma_multiply(0.5));
    label(BRIDGE_X + 1.0, sy + 19.0, Align2::CENTER_TOP, "Bridge", Some(Part::Bridge));
    let xs = AGRAFFE_X + n.strike_pos as f32 * len_px;
    p.add(ellipse(at(xs, sy + 8.0), 4.0 * k, 7.0 * k, if lit(Part::Strike) { HIGHLIGHT } else { FELT }, line(1.0, weak)));
    p.line_segment([at(xs, sy + 15.0), at(xs, sy + 24.0)], line(2.0, WOOD));
    label(xs + 8.0, sy + 18.0, Align2::LEFT_CENTER, &format!("Hammer at {:.3} L", n.strike_pos), Some(Part::Strike));
    let d = &model.damper;
    if *key <= d.highest_note {
        let xd = AGRAFFE_X + d.position as f32 * len_px;
        let w = (d.width / n.length) as f32 * len_px;
        let w = w.max(3.0);
        let pad = r(xd - w / 2.0, sy - 11.0, xd + w / 2.0, sy);
        p.rect_filled(pad, 1.0, if lit(Part::DamperPoint) { HIGHLIGHT } else { FELT });
        p.line_segment([at(xd, sy - 11.0), at(xd, sy - 22.0)], line(1.0, weak));
        label(xd + w / 2.0 + 4.0, sy - 17.0, Align2::LEFT_CENTER, &format!("Damper at {:.2} L", d.position), Some(Part::DamperPoint));
    } else {
        label(AGRAFFE_X + 60.0, sy - 17.0, Align2::LEFT_CENTER, "No damper: this string rings freely", Some(Part::DamperPoint));
    }
    if lit(Part::Nonlinear) {
        let y = sy - 6.0;
        arrow(&p, at(250.0, y), at(210.0, y), line(1.5, HIGHLIGHT));
        arrow(&p, at(270.0, y), at(310.0, y), line(1.5, HIGHLIGHT));
        label(260.0, y - 5.0, Align2::CENTER_BOTTOM, "stretches when struck hard", Some(Part::Nonlinear));
    }
    // Length and tension.
    let dy = sy + 34.0;
    p.line_segment([at(AGRAFFE_X, dy), at(BRIDGE_X, dy)], line(1.0, color(Part::Length, weak)));
    for x in [AGRAFFE_X, BRIDGE_X] {
        p.line_segment([at(x, dy - 4.0), at(x, dy + 4.0)], line(1.0, color(Part::Length, weak)));
    }
    let dims = format!("L = {:.2} m, tension {:.0} N, {:.1} g/m", n.length, n.tension, 1e3 * n.mu);
    label((AGRAFFE_X + BRIDGE_X) / 2.0, dy + 3.0, Align2::CENTER_TOP, &dims, Some(Part::Length));

    // The unison from above: one line per string, as thick as its share of
    // the blow, its mistuning written beside it.
    let uy = 100.0;
    label(4.0, uy, Align2::LEFT_TOP, &format!("{} string{} from above", n.strings, if n.strings > 1 { "s" } else { "" }), Some(Part::Unison));
    let hits = model.unison.hits(*key);
    let mid = uy + 36.0;
    for (j, (hit, detune)) in hits.iter().zip(n.detune).take(n.strings).enumerate() {
        let y = mid + (j as f32 - (n.strings as f32 - 1.0) / 2.0) * 11.0;
        let w = (1.6 * *hit as f32).clamp(0.5, 3.0);
        p.line_segment([at(16.0, y), at(190.0, y)], line(w, color(Part::Unison, ink)));
        if n.strings > 1 {
            label(196.0, y, Align2::LEFT_CENTER, &format!("{detune:+.2} c"), Some(Part::Unison));
        }
    }
    let hx = 16.0 + n.strike_pos as f32 * 174.0;
    p.rect_stroke(r(hx - 3.0, mid - 17.0, hx + 3.0, mid + 17.0), 1.0, line(1.0, weak), egui::StrokeKind::Middle);
    if lit(Part::Sympathy) {
        for y in [mid + 22.0, mid + 27.0] {
            p.extend(Shape::dashed_line(&[at(16.0, y), at(190.0, y)], line(1.0, HIGHLIGHT), 4.0, 3.0));
        }
        label(16.0, mid + 31.0, Align2::LEFT_TOP, "the other strings answer through the bridge", Some(Part::Sympathy));
    }

    // Vibration: up-and-down, and sideways (weaker coupling, longer ring).
    let (vx, vy) = (282.0, mid);
    label(vx, uy, Align2::CENTER_TOP, "Vibration", Some(Part::Polarization));
    p.circle_filled(at(vx, vy), 3.5 * k, ink);
    let pc = color(Part::Polarization, ink);
    arrow(&p, at(vx, vy - 6.0), at(vx, vy - 24.0), line(1.5, pc));
    arrow(&p, at(vx, vy + 6.0), at(vx, vy + 24.0), line(1.5, pc));
    if model.polarization.enabled {
        let pw = color(Part::Polarization, weak);
        arrow(&p, at(vx + 6.0, vy), at(vx + 20.0, vy), line(1.0, pw));
        arrow(&p, at(vx - 6.0, vy), at(vx - 20.0, vy), line(1.0, pw));
    }

    // Decay of the fundamental from the string's own losses.
    let t60 = 3.0 * std::f64::consts::LN_10 / model.losses.sigma(*key, n.f0);
    let (bx0, bx1, by0, by1) = (320.0, W - 4.0, uy + 14.0, uy + 56.0);
    label(bx0, uy, Align2::LEFT_TOP, "Decay (string alone)", Some(Part::Losses));
    let dc = color(Part::Losses, ink);
    let wave: Vec<Pos2> = (0..=240)
        .map(|i| i as f32 / 240.0)
        .map(|t| {
            let env = (-3.0 * std::f32::consts::LN_10 * t).exp(); // −60 dB over the box
            let y = (by0 + by1) / 2.0 - env * (t * 18.0 * std::f32::consts::TAU).sin() * (by1 - by0) / 2.0;
            at(bx0 + t * (bx1 - bx0), y)
        })
        .collect();
    p.add(Shape::line(wave, line(0.8, dc.gamma_multiply(0.8))));
    label(bx0, by1 + 4.0, Align2::LEFT_TOP, &format!("60 dB down in {}", seconds(t60)), Some(Part::Losses));

    // Partials: whole multiples of the fundamental (faint) and the stiff
    // string's, as tall as the strike excites them.
    let py = 184.0;
    label(4.0, py, Align2::LEFT_TOP, "Partials 1–16 (faint: whole multiples; height: how hard struck)", Some(Part::Partials));
    let base = py + 36.0;
    let px = |ratio: f64| 16.0 + (ratio / 17.0) as f32 * (W - 26.0);
    p.line_segment([at(12.0, base), at(W - 8.0, base)], line(1.0, weak));
    for h in 1..=16u32 {
        let k_ = h as f64;
        p.line_segment([at(px(k_), base), at(px(k_), base - 20.0)], line(1.0, weak.gamma_multiply(0.4)));
        let ratio = k_ * (1.0 + n.b * k_ * k_).sqrt();
        let amp = (k_ * std::f64::consts::PI * n.strike_pos).sin().abs() as f32;
        if ratio < 17.0 {
            p.line_segment([at(px(ratio), base), at(px(ratio), base - 3.0 - 17.0 * amp)], line(2.0, color(Part::Partials, ink)));
        }
    }
    label(W - 4.0, py, Align2::RIGHT_TOP, &format!("B = {:.1e}", n.b), Some(Part::Partials));

    // Keyboard: the key shown, the lit range of a key-number parameter.
    let ky = KEYS_Y;
    let range = lit(Part::Keys).then(|| key_range(model, path)).flatten();
    let in_range = |m: u8| range.is_some_and(|(lo, hi)| (lo..=hi).contains(&m));
    let key_fill = |m: u8, black: bool| {
        if m == *key {
            vis.selection.bg_fill
        } else if in_range(m) {
            if black { HIGHLIGHT.gamma_multiply(0.7) } else { HIGHLIGHT.gamma_multiply(0.45) }
        } else if black {
            Color32::from_gray(35)
        } else {
            Color32::from_gray(215)
        }
    };
    for black in [false, true] {
        for m in (21..=108u8).filter(|&m| is_black(m) == black) {
            let (u, w) = key_span(m);
            let (x0, x1) = (10.0 + u * ww, 10.0 + (u + w) * ww);
            let rect = r(x0, ky, x1, ky + if black { 0.6 * KEYS_H } else { KEYS_H });
            p.rect_filled(rect, 1.0, key_fill(m, black));
            p.rect_stroke(rect, 1.0, Stroke::new(0.5, Color32::from_gray(90)), egui::StrokeKind::Inside);
        }
    }
    if let Some((lo, hi)) = range {
        label(W - 10.0, ky + KEYS_H + 3.0, Align2::RIGHT_TOP, &format!("{}–{}", note_name(lo), note_name(hi)), Some(Part::Keys));
    }
    let (u, w) = key_span(*key);
    let kx = 10.0 + (u + w / 2.0) * ww;
    let ky = ky + KEYS_H - 18.0;
    let text = format!("{name}, {} string{}", n.strings, if n.strings > 1 { "s" } else { "" });
    let align = if kx < W / 2.0 { Align2::LEFT_TOP } else { Align2::RIGHT_TOP };
    p.text(at(kx + if kx < W / 2.0 { 6.0 } else { -6.0 }, ky + 26.0), align, text, font.clone(), ink);
    resp
}

const KEYS_H: f32 = 22.0;

pub(crate) fn is_black(m: u8) -> bool {
    matches!(m % 12, 1 | 3 | 6 | 8 | 10)
}

/// Left edge and width of a key, in white-key widths from A0.
pub(crate) fn key_span(m: u8) -> (f32, f32) {
    let whites = (21..m).filter(|&n| !is_black(n)).count() as f32;
    if is_black(m) { (whites - 0.3, 0.6) } else { (whites, 1.0) }
}

/// The key at `u` white-key widths from A0 (`upper`: where black keys are).
fn key_at(u: f32, upper: bool) -> u8 {
    let hit = |m: &u8| {
        let (x, w) = key_span(*m);
        (x..x + w).contains(&u)
    };
    let found = if upper { (21..=108u8).filter(|&m| is_black(m)).find(hit) } else { None };
    found.or_else(|| (21..=108u8).filter(|&m| !is_black(m)).find(hit)).unwrap_or(if u < 0.0 { 21 } else { 108 })
}

/// The case from above, to scale: keyboard at the bottom, treble on the
/// right, as the pianist sees it; beside it, the side section (keyboard on
/// the left, heights up) at its own scale. Clicking the keyboard picks the
/// key whose string and bridge point are shown.
pub fn plan(ui: &mut Ui, model: &PianoModel, lit: &[Part], key: &mut u8, width: f32) -> Response {
    let Some(sb) = model.soundboard else {
        return ui.label("This model has no soundboard: there is no case to draw.");
    };
    let (f, rim) = (&model.frame, &model.rim);
    let t = if rim.enabled { rim.thickness } else { 0.0 };
    let fx = if f.enabled { f.offset_x } else { -0.2 };
    let front = fx - 0.15;
    // Plan extent (m): y across, x along; section extent: z.
    let (y0, y1) = (-t - 0.04, sb.b + t + 0.04);
    let (x0, x1) = (front - 0.04, sb.a + t + 0.04);
    let z0 = (-sb.back_depth - sb.wall_distance).min(rim.top - rim.height) - 0.05;
    let z1 = f.height.max(rim.top) + 0.12;
    // Design units per metre: the plan 250 wide at most, 330 tall; the
    // section fills the rest of the width.
    let s = (250.0 / (y1 - y0) as f32).min(330.0 / (x1 - x0) as f32);
    let sec_x = 8.0 + (y1 - y0) as f32 * s + 44.0;
    let s2 = (W - 4.0 - sec_x) / (x1 - x0) as f32;
    let top = 18.0;
    let sec_top = top + 30.0;
    // The section's heights, stretched to be read.
    const STRETCH: f32 = 2.5;
    let h = top + (x1 - x0) as f32 * s + 22.0;
    let k = width / W;
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(W, h) * k, Sense::hover());
    let at = |x: f32, y: f32| rect.min + Vec2::new(x, y) * k;
    // Plan: (x along, y across) in metres → screen.
    let pl = |x: f64, y: f64| at(8.0 + (y1 - y) as f32 * s, top + (x1 - x) as f32 * s);
    let sec = |x: f64, z: f64| at(sec_x + (x - x0) as f32 * s2, sec_top + (z1 - z) as f32 * s2 * STRETCH);
    let bridge_y = |m: u8| sb.key_across(m);

    // The keyboard picks the key whose strings are nearest across.
    let keys = Rect::from_two_pos(pl(fx, 0.0), pl(front, sb.b));
    let pick = ui.interact(keys, ui.scope_id().with("piano-plan-key"), Sense::click_and_drag());
    if let Some(pos) = pick.interact_pointer_pos().filter(|_| pick.clicked() || pick.dragged()) {
        let y = y1 - ((pos.x - rect.left()) / k - 8.0) as f64 / s as f64;
        *key = (21..=108u8).min_by(|&a, &b| (bridge_y(a) - y).abs().total_cmp(&(bridge_y(b) - y).abs())).unwrap_or(*key);
    }
    pick.on_hover_text("Click or drag along the keyboard to pick a key");
    if !ui.is_rect_visible(rect) {
        return resp;
    }
    let p = ui.painter_at(rect);
    let vis = ui.visuals();
    let (ink, weak) = (vis.text_color(), vis.weak_text_color());
    let font = FontId::proportional((10.0 * k).max(9.0));
    let lit = |part: Part| lit.contains(&part);
    let line = |w: f32, c: Color32| Stroke::new(w * k.max(0.6), c);
    let label = |pos: Pos2, align: Align2, text: &str, part: Option<Part>| {
        let c = if part.is_some_and(lit) { ink } else { weak };
        p.text(pos, align, text, font.clone(), c);
    };
    let rect_m = |a: (f64, f64), b: (f64, f64)| Rect::from_two_pos(pl(a.0, a.1), pl(b.0, b.1));
    p.text(at(8.0, top - 14.0), Align2::LEFT_TOP, "The case from above (to scale)", font.clone(), weak);

    // Rim and case; the piston mode moves it all.
    // The board within its outline (the full rectangle unless one is set),
    // the rim's wall around it.
    let outline = crate::model::scene::board_outline(&sb);
    if rim.enabled {
        let wall: Vec<Pos2> = outline.iter().map(|&(x, y)| pl(x, y)).collect();
        let c = if lit(Part::Rim) { HIGHLIGHT.gamma_multiply(0.6) } else { weak.gamma_multiply(0.35) };
        p.add(Shape::closed_line(wall, Stroke::new(2.0 * t as f32 * s * k, c)));
    }
    let board = rect_m((0.0, 0.0), (sb.a, sb.b));
    let fill = if lit(Part::Board) { HIGHLIGHT.gamma_multiply(0.45) } else { WOOD.gamma_multiply(0.3) };
    let mut mesh = egui::Mesh::default();
    for &(x, y) in &outline {
        mesh.colored_vertex(pl(x, y), fill);
    }
    for [a, b, c] in crate::model::scene::triangulate(&outline) {
        mesh.add_triangle(a as u32, b as u32, c as u32);
    }
    p.add(Shape::mesh(mesh));
    if lit(Part::BoardEdge) {
        // A band as deep as the losses reach (e^(-d/edge_depth)).
        let d = (sb.edge_depth as f32 * s * k).clamp(2.0, board.width().min(board.height()) / 2.0);
        p.rect_stroke(board.shrink(d / 2.0), 0.0, Stroke::new(d, HIGHLIGHT.gamma_multiply(0.35)), egui::StrokeKind::Middle);
        let text = format!("losses within {:.0} cm of the edges", sb.edge_depth * 100.0);
        label(board.center_bottom() + Vec2::new(0.0, -d - 4.0 * k), Align2::CENTER_BOTTOM, &text, Some(Part::BoardEdge));
    }
    if lit(Part::Piston) {
        p.rect_stroke(rect_m((-t, -t), (sb.a + t, sb.b + t)), 3.0 * k, line(2.5, HIGHLIGHT), egui::StrokeKind::Outside);
        arrow(&p, pl(sb.a * 0.5, sb.b * 0.5) + Vec2::new(0.0, 10.0 * k), pl(sb.a * 0.5, sb.b * 0.5) - Vec2::new(0.0, 10.0 * k), line(1.5, HIGHLIGHT));
        label(pl(sb.a * 0.5, sb.b * 0.5) + Vec2::new(6.0 * k, 0.0), Align2::LEFT_CENTER, "the whole case moves", Some(Part::Piston));
    }
    // The board's sides as the strings drive it (impedance slopes).
    let along = |from: u8, to: u8| -> Vec<Pos2> { (from..=to).map(|m| sb.bridge_point(m)).map(|(x, y)| pl(x, y)).collect() };
    if lit(Part::BoardTreble) {
        p.add(Shape::line(along(60, 108), line(9.0, HIGHLIGHT.gamma_multiply(0.5))));
        label(pl(sb.bridge_point(96).0, -t) + Vec2::new(4.0 * k, 0.0), Align2::LEFT_CENTER, "stiffer", Some(Part::BoardTreble));
    }
    if lit(Part::BoardBass) {
        p.add(Shape::line(along(sb.main_bridge_from, 60), line(9.0, HIGHLIGHT.gamma_multiply(0.5))));
        p.add(Shape::line(along(21, sb.main_bridge_from - 1), line(9.0, HIGHLIGHT.gamma_multiply(0.5))));
    }

    // Every key's string, from its agraffe to its bridge point, on the line
    // from where it crosses the keys' line (the main strings slant).
    let agraffe = fx + if f.enabled { f.agraffe_x } else { 0.25 };
    for m in 21..=108u8 {
        let (bx, by) = sb.bridge_point(m);
        let ky = sb.key_across(m);
        let ay = ky + (by - ky) * (agraffe - KEY_LINE_X) / (bx - KEY_LINE_X);
        let c = if m == *key { ink } else { weak.gamma_multiply(0.35) };
        p.line_segment([pl(agraffe, ay), pl(bx, by)], line(if m == *key { 1.5 } else { 0.5 }, c));
    }

    // Frame and agraffes.
    if f.enabled {
        // Its outline as the engine has it (the case's, over the pin block).
        let mut frame: Vec<Pos2> = crate::model::scene::frame_outline(model, &sb).iter().map(|&(x, y)| pl(x, y)).collect();
        let corner = frame.iter().fold(Pos2::new(f32::INFINITY, f32::INFINITY), |c, q| Pos2::new(c.x.min(q.x), c.y.min(q.y)));
        frame.push(frame[0]);
        let fs = if lit(Part::Frame) { line(2.0, HIGHLIGHT) } else { line(1.0, weak) };
        p.extend(Shape::dashed_line(&frame, fs, 4.0, 3.0));
        label(corner + Vec2::new(3.0, 2.0) * k, Align2::LEFT_TOP, "Frame", Some(Part::Frame));
    }
    let ag = if lit(Part::Agraffes) { line(2.5, HIGHLIGHT) } else { line(1.0, ink) };
    p.line_segment([pl(agraffe, 0.0), pl(agraffe, sb.b)], ag);
    label(pl(agraffe, 0.0) + Vec2::new(3.0 * k, 0.0), Align2::LEFT_CENTER, "Agraffes", Some(Part::Agraffes));

    // Bridges.
    let bridge = |main: bool, on: bool| {
        let pts: Vec<Pos2> = sb.bridge_line(main).into_iter().map(|(x, y)| pl(x, y)).collect();
        p.add(Shape::line(pts, line(3.0, if on { HIGHLIGHT } else { BRIDGE })));
    };
    bridge(true, lit(Part::Bridges));
    bridge(false, lit(Part::BassBridge) || lit(Part::Bridges));
    let (mx, my) = (sb.treble_bridge[2].0 * sb.a, sb.treble_bridge[2].1 * sb.b);
    label(pl(mx, my) + Vec2::new(8.0 * k, 0.0), Align2::LEFT_CENTER, "Bridge", Some(Part::Bridges));
    let (bx, by) = (sb.bass_bridge[0].0 * sb.a, sb.bass_bridge[0].1 * sb.b);
    label(pl(bx, by) + Vec2::new(-4.0 * k, -4.0 * k), Align2::RIGHT_BOTTOM, "Bass bridge", Some(Part::BassBridge));
    if lit(Part::BassBridge) {
        let text = format!("A0–{} on the bass bridge", note_name(sb.main_bridge_from - 1));
        label(pl(bx, by) + Vec2::new(0.0, 6.0 * k), Align2::CENTER_TOP, &text, Some(Part::BassBridge));
    }

    // The key shown: its bridge point, and how far its force spreads.
    let n = model.note(*key);
    let (kx, ky) = sb.bridge_point(*key);
    if lit(Part::BridgeSpread) && sb.bridge_stiffness > 0.0 {
        let omega = std::f64::consts::TAU * n.f0;
        let len = (sb.bridge_spread * std::f64::consts::TAU * (sb.bridge_stiffness / (sb.bridge_line_mass * omega * omega)).powf(0.25)).min(0.5);
        let main = *key >= sb.main_bridge_from;
        let (ax, ay) = sb.bridge_point_at(*key as f64 - 0.5, main);
        let (cx, cy) = sb.bridge_point_at(*key as f64 + 0.5, main);
        let (dx, dy) = (cx - ax, cy - ay);
        let norm = (dx * dx + dy * dy).sqrt().max(1e-9);
        let (ux, uy) = (dx / norm * len / 2.0, dy / norm * len / 2.0);
        p.line_segment([pl(kx - ux, ky - uy), pl(kx + ux, ky + uy)], line(6.0, HIGHLIGHT.gamma_multiply(0.7)));
        let text = format!("{}: spread over {:.0} cm", note_name(*key), len * 100.0);
        label(pl(kx, ky) + Vec2::new(10.0 * k, 0.0), Align2::LEFT_CENTER, &text, Some(Part::BridgeSpread));
    }
    p.circle_filled(pl(kx, ky), 3.5 * k, ink);

    // Keyboard, the key shown marked.
    p.rect_filled(keys, 2.0, vis.extreme_bg_color);
    p.rect_stroke(keys, 2.0, line(1.0, weak), egui::StrokeKind::Inside);
    for m in (21..=108u8).filter(|m| !is_black(*m)) {
        let y = bridge_y(m);
        p.line_segment([pl(fx, y), pl(front, y)], Stroke::new(0.5, weak.gamma_multiply(0.6)));
    }
    let ky_screen = pl(front, bridge_y(*key));
    p.add(Shape::convex_polygon(
        vec![ky_screen, ky_screen + Vec2::new(-4.0, 7.0) * k, ky_screen + Vec2::new(4.0, 7.0) * k],
        ink,
        Stroke::NONE,
    ));
    label(ky_screen + Vec2::new(0.0, 8.0 * k), Align2::CENTER_TOP, &note_name(*key), None);
    label(pl(front, sb.b) + Vec2::new(0.0, 8.0 * k), Align2::LEFT_TOP, "Bass", None);
    label(pl(front, 0.0) + Vec2::new(0.0, 8.0 * k), Align2::RIGHT_TOP, "Treble", None);

    // Side section, keyboard on the left: rim walls, board and the image
    // of its underside, frame, keys.
    p.text(at(sec_x, top - 14.0), Align2::LEFT_TOP, "Side section", font.clone(), weak);
    p.text(at(sec_x, top), Align2::LEFT_TOP, "keyboard left, heights ×2.5", font.clone(), weak);
    let keys_side = Rect::from_two_pos(sec(front, 0.0), sec(fx, 0.04));
    p.rect_filled(keys_side, 1.0, vis.extreme_bg_color);
    p.rect_stroke(keys_side, 1.0, line(1.0, weak), egui::StrokeKind::Inside);
    if rim.enabled {
        let rs = if lit(Part::RimSection) || lit(Part::Rim) { HIGHLIGHT } else { weak.gamma_multiply(0.6) };
        for (a, b) in [(-t, 0.0), (sb.a, sb.a + t)] {
            p.rect_filled(Rect::from_two_pos(sec(a, rim.top - rim.height), sec(b, rim.top)), 1.0, rs);
        }
        label(sec(sb.a + t, rim.top - rim.height) + Vec2::new(0.0, 3.0 * k), Align2::RIGHT_TOP, "Rim", Some(Part::RimSection));
    }
    let bs = if lit(Part::Board) { line(3.0, HIGHLIGHT) } else { line(3.0, WOOD) };
    p.line_segment([sec(0.0, 0.0), sec(sb.a, 0.0)], bs);
    if f.enabled {
        let fs = if lit(Part::FrameHeight) || lit(Part::Frame) { line(2.5, HIGHLIGHT) } else { line(1.5, weak) };
        p.line_segment([sec(f.offset_x, f.height), sec(f.offset_x + f.a, f.height)], fs);
        label(sec(f.offset_x, f.height) + Vec2::new(0.0, -2.0 * k), Align2::LEFT_BOTTOM, "Frame", Some(Part::FrameHeight));
    }
    label(sec(sb.a * 0.5, 0.0) + Vec2::new(0.0, 2.0 * k), Align2::CENTER_TOP, "Board", Some(Part::Board));
    let back = if lit(Part::BackFace) { line(1.5, HIGHLIGHT) } else { line(1.0, weak.gamma_multiply(0.6)) };
    p.extend(Shape::dashed_line(&[sec(0.0, -sb.back_depth), sec(sb.a, -sb.back_depth)], back, 4.0, 3.0));
    label(sec(sb.a * 0.5, -sb.back_depth) + Vec2::new(0.0, 2.0 * k), Align2::CENTER_TOP, "Underside (its image)", Some(Part::BackFace));
    // An upright's wall, parallel to the board behind its underside.
    if sb.wall_distance > 0.0 {
        let wz = -sb.back_depth - sb.wall_distance;
        let ws = if lit(Part::Wall) { line(3.0, HIGHLIGHT) } else { line(2.0, weak) };
        p.line_segment([sec(x0, wz), sec(x1, wz)], ws);
        for i in 0..12 {
            let x = x0 + (x1 - x0) * i as f64 / 12.0;
            p.line_segment([sec(x, wz), sec(x, wz) + Vec2::new(-4.0, 4.0) * k], line(1.0, weak));
        }
        let text = format!("Wall, {:.2} m behind (reflects {:.0} %)", sb.wall_distance, 100.0 * sb.wall_reflection);
        label(sec(x1, wz) + Vec2::new(0.0, 6.0 * k), Align2::RIGHT_TOP, &text, Some(Part::Wall));
    } else if lit(Part::Wall) {
        label(sec(sb.a * 0.5, -sb.back_depth) + Vec2::new(0.0, 16.0 * k), Align2::CENTER_TOP, "No wall (distance 0)", Some(Part::Wall));
    }
    // 10 cm scale under the section.
    let y = sec_top + (z1 - z0) as f32 * s2 * STRETCH + if sb.wall_distance > 0.0 { 30.0 } else { 18.0 };
    let l = 0.1 * s2;
    p.line_segment([at(sec_x, y), at(sec_x + l, y)], line(1.5, ink));
    p.text(at(sec_x + l + 4.0, y), Align2::LEFT_CENTER, "10 cm", font.clone(), weak);
    resp
}

/// The room from above (x up, y to the left, as the case is drawn) and
/// from the side, to scale, and the decay of its reverberation.
pub fn room(ui: &mut Ui, model: &PianoModel, lit: &[Part], width: f32) -> Response {
    let rp = &model.room;
    let Some(sb) = model.soundboard.map(|s| s.resolved()) else {
        return ui.label("This model has no soundboard: no sound reaches the room.");
    };
    let k = width / W;
    let [lx, ly, lz] = rp.size;
    // Plan in the left column (250 units at most), side view and decay on
    // the right.
    let s = (250.0 / ly as f32).min(250.0 / lx as f32);
    let (pw, ph) = (ly as f32 * s, lx as f32 * s);
    let top = 18.0;
    let rx = 8.0 + pw + 30.0;
    let rw = W - 4.0 - rx;
    let se = rw / lx as f32;
    let side_h = lz as f32 * se;
    let h = (top + ph + 22.0).max(top + side_h + 130.0);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(W, h) * k, Sense::hover());
    if !ui.is_rect_visible(rect) {
        return resp;
    }
    let at = |x: f32, y: f32| rect.min + Vec2::new(x, y) * k;
    let pl = |x: f64, y: f64| at(8.0 + (ly - y) as f32 * s, top + (lx - x) as f32 * s);
    let side = |x: f64, z: f64| at(rx + x as f32 * se, top + (lz - z) as f32 * se);
    let p = ui.painter_at(rect);
    let vis = ui.visuals();
    let (ink, weak) = (vis.text_color(), vis.weak_text_color());
    let font = FontId::proportional((10.0 * k).max(9.0));
    let lit = |part: Part| lit.contains(&part);
    let line = |w: f32, c: Color32| Stroke::new(w * k.max(0.6), c);
    let label = |pos: Pos2, align: Align2, text: &str, part: Option<Part>| {
        let c = if part.is_some_and(lit) { ink } else { weak };
        p.text(pos, align, text, font.clone(), c);
    };
    let piano = rp.piano;
    let mics: Vec<[f64; 3]> = sb.mics.iter().map(|m| crate::mics::in_room(model, &sb, piano, &[m.0, m.1, m.2])).collect();
    // The board's extent in the room: flat, or standing on its foot.
    let (blo, bhi) = crate::mics::footprint(model, &sb, piano);
    let (src, rc) = crate::mics::source_and_critical_distance(model, &sb, piano);
    let walls = |part| if lit(part) { line(2.5, HIGHLIGHT) } else { line(1.0, weak) };
    let absorb = lit(Part::AbsorbLow) || lit(Part::AbsorbHigh);
    let wall_stroke = if absorb { line(4.0, HIGHLIGHT.gamma_multiply(0.7)) } else { walls(Part::RoomSize) };

    // From above: walls, critical distance, piano, its beam, the mics, and
    // the first reflections off the four walls towards the left mic.
    p.text(at(8.0, top - 14.0), Align2::LEFT_TOP, format!("The room from above, {lx} × {ly} m"), font.clone(), weak);
    let room = Rect::from_two_pos(pl(0.0, 0.0), pl(lx, ly));
    p.rect_filled(room, 0.0, vis.extreme_bg_color);
    p.rect_stroke(room, 0.0, wall_stroke, egui::StrokeKind::Middle);
    // What spreads from the piano stays inside the room's outline.
    let inside = p.with_clip_rect(room.intersect(p.clip_rect()));
    let rc_stroke = if lit(Part::Directivity) || lit(Part::Tail) { line(1.5, HIGHLIGHT) } else { line(1.0, weak.gamma_multiply(0.7)) };
    inside.circle_stroke(pl(src[0], src[1]), rc as f32 * s * k, rc_stroke);
    if lit(Part::Directivity) || lit(Part::Tail) {
        let text = format!("reverberation = direct sound at {rc:.1} m");
        label(room.left_bottom() + Vec2::new(0.0, 4.0 * k), Align2::LEFT_TOP, &text, Some(Part::Directivity));
    }
    if lit(Part::Reflections) && !mics.is_empty() {
        let m = mics[0];
        for (wall, mirror) in [(0, 0.0), (0, lx), (1, 0.0), (1, ly)] {
            // Image of the source in the wall; the path bends where the
            // line from the image to the mic crosses the wall.
            let mut img = [src[0], src[1]];
            img[wall] = 2.0 * mirror - src[wall];
            let t = (mirror - img[wall]) / (m[wall] - img[wall]);
            let hit = [img[0] + t * (m[0] - img[0]), img[1] + t * (m[1] - img[1])];
            inside.add(Shape::line(vec![pl(src[0], src[1]), pl(hit[0], hit[1]), pl(m[0], m[1])], line(1.0, HIGHLIGHT.gamma_multiply(0.8))));
            p.circle_filled(pl(hit[0], hit[1]), 3.0 * k, HIGHLIGHT);
        }
        let o = rp.order as i64;
        let paths = (-o..=o)
            .flat_map(|a| (-o..=o).flat_map(move |b| (-o..=o).map(move |c| a.abs() + b.abs() + c.abs())))
            .filter(|&n| n >= 1 && n <= o)
            .count();
        let text = format!("first reflections to the left mic; order {o}: {paths} paths per mic");
        label(pl(0.0, ly) + Vec2::new(0.0, 4.0 * k), Align2::LEFT_TOP, &text, Some(Part::Reflections));
    }
    let board = Rect::from_two_pos(pl(blo[0], blo[1]), pl(bhi[0], bhi[1])).expand(1.5 * k);
    p.rect_filled(board, 0.0, if lit(Part::PianoPlace) { HIGHLIGHT } else { WOOD });
    if lit(Part::Directivity) {
        // The beam a + (1 − a)·cos θ of directivity factor Q, towards the
        // lid's opening (−y): Q = 1/(a² + (1 − a)²/3).
        let q = piano_core::room::piano_directivity(rp, &sb);
        // A grand beams towards its lid's opening (−y), an upright away
        // from its wall (+y).
        let towards = if sb.vertical { 1.0 } else { -1.0 };
        let a = ((1.0 + (12.0 / q - 3.0).max(0.0).sqrt()) / 4.0).clamp(0.25, 1.0);
        let r0 = (rc * 0.6).min(lx.min(ly) * 0.3);
        let pts: Vec<Pos2> = (0..=72)
            .map(|i| i as f64 / 72.0 * std::f64::consts::TAU)
            .map(|th| {
                let g = (a + (1.0 - a) * th.cos()).abs() * r0;
                pl(src[0] + g * th.sin(), src[1] + towards * g * th.cos())
            })
            .collect();
        inside.add(Shape::closed_line(pts, line(1.5, HIGHLIGHT)));
        let text = if rp.directivity > 0.0 { format!("Q = {q:.1}") } else { format!("Q = {q:.1} (from the piano)") };
        label(room.left_top() + Vec2::new(4.0, 4.0) * k, Align2::LEFT_TOP, &text, Some(Part::Directivity));
    }
    for (m, c) in mics.iter().zip([Color32::from_rgb(127, 119, 221), Color32::from_rgb(29, 158, 117)]) {
        p.circle_filled(pl(m[0], m[1]), 3.0 * k, c);
    }
    if lit(Part::PianoPlace) {
        let text = format!("piano corner at {:.1}, {:.1} m", piano[0], piano[1]);
        label(board.center_top() + Vec2::new(0.0, -4.0 * k), Align2::CENTER_BOTTOM, &text, Some(Part::PianoPlace));
    }

    // From the side: floor, ceiling, board height, mics; floor and ceiling
    // reflections.
    p.text(at(rx, top - 14.0), Align2::LEFT_TOP, format!("From the side, {lz} m high"), font.clone(), weak);
    let elev = Rect::from_two_pos(side(0.0, 0.0), side(lx, lz));
    p.rect_filled(elev, 0.0, vis.extreme_bg_color);
    p.rect_stroke(elev, 0.0, wall_stroke, egui::StrokeKind::Middle);
    let standing = Rect::from_two_pos(side(blo[0], blo[2]), side(bhi[0], bhi[2])).expand(1.5 * k);
    p.rect_filled(standing, 0.0, if lit(Part::PianoPlace) { HIGHLIGHT } else { WOOD });
    if lit(Part::Reflections) && !mics.is_empty() {
        let m = mics[0];
        for mirror in [0.0, lz] {
            let img = 2.0 * mirror - src[2];
            let t = (mirror - img) / (m[2] - img);
            let hit = src[0] + t * (m[0] - src[0]);
            p.add(Shape::line(vec![side(src[0], src[2]), side(hit, mirror), side(m[0], m[2])], line(1.0, HIGHLIGHT.gamma_multiply(0.8))));
            p.circle_filled(side(hit, mirror), 3.0 * k, HIGHLIGHT);
        }
    }
    for (m, c) in mics.iter().zip([Color32::from_rgb(127, 119, 221), Color32::from_rgb(29, 158, 117)]) {
        p.circle_filled(side(m[0], m[2]), 3.0 * k, c);
    }

    // Decay of the reverberation: −60 dB at T60, low and high frequency;
    // the tail level shifts both.
    let (t_lo, t_hi) = rp.t60();
    let gy = top + side_h + 34.0;
    let (gx0, gx1, gy0, gy1) = (rx + 20.0, W - 6.0, gy, gy + 70.0);
    p.text(at(rx, gy - 16.0), Align2::LEFT_TOP, "Reverberation decay", font.clone(), weak);
    let tmax = 1.15 * t_lo.max(t_hi).max(0.1);
    let level = 20.0 * rp.tail_gain.max(1e-3).log10();
    let gpt = |t: f64, db: f64| at(gx0 + (t / tmax) as f32 * (gx1 - gx0), gy0 + ((-db).clamp(0.0, 70.0) / 70.0) as f32 * (gy1 - gy0));
    p.line_segment([gpt(0.0, 0.0), gpt(0.0, -70.0)], line(1.0, weak));
    p.line_segment([gpt(0.0, -70.0), gpt(tmax, -70.0)], line(1.0, weak));
    for (db, text) in [(0.0, "0 dB"), (-60.0, "−60")] {
        p.text(gpt(0.0, db) + Vec2::new(-3.0 * k, 0.0), Align2::RIGHT_CENTER, text, font.clone(), weak);
    }
    let tail = lit(Part::Tail);
    for (t60, part, name) in [(t_lo, Part::AbsorbLow, "low"), (t_hi, Part::AbsorbHigh, "high")] {
        let c = if lit(part) || tail { HIGHLIGHT } else if name == "low" { ink } else { weak };
        let end = (t60 * (60.0 + level) / 60.0).min(tmax);
        p.line_segment([gpt(0.0, level), gpt(end, level - 60.0 * end / t60)], line(1.5, c));
        // The slower (low) curve labelled above, the faster below.
        let mid = 0.45 * end;
        let slower = if name == "low" { t_lo >= t_hi } else { t_hi > t_lo };
        let text = format!("{name}: {t60:.1} s");
        let (off, align) = if slower { (-3.0, Align2::LEFT_BOTTOM) } else { (3.0, Align2::RIGHT_TOP) };
        label(gpt(mid, level - 60.0 * mid / t60) + Vec2::new(0.0, off * k), align, &text, Some(part));
    }
    p.text(gpt(tmax, -70.0) + Vec2::new(0.0, 2.0 * k), Align2::RIGHT_TOP, format!("{tmax:.1} s"), font.clone(), weak);
    if tail {
        label(gpt(tmax, level) + Vec2::new(0.0, 2.0 * k), Align2::RIGHT_TOP, &format!("tail level {level:+.1} dB"), Some(Part::Tail));
    }
    if !rp.enabled {
        p.text(room.center(), Align2::CENTER_CENTER, "Room off: the microphones hear the piano alone", font.clone(), ink);
    }
    resp
}

/// The keys a key-number parameter covers.
fn key_range(model: &PianoModel, path: &str) -> Option<(u8, u8)> {
    let u = &model.unison;
    Some(match path {
        "damper.highest_note" => (21, model.damper.highest_note),
        "scale.break_note" => (21, model.scale.break_note),
        "unison.bichord_from" => (u.bichord_from, u.trichord_from.saturating_sub(1)),
        "unison.trichord_from" => (u.trichord_from, 108),
        "nonlinear.max_note" => (21, model.nonlinear.max_note),
        "losses.wound_until" => (21, model.losses.wound_until),
        "losses.top_from" => (model.losses.top_from, 108),
        _ => return None,
    })
}

pub(crate) fn note_name(n: u8) -> String {
    crate::widgets::note_name(n)
}

fn seconds(t: f64) -> String {
    if t >= 10.0 { format!("{t:.0} s") } else { format!("{t:.1} s") }
}

/// The soft pedal at full travel: a grand's una corda shifts the hammer
/// across (seen from above), an upright's half blow brings it closer to
/// the strings (seen from the side); both may combine.
#[allow(clippy::too_many_arguments)]
fn soft_pedal(p: &egui::Painter, at: &dyn Fn(f32, f32) -> Pos2, model: &PianoModel, lit: bool, felt: Color32, ink: Color32, weak: Color32, font: &FontId, k: f32) {
    let r = |x0: f32, y0: f32, x1: f32, y1: f32| Rect::from_two_pos(at(x0, y0), at(x1, y1));
    let line = |w: f32, c: Color32| Stroke::new(w * k.max(0.6), c);
    let text_c = if lit { ink } else { weak };
    let fill = if lit { HIGHLIGHT.gamma_multiply(0.55) } else { felt };
    let (shift, blow) = (model.una_corda_shift.clamp(0.0, 1.0) as f32, model.half_blow.max(0.0));
    let y0 = 196.0;
    if shift > 0.0 {
        p.text(at(20.0, y0), Align2::LEFT_TOP, "Soft pedal: una corda, from above", font.clone(), text_c);
        for (i, sy) in [214.0, 222.0, 230.0].into_iter().enumerate() {
            let c = if i == 0 && lit { HIGHLIGHT } else { ink };
            p.line_segment([at(24.0, sy), at(170.0, sy)], line(1.0, c));
        }
        p.rect_stroke(r(80.0, 209.0, 96.0, 235.0), 2.0, line(1.0, weak), egui::StrokeKind::Inside);
        let dy = 8.0 * shift;
        p.rect_filled(r(104.0, 209.0 + dy, 120.0, 235.0 + dy), 2.0, fill);
        arrow(p, at(97.0, 222.0), at(103.0, 222.0 + dy), line(1.0, weak));
    } else {
        p.text(at(20.0, y0), Align2::LEFT_TOP, "Soft pedal: half blow, from the side", font.clone(), text_c);
        p.line_segment([at(24.0, 217.0), at(170.0, 217.0)], line(1.5, ink));
        p.add(ellipse(at(84.0, 236.0), 6.0 * k, 8.0 * k, Color32::TRANSPARENT, line(1.0, weak)));
        let up = (60.0 * blow as f32).clamp(0.0, 12.0);
        p.add(ellipse(at(112.0, 236.0 - up), 6.0 * k, 8.0 * k, fill, Stroke::NONE));
        arrow(p, at(94.0, 240.0), at(104.0, 240.0 - up), line(1.0, weak));
    }
    if lit {
        let mut effects = Vec::new();
        if shift > 0.0 {
            effects.push(format!("softer felt by {:.0} %", 100.0 * model.una_corda_softening * shift as f64));
        }
        if blow > 0.0 {
            effects.push(format!("hammer {:.0} % slower", 100.0 * blow));
        }
        let text = if effects.is_empty() { "no effect".to_string() } else { effects.join(", ") };
        p.text(at(126.0, 238.0), Align2::LEFT_CENTER, text, font.clone(), ink);
    }
}

/// The shank under the head: rigid, or its spring's frequency (middle–top).
fn shank_text(model: &PianoModel) -> String {
    if model.shank_share > 0.0 {
        format!("Flexible shank, {:.0}–{:.0} Hz", model.shank_freq_at(60.0), model.shank_freq_at(108.0))
    } else {
        "Rigid shank".to_string()
    }
}

/// How a key struck again soon after its release sounds.
fn repetition_text(model: &PianoModel) -> String {
    if model.repetition_time > 0.0 {
        let slower = 100.0 * (1.0 - model.repetition_floor.clamp(0.0, 1.0));
        format!("re-struck within {:.0} ms, up to {slower:.0} % slower", 1e3 * model.repetition_time)
    } else {
        "re-strikes at full speed at once".to_string()
    }
}

/// The felt's layers: a top felt over a core at some depth, or a felt
/// graded towards its core (depth 0).
fn felt_text(model: &PianoModel, depth: f64, key: &str) -> String {
    if depth > 0.0 {
        format!("Top felt {depth:.2} mm ({key})")
    } else {
        format!("Graded felt, inner exponent {:.0}", model.core_p)
    }
}

/// Rings of a graded felt from the surface inwards: (share of the way in,
/// colour from the felt's to the core's, or to the highlight when lit).
fn graded_rings(felt: Color32, lit: bool) -> impl Iterator<Item = (f32, Color32)> {
    let inner = if lit { HIGHLIGHT } else { CORE };
    let mix = move |t: f32| {
        let m = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t) as u8;
        Color32::from_rgb(m(felt.r(), inner.r()), m(felt.g(), inner.g()), m(felt.b(), inner.b()))
    };
    (1..=4).map(move |i| i as f32 / 5.0).map(move |f| (f, mix(f)))
}

/// Top-felt depth (mm) shown for a parameter, and the key it is at.
fn felt_depth(model: &PianoModel, path: &str) -> (f64, &'static str) {
    match path {
        "core_depth_bass" => (model.core_depth_bass, "A0"),
        "core_depth_treble" => (model.core_depth_treble, "C8"),
        _ => (model.core_depth_mid, "C4"),
    }
}

/// Lighter felt for softer voicing (hardness on a log scale around 0.3).
fn felt_shade(hardness: f64) -> Color32 {
    let t = ((hardness.max(1e-3) / 0.3).log10() / 1.2).clamp(-1.0, 1.0) as f32;
    let mix = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * (t + 1.0) / 2.0) as u8;
    Color32::from_rgb(mix(245, 190), mix(238, 170), mix(220, 140))
}

/// Damper contact against pedal travel (0 → 1), the half-pedal zone shaded.
fn pedal_travel(p: &egui::Painter, rect: Rect, model: &PianoModel, lit: bool, ink: Color32, weak: Color32, font: &FontId) {
    let d = &model.damper;
    let x = |t: f64| rect.left() + t as f32 * rect.width();
    let half = Rect::from_x_y_ranges(x(d.pedal_free)..=x(d.pedal_contact), rect.y_range());
    p.rect_filled(half, 0.0, if lit { HIGHLIGHT.gamma_multiply(0.3) } else { weak.gamma_multiply(0.15) });
    p.rect_stroke(rect, 0.0, Stroke::new(1.0, weak), egui::StrokeKind::Inside);
    let pts: Vec<Pos2> = (0..=48)
        .map(|i| i as f64 / 48.0)
        .map(|t| Pos2::new(x(t), rect.bottom() - d.pedal_engagement(t) as f32 * rect.height()))
        .collect();
    p.add(Shape::line(pts, Stroke::new(1.5, if lit { HIGHLIGHT } else { ink })));
    p.text(rect.left_bottom() + Vec2::new(0.0, 2.0), Align2::LEFT_TOP, "up", font.clone(), weak);
    p.text(rect.right_bottom() + Vec2::new(0.0, 2.0), Align2::RIGHT_TOP, "down", font.clone(), weak);
}

fn ellipse(c: Pos2, rx: f32, ry: f32, fill: Color32, stroke: Stroke) -> Shape {
    let pts = (0..32).map(|i| i as f32 / 32.0 * std::f32::consts::TAU).map(|a| c + Vec2::new(rx * a.cos(), ry * a.sin())).collect();
    Shape::convex_polygon(pts, fill, stroke)
}

fn arrow(p: &egui::Painter, from: Pos2, to: Pos2, stroke: Stroke) {
    p.line_segment([from, to], stroke);
    let dir = (to - from).normalized();
    let side = Vec2::new(-dir.y, dir.x);
    let h = 3.0 + 2.0 * stroke.width;
    p.add(Shape::convex_polygon(vec![to, to - dir * h + side * h * 0.6, to - dir * h - side * h * 0.6], stroke.color, Stroke::NONE));
}

/// An impact: short rays around a point.
fn knock(p: &egui::Painter, c: Pos2, r: f32, lit: bool, weak: Color32) {
    let (color, w) = if lit { (HIGHLIGHT, 2.0) } else { (weak.gamma_multiply(0.6), 1.0) };
    for i in 0..8 {
        let a = i as f32 / 8.0 * std::f32::consts::TAU;
        let d = Vec2::angled(a);
        p.line_segment([c + d * 0.4 * r, c + d * r], Stroke::new(w, color));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parts_name_real_parameters() {
        let reg = piano_params::Registry::new();
        let parts_of = |p: &str| locate(p).map_or(&[][..], |(_, parts)| parts);
        assert_eq!(locate("damper.pedal_free"), Some((Sketch::Action, &[Part::SustainPedal][..])));
        assert_eq!(locate("damper.fall_time"), Some((Sketch::Action, &[Part::Damper][..])));
        assert_eq!(locate("damper.position"), Some((Sketch::String, &[Part::DamperPoint][..])));
        assert_eq!(locate("damper.horizontal"), Some((Sketch::String, &[Part::DamperPoint, Part::Polarization][..])));
        assert_eq!(locate("unison.bichord_from"), Some((Sketch::String, &[Part::Unison, Part::Keys][..])));
        assert_eq!(parts_of("core_depth_mid"), &[Part::Core]);
        assert_eq!(parts_of("hardness_slope_bass"), &[Part::Felt]);
        assert!(locate("output_gain_db").is_none());
        assert_eq!(locate("soundboard.piston_mass"), Some((Sketch::Plan, &[Part::Piston][..])));
        assert_eq!(locate("soundboard.bridge_mass"), Some((Sketch::String, &[Part::Bridge][..])));
        assert_eq!(locate("soundboard.dx"), Some((Sketch::Plan, &[Part::Board][..])));
        assert!(locate("soundboard.mics[0][1]").is_none() && locate("soundboard.lid_angle").is_none());
        assert_eq!(locate("rim.top"), Some((Sketch::Plan, &[Part::RimSection][..])));
        assert_eq!(locate("room.size[2]"), Some((Sketch::Room, &[Part::RoomSize][..])));
        assert_eq!(locate("half_blow"), Some((Sketch::Action, &[Part::UnaCorda][..])));
        assert_eq!(locate("una_corda_shift"), Some((Sketch::Action, &[Part::UnaCorda][..])));
        assert_eq!(locate("soundboard.wall_distance"), Some((Sketch::Plan, &[Part::Wall][..])));
        // Every key-number parameter lit on the keyboard has its range.
        let m = PianoModel::default();
        for p in reg.params().iter().filter(|p| parts_of(&p.path).contains(&Part::Keys)) {
            let (lo, hi) = key_range(&m, &p.path).unwrap_or_else(|| panic!("{} has no key range", p.path));
            assert!(21 <= lo && lo <= hi && hi <= 108, "{}: {lo}–{hi}", p.path);
        }
        // Every part is reached by some parameter.
        for part in [
            Part::Key,
            Part::Felt,
            Part::Core,
            Part::Damper,
            Part::SustainPedal,
            Part::UnaCorda,
            Part::KeybedKnock,
            Part::DamperLanding,
            Part::PedalKnock,
            Part::ActionKnock,
            Part::Shank,
            Part::MuteFelt,
            Part::Repetition,
            Part::Strike,
            Part::DamperPoint,
            Part::Length,
            Part::Unison,
            Part::Polarization,
            Part::Nonlinear,
            Part::Losses,
            Part::Bridge,
            Part::Partials,
            Part::Sympathy,
            Part::Keys,
            Part::Board,
            Part::BoardTreble,
            Part::BoardBass,
            Part::Bridges,
            Part::BassBridge,
            Part::BridgeSpread,
            Part::Piston,
            Part::BackFace,
            Part::BoardEdge,
            Part::Wall,
            Part::Frame,
            Part::Agraffes,
            Part::Rim,
            Part::FrameHeight,
            Part::RimSection,
            Part::RoomSize,
            Part::PianoPlace,
            Part::AbsorbLow,
            Part::AbsorbHigh,
            Part::Reflections,
            Part::Tail,
            Part::Directivity,
        ] {
            assert!(reg.params().iter().any(|p| parts_of(&p.path).contains(&part)), "{part:?}");
        }
    }

    #[test]
    fn keyboard_layout() {
        assert_eq!(key_span(21), (0.0, 1.0));
        assert_eq!(key_span(108), (51.0, 1.0));
        assert_eq!(key_at(0.5, false), 21);
        assert_eq!(key_at(0.95, true), 22); // A#0, over A0's right edge
        assert_eq!(key_at(0.95, false), 21);
        assert_eq!(key_at(51.5, true), 108);
        let c4 = key_span(60);
        assert_eq!(key_at(c4.0 + 0.5, false), 60);
        assert_eq!(key_at(-1.0, false), 21);
        assert_eq!(key_at(60.0, false), 108);
    }

    #[test]
    fn both_actions_draw() {
        let ctx = egui::Context::default();
        let grand = PianoModel::default();
        let upright = PianoModel { soundboard: grand.soundboard.map(|b| piano_core::soundboard::SoundboardParams { vertical: true, ..b }), ..grand };
        for m in [grand, upright] {
            ctx.run_ui(Default::default(), |ui| {
                for part in [&[Part::Key][..], &[Part::Core], &[Part::Damper], &[Part::UnaCorda], &[Part::MuteFelt], &[Part::SustainPedal], &[Part::Repetition]] {
                    action(ui, &m, part, "core_depth_mid", 440.0);
                }
            })
            .drop_without_applying_deltas();
        }
    }

    #[test]
    fn felt_shades_stay_in_range() {
        for h in [0.0, 0.02, 0.3, 2.0, 100.0] {
            let _ = felt_shade(h);
        }
    }
}
