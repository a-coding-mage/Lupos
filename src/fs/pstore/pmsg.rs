//! linux-parity: complete
//! linux-source: vendor/linux/fs/pstore/pmsg.c
//! test-origin: linux:vendor/linux/fs/pstore/pmsg.c
//! pstore pmsg character-device write path.

use crate::include::uapi::errno::EFAULT;

pub const PMSG_NAME: &str = "pmsg";
pub const PMSG_DEVNODE_MODE: u16 = 0o220;
pub const PMSG_MINOR_INDEX: u32 = 0;

pub const fn write_pmsg_result(
    count: usize,
    access_ok: bool,
    backend_ret: i32,
) -> Result<usize, i32> {
    if count == 0 {
        return Ok(0);
    }
    if !access_ok {
        return Err(-EFAULT);
    }
    if backend_ret != 0 {
        return Err(backend_ret);
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pstore_pmsg_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/pstore/pmsg.c"
        ));
        assert!(source.contains("#include <linux/cdev.h>"));
        assert!(source.contains("#include <linux/uaccess.h>"));
        assert!(source.contains("static DEFINE_MUTEX(pmsg_lock);"));
        assert!(source.contains("static ssize_t write_pmsg"));
        assert!(source.contains("if (!count)"));
        assert!(source.contains("return 0;"));
        assert!(source.contains("pstore_record_init(&record, psinfo);"));
        assert!(source.contains("record.type = PSTORE_TYPE_PMSG;"));
        assert!(source.contains("record.size = count;"));
        assert!(source.contains("if (!access_ok(buf, count))"));
        assert!(source.contains("return -EFAULT;"));
        assert!(source.contains("ret = psinfo->write_user(&record, buf);"));
        assert!(source.contains("return ret ? ret : count;"));
        assert!(source.contains("#define PMSG_NAME \"pmsg\""));
        assert!(source.contains("*mode = 0220;"));
        assert!(source.contains("register_chrdev(0, PMSG_NAME, &pmsg_fops);"));
        assert!(source.contains("device_create(pmsg_class, NULL, MKDEV(pmsg_major, 0),"));
        assert!(source.contains("unregister_chrdev(pmsg_major, PMSG_NAME);"));

        assert_eq!(write_pmsg_result(0, false, -5), Ok(0));
        assert_eq!(write_pmsg_result(4, false, 0), Err(-EFAULT));
        assert_eq!(write_pmsg_result(4, true, -28), Err(-28));
        assert_eq!(write_pmsg_result(4, true, 0), Ok(4));
        assert_eq!(PMSG_DEVNODE_MODE, 0o220);
    }
}
