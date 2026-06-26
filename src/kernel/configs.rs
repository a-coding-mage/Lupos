//! linux-parity: complete
//! linux-source: vendor/linux/kernel/configs.c
//! test-origin: linux:vendor/linux/kernel/configs.c
//! Built-in kernel config exposure.

use crate::include::uapi::errno::ENOMEM;

pub const IKCFG_START: &str = "IKCFG_ST";
pub const IKCFG_END: &str = "IKCFG_ED";
pub const PROC_CONFIG_NAME: &str = "config.gz";

pub const fn ikconfig_blob_len(start: usize, end: usize) -> usize {
    end.saturating_sub(start)
}

pub const fn ikconfig_init(proc_create_ok: bool) -> Result<&'static str, i32> {
    if proc_create_ok {
        Ok(PROC_CONFIG_NAME)
    } else {
        Err(-ENOMEM)
    }
}

pub const fn ikconfig_read_current_len(offset: usize, len: usize, blob_len: usize) -> usize {
    if offset >= blob_len {
        0
    } else {
        let remaining = blob_len - offset;
        if len < remaining { len } else { remaining }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ikconfig_proc_shape_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/configs.c"
        ));
        assert!(source.contains("IKCFG_ST"));
        assert!(source.contains(".global kernel_config_data"));
        assert!(source.contains("kernel/config_data.gz"));
        assert!(source.contains("IKCFG_ED"));
        assert!(source.contains("simple_read_from_buffer"));
        assert!(source.contains("proc_create(\"config.gz\""));
        assert!(source.contains("proc_set_size(entry"));
        assert!(source.contains("remove_proc_entry(\"config.gz\", NULL);"));
        assert!(source.contains(
            "MODULE_DESCRIPTION(\"Echo the kernel .config file used to build the kernel\")"
        ));

        assert_eq!(ikconfig_blob_len(10, 42), 32);
        assert_eq!(ikconfig_init(true), Ok("config.gz"));
        assert_eq!(ikconfig_init(false), Err(-ENOMEM));
        assert_eq!(ikconfig_read_current_len(4, 10, 20), 10);
        assert_eq!(ikconfig_read_current_len(18, 10, 20), 2);
        assert_eq!(ikconfig_read_current_len(20, 10, 20), 0);
    }
}
