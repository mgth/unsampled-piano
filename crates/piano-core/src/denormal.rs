//! Flush-to-zero / denormals-are-zero for the duration of a callback.
//! Decaying modes otherwise end up in subnormal range, which is ~100× slower.

pub struct DenormalGuard {
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    old: u64,
}

impl DenormalGuard {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        #[cfg(target_arch = "x86_64")]
        unsafe {
            let mut csr: u32 = 0;
            core::arch::asm!("stmxcsr [{}]", in(reg) &mut csr, options(nostack, preserves_flags));
            let new = csr | 0x8040; // FTZ | DAZ
            core::arch::asm!("ldmxcsr [{}]", in(reg) &new, options(nostack, preserves_flags));
            Self { old: csr as u64 }
        }
        #[cfg(target_arch = "aarch64")]
        unsafe {
            let fpcr: u64;
            core::arch::asm!("mrs {}, fpcr", out(reg) fpcr, options(nomem, nostack, preserves_flags));
            core::arch::asm!("msr fpcr, {}", in(reg) fpcr | (1 << 24), options(nomem, nostack, preserves_flags));
            Self { old: fpcr }
        }
        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
        Self {}
    }
}

impl Drop for DenormalGuard {
    fn drop(&mut self) {
        #[cfg(target_arch = "x86_64")]
        unsafe {
            let csr = self.old as u32;
            core::arch::asm!("ldmxcsr [{}]", in(reg) &csr, options(nostack, preserves_flags));
        }
        #[cfg(target_arch = "aarch64")]
        unsafe {
            core::arch::asm!("msr fpcr, {}", in(reg) self.old, options(nomem, nostack, preserves_flags));
        }
    }
}
