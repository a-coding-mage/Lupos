//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/kvm/cpuid.c
//! test-origin: linux:vendor/linux/arch/x86/kvm/cpuid.c
//! KVM guest CPUID virtualization.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kvm/cpuid.c

// KVM keeps a per-vCPU CPUID table sorted by (function, index). On VMEXIT
// for CPUID, the hypervisor walks the table and emulates the result.
// We model the table lookup; the actual VMEXIT path is owned by the
// virtualization runtime.

extern crate alloc;

use alloc::vec::Vec;

use crate::include::uapi::errno::ENOENT;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KvmCpuidEntry {
    pub function: u32,
    pub index: u32,
    pub eax: u32,
    pub ebx: u32,
    pub ecx: u32,
    pub edx: u32,
    pub flags: u32,
}

pub const KVM_CPUID_FLAG_SIGNIFCANT_INDEX: u32 = 1 << 0;

#[derive(Default, Debug)]
pub struct KvmCpuidTable {
    entries: Vec<KvmCpuidEntry>,
}

impl KvmCpuidTable {
    pub fn add(&mut self, entry: KvmCpuidEntry) {
        self.entries.push(entry);
    }

    pub fn lookup(&self, function: u32, index: u32) -> Result<KvmCpuidEntry, i32> {
        for entry in self.entries.iter() {
            if entry.function == function {
                if entry.flags & KVM_CPUID_FLAG_SIGNIFCANT_INDEX != 0 {
                    if entry.index == index {
                        return Ok(*entry);
                    }
                } else {
                    return Ok(*entry);
                }
            }
        }
        Err(ENOENT)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_respects_significant_index_flag() {
        let mut t = KvmCpuidTable::default();
        t.add(KvmCpuidEntry {
            function: 7,
            index: 0,
            eax: 1,
            ebx: 0,
            ecx: 0,
            edx: 0,
            flags: KVM_CPUID_FLAG_SIGNIFCANT_INDEX,
        });
        t.add(KvmCpuidEntry {
            function: 7,
            index: 1,
            eax: 2,
            ebx: 0,
            ecx: 0,
            edx: 0,
            flags: KVM_CPUID_FLAG_SIGNIFCANT_INDEX,
        });
        assert_eq!(t.lookup(7, 1).unwrap().eax, 2);
        assert!(t.lookup(7, 5).is_err());
    }
}
