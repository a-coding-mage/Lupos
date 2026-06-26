//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/kernel/cpu/sgx/main.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/sgx/main.c
//! SGX EPC memory allocator core.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kernel/cpu/sgx/main.c

// `main.c` enumerates EPC sections from CPUID(0x12) subleaves 2..N. Each
// section is a chunk of reserved RAM dedicated to enclave memory. We
// model the section list and a NUMA-aware free-page allocator.

extern crate alloc;

use alloc::vec::Vec;

use crate::include::uapi::errno::ENOMEM;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EpcSection {
    pub base: u64,
    pub size: u64,
    pub numa_node: u32,
    pub free_pages: u64,
}

#[derive(Default, Debug)]
pub struct EpcAllocator {
    pub sections: Vec<EpcSection>,
}

impl EpcAllocator {
    pub fn add_section(&mut self, section: EpcSection) {
        self.sections.push(section);
    }

    pub fn total_free_pages(&self) -> u64 {
        self.sections.iter().map(|s| s.free_pages).sum()
    }

    pub fn allocate(&mut self, preferred_node: u32) -> Result<u64, i32> {
        if let Some(s) = self
            .sections
            .iter_mut()
            .find(|s| s.numa_node == preferred_node && s.free_pages > 0)
        {
            s.free_pages -= 1;
            return Ok(s.base);
        }
        if let Some(s) = self.sections.iter_mut().find(|s| s.free_pages > 0) {
            s.free_pages -= 1;
            return Ok(s.base);
        }
        Err(ENOMEM)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocate_prefers_local_node_then_falls_back() {
        let mut a = EpcAllocator::default();
        a.add_section(EpcSection {
            base: 0x1000_0000,
            size: 0x1000,
            numa_node: 0,
            free_pages: 1,
        });
        a.add_section(EpcSection {
            base: 0x2000_0000,
            size: 0x1000,
            numa_node: 1,
            free_pages: 1,
        });
        assert_eq!(a.allocate(1), Ok(0x2000_0000));
        assert_eq!(a.allocate(1), Ok(0x1000_0000));
        assert_eq!(a.allocate(1), Err(ENOMEM));
    }

    #[test]
    fn total_free_pages_sums_sections() {
        let mut a = EpcAllocator::default();
        a.add_section(EpcSection {
            base: 0,
            size: 0,
            numa_node: 0,
            free_pages: 4,
        });
        a.add_section(EpcSection {
            base: 0,
            size: 0,
            numa_node: 1,
            free_pages: 5,
        });
        assert_eq!(a.total_free_pages(), 9);
    }
}
