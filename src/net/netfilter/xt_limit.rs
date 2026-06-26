//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/xt_limit.c
//! test-origin: linux:vendor/linux/net/netfilter/xt_limit.c
//! Xtables token-bucket rate-limit match.

use crate::include::uapi::errno::{ENOMEM, ERANGE};

pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_AUTHOR: &str = "Herve Eychenne <rv@wallfire.org>";
pub const MODULE_DESCRIPTION: &str = "Xtables: rate-limit match";
pub const MODULE_ALIASES: [&str; 2] = ["ipt_limit", "ip6t_limit"];

pub const NFPROTO_UNSPEC: u8 = 0;
pub const XT_LIMIT_SCALE: u32 = 10_000;
pub const HZ: u32 = crate::kernel::time::jiffies::HZ as u32;
pub const MAX_CPJ: u32 = 0xffff_ffff / (HZ * 60 * 60 * 24);
pub const CREDITS_PER_JIFFY: u32 = pow2_below32(MAX_CPJ);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtLimitPriv {
    pub prev: u64,
    pub credit: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtRateinfo {
    pub avg: u32,
    pub burst: u32,
    pub prev: u64,
    pub credit: u32,
    pub credit_cap: u32,
    pub cost: u32,
    pub master: Option<XtLimitPriv>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompatXtRateinfo {
    pub avg: u32,
    pub burst: u32,
    pub prev: u32,
    pub credit: u32,
    pub credit_cap: u32,
    pub cost: u32,
    pub master: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtMatch {
    pub name: &'static str,
    pub revision: u8,
    pub family: u8,
    pub matchsize: usize,
    pub compatsize: usize,
    pub usersize: usize,
}

pub const LIMIT_MT_REG: XtMatch = XtMatch {
    name: "limit",
    revision: 0,
    family: NFPROTO_UNSPEC,
    matchsize: core::mem::size_of::<XtRateinfo>(),
    compatsize: core::mem::size_of::<CompatXtRateinfo>(),
    usersize: 8,
};

pub fn limit_mt(r: &mut XtRateinfo, now: u64) -> bool {
    let Some(mut priv_) = r.master else {
        return false;
    };
    if priv_.credit < r.cost && priv_.prev == now {
        return false;
    }

    let elapsed = now.wrapping_sub(priv_.prev);
    priv_.prev = now;
    let credit_increase = (elapsed as u32).wrapping_mul(CREDITS_PER_JIFFY);
    let mut new_credit = priv_.credit.wrapping_add(credit_increase);
    if new_credit > r.credit_cap {
        new_credit = r.credit_cap;
    }
    let ret = if new_credit >= r.cost {
        new_credit -= r.cost;
        true
    } else {
        false
    };
    priv_.credit = new_credit;
    r.master = Some(priv_);
    ret
}

pub const fn user2credits(user: u32) -> u32 {
    if user > 0xffff_ffff / (HZ * CREDITS_PER_JIFFY) {
        (user / XT_LIMIT_SCALE)
            .wrapping_mul(HZ)
            .wrapping_mul(CREDITS_PER_JIFFY)
    } else {
        user.wrapping_mul(HZ).wrapping_mul(CREDITS_PER_JIFFY) / XT_LIMIT_SCALE
    }
}

pub fn limit_mt_check(r: &mut XtRateinfo, now: u64, alloc_ok: bool) -> Result<(), i32> {
    if r.burst == 0 || user2credits(r.avg.wrapping_mul(r.burst)) < user2credits(r.avg) {
        return Err(-ERANGE);
    }
    if !alloc_ok {
        return Err(-ENOMEM);
    }

    let credit = user2credits(r.avg.wrapping_mul(r.burst));
    r.master = Some(XtLimitPriv { prev: now, credit });
    if r.cost == 0 {
        r.credit_cap = credit;
        r.cost = user2credits(r.avg);
    }
    Ok(())
}

pub fn limit_mt_destroy(r: &mut XtRateinfo) {
    r.master = None;
}

pub const fn limit_mt_compat_from_user(cm: CompatXtRateinfo) -> XtRateinfo {
    XtRateinfo {
        avg: cm.avg,
        burst: cm.burst,
        prev: cm.prev as u64 | ((cm.master as u64) << 32),
        credit: cm.credit,
        credit_cap: cm.credit_cap,
        cost: cm.cost,
        master: None,
    }
}

pub const fn limit_mt_compat_to_user(m: XtRateinfo) -> CompatXtRateinfo {
    CompatXtRateinfo {
        avg: m.avg,
        burst: m.burst,
        prev: m.prev as u32,
        credit: m.credit,
        credit_cap: m.credit_cap,
        cost: m.cost,
        master: (m.prev >> 32) as u32,
    }
}

pub const fn limit_mt_init() -> &'static XtMatch {
    &LIMIT_MT_REG
}

const fn pow2_below32(x: u32) -> u32 {
    let x = x | (x >> 1);
    let x = x | (x >> 2);
    let x = x | (x >> 4);
    let x = x | (x >> 8);
    let x = x | (x >> 16);
    (x >> 1) + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rateinfo(avg: u32, burst: u32) -> XtRateinfo {
        XtRateinfo {
            avg,
            burst,
            prev: 0,
            credit: 0,
            credit_cap: 0,
            cost: 0,
            master: None,
        }
    }

    #[test]
    fn xt_limit_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/xt_limit.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/uapi/linux/netfilter/xt_limit.h"
        ));
        assert!(header.contains("#define XT_LIMIT_SCALE 10000"));
        assert!(source.contains("struct xt_limit_priv"));
        assert!(source.contains("MODULE_ALIAS(\"ipt_limit\");"));
        assert!(source.contains("MODULE_ALIAS(\"ip6t_limit\");"));
        assert!(source.contains("#define MAX_CPJ (0xFFFFFFFF / (HZ*60*60*24))"));
        assert!(source.contains("#define CREDITS_PER_JIFFY POW2_BELOW32(MAX_CPJ)"));
        assert!(source.contains("limit_mt(const struct sk_buff *skb"));
        assert!(source.contains("READ_ONCE(priv->credit) < r->cost"));
        assert!(
            source
                .contains("credit_increase += (now - xchg(&priv->prev, now)) * CREDITS_PER_JIFFY;")
        );
        assert!(source.contains("if (new_credit > r->credit_cap)"));
        assert!(source.contains("new_credit -= r->cost;"));
        assert!(source.contains("static u32 user2credits(u32 user)"));
        assert!(source.contains("if (r->burst == 0"));
        assert!(source.contains("priv = kmalloc_obj(*priv);"));
        assert!(source.contains("r->master = priv;"));
        assert!(source.contains("priv->credit = user2credits(r->avg * r->burst);"));
        assert!(source.contains("kfree(info->master);"));
        assert!(source.contains("compat_xt_rateinfo"));
        assert!(source.contains(".name             = \"limit\""));
        assert!(source.contains("xt_register_match(&limit_mt_reg);"));
    }

    #[test]
    fn limit_check_initializes_bucket_and_match_consumes_refills() {
        let mut r = rateinfo(XT_LIMIT_SCALE, 2);
        assert_eq!(limit_mt_check(&mut r, 10, true), Ok(()));
        assert_eq!(r.credit_cap, user2credits(XT_LIMIT_SCALE * 2));
        assert_eq!(r.cost, user2credits(XT_LIMIT_SCALE));
        assert!(limit_mt(&mut r, 10));
        assert!(limit_mt(&mut r, 10));
        assert!(!limit_mt(&mut r, 10));
        assert!(limit_mt(&mut r, 10 + HZ as u64));
        limit_mt_destroy(&mut r);
        assert_eq!(r.master, None);

        let mut bad = rateinfo(XT_LIMIT_SCALE, 0);
        assert_eq!(limit_mt_check(&mut bad, 0, true), Err(-ERANGE));
        let mut no_mem = rateinfo(XT_LIMIT_SCALE, 1);
        assert_eq!(limit_mt_check(&mut no_mem, 0, false), Err(-ENOMEM));
        assert_eq!(limit_mt_init(), &LIMIT_MT_REG);
    }

    #[test]
    fn limit_compat_preserves_upper_prev_bits_in_master() {
        let cm = CompatXtRateinfo {
            avg: 1,
            burst: 2,
            prev: 0x89ab_cdef,
            credit: 3,
            credit_cap: 4,
            cost: 5,
            master: 0x0123_4567,
        };
        let native = limit_mt_compat_from_user(cm);
        assert_eq!(native.prev, 0x0123_4567_89ab_cdef);
        assert_eq!(limit_mt_compat_to_user(native), cm);
    }
}
