//! Strings of a unison coupled through a common bridge (Weinreich, JASA 1977).
//!
//! For one partial k, each string has a vertical and a horizontal
//! polarisation; each is a damped oscillator with complex rate
//! d = −σ + iω (rotating-wave form ζ̇ = d·ζ). The bridge moves under the
//! summed string forces and drives every string end back:
//!
//! ```text
//! ζ̇ = A·ζ,   A = diag(d) − ε_V·1_V·1_Vᵀ − ε_H·1_H·1_Hᵀ − ε_X·(1_V·1_Hᵀ + 1_H·1_Vᵀ)
//! ```
//!
//! with ε_V = (T/L)·Y the vertical coupling (Re ε_V is one string's bridge
//! loss), ε_H the weaker horizontal one and ε_X the cross term (bridge
//! rocking, oblique terminations) that transfers energy between
//! polarisations. A is complex symmetric, so the eigenvectors can be
//! normalised to Vᵀ·V = I and V⁻¹ = Vᵀ: the coupled system is solved once
//! per note and the modal bank runs its coupled modes. In-phase vertical
//! motion loads the bridge (fast "prompt" decay); out-of-phase and
//! horizontal motion barely do (slow "aftersound"); detuning larger than
//! |ε| gives beating.

// Dense small-matrix code reads better with explicit indices.
#![allow(clippy::needless_range_loop)]

use crate::complex::C64;

/// Up to 3 strings × 2 polarisations.
pub const MAX_OSC: usize = 6;

/// Coupled modes of one partial: eigenvalues λ_m and eigenvectors
/// (`v[j][m]` = component of oscillator j in mode m), normalised Vᵀ·V = I.
#[derive(Clone, Copy, Debug, Default)]
pub struct Cluster {
    pub n: usize,
    pub lam: [C64; MAX_OSC],
    pub v: [[C64; MAX_OSC]; MAX_OSC],
}

impl Cluster {
    /// Coupled amplitudes c = Vᵀ·ζ from oscillator amplitudes ζ.
    pub fn to_modes(&self, zeta: &[C64]) -> [C64; MAX_OSC] {
        let mut c = [C64::ZERO; MAX_OSC];
        for (m, cm) in c.iter_mut().enumerate().take(self.n) {
            for (j, &z) in zeta.iter().enumerate().take(self.n) {
                *cm += self.v[j][m] * z;
            }
        }
        c
    }

    /// Oscillator amplitudes ζ = V·c from coupled amplitudes c.
    pub fn to_strings(&self, c: &[C64]) -> [C64; MAX_OSC] {
        let mut z = [C64::ZERO; MAX_OSC];
        for (j, zj) in z.iter_mut().enumerate().take(self.n) {
            for (m, &cm) in c.iter().enumerate().take(self.n) {
                *zj += self.v[j][m] * cm;
            }
        }
        z
    }

    /// Output of mode m for per-oscillator output weights: Σ_j w_j·V_jm.
    pub fn output_weight(&self, m: usize, w: &[f64]) -> C64 {
        let mut s = C64::ZERO;
        for (j, &wj) in w.iter().enumerate().take(self.n) {
            s += self.v[j][m].scale(wj);
        }
        s
    }

    /// Bridge participation with unit weights on every oscillator.
    pub fn bridge_weight(&self, m: usize) -> C64 {
        self.output_weight(m, &[1.0; MAX_OSC])
    }
}

/// N strings, vertical polarisation only: diag(d) − ε·1·1ᵀ.
pub fn couple(d: &[C64], eps: C64) -> Cluster {
    let n = d.len();
    let mut a = [[C64::ZERO; MAX_OSC]; MAX_OSC];
    for (i, row) in a.iter_mut().enumerate().take(n) {
        row[..n].fill(-eps);
        row[i] = d[i] - eps;
    }
    eigen_symmetric(&a, n)
}

/// N strings with both polarisations. Oscillators 0..N are vertical,
/// N..2N horizontal (same string order).
pub fn couple_polarised(dv: &[C64], dh: &[C64], eps_v: C64, eps_h: C64, eps_x: C64) -> Cluster {
    let n = dv.len();
    assert_eq!(dh.len(), n);
    let mut a = [[C64::ZERO; MAX_OSC]; MAX_OSC];
    for (i, row) in a.iter_mut().enumerate().take(2 * n) {
        for (j, x) in row.iter_mut().enumerate().take(2 * n) {
            *x = -match (i < n, j < n) {
                (true, true) => eps_v,
                (false, false) => eps_h,
                _ => eps_x,
            };
        }
    }
    for j in 0..n {
        a[j][j] += dv[j];
        a[n + j][n + j] += dh[j];
    }
    eigen_symmetric(&a, 2 * n)
}

