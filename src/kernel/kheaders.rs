//! linux-parity: complete
//! linux-source: vendor/linux/kernel/kheaders.c
//! test-origin: linux:vendor/linux/kernel/kheaders.c
//! Sysfs exposure of the compressed kernel header archive.

pub const KHEADERS_INCBIN: &str = "kernel/kheaders_data.tar.xz";
pub const KHEADERS_SYMBOL_START: &str = "kernel_headers_data";
pub const KHEADERS_SYMBOL_END: &str = "kernel_headers_data_end";
pub const KHEADERS_NAME: &str = "kheaders.tar.xz";
pub const KHEADERS_MODE: u32 = 0o444;
pub const MODULE_LICENSE: &str = "GPL v2";
pub const MODULE_AUTHOR: &str = "Joel Fernandes";
pub const MODULE_DESCRIPTION: &str = "Echo the kernel header artifacts used to build the kernel";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KheadersData<'a> {
    pub start_symbol: &'static str,
    pub end_symbol: &'static str,
    pub incbin_path: &'static str,
    pub bytes: &'a [u8],
}

impl<'a> KheadersData<'a> {
    pub const fn new(bytes: &'a [u8]) -> Self {
        Self {
            start_symbol: KHEADERS_SYMBOL_START,
            end_symbol: KHEADERS_SYMBOL_END,
            incbin_path: KHEADERS_INCBIN,
            bytes,
        }
    }

    pub const fn size(self) -> usize {
        self.bytes.len()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KheadersAttribute<'a> {
    pub name: &'static str,
    pub mode: u32,
    pub private: Option<&'a [u8]>,
    pub size: usize,
}

pub const fn kheaders_attr() -> KheadersAttribute<'static> {
    KheadersAttribute {
        name: KHEADERS_NAME,
        mode: KHEADERS_MODE,
        private: None,
        size: 0,
    }
}

pub const fn ikheaders_init<'a>(
    data: KheadersData<'a>,
    sysfs_create_ret: i32,
) -> (i32, KheadersAttribute<'a>) {
    let attr = KheadersAttribute {
        name: KHEADERS_NAME,
        mode: KHEADERS_MODE,
        private: Some(data.bytes),
        size: data.size(),
    };
    (sysfs_create_ret, attr)
}

pub const fn ikheaders_cleanup<'a>(
    attr: KheadersAttribute<'a>,
) -> (&'static str, KheadersAttribute<'a>) {
    ("sysfs_remove_bin_file", attr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kheaders_sysfs_attribute_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/kheaders.c"
        ));
        assert!(source.contains("kheaders_data.tar.xz"));
        assert!(source.contains("kernel_headers_data"));
        assert!(source.contains("kernel_headers_data_end"));
        assert!(source.contains("kernel/kheaders_data.tar.xz"));
        assert!(source.contains("__BIN_ATTR_SIMPLE_RO(kheaders.tar.xz, 0444)"));
        assert!(source.contains("kheaders_attr.private = kernel_headers_data;"));
        assert!(source.contains("kheaders_attr.size = (kernel_headers_data_end -"));
        assert!(source.contains("sysfs_create_bin_file(kernel_kobj, &kheaders_attr);"));
        assert!(source.contains("sysfs_remove_bin_file(kernel_kobj, &kheaders_attr);"));
        assert!(source.contains("module_init(ikheaders_init);"));
        assert!(source.contains("module_exit(ikheaders_cleanup);"));
        assert!(source.contains("MODULE_LICENSE(\"GPL v2\")"));
        assert!(source.contains("MODULE_AUTHOR(\"Joel Fernandes\")"));
        assert!(source.contains(
            "MODULE_DESCRIPTION(\"Echo the kernel header artifacts used to build the kernel\")"
        ));

        let payload = b"headers";
        let data = KheadersData::new(payload);
        assert_eq!(data.start_symbol, "kernel_headers_data");
        assert_eq!(data.end_symbol, "kernel_headers_data_end");
        assert_eq!(data.incbin_path, "kernel/kheaders_data.tar.xz");
        assert_eq!(data.size(), payload.len());

        let empty_attr = kheaders_attr();
        assert_eq!(empty_attr.name, "kheaders.tar.xz");
        assert_eq!(empty_attr.mode, 0o444);
        assert_eq!(empty_attr.private, None);
        assert_eq!(empty_attr.size, 0);

        let (ret, attr) = ikheaders_init(data, -5);
        assert_eq!(ret, -5);
        assert_eq!(attr.name, "kheaders.tar.xz");
        assert_eq!(attr.mode, 0o444);
        assert_eq!(attr.private, Some(payload.as_slice()));
        assert_eq!(attr.size, payload.len());

        assert_eq!(ikheaders_cleanup(attr).0, "sysfs_remove_bin_file");
    }
}
