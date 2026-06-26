//! linux-parity: complete
//! linux-source: vendor/linux/lib/memcat_p.c
//! test-origin: linux:vendor/linux/lib/memcat_p.c
//! Concatenate NULL-terminated pointer arrays.

extern crate alloc;

use alloc::alloc::{Layout, alloc, dealloc};
use alloc::vec::Vec;
use core::ffi::c_void;
use core::ptr;

use crate::kernel::module::{export_symbol, find_symbol};

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("__memcat_p", __memcat_p as usize, true);
}

pub fn memcat_p_values<T: Copy>(a: &[Option<T>], b: &[Option<T>]) -> Vec<Option<T>> {
    let a_len = null_terminated_len(a);
    let b_len = null_terminated_len(b);
    let mut out = Vec::with_capacity(a_len + b_len + 1);
    out.extend_from_slice(&a[..a_len]);
    out.extend_from_slice(&b[..b_len]);
    out.push(None);
    out
}

fn null_terminated_len<T>(items: &[Option<T>]) -> usize {
    items
        .iter()
        .position(Option::is_none)
        .unwrap_or(items.len())
}

unsafe fn raw_null_terminated_len(mut p: *const *const c_void) -> usize {
    let mut len = 0usize;
    unsafe {
        while !(*p).is_null() {
            len += 1;
            p = p.add(1);
        }
    }
    len
}

pub unsafe extern "C" fn __memcat_p(
    a: *const *const c_void,
    b: *const *const c_void,
) -> *mut *const c_void {
    if a.is_null() || b.is_null() {
        return ptr::null_mut();
    }

    let a_len = unsafe { raw_null_terminated_len(a) };
    let b_len = unsafe { raw_null_terminated_len(b) };
    let Some(total) = a_len.checked_add(b_len).and_then(|len| len.checked_add(1)) else {
        return ptr::null_mut();
    };
    let Ok(layout) = Layout::array::<*const c_void>(total) else {
        return ptr::null_mut();
    };
    let out = unsafe { alloc(layout) as *mut *const c_void };
    if out.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        for idx in 0..a_len {
            out.add(idx).write(*a.add(idx));
        }
        for idx in 0..b_len {
            out.add(a_len + idx).write(*b.add(idx));
        }
        out.add(total - 1).write(ptr::null());
    }

    out
}

pub unsafe fn memcat_p_free(p: *mut *const c_void, entries_with_null: usize) {
    if p.is_null() || entries_with_null == 0 {
        return;
    }
    if let Ok(layout) = Layout::array::<*const c_void>(entries_with_null) {
        unsafe { dealloc(p.cast::<u8>(), layout) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_memcat_p_preserves_order_and_terminator() {
        assert_eq!(
            memcat_p_values(&[Some(1), Some(2), None], &[Some(3), None]),
            [Some(1), Some(2), Some(3), None]
        );
        assert_eq!(
            memcat_p_values(&[None, Some(99)], &[Some(7), None]),
            [Some(7), None]
        );
    }

    #[test]
    fn raw_memcat_p_matches_linux_pointer_array_contract() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/memcat_p.c"
        ));
        assert!(source.contains("for (nr = 0, p = a; *p; nr++, p++)"));
        assert!(source.contains("for (p = b; *p; nr++, p++)"));
        assert!(source.contains("kmalloc_array(nr, sizeof(void *), GFP_KERNEL)"));
        assert!(source.contains("for (nr--; nr >= 0; nr--"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(__memcat_p);"));

        let one = 1usize as *const c_void;
        let two = 2usize as *const c_void;
        let three = 3usize as *const c_void;
        let a = [one, two, ptr::null()];
        let b = [three, ptr::null()];
        let out = unsafe { __memcat_p(a.as_ptr(), b.as_ptr()) };
        assert!(!out.is_null());
        let merged = unsafe { core::slice::from_raw_parts(out, 4) };
        assert_eq!(merged, [one, two, three, ptr::null()]);
        unsafe { memcat_p_free(out, 4) };

        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("__memcat_p"),
            Some(__memcat_p as usize)
        );
    }
}
