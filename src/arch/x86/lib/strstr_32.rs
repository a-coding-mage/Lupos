//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/lib/strstr_32.c
//! test-origin: linux:vendor/linux/arch/x86/lib/strstr_32.c
//! 32-bit x86 exported C-string substring search.

pub fn strstr(cs: &[u8], ct: &[u8]) -> Option<usize> {
    super::arch_lib::c_strstr(cs, ct)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strstr_uses_linux_c_string_boundaries() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/lib/strstr_32.c"
        ));
        assert!(source.contains("char *strstr(const char *cs, const char *ct)"));
        assert!(source.contains("EXPORT_SYMBOL(strstr);"));

        assert_eq!(strstr(b"hello kernel\0tail", b"kernel\0ignored"), Some(6));
        assert_eq!(strstr(b"hello\0kernel", b"kernel"), None);
        assert_eq!(strstr(b"hello", b"\0"), Some(0));
    }
}
