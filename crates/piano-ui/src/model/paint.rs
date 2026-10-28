//! The scene drawn by egui itself: triangles and lines projected, sorted
//! from far to near and shaded by one light. Enough for the plan (an
//! orthographic view of the board's face) and a fallback for the 3D view
//! where there is no GPU drawing.

use egui::epaint::{Mesh, Vertex as EVertex, WHITE_UV};
use egui::{Color32, Pos2, Rect, Shape};

use super::cam::{dot, norm, Mat4, V3};
use super::scene::{Scene, Vertex};

/// Light direction (towards the light), shared with the GPU shader.
pub const LIGHT: V3 = [-0.35, -0.55, 0.76];

/// The shade of a face: ambient plus diffuse, both faces lit.
pub fn shade(normal: V3) -> f32 {
    0.55 + 0.45 * dot(norm(normal), norm(LIGHT)).abs()
}

/// Clip coordinates to the screen in `rect`, with the depth (−1 near … 1
/// far); None behind the eye.
pub fn to_screen(m: &Mat4, rect: Rect, p: V3) -> Option<(Pos2, f32)> {
    let c = m.project(p);
    (c[3] > 1e-6).then(|| {
        let (x, y, z) = (c[0] / c[3], c[1] / c[3], c[2] / c[3]);
        (Pos2::new(rect.left() + (x + 1.0) / 2.0 * rect.width(), rect.top() + (1.0 - y) / 2.0 * rect.height()), z)
    })
}

fn color(v: &Vertex, light: f32) -> Color32 {
    let [r, g, b, a] = v.color;
    let k = |x: f32| (x * light * 255.0).clamp(0.0, 255.0) as u8;
    Color32::from_rgba_unmultiplied(k(r), k(g), k(b), (a * 255.0) as u8)
}

enum Prim<'a> {
    Tri(&'a [Vertex]),
    Line(&'a [Vertex]),
}

/// Draws `scene` seen through `m` into `rect`, behind anything drawn later.
pub fn draw(painter: &egui::Painter, rect: Rect, scene: &Scene, m: &Mat4, line_width: f32) {
    let mut prims: Vec<(f32, Prim)> = Vec::with_capacity((scene.tris.len() + scene.glass.len()) / 3 + scene.lines.len() / 2);
    let depth = |vs: &[Vertex]| -> Option<f32> {
        let mut sum = 0.0;
        for v in vs {
            let c = m.project(v.pos);
            if c[3] <= 1e-6 {
                return None;
            }
            sum += c[2] / c[3];
        }
        Some(sum / vs.len() as f32)
    };
    for t in scene.tris.as_chunks::<3>().0.iter().chain(scene.glass.as_chunks::<3>().0) {
        if let Some(d) = depth(t) {
            prims.push((d, Prim::Tri(t)));
        }
    }
    for l in scene.lines.as_chunks::<2>().0 {
        // Lines just in front of the faces they lie on.
        if let Some(d) = depth(l) {
            prims.push((d - 1e-4, Prim::Line(l)));
        }
    }
    prims.sort_by(|a, b| b.0.total_cmp(&a.0));
    let mut mesh = Mesh::default();
    let mut push = |p: Pos2, c: Color32| {
        mesh.vertices.push(EVertex { pos: p, uv: WHITE_UV, color: c });
    };
    let mut n = 0u32;
    for (_, prim) in &prims {
        match prim {
            Prim::Tri(t) => {
                let Some(pts) = t.iter().map(|v| to_screen(m, rect, v.pos).map(|s| s.0)).collect::<Option<Vec<_>>>() else { continue };
                let light = shade(t[0].normal);
                for (p, v) in pts.into_iter().zip(t.iter()) {
                    push(p, color(v, light));
                }
                mesh.indices.extend([n, n + 1, n + 2]);
                n += 3;
            }
            Prim::Line(l) => {
                let (Some((a, _)), Some((b, _))) = (to_screen(m, rect, l[0].pos), to_screen(m, rect, l[1].pos)) else { continue };
                let d = b - a;
                let len = d.length();
                if len < 1e-3 {
                    continue;
                }
                let side = egui::vec2(-d.y, d.x) / len * line_width / 2.0;
                let (ca, cb) = (color(&l[0], 1.0), color(&l[1], 1.0));
                push(a - side, ca);
                push(a + side, ca);
                push(b + side, cb);
                push(b - side, cb);
                mesh.indices.extend([n, n + 1, n + 2, n, n + 2, n + 3]);
                n += 4;
            }
        }
    }
    painter.add(Shape::mesh(mesh));
}
