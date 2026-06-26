//! linux-parity: complete
//! linux-source: vendor/linux/lib/bsearch.c
//! test-origin: linux:vendor/linux/lib/bsearch.c
//! Generic Linux binary search helper.

use core::cmp::Ordering;
use core::ffi::c_void;
use core::ptr;

use crate::kernel::module::{export_symbol, find_symbol};

pub type CmpFunc = unsafe extern "C" fn(key: *const c_void, elem: *const c_void) -> i32;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("bsearch", bsearch as usize, false);
}

pub fn bsearch_by<'a, K, T, F>(key: &K, base: &'a [T], mut cmp: F) -> Option<&'a T>
where
    F: FnMut(&K, &T) -> Ordering,
{
    let mut start = 0usize;
    let mut len = base.len();

    while len > 0 {
        let half = len / 2;
        let mid = start + half;
        match cmp(key, &base[mid]) {
            Ordering::Equal => return Some(&base[mid]),
            Ordering::Greater => {
                start = mid + 1;
                len -= half + 1;
            }
            Ordering::Less => {
                len = half;
            }
        }
    }

    None
}

pub unsafe extern "C" fn bsearch(
    key: *const c_void,
    base: *const c_void,
    mut num: usize,
    size: usize,
    cmp: Option<CmpFunc>,
) -> *mut c_void {
    let Some(cmp) = cmp else {
        return ptr::null_mut();
    };
    if key.is_null() || base.is_null() || size == 0 {
        return ptr::null_mut();
    }

    let mut start = 0usize;
    while num > 0 {
        let half = num / 2;
        let mid = start + half;
        let Some(byte_offset) = mid.checked_mul(size) else {
            return ptr::null_mut();
        };
        let elem = unsafe { (base as *const u8).add(byte_offset) as *const c_void };
        let result = unsafe { cmp(key, elem) };
        if result == 0 {
            return elem as *mut c_void;
        }
        if result > 0 {
            start = mid + 1;
            num -= half + 1;
        } else {
            num = half;
        }
    }

    ptr::null_mut()
}

#[cfg(test)]
mod tests {
    use super::*;

    unsafe extern "C" fn cmp_i32(key: *const c_void, elem: *const c_void) -> i32 {
        let key = unsafe { *(key as *const i32) };
        let elem = unsafe { *(elem as *const i32) };
        match key.cmp(&elem) {
            Ordering::Less => -1,
            Ordering::Equal => 0,
            Ordering::Greater => 1,
        }
    }

    #[test]
    fn generic_bsearch_matches_sorted_array_contract() {
        let values = [1, 3, 5, 7, 9];
        assert_eq!(bsearch_by(&7, &values, |key, elem| key.cmp(elem)), Some(&7));
        assert_eq!(bsearch_by(&8, &values, |key, elem| key.cmp(elem)), None);
    }

    #[test]
    fn raw_bsearch_matches_linux_exported_helper() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/bsearch.c"
        ));
        assert!(source.contains("return __inline_bsearch(key, base, num, size, cmp);"));
        assert!(source.contains("EXPORT_SYMBOL(bsearch);"));
        assert!(source.contains("NOKPROBE_SYMBOL(bsearch);"));

        let values = [1i32, 3, 5, 7, 9];
        let key = 7i32;
        let found = unsafe {
            bsearch(
                &key as *const i32 as *const c_void,
                values.as_ptr() as *const c_void,
                values.len(),
                core::mem::size_of::<i32>(),
                Some(cmp_i32),
            )
        };
        assert!(!found.is_null());
        assert_eq!(unsafe { *(found as *const i32) }, 7);

        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("bsearch"),
            Some(bsearch as usize)
        );
    }
}
