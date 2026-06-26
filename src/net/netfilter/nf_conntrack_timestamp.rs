//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/nf_conntrack_timestamp.c
//! test-origin: linux:vendor/linux/net/netfilter/nf_conntrack_timestamp.c
//! Conntrack timestamp module parameter propagation.

use core::sync::atomic::{AtomicBool, Ordering};

static NF_CT_TSTAMP: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConntrackSysctl {
    pub sysctl_tstamp: bool,
}

pub fn set_nf_conntrack_tstamp(enabled: bool) {
    NF_CT_TSTAMP.store(enabled, Ordering::Release);
}

pub fn nf_conntrack_tstamp_pernet_init() -> ConntrackSysctl {
    ConntrackSysctl {
        sysctl_tstamp: NF_CT_TSTAMP.load(Ordering::Acquire),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nf_conntrack_timestamp_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/nf_conntrack_timestamp.c"
        ));
        assert!(source.contains("static bool nf_ct_tstamp __read_mostly;"));
        assert!(source.contains("module_param_named(tstamp, nf_ct_tstamp, bool, 0644);"));
        assert!(source.contains("net->ct.sysctl_tstamp = nf_ct_tstamp;"));
        set_nf_conntrack_tstamp(false);
        assert_eq!(
            nf_conntrack_tstamp_pernet_init(),
            ConntrackSysctl {
                sysctl_tstamp: false
            }
        );
        set_nf_conntrack_tstamp(true);
        assert_eq!(
            nf_conntrack_tstamp_pernet_init(),
            ConntrackSysctl {
                sysctl_tstamp: true
            }
        );
        set_nf_conntrack_tstamp(false);
    }
}
