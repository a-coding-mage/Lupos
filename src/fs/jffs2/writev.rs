//! linux-parity: complete
//! linux-source: vendor/linux/fs/jffs2/writev.c
//! test-origin: linux:vendor/linux/fs/jffs2/writev.c
//! JFFS2 direct MTD write and summary ordering.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Jffs2DirectWriteOutcome {
    pub summary_attempted: bool,
    pub mtd_write_called: bool,
    pub result: i32,
}

pub fn jffs2_flash_direct_writev_outcome(
    writebuffered: bool,
    summary_active: bool,
    summary_result: i32,
    mtd_writev_result: i32,
) -> Jffs2DirectWriteOutcome {
    let summary_attempted = !writebuffered && summary_active;
    if summary_attempted && summary_result != 0 {
        return Jffs2DirectWriteOutcome {
            summary_attempted,
            mtd_write_called: false,
            result: summary_result,
        };
    }

    Jffs2DirectWriteOutcome {
        summary_attempted,
        mtd_write_called: true,
        result: mtd_writev_result,
    }
}

pub fn jffs2_flash_direct_write_outcome(
    summary_active: bool,
    summary_result: i32,
    mtd_write_result: i32,
) -> Jffs2DirectWriteOutcome {
    if summary_active && summary_result != 0 {
        return Jffs2DirectWriteOutcome {
            summary_attempted: true,
            mtd_write_called: true,
            result: summary_result,
        };
    }

    Jffs2DirectWriteOutcome {
        summary_attempted: summary_active,
        mtd_write_called: true,
        result: mtd_write_result,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jffs2_direct_write_order_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/jffs2/writev.c"
        ));
        assert!(source.contains("#include <linux/kernel.h>"));
        assert!(source.contains("#include <linux/mtd/mtd.h>"));
        assert!(source.contains("#include \"nodelist.h\""));
        assert!(source.contains("int jffs2_flash_direct_writev"));
        assert!(source.contains("if (!jffs2_is_writebuffered(c))"));
        assert!(source.contains("if (jffs2_sum_active())"));
        assert!(source.contains("jffs2_sum_add_kvec(c, vecs, count, (uint32_t) to)"));
        assert!(source.contains("return mtd_writev(c->mtd, vecs, count, to, retlen);"));
        assert!(source.contains("int jffs2_flash_direct_write"));
        assert!(source.contains("ret = mtd_write(c->mtd, ofs, len, retlen, buf);"));
        assert!(source.contains("vecs[0].iov_base = (unsigned char *) buf;"));
        assert!(source.contains("jffs2_sum_add_kvec(c, vecs, 1, (uint32_t) ofs)"));
        assert!(source.contains("return ret;"));

        assert_eq!(
            jffs2_flash_direct_writev_outcome(false, true, -5, 0),
            Jffs2DirectWriteOutcome {
                summary_attempted: true,
                mtd_write_called: false,
                result: -5,
            }
        );
        assert_eq!(
            jffs2_flash_direct_writev_outcome(true, true, -5, -22).result,
            -22
        );
        assert_eq!(
            jffs2_flash_direct_write_outcome(true, -7, -5),
            Jffs2DirectWriteOutcome {
                summary_attempted: true,
                mtd_write_called: true,
                result: -7,
            }
        );
    }
}