/// Eigen-decomposition of a complex symmetric n×n matrix (n ≤ 6) with
/// distinct eigenvalues: characteristic polynomial of the centred matrix
/// (Faddeev–LeVerrier), Durand–Kerner roots, then inverse-iteration and
/// Rayleigh-quotient refinement of each pair. No allocation.
pub fn eigen_symmetric(a: &[[C64; MAX_OSC]; MAX_OSC], n: usize) -> Cluster {
    assert!((1..=MAX_OSC).contains(&n));
    let mut cl = Cluster { n, ..Default::default() };
    if n == 1 {
        cl.lam[0] = a[0][0];
        cl.v[0][0] = C64::ONE;
        return cl;
    }
    // Centre: eigenvalues are ~1e4 rad/s from 0 but ~1e-2 apart.
    let mut mu = C64::ZERO;
    for (i, row) in a.iter().enumerate().take(n) {
        mu += row[i];
    }
    let mu = mu.scale(1.0 / n as f64);
    let mut b = *a;
    for (i, row) in b.iter_mut().enumerate().take(n) {
        row[i] = row[i] - mu;
    }
    let mut scale = 0.0f64;
    for row in b.iter().take(n) {
        for x in row.iter().take(n) {
            scale = scale.max(x.norm());
        }
    }
    let scale = scale.max(1e-300);

    let roots = poly_roots(&b, n, scale);
    for (m, &r) in roots.iter().enumerate().take(n) {
        let (lam, v) = refine(&b, n, r, scale);
        cl.lam[m] = lam + mu;
        for j in 0..n {
            cl.v[j][m] = v[j];
        }
    }
    cl
}

/// Roots of det(λI − B) via Faddeev–LeVerrier and Durand–Kerner.
fn poly_roots(b: &[[C64; MAX_OSC]; MAX_OSC], n: usize, scale: f64) -> [C64; MAX_OSC] {
    // c[i] coefficient of λ^i, monic of degree n.
    let mut c = [C64::ZERO; MAX_OSC + 1];
    c[n] = C64::ONE;
    let mut mk = [[C64::ZERO; MAX_OSC]; MAX_OSC];
    for k in 1..=n {
        // M_k = B·M_{k−1} + c_{n−k+1}·I
        let mut next = [[C64::ZERO; MAX_OSC]; MAX_OSC];
        for i in 0..n {
            for j in 0..n {
                let mut s = C64::ZERO;
                for l in 0..n {
                    s += b[i][l] * mk[l][j];
                }
                next[i][j] = s;
            }
            next[i][i] += c[n - k + 1];
        }
        mk = next;
        // c_{n−k} = −tr(B·M_k)/k
        let mut tr = C64::ZERO;
        for i in 0..n {
            for l in 0..n {
                tr += b[i][l] * mk[l][i];
            }
        }
        c[n - k] = tr.scale(-1.0 / k as f64);
    }
    let p = |z: C64| {
        let mut acc = C64::ZERO;
        for i in (0..=n).rev() {
            acc = acc * z + c[i];
        }
        acc
    };
    // Start from the diagonal (weak coupling) with small distinct offsets.
    let mut z = [C64::ZERO; MAX_OSC];
    for (i, zi) in z.iter_mut().enumerate().take(n) {
        let ang = 0.7 + 1.9 * i as f64;
        *zi = b[i][i] + C64::polar(1e-3 * scale * (1.0 + i as f64), ang);
    }
    for _ in 0..500 {
        let mut moved = 0.0f64;
        for i in 0..n {
            let mut den = C64::ONE;
            for j in 0..n {
                if j != i {
                    den = den * (z[i] - z[j]);
                }
            }
            if den.norm_sqr() == 0.0 {
                z[i] += C64::new(1e-9 * scale, 1e-9 * scale);
                continue;
            }
            let step = p(z[i]) / den;
            z[i] = z[i] - step;
            moved = moved.max(step.norm());
        }
        if moved <= 1e-14 * scale {
            break;
        }
    }
    z
}

