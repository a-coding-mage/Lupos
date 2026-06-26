//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/nf_conntrack_acct.c
//! test-origin: linux:vendor/linux/net/netfilter/nf_conntrack_acct.c
//! Conntrack accounting module parameter propagation.

use core::sync::atomic::{AtomicBool, Ordering};

static NF_CT_ACCT: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConntrackAcctSysctl {
    pub sysctl_acct: bool,
}

pub fn set_nf_conntrack_acct(enabled: bool) {
    NF_CT_ACCT.store(enabled, Ordering::Release);
}

pub fn nf_conntrack_acct_pernet_init() -> ConntrackAcctSysctl {
    ConntrackAcctSysctl {
        sysctl_acct: NF_CT_ACCT.load(Ordering::Acquire),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nf_conntrack_acct_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/nf_conntrack_acct.c"
        ));
        assert!(source.contains("static bool nf_ct_acct __read_mostly;"));
        assert!(source.contains("module_param_named(acct, nf_ct_acct, bool, 0644);"));
        assert!(source.contains("net->ct.sysctl_acct = nf_ct_acct;"));
        set_nf_conntrack_acct(false);
        assert_eq!(
            nf_conntrack_acct_pernet_init(),
            ConntrackAcctSysctl { sysctl_acct: false }
        );
        set_nf_conntrack_acct(true);
        assert_eq!(
            nf_conntrack_acct_pernet_init(),
            ConntrackAcctSysctl { sysctl_acct: true }
        );
        set_nf_conntrack_acct(false);
    }
}
