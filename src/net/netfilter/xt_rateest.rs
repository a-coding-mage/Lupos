//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/xt_rateest.c
//! test-origin: linux:vendor/linux/net/netfilter/xt_rateest.c
//! Xtables rate estimator match.

use crate::include::uapi::errno::{EINVAL, ENAMETOOLONG, ENOENT};

pub const MODULE_AUTHOR: &str = "Patrick McHardy <kaber@trash.net>";
pub const MODULE_DESCRIPTION: &str = "xtables rate estimator match";
pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_ALIASES: [&str; 2] = ["ipt_rateest", "ip6t_rateest"];

pub const IFNAMSIZ: usize = 16;
pub const NFPROTO_UNSPEC: u8 = 0;

pub const XT_RATEEST_MATCH_INVERT: u16 = 1 << 0;
pub const XT_RATEEST_MATCH_ABS: u16 = 1 << 1;
pub const XT_RATEEST_MATCH_REL: u16 = 1 << 2;
pub const XT_RATEEST_MATCH_DELTA: u16 = 1 << 3;
pub const XT_RATEEST_MATCH_BPS: u16 = 1 << 4;
pub const XT_RATEEST_MATCH_PPS: u16 = 1 << 5;

pub const XT_RATEEST_MATCH_NONE: u16 = 0;
pub const XT_RATEEST_MATCH_EQ: u16 = 1;
pub const XT_RATEEST_MATCH_LT: u16 = 2;
pub const XT_RATEEST_MATCH_GT: u16 = 3;