/// Inverse iteration around `lam0`, then the bilinear Rayleigh quotient;
/// returns the eigenvalue and the eigenvector normalised so that vᵀv = 1.
fn refine(b: &[[C64; MAX_OSC]; MAX_OSC], n: usize, lam0: C64, scale: f64) -> (C64, [C64; MAX_OSC]) {
    let mut lam = lam0;
    let mut x = [C64::ONE; MAX_OSC];
    for (i, xi) in x.iter_mut().enumerate().take(n) {
        // Favour the oscillator closest to λ: good start under weak coupling.
        let d = (b[i][i] - lam0).norm() + 1e-6 * scale;
        *xi = C64::new(1.0 + 1e-3 * i as f64, 0.0).scale(1.0 / d);
    }
    for _ in 0..4 {
        // Solve (B − λ·I − δ)y = x; δ keeps the system solvable at convergence.
        let shift = lam + C64::new(1e-11 * scale, 1e-11 * scale);
        let mut m = *b;
        for (i, row) in m.iter_mut().enumerate().take(n) {
            row[i] = row[i] - shift;
        }
        let y = solve(m, x, n);
        let mut norm = 0.0;
        for yi in y.iter().take(n) {
            norm += yi.norm_sqr();
        }
        let norm = norm.sqrt();
        if !(norm.is_finite() && norm > 0.0) {
            break;
        }
        for i in 0..n {
            x[i] = y[i].scale(1.0 / norm);
        }
        // Rayleigh quotient for complex symmetric matrices: xᵀBx / xᵀx.
        let (mut num, mut den) = (C64::ZERO, C64::ZERO);
        for i in 0..n {
            let mut bx = C64::ZERO;
            for j in 0..n {
                bx += b[i][j] * x[j];
            }
            num += x[i] * bx;
            den += x[i] * x[i];
        }
        if den.norm() > 1e-12 {
            lam = num / den;
        }
    }
    let mut s = C64::ZERO;
    for xi in x.iter().take(n) {
        s += *xi * *xi;
    }
    let s = s.sqrt();
    for xi in x.iter_mut().take(n) {
        *xi = *xi / s;
    }
    (lam, x)
}

/// Gaussian elimination with partial pivoting (n ≤ 6).
fn solve(mut m: [[C64; MAX_OSC]; MAX_OSC], mut rhs: [C64; MAX_OSC], n: usize) -> [C64; MAX_OSC] {
    for col in 0..n {
        let mut piv = col;
        for r in col + 1..n {
            if m[r][col].norm_sqr() > m[piv][col].norm_sqr() {
                piv = r;
            }
        }
        m.swap(col, piv);
        rhs.swap(col, piv);
        let d = m[col][col];
        if d.norm_sqr() == 0.0 {
            continue;
        }
        for r in col + 1..n {
            let f = m[r][col] / d;
            for k in col..n {
                m[r][k] = m[r][k] - f * m[col][k];
            }
            rhs[r] = rhs[r] - f * rhs[col];
        }
    }
    let mut x = [C64::ZERO; MAX_OSC];
    for r in (0..n).rev() {
        let mut s = rhs[r];
        for k in r + 1..n {
            s = s - m[r][k] * x[k];
        }
        x[r] = if m[r][r].norm_sqr() == 0.0 { C64::ZERO } else { s / m[r][r] };
    }
    x
}

#[cfg(test)]
mod tests {
    use super::*;

    fn verify(a: &[[C64; MAX_OSC]; MAX_OSC], cl: &Cluster) {
        let n = cl.n;
        for m in 0..n {
            let mut err = 0.0f64;
            for i in 0..n {
                let mut av = C64::ZERO;
                for j in 0..n {
                    av += a[i][j] * cl.v[j][m];
                }
                err = err.max((av - cl.lam[m] * cl.v[i][m]).norm());
            }
            assert!(err < 1e-8, "residual {err} for mode {m}: {cl:?}");
            for m2 in 0..n {
                let mut s = C64::ZERO;
                for j in 0..n {
                    s += cl.v[j][m] * cl.v[j][m2];
                }
                let want = if m == m2 { 1.0 } else { 0.0 };
                assert!((s - C64::new(want, 0.0)).norm() < 1e-8, "VᵀV[{m}][{m2}] = {s:?}");
            }
        }
        let zeta = [C64::new(1.0, 0.2), C64::new(-0.3, 0.5), C64::new(0.7, -0.1), C64::new(0.1, 0.1), C64::new(0.0, -1.0), C64::new(2.0, 0.0)];
        let back = cl.to_strings(&cl.to_modes(&zeta[..n]));
        for j in 0..n {
            assert!((back[j] - zeta[j]).norm() < 1e-8);
        }
    }

