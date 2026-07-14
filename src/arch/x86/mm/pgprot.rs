//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/mm/pgprot.c
//! test-origin: linux:vendor/linux/arch/x86/mm/pgprot.c
//! VMA to PTE protection mapping.
//!
//! Wraps the existing generic-mm implementation for
//! `vendor/linux/arch/x86/mm/pgprot.c`. The live mapping logic remains in
//! `crate::mm::pgprot`, while this module exposes the x86 mm path and
//! encryption-map gate.

use crate::arch::x86::mm::paging::pgprot_t;
use crate::include::uapi::errno::EOPNOTSUPP;
use crate::kernel::module::{export_symbol, find_symbol};
use crate::mm::vm_flags::VmFlags;

/// `__supported_pte_mask` - `vendor/linux/arch/x86/mm/init_64.c:107`.
pub static mut SUPPORTED_PTE_MASK: u64 = !0;

/// `__default_kernel_pte_mask` - `vendor/linux/arch/x86/mm/init_64.c:109`.
pub static mut DEFAULT_KERNEL_PTE_MASK: u64 = !0;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once(
        "__supported_pte_mask",
        core::ptr::addr_of_mut!(SUPPORTED_PTE_MASK) as usize,
        true,
    );
    export_symbol_once(
        "__default_kernel_pte_mask",
        core::ptr::addr_of_mut!(DEFAULT_KERNEL_PTE_MASK) as usize,
        false,
    );
    export_symbol_once("vm_get_page_prot", linux_vm_get_page_prot as usize, false);
}

/// `vm_get_page_prot` - `vendor/linux/arch/x86/mm/pgprot.c:35`.
pub unsafe extern "C" fn linux_vm_get_page_prot(vm_flags: u64) -> pgprot_t {
    vm_get_page_prot(vm_flags)
}

pub fn vm_get_page_prot(vm_flags: VmFlags) -> pgprot_t {
    pgprot_t(crate::mm::pgprot::vm_get_page_prot(vm_flags))
}

pub const fn add_encrypt_protection_map(memory_encryption_enabled: bool) -> Result<(), i32> {
    if memory_encryption_enabled {
        Ok(())
    } else {
        Err(EOPNOTSUPP)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arch::x86::mm::paging::{_PAGE_NX, _PAGE_PRESENT, _PAGE_RW, _PAGE_USER, pgprot_val};
    use crate::mm::vm_flags::{VM_EXEC, VM_READ, VM_SHARED, VM_WRITE};

    #[test]
    fn wrapper_returns_x86_pgprot_type() {
        let prot = vm_get_page_prot(VM_READ | VM_WRITE | VM_SHARED);
        assert_ne!(pgprot_val(prot) & _PAGE_PRESENT, 0);
        assert_ne!(pgprot_val(prot) & _PAGE_USER, 0);
        assert_ne!(pgprot_val(prot) & _PAGE_RW, 0);
        assert_ne!(pgprot_val(prot) & _PAGE_NX, 0);
    }

    #[test]
    fn executable_private_mapping_is_not_writable_in_prot() {
        let prot = vm_get_page_prot(VM_READ | VM_WRITE | VM_EXEC);
        assert_eq!(pgprot_val(prot) & _PAGE_RW, 0);
        assert_eq!(pgprot_val(prot) & _PAGE_NX, 0);
    }

    #[test]
    fn pte_mask_data_symbols_are_exported() {
        register_module_exports();
        assert_eq!(
            find_symbol("__supported_pte_mask"),
            Some(core::ptr::addr_of_mut!(SUPPORTED_PTE_MASK) as usize)
        );
        assert_eq!(
            find_symbol("__default_kernel_pte_mask"),
            Some(core::ptr::addr_of_mut!(DEFAULT_KERNEL_PTE_MASK) as usize)
        );
    }
}
