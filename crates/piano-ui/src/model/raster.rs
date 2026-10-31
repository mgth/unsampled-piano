//! A software rendering of the scene for visual checks without a window:
//! `PIANO_SHOT_DIR=<dir> cargo test -p piano-ui --lib shots -- --ignored`
//! writes `<dir>/<name>.ppm` for each view in `PIANO_SHOTS`
//! (`name:yaw,pitch,dist,x,y,z[,key-down]` separated by `;`, the camera
//! of [`Orbit`]; `key-down` presses that key, its hammer on the strings;
//! `case` 1 draws the case; `cut` leaves out what lies across below it, a
//! section).

use super::cam::{Mat4, Orbit};
use super::paint::shade;
use super::scene::{self, Live, Options, Scene};
use egui::Color32;
use piano_core::params::PianoModel;
use piano_core::Observation;

/// Draws `scene` through `m` into a `w` × `h` image, 2× supersampled.
pub fn render(scene: &Scene, m: &Mat4, w: usize, h: usize) -> Vec<u8> {
    let (sw, sh) = (2 * w, 2 * h);
    let mut col = vec![[24.0f32, 24.0, 28.0]; sw * sh];
    let mut depth = vec![f32::INFINITY; sw * sh];
    let screen = |p: [f32; 3]| {
        let c = m.project(p);
        (c[3] > 1e-6).then(|| [(c[0] / c[3] + 1.0) / 2.0 * sw as f32, (1.0 - c[1] / c[3]) / 2.0 * sh as f32, c[2] / c[3]])
    };
    let mut tri = |v: &[scene::Vertex], alpha: bool| {
        let Some(p) = v.iter().map(|v| screen(v.pos)).collect::<Option<Vec<_>>>() else { return };
        let c: Vec<[f32; 4]> = v.iter().map(|v| {
            let l = shade(v.normal);
            [v.color[0] * l * 255.0, v.color[1] * l * 255.0, v.color[2] * l * 255.0, v.color[3]]
        }).collect();
        let area = (p[1][0] - p[0][0]) * (p[2][1] - p[0][1]) - (p[2][0] - p[0][0]) * (p[1][1] - p[0][1]);
        if area.abs() < 1e-9 {
            return;
        }
        let (x0, x1) = (p.iter().map(|q| q[0]).fold(f32::MAX, f32::min).max(0.0) as usize, (p.iter().map(|q| q[0]).fold(f32::MIN, f32::max).min(sw as f32 - 1.0)) as usize);
        let (y0, y1) = (p.iter().map(|q| q[1]).fold(f32::MAX, f32::min).max(0.0) as usize, (p.iter().map(|q| q[1]).fold(f32::MIN, f32::max).min(sh as f32 - 1.0)) as usize);
        for y in y0..=y1 {
            for x in x0..=x1 {
                let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
                let e = |a: usize, b: usize| (p[b][0] - p[a][0]) * (py - p[a][1]) - (p[b][1] - p[a][1]) * (px - p[a][0]);
                let (w0, w1, w2) = (e(1, 2) / area, e(2, 0) / area, e(0, 1) / area);
                if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                    continue;
                }
                let z = w0 * p[0][2] + w1 * p[1][2] + w2 * p[2][2];
                let i = y * sw + x;
                if z >= depth[i] {
                    continue;
                }
                let k = |j: usize| w0 * c[0][j] + w1 * c[1][j] + w2 * c[2][j];
                if alpha {
                    let a = k(3);
                    for (j, v) in col[i].iter_mut().enumerate() {
                        *v = *v * (1.0 - a) + k(j) * a;
                    }
                } else {
                    depth[i] = z;
                    col[i] = [k(0), k(1), k(2)];
                }
            }
        }
    };
    for t in scene.tris.chunks(3) {
        tri(t, false);
    }
    for t in scene.glass.chunks(3) {
        tri(t, true);
    }
    for l in scene.lines.chunks(2) {
        let (Some(a), Some(b)) = (screen(l[0].pos), screen(l[1].pos)) else { continue };
        let n = ((b[0] - a[0]).abs().max((b[1] - a[1]).abs()).ceil() as usize).max(1);
        for s in 0..=n {
            let t = s as f32 / n as f32;
            let (x, y, z) = (a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t - 1e-5);
            if x < 0.0 || y < 0.0 || x >= sw as f32 || y >= sh as f32 {
                continue;
            }
            let i = y as usize * sw + x as usize;
            if z < depth[i] {
                depth[i] = z;
                col[i] = [l[0].color[0] * 255.0, l[0].color[1] * 255.0, l[0].color[2] * 255.0];
            }
        }
    }
    let mut out = format!("P6 {w} {h} 255\n").into_bytes();
    for y in 0..h {
        for x in 0..w {
            let at = |dx: usize, dy: usize| col[(2 * y + dy) * sw + 2 * x + dx];
            let (a, b, c, d) = (at(0, 0), at(1, 0), at(0, 1), at(1, 1));
            out.extend((0..3).map(|j| ((a[j] + b[j] + c[j] + d[j]) / 4.0).clamp(0.0, 255.0) as u8));
        }
    }
    out
}

