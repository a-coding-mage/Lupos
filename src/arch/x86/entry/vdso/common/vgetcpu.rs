//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/entry/vdso/common/vgetcpu.c
//! test-origin: linux:vendor/linux/arch/x86/entry/vdso/common/vgetcpu.c
//! vDSO getcpu wrapper.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/entry/vdso/common/vgetcpu.c

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CpuNode {
    pub cpu: u32,
    pub node: u32,
}

pub fn vdso_read_cpunode(cpu: Option<&mut u32>, node: Option<&mut u32>, current: CpuNode) {
    if let Some(cpu) = cpu {
        *cpu = current.cpu;
    }
    if let Some(node) = node {
        *node = current.node;
    }
}

pub fn vdso_getcpu(cpu: Option<&mut u32>, node: Option<&mut u32>, current: CpuNode) -> i64 {
    vdso_read_cpunode(cpu, node, current);
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn getcpu_writes_only_requested_outputs() {
        let mut cpu = 0;
        assert_eq!(
            vdso_getcpu(Some(&mut cpu), None, CpuNode { cpu: 7, node: 2 }),
            0
        );
        assert_eq!(cpu, 7);
    }
}
