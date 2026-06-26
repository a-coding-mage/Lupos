//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/lib/memcpy_32.c
//! test-origin: linux:vendor/linux/arch/x86/lib/memcpy_32.c
//! 32-bit x86 exported memcpy/memset wrappers.

/// Linux `memcpy()` in this file delegates to `__memcpy()` and returns `to`.
pub fn memcpy<'a>(to: &'a mut [u8], from: &[u8], n: usize) -> &'a mut [u8] {
    let len = n.min(to.len()).min(from.len());
    to[..len].copy_from_slice(&from[..len]);
    to
}

/// Linux `memset()` delegates to `__memset()` and returns `s`.
pub fn memset(s: &mut [u8], c: u8, count: usize) -> &mut [u8] {
    let len = count.min(s.len());
    s[..len].fill(c);
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exported_memcpy_and_memset_match_wrapper_shape() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/lib/memcpy_32.c"
        ));
        assert!(source.contains("return __memcpy(to, from, n);"));
        assert!(source.contains("return __memset(s, c, count);"));
        assert!(source.contains("EXPORT_SYMBOL(memcpy);"));
        assert!(source.contains("EXPORT_SYMBOL(memset);"));

        let mut dst = [0u8; 4];
        let ptr = memcpy(&mut dst, b"abcd", 4).as_ptr();
        assert_eq!(&dst, b"abcd");
        assert_eq!(ptr, dst.as_ptr());

        let ptr = memset(&mut dst, b'Z', 2).as_ptr();
        assert_eq!(&dst, b"ZZcd");
        assert_eq!(ptr, dst.as_ptr());
    }
}
