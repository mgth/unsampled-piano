//! Eigen-decomposition of a small dense symmetric matrix (cyclic Jacobi),
//! for building the instrument (not real-time).

/// Eigenvalues (descending) and eigenvectors (`vecs[k]` the k-th, unit
/// length) of the symmetric `n`×`n` matrix `a` (row-major).
pub fn symmetric(a: &[f64], n: usize) -> (Vec<f64>, Vec<Vec<f64>>) {
    assert_eq!(a.len(), n * n);
    let mut m = a.to_vec();
    let mut v = vec![0.0; n * n];
    for i in 0..n {
        v[i * n + i] = 1.0;
    }
    let norm: f64 = m.iter().map(|x| x * x).sum::<f64>().sqrt().max(f64::MIN_POSITIVE);
    for _sweep in 0..100 {
        let off: f64 = (0..n).flat_map(|i| (0..n).filter(move |&j| j != i).map(move |j| (i, j))).map(|(i, j)| m[i * n + j].powi(2)).sum();
        if off.sqrt() <= 1e-14 * norm {
            break;
        }
        for p in 0..n {
            for q in p + 1..n {
                let apq = m[p * n + q];
                if apq.abs() <= 1e-300 {
                    continue;
                }
                let theta = (m[q * n + q] - m[p * n + p]) / (2.0 * apq);
                let t = theta.signum().max(0.0) * 2.0 - 1.0;
                let t = t / (theta.abs() + (theta * theta + 1.0).sqrt());
                let c = 1.0 / (t * t + 1.0).sqrt();
                let s = t * c;
                for k in 0..n {
                    let (mkp, mkq) = (m[k * n + p], m[k * n + q]);
                    m[k * n + p] = c * mkp - s * mkq;
                    m[k * n + q] = s * mkp + c * mkq;
                }
                for k in 0..n {
                    let (mpk, mqk) = (m[p * n + k], m[q * n + k]);
                    m[p * n + k] = c * mpk - s * mqk;
                    m[q * n + k] = s * mpk + c * mqk;
                }
                for k in 0..n {
                    let (vkp, vkq) = (v[k * n + p], v[k * n + q]);
                    v[k * n + p] = c * vkp - s * vkq;
                    v[k * n + q] = s * vkp + c * vkq;
                }
            }
        }
    }
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&i, &j| m[j * n + j].total_cmp(&m[i * n + i]));
    let vals = order.iter().map(|&i| m[i * n + i]).collect();
    let vecs = order.iter().map(|&i| (0..n).map(|k| v[k * n + i]).collect()).collect();
    (vals, vecs)
}

