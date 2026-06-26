//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/entry/vdso/vdso64/vgetcpu.c
//! test-origin: linux:vendor/linux/arch/x86/entry/vdso/vdso64/vgetcpu.c
//! 64-bit vDSO getcpu wrapper include.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/entry/vdso/vdso64/vgetcpu.c

pub use crate::arch::x86::entry::vdso::common::vgetcpu::{CpuNode, vdso_getcpu};

pub fn vdso64_getcpu(cpu: Option<&mut u32>, node: Option<&mut u32>, current: CpuNode) -> i64 {
    vdso_getcpu(cpu, node, current)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vdso64_vgetcpu_matches_linux_include_wrapper() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/entry/vdso/vdso64/vgetcpu.c"
        ));
        assert_eq!(source.trim(), "#include \"common/vgetcpu.c\"");

        let mut cpu = 0;
        let mut node = 0;
        assert_eq!(
            vdso64_getcpu(Some(&mut cpu), Some(&mut node), CpuNode { cpu: 5, node: 2 }),
            0
        );
        assert_eq!((cpu, node), (5, 2));
    }
}
