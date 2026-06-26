//! linux-parity: complete
//! linux-source: vendor/linux/init/version-timestamp.c
//! test-origin: linux:vendor/linux/init/version-timestamp.c
//! Fixed Linux build identity strings.

use crate::init::version::{
    LINUX_COMPILE_BY, LINUX_COMPILE_HOST, LINUX_COMPILER, UTS_DOMAINNAME, UTS_MACHINE,
    UTS_NODENAME, UTS_RELEASE, UTS_SYSNAME, UTS_VERSION,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InitUtsNamespaceName {
    pub sysname: &'static str,
    pub nodename: &'static str,
    pub release: &'static str,
    pub version: &'static str,
    pub machine: &'static str,
    pub domainname: &'static str,
}

pub const INIT_UTS_NS_NAME: InitUtsNamespaceName = InitUtsNamespaceName {
    sysname: UTS_SYSNAME,
    nodename: UTS_NODENAME,
    release: UTS_RELEASE,
    version: UTS_VERSION,
    machine: UTS_MACHINE,
    domainname: UTS_DOMAINNAME,
};

pub fn linux_banner() -> alloc::string::String {
    alloc::format!(
        "Linux version {} ({}@{}) ({}) {}\n",
        UTS_RELEASE,
        LINUX_COMPILE_BY,
        LINUX_COMPILE_HOST,
        LINUX_COMPILER,
        UTS_VERSION
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_timestamp_source_matches_linux_shape() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/init/version-timestamp.c"
        ));
        assert!(source.contains("#include <generated/compile.h>"));
        assert!(source.contains("#include <generated/utsrelease.h>"));
        assert!(source.contains("struct uts_namespace init_uts_ns"));
        assert!(source.contains("const char linux_banner[]"));
        assert!(source.contains("\"Linux version \" UTS_RELEASE"));
        assert_eq!(INIT_UTS_NS_NAME.release, UTS_RELEASE);
        let banner = linux_banner();
        assert!(banner.starts_with("Linux version "));
        assert!(banner.contains(UTS_RELEASE));
        assert!(banner.ends_with('\n'));
    }
}