/// Eigenvalues (ascending) and eigenvectors (`vecs[k]` the k-th, unit
/// length) of the symmetric `n`×`n` matrix `a` (row-major): Householder
/// reduction to a tridiagonal matrix, then implicit QL (EISPACK tred2 and
/// tql2, as in JAMA). One pass of O(n³) where [`symmetric`] needs several
/// sweeps: ≈ 30× faster for a few hundred.
pub fn symmetric_ql(a: &[f64], n: usize) -> (Vec<f64>, Vec<Vec<f64>>) {
    assert_eq!(a.len(), n * n);
    if n == 0 {
        return (Vec::new(), Vec::new());
    }
    // Rows of `v` are the matrix's columns: the QL rotations below work on
    // pairs of columns, contiguous this way.
    let mut v: Vec<Vec<f64>> = (0..n).map(|i| (0..n).map(|j| a[j * n + i]).collect()).collect();
    let (mut d, mut e) = (vec![0.0; n], vec![0.0; n]);
    // tred2, written on the transpose (v[j][i] holds V[i][j]).
    for j in 0..n {
        d[j] = v[j][n - 1];
    }
    for i in (1..n).rev() {
        let scale: f64 = d[..i].iter().map(|x| x.abs()).sum();
        let mut h = 0.0;
        if scale == 0.0 {
            e[i] = d[i - 1];
            for j in 0..i {
                d[j] = v[j][i - 1];
                v[j][i] = 0.0;
                v[i][j] = 0.0;
            }
        } else {
            for x in &mut d[..i] {
                *x /= scale;
                h += *x * *x;
            }
            let f = d[i - 1];
            let g = if f > 0.0 { -h.sqrt() } else { h.sqrt() };
            e[i] = scale * g;
            h -= f * g;
            d[i - 1] = f - g;
            e[..i].fill(0.0);
            for j in 0..i {
                let f = d[j];
                v[i][j] = f;
                let mut g = e[j] + v[j][j] * f;
                for k in j + 1..i {
                    g += v[j][k] * d[k];
                    e[k] += v[j][k] * f;
                }
                e[j] = g;
            }
            let mut f = 0.0;
            for j in 0..i {
                e[j] /= h;
                f += e[j] * d[j];
            }
            let hh = f / (h + h);
            for j in 0..i {
                e[j] -= hh * d[j];
            }
            for j in 0..i {
                let (f, g) = (d[j], e[j]);
                for k in j..i {
                    v[j][k] -= f * e[k] + g * d[k];
                }
                d[j] = v[j][i - 1];
                v[j][i] = 0.0;
            }
        }
        d[i] = h;
    }
    for i in 0..n - 1 {
        v[i][n - 1] = v[i][i];
        v[i][i] = 1.0;
        let h = d[i + 1];
        if h != 0.0 {
            for k in 0..=i {
                d[k] = v[i + 1][k] / h;
            }
            for j in 0..=i {
                let g: f64 = (0..=i).map(|k| v[i + 1][k] * v[j][k]).sum();
                for k in 0..=i {
                    v[j][k] -= g * d[k];
                }
            }
        }
        v[i + 1][..=i].fill(0.0);
    }
    for j in 0..n {
        d[j] = v[j][n - 1];
        v[j][n - 1] = 0.0;
    }
    v[n - 1][n - 1] = 1.0;
    // tql2.
    for i in 1..n {
        e[i - 1] = e[i];
    }
    e[n - 1] = 0.0;
    let (mut f, mut tst1) = (0.0f64, 0.0f64);
    let eps = f64::EPSILON;
    for l in 0..n {
        tst1 = tst1.max(d[l].abs() + e[l].abs());
        let mut m = l;
        while m < n - 1 && e[m].abs() > eps * tst1 {
            m += 1;
        }
        if m > l {
            loop {
                let g = d[l];
                let p = (d[l + 1] - g) / (2.0 * e[l]);
                let r = if p < 0.0 { -p.hypot(1.0) } else { p.hypot(1.0) };
                d[l] = e[l] / (p + r);
                d[l + 1] = e[l] * (p + r);
                let dl1 = d[l + 1];
                let h = g - d[l];
                for x in &mut d[l + 2..] {
                    *x -= h;
                }
                f += h;
                let mut p = d[m];
                let (mut c, mut c2, mut c3) = (1.0, 1.0, 1.0);
                let el1 = e[l + 1];
                let (mut s, mut s2) = (0.0, 0.0);
                for i in (l..m).rev() {
                    c3 = c2;
                    c2 = c;
                    s2 = s;
                    let g = c * e[i];
                    let h = c * p;
                    let r = p.hypot(e[i]);
                    e[i + 1] = s * r;
                    s = e[i] / r;
                    c = p / r;
                    p = c * d[i] - s * g;
                    d[i + 1] = h + s * (c * g + s * d[i]);
                    let (lo, hi) = v.split_at_mut(i + 1);
                    for (x, y) in lo[i].iter_mut().zip(hi[0].iter_mut()) {
                        let h = *y;
                        *y = s * *x + c * h;
                        *x = c * *x - s * h;
                    }
                }
                p = -s * s2 * c3 * el1 * e[l] / dl1;
                e[l] = s * p;
                d[l] = c * p;
                if e[l].abs() <= eps * tst1 {
                    break;
                }
            }
        }
        d[l] += f;
        e[l] = 0.0;
    }
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&i, &j| d[i].total_cmp(&d[j]));
    (order.iter().map(|&i| d[i]).collect(), order.into_iter().map(|i| std::mem::take(&mut v[i])).collect())
}

