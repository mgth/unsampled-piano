//! The 3D view's realistic look, on the GPU like the plain one
//! ([`super::gpu`]): each part shaded as what it is made of
//! ([`Material`]: lacquer under a clear coat, varnished spruce with its
//! grain, gold-painted iron, steel, brass, felt…) and lit by a studio: a
//! key softbox that casts soft shadows (a shadow map, softer the farther
//! the shadow falls from what casts it), two strip lights behind and a
//! fill in front, which the polished parts reflect; ambient occlusion in
//! the recesses (from the view's depth); a wooden floor that takes the
//! shadows and reflects the instrument; the strings as wires of their own
//! thickness. Drawn in floating point, multisampled, then exposed and
//! tone-mapped as a photograph is, with a little bloom on the brightest
//! highlights.
//!
//! The passes: the key light's shadow map; the view's depth alone, for the
//! occlusion, then the occlusion and its blur; the scene into a
//! multisampled float target (the room behind, the instrument reflected
//! in the floor, the floor, the instrument, its wires, the see-through
//! parts); resolved, the bloom from it, and the photograph into egui's
//! target.

use std::collections::HashMap;

use egui_glow::glow::{self, HasContext};

use super::cam::{add, cross, dot, norm, scale, sub, Camera, Mat4, V3, FOVY};
use super::scene::{Scene, Vertex};

/// The shadow map's size (texels).
const SHADOW: i32 = 2048;
/// The floor's material number, after [`Material`]'s.
const FLOOR: f32 = 14.0;
/// How far the floor reaches around the instrument (m).
const FLOOR_R: f32 = 30.0;

/// The studio's softboxes: the direction towards each, its half-sizes
/// across and up (tangents of the half angles) and its radiance (linear
/// RGB). The first is the key light, the one that casts shadows.
const BOXES: [(V3, [f32; 2], [f32; 3]); 4] = [
    // Key: high, in front and on the treble's side.
    ([0.45, -0.55, 0.85], [0.32, 0.22], [12.0, 11.6, 10.9]),
    // Fill: low, in front on the bass's side, cooler and dim.
    ([-0.8, -0.55, 0.35], [0.5, 0.35], [1.0, 1.06, 1.16]),
    // Strips behind on both sides, drawing the case's edges.
    ([-0.9, 0.75, 0.4], [0.07, 0.5], [8.0, 8.0, 8.0]),
    ([0.85, 0.85, 0.35], [0.07, 0.5], [6.0, 6.1, 6.3]),
];
/// The key light's angular radius as its shadows see it (rad): their
/// penumbra grows by this much per metre between caster and receiver.
const KEY_SOFT: f32 = 0.035;
/// Exposure of the photograph.
const EXPOSURE: f32 = 1.15;

/// A wire's vertex: an end, the wire's direction, its colour and
/// material, its diameter, which side of the ribbon (−1, 1) and how far
/// along (m).
#[derive(Clone, Copy)]
#[repr(C)]
struct WireVertex {
    pos: V3,
    dir: V3,
    color: [f32; 4],
    mat: f32,
    width: f32,
    at: [f32; 2],
}

fn bytes<T: Copy>(v: &[T]) -> &[u8] {
    // SAFETY: plain repr(C) f32 records with no padding.
    unsafe { std::slice::from_raw_parts(v.as_ptr().cast::<u8>(), std::mem::size_of_val(v)) }
}

const COMMON: &str = r#"
#ifdef GL_ES
precision highp float;
precision highp int;
precision highp sampler2D;
#endif
"#;

const SURFACE_VS: &str = r#"
in vec3 a_pos;
in vec3 a_normal;
in vec4 a_color;
in float a_mat;
uniform mat4 u_vp;
out vec3 v_pos;
out vec3 v_normal;
out vec4 v_color;
flat out int v_mat;
void main() {
    v_pos = a_pos;
    v_normal = a_normal;
    v_color = a_color;
    v_mat = int(a_mat + 0.5);
    gl_Position = u_vp * vec4(a_pos, 1.0);
}
"#;

/// A wire's ribbon, turned to face the eye, at least two pixels wide
/// (thinner wires cover only their share of it, and say how many pixels
/// thick they are).
const WIRE_VS: &str = r#"
in vec3 a_pos;
in vec3 a_dir;
in vec4 a_color;
in float a_mat;
in float a_width;
in vec2 a_at;
uniform mat4 u_vp;
uniform vec3 u_eye;
uniform float u_pixel;
out vec3 v_pos;
out vec3 v_axis;
out vec3 v_side;
out float v_s;
out vec4 v_color;
flat out int v_mat;
out float v_cover;
out float v_along;
out float v_d;
out float v_px;
void main() {
    vec3 axis = normalize(a_dir);
    vec3 to_eye = u_eye - a_pos;
    vec3 side = cross(axis, to_eye);
    side = length(side) > 1e-9 ? normalize(side) : vec3(0.0, 0.0, 1.0);
    float px = u_pixel * length(to_eye);
    float w = max(a_width, 2.0 * px);
    v_cover = a_width / w;
    v_px = a_width / px;
    v_pos = a_pos + side * (0.5 * w * a_at.x);
    v_axis = axis;
    v_side = side;
    v_s = a_at.x;
    v_color = a_color;
    v_mat = int(a_mat + 0.5);
    v_along = a_at.y;
    v_d = a_width;
    gl_Position = u_vp * vec4(v_pos, 1.0);
}
"#;

const DEPTH_FS: &str = r#"
out vec4 frag;
void main() {
    frag = vec4(1.0);
}
"#;

const FULLSCREEN_VS: &str = r#"
out vec2 v_uv;
void main() {
    vec2 p = vec2(float((gl_VertexID << 1) & 2), float(gl_VertexID & 2));
    v_uv = p;
    gl_Position = vec4(p * 2.0 - 1.0, 0.0, 1.0);
}
"#;

/// What the scene's shaders share: noise, the studio, the materials and
/// the lighting.
const LIGHTING: &str = r#"
const float PI = 3.14159265;
uniform vec3 u_eye;
uniform vec3 u_box_c[4];
uniform vec3 u_box_r[4];
uniform vec3 u_box_u[4];
uniform vec2 u_box_h[4];
uniform vec3 u_box_l[4];
uniform vec3 u_box_e[4];
uniform mat4 u_light;
uniform sampler2D u_shadow;
// Texel (uv), depth range (m), the map's width (m).
uniform vec3 u_shadow_info;
uniform float u_soft;
uniform sampler2D u_ao;
uniform vec2 u_view;
uniform float u_floor;
uniform float u_mirror;
// Over a grand's soundboard and pin block (x, y) and the board's height,
// what the parts see below them: the board and plate lit.
uniform vec4 u_board;
uniform float u_board_z;
uniform vec3 u_board_l;
// The board coloured by its motion (the live view).
uniform float u_live;

// The drawing's colours (sRGB): the highlight; the board's wood, and its
// motion's colours from rest to the loudest; steel and copper strings.
const vec3 HIGHLIGHT = vec3(239.0, 159.0, 39.0) / 255.0;
const vec3 WOOD = vec3(186.0, 117.0, 23.0) / 255.0;
const vec3 HEAT_0 = vec3(149.0, 94.0, 18.0) / 255.0;
const vec3 HEAT_1 = vec3(244.0, 200.0, 128.0) / 255.0;
const vec3 STEEL = vec3(170.0, 172.0, 178.0) / 255.0;
const vec3 COPPER = vec3(184.0, 115.0, 70.0) / 255.0;

