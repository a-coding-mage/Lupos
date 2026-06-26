//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/xt_statistic.c
//! test-origin: linux:vendor/linux/net/netfilter/xt_statistic.c
//! Xtables random and nth statistic match.

use crate::include::uapi::errno::{EINVAL, ENOMEM};

pub const XT_STATISTIC_MODE_RANDOM: u16 = 0;
pub const XT_STATISTIC_MODE_NTH: u16 = 1;
pub const XT_STATISTIC_MODE_MAX: u16 = 1;
pub const XT_STATISTIC_INVERT: u16 = 0x1;
pub const XT_STATISTIC_MASK: u16 = 0x1;
pub const NFPROTO_UNSPEC: u8 = 0;
pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_AUTHOR: &str = "Patrick McHardy <kaber@trash.net>";
pub const MODULE_DESCRIPTION: &str = "Xtables: statistics-based matching (\"Nth\", random)";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StatisticPriv {
    pub count: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtStatisticInfo {
    pub mode: u16,
    pub flags: u16,
    pub probability: u32,
    pub every: u32,
    pub count: u32,
    pub master: Option<StatisticPriv>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtMatch {
    pub name: &'static str,
    pub revision: u8,
    pub family: u8,
}

pub const XT_STATISTIC_MT_REG: XtMatch = XtMatch {
    name: "statistic",
    revision: 0,
    family: NFPROTO_UNSPEC,
};

pub fn statistic_mt(info: &mut XtStatisticInfo, random_u32: u32) -> bool {
    let mut ret = info.flags & XT_STATISTIC_INVERT != 0;
    match info.mode {
        XT_STATISTIC_MODE_RANDOM => {
            if (random_u32 & 0x7fff_ffff) < info.probability {
                ret = !ret;
            }
        }
        XT_STATISTIC_MODE_NTH => {
            let master = info
                .master
                .get_or_insert(StatisticPriv { count: info.count });
            let old = master.count;
            let new = if old == info.every { 0 } else { old + 1 };
            master.count = new;
            if new == 0 {
                ret = !ret;
            }
        }
        _ => {}
    }
    ret
}

pub fn statistic_mt_check(info: &mut XtStatisticInfo, alloc_ok: bool) -> Result<(), i32> {
    if info.mode > XT_STATISTIC_MODE_MAX || (info.flags & !XT_STATISTIC_MASK) != 0 {
        return Err(-EINVAL);
    }
    if !alloc_ok {
        return Err(-ENOMEM);
    }
    info.master = Some(StatisticPriv { count: info.count });
    Ok(())
}

pub fn statistic_mt_destroy(info: &mut XtStatisticInfo) {
    info.master = None;
}

pub const fn statistic_mt_init() -> &'static XtMatch {
    &XT_STATISTIC_MT_REG
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xt_statistic_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/xt_statistic.c"
        ));
        assert!(source.contains("struct xt_statistic_priv"));
        assert!(source.contains("MODULE_AUTHOR(\"Patrick McHardy"));
        assert!(source.contains("MODULE_ALIAS(\"ipt_statistic\");"));
        assert!(source.contains("statistic_mt(const struct sk_buff *skb"));
        assert!(source.contains("bool ret = info->flags & XT_STATISTIC_INVERT;"));
        assert!(source.contains("case XT_STATISTIC_MODE_RANDOM:"));
        assert!(source.contains("(get_random_u32() & 0x7FFFFFFF) < info->u.random.probability"));
        assert!(source.contains("case XT_STATISTIC_MODE_NTH:"));
        assert!(source.contains("nval = (oval == info->u.nth.every) ? 0 : oval + 1;"));
        assert!(source.contains("atomic_cmpxchg(&info->master->count, oval, nval)"));
        assert!(source.contains("if (nval == 0)"));
        assert!(source.contains("if (info->mode > XT_STATISTIC_MODE_MAX"));
        assert!(source.contains("info->flags & ~XT_STATISTIC_MASK"));
        assert!(source.contains("info->master = kzalloc_obj(*info->master);"));
        assert!(source.contains("atomic_set(&info->master->count, info->u.nth.count);"));
        assert!(source.contains("kfree(info->master);"));
        assert!(source.contains(".name       = \"statistic\""));
        assert!(source.contains("xt_register_match(&xt_statistic_mt_reg);"));
    }

    #[test]
    fn statistic_match_handles_random_nth_and_check_edges() {
        let mut random = XtStatisticInfo {
            mode: XT_STATISTIC_MODE_RANDOM,
            flags: 0,
            probability: 10,
            every: 0,
            count: 0,
            master: None,
        };
        assert!(statistic_mt(&mut random, 9));
        assert!(!statistic_mt(&mut random, 10));
        let mut nth = XtStatisticInfo {
            mode: XT_STATISTIC_MODE_NTH,
            flags: 0,
            probability: 0,
            every: 2,
            count: 1,
            master: None,
        };
        assert_eq!(statistic_mt_check(&mut nth, true), Ok(()));
        assert!(!statistic_mt(&mut nth, 0));
        assert_eq!(nth.master.unwrap().count, 2);
        assert!(statistic_mt(&mut nth, 0));
        assert_eq!(nth.master.unwrap().count, 0);
        statistic_mt_destroy(&mut nth);
        assert!(nth.master.is_none());
        assert_eq!(
            statistic_mt_check(&mut XtStatisticInfo { mode: 3, ..random }, true,),
            Err(-EINVAL)
        );
        assert_eq!(statistic_mt_check(&mut random, false), Err(-ENOMEM));
        assert_eq!(statistic_mt_init(), &XT_STATISTIC_MT_REG);
    }
}