#[test]
#[ignore]
fn shots() {
    let dir = std::env::var("PIANO_SHOT_DIR").expect("PIANO_SHOT_DIR");
    let mut model = PianoModel::default();
    if let Some(sb) = model.soundboard.as_mut() {
        sb.vertical = std::env::var("PIANO_SHOT_UPRIGHT").is_ok();
        if let Some(a) = std::env::var("PIANO_SHOT_SLANT").ok().and_then(|a| a.parse().ok()) {
            sb.main_string_slant = a;
        }
    }
    let options = Options { key: 0, lit: &[], lid: false, frame: false, mics: false, case: false, ink: Color32::WHITE };
    for view in std::env::var("PIANO_SHOTS").unwrap_or_default().split(';').filter(|s| !s.is_empty()) {
        let (name, v) = view.split_once(':').unwrap();
        let v: Vec<f32> = v.split(',').map(|x| x.parse().unwrap()).collect();
        let mut obs = Observation::default();
        let levels = [0.0f32; 128];
        if let Some(&k) = v.get(6).filter(|&&k| k > 0.0) {
            let k = &mut obs.keys[k as usize];
            (k.down, k.hammer, k.damper) = (true, true, 0.0);
        }
        let live = Live { obs: &obs, keys: &levels, board: None, time: 0.0 };
        let t = std::time::Instant::now();
        let mut scene = scene::build(&model, Some(&live), &Options { case: v.get(7).is_some_and(|&c| c > 0.0), ..options });
        if let Some(&cut) = v.get(8) {
            let keep = |t: &[scene::Vertex]| t.iter().any(|v| v.pos[0] >= cut);
            scene.tris = scene.tris.chunks(3).filter(|t| keep(t)).flatten().copied().collect();
            scene.lines = scene.lines.chunks(2).filter(|t| keep(t)).flatten().copied().collect();
        }
        eprintln!("{name}: {} triangles in {:?}", scene.tris.len() / 3, t.elapsed());
        let orbit = Orbit { yaw: v[0], pitch: v[1], dist: v[2], target: [v[3], v[4], v[5]] };
        let (w, h) = (1200, 800);
        // A negative distance: orthographic, that many metres high.
        let m = if orbit.dist < 0.0 {
            let (k, a) = (-orbit.dist / 2.0, w as f32 / h as f32);
            let eye = Orbit { dist: 5.0, ..orbit }.eye();
            Mat4::ortho(-k * a, k * a, -k, k, 0.1, 10.0).mul(&Mat4::look_at(eye, orbit.target, [0.0, 0.0, 1.0]))
        } else {
            orbit.matrix(w as f32 / h as f32)
        };
        std::fs::write(format!("{dir}/{name}.ppm"), render(&scene, &m, w, h)).unwrap();
    }
}

#[test]
#[ignore]
fn build_time() {
    let model = PianoModel::default();
    let options = Options { key: 0, lit: &[], lid: true, frame: true, mics: true, case: std::env::var("CASE_OFF").is_err(), ink: Color32::WHITE };
    let obs = Observation::default();
    let levels = [0.0f32; 128];
    let live = Live { obs: &obs, keys: &levels, board: None, time: 0.0 };
    let n = 200;
    let t = std::time::Instant::now();
    let mut tris = 0;
    for _ in 0..n {
        tris = scene::build(&model, Some(&live), &options).tris.len() / 3;
    }
    eprintln!("{tris} triangles, {:?} a build", t.elapsed() / n);
}