float ign(vec2 p) {
    return fract(52.9829189 * fract(dot(p, vec2(0.06711056, 0.00583715))));
}
float hash(vec3 p) {
    p = fract(p * 0.3183099 + vec3(0.1, 0.2, 0.3));
    p *= 17.0;
    return fract(p.x * p.y * p.z * (p.x + p.y + p.z));
}
float noise(vec3 x) {
    vec3 i = floor(x);
    vec3 f = fract(x);
    f = f * f * (3.0 - 2.0 * f);
    return mix(mix(mix(hash(i), hash(i + vec3(1, 0, 0)), f.x), mix(hash(i + vec3(0, 1, 0)), hash(i + vec3(1, 1, 0)), f.x), f.y),
               mix(mix(hash(i + vec3(0, 0, 1)), hash(i + vec3(1, 0, 1)), f.x), mix(hash(i + vec3(0, 1, 1)), hash(i + vec3(1, 1, 1)), f.x), f.y), f.z);
}
float fbm(vec3 p) {
    float s = 0.0;
    float a = 0.5;
    for (int i = 0; i < 4; i++) {
        s += a * noise(p);
        p = p * 2.03 + vec3(1.7, 9.2, 3.1);
        a *= 0.5;
    }
    return s;
}
// 1 where a pattern of `scale` cycles a metre is a few pixels wide, 0
// where it would only shimmer.
float detail(vec3 p, float scale) {
    return 1.0 - smoothstep(0.25, 0.7, length(fwidth(p)) * scale);
}
// The normal `n` tilted by the slope of the height `h` (m) over the
// surface at `p`.
vec3 bump(vec3 n, vec3 p, float h) {
    vec3 dpdx = dFdx(p);
    vec3 dpdy = dFdy(p);
    vec3 r1 = cross(dpdy, n);
    vec3 r2 = cross(n, dpdx);
    float det = dot(dpdx, r1);
    vec3 g = sign(det) * (dFdx(h) * r1 + dFdy(h) * r2);
    vec3 m = abs(det) * n - g;
    return length(m) > 1e-12 ? normalize(m) : n;
}

// The room, without its softboxes: grey walls lit by them, a darker
// ceiling with a broad bounce over the instrument, the floor's colour
// below the horizon; blurred with the roughness.
vec3 room(vec3 d, float rough) {
    vec3 up = vec3(0.045, 0.046, 0.050);
    vec3 wall = vec3(0.12, 0.118, 0.115);
    vec3 down = vec3(0.012, 0.009, 0.007);
    float k = 0.08 + 0.6 * rough;
    vec3 c = d.z > 0.0 ? mix(wall, up, smoothstep(0.0, 0.7 + k, d.z)) : mix(wall, down, smoothstep(0.0, k, -d.z));
    return c + vec3(0.10, 0.10, 0.11) * smoothstep(0.75 - k, 0.97, d.z);
}
vec3 room_irradiance(vec3 n) {
    return PI * mix(vec3(0.022, 0.018, 0.015), vec3(0.075, 0.073, 0.074), 0.5 + 0.5 * n.z);
}
// Softbox `i` seen along `d`, its edges blurred by `b` (tangent units),
// its radiance spread as much.
float box(vec3 d, int i, float b) {
    float z = dot(d, u_box_c[i]);
    if (z <= 0.0) {
        return 0.0;
    }
    vec2 p = abs(vec2(dot(d, u_box_r[i]), dot(d, u_box_u[i])) / z);
    vec2 h = u_box_h[i];
    vec2 e = 1.0 - smoothstep(h - b, h + b, p);
    return e.x * e.y * (h.x * h.y) / ((h.x + b) * (h.y + b));
}
// The studio's radiance along `d` from `p` for a surface of roughness
// `rough`, the key light seen through `key` (its shadow). Over a grand's
// board and plate, what looks down sees them.
vec3 env(vec3 p, vec3 d, float rough, float key) {
    float b = 0.004 + 0.9 * rough * rough;
    vec3 c = room(d, rough);
    vec2 inside = step(u_board.xy, p.xy) * step(p.xy, u_board.zw);
    if (inside.x * inside.y > 0.5 && p.z > u_board_z - 0.005 && p.z < u_board_z + 0.3) {
        c = mix(c, u_board_l, smoothstep(0.0, 0.15 + rough, -d.z));
    }
    c += u_box_l[0] * box(d, 0, b) * key;
    for (int i = 1; i < 4; i++) {
        c += u_box_l[i] * box(d, i, b);
    }
    return c;
}
// Split-sum environment BRDF (Karis' fit).
vec2 env_brdf(float nov, float r) {
    vec4 c0 = vec4(-1.0, -0.0275, -0.572, 0.022);
    vec4 c1 = vec4(1.0, 0.0425, 1.04, -0.04);
    vec4 r4 = r * c0 + c1;
    float a004 = min(r4.x * r4.x, exp2(-9.28 * nov)) * r4.x + r4.y;
    return vec2(-1.04, 1.04) * a004 + r4.zw;
}

vec2 vogel(int i, int n, float turn) {
    float r = sqrt((float(i) + 0.5) / float(n));
    float a = float(i) * 2.39996 + turn;
    return r * vec2(cos(a), sin(a));
}
// The key light's share at `p` (geometric normal `ng`): its shadow map
// searched for what lies in between, then filtered as widely as the
// penumbra that far below it.
float shadow(vec3 p, vec3 ng) {
    float texel_m = u_shadow_info.x * u_shadow_info.z;
    vec3 q = p + ng * (2.0 * texel_m);
    vec3 s = (u_light * vec4(q, 1.0)).xyz * 0.5 + 0.5;
    if (s.x <= 0.0 || s.y <= 0.0 || s.x >= 1.0 || s.y >= 1.0 || s.z >= 1.0) {
        return 1.0;
    }
    float bias = 0.0015 / u_shadow_info.y;
    float turn = 6.2831853 * ign(gl_FragCoord.xy);
    float reach = u_soft * min(s.z * u_shadow_info.y, 0.8) / u_shadow_info.z;
    float zb = 0.0;
    float nb = 0.0;
    for (int i = 0; i < 12; i++) {
        float d = texture(u_shadow, s.xy + vogel(i, 12, turn) * reach).r;
        if (d < s.z - bias) {
            zb += d;
            nb += 1.0;
        }
    }
    if (nb < 0.5) {
        return 1.0;
    }
    zb /= nb;
    float pen = max(u_soft * (s.z - zb) * u_shadow_info.y / u_shadow_info.z, 1.5 * u_shadow_info.x);
    float lit = 0.0;
    for (int i = 0; i < 16; i++) {
        float d = texture(u_shadow, s.xy + vogel(i, 16, turn) * pen).r;
        lit += d < s.z - bias ? 0.0 : 1.0;
    }
    return lit / 16.0;
}

struct Surf {
    vec3 albedo;
    float metal;
    float rough;
    // A clear coat over it: its share, roughness and normal.
    float coat;
    float coat_rough;
    vec3 coat_n;
    // Felt's fuzz, light let through (wax, ivory), light given out.
    float sheen;
    float wrap;
    vec3 glow;
    vec3 n;
};

vec3 lin(vec3 c) {
    return pow(c, vec3(2.2));
}
// Highlights no sharper than the normal `n` turns over a pixel can show
// (geometric specular antialiasing): small curved parts do not sparkle.
void soften(inout Surf s, vec3 n, float extra) {
    vec3 d = fwidth(n);
    float k = min(dot(d, d) + extra, 0.5);
    s.rough = sqrt(min(s.rough * s.rough + k, 1.0));
    s.coat_rough = sqrt(min(s.coat_rough * s.coat_rough + k, 1.0));
}
// How far the colour `c` went from `from` towards `to` (0 … 1).
float toward(vec3 c, vec3 from, vec3 to) {
    vec3 d = to - from;
    return clamp(dot(c - from, d) / dot(d, d), 0.0, 1.0);
}

