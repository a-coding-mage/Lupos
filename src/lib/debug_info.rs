//! linux-parity: complete
//! linux-source: vendor/linux/lib/debug_info.c
//! test-origin: linux:vendor/linux/lib/debug_info.c
//! Debug-info-only core type include list.

pub const CORE_DEBUG_INFO_HEADERS: &[&str] = &[
    "#include <linux/cred.h>",
    "#include <linux/crypto.h>",
    "#include <linux/dcache.h>",
    "#include <linux/device.h>",
    "#include <linux/fs.h>",
    "#include <linux/fscache-cache.h>",
    "#include <linux/io.h>",
    "#include <linux/kallsyms.h>",
    "#include <linux/kernel.h>",
    "#include <linux/kobject.h>",
    "#include <linux/mm.h>",
    "#include <linux/module.h>",
    "#include <linux/net.h>",
    "#include <linux/sched.h>",
    "#include <linux/slab.h>",
    "#include <linux/stdarg.h>",
    "#include <linux/types.h>",
    "#include <net/addrconf.h>",
    "#include <net/sock.h>",
    "#include <net/tcp.h>",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_info_include_contract_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/debug_info.c"
        ));
        assert!(source.contains("Please do not add actual code"));
        for header in CORE_DEBUG_INFO_HEADERS {
            assert!(source.contains(header));
        }
    }
}
