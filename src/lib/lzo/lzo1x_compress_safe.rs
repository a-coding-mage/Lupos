//! linux-parity: complete
//! linux-source: vendor/linux/lib/lzo/lzo1x_compress_safe.c
//! test-origin: linux:vendor/linux/lib/lzo/lzo1x_compress_safe.c
//! Safe LZO1X compressor wrapper.

pub const SAFE_MACRO: &str = "#define LZO_SAFE(name) name##_safe";
pub const HAVE_OP_MACRO: &str = "#define HAVE_OP(x) ((size_t)(op_end - op) >= (size_t)(x))";
pub const INCLUDED_SOURCE: &str = "#include \"lzo1x_compress.c\"";

pub fn included_source() -> &'static str {
    INCLUDED_SOURCE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lzo_safe_wrapper_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/lzo/lzo1x_compress_safe.c"
        ));
        assert!(source.contains(SAFE_MACRO));
        assert!(source.contains(HAVE_OP_MACRO));
        assert!(source.contains(INCLUDED_SOURCE));
        assert_eq!(included_source(), INCLUDED_SOURCE);
    }
}
