//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/nf_nat_tftp.c
//! test-origin: linux:vendor/linux/net/netfilter/nf_nat_tftp.c
//! TFTP NAT helper expectation setup.

use core::sync::atomic::{AtomicBool, Ordering};

pub const NAT_HELPER_NAME: &str = "tftp";
pub const MODULE_AUTHOR: &str = "Magnus Boden <mb@ozaba.mine.nu>";
pub const MODULE_DESCRIPTION: &str = "TFTP NAT helper";
pub const MODULE_LICENSE: &str = "GPL";
pub const NF_DROP: u32 = 0;
pub const NF_ACCEPT: u32 = 1;
pub const IP_CT_DIR_ORIGINAL: u8 = 0;
pub const IP_CT_DIR_REPLY: u8 = 1;

static NF_NAT_TFTP_HOOK_REGISTERED: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NfConn {
    pub original_src_udp_port: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NfConntrackExpect {
    pub master: NfConn,
    pub saved_udp_port: u16,
    pub dir: u8,
    pub expectfn_follow_master: bool,
    pub expect_related_result: i32,
}

impl NfConntrackExpect {
    pub const fn new(master: NfConn) -> Self {
        Self {
            master,
            saved_udp_port: 0,
            dir: IP_CT_DIR_ORIGINAL,
            expectfn_follow_master: false,
            expect_related_result: 0,
        }
    }
}

pub fn help(exp: &mut NfConntrackExpect) -> u32 {
    exp.saved_udp_port = exp.master.original_src_udp_port;
    exp.dir = IP_CT_DIR_REPLY;
    exp.expectfn_follow_master = true;
    if exp.expect_related_result != 0 {
        NF_DROP
    } else {
        NF_ACCEPT
    }
}

pub fn nf_nat_tftp_init() -> Result<(), &'static str> {
    if NF_NAT_TFTP_HOOK_REGISTERED.swap(true, Ordering::AcqRel) {
        return Err("nf_nat_tftp_hook already registered");
    }
    Ok(())
}

pub fn nf_nat_tftp_fini() {
    NF_NAT_TFTP_HOOK_REGISTERED.store(false, Ordering::Release);
}

pub fn nf_nat_tftp_hook_registered() -> bool {
    NF_NAT_TFTP_HOOK_REGISTERED.load(Ordering::Acquire)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nf_nat_tftp_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/nf_nat_tftp.c"
        ));
        assert!(source.contains("#define NAT_HELPER_NAME \"tftp\""));
        assert!(source.contains("MODULE_ALIAS_NF_NAT_HELPER(NAT_HELPER_NAME);"));
        assert!(source.contains("NF_CT_NAT_HELPER_INIT(NAT_HELPER_NAME)"));
        assert!(source.contains("exp->saved_proto.udp.port"));
        assert!(source.contains("tuple.src.u.udp.port"));
        assert!(source.contains("exp->dir = IP_CT_DIR_REPLY;"));
        assert!(source.contains("exp->expectfn = nf_nat_follow_master;"));
        assert!(source.contains("nf_ct_expect_related(exp, 0) != 0"));
        assert!(source.contains("return NF_DROP;"));
        assert!(source.contains("return NF_ACCEPT;"));
        assert!(source.contains("RCU_INIT_POINTER(nf_nat_tftp_hook, help);"));

        let mut exp = NfConntrackExpect::new(NfConn {
            original_src_udp_port: 69,
        });
        assert_eq!(help(&mut exp), NF_ACCEPT);
        assert_eq!(exp.saved_udp_port, 69);
        assert_eq!(exp.dir, IP_CT_DIR_REPLY);
        assert!(exp.expectfn_follow_master);

        exp.expect_related_result = -1;
        assert_eq!(help(&mut exp), NF_DROP);

        nf_nat_tftp_fini();
        assert_eq!(nf_nat_tftp_init(), Ok(()));
        assert!(nf_nat_tftp_hook_registered());
        assert!(nf_nat_tftp_init().is_err());
        nf_nat_tftp_fini();
        assert!(!nf_nat_tftp_hook_registered());
    }
}