pub const XT_RATEEST_USERSIZE: usize = 56;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GnetStatsRateEst64 {
    pub bps: u64,
    pub pps: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtRateestMatchInfo {
    pub name1: [u8; IFNAMSIZ],
    pub name2: [u8; IFNAMSIZ],
    pub flags: u16,
    pub mode: u16,
    pub bps1: u32,
    pub pps1: u32,
    pub bps2: u32,
    pub pps2: u32,
    pub est1: Option<GnetStatsRateEst64>,
    pub est2: Option<GnetStatsRateEst64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XtMatch {
    pub name: &'static str,
    pub revision: u8,
    pub family: u8,
    pub matchsize: usize,
    pub usersize: usize,
}

pub const XT_RATEEST_MT_REG: XtMatch = XtMatch {
    name: "rateest",
    revision: 0,
    family: NFPROTO_UNSPEC,
    matchsize: core::mem::size_of::<XtRateestMatchInfo>(),
    usersize: XT_RATEEST_USERSIZE,
};

pub fn xt_rateest_mt(info: &XtRateestMatchInfo) -> bool {
    let sample1 = info.est1.unwrap_or_default();
    let (bps1, pps1) = rate_pair(info.bps1, info.pps1, sample1, info.flags);

    let (bps2, pps2) = if info.flags & XT_RATEEST_MATCH_ABS != 0 {
        (info.bps2, info.pps2)
    } else {
        let sample2 = info.est2.unwrap_or_default();
        rate_pair(info.bps2, info.pps2, sample2, info.flags)
    };

    let mut ret = true;
    match info.mode {
        XT_RATEEST_MATCH_LT => {
            if info.flags & XT_RATEEST_MATCH_BPS != 0 {
                ret &= bps1 < bps2;
            }
            if info.flags & XT_RATEEST_MATCH_PPS != 0 {
                ret &= pps1 < pps2;
            }
        }
        XT_RATEEST_MATCH_GT => {
            if info.flags & XT_RATEEST_MATCH_BPS != 0 {
                ret &= bps1 > bps2;
            }
            if info.flags & XT_RATEEST_MATCH_PPS != 0 {
                ret &= pps1 > pps2;
            }
        }
        XT_RATEEST_MATCH_EQ => {
            if info.flags & XT_RATEEST_MATCH_BPS != 0 {
                ret &= bps1 == bps2;
            }
            if info.flags & XT_RATEEST_MATCH_PPS != 0 {
                ret &= pps1 == pps2;
            }
        }
        _ => {}
    }

    ret != (info.flags & XT_RATEEST_MATCH_INVERT != 0)
}

pub fn xt_rateest_mt_checkentry(
    info: &mut XtRateestMatchInfo,
    est1: Option<GnetStatsRateEst64>,
    est2: Option<GnetStatsRateEst64>,
) -> Result<(), i32> {
    if hweight32((info.flags & (XT_RATEEST_MATCH_ABS | XT_RATEEST_MATCH_REL)) as u32) != 1 {
        return Err(-EINVAL);
    }
    if info.flags & (XT_RATEEST_MATCH_BPS | XT_RATEEST_MATCH_PPS) == 0 {
        return Err(-EINVAL);
    }
    match info.mode {
        XT_RATEEST_MATCH_EQ | XT_RATEEST_MATCH_LT | XT_RATEEST_MATCH_GT => {}
        _ => return Err(-EINVAL),
    }
    if strnlen(&info.name1) >= info.name1.len() || strnlen(&info.name2) >= info.name2.len() {
        return Err(-ENAMETOOLONG);
    }

    let Some(found1) = est1 else {
        return Err(-ENOENT);
    };
    let found2 = if info.flags & XT_RATEEST_MATCH_REL != 0 {
        let Some(found2) = est2 else {
            return Err(-ENOENT);
        };
        Some(found2)
    } else {
        None
    };

    info.est1 = Some(found1);
    info.est2 = found2;
    Ok(())
}

pub fn xt_rateest_mt_destroy(info: &mut XtRateestMatchInfo) {
    info.est1 = None;
    info.est2 = None;
}

pub const fn xt_rateest_mt_init() -> &'static XtMatch {
    &XT_RATEEST_MT_REG
}

fn rate_pair(cfg_bps: u32, cfg_pps: u32, sample: GnetStatsRateEst64, flags: u16) -> (u32, u32) {
    if flags & XT_RATEEST_MATCH_DELTA != 0 {
        (
            cfg_bps.saturating_sub(sample.bps as u32),
            cfg_pps.saturating_sub(sample.pps as u32),
        )
    } else {
        (sample.bps as u32, sample.pps as u32)
    }
}

const fn hweight32(mut value: u32) -> u32 {
    let mut count = 0;
    while value != 0 {
        count += value & 1;
        value >>= 1;
    }
    count
}

fn strnlen(bytes: &[u8]) -> usize {
    bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn named(name: &[u8], flags: u16, mode: u16) -> XtRateestMatchInfo {
        let mut info = XtRateestMatchInfo {
            name1: [0; IFNAMSIZ],
            name2: [0; IFNAMSIZ],
            flags,
            mode,
            bps1: 100,
            pps1: 10,
            bps2: 50,
            pps2: 5,
            est1: None,
            est2: None,
        };
        info.name1[..name.len()].copy_from_slice(name);
        info
    }

    #[test]
    fn xt_rateest_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/xt_rateest.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/uapi/linux/netfilter/xt_rateest.h"
        ));
        assert!(header.contains("enum xt_rateest_match_flags"));
        assert!(header.contains("XT_RATEEST_MATCH_DELTA"));
        assert!(source.contains("xt_rateest_mt(const struct sk_buff *skb"));
        assert!(source.contains("gen_estimator_read(&info->est1->rate_est, &sample);"));
        assert!(source.contains("case XT_RATEEST_MATCH_LT:"));
        assert!(source.contains("case XT_RATEEST_MATCH_GT:"));
        assert!(source.contains("case XT_RATEEST_MATCH_EQ:"));
        assert!(source.contains("ret ^= info->flags & XT_RATEEST_MATCH_INVERT ? true : false;"));
        assert!(source.contains("hweight32(info->flags & (XT_RATEEST_MATCH_ABS |"));
        assert!(
            source.contains("if (!(info->flags & (XT_RATEEST_MATCH_BPS | XT_RATEEST_MATCH_PPS)))")
        );
        assert!(source.contains("strnlen(info->name1, sizeof(info->name1))"));
        assert!(source.contains("est1 = xt_rateest_lookup(par->net, info->name1);"));
        assert!(source.contains("xt_rateest_put(par->net, info->est1);"));
        assert!(source.contains(".usersize   = offsetof(struct xt_rateest_match_info, est1)"));
        assert!(source.contains("xt_register_match(&xt_rateest_mt_reg);"));
    }

    #[test]
    fn rateest_checkentry_and_match_follow_modes_flags_and_destroy() {
        let mut info = named(
            b"wan\0",
            XT_RATEEST_MATCH_ABS | XT_RATEEST_MATCH_BPS,
            XT_RATEEST_MATCH_GT,
        );
        assert_eq!(
            xt_rateest_mt_checkentry(
                &mut info,
                Some(GnetStatsRateEst64 { bps: 60, pps: 0 }),
                None
            ),
            Ok(())
        );
        assert!(xt_rateest_mt(&info));
        info.flags |= XT_RATEEST_MATCH_INVERT;
        assert!(!xt_rateest_mt(&info));

        let mut rel = named(
            b"wan\0",
            XT_RATEEST_MATCH_REL
                | XT_RATEEST_MATCH_DELTA
                | XT_RATEEST_MATCH_BPS
                | XT_RATEEST_MATCH_PPS,
            XT_RATEEST_MATCH_EQ,
        );
        assert_eq!(
            xt_rateest_mt_checkentry(
                &mut rel,
                Some(GnetStatsRateEst64 { bps: 80, pps: 8 }),
                Some(GnetStatsRateEst64 { bps: 30, pps: 3 })
            ),
            Ok(())
        );
        assert!(xt_rateest_mt(&rel));
        xt_rateest_mt_destroy(&mut rel);
        assert_eq!(rel.est1, None);
        assert_eq!(rel.est2, None);

        let mut bad = named(
            b"bad\0",
            XT_RATEEST_MATCH_ABS | XT_RATEEST_MATCH_REL,
            XT_RATEEST_MATCH_GT,
        );
        assert_eq!(xt_rateest_mt_checkentry(&mut bad, None, None), Err(-EINVAL));
        bad.flags = XT_RATEEST_MATCH_ABS;
        assert_eq!(xt_rateest_mt_checkentry(&mut bad, None, None), Err(-EINVAL));
        bad.flags = XT_RATEEST_MATCH_ABS | XT_RATEEST_MATCH_BPS;
        bad.mode = XT_RATEEST_MATCH_NONE;
        assert_eq!(xt_rateest_mt_checkentry(&mut bad, None, None), Err(-EINVAL));
        bad.mode = XT_RATEEST_MATCH_GT;
        bad.name1 = [b'x'; IFNAMSIZ];
        assert_eq!(
            xt_rateest_mt_checkentry(&mut bad, Some(GnetStatsRateEst64::default()), None),
            Err(-ENAMETOOLONG)
        );
        assert_eq!(xt_rateest_mt_init(), &XT_RATEEST_MT_REG);
    }
}
