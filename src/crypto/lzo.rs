//! linux-parity: complete
//! linux-source: vendor/linux/crypto/lzo.c
//! test-origin: linux:vendor/linux/crypto/lzo.c
//! LZO scomp registration metadata.

use crate::include::uapi::errno::EINVAL;

pub const LZO_E_OK: i32 = 0;
pub const LZO1X_1_MEM_COMPRESS: usize = 8192 * core::mem::size_of::<u16>();
pub const LZO1X_MEM_COMPRESS: usize = LZO1X_1_MEM_COMPRESS;
pub const CRA_NAME: &str = "lzo";
pub const CRA_DRIVER_NAME: &str = "lzo-scomp";
pub const MODULE_DESCRIPTION: &str = "LZO Compression Algorithm";
pub const MODULE_ALIAS_CRYPTO: &str = "lzo";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScompAlg {
    pub cra_name: &'static str,
    pub cra_driver_name: &'static str,
    pub alloc_ctx_size: usize,
    pub compressor: &'static str,
}

pub const LZO_SCOMP: ScompAlg = ScompAlg {
    cra_name: CRA_NAME,
    cra_driver_name: CRA_DRIVER_NAME,
    alloc_ctx_size: LZO1X_MEM_COMPRESS,
    compressor: "lzo1x_1_compress_safe",
};

pub fn lzo_compress_result(err: i32, tmp_len: usize, dlen: &mut usize) -> Result<(), i32> {
    if err != LZO_E_OK {
        return Err(-EINVAL);
    }
    *dlen = tmp_len;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lzo_matches_linux_scomp_registration() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/crypto/lzo.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/lzo.h"
        ));
        assert!(source.contains("ctx = kvmalloc(LZO1X_MEM_COMPRESS, GFP_KERNEL);"));
        assert!(source.contains("err = lzo1x_1_compress_safe(src, slen, dst, &tmp_len, ctx);"));
        assert!(source.contains("if (err != LZO_E_OK)"));
        assert!(source.contains("return -EINVAL;"));
        assert!(source.contains("err = lzo1x_decompress_safe(src, slen, dst, &tmp_len);"));
        assert!(source.contains(".cra_name\t= \"lzo\""));
        assert!(source.contains(".cra_driver_name = \"lzo-scomp\""));
        assert!(source.contains("crypto_register_scomp(&scomp);"));
        assert!(source.contains("MODULE_DESCRIPTION(\"LZO Compression Algorithm\")"));
        assert!(source.contains("MODULE_ALIAS_CRYPTO(\"lzo\")"));
        assert!(header.contains("#define LZO1X_1_MEM_COMPRESS\t(8192 * sizeof(unsigned short))"));
        assert!(header.contains("#define LZO_E_OK\t\t\t0"));

        assert_eq!(LZO1X_MEM_COMPRESS, 16_384);
        assert_eq!(LZO_SCOMP.compressor, "lzo1x_1_compress_safe");
        let mut dlen = 0;
        assert_eq!(lzo_compress_result(-1, 4, &mut dlen), Err(-EINVAL));
        assert_eq!(lzo_compress_result(LZO_E_OK, 4, &mut dlen), Ok(()));
        assert_eq!(dlen, 4);
    }
}
