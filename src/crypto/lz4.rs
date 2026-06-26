//! linux-parity: complete
//! linux-source: vendor/linux/crypto/lz4.c
//! test-origin: linux:vendor/linux/crypto/lz4.c
//! LZ4 scomp registration metadata.

use crate::include::uapi::errno::EINVAL;

pub const LZ4_MEMORY_USAGE: usize = 14;
pub const LZ4_STREAMSIZE_U64: usize = (1 << (LZ4_MEMORY_USAGE - 3)) + 4;
pub const LZ4_MEM_COMPRESS: usize = LZ4_STREAMSIZE_U64 * core::mem::size_of::<u64>();
pub const CRA_NAME: &str = "lz4";
pub const CRA_DRIVER_NAME: &str = "lz4-scomp";
pub const MODULE_DESCRIPTION: &str = "LZ4 Compression Algorithm";
pub const MODULE_ALIAS_CRYPTO: &str = "lz4";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScompAlg {
    pub cra_name: &'static str,
    pub cra_driver_name: &'static str,
    pub alloc_ctx_size: usize,
}

pub const LZ4_SCOMP: ScompAlg = ScompAlg {
    cra_name: CRA_NAME,
    cra_driver_name: CRA_DRIVER_NAME,
    alloc_ctx_size: LZ4_MEM_COMPRESS,
};

pub fn lz4_compress_result(out_len: i32, dlen: &mut usize) -> Result<(), i32> {
    if out_len == 0 {
        return Err(-EINVAL);
    }
    *dlen = out_len as usize;
    Ok(())
}

pub fn lz4_decompress_result(out_len: i32, dlen: &mut usize) -> Result<(), i32> {
    if out_len < 0 {
        return Err(-EINVAL);
    }
    *dlen = out_len as usize;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lz4_matches_linux_scomp_registration() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/crypto/lz4.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/lz4.h"
        ));
        assert!(source.contains("ctx = vmalloc(LZ4_MEM_COMPRESS);"));
        assert!(source.contains("LZ4_compress_default(src, dst,"));
        assert!(source.contains("if (!out_len)"));
        assert!(source.contains("return -EINVAL;"));
        assert!(source.contains("LZ4_decompress_safe(src, dst, slen, *dlen);"));
        assert!(source.contains(".cra_name\t= \"lz4\""));
        assert!(source.contains(".cra_driver_name = \"lz4-scomp\""));
        assert!(source.contains("crypto_register_scomp(&scomp);"));
        assert!(source.contains("crypto_unregister_scomp(&scomp);"));
        assert!(source.contains("MODULE_DESCRIPTION(\"LZ4 Compression Algorithm\")"));
        assert!(source.contains("MODULE_ALIAS_CRYPTO(\"lz4\")"));
        assert!(header.contains("#define LZ4_MEMORY_USAGE 14"));
        assert!(header.contains("#define LZ4_MEM_COMPRESS\tLZ4_STREAMSIZE"));

        assert_eq!(LZ4_MEM_COMPRESS, 16_416);
        assert_eq!(LZ4_SCOMP.cra_driver_name, "lz4-scomp");
        let mut dlen = 0;
        assert_eq!(lz4_compress_result(0, &mut dlen), Err(-EINVAL));
        assert_eq!(lz4_compress_result(7, &mut dlen), Ok(()));
        assert_eq!(dlen, 7);
        assert_eq!(lz4_decompress_result(-1, &mut dlen), Err(-EINVAL));
    }
}
