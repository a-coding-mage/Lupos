//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/xt_quota.c
//! test-origin: linux:vendor/linux/net/netfilter/xt_quota.c
//! Xtables countdown quota match.

use crate::include::uapi::errno::{EINVAL, ENOMEM};

pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_AUTHOR: &str = "Sam Johnston <samj@samj.net>";
pub const MODULE_DESCRIPTION: &str = "Xtables: countdown quota match";
pub const MODULE_ALIASES: [&str; 2] = ["ipt_quota", "ip6t_quota"];
pub const XT_QUOTA_INVERT: u32 = 0x1;
pub const XT_QUOTA_MASK: u32 = 0x1;
pub const NFPROTO_UNSPEC: u8 = 0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtQuotaInfo {
    pub quota: u64,
    pub flags: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtQuotaPriv {
    pub quota: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtMatch {
    pub name: &'static str,
    pub revision: u8,
    pub family: u8,
    pub matchsize: usize,
}

pub const QUOTA_MT_REG: XtMatch = XtMatch {
    name: "quota",
    revision: 0,
    family: NFPROTO_UNSPEC,
    matchsize: core::mem::size_of::<XtQuotaInfo>(),
};

pub fn quota_mt(skb_len: u64, info: XtQuotaInfo, priv_: &mut XtQuotaPriv) -> bool {
    let mut ret = info.flags & XT_QUOTA_INVERT != 0;
    if priv_.quota >= skb_len {
        priv_.quota -= skb_len;
        ret = !ret;
    } else {
        priv_.quota = 0;
    }
    ret
}

pub const fn quota_mt_check(info: XtQuotaInfo, alloc_ok: bool) -> Result<XtQuotaPriv, i32> {
    if info.flags & !XT_QUOTA_MASK != 0 {
        return Err(-EINVAL);
    }
    if !alloc_ok {
        return Err(-ENOMEM);
    }
    Ok(XtQuotaPriv { quota: info.quota })
}

pub const fn quota_mt_destroy(_priv_: XtQuotaPriv) -> bool {
    true
}

pub const fn quota_mt_init() -> &'static XtMatch {
    &QUOTA_MT_REG
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xt_quota_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/xt_quota.c"
        ));
        assert!(source.contains("struct xt_quota_priv"));
        assert!(source.contains("uint64_t\tquota;"));
        assert!(source.contains("MODULE_AUTHOR(\"Sam Johnston <samj@samj.net>\");"));
        assert!(source.contains("MODULE_ALIAS(\"ipt_quota\");"));
        assert!(source.contains("MODULE_ALIAS(\"ip6t_quota\");"));
        assert!(source.contains("static bool"));
        assert!(source.contains("quota_mt(const struct sk_buff *skb"));
        assert!(source.contains("bool ret = q->flags & XT_QUOTA_INVERT;"));
        assert!(source.contains("if (priv->quota >= skb->len)"));
        assert!(source.contains("priv->quota -= skb->len;"));
        assert!(source.contains("ret = !ret;"));
        assert!(source.contains("priv->quota = 0;"));
        assert!(source.contains("if (q->flags & ~XT_QUOTA_MASK)"));
        assert!(source.contains("return -EINVAL;"));
        assert!(source.contains("return -ENOMEM;"));
        assert!(source.contains("spin_lock_init(&q->master->lock);"));
        assert!(source.contains("q->master->quota = q->quota;"));
        assert!(source.contains(".name       = \"quota\""));
        assert!(source.contains(".family     = NFPROTO_UNSPEC"));
        assert!(source.contains("xt_register_match(&quota_mt_reg);"));
    }

    #[test]
    fn quota_match_counts_down_and_honors_invert() {
        let info = XtQuotaInfo {
            quota: 10,
            flags: 0,
        };
        let mut priv_ = quota_mt_check(info, true).unwrap();
        assert!(quota_mt(4, info, &mut priv_));
        assert_eq!(priv_.quota, 6);
        assert!(!quota_mt(7, info, &mut priv_));
        assert_eq!(priv_.quota, 0);

        let invert = XtQuotaInfo {
            flags: XT_QUOTA_INVERT,
            ..info
        };
        let mut priv_ = quota_mt_check(invert, true).unwrap();
        assert!(!quota_mt(4, invert, &mut priv_));
        assert!(quota_mt(20, invert, &mut priv_));
        assert_eq!(
            quota_mt_check(XtQuotaInfo { flags: 0x2, ..info }, true),
            Err(-EINVAL)
        );
        assert_eq!(quota_mt_check(info, false), Err(-ENOMEM));
        assert_eq!(quota_mt_init(), &QUOTA_MT_REG);
    }
}
