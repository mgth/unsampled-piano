//! 16 lanes of f32 for the explicit kernels (soundboard, string banks):
//! one code, one implementation per ISA.

use crate::modal::LANES;

/// 16 lanes of f32 as one ISA holds them. Only plain multiplies and adds
/// (no fused multiply-add): every ISA gives the same result.
pub(crate) trait V16: Copy {
    fn zero() -> Self;
    fn splat(x: f32) -> Self;
    fn ld(a: &[f32; LANES]) -> Self;
    fn st(self, a: &mut [f32; LANES]);
    fn add(self, o: Self) -> Self;
    fn sub(self, o: Self) -> Self;
    fn mul(self, o: Self) -> Self;
}

impl V16 for [f32; LANES] {
    #[inline(always)]
    fn zero() -> Self {
        [0.0; LANES]
    }
    #[inline(always)]
    fn splat(x: f32) -> Self {
        [x; LANES]
    }
    #[inline(always)]
    fn ld(a: &[f32; LANES]) -> Self {
        *a
    }
    #[inline(always)]
    fn st(self, a: &mut [f32; LANES]) {
        *a = self;
    }
    #[inline(always)]
    fn add(self, o: Self) -> Self {
        std::array::from_fn(|l| self[l] + o[l])
    }
    #[inline(always)]
    fn sub(self, o: Self) -> Self {
        std::array::from_fn(|l| self[l] - o[l])
    }
    #[inline(always)]
    fn mul(self, o: Self) -> Self {
        std::array::from_fn(|l| self[l] * o[l])
    }
}

#[cfg(target_arch = "x86_64")]
pub(crate) mod x86 {
    use super::{V16, LANES};
    use std::arch::x86_64::*;

    #[derive(Clone, Copy)]
    pub struct Z(__m512);
    #[derive(Clone, Copy)]
    pub struct Y(__m256, __m256);

    // SAFETY (all methods): only used inside functions compiled with the
    // matching target features, selected when detected.
    impl V16 for Z {
        #[inline(always)]
        fn zero() -> Self {
            unsafe { Z(_mm512_setzero_ps()) }
        }
        #[inline(always)]
        fn splat(x: f32) -> Self {
            unsafe { Z(_mm512_set1_ps(x)) }
        }
        #[inline(always)]
        fn ld(a: &[f32; LANES]) -> Self {
            unsafe { Z(_mm512_loadu_ps(a.as_ptr())) }
        }
        #[inline(always)]
        fn st(self, a: &mut [f32; LANES]) {
            unsafe { _mm512_storeu_ps(a.as_mut_ptr(), self.0) }
        }
        #[inline(always)]
        fn add(self, o: Self) -> Self {
            unsafe { Z(_mm512_add_ps(self.0, o.0)) }
        }
        #[inline(always)]
        fn sub(self, o: Self) -> Self {
            unsafe { Z(_mm512_sub_ps(self.0, o.0)) }
        }
        #[inline(always)]
        fn mul(self, o: Self) -> Self {
            unsafe { Z(_mm512_mul_ps(self.0, o.0)) }
        }
    }

    impl V16 for Y {
        #[inline(always)]
        fn zero() -> Self {
            unsafe { Y(_mm256_setzero_ps(), _mm256_setzero_ps()) }
        }
        #[inline(always)]
        fn splat(x: f32) -> Self {
            unsafe { Y(_mm256_set1_ps(x), _mm256_set1_ps(x)) }
        }
        #[inline(always)]
        fn ld(a: &[f32; LANES]) -> Self {
            unsafe { Y(_mm256_loadu_ps(a.as_ptr()), _mm256_loadu_ps(a.as_ptr().add(8))) }
        }
        #[inline(always)]
        fn st(self, a: &mut [f32; LANES]) {
            unsafe {
                _mm256_storeu_ps(a.as_mut_ptr(), self.0);
                _mm256_storeu_ps(a.as_mut_ptr().add(8), self.1);
            }
        }
        #[inline(always)]
        fn add(self, o: Self) -> Self {
            unsafe { Y(_mm256_add_ps(self.0, o.0), _mm256_add_ps(self.1, o.1)) }
        }
        #[inline(always)]
        fn sub(self, o: Self) -> Self {
            unsafe { Y(_mm256_sub_ps(self.0, o.0), _mm256_sub_ps(self.1, o.1)) }
        }
        #[inline(always)]
        fn mul(self, o: Self) -> Self {
            unsafe { Y(_mm256_mul_ps(self.0, o.0), _mm256_mul_ps(self.1, o.1)) }
        }
    }
}

/// NEON (every aarch64 CPU): four 4-lane registers.
#[cfg(target_arch = "aarch64")]
pub(crate) mod arm {
    use super::{V16, LANES};
    use std::arch::aarch64::*;

    #[derive(Clone, Copy)]
    pub struct N([float32x4_t; 4]);

    // SAFETY (all methods): NEON is part of the aarch64 baseline; the loads
    // and stores stay within the 16-float arrays.
    impl V16 for N {
        #[inline(always)]
        fn zero() -> Self {
            unsafe { N([vdupq_n_f32(0.0); 4]) }
        }
        #[inline(always)]
        fn splat(x: f32) -> Self {
            unsafe { N([vdupq_n_f32(x); 4]) }
        }
        #[inline(always)]
        fn ld(a: &[f32; LANES]) -> Self {
            unsafe { N(std::array::from_fn(|k| vld1q_f32(a.as_ptr().add(4 * k)))) }
        }
        #[inline(always)]
        fn st(self, a: &mut [f32; LANES]) {
            unsafe {
                for k in 0..4 {
                    vst1q_f32(a.as_mut_ptr().add(4 * k), self.0[k]);
                }
            }
        }
        #[inline(always)]
        fn add(self, o: Self) -> Self {
            unsafe { N(std::array::from_fn(|k| vaddq_f32(self.0[k], o.0[k]))) }
        }
        #[inline(always)]
        fn sub(self, o: Self) -> Self {
            unsafe { N(std::array::from_fn(|k| vsubq_f32(self.0[k], o.0[k]))) }
        }
        #[inline(always)]
        fn mul(self, o: Self) -> Self {
            unsafe { N(std::array::from_fn(|k| vmulq_f32(self.0[k], o.0[k]))) }
        }
    }
}
