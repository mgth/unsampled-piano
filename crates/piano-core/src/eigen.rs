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

#[cfg(test)]
mod tests {
    use super::*;

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
}