// What material `mat` drawn in `vc` is at `p`, its normal `n`; `wire`: a
// string, whose colour beyond its metal's tells its level.
Surf surface(int mat, vec4 vc, vec3 p, vec3 n, bool wire) {
    vec3 c = lin(vc.rgb);
    Surf s;
    s.albedo = c;
    s.metal = 0.0;
    s.rough = 0.5;
    s.coat = 0.0;
    s.coat_rough = 0.05;
    s.coat_n = n;
    s.sheen = 0.0;
    s.wrap = 0.0;
    s.glow = vec3(0.0);
    s.n = n;
    if (mat == 1) {
        // Lacquer: a black base under a thick polished coat, which waves
        // a little (orange peel).
        s.albedo = mix(vec3(dot(c, vec3(0.333))), c, 0.5) * 0.5;
        s.rough = 0.4;
        s.coat = 1.0;
        s.coat_rough = 0.03;
        s.coat_n = bump(n, p, 0.00002 * noise(p * 140.0) * detail(p, 140.0));
    } else if (mat == 2) {
        // Spruce: panels some 11 cm wide, their growth rings 2–4 mm apart
        // across the grain, its fibres along; a satin varnish. Its motion
        // (the live view), or its being pointed at, glows over it.
        vec2 q = abs(n.z) > abs(n.y) ? p.xy : p.xz;
        vec2 g = mat2(0.766, 0.643, -0.643, 0.766) * q;
        float panel = floor(g.x / 0.11);
        float ph = hash(vec3(panel, 1.0, 7.0));
        float rings = 220.0 + 140.0 * ph;
        float x = g.x * rings + 2.5 * fbm(vec3(g.x * 6.0, g.y * 0.7, panel));
        float ring = smoothstep(0.35, 0.8, fract(x)) * (1.0 - smoothstep(0.85, 1.0, fract(x)));
        float fibre = fbm(vec3(g.x * 700.0, g.y * 10.0, panel));
        vec3 wood = vec3(0.66, 0.44, 0.20) * mix(0.82, 1.1, ph) * (0.9 + 0.2 * fbm(vec3(g.x * 9.0, g.y * 1.2, panel)));
        float joint = smoothstep(0.0, 0.0006 + length(fwidth(g)), abs(g.x - (panel + 0.5) * 0.11 + 0.055 * sign(g.x - (panel + 0.5) * 0.11)));
        s.albedo = wood * (1.0 - 0.38 * ring * detail(p, rings)) * mix(1.0, 0.88 + 0.24 * fibre, detail(p, 700.0)) * mix(0.8, 1.0, joint);
        s.rough = 0.5;
        s.coat = 0.7;
        s.coat_rough = 0.16;
        float t = u_live > 0.5 ? toward(vc.rgb, HEAT_0, HEAT_1) / 0.75 : toward(vc.rgb, WOOD, HIGHLIGHT);
        s.glow = lin(HIGHLIGHT) * t * 0.9;
    } else if (mat == 3) {
        // Hard wood, raw or satin: a fine figure; its colours less vivid
        // than the drawing's.
        float l = dot(c, vec3(0.2126, 0.7152, 0.0722));
        s.albedo = mix(vec3(l), c, 0.7) * (0.86 + 0.28 * fbm(p * vec3(60.0, 60.0, 400.0)) * detail(p, 400.0));
        s.rough = 0.6;
    } else if (mat == 4) {
        // Cast iron in bronze paint (bronze powder in a binder): half
        // metallic, mottled, lacquered.
        s.albedo = c * (0.85 + 0.3 * fbm(p * 40.0));
        s.metal = 0.5;
        s.rough = 0.38;
        s.coat = 0.6;
        s.coat_rough = 0.1;
    } else if (mat == 5 || mat == 6) {
        // Steel, the wound strings' copper: their windings ridged along.
        s.metal = 1.0;
        s.rough = mat == 5 ? 0.22 : 0.3;
        if (wire) {
            // A string's level glows; it is drawn in its metal.
            vec3 metal = mat == 5 ? STEEL : COPPER;
            float t = toward(vc.rgb, metal, HIGHLIGHT);
            s.glow = lin(HIGHLIGHT) * t * 2.0;
            s.albedo = lin(mix(vc.rgb, metal, t));
        }
    } else if (mat == 7) {
        s.metal = 1.0;
        s.rough = 0.16;
    } else if (mat == 8) {
        // Felt: matte, fibrous, its fuzz catching the light at grazing
        // angles.
        s.albedo = c * (0.82 + 0.36 * fbm(p * 1500.0) * detail(p, 1500.0));
        s.rough = 0.95;
        s.sheen = 0.7;
    } else if (mat == 9) {
        s.albedo = c * (0.85 + 0.3 * fbm(p * 500.0) * detail(p, 500.0));
        s.rough = 0.62;
    } else if (mat == 10) {
        // The white keys: polished, a little translucent.
        s.albedo = c * 0.9;
        s.rough = 0.14;
        s.wrap = 0.35;
    } else if (mat == 11) {
        s.rough = 0.24;
    } else if (mat == 12) {
        s.metal = 1.0;
        s.albedo = c * 0.7;
        s.rough = 0.65;
    } else if (mat == 13) {
        // Gold leaf laid on size: a metal broken into flakes, which
        // scatter the light about as much as they mirror it.
        s.metal = 0.6;
        s.rough = 0.5;
    } else if (mat == 14) {
        // The floor: a stage of dark planks 14 cm wide, their ends
        // staggered, satin.
        float row = floor(p.x / 0.14);
        float shift = hash(vec3(row, 3.0, 1.0)) * 2.0;
        float len = 1.6;
        float y = p.y + shift;
        float id = hash(vec3(row, floor(y / len), 5.0));
        vec2 at = vec2(p.x - row * 0.14, y - floor(y / len) * len);
        float grain = fbm(vec3(p.x * 120.0, y * 5.0, id * 13.0));
        s.albedo = vec3(0.050, 0.032, 0.021) * (0.75 + 0.5 * id) * (0.8 + 0.4 * grain);
        float joint = min(min(at.x, 0.14 - at.x), min(at.y, len - at.y) * 0.3);
        float aa = max(fwidth(p.x), fwidth(p.y));
        s.albedo *= mix(0.4, 1.0, smoothstep(0.0, 0.0008 + aa, joint));
        s.rough = 0.4;
    } else {
        s.rough = 0.45;
    }
    return s;
}

// The surface lit at `p`, seen from the eye, the key light's share `key`,
// the ambient occlusion `ao`.
vec3 light(Surf s, vec3 p, float key, float ao) {
    vec3 v = normalize(u_eye - p);
    vec3 n = s.n;
    float nov = clamp(dot(n, v), 1e-3, 1.0);
    vec3 f0 = mix(vec3(0.04), s.albedo, s.metal);
    vec2 ab = env_brdf(nov, s.rough);
    vec3 fs = f0 * ab.x + ab.y;
    // Diffuse: the key light, wrapped round where light passes through;
    // the other softboxes and the room, occluded.
    float nl = dot(n, u_box_c[0]);
    float wrapped = max((nl + s.wrap) / (1.0 + s.wrap), 0.0);
    vec3 e = u_box_e[0] * wrapped * key * mix(1.0, ao, 0.4);
    vec3 amb = room_irradiance(n);
    for (int i = 1; i < 4; i++) {
        amb += u_box_e[i] * max(dot(n, u_box_c[i]), 0.0);
    }
    e += amb * ao;
    vec3 col = s.albedo * (1.0 - s.metal) * (1.0 - fs) / PI * e;
    // Fuzz: brighter at grazing angles.
    col += s.sheen * s.albedo * pow(1.0 - nov, 3.0) * e / PI;
    // Specular: the studio reflected, occluded in the recesses.
    float occ = clamp(pow(nov + ao, exp2(-16.0 * s.rough - 1.0)) - 1.0 + ao, 0.0, 1.0);
    vec3 r = reflect(-v, n);
    col += env(p, r, s.rough, key) * fs * occ * (1.0 - 0.7 * s.sheen);
    // The clear coat over it all.
    if (s.coat > 0.0) {
        float cv = clamp(dot(s.coat_n, v), 1e-3, 1.0);
        float fc = (0.04 + 0.96 * pow(1.0 - cv, 5.0)) * s.coat;
        vec2 cab = env_brdf(cv, s.coat_rough);
        vec3 rc = reflect(-v, s.coat_n);
        col = col * (1.0 - fc) + env(p, rc, s.coat_rough, key) * (0.04 * cab.x + cab.y) * s.coat * occ;
    }
    return col + s.glow;
}
"#;

