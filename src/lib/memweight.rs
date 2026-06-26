//! linux-parity: complete
//! linux-source: vendor/linux/lib/memweight.c
//! test-origin: linux:vendor/linux/lib/memweight.c
//! Count set bits across a memory range.

use core::ffi::c_void;

use crate::kernel::module::{export_symbol, find_symbol};

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("memweight", memweight as usize, false);
}

pub fn memweight_bytes(bytes: &[u8]) -> usize {
    bytes.iter().map(|byte| byte.count_ones() as usize).sum()
}

pub unsafe extern "C" fn memweight(ptr: *const c_void, bytes: usize) -> usize {
    if ptr.is_null() && bytes != 0 {
        return 0;
    }
    let bytes = unsafe { core::slice::from_raw_parts(ptr as *const u8, bytes) };
    memweight_bytes(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memweight_counts_bits_and_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/memweight.c"
        ));
        assert!(source.contains("size_t memweight(const void *ptr, size_t bytes)"));
        assert!(source.contains("ret += hweight8(*bitmap);"));
        assert!(source.contains("ret += bitmap_weight((unsigned long *)bitmap"));
        assert!(source.contains("EXPORT_SYMBOL(memweight);"));
        let bytes = [0b1010_1010u8, 0xff, 0x00, 0x01];
        assert_eq!(memweight_bytes(&bytes), 13);
        assert_eq!(
            unsafe { memweight(bytes.as_ptr() as *const c_void, bytes.len()) },
            13
        );
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("memweight"),
            Some(memweight as usize)
        );
    }
}
