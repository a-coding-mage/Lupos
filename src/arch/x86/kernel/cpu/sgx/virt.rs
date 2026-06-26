//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/kernel/cpu/sgx/virt.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/sgx/virt.c
//! SGX virtualization support for KVM guests.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kernel/cpu/sgx/virt.c

// `virt.c` exposes ioctls so KVM can carve out vEPC sections per VM. The
// VM-scoped allocator wraps a slice of the host EPC and reports per-VM
// usage. We model the per-VM accounting.

use crate::include::uapi::errno::ENOMEM;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VmEpcQuota {
    pub assigned: u64,
    pub in_use: u64,
}

impl VmEpcQuota {
    pub const fn new(assigned: u64) -> Self {
        Self {
            assigned,
            in_use: 0,
        }
    }

    pub fn reserve(&mut self, count: u64) -> Result<(), i32> {
        if self.in_use + count > self.assigned {
            return Err(ENOMEM);
        }
        self.in_use += count;
        Ok(())
    }

    pub fn release(&mut self, count: u64) {
        self.in_use = self.in_use.saturating_sub(count);
    }

    pub const fn remaining(&self) -> u64 {
        self.assigned.saturating_sub(self.in_use)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reserve_respects_assigned_quota() {
        let mut q = VmEpcQuota::new(10);
        assert!(q.reserve(7).is_ok());
        assert_eq!(q.remaining(), 3);
        assert_eq!(q.reserve(4), Err(ENOMEM));
        q.release(5);
        assert_eq!(q.remaining(), 8);
    }
}
