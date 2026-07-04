//! linux-parity: complete
//! linux-source: vendor/linux/init/version.c
//! test-origin: linux:vendor/linux/init/version.c
//! Kernel version, UTS identity, and early hostname parameter handling.
//!
//! Mirrors `vendor/linux/init/version.c` and
//! `vendor/linux/init/version-timestamp.c`: Linux keeps fixed build identity
//! strings in `init_uts_ns`, exposes a banner for printk/proc, and applies
//! the early `hostname=` parameter by copying into the init UTS namespace.

extern crate alloc;

use alloc::format;
use alloc::string::String;

use crate::kernel::utsname::NEW_UTS_LEN_PLUS_NUL;

pub use crate::init::version_timestamp::linux_banner;

pub const UTS_SYSNAME: &str = "Lupos";
pub const UTS_NODENAME: &str = "(none)";
pub const UTS_RELEASE: &str = concat!(env!("CARGO_PKG_VERSION"), "-lupos");
pub const UTS_VERSION: &str = "#1 SMP";
pub const UTS_MACHINE: &str = "x86_64";
pub const UTS_DOMAINNAME: &str = "(none)";
pub const LINUX_COMPILE_BY: &str = "lupos";
pub const LINUX_COMPILE_HOST: &str = "build";
pub const LINUX_COMPILER: &str = "rustc";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HostnameParam {
    pub nodename: [u8; NEW_UTS_LEN_PLUS_NUL],
    pub truncated: bool,
}

pub fn linux_proc_banner(sysname: &str, release: &str, version: &str) -> String {
    format!(
        "{} version {} ({}@{}) ({}) {}\n",
        sysname, release, LINUX_COMPILE_BY, LINUX_COMPILE_HOST, LINUX_COMPILER, version
    )
}

pub fn hostname_from_param(arg: &str) -> HostnameParam {
    let bytes = arg.as_bytes();
    let max = NEW_UTS_LEN_PLUS_NUL - 1;
    let len = bytes.len().min(max);
    let mut nodename = [0u8; NEW_UTS_LEN_PLUS_NUL];
    nodename[..len].copy_from_slice(&bytes[..len]);
    HostnameParam {
        nodename,
        truncated: bytes.len() > max,
    }
}

pub fn apply_hostname_param(arg: &str) -> HostnameParam {
    let param = hostname_from_param(arg);
    if param.truncated {
        crate::log_warn!(
            "",
            "hostname parameter exceeds {} characters and will be truncated",
            NEW_UTS_LEN_PLUS_NUL - 1
        );
    }
    crate::kernel::utsname::set_current_nodename_packed(param.nodename);
    param
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linux_banner_matches_version_timestamp_shape() {
        let banner = linux_banner();
        assert!(banner.starts_with("Linux version "));
        assert!(banner.contains(UTS_RELEASE));
        assert!(banner.contains("lupos@build"));
        assert!(banner.ends_with('\n'));
    }

    #[test]
    fn proc_banner_uses_runtime_uts_fields() {
        let banner = linux_proc_banner("Lupos", "0.1.0-lupos", "#1 SMP");
        assert_eq!(
            banner,
            "Lupos version 0.1.0-lupos (lupos@build) (rustc) #1 SMP\n"
        );
    }

    #[test]
    fn hostname_parameter_truncates_like_strscpy_destination() {
        let long = "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnop";
        let param = hostname_from_param(long);
        assert!(param.truncated);
        assert_eq!(param.nodename[NEW_UTS_LEN_PLUS_NUL - 1], 0);
        assert_eq!(&param.nodename[..3], b"abc");
    }
}
