//! linux-parity: complete
//! linux-source: vendor/linux/fs/proc/util.c
//! test-origin: linux:vendor/linux/fs/proc/util.c
//! Shared procfs formatting helpers.
//!
//! Ref: `vendor/linux/fs/proc/util.c`

extern crate alloc;

use alloc::sync::Arc;

use crate::fs::kernfs::KernfsNode;

pub type ProcShow = fn(&Arc<KernfsNode>, &mut [u8]) -> Result<usize, i32>;

pub fn name_to_int(name: &str) -> u32 {
    let bytes = name.as_bytes();
    let mut len = bytes.len();
    if len == 0 || (len > 1 && bytes[0] == b'0') {
        return u32::MAX;
    }

    let mut n = 0u32;
    let mut idx = 0usize;
    loop {
        let c = bytes[idx].wrapping_sub(b'0');
        if c > 9 || n >= (u32::MAX - 9) / 10 {
            return u32::MAX;
        }
        n = n * 10 + c as u32;
        len -= 1;
        if len == 0 {
            return n;
        }
        idx += 1;
    }
}

pub fn copy_into(buf: &mut [u8], s: &str) -> Result<usize, i32> {
    let n = s.len().min(buf.len());
    buf[..n].copy_from_slice(&s.as_bytes()[..n]);
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proc_util_name_to_int_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/proc/util.c"
        ));
        assert!(source.contains("unsigned name_to_int(const struct qstr *qstr)"));
        assert!(source.contains("if (len > 1 && *name == '0')"));
        assert!(source.contains("if (n >= (~0U-9)/10)"));
        assert!(source.contains("return ~0U;"));

        assert_eq!(name_to_int("0"), 0);
        assert_eq!(name_to_int("42"), 42);
        assert_eq!(name_to_int("01"), u32::MAX);
        assert_eq!(name_to_int("4a"), u32::MAX);
        assert_eq!(name_to_int("429496728"), 429496728);
        assert_eq!(name_to_int("4294967280"), u32::MAX);
        assert_eq!(name_to_int(""), u32::MAX);
    }
}