const SURFACE_FS: &str = r#"
in vec3 v_pos;
in vec3 v_normal;
in vec4 v_color;
flat in int v_mat;
out vec4 frag;
void main() {
    // In the floor's reflection, nothing below it.
    if (u_mirror > 0.5 && v_pos.z < u_floor - 0.001) {
        discard;
    }
    vec3 v = normalize(u_eye - v_pos);
    // Faces are drawn from both sides: turned towards the eye.
    vec3 ng = normalize(cross(dFdx(v_pos), dFdy(v_pos)));
    if (dot(ng, v) < 0.0) {
        ng = -ng;
    }
    vec3 n = normalize(v_normal);
    if (dot(n, ng) < 0.0) {
        n = -n;
    }
    Surf s = surface(v_mat, v_color, v_pos, n, false);
    soften(s, n, 0.0);
    float key = shadow(v_pos, ng);
    float ao = u_mirror > 0.5 ? 1.0 : texture(u_ao, gl_FragCoord.xy / u_view).r;
    vec3 col = light(s, v_pos, key, ao);
    if (v_mat == 14) {
        // The floor over its reflection: its own light, and how much it
        // reflects (the blend's factor), fading far off into the room.
        float f = 0.04 + 0.96 * pow(1.0 - clamp(dot(n, v), 0.0, 1.0), 5.0);
        float far = smoothstep(4.0, 14.0, length(v_pos.xy - u_eye.xy));
        frag = vec4(col * (1.0 - f) * (1.0 - far), mix(0.03 + f, 1.0, far));
        return;
    }
    if (u_mirror > 0.5) {
        // Reflected, blurred out with the height above the floor.
        float h = v_pos.z - u_floor;
        col = mix(room(reflect(-v, vec3(0.0, 0.0, 1.0)), 0.3), col, exp(-h / 0.45));
    }
    frag = vec4(col, v_color.a);
}
"#;

const WIRE_FS: &str = r#"
in vec3 v_pos;
in vec3 v_axis;
in vec3 v_side;
in float v_s;
in vec4 v_color;
flat in int v_mat;
in float v_cover;
in float v_along;
in float v_d;
in float v_px;
out vec4 frag;
void main() {
    // The wire's round surface under the ribbon.
    vec3 v = normalize(u_eye - v_pos);
    vec3 facing = normalize(v - v_axis * dot(v, v_axis));
    float s = clamp(v_s, -1.0, 1.0);
    vec3 n = normalize(v_side * s + facing * sqrt(max(1.0 - s * s, 0.0)));
    Surf su = surface(v_mat, v_color, v_pos, n, true);
    if (v_mat == 6) {
        // A winding's turns, as far apart as it is thick.
        float pitch = max(0.25 * v_d, 0.0003);
        float t = v_along / pitch;
        float k = 1.0 - smoothstep(0.2, 0.6, fwidth(t));
        su.n = normalize(n + v_axis * (0.35 * k * sin(6.2831853 * t)));
    }
    // A wire a few pixels thick or less: each pixel sees much of its
    // round, its highlight spread as much.
    soften(su, su.n, 0.35 * (1.0 - smoothstep(0.5, 4.0, v_px)));
    float key = shadow(v_pos, n);
    float ao = texture(u_ao, gl_FragCoord.xy / u_view).r;
    // Its share of the ribbon, blended.
    frag = vec4(light(su, v_pos, key, ao), v_cover * v_color.a);
}
"#;

/// The room behind the instrument; below the horizon, what the floor
/// reflects of it (the floor is drawn over it).
const SKY_FS: &str = r#"
in vec2 v_uv;
uniform vec3 u_cam_r;
uniform vec3 u_cam_u;
uniform vec3 u_cam_f;
uniform vec2 u_tan;
uniform float u_above;
out vec4 frag;
void main() {
    vec2 ndc = v_uv * 2.0 - 1.0;
    vec3 d = normalize(u_cam_f + u_cam_r * ndc.x * u_tan.x + u_cam_u * ndc.y * u_tan.y);
    if (u_above > 0.5 && d.z < 0.0) {
        d.z = -d.z;
    }
    frag = vec4(room(d, 0.0), 1.0);
}
"#;

const SSAO_FS: &str = r#"
in vec2 v_uv;
uniform sampler2D u_depth;
// P00, P11, near, far.
uniform vec4 u_proj;
uniform vec2 u_texel;
out vec4 frag;
float view_z(vec2 uv) {
    float zn = texture(u_depth, uv).r * 2.0 - 1.0;
    return 2.0 * u_proj.z * u_proj.w / ((u_proj.w + u_proj.z) - zn * (u_proj.w - u_proj.z));
}
vec3 view_pos(vec2 uv) {
    float z = view_z(uv);
    vec2 ndc = uv * 2.0 - 1.0;
    return vec3(ndc.x * z / u_proj.x, ndc.y * z / u_proj.y, -z);
}
float ign(vec2 p) {
    return fract(52.9829189 * fract(dot(p, vec2(0.06711056, 0.00583715))));
}
void main() {
    if (texture(u_depth, v_uv).r >= 1.0) {
        frag = vec4(1.0);
        return;
    }
    vec3 p = view_pos(v_uv);
    vec3 px = view_pos(v_uv + vec2(u_texel.x, 0.0)) - p;
    vec3 nx = p - view_pos(v_uv - vec2(u_texel.x, 0.0));
    vec3 py = view_pos(v_uv + vec2(0.0, u_texel.y)) - p;
    vec3 ny = p - view_pos(v_uv - vec2(0.0, u_texel.y));
    vec3 dx = abs(px.z) < abs(nx.z) ? px : nx;
    vec3 dy = abs(py.z) < abs(ny.z) ? py : ny;
    vec3 n = normalize(cross(dx, dy));
    if (dot(n, p) > 0.0) {
        n = -n;
    }
    float a = 6.2831853 * ign(gl_FragCoord.xy);
    vec3 t = normalize(cross(n, abs(n.z) < 0.9 ? vec3(0.0, 0.0, 1.0) : vec3(1.0, 0.0, 0.0)));
    vec3 b = cross(n, t);
    vec3 t2 = t * cos(a) + b * sin(a);
    vec3 b2 = cross(n, t2);
    float occ = 0.0;
    // Two reaches: the recesses between parts (3 cm), the shade of the
    // case and of what stands over the floor (30 cm).
    for (int i = 0; i < 16; i++) {
        float fi = float(i);
        float r = i < 8 ? 0.03 : 0.3;
        float k = (mod(fi, 8.0) + 0.5) / 8.0;
        float phi = fi * 2.39996;
        vec3 dir = (t2 * cos(phi) + b2 * sin(phi)) * sqrt(k) + n * sqrt(1.0 - k);
        float reach = r * mix(0.25, 1.0, fract(fi * 0.618 + 0.3));
        vec3 s = p + dir * reach;
        vec2 uv = vec2(s.x * u_proj.x, s.y * u_proj.y) / -s.z * 0.5 + 0.5;
        float sz = -view_z(uv);
        float range = smoothstep(0.0, 1.0, r / max(abs(p.z - sz), 1e-4));
        occ += sz >= s.z + 0.02 * r ? range : 0.0;
    }
    frag = vec4(vec3(pow(clamp(1.0 - occ / 16.0, 0.0, 1.0), 1.4)), 1.0);
}
"#;

/// The occlusion blurred over 4 × 4 pixels (its noise's tile), not
/// across depth edges.
const BLUR_AO_FS: &str = r#"
in vec2 v_uv;
uniform sampler2D u_src;
uniform sampler2D u_depth;
uniform vec2 u_texel;
uniform vec4 u_proj;
out vec4 frag;
float view_z(vec2 uv) {
    float zn = texture(u_depth, uv).r * 2.0 - 1.0;
    return 2.0 * u_proj.z * u_proj.w / ((u_proj.w + u_proj.z) - zn * (u_proj.w - u_proj.z));
}
void main() {
    float z = view_z(v_uv);
    float sum = 0.0;
    float w = 0.0;
    for (int j = -2; j < 2; j++) {
        for (int i = -2; i < 2; i++) {
            vec2 uv = v_uv + (vec2(float(i), float(j)) + 0.5) * u_texel;
            float k = exp(-abs(view_z(uv) - z) / (0.01 * z));
            sum += texture(u_src, uv).r * k;
            w += k;
        }
    }
    frag = vec4(vec3(sum / max(w, 1e-4)), 1.0);
}
"#;