    fn matrix(d: &[C64], eps: C64) -> [[C64; MAX_OSC]; MAX_OSC] {
        let mut a = [[C64::ZERO; MAX_OSC]; MAX_OSC];
        for i in 0..d.len() {
            for j in 0..d.len() {
                a[i][j] = -eps;
            }
            a[i][i] = d[i] - eps;
        }
        a
    }

    fn w(hz: f64) -> f64 {
        std::f64::consts::TAU * hz
    }

    #[test]
    fn nearly_identical_strings_split_into_prompt_and_aftersound() {
        // Two strings 0.01 Hz apart, bridge loss 1/s per string: in-phase mode
        // decays at σ0 + 2·Re ε, out-of-phase at ≈ σ0, and it barely radiates.
        let s0 = 0.3;
        let d = [C64::new(-s0, w(220.0)), C64::new(-s0, w(220.01))];
        let cl = couple(&d, C64::new(1.0, 0.0));
        verify(&matrix(&d, C64::new(1.0, 0.0)), &cl);
        let mut sig: Vec<f64> = cl.lam[..2].iter().map(|l| -l.re).collect();
        sig.sort_by(f64::total_cmp);
        assert!((sig[0] - s0).abs() < 0.01 && (sig[1] - (s0 + 2.0)).abs() < 0.01, "{sig:?}");
        let slow = if -cl.lam[0].re < -cl.lam[1].re { 0 } else { 1 };
        assert!(cl.bridge_weight(slow).norm() < 0.05, "aftersound radiates {:?}", cl.bridge_weight(slow));
    }

    #[test]
    fn detuned_strings_beat_with_shared_damping() {
        // 2 Hz apart ≫ |ε|: each string keeps its frequency, damping σ0 + Re ε.
        let d = [C64::new(-0.3, w(440.0)), C64::new(-0.3, w(442.0))];
        let cl = couple(&d, C64::new(0.5, 0.0));
        verify(&matrix(&d, C64::new(0.5, 0.0)), &cl);
        for l in &cl.lam[..2] {
            assert!((-l.re - 0.8).abs() < 0.02, "{l:?}");
        }
    }

    #[test]
    fn trichords_diagonalise() {
        for (sp, e) in [(0.0005, 1.0), (0.3, 0.8), (2.0, 0.5), (0.05, 3.0)] {
            let d = [C64::new(-0.5, w(261.6)), C64::new(-0.52, w(261.6 + sp * 0.37)), C64::new(-0.49, w(261.6 + sp))];
            let eps = C64::new(e, 0.3 * e);
            verify(&matrix(&d, eps), &couple(&d, eps));
        }
        let cl = couple(&[C64::new(-0.2, w(30.0))], C64::new(0.4, 0.0));
        assert!((cl.lam[0].re + 0.6).abs() < 1e-12);
    }

    #[test]
    fn polarised_trichord_and_single_string() {
        let dv = [C64::new(-0.5, w(261.6)), C64::new(-0.5, w(261.65)), C64::new(-0.5, w(261.7))];
        let dh: Vec<C64> = dv.iter().map(|d| *d + C64::new(0.0, w(0.04))).collect();
        let (ev, eh, ex) = (C64::new(0.6, 0.9), C64::new(0.06, 0.09), C64::new(0.1, 0.15));
        let cl = couple_polarised(&dv, &dh, ev, eh, ex);
        let mut a = [[C64::ZERO; MAX_OSC]; MAX_OSC];
        for i in 0..6 {
            for j in 0..6 {
                a[i][j] = -match (i < 3, j < 3) {
                    (true, true) => ev,
                    (false, false) => eh,
                    _ => ex,
                };
            }
        }
        for j in 0..3 {
            a[j][j] += dv[j];
            a[3 + j][3 + j] += dh[j];
        }
        verify(&a, &cl);
        // A single string gets an aftersound from its horizontal polarisation.
        let cl1 = couple_polarised(&[C64::new(-0.2, w(27.5))], &[C64::new(-0.2, w(27.501))], ev, eh, ex);
        let mut sig: Vec<f64> = cl1.lam[..2].iter().map(|l| -l.re).collect();
        sig.sort_by(f64::total_cmp);
        assert!(sig[1] > 2.0 * sig[0], "{sig:?}");
    }
}
