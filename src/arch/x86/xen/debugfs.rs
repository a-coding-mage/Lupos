//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/xen/debugfs.c
//! test-origin: linux:vendor/linux/arch/x86/xen/debugfs.c
//! Xen debugfs root directory initialization.

pub const XEN_DEBUGFS_DIR: &str = "xen";

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct XenDebugFs {
    created: bool,
}

impl XenDebugFs {
    pub const fn new() -> Self {
        Self { created: false }
    }

    pub fn xen_init_debugfs(&mut self) -> &'static str {
        self.created = true;
        XEN_DEBUGFS_DIR
    }

    pub const fn is_created(&self) -> bool {
        self.created
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xen_debugfs_dir_is_singleton_named_xen() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/xen/debugfs.c"
        ));
        assert!(source.contains("debugfs_create_dir(\"xen\", NULL)"));

        let mut debugfs = XenDebugFs::new();
        assert!(!debugfs.is_created());
        assert_eq!(debugfs.xen_init_debugfs(), "xen");
        assert!(debugfs.is_created());
        assert_eq!(debugfs.xen_init_debugfs(), "xen");
    }
}