/// The brightest light at a quarter of the size, for the bloom.
const BRIGHT_FS: &str = r#"
in vec2 v_uv;
uniform sampler2D u_src;
uniform vec2 u_texel;
out vec4 frag;
void main() {
    vec3 c = vec3(0.0);
    for (int j = 0; j < 2; j++) {
        for (int i = 0; i < 2; i++) {
            c += texture(u_src, v_uv + (vec2(float(i), float(j)) - 0.5) * 2.0 * u_texel).rgb;
        }
    }
    c *= 0.25;
    float l = max(c.r, max(c.g, c.b));
    frag = vec4(c * smoothstep(1.0, 4.0, l), 1.0);
}
"#;

const GAUSS_FS: &str = r#"
in vec2 v_uv;
uniform sampler2D u_src;
uniform vec2 u_step;
out vec4 frag;
void main() {
    vec3 c = texture(u_src, v_uv).rgb * 0.227;
    c += (texture(u_src, v_uv + u_step).rgb + texture(u_src, v_uv - u_step).rgb) * 0.1945;
    c += (texture(u_src, v_uv + 2.0 * u_step).rgb + texture(u_src, v_uv - 2.0 * u_step).rgb) * 0.1216;
    c += (texture(u_src, v_uv + 3.0 * u_step).rgb + texture(u_src, v_uv - 3.0 * u_step).rgb) * 0.054;
    c += (texture(u_src, v_uv + 4.0 * u_step).rgb + texture(u_src, v_uv - 4.0 * u_step).rgb) * 0.0162;
    frag = vec4(c, 1.0);
}
"#;

/// The photograph: exposed, the bloom added, tone-mapped (ACES), a slight
/// vignette, into sRGB, dithered.
const FINAL_FS: &str = r#"
in vec2 v_uv;
uniform sampler2D u_hdr;
uniform sampler2D u_bloom;
uniform float u_exposure;
out vec4 frag;
vec3 aces(vec3 x) {
    return clamp(x * (2.51 * x + 0.03) / (x * (2.43 * x + 0.59) + 0.14), 0.0, 1.0);
}
void main() {
    vec3 c = texture(u_hdr, v_uv).rgb + 0.15 * texture(u_bloom, v_uv).rgb;
    vec2 q = v_uv - 0.5;
    c *= u_exposure * (1.0 - 0.35 * dot(q, q));
    c = pow(aces(c), vec3(1.0 / 2.2));
    float dither = fract(52.9829189 * fract(dot(gl_FragCoord.xy, vec2(0.06711056, 0.00583715)))) - 0.5;
    frag = vec4(c + dither / 255.0, 1.0);
}
"#;

/// A linked program and its uniforms by name.
struct Prog {
    p: glow::Program,
    u: HashMap<String, glow::UniformLocation>,
}

impl Prog {
    unsafe fn new(gl: &glow::Context, header: &str, vs: &str, fs: &str, attribs: &[&str]) -> Result<Self, String> {
        unsafe {
            let p = gl.create_program()?;
            let mut shaders = Vec::new();
            for (kind, src) in [(glow::VERTEX_SHADER, vs.to_string()), (glow::FRAGMENT_SHADER, fs.to_string())] {
                let s = gl.create_shader(kind)?;
                gl.shader_source(s, &format!("{header}{COMMON}{src}"));
                gl.compile_shader(s);
                if !gl.get_shader_compile_status(s) {
                    return Err(format!("realistic view shader: {}", gl.get_shader_info_log(s)));
                }
                gl.attach_shader(p, s);
                shaders.push(s);
            }
            for (i, name) in attribs.iter().enumerate() {
                gl.bind_attrib_location(p, i as u32, name);
            }
            gl.link_program(p);
            if !gl.get_program_link_status(p) {
                return Err(format!("realistic view shaders: {}", gl.get_program_info_log(p)));
            }
            for s in shaders {
                gl.detach_shader(p, s);
                gl.delete_shader(s);
            }
            let mut u = HashMap::new();
            for i in 0..gl.get_active_uniforms(p) {
                if let Some(a) = gl.get_active_uniform(p, i)
                    && let Some(l) = gl.get_uniform_location(p, &a.name)
                {
                    u.insert(a.name.trim_end_matches("[0]").to_string(), l);
                }
            }
            Ok(Self { p, u })
        }
    }

    unsafe fn use_(&self, gl: &glow::Context) {
        unsafe { gl.use_program(Some(self.p)) };
    }

    fn at(&self, name: &str) -> Option<&glow::UniformLocation> {
        self.u.get(name)
    }

    unsafe fn f1(&self, gl: &glow::Context, name: &str, v: f32) {
        unsafe { gl.uniform_1_f32(self.at(name), v) };
    }

    unsafe fn f2(&self, gl: &glow::Context, name: &str, v: [f32; 2]) {
        unsafe { gl.uniform_2_f32(self.at(name), v[0], v[1]) };
    }

    unsafe fn f3(&self, gl: &glow::Context, name: &str, v: [f32; 3]) {
        unsafe { gl.uniform_3_f32(self.at(name), v[0], v[1], v[2]) };
    }

    unsafe fn f4(&self, gl: &glow::Context, name: &str, v: [f32; 4]) {
        unsafe { gl.uniform_4_f32(self.at(name), v[0], v[1], v[2], v[3]) };
    }

    unsafe fn i1(&self, gl: &glow::Context, name: &str, v: i32) {
        unsafe { gl.uniform_1_i32(self.at(name), v) };
    }

    unsafe fn m4(&self, gl: &glow::Context, name: &str, m: &Mat4) {
        unsafe { gl.uniform_matrix_4_f32_slice(self.at(name), false, &m.0) };
    }
}

/// A texture and the framebuffer that draws into it.
struct Canvas {
    fbo: glow::Framebuffer,
    tex: glow::Texture,
}

unsafe fn texture(gl: &glow::Context, size: (i32, i32), internal: u32, format: u32, ty: u32, filter: u32) -> Result<glow::Texture, String> {
    unsafe {
        let t = gl.create_texture()?;
        gl.bind_texture(glow::TEXTURE_2D, Some(t));
        gl.tex_image_2d(glow::TEXTURE_2D, 0, internal as i32, size.0, size.1, 0, format, ty, glow::PixelUnpackData::Slice(None));
        for (k, v) in [(glow::TEXTURE_MIN_FILTER, filter), (glow::TEXTURE_MAG_FILTER, filter), (glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE), (glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE)] {
            gl.tex_parameter_i32(glow::TEXTURE_2D, k, v as i32);
        }
        gl.bind_texture(glow::TEXTURE_2D, None);
        Ok(t)
    }
}

/// A framebuffer onto `tex`, as its colour or (`depth`) its depth.
unsafe fn canvas(gl: &glow::Context, tex: glow::Texture, depth: bool) -> Result<Canvas, String> {
    unsafe {
        let fbo = gl.create_framebuffer()?;
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));
        let at = if depth { glow::DEPTH_ATTACHMENT } else { glow::COLOR_ATTACHMENT0 };
        gl.framebuffer_texture_2d(glow::FRAMEBUFFER, at, glow::TEXTURE_2D, Some(tex), 0);
        if depth {
            gl.draw_buffers(&[glow::NONE]);
            gl.read_buffer(glow::NONE);
        }
        let status = gl.check_framebuffer_status(glow::FRAMEBUFFER);
        if status != glow::FRAMEBUFFER_COMPLETE {
            return Err(format!("realistic view: target incomplete (0x{status:x})"));
        }
        Ok(Canvas { fbo, tex })
    }
}

unsafe fn drop_canvas(gl: &glow::Context, c: &Canvas) {
    unsafe {
        gl.delete_framebuffer(c.fbo);
        gl.delete_texture(c.tex);
    }
}

/// What depends on the view's size.
struct Targets {
    size: (i32, i32),
    depth: Canvas,
    ao: Canvas,
    ao_blur: Canvas,
    msaa: glow::Framebuffer,
    msaa_color: glow::Renderbuffer,
    msaa_depth: glow::Renderbuffer,
    hdr: Canvas,
    bloom: [Canvas; 2],
}

