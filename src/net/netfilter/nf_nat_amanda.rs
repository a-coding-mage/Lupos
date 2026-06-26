//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/nf_nat_amanda.c
//! test-origin: linux:vendor/linux/net/netfilter/nf_nat_amanda.c
//! Amanda NAT helper expectation rewriting.

use core::sync::atomic::{AtomicBool, Ordering};

pub const NAT_HELPER_NAME: &str = "amanda";
pub const MODULE_AUTHOR: &str = "Brian J. Murrell <netfilter@interlinx.bc.ca>";
pub const MODULE_DESCRIPTION: &str = "Amanda NAT helper";
pub const MODULE_LICENSE: &str = "GPL";
pub const NF_DROP: u32 = 0;
pub const NF_ACCEPT: u32 = 1;
pub const IP_CT_DIR_ORIGINAL: u8 = 0;

static NF_NAT_AMANDA_HOOK_REGISTERED: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NfConntrackExpect {
    pub tuple_dst_tcp_port: u16,
    pub saved_tcp_port: u16,
    pub dir: u8,
    pub expectfn_follow_master: bool,
    pub unexpect_related: bool,
}

impl NfConntrackExpect {
    pub const fn new(tuple_dst_tcp_port: u16) -> Self {
        Self {
            tuple_dst_tcp_port,
            saved_tcp_port: 0,
            dir: IP_CT_DIR_ORIGINAL,
            expectfn_follow_master: false,
            unexpect_related: false,
        }
    }
}

pub fn help(exp: &mut NfConntrackExpect, found_port: u16, mangle_success: bool) -> u32 {
    exp.saved_tcp_port = exp.tuple_dst_tcp_port;
    exp.dir = IP_CT_DIR_ORIGINAL;
    exp.expectfn_follow_master = true;

    if found_port == 0 {
        return NF_DROP;
    }
    if !mangle_success {
        exp.unexpect_related = true;
        return NF_DROP;
    }
    NF_ACCEPT
}

pub fn nf_nat_amanda_init() -> Result<(), &'static str> {
    if NF_NAT_AMANDA_HOOK_REGISTERED.swap(true, Ordering::AcqRel) {
        return Err("nf_nat_amanda_hook already registered");
    }
    Ok(())
}

pub fn nf_nat_amanda_fini() {
    NF_NAT_AMANDA_HOOK_REGISTERED.store(false, Ordering::Release);
}

pub fn nf_nat_amanda_hook_registered() -> bool {
    NF_NAT_AMANDA_HOOK_REGISTERED.load(Ordering::Acquire)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nf_nat_amanda_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/nf_nat_amanda.c"
        ));
        assert!(source.contains("#define NAT_HELPER_NAME \"amanda\""));
        assert!(source.contains("MODULE_AUTHOR(\"Brian J. Murrell"));
        assert!(source.contains("MODULE_DESCRIPTION(\"Amanda NAT helper\")"));
        assert!(source.contains("MODULE_ALIAS_NF_NAT_HELPER(NAT_HELPER_NAME);"));
        assert!(source.contains("NF_CT_NAT_HELPER_INIT(NAT_HELPER_NAME)"));
        assert!(source.contains("exp->saved_proto.tcp.port = exp->tuple.dst.u.tcp.port;"));
        assert!(source.contains("exp->dir = IP_CT_DIR_ORIGINAL;"));
        assert!(source.contains("exp->expectfn = nf_nat_follow_master;"));
        assert!(source.contains("port = nf_nat_exp_find_port"));
        assert!(source.contains("return NF_DROP;"));
        assert!(source.contains("nf_nat_mangle_udp_packet"));
        assert!(source.contains("nf_ct_unexpect_related(exp);"));
        assert!(source.contains("return NF_ACCEPT;"));
        assert!(source.contains("RCU_INIT_POINTER(nf_nat_amanda_hook, help);"));

        let mut exp = NfConntrackExpect::new(10_001);
        assert_eq!(help(&mut exp, 10_002, true), NF_ACCEPT);
        assert_eq!(exp.saved_tcp_port, 10_001);
        assert_eq!(exp.dir, IP_CT_DIR_ORIGINAL);
        assert!(exp.expectfn_follow_master);

        assert_eq!(help(&mut exp, 0, true), NF_DROP);
        assert!(!exp.unexpect_related);
        assert_eq!(help(&mut exp, 10_003, false), NF_DROP);
        assert!(exp.unexpect_related);

        nf_nat_amanda_fini();
        assert_eq!(nf_nat_amanda_init(), Ok(()));
        assert!(nf_nat_amanda_hook_registered());
        assert!(nf_nat_amanda_init().is_err());
        nf_nat_amanda_fini();
    }
}
