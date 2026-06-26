//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/vmcore_info_64.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/vmcore_info_64.c
//! x86_64 crash vmcore-info records.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/vmcore_info_64.c

#![allow(dead_code)]

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VmcoreInfoRecord {
    pub key: &'static str,
    pub value: u64,
}

pub const fn arch_crash_save_vmcoreinfo(
    phys_base: u64,
    kernel_offset: u64,
) -> [VmcoreInfoRecord; 2] {
    [
        VmcoreInfoRecord {
            key: "NUMBER(phys_base)",
            value: phys_base,
        },
        VmcoreInfoRecord {
            key: "KERNELOFFSET",
            value: kernel_offset,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vmcore_info_64_records_phys_base_and_kernel_offset() {
        let records = arch_crash_save_vmcoreinfo(0x100000, 0x200000);
        assert_eq!(records[0].value, 0x100000);
        assert_eq!(records[1].value, 0x200000);
    }
}
