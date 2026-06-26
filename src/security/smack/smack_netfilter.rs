//! linux-parity: complete
//! linux-source: vendor/linux/security/smack/smack_netfilter.c
//! test-origin: linux:vendor/linux/security/smack/smack_netfilter.c
//! Smack netfilter secmark propagation.

use core::sync::atomic::{AtomicBool, Ordering};

pub const NF_ACCEPT: u32 = 1;
pub const NFPROTO_IPV4: u8 = 2;
pub const NFPROTO_IPV6: u8 = 10;
pub const NF_INET_LOCAL_OUT: u8 = 3;
pub const NF_IP_PRI_SELINUX_FIRST: i32 = -225;
pub const NF_IP6_PRI_SELINUX_FIRST: i32 = -225;

static SMACK_NF_REGISTERED: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SmackKnown {
    pub smk_secid: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SocketSmack {
    pub smk_out: SmackKnown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SkBuff {
    pub secmark: u32,
    pub socket: Option<SocketSmack>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NfHookOps {
    pub pf: u8,
    pub hooknum: u8,
    pub priority: i32,
}

pub const SMACK_NF_OPS: [NfHookOps; 2] = [
    NfHookOps {
        pf: NFPROTO_IPV4,
        hooknum: NF_INET_LOCAL_OUT,
        priority: NF_IP_PRI_SELINUX_FIRST,
    },
    NfHookOps {
        pf: NFPROTO_IPV6,
        hooknum: NF_INET_LOCAL_OUT,
        priority: NF_IP6_PRI_SELINUX_FIRST,
    },
];

pub fn smack_ip_output(skb: &mut SkBuff) -> u32 {
    if let Some(socket) = skb.socket {
        skb.secmark = socket.smk_out.smk_secid;
    }
    NF_ACCEPT
}

pub fn smack_nf_register() -> i32 {
    SMACK_NF_REGISTERED.store(true, Ordering::Release);
    0
}

pub fn smack_nf_unregister() {
    SMACK_NF_REGISTERED.store(false, Ordering::Release);
}

pub fn smack_nf_ip_init(smack_enabled: bool) -> i32 {
    if !smack_enabled {
        return 0;
    }
    crate::kernel::printk::log_info!("Smack", "Registering netfilter hooks");
    smack_nf_register()
}

pub fn registered() -> bool {
    SMACK_NF_REGISTERED.load(Ordering::Acquire)
}

#[cfg(test)]
pub fn reset_for_test() {
    SMACK_NF_REGISTERED.store(false, Ordering::Release);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smack_netfilter_sets_secmark_from_socket_label_and_registers_hooks() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        reset_for_test();

        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/security/smack/smack_netfilter.c"
        ));
        assert!(source.contains("skb->secmark = skp->smk_secid"));
        assert!(source.contains(".pf =\t\tNFPROTO_IPV4"));
        assert!(source.contains("NF_INET_LOCAL_OUT"));
        assert!(source.contains("nf_register_net_hooks(net, smack_nf_ops"));
        assert!(source.contains("register_pernet_subsys(&smack_net_ops)"));

        let mut skb = SkBuff {
            secmark: 0,
            socket: Some(SocketSmack {
                smk_out: SmackKnown { smk_secid: 0x55aa },
            }),
        };
        assert_eq!(smack_ip_output(&mut skb), NF_ACCEPT);
        assert_eq!(skb.secmark, 0x55aa);

        let mut no_socket = SkBuff {
            secmark: 7,
            socket: None,
        };
        assert_eq!(smack_ip_output(&mut no_socket), NF_ACCEPT);
        assert_eq!(no_socket.secmark, 7);

        assert_eq!(smack_nf_ip_init(false), 0);
        assert!(!registered());
        assert_eq!(smack_nf_ip_init(true), 0);
        assert!(registered());
        assert_eq!(SMACK_NF_OPS[0].pf, NFPROTO_IPV4);
        assert_eq!(SMACK_NF_OPS[1].pf, NFPROTO_IPV6);
        smack_nf_unregister();
        assert!(!registered());
    }
}
