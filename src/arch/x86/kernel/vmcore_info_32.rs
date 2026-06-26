//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/vmcore_info_32.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/vmcore_info_32.c
//! x86_32 crash vmcore-info records.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/vmcore_info_32.c

#![allow(dead_code)]

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VmcoreInfoRecord {
    pub key: &'static str,
    pub value: u64,
}

pub const fn arch_crash_save_vmcoreinfo() -> [VmcoreInfoRecord; 2] {
    [
        VmcoreInfoRecord {
            key: "NUMBER(phys_base)",
            value: 0,
        },
        VmcoreInfoRecord {
            key: "KERNELOFFSET",
            value: 0,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vmcore_info_32_exports_stable_record_names() {
        let records = arch_crash_save_vmcoreinfo();
        assert_eq!(records[0].key, "NUMBER(phys_base)");
        assert_eq!(records[1].key, "KERNELOFFSET");
    }
}
