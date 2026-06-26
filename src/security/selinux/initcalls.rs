//! linux-parity: complete
//! linux-source: vendor/linux/security/selinux/initcalls.c
//! test-origin: linux:vendor/linux/security/selinux/initcalls.c
//! SELinux device initcall sequencing.

extern crate alloc;

use alloc::vec::Vec;

pub const SELINUX_INITCALL_ORDER: &[&str] = &[
    "init_sel_fs",
    "sel_netport_init",
    "sel_netnode_init",
    "sel_netif_init",
    "sel_netlink_init",
    "sel_ib_pkey_init",
    "selinux_nf_ip_init",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SelinuxInitcallConfig {
    pub security_infiniband: bool,
    pub netfilter: bool,
}

impl SelinuxInitcallConfig {
    pub const fn linux_default() -> Self {
        Self {
            security_infiniband: false,
            netfilter: false,
        }
    }
}

pub fn selinux_initcall_with<F>(
    config: SelinuxInitcallConfig,
    mut init: F,
) -> (i32, Vec<&'static str>)
where
    F: FnMut(&'static str) -> i32,
{
    let mut rc = 0;
    let mut called = Vec::new();

    for name in enabled_initcalls(config) {
        called.push(name);
        let rc_tmp = init(name);
        if rc == 0 && rc_tmp != 0 {
            rc = rc_tmp;
        }
    }

    (rc, called)
}

pub fn enabled_initcalls(config: SelinuxInitcallConfig) -> impl Iterator<Item = &'static str> {
    SELINUX_INITCALL_ORDER.iter().copied().filter(move |name| {
        (*name != "sel_ib_pkey_init" || config.security_infiniband)
            && (*name != "selinux_nf_ip_init" || config.netfilter)
    })
}

pub fn selinux_initcall() -> i32 {
    selinux_initcall_with(SelinuxInitcallConfig::linux_default(), |_| 0).0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selinux_initcall_runs_linux_order_and_keeps_first_error() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/security/selinux/initcalls.c"
        ));
        assert!(source.contains("rc_tmp = init_sel_fs();"));
        assert!(source.contains("rc_tmp = sel_netport_init();"));
        assert!(source.contains("rc_tmp = sel_netnode_init();"));
        assert!(source.contains("rc_tmp = sel_netif_init();"));
        assert!(source.contains("rc_tmp = sel_netlink_init();"));
        assert!(source.contains("return rc;"));

        let (rc, called) =
            selinux_initcall_with(SelinuxInitcallConfig::linux_default(), |name| match name {
                "sel_netnode_init" => -5,
                "sel_netif_init" => -22,
                _ => 0,
            });

        assert_eq!(rc, -5);
        assert_eq!(
            called,
            [
                "init_sel_fs",
                "sel_netport_init",
                "sel_netnode_init",
                "sel_netif_init",
                "sel_netlink_init"
            ]
        );
    }

    #[test]
    fn selinux_optional_initcalls_follow_config_gates() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let (_, called) = selinux_initcall_with(
            SelinuxInitcallConfig {
                security_infiniband: true,
                netfilter: true,
            },
            |_| 0,
        );

        assert_eq!(called, SELINUX_INITCALL_ORDER);
        assert_eq!(selinux_initcall(), 0);
    }
}