impl Targets {
    unsafe fn new(gl: &glow::Context, size: (i32, i32), samples: i32) -> Result<Self, String> {
        unsafe {
            let quarter = ((size.0 / 4).max(1), (size.1 / 4).max(1));
            let depth = canvas(gl, texture(gl, size, glow::DEPTH_COMPONENT24, glow::DEPTH_COMPONENT, glow::UNSIGNED_INT, glow::NEAREST)?, true)?;
            let ao = canvas(gl, texture(gl, size, glow::RGBA8, glow::RGBA, glow::UNSIGNED_BYTE, glow::NEAREST)?, false)?;
            let ao_blur = canvas(gl, texture(gl, size, glow::RGBA8, glow::RGBA, glow::UNSIGNED_BYTE, glow::NEAREST)?, false)?;
            let msaa = gl.create_framebuffer()?;
            let msaa_color = gl.create_renderbuffer()?;
            let msaa_depth = gl.create_renderbuffer()?;
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(msaa));
            for (rb, format, attachment) in [(msaa_color, glow::RGBA16F, glow::COLOR_ATTACHMENT0), (msaa_depth, glow::DEPTH_COMPONENT24, glow::DEPTH_ATTACHMENT)] {
                gl.bind_renderbuffer(glow::RENDERBUFFER, Some(rb));
                gl.renderbuffer_storage_multisample(glow::RENDERBUFFER, samples, format, size.0, size.1);
                gl.framebuffer_renderbuffer(glow::FRAMEBUFFER, attachment, glow::RENDERBUFFER, Some(rb));
            }
            gl.bind_renderbuffer(glow::RENDERBUFFER, None);
            let status = gl.check_framebuffer_status(glow::FRAMEBUFFER);
            let hdr = canvas(gl, texture(gl, size, glow::RGBA16F, glow::RGBA, glow::HALF_FLOAT, glow::LINEAR)?, false)?;
            let bloom = [
                canvas(gl, texture(gl, quarter, glow::RGBA16F, glow::RGBA, glow::HALF_FLOAT, glow::LINEAR)?, false)?,
                canvas(gl, texture(gl, quarter, glow::RGBA16F, glow::RGBA, glow::HALF_FLOAT, glow::LINEAR)?, false)?,
            ];
            let t = Self { size, depth, ao, ao_blur, msaa, msaa_color, msaa_depth, hdr, bloom };
            if status != glow::FRAMEBUFFER_COMPLETE {
                t.delete(gl);
                return Err(format!("realistic view: this context cannot draw in floating point (0x{status:x})"));
            }
            Ok(t)
        }
    }

    unsafe fn delete(&self, gl: &glow::Context) {
        unsafe {
            for c in [&self.depth, &self.ao, &self.ao_blur, &self.hdr, &self.bloom[0], &self.bloom[1]] {
                drop_canvas(gl, c);
            }
            gl.delete_framebuffer(self.msaa);
            gl.delete_renderbuffer(self.msaa_color);
            gl.delete_renderbuffer(self.msaa_depth);
        }
    }
}

pub struct Real {
    depth: Prog,
    surface: Prog,
    wire: Prog,
    sky: Prog,
    ssao: Prog,
    blur_ao: Prog,
    bright: Prog,
    gauss: Prog,
    photo: Prog,
    vao: glow::VertexArray,
    vbo: glow::Buffer,
    wire_vao: glow::VertexArray,
    wire_vbo: glow::Buffer,
    /// For the full-screen passes, which have no attributes.
    empty: glow::VertexArray,
    shadow: Canvas,
    shadow_size: i32,
    targets: Option<Targets>,
    samples: i32,
}

/// The vertices drawn, in the order of the passes: what casts shadows and
/// hides (the scene's opaque triangles, then `rest`: the lid, the floor),
/// the wires (lines), then what one sees through.
struct Batches {
    rest: Vec<Vertex>,
    opaque: i32,
    floor: i32,
    lines: i32,
    glass: i32,
}

