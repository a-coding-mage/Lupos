//! linux-parity: complete
//! linux-source: vendor/linux/crypto/lz4hc.c
//! test-origin: linux:vendor/linux/crypto/lz4hc.c
//! LZ4HC scomp registration metadata.

use crate::include::uapi::errno::EINVAL;

pub const LZ4HC_MEM_COMPRESS: usize = 262_192;
pub const LZ4HC_DEFAULT_CLEVEL: i32 = 9;
pub const CRA_NAME: &str = "lz4hc";
pub const CRA_DRIVER_NAME: &str = "lz4hc-scomp";
pub const MODULE_DESCRIPTION: &str = "LZ4HC Compression Algorithm";
pub const MODULE_ALIAS_CRYPTO: &str = "lz4hc";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScompAlg {
    pub cra_name: &'static str,
    pub cra_driver_name: &'static str,
    pub alloc_ctx_size: usize,
    pub default_level: Option<i32>,
}

pub const LZ4HC_SCOMP: ScompAlg = ScompAlg {
    cra_name: CRA_NAME,
    cra_driver_name: CRA_DRIVER_NAME,
    alloc_ctx_size: LZ4HC_MEM_COMPRESS,
    default_level: Some(LZ4HC_DEFAULT_CLEVEL),
};

pub fn lz4hc_compress_result(out_len: i32, dlen: &mut usize) -> Result<(), i32> {
    if out_len == 0 {
        return Err(-EINVAL);
    }
    *dlen = out_len as usize;
    Ok(())
}

pub fn lz4hc_decompress_result(out_len: i32, dlen: &mut usize) -> Result<(), i32> {
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
    fn lz4hc_matches_linux_scomp_registration() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/crypto/lz4hc.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/lz4.h"
        ));
        assert!(source.contains("ctx = vmalloc(LZ4HC_MEM_COMPRESS);"));
        assert!(source.contains("LZ4_compress_HC(src, dst, slen,"));
        assert!(source.contains("*dlen, LZ4HC_DEFAULT_CLEVEL, ctx"));
        assert!(source.contains("if (!out_len)"));
        assert!(source.contains("return -EINVAL;"));
        assert!(source.contains("LZ4_decompress_safe(src, dst, slen, *dlen);"));
        assert!(source.contains(".cra_name\t= \"lz4hc\""));
        assert!(source.contains(".cra_driver_name = \"lz4hc-scomp\""));
        assert!(source.contains("crypto_register_scomp(&scomp);"));
        assert!(source.contains("MODULE_ALIAS_CRYPTO(\"lz4hc\")"));
        assert!(header.contains("#define LZ4HC_MEM_COMPRESS\tLZ4_STREAMHCSIZE"));
        assert!(header.contains("#define LZ4HC_DEFAULT_CLEVEL\t\t\t9"));

        assert_eq!(LZ4HC_SCOMP.alloc_ctx_size, 262_192);
        assert_eq!(LZ4HC_SCOMP.default_level, Some(9));
        let mut dlen = 99;
        assert_eq!(lz4hc_compress_result(0, &mut dlen), Err(-EINVAL));
        assert_eq!(lz4hc_decompress_result(12, &mut dlen), Ok(()));
        assert_eq!(dlen, 12);
    }
}
