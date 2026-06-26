//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/kernel/cpu/sgx/encl.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/sgx/encl.c
//! SGX enclave memory backing store.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kernel/cpu/sgx/encl.c

// Each enclave tracks its EPC pages by linear address. The driver keeps
// an xarray indexed by ENCLU page offset. We model the page map as a
// sorted vector keyed by page offset, with the SECINFO permission bits.

extern crate alloc;

use alloc::vec::Vec;

use crate::include::uapi::errno::EINVAL;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SgxPagePermissions(pub u8);

impl SgxPagePermissions {
    pub const READ: Self = Self(0x01);
    pub const WRITE: Self = Self(0x02);
    pub const EXEC: Self = Self(0x04);

    pub const fn allows(self, kind: u8) -> bool {
        self.0 & kind != 0
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Enclave {
    pub size: u64,
    pages: Vec<(u64, SgxPagePermissions)>,
}

impl Enclave {
    pub fn new(size: u64) -> Self {
        Self {
            size,
            pages: Vec::new(),
        }
    }

    pub fn add_page(&mut self, offset: u64, perms: SgxPagePermissions) -> Result<(), i32> {
        if offset >= self.size {
            return Err(EINVAL);
        }
        if self.pages.iter().any(|(o, _)| *o == offset) {
            return Err(EINVAL);
        }
        self.pages.push((offset, perms));
        Ok(())
    }

    pub fn permissions(&self, offset: u64) -> Option<SgxPagePermissions> {
        self.pages
            .iter()
            .find(|(o, _)| *o == offset)
            .map(|(_, p)| *p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_page_rejects_offset_beyond_size() {
        let mut e = Enclave::new(0x2000);
        assert!(e.add_page(0x3000, SgxPagePermissions::READ).is_err());
        assert!(e.add_page(0x1000, SgxPagePermissions::READ).is_ok());
    }

    #[test]
    fn permissions_use_bitmask() {
        let perms = SgxPagePermissions(0x05);
        assert!(perms.allows(SgxPagePermissions::READ.0));
        assert!(perms.allows(SgxPagePermissions::EXEC.0));
        assert!(!perms.allows(SgxPagePermissions::WRITE.0));
    }
}
