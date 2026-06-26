//! linux-parity: complete
//! linux-source: vendor/linux/fs/udf/lowlevel.c
//! test-origin: linux:vendor/linux/fs/udf/lowlevel.c
//! UDF low-level CD-ROM session and last-block helpers.

pub const CDROM_LBA: u8 = 0x01;
pub const UDF_PBLK_MAX: u64 = u32::MAX as u64;

pub const fn udf_get_last_session(
    cdrom_device_present: bool,
    multisession_ok: bool,
    xa_flag: bool,
    lba: u32,
) -> u32 {
    if !cdrom_device_present {
        return 0;
    }
    if multisession_ok && xa_flag {
        return lba;
    }
    0
}

pub const fn udf_get_last_block(
    cdrom_device_present: bool,
    last_written_ok: bool,
    last_written_lblock: u64,
    device_blocks: u64,
) -> u32 {
    let mut lblock = last_written_lblock;
    if !cdrom_device_present || !last_written_ok || lblock == 0 {
        if device_blocks > UDF_PBLK_MAX {
            return 0;
        }
        lblock = device_blocks;
    }
    if lblock != 0 { (lblock - 1) as u32 } else { 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn udf_lowlevel_helpers_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/udf/lowlevel.c"
        ));
        assert!(source.contains("#include \"udfdecl.h\""));
        assert!(source.contains("#include <linux/blkdev.h>"));
        assert!(source.contains("#include <linux/cdrom.h>"));
        assert!(source.contains("#include <linux/uaccess.h>"));
        assert!(source.contains("#include \"udf_sb.h\""));
        assert!(source.contains("unsigned int udf_get_last_session"));
        assert!(source.contains("if (!cdi)"));
        assert!(source.contains("ms_info.addr_format = CDROM_LBA;"));
        assert!(source.contains("if (cdrom_multisession(cdi, &ms_info) == 0)"));
        assert!(source.contains("if (ms_info.xa_flag)"));
        assert!(source.contains("return ms_info.addr.lba;"));
        assert!(source.contains("udf_pblk_t udf_get_last_block"));
        assert!(source.contains("cdrom_get_last_written(cdi, &lblock)"));
        assert!(source.contains("sb_bdev_nr_blocks(sb) > ~(udf_pblk_t)0"));
        assert!(source.contains("return lblock - 1;"));

        assert_eq!(udf_get_last_session(false, true, true, 44), 0);
        assert_eq!(udf_get_last_session(true, true, false, 44), 0);
        assert_eq!(udf_get_last_session(true, true, true, 44), 44);
        assert_eq!(udf_get_last_block(true, true, 200, 400), 199);
        assert_eq!(udf_get_last_block(false, false, 0, 400), 399);
        assert_eq!(udf_get_last_block(false, false, 0, UDF_PBLK_MAX + 1), 0);
    }
}