/// The generalised problem K·v = λ·M·v, K symmetric, M symmetric positive
/// definite (both `n`×`n`, row-major): eigenvalues ascending and
/// eigenvectors normalised to vᵀ·M·v = 1. Cholesky M = L·Lᵀ, then the
/// symmetric L⁻¹·K·L⁻ᵀ by [`symmetric_ql`], and v = L⁻ᵀ·y.
pub fn generalized(k: &[f64], m: &[f64], n: usize) -> (Vec<f64>, Vec<Vec<f64>>) {
    assert!(k.len() == n * n && m.len() == n * n);
    // Cholesky, lower triangle.
    let mut l = vec![0.0; n * n];
    for i in 0..n {
        for j in 0..=i {
            let mut s = m[i * n + j];
            for q in 0..j {
                s -= l[i * n + q] * l[j * n + q];
            }
            l[i * n + j] = if i == j { s.max(1e-300).sqrt() } else { s / l[j * n + j] };
        }
    }
    // Y = L⁻¹·K (forward substitution, one column at a time, rows kept).
    let forward = |b: &mut [f64]| {
        for i in 0..n {
            let mut s = b[i];
            for q in 0..i {
                s -= l[i * n + q] * b[q];
            }
            b[i] = s / l[i * n + i];
        }
    };
    let mut y = vec![0.0; n * n];
    let mut col = vec![0.0; n];
    for j in 0..n {
        for i in 0..n {
            col[i] = k[i * n + j];
        }
        forward(&mut col);
        for i in 0..n {
            y[i * n + j] = col[i];
        }
    }
    // C = L⁻¹·Yᵀ (K symmetric: (L⁻¹K)ᵀ = K·L⁻ᵀ), symmetrised.
    let mut c = vec![0.0; n * n];
    for j in 0..n {
        for i in 0..n {
            col[i] = y[j * n + i];
        }
        forward(&mut col);
        for i in 0..n {
            c[i * n + j] = col[i];
        }
    }
    for i in 0..n {
        for j in 0..i {
            let s = 0.5 * (c[i * n + j] + c[j * n + i]);
            c[i * n + j] = s;
            c[j * n + i] = s;
        }
    }
    let (vals, vecs) = symmetric_ql(&c, n);
    // v = L⁻ᵀ·y (back substitution).
    let vecs = vecs
        .into_iter()
        .map(|mut v| {
            for i in (0..n).rev() {
                let mut s = v[i];
                for q in i + 1..n {
                    s -= l[q * n + i] * v[q];
                }
                v[i] = s / l[i * n + i];
            }
            v
        })
        .collect();
    (vals, vecs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ql_matches_jacobi() {
        let n = 40;
        let mut a = vec![0.0; n * n];
        for i in 0..n {
            for j in 0..=i {
                let x = ((i * 37 + j * 11) % 23) as f64 / 23.0 - 0.5 + if i == j { i as f64 } else { 0.0 };
                a[i * n + j] = x;
                a[j * n + i] = x;
            }
        }
        let (vj, _) = symmetric(&a, n);
        let (vq, xq) = symmetric_ql(&a, n);
        for (k, (l, x)) in vq.iter().zip(&xq).enumerate() {
            assert!((l - vj[n - 1 - k]).abs() < 1e-10, "{l} vs {}", vj[n - 1 - k]);
            for i in 0..n {
                let ax: f64 = (0..n).map(|j| a[i * n + j] * x[j]).sum();
                assert!((ax - l * x[i]).abs() < 1e-10);
            }
            assert!((x.iter().map(|v| v * v).sum::<f64>() - 1.0).abs() < 1e-12);
        }
    }

    #[test]
    fn decomposes_a_symmetric_matrix() {
        let a = [4.0, 1.0, 2.0, 1.0, 3.0, 0.5, 2.0, 0.5, 5.0];
        let (vals, vecs) = symmetric(&a, 3);
        assert!(vals.windows(2).all(|w| w[0] >= w[1]));
        for (l, x) in vals.iter().zip(&vecs) {
            for i in 0..3 {
                let ax: f64 = (0..3).map(|j| a[i * 3 + j] * x[j]).sum();
                assert!((ax - l * x[i]).abs() < 1e-10, "{ax} vs {}", l * x[i]);
            }
            assert!((x.iter().map(|v| v * v).sum::<f64>() - 1.0).abs() < 1e-12);
        }
    }

    #[test]
    fn generalized_solves_k_v_equals_lambda_m_v() {
        let n = 5;
        let k: Vec<f64> = (0..n * n).map(|x| { let (i, j) = (x / n, x % n); if i == j { 4.0 + i as f64 } else { 0.3 / (1.0 + (i + j) as f64) } }).collect();
        let m: Vec<f64> = (0..n * n).map(|x| { let (i, j) = (x / n, x % n); if i == j { 1.0 + 0.2 * j as f64 } else { 0.05 } }).collect();
        let (vals, vecs) = generalized(&k, &m, n);
        for (lam, v) in vals.iter().zip(&vecs) {
            for i in 0..n {
                let kv: f64 = (0..n).map(|j| k[i * n + j] * v[j]).sum();
                let mv: f64 = (0..n).map(|j| m[i * n + j] * v[j]).sum();
                assert!((kv - lam * mv).abs() < 1e-10, "{kv} vs {}", lam * mv);
            }
            let vmv: f64 = (0..n).map(|i| v[i] * (0..n).map(|j| m[i * n + j] * v[j]).sum::<f64>()).sum();
            assert!((vmv - 1.0).abs() < 1e-10);
        }
        assert!(vals.windows(2).all(|w| w[0] <= w[1]));
    }
}
