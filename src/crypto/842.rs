//! linux-parity: complete
//! linux-source: vendor/linux/crypto/842.c
//! test-origin: linux:vendor/linux/crypto/842.c
//! Crypto API registration metadata for the software 842 compressor.

use crate::include::uapi::errno::ENOMEM;

pub const SW842_MEM_COMPRESS: usize = 0xf000;
pub const CRA_NAME: &str = "842";
pub const CRA_DRIVER_NAME: &str = "842-scomp";
pub const CRA_PRIORITY: u32 = 100;
pub const MODULE_DESCRIPTION: &str = "842 Software Compression Algorithm";
pub const MODULE_ALIAS_CRYPTO: [&str; 2] = ["842", "842-generic"];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScompAlg {
    pub cra_name: &'static str,
    pub cra_driver_name: &'static str,
    pub cra_priority: u32,
    pub alloc_ctx_size: usize,
}

pub const SCOMP_ALG: ScompAlg = ScompAlg {
    cra_name: CRA_NAME,
    cra_driver_name: CRA_DRIVER_NAME,
    cra_priority: CRA_PRIORITY,
    alloc_ctx_size: SW842_MEM_COMPRESS,
};

pub const fn crypto842_alloc_ctx_size() -> usize {
    SW842_MEM_COMPRESS
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Crypto842Ctx {
    pub len: usize,
}

pub const fn crypto842_alloc_ctx(kmalloc_ok: bool) -> Result<Crypto842Ctx, i32> {
    if !kmalloc_ok {
        return Err(-ENOMEM);
    }

    Ok(Crypto842Ctx {
        len: SW842_MEM_COMPRESS,
    })
}

pub const fn crypto842_free_ctx(_ctx: Crypto842Ctx) {}

pub fn crypto842_scompress<F>(
    src: &[u8],
    dst: &mut [u8],
    dlen: &mut usize,
    ctx: &mut Crypto842Ctx,
    sw842_compress: F,
) -> i32
where
    F: FnOnce(&[u8], &mut [u8], &mut usize, &mut Crypto842Ctx) -> i32,
{
    sw842_compress(src, dst, dlen, ctx)
}

pub fn crypto842_sdecompress<F>(
    src: &[u8],
    dst: &mut [u8],
    dlen: &mut usize,
    _ctx: &mut Crypto842Ctx,
    sw842_decompress: F,
) -> i32
where
    F: FnOnce(&[u8], &mut [u8], &mut usize) -> i32,
{
    sw842_decompress(src, dst, dlen)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crypto_842_matches_linux_scomp_registration() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/crypto/842.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/sw842.h"
        ));
        assert!(source.contains("ctx = kmalloc(SW842_MEM_COMPRESS, GFP_KERNEL);"));
        assert!(source.contains("return sw842_compress(src, slen, dst, dlen, ctx);"));
        assert!(source.contains("return sw842_decompress(src, slen, dst, dlen);"));
        assert!(source.contains(".cra_name\t= \"842\""));
        assert!(source.contains(".cra_driver_name = \"842-scomp\""));
        assert!(source.contains("return crypto_register_scomp(&scomp);"));
        assert!(source.contains("crypto_unregister_scomp(&scomp);"));
        assert!(source.contains("MODULE_ALIAS_CRYPTO(\"842-generic\")"));
        assert!(header.contains("#define SW842_MEM_COMPRESS\t(0xf000)"));

        assert_eq!(crypto842_alloc_ctx_size(), 0xf000);
        assert_eq!(crypto842_alloc_ctx(false), Err(-ENOMEM));
        let mut ctx = crypto842_alloc_ctx(true).expect("ctx");
        assert_eq!(ctx.len, SW842_MEM_COMPRESS);
        assert_eq!(SCOMP_ALG.cra_name, "842");
        assert_eq!(SCOMP_ALG.cra_driver_name, "842-scomp");
        assert_eq!(MODULE_ALIAS_CRYPTO, ["842", "842-generic"]);

        let mut dst = [0u8; 8];
        let mut dlen = dst.len();
        let ret = crypto842_scompress(
            b"abc",
            &mut dst,
            &mut dlen,
            &mut ctx,
            |src, dst, dlen, ctx| {
                assert_eq!(src, b"abc");
                assert_eq!(ctx.len, SW842_MEM_COMPRESS);
                dst[..3].copy_from_slice(src);
                *dlen = 3;
                0
            },
        );
        assert_eq!(ret, 0);
        assert_eq!(&dst[..dlen], b"abc");

        let ret = crypto842_sdecompress(b"xyz", &mut dst, &mut dlen, &mut ctx, |src, dst, dlen| {
            dst[..src.len()].copy_from_slice(src);
            *dlen = src.len();
            7
        });
        assert_eq!(ret, 7);
        assert_eq!(&dst[..dlen], b"xyz");
        crypto842_free_ctx(ctx);
    }
}