fn batches(scene: &Scene) -> Batches {
    let mut rest = Vec::with_capacity(scene.lines.len() + scene.lid.len() + scene.glass.len() + 6);
    // The lid is lacquered wood, not glass.
    rest.extend(scene.lid.iter().map(|v| Vertex { color: [v.color[0], v.color[1], v.color[2], 1.0], ..*v }));
    let opaque = (scene.tris.len() + rest.len()) as i32;
    let (lo, hi) = scene.body;
    let (cx, cy, z) = ((lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0, scene.floor);
    let corner = |x: f32, y: f32| Vertex { pos: [cx + x * FLOOR_R, cy + y * FLOOR_R, z], normal: [0.0, 0.0, 1.0], color: [1.0; 4], mat: FLOOR, width: 0.0 };
    rest.extend([corner(-1.0, -1.0), corner(1.0, -1.0), corner(1.0, 1.0), corner(-1.0, -1.0), corner(1.0, 1.0), corner(-1.0, 1.0)]);
    let floor = 6;
    // The strokes of the drawing are not parts of the instrument: left
    // out.
    let from = rest.len();
    rest.extend(scene.lines.chunks(2).filter(|l| l[0].width > 0.0).flatten());
    let lines = (rest.len() - from) as i32;
    let from = rest.len();
    rest.extend_from_slice(&scene.glass);
    let glass = (rest.len() - from) as i32;
    Batches { rest, opaque, floor, lines, glass }
}

/// The wires' ribbons: two triangles per line.
fn wires(lines: &[Vertex]) -> Vec<WireVertex> {
    let mut out = Vec::with_capacity(lines.len() * 3);
    for l in lines.chunks(2) {
        let dir = sub(l[1].pos, l[0].pos);
        let length = dot(dir, dir).sqrt();
        if length < 1e-7 {
            continue;
        }
        let v = |end: usize, side: f32| WireVertex { pos: l[end].pos, dir, color: l[end].color, mat: l[end].mat, width: l[end].width, at: [side, end as f32 * length] };
        out.extend([v(0, -1.0), v(1, -1.0), v(1, 1.0), v(0, -1.0), v(1, 1.0), v(0, 1.0)]);
    }
    out
}

/// The key light's view: an orthographic box along it that holds the
/// instrument down to the floor. Its matrix, depth range and width (m).
fn key_view(scene: &Scene, key: V3) -> (Mat4, f32, f32) {
    let (mut lo, hi) = scene.body;
    lo[2] = lo[2].min(scene.floor);
    let c = scale(add(lo, hi), 0.5);
    let up = if key[2].abs() > 0.95 { [0.0, 1.0, 0.0] } else { [0.0, 0.0, 1.0] };
    let view = Mat4::look_at(add(c, scale(key, 20.0)), c, up);
    let (mut a, mut b) = ([f32::MAX; 3], [f32::MIN; 3]);
    for i in 0..8 {
        let p = [if i & 1 == 0 { lo[0] } else { hi[0] }, if i & 2 == 0 { lo[1] } else { hi[1] }, if i & 4 == 0 { lo[2] } else { hi[2] }];
        let q = view.project(p);
        for k in 0..3 {
            a[k] = a[k].min(q[k]);
            b[k] = b[k].max(q[k]);
        }
    }
    let half = ((b[0] - a[0]).max(b[1] - a[1]) / 2.0) * 1.02 + 0.02;
    let (mx, my) = ((a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0);
    let (near, far) = (-b[2] - 0.1, -a[2] + 0.1);
    (Mat4::ortho(mx - half, mx + half, my - half, my + half, near, far).mul(&view), far - near, 2.0 * half)
}

/// A mirror in the floor's plane, z = `h`.
fn mirror(h: f32) -> Mat4 {
    Mat4([1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 2.0 * h, 1.0])
}

impl Real {
    pub unsafe fn new(gl: &glow::Context) -> Result<Self, String> {
        let header = egui_glow::ShaderVersion::get(gl).version_declaration();
        let surf = ["a_pos", "a_normal", "a_color", "a_mat"];
        let none: [&str; 0] = [];
        unsafe {
            let lit = |fs: &str| format!("{LIGHTING}{fs}");
            let depth = Prog::new(gl, header, SURFACE_VS, DEPTH_FS, &surf)?;
            let surface = Prog::new(gl, header, SURFACE_VS, &lit(SURFACE_FS), &surf)?;
            let wire = Prog::new(gl, header, WIRE_VS, &lit(WIRE_FS), &["a_pos", "a_dir", "a_color", "a_mat", "a_width", "a_at"])?;
            let sky = Prog::new(gl, header, FULLSCREEN_VS, &lit(SKY_FS), &none)?;
            let ssao = Prog::new(gl, header, FULLSCREEN_VS, SSAO_FS, &none)?;
            let blur_ao = Prog::new(gl, header, FULLSCREEN_VS, BLUR_AO_FS, &none)?;
            let bright = Prog::new(gl, header, FULLSCREEN_VS, BRIGHT_FS, &none)?;
            let gauss = Prog::new(gl, header, FULLSCREEN_VS, GAUSS_FS, &none)?;
            let photo = Prog::new(gl, header, FULLSCREEN_VS, FINAL_FS, &none)?;
            let layout = |gl: &glow::Context, attribs: &[(i32, i32)], stride: usize| -> Result<(glow::VertexArray, glow::Buffer), String> {
                let vao = gl.create_vertex_array()?;
                let vbo = gl.create_buffer()?;
                gl.bind_vertex_array(Some(vao));
                gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
                for (i, &(size, offset)) in attribs.iter().enumerate() {
                    gl.enable_vertex_attrib_array(i as u32);
                    gl.vertex_attrib_pointer_f32(i as u32, size, glow::FLOAT, false, stride as i32, offset);
                }
                gl.bind_vertex_array(None);
                gl.bind_buffer(glow::ARRAY_BUFFER, None);
                Ok((vao, vbo))
            };
            let (vao, vbo) = layout(gl, &[(3, 0), (3, 12), (4, 24), (1, 40)], std::mem::size_of::<Vertex>())?;
            let (wire_vao, wire_vbo) = layout(gl, &[(3, 0), (3, 12), (4, 24), (1, 40), (1, 44), (2, 48)], std::mem::size_of::<WireVertex>())?;
            let empty = gl.create_vertex_array()?;
            let shadow_size = SHADOW.min(gl.get_parameter_i32(glow::MAX_TEXTURE_SIZE));
            let shadow = canvas(gl, texture(gl, (shadow_size, shadow_size), glow::DEPTH_COMPONENT24, glow::DEPTH_COMPONENT, glow::UNSIGNED_INT, glow::NEAREST)?, true)?;
            let samples = gl.get_parameter_i32(glow::MAX_SAMPLES).clamp(1, 8);
            Ok(Self { depth, surface, wire, sky, ssao, blur_ao, bright, gauss, photo, vao, vbo, wire_vao, wire_vbo, empty, shadow, shadow_size, targets: None, samples })
        }
    }

    unsafe fn targets(&mut self, gl: &glow::Context, size: (i32, i32)) -> Result<(), String> {
        if self.targets.as_ref().is_some_and(|t| t.size == size) {
            return Ok(());
        }
        unsafe {
            if let Some(t) = self.targets.take() {
                t.delete(gl);
            }
            self.targets = Some(Targets::new(gl, size, self.samples)?);
        }
        Ok(())
    }

    /// The studio's uniforms, shared by the scene's programs.
    #[allow(clippy::too_many_arguments)]
    unsafe fn studio(&self, gl: &glow::Context, p: &Prog, scene: &Scene, light: &Mat4, range: f32, width: f32, size: (i32, i32)) {
        let (mut c, mut r, mut u, mut h, mut l, mut e) = (vec![], vec![], vec![], vec![], vec![], vec![]);
        for (d, half, rad) in BOXES {
            let cc = norm(d);
            let rr = norm(cross(cc, [0.0, 0.0, 1.0]));
            let uu = cross(rr, cc);
            // The softbox's solid angle, its irradiance square to it.
            let omega = 4.0 * (half[0] * half[1] / (1.0 + half[0] * half[0] + half[1] * half[1]).sqrt()).atan();
            c.extend(cc);
            r.extend(rr);
            u.extend(uu);
            h.extend(half);
            l.extend(rad);
            e.extend(rad.map(|x| x * omega));
        }
        unsafe {
            for (name, v) in [("u_box_c", &c), ("u_box_r", &r), ("u_box_u", &u), ("u_box_l", &l), ("u_box_e", &e)] {
                gl.uniform_3_f32_slice(p.at(name), v);
            }
            gl.uniform_2_f32_slice(p.at("u_box_h"), &h);
            p.m4(gl, "u_light", light);
            p.f3(gl, "u_shadow_info", [1.0 / self.shadow_size as f32, range, width]);
            p.f1(gl, "u_soft", KEY_SOFT);
            p.i1(gl, "u_shadow", 0);
            p.i1(gl, "u_ao", 1);
            p.f2(gl, "u_view", [size.0 as f32, size.1 as f32]);
            // The spruce in the key light (some 3 W/m² from it, as many
            // from all else, through some shade), from the case's front.
            let (lo, hi) = if scene.vertical { ([1.0; 3], [-1.0; 3]) } else { scene.board };
            p.f4(gl, "u_board", [lo[0], lo[1].min(scene.body.0[1]), hi[0], hi[1]]);
            p.f1(gl, "u_board_z", hi[2]);
            p.f3(gl, "u_board_l", [0.45, 0.30, 0.14]);
            p.f1(gl, "u_live", if scene.live { 1.0 } else { 0.0 });
        }
    }

    unsafe fn fullscreen(&self, gl: &glow::Context) {
        unsafe {
            gl.bind_vertex_array(Some(self.empty));
            gl.draw_arrays(glow::TRIANGLES, 0, 3);
        }
    }

    unsafe fn bind(&self, gl: &glow::Context, unit: u32, tex: glow::Texture) {
        unsafe {
            gl.active_texture(glow::TEXTURE0 + unit);
            gl.bind_texture(glow::TEXTURE_2D, Some(tex));
        }
    }

    pub unsafe fn draw(&mut self, gl: &glow::Context, info: &egui::PaintCallbackInfo, out: Option<glow::Framebuffer>, scene: &Scene, cam: &Camera) -> Result<(), String> {
        let vp = info.viewport_in_pixels();
        let clip = info.clip_rect_in_pixels();
        let size = (vp.width_px.max(1), vp.height_px.max(1));
        let b = batches(scene);
        let lines_from = b.rest.len() - (b.lines + b.glass) as usize;
        let wire = wires(&b.rest[lines_from..lines_from + b.lines as usize]);
        let key = norm(BOXES[0].0);
        let (light, range, width) = key_view(scene, key);
        let vp_m = cam.proj.mul(&cam.view);
        let above = cam.eye[2] > scene.floor + 0.01;
        unsafe {
            for _ in 0..8 {
                if gl.get_error() == glow::NO_ERROR {
                    break;
                }
            }
            self.targets(gl, size)?;
            let t = self.targets.as_ref().unwrap();
            gl.disable(glow::SCISSOR_TEST);
            gl.disable(glow::BLEND);
            gl.disable(glow::CULL_FACE);
            gl.enable(glow::DEPTH_TEST);
            gl.depth_func(glow::LEQUAL);
            gl.depth_mask(true);
            gl.bind_vertex_array(Some(self.vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vbo));
            let (tris, rest) = (bytes(&scene.tris), bytes(&b.rest));
            gl.buffer_data_size(glow::ARRAY_BUFFER, (tris.len() + rest.len()) as i32, glow::STREAM_DRAW);
            gl.buffer_sub_data_u8_slice(glow::ARRAY_BUFFER, 0, tris);
            gl.buffer_sub_data_u8_slice(glow::ARRAY_BUFFER, tris.len() as i32, rest);

            // The key light's shadow map: what is opaque, and the wires as
            // lines.
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.shadow.fbo));
            gl.viewport(0, 0, self.shadow_size, self.shadow_size);
            gl.clear(glow::DEPTH_BUFFER_BIT);
            gl.enable(glow::POLYGON_OFFSET_FILL);
            gl.polygon_offset(1.5, 2.0);
            self.depth.use_(gl);
            self.depth.m4(gl, "u_vp", &light);
            gl.draw_arrays(glow::TRIANGLES, 0, b.opaque);
            gl.draw_arrays(glow::LINES, b.opaque + b.floor, b.lines);
            gl.disable(glow::POLYGON_OFFSET_FILL);

            // The view's depth, and the occlusion from it.
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(t.depth.fbo));
            gl.viewport(0, 0, size.0, size.1);
            gl.clear(glow::DEPTH_BUFFER_BIT);
            self.depth.m4(gl, "u_vp", &vp_m);
            gl.draw_arrays(glow::TRIANGLES, 0, b.opaque + if above { b.floor } else { 0 });
            gl.disable(glow::DEPTH_TEST);
            gl.depth_mask(false);
            let proj = [cam.proj.0[0], cam.proj.0[5], cam.near, cam.far];
            let texel = [1.0 / size.0 as f32, 1.0 / size.1 as f32];
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(t.ao.fbo));
            self.ssao.use_(gl);
            self.bind(gl, 0, t.depth.tex);
            self.ssao.i1(gl, "u_depth", 0);
            self.ssao.f4(gl, "u_proj", proj);
            self.ssao.f2(gl, "u_texel", texel);
            self.fullscreen(gl);
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(t.ao_blur.fbo));
            self.blur_ao.use_(gl);
            self.bind(gl, 1, t.ao.tex);
            self.blur_ao.i1(gl, "u_depth", 0);
            self.blur_ao.i1(gl, "u_src", 1);
            self.blur_ao.f4(gl, "u_proj", proj);
            self.blur_ao.f2(gl, "u_texel", texel);
            self.fullscreen(gl);

            // The scene, multisampled.
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(t.msaa));
            gl.clear_color(0.0, 0.0, 0.0, 1.0);
            gl.depth_mask(true);
            gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);
            gl.depth_mask(false);
            self.bind(gl, 0, self.shadow.tex);
            self.bind(gl, 1, t.ao_blur.tex);
            // The room.
            self.sky.use_(gl);
            self.studio(gl, &self.sky, scene, &light, range, width, size);
            let v = &cam.view.0;
            self.sky.f3(gl, "u_cam_r", [v[0], v[4], v[8]]);
            self.sky.f3(gl, "u_cam_u", [v[1], v[5], v[9]]);
            self.sky.f3(gl, "u_cam_f", [-v[2], -v[6], -v[10]]);
            let ty = (FOVY / 2.0).tan();
            self.sky.f2(gl, "u_tan", [ty * cam.aspect, ty]);
            self.sky.f1(gl, "u_above", if above { 1.0 } else { 0.0 });
            self.fullscreen(gl);
            gl.enable(glow::DEPTH_TEST);
            gl.depth_mask(true);
            gl.bind_vertex_array(Some(self.vao));
            self.surface.use_(gl);
            self.studio(gl, &self.surface, scene, &light, range, width, size);
            self.surface.f1(gl, "u_floor", scene.floor);
            if above {
                // The instrument reflected in the floor: seen from the
                // eye's mirror image, shaded as it stands.
                let m = mirror(scene.floor);
                let eye = m.project(cam.eye);
                self.surface.m4(gl, "u_vp", &vp_m.mul(&m));
                self.surface.f3(gl, "u_eye", [eye[0], eye[1], eye[2]]);
                self.surface.f1(gl, "u_mirror", 1.0);
                gl.draw_arrays(glow::TRIANGLES, 0, b.opaque);
            }
            self.surface.m4(gl, "u_vp", &vp_m);
            self.surface.f3(gl, "u_eye", cam.eye);
            self.surface.f1(gl, "u_mirror", 0.0);
            if above {
                gl.enable(glow::BLEND);
                gl.blend_func_separate(glow::ONE, glow::SRC_ALPHA, glow::ZERO, glow::ONE);
                gl.draw_arrays(glow::TRIANGLES, b.opaque, b.floor);
                gl.disable(glow::BLEND);
            }
            gl.draw_arrays(glow::TRIANGLES, 0, b.opaque);
            // The wires, as thick as they are: thinner than their ribbon,
            // blended by the share of it they cover, over what is opaque.
            if !wire.is_empty() {
                gl.bind_vertex_array(Some(self.wire_vao));
                gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.wire_vbo));
                gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytes(&wire), glow::STREAM_DRAW);
                self.wire.use_(gl);
                self.studio(gl, &self.wire, scene, &light, range, width, size);
                self.wire.m4(gl, "u_vp", &vp_m);
                self.wire.f3(gl, "u_eye", cam.eye);
                self.wire.f1(gl, "u_pixel", 2.0 * (FOVY / 2.0).tan() / size.1 as f32);
                self.wire.f1(gl, "u_mirror", 0.0);
                gl.enable(glow::BLEND);
                gl.blend_func_separate(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA, glow::ZERO, glow::ONE);
                gl.depth_mask(false);
                gl.draw_arrays(glow::TRIANGLES, 0, wire.len() as i32);
                gl.depth_mask(true);
                gl.disable(glow::BLEND);
                gl.bind_vertex_array(Some(self.vao));
                gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vbo));
                self.surface.use_(gl);
            }
            // What one sees through (the case's front, as see-through as
            // asked), over the rest.
            if b.glass > 0 {
                gl.enable(glow::BLEND);
                gl.blend_func_separate(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA, glow::ZERO, glow::ONE);
                gl.depth_mask(false);
                gl.draw_arrays(glow::TRIANGLES, b.opaque + b.floor + b.lines, b.glass);
                gl.depth_mask(true);
                gl.disable(glow::BLEND);
            }
            gl.disable(glow::DEPTH_TEST);

            // Resolved, the bloom, the photograph.
            gl.bind_framebuffer(glow::READ_FRAMEBUFFER, Some(t.msaa));
            gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, Some(t.hdr.fbo));
            gl.blit_framebuffer(0, 0, size.0, size.1, 0, 0, size.0, size.1, glow::COLOR_BUFFER_BIT, glow::NEAREST);
            let quarter = ((size.0 / 4).max(1), (size.1 / 4).max(1));
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(t.bloom[0].fbo));
            gl.viewport(0, 0, quarter.0, quarter.1);
            self.bright.use_(gl);
            self.bind(gl, 0, t.hdr.tex);
            self.bright.i1(gl, "u_src", 0);
            self.bright.f2(gl, "u_texel", texel);
            self.fullscreen(gl);
            self.gauss.use_(gl);
            self.gauss.i1(gl, "u_src", 0);
            for (from, to, step) in [(0, 1, [1.5 / quarter.0 as f32, 0.0]), (1, 0, [0.0, 1.5 / quarter.1 as f32])] {
                gl.bind_framebuffer(glow::FRAMEBUFFER, Some(t.bloom[to].fbo));
                self.bind(gl, 0, t.bloom[from].tex);
                self.gauss.f2(gl, "u_step", step);
                self.fullscreen(gl);
            }
            // Into egui's target, clipped as egui clips the view.
            gl.bind_framebuffer(glow::FRAMEBUFFER, out);
            gl.viewport(vp.left_px, vp.from_bottom_px, size.0, size.1);
            gl.enable(glow::SCISSOR_TEST);
            gl.scissor(clip.left_px, clip.from_bottom_px, clip.width_px, clip.height_px);
            self.photo.use_(gl);
            self.bind(gl, 1, t.bloom[0].tex);
            self.bind(gl, 0, t.hdr.tex);
            self.photo.i1(gl, "u_hdr", 0);
            self.photo.i1(gl, "u_bloom", 1);
            self.photo.f1(gl, "u_exposure", EXPOSURE);
            self.fullscreen(gl);
            gl.bind_vertex_array(None);
            gl.bind_buffer(glow::ARRAY_BUFFER, None);
            gl.bind_texture(glow::TEXTURE_2D, None);
            gl.active_texture(glow::TEXTURE1);
            gl.bind_texture(glow::TEXTURE_2D, None);
            gl.active_texture(glow::TEXTURE0);
            let err = gl.get_error();
            if err != glow::NO_ERROR {
                return Err(format!("realistic view: OpenGL error 0x{err:x}"));
            }
        }
        Ok(())
    }
}
