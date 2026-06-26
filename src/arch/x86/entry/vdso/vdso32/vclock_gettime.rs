//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/entry/vdso/vdso32/vclock_gettime.c
//! test-origin: linux:vendor/linux/arch/x86/entry/vdso/vdso32/vclock_gettime.c
//! 32-bit vDSO clock wrapper include.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/entry/vdso/vdso32/vclock_gettime.c

pub use crate::arch::x86::entry::vdso::common::vclock_gettime::{
    CLOCK_MONOTONIC, CLOCK_REALTIME, KernelTimespec, OldTimespec32, VdsoTimeSnapshot,
    vdso_clock_getres, vdso_clock_gettime32,
};

pub fn vdso32_clock_gettime(clock: i32, ts: &mut OldTimespec32, snap: VdsoTimeSnapshot) -> i32 {
    vdso_clock_gettime32(clock, ts, snap)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vdso32_vclock_gettime_matches_linux_include_wrapper() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/entry/vdso/vdso32/vclock_gettime.c"
        ));
        assert_eq!(source.trim(), "#include \"common/vclock_gettime.c\"");

        let snap = VdsoTimeSnapshot {
            realtime: KernelTimespec {
                tv_sec: 7,
                tv_nsec: 8,
            },
            monotonic: KernelTimespec::default(),
        };
        let mut ts = OldTimespec32::default();
        assert_eq!(vdso32_clock_gettime(CLOCK_REALTIME, &mut ts, snap), 0);
        assert_eq!((ts.tv_sec, ts.tv_nsec), (7, 8));
    }
}
