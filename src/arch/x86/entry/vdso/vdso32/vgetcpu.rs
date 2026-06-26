//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/entry/vdso/vdso32/vgetcpu.c
//! test-origin: linux:vendor/linux/arch/x86/entry/vdso/vdso32/vgetcpu.c
//! 32-bit vDSO getcpu wrapper include.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/entry/vdso/vdso32/vgetcpu.c

pub use crate::arch::x86::entry::vdso::common::vgetcpu::{CpuNode, vdso_getcpu};

pub fn vdso32_getcpu(cpu: Option<&mut u32>, node: Option<&mut u32>, current: CpuNode) -> i64 {
    vdso_getcpu(cpu, node, current)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vdso32_vgetcpu_matches_linux_include_wrapper() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/entry/vdso/vdso32/vgetcpu.c"
        ));
        assert_eq!(source.trim(), "#include \"common/vgetcpu.c\"");

        let mut cpu = 0;
        let mut node = 0;
        assert_eq!(
            vdso32_getcpu(Some(&mut cpu), Some(&mut node), CpuNode { cpu: 3, node: 1 }),
            0
        );
        assert_eq!((cpu, node), (3, 1));
    }
}
