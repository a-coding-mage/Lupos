//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/lib/copy_page_64.S
//! linux-source: vendor/linux/arch/x86/lib/clear_page_64.S
//! x86 page copy/clear helpers exported to Linux-built modules.

use crate::kernel::module::{export_symbol, find_symbol};

const PAGE_SIZE: usize = 4096;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("copy_page", linux_copy_page as usize, false);
    export_symbol_once(
        "__clear_pages_unrolled",
        linux___clear_pages_unrolled as usize,
        true,
    );
}

/// `copy_page` - `vendor/linux/arch/x86/lib/copy_page_64.S:17`.
pub unsafe extern "C" fn linux_copy_page(to: *mut u8, from: *const u8) {
    if to.is_null() || from.is_null() {
        return;
    }
    unsafe { core::ptr::copy_nonoverlapping(from, to, PAGE_SIZE) };
}

/// `__clear_pages_unrolled` - `vendor/linux/arch/x86/lib/clear_page_64.S:13`.
///
/// Linux's inline `clear_pages()` call sequence passes the destination in RDI
/// and the byte length in RCX, not in the normal C second-argument register.
#[unsafe(naked)]
pub unsafe extern "C" fn linux___clear_pages_unrolled() {
    core::arch::naked_asm!(
        "sub rsp, 8",
        "mov rsi, rcx",
        "call {body}",
        "add rsp, 8",
        "ret",
        body = sym clear_pages_unrolled_impl,
    );
}

unsafe extern "C" fn clear_pages_unrolled_impl(page: *mut u8, len: usize) {
    if page.is_null() || len == 0 {
        return;
    }
    unsafe { core::ptr::write_bytes(page, 0, len) };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registers_page_copy_exports() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("copy_page"),
            Some(linux_copy_page as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("__clear_pages_unrolled"),
            Some(linux___clear_pages_unrolled as usize)
        );
    }

    #[test]
    fn copy_page_copies_exactly_one_page() {
        let from = [0x5au8; PAGE_SIZE];
        let mut to = [0u8; PAGE_SIZE];
        unsafe { linux_copy_page(to.as_mut_ptr(), from.as_ptr()) };
        assert_eq!(to, from);
    }

    #[test]
    fn clear_pages_impl_zeros_requested_bytes() {
        let mut bytes = [0x33u8; 96];
        unsafe { clear_pages_unrolled_impl(bytes.as_mut_ptr(), 64) };
        assert!(bytes[..64].iter().all(|byte| *byte == 0));
        assert!(bytes[64..].iter().all(|byte| *byte == 0x33));
    }
}
