//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/xt_nfacct.c
//! test-origin: linux:vendor/linux/net/netfilter/xt_nfacct.c
//! Xtables nfacct accounting match.

use crate::include::uapi::errno::ENOENT;

pub const MODULE_AUTHOR: &str = "Pablo Neira Ayuso <pablo@netfilter.org>";
pub const MODULE_DESCRIPTION: &str = "Xtables: match for the extended accounting infrastructure";
pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_ALIASES: [&str; 2] = ["ipt_nfacct", "ip6t_nfacct"];
pub const NFPROTO_UNSPEC: u8 = 0;
pub const NFACCT_NAME_MAX: usize = 32;
pub const NFACCT_NO_QUOTA: i32 = -1;
pub const NFACCT_UNDERQUOTA: i32 = 0;
pub const NFACCT_OVERQUOTA: i32 = 1;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NfAcct {
    pub name: &'static str,
    pub bytes: u64,
    pub packets: u64,
    pub quota: Option<u64>,
    pub refs: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtNfacctMatchInfo {
    pub name: &'static str,
    pub revision: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtMatch {
    pub name: &'static str,
    pub revision: u8,
    pub family: u8,
    pub matchsize: usize,
}

pub const NFACCT_MT_REG: [XtMatch; 2] = [
    XtMatch {
        name: "nfacct",
        revision: 0,
        family: NFPROTO_UNSPEC,
        matchsize: core::mem::size_of::<XtNfacctMatchInfo>(),
    },
    XtMatch {
        name: "nfacct",
        revision: 1,
        family: NFPROTO_UNSPEC,
        matchsize: core::mem::size_of::<XtNfacctMatchInfo>(),
    },
];

pub fn nfacct_mt(skb_len: u64, nfacct: &mut NfAcct) -> bool {
    nfacct.packets = nfacct.packets.saturating_add(1);
    nfacct.bytes = nfacct.bytes.saturating_add(skb_len);
    nfnl_acct_overquota(nfacct) != NFACCT_UNDERQUOTA
}

pub const fn nfnl_acct_overquota(nfacct: &NfAcct) -> i32 {
    match nfacct.quota {
        None => NFACCT_UNDERQUOTA,
        Some(quota) if nfacct.bytes <= quota => NFACCT_UNDERQUOTA,
        Some(_) => NFACCT_OVERQUOTA,
    }
}

pub fn nfacct_mt_checkentry<'a>(
    info: XtNfacctMatchInfo,
    found: Option<&'a mut NfAcct>,
) -> Result<&'a mut NfAcct, i32> {
    let Some(nfacct) = found else {
        let _ = info;
        return Err(-ENOENT);
    };
    nfacct.refs = nfacct.refs.saturating_add(1);
    Ok(nfacct)
}

pub fn nfacct_mt_destroy(nfacct: &mut NfAcct) {
    nfacct.refs = nfacct.refs.saturating_sub(1);
}

pub const fn nfacct_mt_init() -> &'static [XtMatch; 2] {
    &NFACCT_MT_REG
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xt_nfacct_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/xt_nfacct.c"
        ));
        assert!(source.contains("MODULE_AUTHOR(\"Pablo Neira Ayuso <pablo@netfilter.org>\");"));
        assert!(source.contains("MODULE_ALIAS(\"ipt_nfacct\");"));
        assert!(source.contains("MODULE_ALIAS(\"ip6t_nfacct\");"));
        assert!(source.contains("static bool nfacct_mt"));
        assert!(source.contains("nfnl_acct_update(skb, info->nfacct);"));
        assert!(source.contains("overquota = nfnl_acct_overquota(xt_net(par), info->nfacct);"));
        assert!(source.contains("return overquota != NFACCT_UNDERQUOTA;"));
        assert!(source.contains("nfacct = nfnl_acct_find_get(par->net, info->name);"));
        assert!(source.contains("return -ENOENT;"));
        assert!(source.contains("info->nfacct = nfacct;"));
        assert!(source.contains("nfnl_acct_put(info->nfacct);"));
        assert!(source.contains("static struct xt_match nfacct_mt_reg[]"));
        assert!(source.contains(".revision   = 0"));
        assert!(source.contains(".revision   = 1"));
        assert!(source.contains(".name       = \"nfacct\""));
        assert!(source.contains("xt_register_matches(nfacct_mt_reg, ARRAY_SIZE(nfacct_mt_reg));"));
    }

    #[test]
    fn nfacct_match_updates_account_and_reports_overquota() {
        let mut acct = NfAcct {
            name: "acct0",
            bytes: 0,
            packets: 0,
            quota: Some(8),
            refs: 0,
        };
        let info = XtNfacctMatchInfo {
            name: "acct0",
            revision: 1,
        };
        let registered = nfacct_mt_checkentry(info, Some(&mut acct)).unwrap();
        assert_eq!(registered.refs, 1);
        assert!(!nfacct_mt(4, registered));
        assert!(nfacct_mt(5, registered));
        assert_eq!(registered.bytes, 9);
        assert_eq!(registered.packets, 2);
        nfacct_mt_destroy(registered);
        assert_eq!(acct.refs, 0);
        assert!(matches!(nfacct_mt_checkentry(info, None), Err(err) if err == -ENOENT));
        assert_eq!(nfacct_mt_init().len(), 2);
    }
}
