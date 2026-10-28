//! The little linear algebra the views need: 3-vectors, column-major 4×4
//! matrices (OpenGL's layout) and the cameras.

pub type V3 = [f32; 3];

pub fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

pub fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

pub fn scale(a: V3, s: f32) -> V3 {
    [a[0] * s, a[1] * s, a[2] * s]
}

pub fn dot(a: V3, b: V3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

pub fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

pub fn norm(a: V3) -> V3 {
    let l = dot(a, a).sqrt();
    if l > 0.0 { scale(a, 1.0 / l) } else { a }
}

/// Column-major: element (row r, column c) at `c * 4 + r`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mat4(pub [f32; 16]);

impl Mat4 {
    pub fn mul(&self, o: &Mat4) -> Mat4 {
        let (a, b) = (&self.0, &o.0);
        Mat4(std::array::from_fn(|i| {
            let (c, r) = (i / 4, i % 4);
            (0..4).map(|k| a[k * 4 + r] * b[c * 4 + k]).sum()
        }))
    }

    /// The point `p` in clip coordinates (x, y, z, w).
    pub fn project(&self, p: V3) -> [f32; 4] {
        let m = &self.0;
        std::array::from_fn(|r| m[r] * p[0] + m[4 + r] * p[1] + m[8 + r] * p[2] + m[12 + r])
    }

    pub fn look_at(eye: V3, target: V3, up: V3) -> Mat4 {
        let f = norm(sub(target, eye));
        let s = norm(cross(f, up));
        let u = cross(s, f);
        Mat4([s[0], u[0], -f[0], 0.0, s[1], u[1], -f[1], 0.0, s[2], u[2], -f[2], 0.0, -dot(s, eye), -dot(u, eye), dot(f, eye), 1.0])
    }

    pub fn perspective(fovy: f32, aspect: f32, near: f32, far: f32) -> Mat4 {
        let f = 1.0 / (fovy / 2.0).tan();
        let d = near - far;
        Mat4([f / aspect, 0.0, 0.0, 0.0, 0.0, f, 0.0, 0.0, 0.0, 0.0, (far + near) / d, -1.0, 0.0, 0.0, 2.0 * far * near / d, 0.0])
    }

    pub fn ortho(l: f32, r: f32, b: f32, t: f32, n: f32, f: f32) -> Mat4 {
        Mat4([
            2.0 / (r - l),
            0.0,
            0.0,
            0.0,
            0.0,
            2.0 / (t - b),
            0.0,
            0.0,
            0.0,
            0.0,
            -2.0 / (f - n),
            0.0,
            -(r + l) / (r - l),
            -(t + b) / (t - b),
            -(f + n) / (f - n),
            1.0,
        ])
    }
}

/// A camera turning around the instrument: drag to turn, scroll to come
/// closer, right-drag to move the point looked at.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Orbit {
    /// Around the vertical (0: from the pianist's seat) and above the
    /// horizontal (radians).
    pub yaw: f32,
    pub pitch: f32,
    /// Distance to the target (m).
    pub dist: f32,
    pub target: V3,
}

impl Orbit {
    /// Three-quarter view from the pianist's left, looking at the centre of
    /// `min`–`max` from far enough to see all of it.
    pub fn framing(min: V3, max: V3) -> Self {
        let size = dot(sub(max, min), sub(max, min)).sqrt();
        Self { yaw: -0.6, pitch: 0.55, dist: 1.25 * size.max(0.5), target: scale(add(min, max), 0.5) }
    }

    pub fn eye(&self) -> V3 {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        add(self.target, scale([sy * cp, -cy * cp, sp], self.dist))
    }

    pub fn matrix(&self, aspect: f32) -> Mat4 {
        let proj = Mat4::perspective(0.6, aspect, 0.02 * self.dist, 20.0 * self.dist);
        proj.mul(&Mat4::look_at(self.eye(), self.target, [0.0, 0.0, 1.0]))
    }

    /// Moves the target by a drag of (dx, dy) points on a view `height` high.
    pub fn pan(&mut self, dx: f32, dy: f32, height: f32) {
        let f = norm(sub(self.target, self.eye()));
        let s = norm(cross(f, [0.0, 0.0, 1.0]));
        let u = cross(s, f);
        let k = 2.0 * self.dist * (0.3f32).tan() / height.max(1.0);
        self.target = add(self.target, add(scale(s, -dx * k), scale(u, dy * k)));
    }
}

/// An orthographic view of the board's face (from above on a grand, from
/// the front on an upright), fitting `min`–`max` into `aspect`.
pub fn face_view(min: V3, max: V3, vertical: bool, aspect: f32) -> Mat4 {
    let c = scale(add(min, max), 0.5);
    // Screen axes: x across the keyboard, y away from the pianist (grand)
    // or up (upright); depth along the third axis.
    let (w, h, depth) = if vertical { (max[0] - min[0], max[2] - min[2], max[1] - min[1]) } else { (max[0] - min[0], max[1] - min[1], max[2] - min[2]) };
    let (w, h) = (w * 1.04 + 0.02, h * 1.04 + 0.02);
    let (w, h) = if w / h > aspect { (w, w / aspect) } else { (h * aspect, h) };
    let far = depth + 4.0;
    let view = if vertical {
        Mat4::look_at(sub(c, [0.0, far / 2.0, 0.0]), c, [0.0, 0.0, 1.0])
    } else {
        Mat4::look_at(add(c, [0.0, 0.0, far / 2.0]), c, [0.0, 1.0, 0.0])
    };
    Mat4::ortho(-w / 2.0, w / 2.0, -h / 2.0, h / 2.0, 0.0, far).mul(&view)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn top_view_keeps_the_pianist_at_the_bottom() {
        let m = face_view([-1.0, -0.2, -0.3], [0.0, 1.8, 0.3], false, 1.0);
        let near = m.project([-0.5, -0.2, 0.0]);
        let far = m.project([-0.5, 1.8, 0.0]);
        assert!(near[1] < far[1], "{near:?} {far:?}");
        // Treble (x = 0) on the right of the bass (x = -1).
        assert!(m.project([0.0, 0.5, 0.0])[0] > m.project([-1.0, 0.5, 0.0])[0]);
        // Higher parts nearer (smaller depth).
        assert!(m.project([-0.5, 0.5, 0.2])[2] < m.project([-0.5, 0.5, -0.2])[2]);
    }

    #[test]
    fn orbit_sees_its_target() {
        let o = Orbit::framing([-1.0, -0.2, -0.3], [0.0, 1.8, 0.3]);
        let c = o.matrix(1.5).project(o.target);
        assert!(c[3] > 0.0 && (c[0] / c[3]).abs() < 1e-4 && (c[1] / c[3]).abs() < 1e-4 && (c[2] / c[3]).abs() < 1.0, "{c:?}");
    }
}
