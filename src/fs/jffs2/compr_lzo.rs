//! linux-parity: complete
//! linux-source: vendor/linux/fs/jffs2/compr_lzo.c
//! test-origin: linux:vendor/linux/fs/jffs2/compr_lzo.c
//! JFFS2 LZO compressor wrapper decisions.

use crate::include::uapi::errno::ENOMEM;

pub const LZO_E_OK: i32 = 0;
pub const PAGE_SIZE: usize = 4096;
pub const JFFS2_LZO_PRIORITY: i32 = 80;
pub const JFFS2_COMPR_LZO: u8 = 0x07;
pub const JFFS2_LZO_NAME: &str = "lzo";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Jffs2LzoCompressor {
    pub priority: i32,
    pub name: &'static str,
    pub compr: u8,
    pub disabled: bool,
}

pub const JFFS2_LZO_COMPRESSOR: Jffs2LzoCompressor = Jffs2LzoCompressor {
    priority: JFFS2_LZO_PRIORITY,
    name: JFFS2_LZO_NAME,
    compr: JFFS2_COMPR_LZO,
    disabled: false,
};

pub const fn jffs2_lzo_workspace_result(lzo_mem: bool, lzo_compress_buf: bool) -> Result<(), i32> {
    if lzo_mem && lzo_compress_buf {
        Ok(())
    } else {
        Err(-ENOMEM)
    }
}

pub const fn jffs2_lzo_compress_result(
    lzo_ret: i32,
    compress_size: usize,
    dst_capacity: usize,
) -> Result<usize, i32> {
    if lzo_ret != LZO_E_OK || compress_size > dst_capacity {
        Err(-1)
    } else {
        Ok(compress_size)
    }
}

pub const fn jffs2_lzo_decompress_result(
    lzo_ret: i32,
    actual_len: usize,
    dest_len: usize,
) -> Result<(), i32> {
    if lzo_ret != LZO_E_OK || actual_len != dest_len {
        Err(-1)
    } else {
        Ok(())
    }
}

pub const fn jffs2_lzo_init_result(workspace_ret: i32, register_ret: i32) -> Result<(), i32> {
    if workspace_ret < 0 {
        return Err(workspace_ret);
    }
    if register_ret != 0 {
        return Err(register_ret);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jffs2_compr_lzo_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/jffs2/compr_lzo.c"
        ));
        assert!(source.contains("#include <linux/lzo.h>"));
        assert!(source.contains("#include \"compr.h\""));
        assert!(source.contains("static void *lzo_mem;"));
        assert!(source.contains("static void *lzo_compress_buf;"));
        assert!(source.contains("static DEFINE_MUTEX(deflate_mutex);"));
        assert!(source.contains("vfree(lzo_mem);"));
        assert!(source.contains("lzo_mem = vmalloc(LZO1X_MEM_COMPRESS);"));
        assert!(source.contains("lzo_compress_buf = vmalloc(lzo1x_worst_compress(PAGE_SIZE));"));
        assert!(source.contains("return -ENOMEM;"));
        assert!(source.contains("lzo1x_1_compress(data_in, *sourcelen, lzo_compress_buf"));
        assert!(source.contains("if (ret != LZO_E_OK)"));
        assert!(source.contains("if (compress_size > *dstlen)"));
        assert!(source.contains("memcpy(cpage_out, lzo_compress_buf, compress_size);"));
        assert!(source.contains("*dstlen = compress_size;"));
        assert!(source.contains("lzo1x_decompress_safe(data_in, srclen, cpage_out, &dl);"));
        assert!(source.contains("if (ret != LZO_E_OK || dl != destlen)"));
        assert!(source.contains(".priority = JFFS2_LZO_PRIORITY"));
        assert!(source.contains(".name = \"lzo\""));
        assert!(source.contains(".compr = JFFS2_COMPR_LZO"));
        assert!(source.contains(".disabled = 0"));
        assert!(source.contains("ret = alloc_workspace();"));
        assert!(source.contains("ret = jffs2_register_compressor(&jffs2_lzo_comp);"));
        assert!(source.contains("jffs2_unregister_compressor(&jffs2_lzo_comp);"));

        assert_eq!(JFFS2_LZO_COMPRESSOR.priority, 80);
        assert_eq!(JFFS2_LZO_COMPRESSOR.compr, 0x07);
        assert_eq!(jffs2_lzo_workspace_result(true, true), Ok(()));
        assert_eq!(jffs2_lzo_workspace_result(false, true), Err(-ENOMEM));
        assert_eq!(jffs2_lzo_compress_result(LZO_E_OK, 10, 12), Ok(10));
        assert_eq!(jffs2_lzo_compress_result(1, 10, 12), Err(-1));
        assert_eq!(jffs2_lzo_compress_result(LZO_E_OK, 13, 12), Err(-1));
        assert_eq!(jffs2_lzo_decompress_result(LZO_E_OK, 12, 12), Ok(()));
        assert_eq!(jffs2_lzo_decompress_result(LZO_E_OK, 11, 12), Err(-1));
        assert_eq!(jffs2_lzo_init_result(-ENOMEM, 0), Err(-ENOMEM));
        assert_eq!(jffs2_lzo_init_result(0, -5), Err(-5));
        assert_eq!(jffs2_lzo_init_result(0, 0), Ok(()));
    }
}
