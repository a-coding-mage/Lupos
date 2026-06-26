//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/entry/vdso/vdso64/vclock_gettime.c
//! test-origin: linux:vendor/linux/arch/x86/entry/vdso/vdso64/vclock_gettime.c
//! 64-bit vDSO clock wrapper include.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/entry/vdso/vdso64/vclock_gettime.c

pub use crate::arch::x86::entry::vdso::common::vclock_gettime::{
    CLOCK_MONOTONIC, CLOCK_REALTIME, KernelTimespec, VdsoTimeSnapshot, vdso_clock_getres,
    vdso_clock_gettime,
};

pub fn vdso64_clock_gettime(clock: i32, ts: &mut KernelTimespec, snap: VdsoTimeSnapshot) -> i32 {
    vdso_clock_gettime(clock, ts, snap)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vdso64_vclock_gettime_matches_linux_include_wrapper() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/entry/vdso/vdso64/vclock_gettime.c"
        ));
        assert_eq!(source.trim(), "#include \"common/vclock_gettime.c\"");

        let snap = VdsoTimeSnapshot {
            realtime: KernelTimespec::default(),
            monotonic: KernelTimespec {
                tv_sec: 11,
                tv_nsec: 12,
            },
        };
        let mut ts = KernelTimespec::default();
        assert_eq!(vdso64_clock_gettime(CLOCK_MONOTONIC, &mut ts, snap), 0);
        assert_eq!((ts.tv_sec, ts.tv_nsec), (11, 12));
    }
}
